//! Native DOM boundary ordering retains child offsets, including non-layout nodes.
use crate::layout_snapshot::SourceId;
use crate::live::source_id;
use std::collections::HashMap;
use turbo_dom::rtdom::tree::Handle;
use turbo_dom::rtdom::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Element(SourceId),
    Text {
        source: SourceId,
        start: usize,
        end: usize,
    },
}
struct Span {
    before: u64,
    after: u64,
    data: Option<(u64, usize)>,
    children: Vec<u64>,
    parent: Option<Handle>,
    kind: u8,
}
struct Order {
    next: u64,
    left: usize,
    units_left: usize,
    spans: HashMap<Handle, Span>,
}
impl Order {
    fn visit(&mut self, tree: &Tree, handle: Handle, depth: usize) -> Result<(), String> {
        if depth > 256 || self.left == 0 {
            return Err("native Range ordering exceeds source budget".into());
        }
        self.left -= 1;
        let before = self.next;
        self.next += 1;
        let kind = tree.node_type_id(handle);
        let data = if matches!(kind, 3 | 4 | 7 | 8) {
            let units = tree.node_value_utf16(handle).map_or(0, |value| value.len());
            self.units_left = self
                .units_left
                .checked_sub(units)
                .ok_or("native Range ordering exceeds character-data budget")?;
            let start = self.next;
            self.next += units as u64;
            Some((start, units))
        } else {
            None
        };
        let mut children = vec![self.next];
        for child in tree.children(handle) {
            self.visit(tree, child, depth + 1)?;
            children.push(self.next);
        }
        let after = self.next;
        self.next += 1;
        self.spans.insert(
            handle,
            Span {
                before,
                after,
                data,
                children,
                parent: tree.parent(handle),
                kind,
            },
        );
        Ok(())
    }
    fn point(&self, (handle, offset): (Handle, usize)) -> Result<u64, String> {
        let span = self
            .spans
            .get(&handle)
            .ok_or("native Range boundary is outside connected tree")?;
        if let Some((start, units)) = span.data {
            if offset > units {
                return Err("native Range UTF-16 boundary exceeds source".into());
            }
            Ok(start + offset as u64)
        } else {
            span.children
                .get(offset)
                .copied()
                .ok_or("native Range child boundary exceeds source".into())
        }
    }
}
fn fully_selected(span: &Span, start: u64, end: u64) -> bool {
    start <= span.before && end >= span.after
}
pub fn select(
    tree: &Tree,
    document: u32,
    start: (Handle, usize),
    end: (Handle, usize),
) -> Result<Vec<Selection>, String> {
    let mut order = Order {
        next: 0,
        left: 50_000,
        units_left: 2_000_000,
        spans: HashMap::new(),
    };
    let mut root = start.0;
    while let Some(parent) = tree.parent(root) {
        root = parent;
    }
    order.visit(tree, root, 0)?;
    let begin = order.point(start)?;
    let finish = order.point(end)?;
    if begin > finish {
        return Err("native Range boundaries are reversed".into());
    }
    let mut nodes: Vec<_> = order.spans.iter().collect();
    nodes.sort_by_key(|(_, span)| span.before);
    let mut result = Vec::new();
    for (&handle, span) in nodes {
        let source = source_id(document, handle);
        if span.kind == 1 && fully_selected(span, begin, finish) {
            let parent_selected = span
                .parent
                .and_then(|id| order.spans.get(&id))
                .is_some_and(|parent| parent.kind == 1 && fully_selected(parent, begin, finish));
            if !parent_selected {
                result.push(Selection::Element(source));
            }
        }
        // Only Text contributes character boxes (its span always records its data).
        let (3, Some((base, units))) = (span.kind, span.data) else {
            continue;
        };
        if units == 0 {
            continue;
        }
        let end_position = base + units as u64;
        let intersects = if begin == finish {
            begin >= base && begin <= end_position
        } else {
            (begin < end_position && finish > base) || handle == start.0 || handle == end.0
        };
        if intersects {
            result.push(Selection::Text {
                source,
                start: begin.saturating_sub(base).min(units as u64) as usize,
                end: finish.saturating_sub(base).min(units as u64) as usize,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_child_comments_and_astral_units_keep_native_dom_boundaries() {
        let tree = Tree::parse("<div>a😀<!--sentinel--><b>bc</b>d</div>");
        let div = tree.query_selector("div").unwrap();
        let children = tree.children(div);
        let selections = select(&tree, 7, (children[0], 1), (div, 3)).unwrap();
        assert_eq!(
            selections,
            vec![
                Selection::Text {
                    source: source_id(7, children[0]),
                    start: 1,
                    end: 3
                },
                Selection::Element(source_id(7, children[2])),
                Selection::Text {
                    source: source_id(7, tree.children(children[2])[0]),
                    start: 0,
                    end: 2
                },
            ]
        );
        assert!(select(&tree, 7, (div, 2), (div, 2)).unwrap().is_empty());
        assert!(select(&tree, 7, (children[0], 4), (children[0], 4)).is_err());
    }

    /// DOM boundary rules (Chrome's Range would throw before getClientRects for each):
    /// reversed ends, an offset past the node's length, a node outside the tree.
    #[test]
    fn invalid_boundaries_are_errors() {
        let mut tree = Tree::parse("<div>ab<i></i></div>");
        let div = tree.query_selector("div").unwrap();
        let text = tree.children(div)[0];
        let error = |tree: &Tree, start, end| select(tree, 1, start, end).unwrap_err();
        assert_eq!(
            error(&tree, (text, 2), (text, 1)),
            "native Range boundaries are reversed"
        );
        assert_eq!(
            error(&tree, (text, 0), (text, 3)),
            "native Range UTF-16 boundary exceeds source"
        );
        assert_eq!(
            error(&tree, (div, 0), (div, 3)),
            "native Range child boundary exceeds source"
        );
        let detached = tree.create_element("p");
        assert_eq!(
            error(&tree, (div, 0), (detached, 0)),
            "native Range boundary is outside connected tree"
        );
        assert_eq!(select(&tree, 1, (div, 2), (div, 2)).unwrap(), vec![]);
        assert_eq!(select(&tree, 1, (text, 2), (text, 2)).unwrap().len(), 1);
    }

    /// An empty Text node has no character boxes (Chrome: no client rect) even when it
    /// holds a boundary; the element around it is still selected.
    #[test]
    fn empty_text_contributes_nothing() {
        let mut tree = Tree::parse("<div><b></b></div>");
        let b = tree.query_selector("b").unwrap();
        let empty = tree.create_text_node("");
        tree.append_child(b, empty);
        let div = tree.query_selector("div").unwrap();
        assert_eq!(
            select(&tree, 1, (div, 0), (div, 1)).unwrap(),
            vec![Selection::Element(source_id(1, b))]
        );
        assert_eq!(select(&tree, 1, (empty, 0), (empty, 0)).unwrap(), vec![]);
    }

    #[test]
    fn source_budgets_bound_the_walk() {
        let deep = format!("{}x", "<span>".repeat(300));
        let tree = Tree::parse(&deep);
        let root = tree.root();
        assert_eq!(
            select(&tree, 1, (root, 0), (root, 0)).unwrap_err(),
            "native Range ordering exceeds source budget"
        );
        let wide = "<i></i>".repeat(50_001);
        let tree = Tree::parse(&wide);
        let root = tree.root();
        assert_eq!(
            select(&tree, 1, (root, 0), (root, 0)).unwrap_err(),
            "native Range ordering exceeds source budget"
        );
        let long = format!("<p>{}</p>", "a".repeat(2_000_001));
        let tree = Tree::parse(&long);
        let root = tree.root();
        assert_eq!(
            select(&tree, 1, (root, 0), (root, 0)).unwrap_err(),
            "native Range ordering exceeds character-data budget"
        );
        // At the limits the walk still succeeds.
        let tree = Tree::parse(&format!("<p>{}</p>", "a".repeat(1_999_999)));
        assert!(select(&tree, 1, (tree.root(), 0), (tree.root(), 0)).is_ok());
    }
}

#[cfg(all(test, feature = "bundled-fonts"))]
mod chrome_tests {
    use super::*;
    use crate::cssom_view::scroll::chrome::{case, close, rect};
    use crate::live::testing::DOCUMENT;
    use serde_json::Value;

    #[test]
    fn range_client_rects_match_chrome_156() {
        let case = case("range-client-rects");
        let boundary = |b: &Value| {
            let element = case
                .tree
                .query_selector(&format!("[data-cid=\"{}\"]", b[0].as_str().unwrap()))
                .unwrap();
            let node = b[1]
                .as_u64()
                .map_or(element, |child| case.tree.children(element)[child as usize]);
            (node, b[2].as_u64().unwrap() as usize)
        };
        let mut failures = Vec::new();
        for want in case.reads("ranges") {
            let selections = select(
                &case.tree,
                DOCUMENT,
                boundary(&want["start"]),
                boundary(&want["end"]),
            )
            .unwrap();
            let mut got = Vec::new();
            for selection in selections {
                match selection {
                    Selection::Element(id) => got.extend(
                        crate::cssom_view::metrics::element_geometry(&case.snapshot, id)
                            .unwrap()
                            .rects,
                    ),
                    Selection::Text { source, start, end } => got.extend(
                        case.snapshot
                            .text_range_boxes(source, start, end)
                            .unwrap()
                            .into_iter()
                            .map(|cell| cell.transformed_bounds),
                    ),
                }
            }
            let chrome: Vec<[f32; 4]> =
                want["rects"].as_array().unwrap().iter().map(rect).collect();
            let same = got.len() == chrome.len()
                && got.iter().zip(&chrome).all(|(r, c)| {
                    close(r.x, c[0])
                        && close(r.y, c[1])
                        && close(r.width, c[2])
                        && close(r.height, c[3])
                });
            if !same {
                failures.push(format!("{want}: got {got:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

//! Native DOM boundary ordering retains child offsets, including non-layout nodes.
use crate::live_layout::source_id;
use std::collections::HashMap;
use turbo_dom_layout::layout_snapshot::SourceId;
use turbo_dom_parser::rtdom::tree::Handle;
use turbo_dom_parser::rtdom::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
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
pub(crate) fn select(
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
        if span.kind != 3 {
            continue;
        }
        let Some((base, units)) = span.data else {
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
}

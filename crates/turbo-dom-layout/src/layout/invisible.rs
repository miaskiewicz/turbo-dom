//! Invisible boxes (miaskiewicz/turbo-dom#209): a box with `opacity: 0` or
//! `visibility: hidden | collapse` is laid out like any other (it keeps its space, as in
//! Chrome) but paints nothing. Box generation keeps such boxes, and after layout
//! [`scrub`] blanks their paint in the galley, so every consumer of the fragment tree
//! (the raster painters, the PDF emitter) leaves them out without reading any flag.
//!
//! `visibility` inherits, so each box and inline item carries its own computed value: a
//! `visibility: visible` descendant of a hidden box still paints. `opacity: 0` hides
//! the whole subtree.

use std::collections::HashSet;

use super::boxgen::{BoxKind, InlineItem, LayoutBox};
use super::fragment::{Fragment, FragmentContent, NodeId};
use super::value::BorderEdges;
use crate::style::ComputedStyle;

/// Whether a computed style is `visibility: hidden` or `collapse` (outside tables
/// `collapse` behaves as `hidden`).
fn invisible(style: &ComputedStyle) -> bool {
    matches!(
        style
            .get("visibility")
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("hidden" | "collapse")
    )
}

/// The ids of the boxes and inline items in `root`'s tree whose own `visibility` hides
/// them.
pub(crate) fn hidden_ids(root: &LayoutBox) -> HashSet<NodeId> {
    let mut hidden = HashSet::new();
    let mut pending = vec![root];
    while let Some(lb) = pending.pop() {
        if invisible(&lb.style) {
            hidden.insert(lb.node_id);
        }
        match &lb.kind {
            BoxKind::Block(kids)
            | BoxKind::Flex(kids)
            | BoxKind::Grid(kids)
            | BoxKind::Table(kids) => {
                pending.extend(kids.iter());
            }
            BoxKind::Lines(items) => {
                for item in items {
                    match item {
                        InlineItem::SourceBreak { node_id, style }
                        | InlineItem::Boundary { node_id, style, .. }
                        | InlineItem::Text { node_id, style, .. } => {
                            if invisible(style) {
                                hidden.insert(*node_id);
                            }
                        }
                        InlineItem::Atomic(atomic) => pending.push(atomic),
                        InlineItem::LineBreak | InlineItem::Marker { .. } => {}
                    }
                }
            }
            BoxKind::Marker(_) => {}
        }
    }
    hidden
}

/// Blank the paint of every fragment of a hidden box (`hidden`) and of every fragment
/// inside an `opacity: 0` one, keeping all geometry.
pub(crate) fn scrub(frag: &mut Fragment, hidden: &HashSet<NodeId>) {
    let mut pending = vec![(frag, false)];
    while let Some((frag, transparent)) = pending.pop() {
        let transparent = transparent || frag.opacity <= 0.0;
        if transparent || hidden.contains(&frag.node_id) {
            blank(&mut frag.content);
        }
        pending.extend(frag.children.iter_mut().map(|child| (child, transparent)));
    }
}

/// Drop everything a fragment's own content paints.
fn blank(content: &mut FragmentContent) {
    match content {
        FragmentContent::Box {
            background,
            border,
            shadow,
            gradient,
            ..
        } => {
            *background = None;
            *border = BorderEdges::default();
            *shadow = None;
            *gradient = None;
        }
        FragmentContent::TextLine { glyphs, text, .. } => {
            glyphs.clear();
            text.clear();
        }
        // A marker is a paged-media directive (a forced break), not paint: a hidden
        // box still breaks the page, as in Chrome.
        FragmentContent::Marker(_) => {}
        FragmentContent::Image(_) => {
            *content = FragmentContent::Box {
                background: None,
                border: BorderEdges::default(),
                border_radius: 0.0,
                shadow: None,
                gradient: None,
                transform: None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::boxgen::build_box_tree;
    use crate::layout::fragment::ImagePlacement;
    use crate::tag::{MarkerKind, Tag};
    use crate::{StyledElement, StyledNode};

    fn el(tag: &str, pairs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Element(StyledElement {
            tag: Tag::Html(tag.to_string()),
            attrs: vec![],
            style: ComputedStyle::from_pairs(
                pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())),
            ),
            children,
            source: None,
        })
    }

    #[test]
    fn hidden_inline_items_are_collected_by_their_own_visibility() {
        let tree = build_box_tree(&[el(
            "p",
            &[],
            vec![
                StyledNode::Text("a".into()),
                el(
                    "span",
                    &[("visibility", "hidden")],
                    vec![StyledNode::Text("b".into())],
                ),
            ],
        )]);
        let hidden = hidden_ids(&tree);
        assert!(!hidden.is_empty(), "the hidden span's items are collected");
        assert!(!hidden.contains(&tree.node_id));
    }

    #[test]
    fn blank_drops_images_but_keeps_markers() {
        let image = FragmentContent::Image(ImagePlacement {
            name: "x.png".into(),
            intrinsic_w: 1,
            intrinsic_h: 1,
            has_alpha: false,
            tint: None,
            orientation: crate::image::Orientation::IDENTITY,
        });
        let mut frag = Fragment::new(NodeId(1), 0.0, 0.0, 5.0, 5.0, image);
        frag.children.push(Fragment::new(
            NodeId(2),
            0.0,
            0.0,
            0.0,
            0.0,
            FragmentContent::Marker(MarkerKind {
                id: 7,
                inline: false,
            }),
        ));
        scrub(&mut frag, &HashSet::from([NodeId(1), NodeId(2)]));
        assert!(matches!(
            frag.content,
            FragmentContent::Box {
                background: None,
                ..
            }
        ));
        assert_eq!((frag.width, frag.height), (5.0, 5.0));
        assert!(matches!(
            frag.children[0].content,
            FragmentContent::Marker(MarkerKind {
                id: 7,
                inline: false
            })
        ));
    }
}

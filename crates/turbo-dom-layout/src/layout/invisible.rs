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
        FragmentContent::Image(_) | FragmentContent::Marker(_) => {
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

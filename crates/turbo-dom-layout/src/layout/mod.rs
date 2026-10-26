//! The layout engine (§5, Stage 3): turns the styled tree into a galley — a
//! single continuous coordinate space of positioned [`Fragment`]s that the
//! fragmenter (Stage 4) paginates. Block and table flow and inline/text layout
//! are owned here; flex delegates to `taffy` (§5.3 decision).

pub mod block;
pub mod boxgen;
pub(crate) mod contents;
pub mod flex;
pub mod fragment;
pub mod imgsize;
pub mod inline;
pub(crate) mod inline_boxes;
mod svg_numbers;
pub(crate) mod svg_viewport;
pub mod table;
pub mod value;

use crate::error::Diagnostics;
use crate::image::{ImageResolver, NoImages};
use crate::style::StyledNode;
use crate::text::FontRegistry;

pub use fragment::Fragment;

use std::collections::{HashMap, HashSet};

use fragment::NodeId;

/// Layout-time options (behaviour switches that used to be compile-time features
/// of the PDF engine). The default is the plain layout every other entry uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayoutOptions {
    /// Lay every `<a href="#name">` out as an atomic inline box, so the link
    /// carries its own fragment (html2pdf's internal links).
    pub atomic_internal_links: bool,
    /// Accept `cmyk(...)` colours (html2pdf's print colour).
    pub accept_cmyk: bool,
    /// The [`NodeId`] the layout's box counter starts at (the root box gets it).
    /// A consumer laying several flows out into one document gives each a disjoint
    /// range (see [`NodeSources::next`]) so ids are unique per render.
    pub first_node_id: u32,
}

/// Where each box of a [`layout_recorded`] galley came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeSources {
    /// The root box's id (the layout's first id).
    pub root: NodeId,
    /// One past the last id this layout allocated: the next flow's `first_node_id`.
    pub next: NodeId,
    /// Element boxes and inline markers -> their source element.
    pub elements: HashMap<NodeId, turbo_dom::Handle>,
    /// Text runs -> their source text node.
    pub texts: HashMap<NodeId, turbo_dom::Handle>,
    /// The boxes laid out as table rows (their single fragment is the synthetic
    /// row box, not an ordinary box fragment).
    pub table_rows: HashSet<NodeId>,
}

/// A galley plus the [`NodeSources`] of its boxes.
#[derive(Debug, Clone)]
pub struct LaidOut {
    pub galley: Fragment,
    pub sources: NodeSources,
}

/// Per-layout image inputs (§7.4): the caller's resolver (for intrinsic sizing)
/// and the page body height that bounds image height (the 60% cap, user spec).
/// `body_height` is `None` when the caller lays out without page geometry (e.g.
/// measuring a region), in which case only the width cap applies.
pub struct ImageCtx<'a> {
    pub resolver: &'a dyn ImageResolver,
    pub body_height: Option<f32>,
}

impl ImageCtx<'_> {
    /// The default: no resolver, no height basis — lays out no images.
    pub(crate) fn none() -> ImageCtx<'static> {
        ImageCtx {
            resolver: &NoImages,
            body_height: None,
        }
    }
}

/// Lay out a styled tree into the galley (Stage 3): box generation followed by
/// block/inline/flex/table layout into one continuous top-down fragment tree of
/// content width `cb_width` px. Missing-glyph and similar issues are collected
/// into `diags`.
///
/// This convenience entry embeds no images; use [`layout_with_images`] to size
/// `<img>`/`background-image` boxes against a caller-supplied resolver.
pub fn layout(
    styled: &[StyledNode],
    cb_width: f32,
    fonts: &FontRegistry,
    diags: &mut Diagnostics,
) -> Fragment {
    let tree = boxgen::build_box_tree(styled);
    block::layout_tree(&tree, cb_width, fonts, diags)
}

/// Lay out a styled tree, sizing raster images against `images` (§7.4). Each
/// `<img>`/`background-image` is probed for its intrinsic pixel size, scaled to
/// preserve aspect ratio, and capped to `max-width = 100%` of its containing
/// block and `max-height ≈ 60%` of `images.body_height` so an image never
/// overflows a page (images are never split across pages).
pub fn layout_with_images(
    styled: &[StyledNode],
    cb_width: f32,
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
) -> Fragment {
    let tree = boxgen::build_box_tree(styled);
    block::layout_tree_with_images(&tree, cb_width, fonts, images, diags)
}

/// [`layout_with_images`] with explicit [`LayoutOptions`].
pub fn layout_with_options(
    styled: &[StyledNode],
    cb_width: f32,
    fonts: &FontRegistry,
    images: &ImageCtx,
    options: &LayoutOptions,
    diags: &mut Diagnostics,
) -> Fragment {
    let _cmyk = value::cmyk_scope(options.accept_cmyk);
    let (tree, _, _) = boxgen::build_box_tree_with(styled, build_options(options, false));
    let mut galley = block::layout_tree_with_images(&tree, cb_width, fonts, images, diags);
    shift_node_ids(&mut galley, options.first_node_id);
    galley
}

/// [`layout_with_options`] that also records each box's source (the
/// [`StyledElement::source`](crate::StyledElement::source) /
/// [`StyledText::source`](crate::style::StyledText::source) it was built from).
/// The galley is identical to [`layout_with_options`]'s; recording is independent
/// of the live-snapshot mode.
pub fn layout_recorded(
    styled: &[StyledNode],
    cb_width: f32,
    fonts: &FontRegistry,
    images: &ImageCtx,
    options: &LayoutOptions,
    diags: &mut Diagnostics,
) -> LaidOut {
    let _cmyk = value::cmyk_scope(options.accept_cmyk);
    let (tree, recorded, count) = boxgen::build_box_tree_with(styled, build_options(options, true));
    let (mut galley, rows) = block::layout_tree_recorded(&tree, cb_width, fonts, images, diags);
    let first = options.first_node_id;
    shift_node_ids(&mut galley, first);
    let shift = |id: NodeId| NodeId(id.0 + first);
    let shift_map = |map: HashMap<NodeId, turbo_dom::Handle>| {
        map.into_iter().map(|(id, h)| (shift(id), h)).collect()
    };
    LaidOut {
        galley,
        sources: NodeSources {
            root: NodeId(first),
            next: NodeId(first + count),
            elements: shift_map(recorded.elements),
            texts: shift_map(recorded.texts),
            table_rows: rows.into_iter().map(shift).collect(),
        },
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // mirrors the other `&LayoutOptions` readers
fn build_options(options: &LayoutOptions, record_sources: bool) -> boxgen::BuildOptions {
    boxgen::BuildOptions {
        atomic_internal_links: options.atomic_internal_links,
        record_sources,
    }
}

/// Offset every fragment's id by `first` (layout always numbers from 0 internally:
/// the root box is recognized by id 0).
fn shift_node_ids(frag: &mut Fragment, first: u32) {
    if first == 0 {
        return;
    }
    let mut pending = vec![frag];
    while let Some(frag) = pending.pop() {
        frag.node_id = NodeId(frag.node_id.0 + first);
        pending.extend(frag.children.iter_mut());
    }
}

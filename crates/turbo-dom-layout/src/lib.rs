//! turbo-dom layout engine.
//!
//! The native HTML/CSS layout half of turbo-dom: a style cascade over a turbo-dom
//! [`Tree`](turbo_dom::Tree) ([`style::style_input`] over a [`style::TreeView`]), box
//! generation, block/inline/flex/grid/table layout into one continuous galley of
//! positioned [`Fragment`]s, a font registry with shaping + fallback (optionally
//! with bundled OFL faces), image intrinsic sizing, and CSSOM-View style layout
//! snapshots. [`layout_html`] drives raw HTML straight to a galley. [`live`] styles a
//! live, embedder-composed [`Tree`](turbo_dom::Tree) without copying it (turbo-surf's
//! CSSOM-View layout), and [`cssom_view`] answers the geometry APIs over its snapshots.
//!
//! Relocated from `turbo-html2pdf-core` 0.3.9, which depends on this crate (its
//! paged-media `t:` directives are [`MarkerKind`] markers classified by html2pdf,
//! and its PDF-only switches are runtime [`LayoutOptions`]); `turbo-dom-raster`
//! paints the galley.

#![forbid(unsafe_code)]

pub mod cmap;
pub mod cssom_view;
pub mod error;
pub mod html_layout;
pub mod image;
pub mod layout;
pub mod layout_snapshot;
pub mod live;
pub mod perf;
pub mod style;
#[cfg(feature = "svg-rasterize")]
pub mod svg;
pub mod tag;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod text;

pub use error::{CompileError, Diagnostics, ErrorCode, Lint, LintCode, RenderError, Span};
pub use html_layout::{
    collect_style_css_tree, layout_boxes, layout_boxes_tree, layout_html, layout_html_tree,
    layout_html_tree_in_viewport, layout_html_with_images, parse_html_tree, style_html_tree,
    CidBox, HtmlOptions,
};
pub use image::{ImageResolver, NoImages};
pub use layout::fragment::{
    BoxShadow, BreakMeta, ClipRect, Fragment, FragmentContent, GradientStop, ImagePlacement,
    LinearGradient, NodeId, PositionedGlyph, RepeatKind, Transform2D,
};
pub use layout::value::{BreakRule, Rgba};
pub use layout::{
    layout, layout_in_viewport, layout_recorded, layout_recorded_in_viewport, layout_with_images,
    layout_with_options, ImageCtx, LaidOut, LayoutOptions, NodeSources,
};
pub use style::{
    build_cascade, build_cascade_with_width, document_root_font_px, root_font_px, root_font_scope,
    set_media_viewport_height, set_media_viewport_width, style_input, Cascade, ComputedStyle,
    RootFontScope, Roots, StyleInput, StyleOptions, StyledElement, StyledNode, StyledText,
    TreeView,
};
pub use tag::{Attr, MarkerClassifier, MarkerKind, Tag};
pub use text::{layout_text, Align, FontFace, FontRegistry, LineBox, TextStyle, WhiteSpace};
pub use turbo_dom;

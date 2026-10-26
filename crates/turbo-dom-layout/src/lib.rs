//! turbo-dom layout engine.
//!
//! The native HTML/CSS layout half of turbo-dom: a style cascade over an owned node
//! tree, box generation, block/inline/flex/grid/table layout into one continuous
//! galley of positioned [`Fragment`]s, a font registry with shaping + fallback
//! (optionally with bundled OFL faces), image intrinsic sizing, and CSSOM-View
//! style layout snapshots. [`layout_html`] drives raw HTML straight to a galley.
//!
//! Relocated verbatim from `turbo-html2pdf-core` 0.3.9, which depends on this crate
//! and re-exports it at its old paths; `turbo-dom-raster` paints the galley.

#![forbid(unsafe_code)]

pub mod cmap;
pub mod error;
pub mod html_layout;
pub mod image;
pub mod layout;
pub mod layout_snapshot;
pub mod markup;
pub mod node;
pub mod perf;
pub mod style;
#[cfg(feature = "svg")]
pub mod svg;
pub mod text;

pub use error::{CompileError, Diagnostics, ErrorCode, Lint, LintCode, RenderError, Span};
pub use html_layout::{
    collect_style_css, layout_boxes, layout_html, layout_html_with_images, parse_html, CidBox,
};
pub use image::{ImageResolver, NoImages};
pub use layout::fragment::{
    BoxShadow, BreakMeta, Fragment, FragmentContent, GradientStop, ImagePlacement, LinearGradient,
    NodeId, PositionedGlyph, RepeatKind, Transform2D,
};
pub use layout::value::{BreakRule, Rgba};
pub use layout::{layout, layout_with_images, ImageCtx};
pub use node::{Attr, Element, Node, TKind, Tag};
pub use style::{
    build_cascade, build_cascade_with_width, set_media_viewport_height, style_tree, Cascade,
    ComputedStyle, StyledElement, StyledNode,
};
pub use text::{layout_text, Align, FontFace, FontRegistry, LineBox, TextStyle, WhiteSpace};

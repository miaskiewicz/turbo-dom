//! CSSOM View over a live [`LayoutSnapshot`](crate::layout_snapshot::LayoutSnapshot):
//! the pure geometry an embedder answers `getBoundingClientRect`, `getClientRects`,
//! offset/client/scroll metrics, `elementFromPoint`, caret and Range rectangles and
//! content-visibility relevance with.
//!
//! Moved from turbo-surf (`turbo-surf-render` `live_layout_*`, consolidation S4); the
//! embedder keeps everything that needs its runtime (the V8 binding, scroll state, CSS
//! `zoom` parsing, stylesheet policy). Inputs are data: a snapshot from
//! [`crate::live::snapshot`], the flat per-source styles ([`scroll::SourceStyles`],
//! with each element's effective zoom supplied by the embedder), scroll offsets and the
//! [`composition::Composition`] graph.
pub mod composition;
pub mod hit;
pub mod inline;
pub mod metrics;
pub mod range;
pub mod scroll;
pub mod text;
pub mod visibility;

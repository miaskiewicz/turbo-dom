//! The live-DOM style input ("surf mode"): style and lay out a live turbo-dom
//! [`Tree`] as an embedder composes it, without copying it into another tree.
//!
//! This is the input turbo-surf's CSSOM-View layout uses (getBoundingClientRect,
//! offset/client/scroll metrics, hit testing over the page's own document). The
//! embedder owns everything that needs its runtime — slot distribution, stylesheet
//! snapshots, selector matching with `:host`/`::slotted` scopes, CSSOM inline-style
//! overrides, inline-declaration parsing — and hands layout the results as data;
//! layout reads the [`Tree`] itself:
//!
//! - **composed children** come from the embedder's provider (`Fn(Handle) -> &[Handle]`),
//!   so slotted light children appear under their slot and shadow roots under their host;
//! - only elements and text are layout sources; comments, doctypes, processing
//!   instructions and document fragments are skipped (but still count against the node
//!   budget). Elements must be in the HTML or SVG namespace (SVG elements keep their local
//!   name in [`Tag::Html`]); the outermost `<svg>` of every composed SVG subtree is an SVG
//!   viewport;
//! - attributes are read by their **qualified** names, in attribute order;
//! - every source has a [`SourceId`] from the document number and its handle
//!   ([`source_id`]) and an index path over the kept composed children
//!   ([`SourceIdentity`]); text sources keep their original UTF-16 code units;
//! - styles come only from **pre-matched** declarations ([`PreMatched`], per rule, with the
//!   rule's shadow-scope depth), the presentational attribute hints and the inline style
//!   (optionally replaced by a CSSOM override when the element has a `style` attribute),
//!   resolved with [`resolve_matched_style`]; no marker classification.
//!
//! The work is split in two passes so a caller can capture a document's provenance
//! while it holds the DOM and lay it out later (on a cache miss): [`capture`] walks the
//! composed tree under the source budgets ([`MAX_NODES`], [`MAX_DEPTH`],
//! [`MAX_TEXT_BYTES`], [`MAX_ATTRIBUTE_BYTES`]) and records the identities, SVG viewports
//! and original text; [`snapshot`] styles the same composed tree (declaration budget
//! [`MAX_DECLARATIONS`]), runs the caller's post-cascade hook over the styled forest and
//! lays it out into a [`LayoutSnapshot`]. The tree must not change between the two.
//!
//! [`resolve_matched_style`]: crate::style::resolve_matched_style

mod capture;
mod style;

use std::collections::HashMap;

use turbo_dom::{Handle, Tree};

use crate::layout_snapshot::{
    LayoutSnapshot, NativeTextOptions, SnapshotInputs, SourceId, SourceIdentity, SourceTextUnits,
    SvgSnapshotError,
};
use crate::style::{CascadeOrigin, Declaration};
use crate::{Diagnostics, StyledNode};

pub use capture::capture;

/// Layout sources (elements, text, and the skipped comments/doctypes/PIs) per capture.
pub const MAX_NODES: usize = 50_000;
/// The deepest composed path an element may have (the document's children are at 1).
pub const MAX_DEPTH: usize = 256;
/// Total text, in bytes of the lossy UTF-8 conversion of every text source.
pub const MAX_TEXT_BYTES: usize = 2_000_000;
/// Total attribute bytes (qualified name + value, the DOM's own `style` value).
pub const MAX_ATTRIBUTE_BYTES: usize = 2_000_000;
/// Pre-matched declarations, counted once per (rule, target, declaration).
pub const MAX_DECLARATIONS: usize = 500_000;

/// Why a live capture or layout failed. The messages are part of the contract: an
/// embedder surfaces them to page script as-is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LiveError {
    #[error("native layout source exceeds node budget")]
    NodeBudget,
    #[error("native layout source exceeds depth budget")]
    DepthBudget,
    #[error("native layout source exceeds text budget")]
    TextBudget,
    #[error("native layout source exceeds attribute budget")]
    AttributeBudget,
    #[error("native layout attribute size overflow")]
    AttributeOverflow,
    #[error("native live layout does not yet support this element namespace")]
    Namespace,
    #[error("native scoped declaration budget exceeded")]
    DeclarationBudget,
    #[error("native stylesheet scope provenance missing")]
    MissingScope,
    /// An error from one of the embedder's callbacks, verbatim.
    #[error("{0}")]
    Embedder(String),
    #[error(transparent)]
    Snapshot(#[from] SvgSnapshotError),
}

/// The live source identity of `handle` in document number `document`: unique across
/// the documents of one embedder, stable while the node moves.
#[must_use]
pub fn source_id(document: u32, handle: Handle) -> SourceId {
    SourceId((u64::from(document) << 32) | u64::from(handle.raw()))
}

/// The document number of a [`source_id`].
#[must_use]
pub fn source_document(id: SourceId) -> u32 {
    (id.0 >> 32) as u32
}

/// The handle of a [`source_id`] (meaningful in its own document only).
#[must_use]
pub fn source_handle(id: SourceId) -> Handle {
    Handle::from_raw(id.0 as u32)
}

/// A live document as the embedder composes it.
#[derive(Clone, Copy)]
pub struct LiveTree<'a> {
    pub tree: &'a Tree,
    /// The embedder's document number ([`source_id`]).
    pub document: u32,
    /// The composed children of a node, in order (empty for a leaf). The walk starts
    /// at the document node.
    pub children: &'a dyn Fn(Handle) -> &'a [Handle],
}

/// What [`capture`] records about a live document.
#[derive(Debug, Clone, Default)]
pub struct LiveCapture {
    pub document: u32,
    /// The tree's version at capture time (an embedder's layout cache key).
    pub tree_version: u64,
    /// Every kept source with its composed index path (children before parents).
    pub identities: Vec<SourceIdentity>,
    /// The outermost `<svg>` element of each composed SVG subtree, in document order.
    pub svg_viewports: Vec<SourceId>,
    /// Each text source's original UTF-16 code units.
    pub original_text: HashMap<SourceId, Vec<u16>>,
    /// The document is in quirks mode (BackCompat): viewport metrics address its body.
    pub quirks: bool,
    /// `document.body`, when it is an HTML `<body>` (not a `<frameset>`).
    pub body: Option<SourceId>,
}

impl LiveCapture {
    /// A capture with no sources (a disconnected or inert document).
    #[must_use]
    pub fn empty(document: u32, tree_version: u64) -> Self {
        LiveCapture {
            document,
            tree_version,
            ..LiveCapture::default()
        }
    }
}

/// One element a pre-matched rule applies to, with the rule's highest matching
/// selector specificity for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchedTarget {
    pub source: SourceId,
    pub specificity: (u32, u32, u32),
}

/// A rule the embedder matched against its own tree scopes: its declarations, the
/// elements it applies to and the shadow-scope depth of the tree it came from (0 for the
/// document).
#[derive(Debug, Clone, PartialEq)]
pub struct PreMatched {
    pub depth: u32,
    pub declarations: Vec<Declaration>,
    pub targets: Vec<MatchedTarget>,
}

/// The rules of one cascade origin in source order. `None` is a rule the embedder has
/// no scope provenance for ([`LiveError::MissingScope`]).
pub struct OriginRules<'a> {
    pub origin: CascadeOrigin,
    pub rules: Vec<Option<&'a PreMatched>>,
}

/// Everything [`snapshot`] styles a captured live tree with.
pub struct LiveStyle<'a> {
    pub tree: LiveTree<'a>,
    /// User-agent rules first, then author rules; a rule's order is its index within
    /// its origin.
    pub origins: &'a [OriginRules<'a>],
    /// The shadow-scope depth of the tree an element belongs to (presentational hints
    /// and the inline style take it).
    pub scope_depth: &'a dyn Fn(Handle) -> Result<u32, String>,
    /// A CSSOM inline-style override; it replaces the value of an existing `style`
    /// attribute only.
    pub inline_override: &'a dyn Fn(Handle) -> Option<&'a str>,
    /// Parse an inline style into declarations.
    pub inline_declarations: &'a dyn Fn(&str) -> Result<Vec<Declaration>, String>,
}

/// Physical scale for live HTML text and SVG viewports.
pub struct LiveOptions {
    pub layout_scale: f32,
}

/// Style the captured composed tree, hand the styled forest to `post_cascade` (it may
/// rewrite used styles, validate them or collect them; its error aborts the layout) and
/// lay it out with native text provenance.
///
/// # Errors
/// The declaration budget, a missing scope, an embedder callback's error, the hook's
/// error, or the snapshot's.
pub fn snapshot<T>(
    style: &LiveStyle,
    capture: &LiveCapture,
    options: &LiveOptions,
    post_cascade: impl FnOnce(&mut [StyledNode]) -> Result<T, String>,
    inputs: &SnapshotInputs,
    diagnostics: &mut Diagnostics,
) -> Result<(LayoutSnapshot, T), LiveError> {
    let mut styled = style::style(style, capture)?;
    let collected = post_cascade(&mut styled).map_err(LiveError::Embedder)?;
    let original: Vec<_> = capture
        .original_text
        .iter()
        .map(|(&source_id, units)| SourceTextUnits { source_id, units })
        .collect();
    let options = NativeTextOptions {
        original_text: &original,
        svg_viewports: &capture.svg_viewports,
        layout_scale: options.layout_scale,
    };
    let snapshot = crate::layout_snapshot::snapshot_html_styled_tree_with_native_text(
        &styled,
        &capture.identities,
        &options,
        inputs,
        diagnostics,
    )?;
    Ok((snapshot, collected))
}

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;

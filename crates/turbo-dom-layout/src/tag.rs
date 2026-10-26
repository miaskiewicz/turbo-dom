//! Element identity in the styled tree: an element's [`Tag`] (a plain element by
//! local name, or an embedder-classified [`MarkerKind`] marker) and its [`Attr`]s.
//! Since consolidation S5 the cascade's only inputs are turbo-dom
//! [`Tree`](turbo_dom::Tree)s ([`crate::style::TreeView`], [`crate::live`]); the legacy
//! owned `Node` tree these types once belonged to (`node.rs`) is gone.

/// A name/value attribute on an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

/// An opaque, consumer-defined *marker* element kind: an element the embedder's
/// marker classifier recognized by its local name (html2pdf's `t:` paged-media
/// directives — footnotes, running regions, page counters, …). Layout treats a
/// marker as an opaque box or inline marker and never interprets `id`; the
/// consumer maps it back to its own kind. `inline` markers sit in the text flow
/// (a footnote reference, a page number); the others are block-level boxes.
///
/// Markers ignore the hidden-box checks in PDF mode, a block marker always lays out
/// as `display: block` (keeping its computed style), and type selectors never match
/// a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MarkerKind {
    /// The consumer's identifier for this kind (opaque to layout).
    pub id: u16,
    /// Whether the marker is inline-level (else block-level).
    pub inline: bool,
}

/// Classifies an element by its local name: `Some` makes it a [`Tag::Marker`].
pub type MarkerClassifier<'a> = &'a dyn Fn(&str) -> Option<MarkerKind>;

/// An element's tag: either a plain element by local name or a classified marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
    /// A normal element, by its local name (e.g. `div`, `table`, SVG `linearGradient`).
    Html(String),
    /// An element the embedder's marker classifier recognized.
    Marker(MarkerKind),
}

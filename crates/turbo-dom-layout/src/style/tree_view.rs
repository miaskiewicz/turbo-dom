//! The cascade over a turbo-dom [`Tree`] — the "html2pdf mode" style input.
//!
//! [`style_input`] styles a [`Tree`] directly and produces the [`StyledNode`] tree,
//! with a `source` handle on every element and text node. The view reproduces the
//! html5ever conversion the pre-consolidation owned-node input used (its
//! equivalence was gated through consolidation S2–S4):
//!
//! - only elements and text are content: comments, processing instructions,
//!   doctypes and `<template>` content (a document-fragment child) are skipped;
//! - attributes are read by their *local* names, in attribute order (SVG
//!   `xlink:href` is `href`);
//! - an optional marker classifier turns elements into [`Tag::Marker`]s;
//! - optionally the non-visual subtrees (`style`, `script`, `head`, …) are
//!   stripped, as the HTML drive does.
//!
//! [`Roots`] picks the scope: [`Roots::Detached`] styles a list of nodes as the
//! roots of their own flow (no ancestor contexts, a default parent style, matching
//! cut off at the scope root — template body flow, footnotes, regions), and
//! [`Roots::Document`] styles the whole document from its root element down, as a
//! browser does (the HTML path): `<html>`, `<head>` and `<body>` are real elements
//! that match, inherit and lay out like any other (the HTML drive's document UA rules
//! hide `<head>` and give `<body>` its 8px margin), and `rem` lengths resolve against
//! the root element's font size ([`root_font_scope`](crate::style::root_font_scope)).
//!
//! Selectors match through turbo-dom's one selector engine
//! ([`match_for_cascade`](turbo_dom::rtdom::query::match_for_cascade)) over the scope's
//! [`Navigator`]: kept children, sibling positions, preceding siblings, `:empty`,
//! `:root` and the scope cutoff.

use std::borrow::Cow;

use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::{Handle, Tree};

use turbo_dom::rtdom::query::{Navigator as SelectorNavigator, PositionCache, SiblingGroup};

use super::{
    resolve_style, tag_key, Attrs, Cascade, ComputedStyle, StyledElement, StyledNode, StyledText,
    Subject,
};
use crate::tag::{Attr, MarkerClassifier, MarkerKind, Tag};

/// Elements whose content is not visible page content (their bodies are CSS/JS/
/// metadata, collected separately or dropped) — removed by
/// [`TreeView::strip_non_visual`]. `<head>` is not among them: it stays in the tree
/// (so `head + body` and `body:first-child` match as in a browser) and the HTML
/// drive's document UA sheet gives it `display: none`.
fn is_non_visual(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "style" | "script" | "title" | "meta" | "link" | "noscript" | "template"
    )
}

/// A read-only view of a [`Tree`] as layout content (see the module docs).
#[derive(Clone, Copy)]
pub struct TreeView<'t> {
    tree: &'t Tree,
    strip_non_visual: bool,
    markers: Option<MarkerClassifier<'t>>,
}

impl<'t> TreeView<'t> {
    /// A view of `tree` with no marker classifier and nothing stripped.
    #[must_use]
    pub fn new(tree: &'t Tree) -> Self {
        TreeView {
            tree,
            strip_non_visual: false,
            markers: None,
        }
    }

    /// Drop the non-visual element subtrees (`style`, `script`, `title`, `meta`,
    /// `link`, `noscript`, `template`; markers are never dropped).
    #[must_use]
    pub fn strip_non_visual(mut self, strip: bool) -> Self {
        self.strip_non_visual = strip;
        self
    }

    /// Classify elements by local name into [`Tag::Marker`]s.
    #[must_use]
    pub fn markers(mut self, classify: MarkerClassifier<'t>) -> Self {
        self.markers = Some(classify);
        self
    }

    /// The viewed tree.
    #[must_use]
    pub fn tree(&self) -> &'t Tree {
        self.tree
    }

    /// The marker kind of an element, if the classifier recognizes it.
    #[must_use]
    pub fn marker(&self, h: Handle) -> Option<MarkerKind> {
        let classify = self.markers?;
        self.tree.local_name(h).and_then(classify)
    }

    /// The layout [`Tag`] of an element (`Tag::Html` of its local name unless it
    /// is a marker). Only meaningful for elements.
    #[must_use]
    pub fn tag(&self, h: Handle) -> Tag {
        match self.marker(h) {
            Some(kind) => Tag::Marker(kind),
            None => Tag::Html(self.tree.local_name(h).unwrap_or_default().to_string()),
        }
    }

    /// Whether `h` is layout content: an element (not stripped) or a text node.
    #[must_use]
    pub fn is_kept(&self, h: Handle) -> bool {
        match self.tree.node_type(h) {
            NodeType::Text => true,
            NodeType::Element => {
                !(self.strip_non_visual
                    && self.marker(h).is_none()
                    && self.tree.local_name(h).is_some_and(is_non_visual))
            }
            _ => false,
        }
    }

    /// The kept children of `h`, in order.
    #[must_use]
    pub fn children(&self, h: Handle) -> Vec<Handle> {
        let mut out = Vec::new();
        self.tree.for_each_child(h, |child| {
            if self.is_kept(child) {
                out.push(child);
            }
        });
        out
    }

    /// An element's attributes by local name, in order.
    #[must_use]
    pub fn attrs(&self, h: Handle) -> Vec<Attr> {
        self.tree
            .local_attributes(h)
            .map(|(name, value)| Attr {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect()
    }

    /// The first attribute of `h` with local name `name`.
    #[must_use]
    pub fn attr(&self, h: Handle, name: &str) -> Option<&'t str> {
        self.tree.get_attribute_local(h, name)
    }

    /// A text node's text (empty for any other node).
    #[must_use]
    pub fn text(&self, h: Handle) -> Cow<'t, str> {
        self.tree.character_data(h).unwrap_or_default()
    }

    /// Depth-first (document order) search of the whole document for the first
    /// element with local name `name`, over every child except `<template>`
    /// content — how the HTML path finds `<html>`/`<body>`.
    #[must_use]
    pub fn find_element(&self, name: &str) -> Option<Handle> {
        let mut pending = vec![self.tree.root()];
        while let Some(node) = pending.pop() {
            if node != self.tree.root()
                && self.tree.node_type(node) == NodeType::Element
                && self.tree.local_name(node) == Some(name)
            {
                return Some(node);
            }
            let mut children = Vec::new();
            self.tree.for_each_child(node, |child| {
                if self.tree.node_type(child) != NodeType::Fragment {
                    children.push(child);
                }
            });
            pending.extend(children.into_iter().rev());
        }
        None
    }

    /// The element's local name as the matcher sees it (`None` for markers, which
    /// type selectors never match).
    fn match_name(&self, h: Handle) -> Option<&'t str> {
        match self.marker(h) {
            None => self.tree.local_name(h),
            Some(_) => None,
        }
    }

    /// Whether `h` has no kept child.
    fn has_no_kept_child(&self, h: Handle) -> bool {
        let mut none = true;
        self.tree
            .for_each_child(h, |child| none &= !self.is_kept(child));
        none
    }
}

/// Which nodes a [`StyleInput`] styles, and in what matching scope.
#[derive(Clone, Copy)]
pub enum Roots<'a> {
    /// These nodes (non-content ones are skipped) as the roots of their own flow:
    /// no ancestor contexts, the default parent style, sibling positions among
    /// the kept roots.
    Detached(&'a [Handle]),
    /// The whole document from its root element (the HTML path; see the module
    /// docs).
    Document,
}

/// A [`TreeView`] plus the [`Roots`] to style.
#[derive(Clone, Copy)]
pub struct StyleInput<'a> {
    pub view: TreeView<'a>,
    pub roots: Roots<'a>,
}

/// Style-time options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StyleOptions {
    /// Accept `cmyk(...)` colours (print colour; html2pdf's `print-color`).
    pub accept_cmyk: bool,
}

/// One element's place among its kept siblings (what the structural pseudo-classes
/// read): 1-based `index` among the element siblings, `of_type_index` /
/// `of_type_total` among those with the same type key, and the element-sibling
/// count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiblingPosition {
    pub index: usize,
    pub of_type_index: usize,
    pub of_type_total: usize,
    pub siblings: usize,
}

/// The structure selector matching reads for one styled scope (a [`TreeView`] plus
/// its [`Roots`]): kept children, parents with the scope cutoff, sibling positions,
/// `:empty`, attributes. It implements turbo-dom's selector [`Navigator`
/// trait](turbo_dom::rtdom::query::Navigator), so the cascade matches through
/// [`match_for_cascade`](turbo_dom::rtdom::query::match_for_cascade) — the one selector
/// engine — with these scope rules:
///
/// - a [`Roots::Detached`] root has no parent and its siblings are the other kept roots;
/// - under [`Roots::Document`] the scope is the document: the root element has no
///   parent and is alone among its siblings (the document's only element child), so
///   `html:first-child` matches as in a browser, and every other element sits under its
///   real parent;
/// - only kept nodes exist (no comments, no template content, stripped subtrees gone);
/// - `:root` is an element named `html` (nothing in a detached flow), type selectors
///   never match a marker and `*-of-type` groups markers by kind.
///
/// Sibling positions are computed once per sibling list (turbo-dom's cascade-mode
/// [`PositionCache`]).
pub struct Navigator<'v, 't> {
    view: &'v TreeView<'t>,
    roots: Vec<Handle>,
    /// [`Roots::Document`]: the roots are the document's root element.
    document: bool,
    positions: PositionCache,
}

impl<'v, 't> Navigator<'v, 't> {
    /// The navigator of `roots` over `view`.
    #[must_use]
    pub fn new(view: &'v TreeView<'t>, roots: Roots) -> Self {
        let (roots, document) = match roots {
            Roots::Detached(roots) => (
                roots.iter().copied().filter(|&h| view.is_kept(h)).collect(),
                false,
            ),
            Roots::Document => (view.children(view.tree().root()), true),
        };
        Navigator {
            view,
            roots,
            document,
            positions: PositionCache::default(),
        }
    }

    /// The styled top-level nodes.
    #[must_use]
    pub fn roots(&self) -> &[Handle] {
        &self.roots
    }

    /// Whether `h` is the document's root element (the `<html>` of a
    /// [`Roots::Document`] scope), whose font size `rem` lengths resolve against.
    #[must_use]
    pub fn is_document_root(&self, h: Handle) -> bool {
        self.document && self.is_root(h) && self.view.tree().node_type(h) == NodeType::Element
    }

    fn is_root(&self, h: Handle) -> bool {
        self.roots.contains(&h)
    }

    /// The kept children of `h`.
    #[must_use]
    pub fn children(&self, h: Handle) -> Vec<Handle> {
        self.view.children(h)
    }

    /// The parent of `h` within the scope (`None` past the scope root).
    #[must_use]
    pub fn parent(&self, h: Handle) -> Option<Handle> {
        if self.is_root(h) {
            return None;
        }
        self.view.tree().parent(h)
    }

    /// The kept siblings of `h`, including `h`.
    #[must_use]
    pub fn siblings(&self, h: Handle) -> Vec<Handle> {
        if self.is_root(h) {
            return self.roots.clone();
        }
        self.parent(h)
            .map_or_else(|| vec![h], |parent| self.view.children(parent))
    }

    /// The structural position of element `h` (`None` for a non-element).
    #[must_use]
    pub fn position(&self, h: Handle) -> Option<SiblingPosition> {
        if self.view.tree().node_type(h) != NodeType::Element {
            return None;
        }
        let group = self.group(h);
        let (index, siblings) = group.child_position(h);
        let (of_type_index, of_type_total) = group.type_position(h);
        Some(SiblingPosition {
            index,
            of_type_index,
            of_type_total,
            siblings,
        })
    }

    /// Whether `h` matches `:empty` (no kept children).
    #[must_use]
    pub fn is_empty(&self, h: Handle) -> bool {
        self.view.has_no_kept_child(h)
    }

    /// The tag of an element node, `None` for text.
    fn tag(&self, h: Handle) -> Option<Tag> {
        (self.view.tree().node_type(h) == NodeType::Element).then(|| self.view.tag(h))
    }

    /// The element siblings of `h` with their `*-of-type` keys, positions
    /// computed once per sibling list.
    fn group(&self, h: Handle) -> std::rc::Rc<SiblingGroup> {
        self.positions.group(h, || {
            self.siblings(h)
                .into_iter()
                .filter_map(|s| self.tag(s).map(|tag| (s, tag_key(&tag))))
                .collect()
        })
    }

    /// What the non-selector cascade inputs read from element `h`.
    pub(crate) fn subject(&self, h: Handle) -> Subject<'t> {
        Subject {
            tag: self.view.match_name(h),
            attrs: Attrs::Tree(self.view.tree(), h),
        }
    }

    /// The element sibling next to `h` (`None` at the ends).
    fn sibling_at(&self, h: Handle, forward: bool) -> Option<Handle> {
        let group = self.group(h);
        let (index, count) = group.child_position(h);
        let elements = group.elements();
        match (forward, index) {
            (_, 0) => None,
            (true, i) if i < count => Some(elements[i]),
            (false, i) if i > 1 => Some(elements[i - 2]),
            _ => None,
        }
    }

    /// `:has()` search: the kept descendants of `root`.
    fn some_kept_descendant(&self, root: Handle, pred: &mut dyn FnMut(Handle) -> bool) -> bool {
        let mut stack = self.view.children(root);
        stack.reverse();
        while let Some(h) = stack.pop() {
            if self.view.tree().node_type(h) == NodeType::Element {
                if pred(h) {
                    return true;
                }
                let mut children = self.view.children(h);
                children.reverse();
                stack.extend(children);
            }
        }
        false
    }
}

impl SelectorNavigator for Navigator<'_, '_> {
    fn tree(&self) -> &Tree {
        self.view.tree()
    }
    fn parent(&self, h: Handle) -> Option<Handle> {
        Navigator::parent(self, h)
    }
    fn previous_element_sibling(&self, h: Handle) -> Option<Handle> {
        self.sibling_at(h, false)
    }
    fn next_element_sibling(&self, h: Handle) -> Option<Handle> {
        self.sibling_at(h, true)
    }
    fn element_siblings(&self, h: Handle) -> Vec<Handle> {
        self.group(h).elements().to_vec()
    }
    fn same_type(&self, a: Handle, b: Handle) -> bool {
        tag_key(&self.view.tag(a)) == tag_key(&self.view.tag(b))
    }
    fn child_position(&self, h: Handle) -> (usize, usize) {
        self.position(h).map_or((0, 0), |p| (p.index, p.siblings))
    }
    fn type_position(&self, h: Handle) -> (usize, usize) {
        self.position(h)
            .map_or((0, 0), |p| (p.of_type_index, p.of_type_total))
    }
    fn type_name(&self, h: Handle) -> Option<&str> {
        self.view.match_name(h)
    }
    fn is_root(&self, h: Handle) -> bool {
        self.view.match_name(h) == Some("html")
    }
    fn is_empty(&self, h: Handle) -> bool {
        Navigator::is_empty(self, h)
    }
    fn some_descendant(&self, root: Handle, pred: &mut dyn FnMut(Handle) -> bool) -> bool {
        self.some_kept_descendant(root, pred)
    }
}

/// Style a [`StyleInput`] under `cascade` into the styled tree (see the module
/// docs).
#[must_use]
pub fn style_input(
    input: &StyleInput,
    cascade: &Cascade,
    options: &StyleOptions,
) -> Vec<StyledNode> {
    let _cmyk = crate::layout::value::cmyk_scope(options.accept_cmyk);
    style_scope(&Navigator::new(&input.view, input.roots), cascade)
}

/// Style a navigator's roots (each detached root inherits the default style).
pub(crate) fn style_scope(nav: &Navigator, cascade: &Cascade) -> Vec<StyledNode> {
    style_siblings(nav, nav.roots(), &ComputedStyle::default(), cascade)
}

fn style_siblings(
    nav: &Navigator,
    nodes: &[Handle],
    parent: &ComputedStyle,
    cascade: &Cascade,
) -> Vec<StyledNode> {
    let view = nav.view;
    let mut out = Vec::with_capacity(nodes.len());
    for &h in nodes {
        let Some(tag) = nav.tag(h) else {
            out.push(StyledNode::Text(StyledText {
                text: view.text(h).into_owned(),
                source: Some(h),
            }));
            continue;
        };
        let style = resolve_style(nav, h, parent, cascade);
        // The root element's font size is what `rem` means below it.
        let _root_font = nav
            .is_document_root(h)
            .then(|| crate::style::root_font_scope(super::computed_font_px(&style)));
        let children = nav.children(h);
        let styled_children = style_siblings(nav, &children, &style, cascade);
        out.push(StyledNode::Element(StyledElement {
            tag,
            attrs: view.attrs(h),
            style,
            children: styled_children,
            source: Some(h),
        }));
    }
    out
}

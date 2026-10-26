//! The cascade over a turbo-dom [`Tree`] — the "html2pdf mode" style input.
//!
//! [`style_input`] styles a [`Tree`] directly (no copy into the legacy
//! [`Node`](crate::Node) tree) and produces exactly the [`StyledNode`] tree the
//! legacy input produced for the same document, plus a `source` handle on every
//! element and text node. The view reproduces the legacy html5ever conversion:
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
//! [`Roots::Shell`] styles the `<body>` content under match-only `<html>`/`<body>`
//! shells exactly as the legacy HTML path did (the shells sit at the root
//! position with no preceding siblings and match `:empty`; only body content is
//! styled; content positions are computed after stripping and comment removal).
//!
//! The selector matcher reads its structure — kept children, sibling positions,
//! preceding siblings, `:empty`, attributes — through [`TreeView`]; this is the
//! seam the shared turbo-dom selector engine plugs into later.

use std::borrow::Cow;

use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::{Handle, Tree};

use super::{
    ctx_from, positions_from_keys, resolve_style, root_pos, tag_key, Attrs, Cascade, ComputedStyle,
    Ctx, ElemPos, StyledElement, StyledNode, StyledText,
};
use crate::node::{Attr, Element, MarkerClassifier, MarkerKind, Node, Tag};

/// Elements whose content is not visible page content (their bodies are CSS/JS/
/// metadata, collected separately or dropped) — removed by
/// [`TreeView::strip_non_visual`].
pub(crate) fn is_non_visual(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "style" | "script" | "head" | "title" | "meta" | "link" | "noscript" | "template"
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

    /// Drop the non-visual element subtrees (`style`, `script`, `head`, `title`,
    /// `meta`, `link`, `noscript`, `template`; markers are never dropped).
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
    /// content — the lookup the legacy conversion used to find `<html>`/`<body>`.
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

    /// The legacy [`Node`] copy of the kept content under `roots` (kept roots only):
    /// what the legacy conversion produced for the same document. A bridge for
    /// embedders and tests that still consume [`Node`]s.
    #[must_use]
    pub fn to_nodes(&self, roots: &[Handle]) -> Vec<Node> {
        roots
            .iter()
            .filter(|&&h| self.is_kept(h))
            .map(|&h| self.node_copy(h))
            .collect()
    }

    fn node_copy(&self, h: Handle) -> Node {
        if self.tree.node_type(h) == NodeType::Text {
            return Node::Text(self.text(h).into_owned());
        }
        Node::Element(Element {
            tag: self.tag(h),
            attrs: self.attrs(h),
            children: self.to_nodes(&self.children(h)),
        })
    }

    /// The element's local name as the matcher sees it (`None` for markers, which
    /// type selectors never match).
    fn match_name(&self, h: Handle, tag: &Tag) -> Option<&'t str> {
        match tag {
            Tag::Html(_) => self.tree.local_name(h),
            Tag::Marker(_) => None,
        }
    }

    fn ctx(&self, h: Handle, tag: &Tag, pos: ElemPos, empty: bool, prev: Vec<Ctx<'t>>) -> Ctx<'t> {
        ctx_from(
            self.match_name(h, tag),
            Attrs::Tree(self.tree, h),
            pos,
            empty,
            prev,
        )
    }
}

/// Which nodes a [`StyleInput`] styles, and in what matching scope.
#[derive(Clone, Copy)]
pub enum Roots<'a> {
    /// These nodes (non-content ones are skipped) as the roots of their own flow:
    /// no ancestor contexts, the default parent style, sibling positions among
    /// the kept roots.
    Detached(&'a [Handle]),
    /// The document's `<body>` content under match-only `<html>`/`<body>` shells
    /// (the legacy HTML path; see the module docs).
    Shell,
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

/// Style a [`StyleInput`] under `cascade` into the styled tree (see the module
/// docs). Equal — up to the `source` fields — to the legacy [`style_tree`] /
/// [`style_tree_with_roots`] over the legacy conversion of the same document.
///
/// [`style_tree`]: super::style_tree
/// [`style_tree_with_roots`]: super::style_tree_with_roots
#[must_use]
pub fn style_input(
    input: &StyleInput,
    cascade: &Cascade,
    options: &StyleOptions,
) -> Vec<StyledNode> {
    let _cmyk = crate::layout::value::cmyk_scope(options.accept_cmyk);
    let view = &input.view;
    match input.roots {
        Roots::Detached(roots) => {
            let nodes: Vec<Handle> = roots.iter().copied().filter(|&h| view.is_kept(h)).collect();
            style_siblings(view, &nodes, &[], &ComputedStyle::default(), cascade)
        }
        Roots::Shell => style_shell(view, cascade),
    }
}

fn style_shell(view: &TreeView, cascade: &Cascade) -> Vec<StyledNode> {
    let body = view
        .find_element("body")
        .expect("the HTML parser always produces a <body>");
    let shells: Vec<Handle> = view
        .find_element("html")
        .into_iter()
        .chain(std::iter::once(body))
        .collect();
    // The shells are plain elements at the root position, with no children and no
    // preceding siblings, whatever the document holds around them.
    let ancestors: Vec<Ctx> = shells
        .iter()
        .map(|&h| {
            ctx_from(
                view.tree.local_name(h),
                Attrs::Tree(view.tree, h),
                root_pos(),
                true,
                Vec::new(),
            )
        })
        .collect();
    let mut parent = ComputedStyle::default();
    for i in 0..ancestors.len() {
        parent = resolve_style(&ancestors[..=i], &parent, cascade);
    }
    style_siblings(view, &view.children(body), &ancestors, &parent, cascade)
}

fn style_siblings<'t>(
    view: &TreeView<'t>,
    nodes: &[Handle],
    ancestors: &[Ctx<'t>],
    parent: &ComputedStyle,
    cascade: &Cascade,
) -> Vec<StyledNode> {
    let tags: Vec<Option<Tag>> = nodes
        .iter()
        .map(|&h| (view.tree.node_type(h) == NodeType::Element).then(|| view.tag(h)))
        .collect();
    let keys: Vec<Option<String>> = tags.iter().map(|tag| tag.as_ref().map(tag_key)).collect();
    let positions = positions_from_keys(&keys);
    // Each preceding element sibling's (prev-less) context, for `+`/`~`.
    let mut prev: Vec<Ctx<'t>> = Vec::new();
    let mut out = Vec::with_capacity(nodes.len());
    for ((&h, tag), pos) in nodes.iter().zip(tags).zip(positions) {
        let Some(tag) = tag else {
            out.push(StyledNode::Text(StyledText {
                text: view.text(h).into_owned(),
                source: Some(h),
            }));
            continue;
        };
        let pos = pos.expect("element has a position");
        let children = view.children(h);
        let empty = children.is_empty();
        let sibling = view.ctx(h, &tag, pos, empty, Vec::new());
        let mut path = ancestors.to_vec();
        path.push(view.ctx(h, &tag, pos, empty, prev.clone()));
        let style = resolve_style(&path, parent, cascade);
        let styled_children = style_siblings(view, &children, &path, &style, cascade);
        out.push(StyledNode::Element(StyledElement {
            tag,
            attrs: view.attrs(h),
            style,
            children: styled_children,
            source: Some(h),
        }));
        prev.push(sibling);
    }
    out
}

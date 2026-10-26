//! The capture pass: walk the composed tree under the source budgets and record each
//! kept source's identity, the SVG viewports and the original text.
//!
//! Budget order is part of the contract: a node is counted before its type is looked
//! at; an element's namespace is checked, then its attributes, then its children (whose
//! depth is checked first); text is measured on its lossy UTF-8 conversion. The first
//! violation in composed document order is reported.
use std::collections::HashMap;

use turbo_dom::rtdom::tree::NodeType;
use turbo_dom::{Handle, Tree};

use super::{
    source_id, LiveCapture, LiveError, LiveTree, MAX_ATTRIBUTE_BYTES, MAX_DEPTH, MAX_NODES,
    MAX_TEXT_BYTES,
};
use crate::layout_snapshot::{SourceId, SourceIdentity};

const HTML: &str = "http://www.w3.org/1999/xhtml";
const SVG: &str = "http://www.w3.org/2000/svg";

/// Capture a live document's layout provenance (see the [module docs](super)).
///
/// # Errors
/// The first budget violation or unsupported element namespace in composed order.
pub fn capture(live: &LiveTree) -> Result<LiveCapture, LiveError> {
    let mut walk = Walk {
        live,
        identities: Vec::new(),
        svg_viewports: Vec::new(),
        original_text: HashMap::new(),
        nodes_left: MAX_NODES,
        text_left: MAX_TEXT_BYTES,
        attributes_left: MAX_ATTRIBUTE_BYTES,
    };
    walk.children(live.tree.root(), &[], false)?;
    Ok(LiveCapture {
        document: live.document,
        tree_version: live.tree.version,
        identities: walk.identities,
        svg_viewports: walk.svg_viewports,
        original_text: walk.original_text,
        quirks: live.tree.compat_mode() == "BackCompat",
        body: document_body(live.tree).map(|body| source_id(live.document, body)),
    })
}

/// `document.body` when it is an HTML `<body>`: the first `body` or `frameset` child of
/// the HTML root element, if that is a `body`.
fn document_body(tree: &Tree) -> Option<Handle> {
    let html = tree
        .children(tree.root())
        .into_iter()
        .find(|&node| tree.node_type(node) == NodeType::Element)
        .filter(|&node| is_html(tree, node, &["html"]))?;
    tree.children(html)
        .into_iter()
        .find(|&node| is_html(tree, node, &["body", "frameset"]))
        .filter(|&node| tree.local_name(node) == Some("body"))
}

fn is_html(tree: &Tree, node: Handle, names: &[&str]) -> bool {
    tree.node_type(node) == NodeType::Element
        && tree
            .local_name(node)
            .is_some_and(|name| names.contains(&name))
        && tree.namespace_uri(node) == Some(HTML)
}

struct Walk<'l, 'a> {
    live: &'l LiveTree<'a>,
    identities: Vec<SourceIdentity>,
    svg_viewports: Vec<SourceId>,
    original_text: HashMap<SourceId, Vec<u16>>,
    nodes_left: usize,
    text_left: usize,
    attributes_left: usize,
}

impl Walk<'_, '_> {
    fn children(&mut self, parent: Handle, path: &[usize], in_svg: bool) -> Result<(), LiveError> {
        if path.len() > MAX_DEPTH {
            return Err(LiveError::DepthBudget);
        }
        let mut kept = 0;
        for &child in (self.live.children)(parent) {
            let mut child_path = path.to_vec();
            child_path.push(kept);
            if self.node(child, &child_path, in_svg)? {
                kept += 1;
            }
        }
        Ok(())
    }

    /// Count `node` and capture it; `false` for a node that is not a layout source.
    fn node(&mut self, node: Handle, path: &[usize], in_svg: bool) -> Result<bool, LiveError> {
        self.nodes_left = self
            .nodes_left
            .checked_sub(1)
            .ok_or(LiveError::NodeBudget)?;
        match self.live.tree.node_type(node) {
            NodeType::Element => self.element(node, path, in_svg)?,
            NodeType::Text => self.text(node)?,
            _ => return Ok(false),
        }
        self.identities.push(SourceIdentity {
            id: self.id(node),
            path: path.to_vec(),
        });
        Ok(true)
    }

    fn id(&self, node: Handle) -> SourceId {
        source_id(self.live.document, node)
    }

    fn element(&mut self, node: Handle, path: &[usize], in_svg: bool) -> Result<(), LiveError> {
        let svg = svg_namespace(self.live.tree, node)?;
        self.note_viewport(node, svg, in_svg);
        self.attributes(node)?;
        self.children(node, path, in_svg || svg)
    }

    /// The outermost `<svg>` element of an SVG subtree is an SVG viewport.
    fn note_viewport(&mut self, node: Handle, svg: bool, in_svg: bool) {
        if svg && !in_svg && self.live.tree.local_name(node) == Some("svg") {
            self.svg_viewports.push(self.id(node));
        }
    }

    /// Charge every attribute's qualified name and DOM value.
    fn attributes(&mut self, node: Handle) -> Result<(), LiveError> {
        let mut result = Ok(());
        let left = &mut self.attributes_left;
        self.live.tree.for_each_attr(node, |name, value| {
            if result.is_ok() {
                result = charge(left, name.len(), value.len());
            }
        });
        result
    }

    fn text(&mut self, node: Handle) -> Result<(), LiveError> {
        let units = self.live.tree.node_value_utf16(node).unwrap_or_default();
        let bytes = lossy_utf8_len(&units);
        self.original_text.insert(self.id(node), units);
        self.text_left = self
            .text_left
            .checked_sub(bytes)
            .ok_or(LiveError::TextBudget)?;
        Ok(())
    }
}

/// Whether an element is in the SVG namespace; anything but HTML and SVG is rejected.
fn svg_namespace(tree: &Tree, node: Handle) -> Result<bool, LiveError> {
    match tree.namespace_uri(node) {
        Some(HTML) => Ok(false),
        Some(SVG) => Ok(true),
        _ => Err(LiveError::Namespace),
    }
}

pub(super) fn charge(left: &mut usize, name: usize, value: usize) -> Result<(), LiveError> {
    let length = name
        .checked_add(value)
        .ok_or(LiveError::AttributeOverflow)?;
    *left = left.checked_sub(length).ok_or(LiveError::AttributeBudget)?;
    Ok(())
}

/// The UTF-8 length of `String::from_utf16_lossy(units)` (an unpaired surrogate is
/// U+FFFD, three bytes), without building the string.
fn lossy_utf8_len(units: &[u16]) -> usize {
    char::decode_utf16(units.iter().copied())
        .map(|c| c.map_or(3, char::len_utf8))
        .sum()
}

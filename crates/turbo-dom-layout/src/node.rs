//! The legacy owned node tree: the input the cascade consumed before it could
//! style a turbo-dom [`Tree`](turbo_dom::Tree) directly (see
//! [`crate::style::TreeView`] and the live-DOM input [`crate::live`]). No production
//! path builds one any more (turbo-surf's live-layout copier is gone since
//! consolidation S4); it stays as the reference the Tree path is checked against and
//! for turbo-html2pdf-core's re-exports and Node-literal tests (`tree_from_nodes`).
//!
//! [`Tag`], [`Attr`] and [`MarkerKind`] are not legacy: the styled tree uses them.

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

/// A styled-but-unpositioned element node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub tag: Tag,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
}

impl Element {
    /// The value of the named attribute, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }
}

/// A node in the resolved tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

impl Node {
    /// Borrow the element if this node is one.
    pub fn as_element(&self) -> Option<&Element> {
        match self {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        }
    }

    /// Borrow the text if this node is a text node.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Node::Text(t) => Some(t),
            Node::Element(_) => None,
        }
    }
}

/// A turbo-dom copy of a legacy [`Node`] forest (see [`tree_from_nodes`]).
pub struct NodeTree {
    /// The document holding the copy (the nodes sit under its `<body>`).
    pub tree: turbo_dom::Tree,
    /// One handle per top-level node, in order.
    pub roots: Vec<turbo_dom::Handle>,
    /// Every copied node by its index path from the forest root (`[i]` is
    /// `nodes[i]`, `[i, j]` its `j`-th child, …).
    pub paths: std::collections::HashMap<Vec<usize>, turbo_dom::Handle>,
}

/// LEGACY bridge: copy a [`Node`] forest into a turbo-dom [`Tree`] so code (and
/// tests) written against [`Node`] literals can drive the Tree input. Elements keep
/// their exact local names (HTML namespace) and attributes in order; each
/// [`Tag::Marker`] becomes an element named `marker_name(kind)` (pair it with a
/// classifier mapping that name back). Text nodes stay separate, as in the forest.
/// Duplicate attribute names on one element collapse to the last value.
#[must_use]
pub fn tree_from_nodes(nodes: &[Node], marker_name: &dyn Fn(MarkerKind) -> String) -> NodeTree {
    let tree = turbo_dom::Tree::parse("");
    let body = tree
        .handles()
        .find(|&h| tree.local_name(h) == Some("body"))
        .expect("an empty document has a <body>");
    let mut out = NodeTree {
        tree,
        roots: Vec::new(),
        paths: std::collections::HashMap::new(),
    };
    for (i, node) in nodes.iter().enumerate() {
        let root = copy_node(&mut out, node, vec![i], marker_name);
        out.tree.append_child(body, root);
        out.roots.push(root);
    }
    out
}

fn copy_node(
    out: &mut NodeTree,
    node: &Node,
    path: Vec<usize>,
    marker_name: &dyn Fn(MarkerKind) -> String,
) -> turbo_dom::Handle {
    let handle = match node {
        Node::Text(text) => out.tree.create_text_node(text),
        Node::Element(element) => {
            let name = match &element.tag {
                Tag::Html(name) => name.clone(),
                Tag::Marker(kind) => marker_name(*kind),
            };
            let h = out
                .tree
                .create_element_ns(&name, turbo_dom::rtdom::tree::Namespace::Html);
            for attr in &element.attrs {
                out.tree.set_attribute(h, &attr.name, &attr.value);
            }
            for (j, child) in element.children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(j);
                let child = copy_node(out, child, child_path, marker_name);
                out.tree.append_child(h, child);
            }
            h
        }
    };
    out.paths.insert(path, handle);
    handle
}

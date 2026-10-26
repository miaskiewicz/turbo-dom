//! Test fixtures shared by this crate's integration tests and downstream crates'
//! tests (turbo-html2pdf-core) — the `test-support` feature. **Never enable it in a
//! production build.**
//!
//! - Font fixtures read from this crate's `assets/fonts/` ([`FIXTURE_FONTS_DIR`]: the
//!   bundled OFL faces plus the test-only Evolventa / Go / Sztylet / Warsaw Gothic
//!   faces), so no downstream crate keeps its own copy of the font files.
//! - [`style`]: a [`ComputedStyle`] from `(property, value)` pairs.
//! - [`literal`]: tree literals for tests that describe their input as nested
//!   element/text values. A literal only ever *builds a turbo-dom [`Tree`]*
//!   ([`literal::tree_from_nodes`]); layout's inputs are Trees (the owned-node cascade
//!   input this shape once fed was retired in consolidation S5).
//!
//! [`Tree`]: turbo_dom::Tree

use crate::text::FontRegistry;
use crate::{ComputedStyle, FontFace};

/// This crate's font directory (`assets/fonts`): the bundled faces and the test-only
/// fixture faces. Resolved at compile time from the crate's own manifest directory,
/// so it works for a path, git or registry dependency alike.
pub const FIXTURE_FONTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");

/// The bytes of the font fixture `name` (a path relative to [`FIXTURE_FONTS_DIR`]).
///
/// # Panics
/// If the file is missing.
#[must_use]
pub fn font_bytes(name: &str) -> Vec<u8> {
    let path = format!("{FIXTURE_FONTS_DIR}/{name}");
    std::fs::read(&path).unwrap_or_else(|_| panic!("fixture {path}"))
}

/// Evolventa Regular (400, upright).
#[must_use]
pub fn evolventa() -> FontFace {
    FontFace::from_bytes(font_bytes("Evolventa-zLXL.ttf"), "Evolventa", 400, false)
        .expect("Evolventa parses")
}

/// Evolventa Bold (700, upright).
#[must_use]
pub fn evolventa_bold() -> FontFace {
    FontFace::from_bytes(
        font_bytes("EvolventaBold-55Xv.ttf"),
        "Evolventa",
        700,
        false,
    )
    .expect("Evolventa Bold parses")
}

/// Go Regular (400, upright).
#[must_use]
pub fn go() -> FontFace {
    FontFace::from_bytes(font_bytes("Go-Regular.ttf"), "Go", 400, false).expect("Go parses")
}

/// A registry with Evolventa (regular + bold) and Go registered.
#[must_use]
pub fn registry() -> FontRegistry {
    let mut reg = FontRegistry::new();
    reg.add(evolventa());
    reg.add(evolventa_bold());
    reg.add(go());
    reg
}

/// Build a [`ComputedStyle`] from `(property, value)` pairs.
#[must_use]
pub fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
    ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

/// Tree literals: nested element/text values that build a turbo-dom [`Tree`].
///
/// [`Tree`]: turbo_dom::Tree
pub mod literal {
    use std::collections::HashMap;

    use turbo_dom::{Handle, Tree};

    use crate::style::{style_input, Cascade, Roots, StyleInput, StyleOptions, TreeView};
    use crate::tag::{Attr, MarkerClassifier, MarkerKind, Tag};
    use crate::StyledNode;

    /// An element literal.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Element {
        pub tag: Tag,
        pub attrs: Vec<Attr>,
        pub children: Vec<Node>,
    }

    impl Element {
        /// The value of the named attribute, if present.
        #[must_use]
        pub fn attr(&self, name: &str) -> Option<&str> {
            self.attrs
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.value.as_str())
        }
    }

    /// An element or text literal.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Node {
        Element(Element),
        Text(String),
    }

    impl Node {
        /// Borrow the element if this literal is one.
        #[must_use]
        pub fn as_element(&self) -> Option<&Element> {
            match self {
                Node::Element(e) => Some(e),
                Node::Text(_) => None,
            }
        }

        /// Borrow the text if this literal is a text node.
        #[must_use]
        pub fn as_text(&self) -> Option<&str> {
            match self {
                Node::Text(t) => Some(t),
                Node::Element(_) => None,
            }
        }
    }

    /// The element name a [`Tag::Marker`] literal gets in its [`Tree`]
    /// (`x-marker-<id>-<i|b>`), the inverse of [`marker_of`].
    #[must_use]
    pub fn marker_name(kind: MarkerKind) -> String {
        format!(
            "x-marker-{}-{}",
            kind.id,
            if kind.inline { "i" } else { "b" }
        )
    }

    /// The marker classifier for [`marker_name`]d elements.
    #[must_use]
    pub fn marker_of(local: &str) -> Option<MarkerKind> {
        let rest = local.strip_prefix("x-marker-")?;
        let (id, display) = rest.split_once('-')?;
        Some(MarkerKind {
            id: id.parse().ok()?,
            inline: display == "i",
        })
    }

    /// The turbo-dom document built from a literal forest (see [`tree_from_nodes`]).
    pub struct NodeTree {
        /// The document holding the content (the nodes sit under its `<body>`).
        pub tree: Tree,
        /// One handle per top-level literal, in order.
        pub roots: Vec<Handle>,
        /// Every built node by its index path from the forest root (`[i]` is
        /// `nodes[i]`, `[i, j]` its `j`-th child, …).
        pub paths: HashMap<Vec<usize>, Handle>,
    }

    impl NodeTree {
        /// The forest as a detached style input with the default [`marker_of`]
        /// classifier (pair with [`tree`]).
        #[must_use]
        pub fn input(&self) -> StyleInput<'_> {
            self.input_with(&marker_of)
        }

        /// The forest as a detached style input with marker classifier `classify`.
        #[must_use]
        pub fn input_with<'a>(&'a self, classify: MarkerClassifier<'a>) -> StyleInput<'a> {
            StyleInput {
                view: TreeView::new(&self.tree).markers(classify),
                roots: Roots::Detached(&self.roots),
            }
        }
    }

    /// Build a literal forest into a fresh turbo-dom [`Tree`] under its `<body>`.
    /// Elements keep their exact local names (HTML namespace) and attributes in
    /// order; each [`Tag::Marker`] becomes an element named `marker_name(kind)` (pair
    /// it with a classifier mapping that name back). Text literals stay separate text
    /// nodes. Duplicate attribute names on one element collapse to the last value.
    ///
    /// # Panics
    /// Never in practice: an empty document always has a `<body>`.
    #[must_use]
    pub fn tree_from_nodes(nodes: &[Node], marker_name: &dyn Fn(MarkerKind) -> String) -> NodeTree {
        let tree = Tree::parse("");
        let body = tree
            .handles()
            .find(|&h| tree.local_name(h) == Some("body"))
            .expect("an empty document has a <body>");
        let mut out = NodeTree {
            tree,
            roots: Vec::new(),
            paths: HashMap::new(),
        };
        for (i, node) in nodes.iter().enumerate() {
            let root = build(&mut out, node, vec![i], marker_name);
            out.tree.append_child(body, root);
            out.roots.push(root);
        }
        out
    }

    /// [`tree_from_nodes`] with the default [`marker_name`] scheme.
    #[must_use]
    pub fn tree(nodes: &[Node]) -> NodeTree {
        tree_from_nodes(nodes, &marker_name)
    }

    fn build(
        out: &mut NodeTree,
        node: &Node,
        path: Vec<usize>,
        marker_name: &dyn Fn(MarkerKind) -> String,
    ) -> Handle {
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
                    let child = build(out, child, child_path, marker_name);
                    out.tree.append_child(h, child);
                }
                h
            }
        };
        out.paths.insert(path, handle);
        handle
    }

    /// The literal copy of the kept content under `roots` of `view` (elements and
    /// text; markers as [`Tag::Marker`]) — for tests that splice parsed HTML into a
    /// literal or edit it before building it back into a [`Tree`].
    #[must_use]
    pub fn from_tree(view: &TreeView, roots: &[Handle]) -> Vec<Node> {
        roots
            .iter()
            .filter(|&&h| view.is_kept(h))
            .map(|&h| {
                if view.tree().node_type(h) == turbo_dom::rtdom::tree::NodeType::Text {
                    return Node::Text(view.text(h).into_owned());
                }
                Node::Element(Element {
                    tag: view.tag(h),
                    attrs: view.attrs(h),
                    children: from_tree(view, &view.children(h)),
                })
            })
            .collect()
    }

    /// The `<body>` content of `html` (parsed as the HTML drive parses,
    /// [`crate::parse_html_tree`]) as literals. Never fails (the `Result` matches the
    /// fallible parsers tests chain it with).
    ///
    /// # Errors
    /// None: HTML parsing always succeeds.
    ///
    /// # Panics
    /// Never in practice: a parsed document always has a `<body>`.
    pub fn parse_html(html: &str) -> Result<Vec<Node>, crate::RenderError> {
        let tree = crate::parse_html_tree(html);
        let view = TreeView::new(&tree);
        let body = view
            .find_element("body")
            .expect("a parsed document has a <body>");
        Ok(from_tree(&view, &view.children(body)))
    }

    /// Style a literal forest as a detached flow (default [`StyleOptions`], default
    /// marker scheme).
    #[must_use]
    pub fn style_nodes(nodes: &[Node], cascade: &Cascade) -> Vec<StyledNode> {
        style_input(&tree(nodes).input(), cascade, &StyleOptions::default())
    }
}

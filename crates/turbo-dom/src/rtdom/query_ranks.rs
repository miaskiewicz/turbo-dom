//! Raw sibling ranks shared by live selector checks within one tree version.
use super::super::tree::{Handle, Namespace, NodeType, Tree};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Default)]
pub(super) struct Position {
    pub child: (usize, usize),
    pub of_type: (usize, usize),
}

#[derive(Default)]
pub(crate) struct SiblingRanks {
    version: u64,
    parents: FxHashMap<Handle, FxHashMap<Handle, Position>>,
}

impl SiblingRanks {
    pub(super) fn position(&mut self, tree: &Tree, node: Handle) -> Position {
        let Some(parent) = tree.parent(node) else {
            return Position {
                child: (1, 1),
                of_type: (1, 1),
            };
        };
        // Every structural/name change advances this version, including detached trees.
        // Cache ranks only: filtered nth-child and state predicates still read live state.
        if self.version != tree.version {
            self.parents.clear();
            self.version = tree.version;
        }
        let positions = self
            .parents
            .entry(parent)
            .or_insert_with(|| build(tree, parent));
        positions.get(&node).copied().unwrap_or(Position {
            child: (0, positions.len()),
            of_type: (0, 0),
        })
    }
}

#[derive(Hash, PartialEq, Eq)]
enum Name<'a> {
    Scalar(&'a str),
    Units(Option<Vec<u16>>),
}

#[derive(Hash, PartialEq, Eq)]
struct ElementType<'a> {
    namespace: u8,
    uri: Option<Vec<u16>>,
    name: Name<'a>,
}

impl<'a> ElementType<'a> {
    fn new(tree: &'a Tree, node: Handle) -> Self {
        let namespace = tree.namespace(node);
        Self {
            namespace: namespace as u8,
            uri: (namespace == Namespace::Other)
                .then(|| tree.namespace_uri_utf16(node))
                .flatten(),
            name: tree
                .local_name_scalar(node)
                .map_or_else(|| Name::Units(tree.local_name_utf16(node)), Name::Scalar),
        }
    }
}

fn build(tree: &Tree, parent: Handle) -> FxHashMap<Handle, Position> {
    let mut positions = FxHashMap::default();
    let mut groups: FxHashMap<ElementType<'_>, Vec<Handle>> = FxHashMap::default();
    tree.for_each_child(parent, |node| {
        if tree.node_type(node) == NodeType::Element {
            let index = positions.len() + 1;
            positions.insert(
                node,
                Position {
                    child: (index, 0),
                    of_type: (0, 0),
                },
            );
            groups
                .entry(ElementType::new(tree, node))
                .or_default()
                .push(node);
        }
    });
    let count = positions.len();
    for group in groups.values() {
        for (index, node) in group.iter().enumerate() {
            // Each group member was inserted into positions by the same child walk.
            let position = positions.get_mut(node).unwrap();
            position.child.1 = count;
            position.of_type = (index + 1, group.len());
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::super::{LiveNavigator, Navigator, ScopedNavigator};
    use super::{Handle, Namespace, Tree};

    #[test]
    fn cached_ranks_equal_default_navigation_across_element_identities() {
        // Keep the public trait's uncached rank implementation as an independent
        // reference, including the scoped navigator's element-type delegation.
        struct Uncached<'a>(ScopedNavigator<'a>);
        impl Navigator for Uncached<'_> {
            fn tree(&self) -> &Tree {
                self.0.tree()
            }
            fn parent(&self, h: Handle) -> Option<Handle> {
                self.0.parent(h)
            }
            fn previous_element_sibling(&self, h: Handle) -> Option<Handle> {
                self.0.previous_element_sibling(h)
            }
            fn next_element_sibling(&self, h: Handle) -> Option<Handle> {
                self.0.next_element_sibling(h)
            }
            fn element_siblings(&self, h: Handle) -> Vec<Handle> {
                self.0.element_siblings(h)
            }
            fn same_type(&self, a: Handle, b: Handle) -> bool {
                self.0.same_type(a, b)
            }
            fn type_name(&self, h: Handle) -> Option<&str> {
                self.0.type_name(h)
            }
            fn is_root(&self, h: Handle) -> bool {
                self.0.is_root(h)
            }
            fn is_empty(&self, h: Handle) -> bool {
                self.0.is_empty(h)
            }
            fn some_descendant(&self, root: Handle, pred: &mut dyn FnMut(Handle) -> bool) -> bool {
                self.0.some_descendant(root, pred)
            }
        }
        let mut tree = Tree::parse("<main><span></span>text<b></b><span></span></main>");
        let scope = tree.query_selector("main").unwrap();
        let nodes = [
            tree.create_element_ns("x", Namespace::Html),
            tree.create_element_ns("x", Namespace::Svg),
            tree.create_element_namespace(Some("urn:a"), "p:x"),
            tree.create_element_namespace(Some("urn:a"), "q:x"),
            tree.create_element_namespace(Some("urn:b"), "x"),
            tree.create_element_ns_units(&[0xd800], Namespace::Html),
            tree.create_element_ns_units(&[0xd800], Namespace::Html),
            tree.create_element_ns_units(&[0xd801], Namespace::Html),
            tree.create_element_ns_units(&[0xfffd], Namespace::Html),
        ];
        for node in nodes {
            tree.append_child(scope, node);
        }
        let detached = tree.create_element("span");
        let mut checks = tree.children(scope);
        checks.push(detached);
        for _ in 0..2 {
            let cached = LiveNavigator(&tree);
            let reference = Uncached(ScopedNavigator {
                live: cached,
                scope,
            });
            for &node in &checks {
                assert_eq!(cached.child_position(node), reference.child_position(node));
                assert_eq!(cached.type_position(node), reference.type_position(node));
            }
            tree.insert_before(scope, nodes[6], Some(nodes[0]));
        }
    }
}

//! Native composition and selector scope are separate graphs: slot placement
//! changes physical ancestry, while author selectors keep the original DOM.
use std::collections::{BTreeMap, HashMap};
use turbo_dom_parser::rtdom::tree::Handle;
use turbo_dom_parser::rtdom::Tree;

#[derive(Default, Clone)]
pub(crate) struct Composition {
    pub(crate) children: HashMap<Handle, Vec<Handle>>,
    pub(crate) parents: HashMap<Handle, Handle>,
    pub(crate) scopes: HashMap<Handle, Handle>,
    pub(crate) hosts: BTreeMap<Handle, Handle>,
    pub(crate) depths: HashMap<Handle, u32>,
    pub(crate) ua_rules: Vec<crate::live_stylesheets::ActiveRule>,
    pub(crate) inline_overrides: HashMap<Handle, String>,
}
impl Composition {
    pub(crate) fn scopes(tree: &Tree) -> Result<Self, String> {
        let mut graph = Self::default();
        let mut pending = vec![(tree.root(), tree.root(), 0, 0)];
        while let Some((node, root, scope_depth, depth)) = pending.pop() {
            if depth > 256 || graph.scopes.len() >= 50_000 {
                return Err("native layout scope graph exceeds source budget".into());
            }
            if graph.scopes.insert(node, root).is_some() {
                return Err("native layout scope graph repeats a source".into());
            }
            graph.depths.insert(root, scope_depth);
            for child in tree.children(node).into_iter().rev() {
                pending.push((child, root, scope_depth, depth + 1));
            }
            if let Some(shadow) = tree.shadow_root(node) {
                graph.hosts.insert(shadow, node);
                pending.push((shadow, shadow, scope_depth + 1, depth + 1));
            }
        }
        Ok(graph)
    }
    pub(crate) fn cache_edges(&self) -> Vec<(u32, Vec<u32>)> {
        let mut edges: Vec<_> = self
            .children
            .iter()
            .map(|(node, children)| {
                (
                    node.raw(),
                    children.iter().map(|child| child.raw()).collect(),
                )
            })
            .collect();
        edges.sort_by_key(|(node, _)| *node);
        edges
    }
    pub(crate) fn retarget(&self, mut node: Handle, root: Handle) -> Handle {
        while let Some(scope) = self.scopes.get(&node) {
            if *scope == root {
                break;
            }
            let Some(host) = self.hosts.get(scope) else {
                break;
            };
            node = *host;
        }
        node
    }
}

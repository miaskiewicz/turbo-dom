//! Native composition and selector scope are separate graphs: slot placement
//! changes physical ancestry, while author selectors keep the original DOM.
//!
//! [`Composition`] is the pure graph over a turbo-dom [`Tree`]: `scopes` maps every
//! node of the original (shadow-including) tree to its tree scope root, `hosts` each
//! shadow root to its host, `depths` each scope root to its encapsulation depth (the
//! document is 0); `children`/`parents` are the composed (flat) tree an embedder
//! distributes slots into (it supplies them — slot assignment needs its runtime).
use std::collections::{BTreeMap, HashMap};
use turbo_dom::rtdom::tree::Handle;
use turbo_dom::rtdom::Tree;

/// The scope graph and composed tree of a live document (see the module docs).
#[derive(Debug, Default, Clone)]
pub struct Composition {
    pub children: HashMap<Handle, Vec<Handle>>,
    pub parents: HashMap<Handle, Handle>,
    pub scopes: HashMap<Handle, Handle>,
    pub hosts: BTreeMap<Handle, Handle>,
    pub depths: HashMap<Handle, u32>,
}
impl Composition {
    /// The scope graph of `tree` (no composed children yet).
    ///
    /// # Errors
    /// More than 50,000 nodes, a node deeper than 256, or a node reached twice.
    pub fn scopes(tree: &Tree) -> Result<Self, String> {
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
    /// The composed children of `node` (empty when it has none).
    #[must_use]
    pub fn children_of(&self, node: Handle) -> &[Handle] {
        self.children.get(&node).map_or(&[], Vec::as_slice)
    }
    /// The composed edges as raw handles, sorted (a layout cache key).
    #[must_use]
    pub fn cache_edges(&self) -> Vec<(u32, Vec<u32>)> {
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
    /// Retarget `node` into the tree scope rooted at `root` (its shadow-including
    /// host chain, as events are retargeted).
    #[must_use]
    pub fn retarget(&self, mut node: Handle, root: Handle) -> Handle {
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

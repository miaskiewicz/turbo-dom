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

#[cfg(test)]
mod tests {
    use super::*;

    /// `<div id=h>` hosting a shadow tree (`<span id=s>` and a nested host `<p id=n>`
    /// whose own shadow tree holds `<b>`): the tree, both shadow roots, and the nodes.
    fn nested() -> (Tree, [Handle; 6]) {
        let mut tree = Tree::parse("<div id=h><i id=light></i></div>");
        let host = tree.get_element_by_id("h").unwrap();
        let light = tree.get_element_by_id("light").unwrap();
        let outer = tree.attach_shadow(host);
        let span = tree.create_element("span");
        let inner_host = tree.create_element("p");
        tree.append_child(outer, span);
        tree.append_child(outer, inner_host);
        let inner = tree.attach_shadow(inner_host);
        let bold = tree.create_element("b");
        tree.append_child(inner, bold);
        (tree, [host, light, outer, span, inner, bold])
    }

    #[test]
    fn scopes_map_every_node_to_its_tree_scope_root_with_encapsulation_depths() {
        let (tree, [host, light, outer, span, inner, bold]) = nested();
        let inner_host = tree.shadow_host(inner).unwrap();
        let graph = Composition::scopes(&tree).unwrap();
        let root = tree.root();
        for (node, scope) in [
            (root, root),
            (host, root),
            (light, root),
            (outer, outer),
            (span, outer),
            (inner_host, outer),
            (inner, inner),
            (bold, inner),
        ] {
            assert_eq!(graph.scopes[&node], scope, "{node:?}");
        }
        assert_eq!(
            graph.hosts,
            BTreeMap::from([(outer, host), (inner, inner_host)])
        );
        assert_eq!(graph.depths.len(), 3);
        assert_eq!(
            (
                graph.depths[&root],
                graph.depths[&outer],
                graph.depths[&inner]
            ),
            (0, 1, 2)
        );
        // No composed children until the embedder distributes slots.
        assert!(graph.children.is_empty() && graph.parents.is_empty());
        let all = tree.handles().count();
        assert_eq!(graph.scopes.len(), all);
    }

    #[test]
    fn retarget_climbs_the_host_chain_into_the_requested_scope() {
        let (mut tree, [host, light, outer, span, inner, bold]) = nested();
        let inner_host = tree.shadow_host(inner).unwrap();
        let graph = Composition::scopes(&tree).unwrap();
        let root = tree.root();
        assert_eq!(graph.retarget(bold, root), host);
        assert_eq!(graph.retarget(bold, outer), inner_host);
        assert_eq!(graph.retarget(bold, inner), bold);
        assert_eq!(graph.retarget(span, root), host);
        assert_eq!(graph.retarget(span, outer), span);
        assert_eq!(graph.retarget(light, root), light);
        // A document node cannot be retargeted into a shadow scope: it stays itself.
        assert_eq!(graph.retarget(light, outer), light);
        // A node the graph never saw is its own target.
        let stray = tree.create_element("em");
        assert_eq!(graph.retarget(stray, root), stray);
    }

    #[test]
    fn composed_children_and_sorted_cache_edges() {
        let (tree, [host, light, _, span, _, bold]) = nested();
        let mut graph = Composition::scopes(&tree).unwrap();
        assert_eq!(graph.children_of(host), &[] as &[Handle]);
        assert!(graph.cache_edges().is_empty());
        graph.children.insert(span, vec![bold]);
        graph.children.insert(host, vec![span, light]);
        assert_eq!(graph.children_of(host), &[span, light]);
        assert_eq!(graph.children_of(bold), &[] as &[Handle]);
        let mut want = vec![
            (host.raw(), vec![span.raw(), light.raw()]),
            (span.raw(), vec![bold.raw()]),
        ];
        want.sort_by_key(|(node, _)| *node);
        assert_eq!(graph.cache_edges(), want);
        assert!(want[0].0 < want[1].0);
    }

    #[test]
    fn scope_graph_budgets_depth_and_nodes() {
        const BUDGET: &str = "native layout scope graph exceeds source budget";
        // The document, <html>, <body> and the divs: a div at depth 257 is too deep,
        // one at depth 256 is not.
        let deep = |divs: usize| Tree::parse(&"<div>".repeat(divs));
        assert!(Composition::scopes(&deep(254)).is_ok());
        assert_eq!(Composition::scopes(&deep(255)).unwrap_err(), BUDGET);
        // A shadow root is one level below its host: an (empty) one on the deepest
        // div of the deepest tree that fits is too deep.
        let mut tree = deep(254);
        let deepest = tree
            .handles()
            .filter(|&h| tree.local_name(h) == Some("div"))
            .last()
            .unwrap();
        assert!(Composition::scopes(&tree).is_ok());
        tree.attach_shadow(deepest);
        assert_eq!(Composition::scopes(&tree).unwrap_err(), BUDGET);
        // 50,000 nodes fit; the 50,001st does not (document, html, head, body + <i>s).
        let wide = |n: usize| Tree::parse(&"<i></i>".repeat(n));
        assert_eq!(
            Composition::scopes(&wide(49_996)).unwrap().scopes.len(),
            50_000
        );
        assert_eq!(Composition::scopes(&wide(49_997)).unwrap_err(), BUDGET);
    }
}

//! Bounded, version-owned seeds for document queries. Full selectors still match live nodes.
use super::{Complex, Handle, NodeType, Tree};
use rustc_hash::FxHashMap;
use std::rc::Rc;

const MAX_PAIRS: usize = 10_000;
#[cfg(test)]
thread_local! { static INDEXED_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
const MAX_BYTES: usize = 2_000_000;
const MAX_NODES: usize = 50_000;
type Entries = FxHashMap<String, Vec<(usize, Handle)>>;

pub(crate) struct CandidateIndex(Option<Entries>);

fn seed(selector: &Complex) -> Option<String> {
    let compound = selector
        .rest
        .last()
        .map_or(&selector.first, |(_, compound)| compound);
    compound
        .id
        .as_ref()
        .map(|id| format!("#{id}"))
        .or_else(|| compound.classes.first().map(|class| format!(".{class}")))
}

pub(super) fn select(tree: &Tree, selectors: &[Complex]) -> Option<Vec<Handle>> {
    // A first query after mutation is cheaper as a walk. Build only once a
    // revision has demonstrated enough query reuse to amortize the index.
    if tree.qcache.borrow().map.len() < 4 {
        return None;
    }
    let seeds = selectors.iter().map(seed).collect::<Option<Vec<_>>>()?;
    let cached = tree.qcache.borrow().candidates.clone();
    let index = cached.unwrap_or_else(|| {
        let index = Rc::new(CandidateIndex(build(tree)));
        tree.qcache.borrow_mut().candidates = Some(index.clone());
        index
    });
    let entries = index.0.as_ref()?;
    let mut nodes = Vec::new();
    for seed in seeds {
        if let Some(matches) = entries.get(&seed) {
            nodes.extend(matches);
        }
    }
    nodes.sort_unstable_by_key(|(order, _)| *order);
    nodes.dedup_by_key(|(order, _)| *order);
    Some(nodes.into_iter().map(|(_, node)| node).collect())
}

fn build(tree: &Tree) -> Option<Entries> {
    let mut result = Entries::default();
    let mut pairs = 0;
    let mut bytes = 0;
    let mut order = 0;
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        order += 1;
        #[cfg(test)]
        INDEXED_NODES.with(|count| count.set(count.get() + 1));
        if order > MAX_NODES {
            return None;
        }
        if tree.node_type(node) == NodeType::Element {
            add_node(tree, node, order, &mut result, &mut pairs, &mut bytes)?;
        }
        let base = stack.len();
        tree.for_each_light_child(node, |child| stack.push(child));
        stack[base..].reverse();
    }
    Some(result)
}

fn add_node(
    tree: &Tree,
    node: Handle,
    order: usize,
    result: &mut Entries,
    pairs: &mut usize,
    bytes: &mut usize,
) -> Option<()> {
    if let Some(id) = tree
        .get_attribute_ns_borrowed(node, None, "id")
        .filter(|_| tree.attribute_is_scalar(node, "id"))
    {
        insert(result, format!("#{id}"), (order, node), pairs, bytes)?;
    }
    if let Some(classes) = tree.get_attribute_ns_borrowed(node, None, "class") {
        for class in classes.split_ascii_whitespace() {
            insert(result, format!(".{class}"), (order, node), pairs, bytes)?;
        }
    }
    Some(())
}

fn insert(
    result: &mut Entries,
    key: String,
    node: (usize, Handle),
    pairs: &mut usize,
    bytes: &mut usize,
) -> Option<()> {
    *pairs += 1;
    *bytes = bytes.checked_add(key.len())?;
    if *pairs > MAX_PAIRS || *bytes > MAX_BYTES {
        return None;
    }
    result.entry(key).or_default().push(node);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_queries_after_mutations_do_not_rebuild_an_unused_index() {
        use std::fmt::Write;
        let mut html = String::from("<!doctype html><main id=touch><i class=pick></i>");
        for index in 0..2000 {
            write!(html, "<div class=other{index}></div>").unwrap();
        }
        let mut tree = Tree::parse(&html);
        let root = tree.get_element_by_id("touch").unwrap();
        INDEXED_NODES.with(|count| count.set(0));
        let started = std::time::Instant::now();
        for index in 0..50 {
            tree.set_attribute(root, "data-revision", &index.to_string());
            assert_eq!(tree.query_selector_all(".pick").len(), 1);
        }
        let visits = INDEXED_NODES.with(std::cell::Cell::get);
        eprintln!(
            "cold-query diagnostic: {:?}, indexed nodes {visits}",
            started.elapsed()
        );
        assert_eq!(visits, 0, "one query per revision cannot amortize an index");
    }

    #[test]
    fn oversized_candidate_indexes_fall_back_without_rejecting_queries() {
        let classes = (0..=MAX_PAIRS)
            .map(|index| format!("x{index}"))
            .collect::<Vec<_>>()
            .join(" ");
        let tree = Tree::parse(&format!("<!doctype html><div class='{classes}'></div>"));
        for selector in ["html", "head", "body", "div"] {
            tree.query_selector_all(selector);
        }
        assert_eq!(tree.query_selector_all(".x10000").len(), 1);
        assert!(tree
            .qcache
            .borrow()
            .candidates
            .as_ref()
            .unwrap()
            .0
            .is_none());
        let mut entries = Entries::default();
        let mut pairs = MAX_PAIRS;
        let mut bytes = 0;
        assert!(insert(
            &mut entries,
            "x".into(),
            (0, tree.root()),
            &mut pairs,
            &mut bytes
        )
        .is_none());
        pairs = 0;
        bytes = MAX_BYTES;
        assert!(insert(
            &mut entries,
            "x".into(),
            (0, tree.root()),
            &mut pairs,
            &mut bytes
        )
        .is_none());
        let tree = Tree::parse(&"<div></div>".repeat(MAX_NODES));
        for selector in ["html", "head", "body", "div"] {
            tree.query_selector_all(selector);
        }
        assert_eq!(tree.query_selector_all(".absent").len(), 0);
        assert!(tree
            .qcache
            .borrow()
            .candidates
            .as_ref()
            .unwrap()
            .0
            .is_none());
    }
}

//! MutationObserver for the Rust-native runtime. Mirrors `MutationObserver` in
//! the JS runtime, minus the async microtask delivery (a JS event-loop concern):
//! a Rust consumer pulls pending records synchronously via `take_records`, exactly
//! like the DOM `takeRecords()` method.
//!
//! The `Tree` buffers mutation records only while recording is on (an observer is
//! attached) — the no-observer path stays zero-alloc. v1 assumes a single observer
//! per tree (`take_records` drains the shared buffer); multi-observer fan-out would
//! need per-observer queues.

use super::tree::{Handle, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    ChildList,
    Attributes,
    CharacterData,
}

/// One observed mutation. `added`/`removed` are populated for `ChildList`;
/// `attribute_name`/`old_value` for `Attributes`; `old_value` for `CharacterData`.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationRecord {
    pub kind: MutationKind,
    pub target: Handle,
    pub added: Vec<Handle>,
    pub removed: Vec<Handle>,
    pub attribute_name: Option<String>,
    pub old_value: Option<String>,
}

impl MutationRecord {
    pub(crate) fn attributes(target: Handle, name: &str, old_value: Option<String>) -> Self {
        MutationRecord {
            kind: MutationKind::Attributes,
            target,
            added: Vec::new(),
            removed: Vec::new(),
            attribute_name: Some(name.to_string()),
            old_value,
        }
    }
    pub(crate) fn child_list(target: Handle, added: Vec<Handle>, removed: Vec<Handle>) -> Self {
        MutationRecord {
            kind: MutationKind::ChildList,
            target,
            added,
            removed,
            attribute_name: None,
            old_value: None,
        }
    }
    pub(crate) fn character_data(target: Handle, old_value: Option<String>) -> Self {
        MutationRecord {
            kind: MutationKind::CharacterData,
            target,
            added: Vec::new(),
            removed: Vec::new(),
            attribute_name: None,
            old_value,
        }
    }
}

/// `observe(target, init)` options — mirrors `MutationObserverInit`.
#[derive(Debug, Clone, Default)]
pub struct ObserverInit {
    pub child_list: bool,
    pub attributes: bool,
    pub character_data: bool,
    pub subtree: bool,
    pub attribute_old_value: bool,
    pub character_data_old_value: bool,
}

pub struct MutationObserver {
    target: Option<Handle>,
    init: ObserverInit,
}

impl Default for MutationObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl MutationObserver {
    pub fn new() -> Self {
        MutationObserver { target: None, init: ObserverInit::default() }
    }

    /// Start observing `target` with `init`; turns on the tree's record buffer.
    pub fn observe(&mut self, tree: &mut Tree, target: Handle, init: ObserverInit) {
        self.target = Some(target);
        self.init = init;
        tree.start_recording();
    }

    /// Stop observing and drop pending records.
    pub fn disconnect(&mut self, tree: &mut Tree) {
        self.target = None;
        tree.stop_recording();
    }

    /// Drain + return the records relevant to this observer (filtered by type,
    /// target/subtree, with old values nulled out when not requested).
    pub fn take_records(&self, tree: &mut Tree) -> Vec<MutationRecord> {
        let target = match self.target {
            Some(t) => t,
            None => return Vec::new(),
        };
        let all = tree.take_mutation_records();
        all.into_iter()
            .filter(|r| self.type_enabled(r.kind))
            .filter(|r| target_matches(tree, target, self.init.subtree, r.target))
            .map(|mut r| {
                self.strip_old_value(&mut r);
                r
            })
            .collect()
    }

    fn type_enabled(&self, kind: MutationKind) -> bool {
        match kind {
            MutationKind::ChildList => self.init.child_list,
            MutationKind::Attributes => self.init.attributes,
            MutationKind::CharacterData => self.init.character_data,
        }
    }

    fn strip_old_value(&self, r: &mut MutationRecord) {
        match r.kind {
            MutationKind::Attributes if !self.init.attribute_old_value => r.old_value = None,
            MutationKind::CharacterData if !self.init.character_data_old_value => r.old_value = None,
            _ => {}
        }
    }
}

/// `node` is the record target; does it match the observed `target` (itself, or a
/// descendant when `subtree`)?
fn target_matches(tree: &Tree, target: Handle, subtree: bool, node: Handle) -> bool {
    if node == target {
        return true;
    }
    if !subtree {
        return false;
    }
    let mut cur = tree.parent(node);
    while let Some(p) = cur {
        if p == target {
            return true;
        }
        cur = tree.parent(p);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(tree: &Tree, tag: &str) -> Handle {
        tree.get_elements_by_tag_name(tag)[0]
    }

    #[test]
    fn child_list_add_and_remove() {
        let mut tree = Tree::parse("<ul></ul>");
        let ul = el(&tree, "ul");
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, ul, ObserverInit { child_list: true, ..Default::default() });
        assert!(tree.is_recording());
        let li = tree.create_element("li");
        tree.append_child(ul, li);
        tree.remove_child(ul, li);
        let recs = obs.take_records(&mut tree);
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].kind, MutationKind::ChildList);
        assert_eq!(recs[0].added, vec![li]);
        assert_eq!(recs[1].removed, vec![li]);
        // drained
        assert!(obs.take_records(&mut tree).is_empty());
    }

    #[test]
    fn attributes_with_and_without_old_value() {
        let mut tree = Tree::parse("<div class=a></div>");
        let div = el(&tree, "div");
        // with attributeOldValue
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, div, ObserverInit { attributes: true, attribute_old_value: true, ..Default::default() });
        tree.set_attribute(div, "class", "b");
        let recs = obs.take_records(&mut tree);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].attribute_name.as_deref(), Some("class"));
        assert_eq!(recs[0].old_value.as_deref(), Some("a"));
        // without attributeOldValue → old_value nulled
        let mut obs2 = MutationObserver::new();
        obs2.observe(&mut tree, div, ObserverInit { attributes: true, ..Default::default() });
        tree.set_attribute(div, "class", "c");
        tree.remove_attribute(div, "class");
        let r2 = obs2.take_records(&mut tree);
        assert_eq!(r2.len(), 2);
        assert!(r2.iter().all(|r| r.old_value.is_none()));
    }

    #[test]
    fn character_data_old_value() {
        let mut tree = Tree::parse("<p>old</p>");
        let p = el(&tree, "p");
        let text = tree.children(p)[0];
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, text, ObserverInit { character_data: true, character_data_old_value: true, ..Default::default() });
        tree.set_text_content(text, "new");
        let recs = obs.take_records(&mut tree);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].kind, MutationKind::CharacterData);
        assert_eq!(recs[0].old_value.as_deref(), Some("old"));
    }

    #[test]
    fn subtree_and_type_filtering() {
        let mut tree = Tree::parse("<section><div><span class=x></span></div></section>");
        let section = el(&tree, "section");
        let span = el(&tree, "span");
        // subtree attributes: a deep descendant attr change is reported
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, section, ObserverInit { attributes: true, subtree: true, ..Default::default() });
        tree.set_attribute(span, "class", "y");
        assert_eq!(obs.take_records(&mut tree).len(), 1);
        // without subtree: descendant change NOT reported
        let mut obs2 = MutationObserver::new();
        obs2.observe(&mut tree, section, ObserverInit { attributes: true, ..Default::default() });
        tree.set_attribute(span, "class", "z");
        assert!(obs2.take_records(&mut tree).is_empty());
        // type filtering: childList off → an append on the observed node is ignored
        let mut obs3 = MutationObserver::new();
        obs3.observe(&mut tree, section, ObserverInit { attributes: true, ..Default::default() });
        let d = tree.create_element("div");
        tree.append_child(section, d);
        assert!(obs3.take_records(&mut tree).is_empty());
    }

    #[test]
    fn disconnect_stops_and_unobserved_takes_nothing() {
        let mut tree = Tree::parse("<div></div>");
        let div = el(&tree, "div");
        let mut obs = MutationObserver::new();
        // take_records before observe → nothing (no target)
        assert!(obs.take_records(&mut tree).is_empty());
        obs.observe(&mut tree, div, ObserverInit { attributes: true, ..Default::default() });
        obs.disconnect(&mut tree);
        assert!(!tree.is_recording());
        tree.set_attribute(div, "class", "a"); // not recorded (disconnected)
        assert!(obs.take_records(&mut tree).is_empty());
    }

    #[test]
    fn char_data_strip_default_and_non_descendant() {
        // Default impl
        let _ = MutationObserver::default();
        // CharacterData WITHOUT character_data_old_value → old_value nulled (strip arm)
        let mut tree = Tree::parse("<p>old</p>");
        let text = tree.children(el(&tree, "p"))[0];
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, text, ObserverInit { character_data: true, ..Default::default() });
        tree.set_text_content(text, "new");
        let recs = obs.take_records(&mut tree);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].old_value.is_none());
        // subtree=true but the mutated node is OUTSIDE the observed subtree → filtered
        let mut t2 = Tree::parse("<section><a></a></section><aside><b></b></aside>");
        let section = el(&t2, "section");
        let b = el(&t2, "b");
        let mut obs2 = MutationObserver::new();
        obs2.observe(&mut t2, section, ObserverInit { attributes: true, subtree: true, ..Default::default() });
        t2.set_attribute(b, "class", "z"); // b is under <aside>, not <section>
        assert!(obs2.take_records(&mut t2).is_empty());
    }

    #[test]
    fn element_set_text_content_is_childlist() {
        let mut tree = Tree::parse("<div>a<span>b</span></div>");
        let div = el(&tree, "div");
        let mut obs = MutationObserver::new();
        obs.observe(&mut tree, div, ObserverInit { child_list: true, ..Default::default() });
        tree.set_text_content(div, "replaced");
        let recs = obs.take_records(&mut tree);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].kind, MutationKind::ChildList);
        assert_eq!(recs[0].added.len(), 1);
        assert_eq!(recs[0].removed.len(), 2); // the old text + span
    }
}

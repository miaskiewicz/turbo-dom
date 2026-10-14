use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
struct TrackedAlloc;
thread_local! { static ALLOCATED: Cell<Option<usize>> = const { Cell::new(None) }; }
fn track_bytes(bytes: usize) {
    let _ = ALLOCATED.try_with(|count| {
        if let Some(value) = count.get() {
            count.set(Some(value.saturating_add(bytes)));
        }
    });
}
unsafe impl GlobalAlloc for TrackedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        track_bytes(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        track_bytes(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        track_bytes(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: TrackedAlloc = TrackedAlloc;

use std::fmt::Write;
use turbo_dom::rtdom::serialize::serialize_outer;
use turbo_dom::rtdom::tree::{CollectionBudget, CollectionError, Handle, Tree};

fn budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 100_000,
        max_edges: 1_000_000,
        max_payload_bytes: 16_000_000,
    }
}
fn remove(tree: &mut Tree, node: Handle) {
    let parent = tree.parent(node).unwrap();
    tree.remove_child(parent, node);
}

#[test]
fn parsed_payload_is_freed_without_renumbering_or_visible_mutation() {
    let payload = "discard me".repeat(50_000);
    let mut tree = Tree::parse(&format!("<!doctype html><main id=keep>kept</main><aside id=discard data-big='{payload}'>{payload}</aside>"));
    let keep = tree.query_selector("#keep").unwrap();
    let discard = tree.query_selector("#discard").unwrap();
    remove(&mut tree, discard);
    let html = serialize_outer(&tree, tree.root());
    let version = tree.version;
    let result = tree.collect_unreachable(&[], budget()).unwrap();
    assert!(result.before.payload_bytes > 900_000);
    assert!(result.after.payload_bytes < 1000);
    assert!(result.reclaimed_nodes >= 2);
    assert_eq!(result.after.parsed_nodes, 1);
    assert!(!tree.is_live(discard));
    assert_eq!(tree.query_selector("#keep"), Some(keep));
    assert_eq!(tree.get_attribute(keep, "id"), Some("keep"));
    assert_eq!(serialize_outer(&tree, tree.root()), html);
    assert_eq!(tree.version, version);
}

#[test]
fn retained_descendant_preserves_composed_ancestors_siblings_shadow_and_attr_identity() {
    let mut tree =
        Tree::parse("<section id=detached><span id=a>one</span><span id=b>two</span></section>");
    let owner = tree.query_selector("#detached").unwrap();
    let a = tree.query_selector("#a").unwrap();
    let b = tree.query_selector("#b").unwrap();
    let attr = tree.attribute_node(b, None, "id").unwrap();
    let shadow = tree.attach_shadow(owner);
    let shadow_child = tree.create_text_node("shadow");
    tree.append_child(shadow, shadow_child);
    remove(&mut tree, owner);
    tree.collect_unreachable(&[a], budget()).unwrap();
    for h in [owner, a, b, attr, shadow, shadow_child] {
        assert!(tree.is_live(h));
    }
    assert_eq!(tree.parent(a), Some(owner));
    assert_eq!(tree.previous_sibling(b), Some(a));
    assert_eq!(tree.attribute_owner(attr), Some(b));
    assert_eq!(tree.shadow_host(shadow), Some(owner));
    tree.set_attribute_value(attr, "new-id");
    assert_eq!(tree.get_attribute(b, "id"), Some("new-id"));
    remove(&mut tree, b);
    tree.collect_unreachable(&[a], budget()).unwrap();
    assert!(!tree.is_live(b));
    assert!(!tree.is_live(attr));
    assert!(tree.is_live(shadow));
    tree.collect_unreachable(&[], budget()).unwrap();
    assert!(!tree.is_live(owner));
    assert!(!tree.is_live(shadow_child));
}

#[test]
fn retained_detached_attribute_keeps_its_element_component_until_detachment() {
    let mut tree = Tree::parse("<p id=x>text</p>");
    let p = tree.query_selector("p").unwrap();
    let attr = tree.attribute_node(p, None, "id").unwrap();
    remove(&mut tree, p);
    tree.collect_unreachable(&[attr], budget()).unwrap();
    assert!(tree.is_live(p));
    tree.remove_attribute_node(p, attr).unwrap();
    tree.collect_unreachable(&[attr], budget()).unwrap();
    assert!(!tree.is_live(p));
    assert_eq!(tree.attribute_owner(attr), None);
    assert_eq!(tree.attribute_data(attr).unwrap().value, "x");
}

#[test]
fn pending_mutation_events_pin_removed_nodes_without_changing_records() {
    let mut tree = Tree::parse("<main><p id=removed>x</p><b>tail</b></main>");
    let p = tree.query_selector("p").unwrap();
    tree.start_recording();
    remove(&mut tree, p);
    let version = tree.version;
    tree.collect_unreachable(&[], budget()).unwrap();
    assert!(tree.is_live(p));
    assert_eq!(tree.version, version);
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].record.removed(), &[p]);
    assert_eq!(
        tree.node_value(tree.first_child(p).unwrap()).as_deref(),
        Some("x")
    );
    tree.collect_unreachable(&[], budget()).unwrap();
    assert!(!tree.is_live(p));
    assert_eq!(tree.take_mutation_events(), []);
}

#[test]
fn repeated_churn_has_bounded_slots_and_never_reuses_collected_ids() {
    let mut tree = Tree::parse("<main id=container></main>");
    let main = tree.query_selector("main").unwrap();
    tree.collect_unreachable(&[], budget()).unwrap();
    let baseline = tree.storage_stats().runtime_slots;
    let mut previous = 0;
    for _ in 0..100 {
        let mut last = main;
        for _ in 0..100 {
            let node = tree.create_element("div");
            assert!(node.raw() > previous);
            previous = node.raw();
            tree.set_attribute(node, "data-payload", &"x".repeat(500));
            tree.append_child(main, node);
            last = node;
        }
        tree.set_inner_html(main, "");
        tree.collect_unreachable(&[], budget()).unwrap();
        assert!(!tree.is_live(last));
        assert_eq!(tree.storage_stats().runtime_slots, baseline);
        assert_eq!(tree.handles().count(), tree.node_count() as usize);
    }
    let after = tree.create_text_node("new");
    assert!(after.raw() > previous);
    assert_eq!(tree.node_value(after).as_deref(), Some("new"));
}

#[test]
fn collection_preserves_namespace_doctype_template_and_mutable_character_data() {
    let mut tree = Tree::parse("<!doctype html PUBLIC 'pub' 'sys'><template><span>x</span></template><svg><a xlink:href='before'/></svg>");
    let svg = tree.query_selector("svg").unwrap();
    let link = tree.query_selector("svg a").unwrap();
    let attr = tree
        .attribute_node(link, Some("http://www.w3.org/1999/xlink"), "href")
        .unwrap();
    let element = tree.create_element_namespace(Some("urn:custom"), "p:item");
    tree.append_child(svg, element);
    let text = tree.create_text_node("before");
    tree.append_child(element, text);
    tree.set_node_value(text, "after");
    let html = serialize_outer(&tree, tree.root());
    tree.collect_unreachable(&[], budget()).unwrap();
    assert_eq!(serialize_outer(&tree, tree.root()), html);
    assert_eq!(tree.attribute_owner(attr), Some(link));
    assert_eq!(tree.namespace_uri(element), Some("urn:custom"));
    assert_eq!(tree.prefix(element), Some("p"));
    assert_eq!(tree.qualified_name(element), Some("p:item"));
    assert_eq!(tree.node_value(text).as_deref(), Some("after"));
    tree.set_attribute_value(attr, "after");
    assert_eq!(
        tree.attribute_data(attr).unwrap().qualified_name,
        "xlink:href"
    );
}

#[test]
fn budget_and_invalid_pin_failures_leave_all_storage_and_dom_state_unchanged() {
    let mut tree = Tree::parse("<main><p>payload</p></main>");
    let discarded = tree.create_text_node("detached");
    let before = tree.storage_stats();
    let html = serialize_outer(&tree, tree.root());
    let version = tree.version;
    for bounds in [
        CollectionBudget {
            max_nodes: 1,
            ..budget()
        },
        CollectionBudget {
            max_edges: 1,
            ..budget()
        },
        CollectionBudget {
            max_payload_bytes: 1,
            ..budget()
        },
    ] {
        assert_eq!(
            tree.collect_unreachable(&[], bounds),
            Err(CollectionError::BudgetExceeded)
        );
        assert_eq!(tree.storage_stats(), before);
        assert_eq!(serialize_outer(&tree, tree.root()), html);
        assert_eq!(tree.version, version);
        assert!(tree.is_live(discarded));
    }
    assert_eq!(
        tree.collect_unreachable(&[Handle::from_raw(u32::MAX)], budget()),
        Err(CollectionError::InvalidPin)
    );
    assert_eq!(tree.storage_stats(), before);
}

#[test]
fn invalid_composed_ancestry_is_rejected_before_collection() {
    let mut tree = Tree::parse("");
    let a = tree.create_element("a");
    tree.append_child(a, a);
    let before = tree.storage_stats();
    assert_eq!(
        tree.collect_unreachable(&[a], budget()),
        Err(CollectionError::InvalidGraph)
    );
    assert_eq!(tree.storage_stats(), before);
    assert!(tree.is_live(a));
    assert_eq!(tree.parent(a), Some(a));
}

#[test]
fn pooled_attribute_expansion_budget_is_rejected_atomically() {
    let value = "same long value".repeat(100);
    let mut html = String::new();
    for n in 0..100 {
        write!(html, "<p data-index='{n}' data-value='{value}'></p>").unwrap();
    }
    let mut tree = Tree::parse(&html);
    let before = tree.storage_stats();
    let version = tree.version;
    let bounds = CollectionBudget {
        max_payload_bytes: before.payload_bytes + 1,
        ..budget()
    };
    assert_eq!(
        tree.collect_unreachable(&[], bounds),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert_eq!(tree.version, version);
    assert_eq!(
        serialize_outer(&tree, tree.root()),
        serialize_outer(&Tree::parse(&html), tree.root())
    );
}

#[test]
fn pinned_overwritten_shadow_root_is_rejected_without_losing_pin() {
    let mut tree = Tree::parse("<section></section>");
    let host = tree.query_selector("section").unwrap();
    let old = tree.attach_shadow(host);
    let child = tree.create_text_node("held old shadow");
    tree.append_child(old, child);
    let current = tree.attach_shadow(host);
    let before = tree.storage_stats();
    assert_eq!(
        tree.collect_unreachable(&[old], budget()),
        Err(CollectionError::InvalidGraph)
    );
    assert_eq!(tree.storage_stats(), before);
    assert!(tree.is_live(old));
    assert!(tree.is_live(child));
    assert_eq!(tree.shadow_root(host), Some(current));
    assert_eq!(tree.node_value(child).as_deref(), Some("held old shadow"));
}

#[test]
fn namespace_metadata_counts_towards_expanded_retained_budget() {
    let mut html = String::new();
    for _ in 0..12 {
        write!(html, "<p data-value='{}'></p>", "v".repeat(100)).unwrap();
    }
    let mut tree = Tree::parse(&html);
    let node = tree.create_element_namespace(Some(&"u".repeat(6000)), "p:x");
    tree.append_child(tree.root(), node);
    let before = tree.storage_stats();
    assert!(before.payload_bytes < 6500);
    assert_eq!(
        tree.collect_unreachable(
            &[],
            CollectionBudget {
                max_payload_bytes: 6500,
                ..budget()
            }
        ),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert_eq!(tree.namespace_uri(node).unwrap().len(), 6000);
}

#[test]
fn pending_record_strings_remain_accounted_until_records_are_drained() {
    let mut tree = Tree::parse("");
    let node = tree.create_element("p");
    tree.append_child(tree.root(), node);
    let uri = "u".repeat(500_000);
    let old = "v".repeat(500_000);
    tree.set_attribute_ns(node, Some(&uri), "p:old", &old);
    tree.start_recording();
    tree.remove_attribute_ns(node, Some(&uri), "old");
    let before = tree.storage_stats();
    assert!(before.payload_bytes >= 1_000_000);
    assert_eq!(
        tree.collect_unreachable(
            &[],
            CollectionBudget {
                max_payload_bytes: 1000,
                ..budget()
            }
        ),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].record.old_value(), Some(old.as_str()));
    assert_eq!(events[0].attribute_namespace.as_deref(), Some(uri.as_str()));
    let report = tree
        .collect_unreachable(
            &[],
            CollectionBudget {
                max_payload_bytes: 1000,
                ..budget()
            },
        )
        .unwrap();
    assert!(report.after.payload_bytes < 1000);
}

#[test]
fn large_unreachable_payload_can_be_reclaimed_with_small_retained_budget() {
    let mut tree = Tree::parse(&format!(
        "<p id=keep>kept</p><aside>{}</aside>",
        "x".repeat(1_000_000)
    ));
    let aside = tree.query_selector("aside").unwrap();
    remove(&mut tree, aside);
    let report = tree
        .collect_unreachable(
            &[],
            CollectionBudget {
                max_payload_bytes: 1000,
                ..budget()
            },
        )
        .unwrap();
    assert!(report.before.payload_bytes >= 1_000_000);
    assert!(report.after.payload_bytes < 1000);
    assert!(!tree.is_live(aside));
}

#[test]
fn interned_payload_rejection_precedes_expensive_materialization() {
    let value = "x".repeat(1000);
    let mut html = String::new();
    for _ in 0..10_000 {
        write!(html, "<p data-value='{value}'></p>").unwrap();
    }
    let mut tree = Tree::parse(&html);
    let before = tree.storage_stats();
    ALLOCATED.with(|count| count.set(Some(0)));
    let result = tree.collect_unreachable(
        &[],
        CollectionBudget {
            max_payload_bytes: 16_000,
            ..budget()
        },
    );
    let allocated = ALLOCATED.with(|count| count.replace(None).unwrap());
    assert_eq!(result, Err(CollectionError::BudgetExceeded));
    assert_eq!(tree.storage_stats(), before);
    assert!(
        allocated < 1_000_000,
        "rejected collection copied interned payloads: {allocated} allocated bytes"
    );
}

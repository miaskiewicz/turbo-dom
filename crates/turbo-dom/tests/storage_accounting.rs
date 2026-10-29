//! Exact storage accounting (P5-cov): the logical payload `storage_stats` reports per node
//! kind and per side table, and the work / payload budget boundaries of
//! `collect_unreachable` — pinned value by value so a change to any term shows up.
use turbo_dom::rtdom::tree::{CollectionBudget, CollectionError, Handle, Tree};
use turbo_dom::NodeRef;

fn u(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn payload(tree: &Tree) -> usize {
    tree.storage_stats().payload_bytes
}

/// The payload a mutation adds.
fn delta(tree: &mut Tree, change: impl FnOnce(&mut Tree)) -> usize {
    let before = payload(tree);
    change(tree);
    payload(tree) - before
}

#[test]
fn created_nodes_count_their_strings() {
    let mut tree = Tree::parse("");
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_element("abc");
        }),
        3
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_text_node("12345");
        }),
        5
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_comment("1234");
        }),
        4
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_processing_instruction("tt", "ddd");
        }),
        5
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_document_type("n", "pp", "sss");
        }),
        6
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_document_fragment();
        }),
        0
    );
    // an Attr node: namespace + prefix + local + qualified + value
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_attribute(Some("urn:x"), "p:loc", "val");
        }),
        5 + 1 + 3 + 5 + 3
    );
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_attribute(None, "ab", "c");
        }),
        2 + 2 + 1
    );
    // namespace metadata: local name + URI + prefix + qualified name
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_element_namespace(Some("urn:x"), "p:yy");
        }),
        2 + 5 + 1 + 4
    );
    // a lone-unit text keeps its units: lossy UTF-8 (3) + 2 bytes per unit
    assert_eq!(
        delta(&mut tree, |t| {
            let _ = t.create_text_node_utf16(&[0xd800]);
        }),
        3 + 2
    );
}

#[test]
fn overlays_and_side_tables_count_their_strings() {
    let mut tree = Tree::parse("<p>ab</p>");
    let p = tree.query_selector("p").unwrap();
    let text = tree.first_child(p).unwrap();
    // a character-data override adds its string (the parsed one stays pooled)
    assert_eq!(delta(&mut tree, |t| t.set_node_value(text, "xyz")), 3);
    // an attribute: namespace + prefix + local + qualified + value
    assert_eq!(
        delta(&mut tree, |t| t.set_attribute(p, "ab", "cde")),
        2 + 2 + 3
    );
    assert_eq!(
        delta(&mut tree, |t| t.set_attribute_ns(
            p,
            Some("urn:x"),
            "q:r",
            "s"
        )),
        5 + 1 + 1 + 3 + 1
    );
    // a lone-unit attribute value: the record's lossy strings (local + qualified + value)
    // + the retained units (local payload + 2 bytes per unit)
    let lone = |units: &[u16]| {
        let mut tree = Tree::parse("");
        let el = tree.create_element("i");
        delta(&mut tree, |t| t.set_attribute_units(el, &u("ab"), units))
    };
    assert_eq!(lone(&[0xd800]), 2 + 2 + 3 + (2 + 2));
    assert_eq!(lone(&[0xd800, 0xd801, 0xd802]), 2 + 2 + 9 + (2 + 6));
    // namespaced: the namespace's payload joins the unit record
    let mut tree = Tree::parse("");
    let el = tree.create_element("i");
    assert_eq!(
        delta(&mut tree, |t| t.set_attribute_ns_units(
            el,
            Some(&u("urn")),
            &u("a"),
            &[0xd800]
        )),
        (3 + 1 + 1 + 3) + (3 + 1 + 2)
    );
    // pending records keep the old value's units (2 bytes each) until drained
    let pending_old = |old: &[u16]| {
        let mut tree = Tree::parse("");
        let el = tree.create_element("i");
        tree.set_attribute_units(el, &u("a"), old);
        tree.start_recording();
        tree.set_attribute(el, "a", "x");
        let pending = payload(&tree);
        tree.take_mutation_events();
        pending - payload(&tree)
    };
    // record: name "a" + lossy old value; units: 2 per old unit
    assert_eq!(pending_old(&[0xd800]), 1 + 3 + 2);
    assert_eq!(pending_old(&[0xd800, 0x62]), 1 + 4 + 4);
    // source-position attribute names: 2 bytes per unit of each recorded name
    let with_name =
        |name: &str| payload(&Tree::parse_with_source_positions(&format!("<p {name}=1>")));
    // longer name: local + qualified (+2 each) and the position name (+2 per unit)
    assert_eq!(with_name("abc") - with_name("a"), 2 + 2 + 2 * 2);
}

#[test]
fn liveness_is_exact_at_the_buffer_and_arena_ends() {
    let mut tree = Tree::parse("<p></p>");
    let parsed = tree.storage_stats().parsed_nodes as u32;
    assert!(tree.is_live(Handle::from_raw(parsed - 1)));
    assert!(!tree.is_live(Handle::from_raw(parsed)));
    let created = tree.create_element("i");
    assert_eq!(created.raw(), parsed);
    assert!(tree.is_live(created));
    assert!(!tree.is_live(Handle::from_raw(parsed + 1)));
}

/// A document with every node kind, namespaced / prefixed attributes, template content,
/// namespace metadata, a detached node and one pending record.
fn varied() -> Tree {
    let mut tree = Tree::parse(
        "<!DOCTYPE html PUBLIC \"p\" \"s\"><p id=a class=b>xy<!--c--></p>\
         <svg xlink:href=u></svg><template><i>t</i></template>",
    );
    let p = tree.query_selector("p").unwrap();
    let ns = tree.create_element_namespace(Some("urn:x"), "q:el");
    tree.append_child(p, ns);
    let pi = tree.create_processing_instruction("tt", "ddd");
    tree.append_child(p, pi);
    let doctype = tree.create_document_type("n", "pp", "sss");
    tree.append_child(p, doctype);
    tree.set_attribute_ns(ns, Some("urn:y"), "r:at", "vv");
    let _detached = tree.create_element("gone");
    tree.start_recording();
    let i = tree.create_element("i");
    tree.append_child(p, i);
    tree
}

fn budget(max_edges: usize, max_payload_bytes: usize) -> CollectionBudget {
    CollectionBudget {
        max_nodes: 1000,
        max_edges,
        max_payload_bytes,
    }
}

#[test]
fn collection_budgets_have_exact_boundaries() {
    let fresh = varied();
    assert_eq!(fresh.storage_stats().payload_bytes, 103);
    assert_eq!(fresh.storage_stats().live_nodes, 18);
    // work: 109 edges is the exact admission boundary for this graph
    let mut tree = varied();
    let before = tree.storage_stats();
    assert_eq!(
        tree.collect_unreachable(&[], budget(108, 1 << 30)),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    let report = tree.collect_unreachable(&[], budget(109, 1 << 30)).unwrap();
    assert_eq!(report.reclaimed_nodes, 1);
    // payload: the retained graph projects 144 bytes
    let mut tree = varied();
    assert_eq!(
        tree.collect_unreachable(&[], budget(1 << 20, 143)),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert!(tree.collect_unreachable(&[], budget(1 << 20, 144)).is_ok());
    // nodes: the limit is inclusive
    let mut tree = varied();
    let nodes = tree.storage_stats().live_nodes;
    let limit = |max_nodes| CollectionBudget {
        max_nodes,
        ..budget(1 << 20, 1 << 30)
    };
    assert_eq!(
        tree.collect_unreachable(&[], limit(nodes - 1)),
        Err(CollectionError::BudgetExceeded)
    );
    assert!(tree.collect_unreachable(&[], limit(nodes)).is_ok());
}

#[test]
fn node_ref_matches_reads_the_selector() {
    let tree = Tree::parse("<p class=a></p><p></p>");
    let ps = tree.query_selector_all("p");
    assert!(NodeRef::new(&tree, ps[0]).matches(".a"));
    assert!(!NodeRef::new(&tree, ps[1]).matches(".a"));
    assert!(NodeRef::new(&tree, ps[1]).matches("p:last-child"));
}

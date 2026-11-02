use turbo_dom::rtdom::{
    cascade::computed_style,
    tree::{CollectionBudget, Namespace, NodeType},
    Tree,
};
use turbo_dom::Handle;

/// The `n`th element named `name` in node order, template content included: selectors do
/// not reach template content (it is a separate fragment, as in Chrome — P5b,
/// miaskiewicz/turbo-dom#18), so these tests find its nodes structurally.
fn nth(tree: &Tree, name: &str, n: usize) -> Handle {
    tree.handles()
        .filter(|&h| tree.node_type(h) == NodeType::Element && tree.local_name(h) == Some(name))
        .nth(n)
        .unwrap_or_else(|| panic!("<{name}> #{n}"))
}

#[test]
fn extracting_template_fragment_preserves_light_children_original_units_and_pending_records() {
    let mut source = "<template><b data-u='X'>X</b><template><i>nested</i></template></template>"
        .encode_utf16()
        .collect::<Vec<_>>();
    for unit in &mut source {
        if *unit == 88 {
            *unit = 0xd800;
        }
    }
    let mut tree = Tree::parse_utf16(&source);
    let template = tree.query_selector("template").unwrap();
    let fragment = tree.first_child(template).unwrap();
    let b = nth(&tree, "b", 0);
    let nested = nth(&tree, "template", 1);
    let light = tree.create_element("p");
    tree.append_child(template, light);
    tree.start_recording();
    tree.set_attribute(template, "data-pending", "kept");
    assert_eq!(tree.take_template_contents(template), Some(fragment));
    assert_eq!(tree.children(template), [light]);
    assert_eq!(tree.parent(fragment), None);
    assert_eq!(tree.text_content_utf16(b), [0xd800]);
    assert_eq!(tree.get_attribute_utf16(b, "data-u"), Some(vec![0xd800]));
    assert_eq!(tree.parent(nested), Some(fragment));
    assert_eq!(tree.take_template_contents(template), None);
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].record.attribute_name(), Some("data-pending"));
    tree.set_attribute(template, "data-later", "recorded");
    assert_eq!(tree.take_mutation_events().len(), 1);
    tree.collect_unreachable(
        &[fragment],
        CollectionBudget {
            max_nodes: 1000,
            max_edges: 10000,
            max_payload_bytes: 100_000,
        },
    )
    .unwrap();
    assert!(tree.is_live(b));
    assert!(tree.is_live(nested));
}
#[test]
fn only_parsed_html_template_fragments_are_extracted() {
    let mut tree = Tree::parse("<body>");
    let template = tree.create_element("template");
    assert_eq!(tree.take_template_contents(template), None);
    let svg = tree.create_element_ns("template", Namespace::Svg);
    assert_eq!(tree.take_template_contents(svg), None);
    let ordinary = tree.create_element("div");
    let shadow = tree.attach_shadow(ordinary);
    assert_eq!(tree.take_template_contents(ordinary), None);
    assert!(tree.is_live(shadow));
}

fn collection_budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 100_000,
        max_edges: 1_000_000,
        max_payload_bytes: 10_000_000,
    }
}

#[test]
fn ordinary_fragments_are_expanded_and_cannot_be_misidentified() {
    let mut tree = Tree::parse("<body>");
    let host = tree.create_element("template");
    let fragment = tree.create_document_fragment();
    let child = tree.create_element("i");
    tree.append_child(fragment, child);
    tree.append_child(host, fragment);
    assert_eq!(tree.children(host), [child]);
    let before = tree.version;
    assert_eq!(tree.take_template_contents(host), None);
    assert_eq!(tree.version, before);
    let next_fragment = tree.create_document_fragment();
    let next_child = tree.create_element("b");
    tree.append_child(next_fragment, next_child);
    tree.insert_before(host, next_fragment, Some(child));
    assert_eq!(tree.children(host), [next_child, child]);
    assert_eq!(tree.take_template_contents(host), None);
}

#[test]
fn imported_scalar_and_unit_parser_fragments_both_survive_collection_and_extraction() {
    for units in [false, true] {
        let mut tree = Tree::parse("<main></main>");
        let main = tree.query_selector("main").unwrap();
        let source = "<template><b>X</b><template><i>Y</i></template></template>";
        if units {
            let source: Vec<_> = source
                .encode_utf16()
                .map(|unit| if unit == 88 { 0xd800 } else { unit })
                .collect();
            tree.set_inner_html_utf16(main, &source);
        } else {
            tree.set_inner_html(main, source);
        }
        let host = tree.query_selector("template").unwrap();
        let nested = nth(&tree, "template", 1);
        let b = nth(&tree, "b", 0);
        tree.collect_unreachable(&[], collection_budget()).unwrap();
        let inner = tree.take_template_contents(nested).unwrap();
        let outer = tree.take_template_contents(host).unwrap();
        assert_eq!(tree.children(host), []);
        assert_eq!(tree.children(nested), []);
        assert_eq!(tree.parent(nested), Some(outer));
        assert_eq!(tree.parent(inner), None);
        assert_eq!(tree.node_type(outer), NodeType::Fragment);
        tree.collect_unreachable(&[outer, inner], collection_budget())
            .unwrap();
        assert_eq!(
            tree.text_content_utf16(b),
            [if units { 0xd800 } else { 88 }]
        );
        assert_eq!(tree.text_content(inner), "Y");
        assert_eq!(tree.take_template_contents(host), None);
        tree.collect_unreachable(&[], collection_budget()).unwrap();
        assert!(!tree.is_live(b));
        assert!(!tree.is_live(inner));
        assert!(!tree.is_live(outer));
    }
}

#[test]
fn pending_exact_unit_metadata_indices_and_ancestry_are_not_changed() {
    let mut tree = Tree::parse("<template><b></b></template>");
    let host = tree.query_selector("template").unwrap();
    let b = nth(&tree, "b", 0);
    let fragment = tree.parent(b).unwrap();
    let text = tree.create_text_node_utf16(&[0xdc00]);
    tree.append_child(b, text);
    tree.set_attribute_ns_utf16(b, Some("urn:review"), "p:value", &[0xd800]);
    tree.start_recording();
    tree.set_attribute_ns_utf16(b, Some("urn:review"), "p:value", &[0xd801]);
    tree.set_text_content_utf16(text, &[0xdc01]);
    assert_eq!(tree.take_template_contents(host), Some(fragment));
    tree.set_text_content_utf16(text, &[0xdc02]);
    let records = tree.take_mutation_events_utf16();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].old_value, Some(vec![0xd800]));
    assert_eq!(
        records[0].attribute_name,
        Some("value".encode_utf16().collect())
    );
    assert_eq!(
        records[0].attribute_namespace,
        Some("urn:review".encode_utf16().collect())
    );
    assert_eq!(records[1].old_value, Some(vec![0xdc00]));
    assert_eq!(records[2].old_value, Some(vec![0xdc01]));
    assert!(records[0].event.inclusive_ancestors.contains(&host));
    assert!(records[1].event.inclusive_ancestors.contains(&host));
    assert!(!records[2].event.inclusive_ancestors.contains(&host));
    assert!(tree.is_recording());
}

#[test]
fn extraction_invalidates_query_and_style_caches_and_no_op_does_not() {
    let mut tree = Tree::parse("<template><style>p{color:red}</style><b></b></template><p>x</p>");
    let host = tree.query_selector("template").unwrap();
    let p = tree.query_selector("p").unwrap();
    // template content is never in the document's queries or cascade (Chrome), before
    // or after extraction; the caches are still rebuilt on the version bump
    assert_eq!(tree.query_selector_all("b").len(), 0);
    assert_eq!(computed_style(&tree, p).get("color"), None);
    let before = tree.version;
    tree.take_template_contents(host).unwrap();
    assert_eq!(tree.version, before + 1);
    assert_eq!(tree.query_selector_all("b").len(), 0);
    assert_eq!(computed_style(&tree, p).get("color"), None);
    assert_eq!(tree.take_template_contents(host), None);
    assert_eq!(tree.version, before + 1);
}

#[test]
fn shadow_root_is_not_extracted_and_real_content_does_not_break_its_ownership() {
    let mut tree = Tree::parse("<template><b>content</b></template>");
    let host = tree.query_selector("template").unwrap();
    let fragment = tree.first_child(host).unwrap();
    let shadow = tree.attach_shadow(host);
    let inside = tree.create_element("i");
    tree.append_child(shadow, inside);
    let light = tree.create_element("p");
    tree.append_child(host, light);
    assert_eq!(tree.take_template_contents(host), Some(fragment));
    assert_eq!(tree.children(host), [light]);
    assert_eq!(tree.shadow_root(host), Some(shadow));
    assert_eq!(tree.shadow_host(shadow), Some(host));
    assert_eq!(tree.children(shadow), [inside]);
    assert_eq!(tree.take_template_contents(host), None);
    tree.collect_unreachable(&[inside, fragment], collection_budget())
        .unwrap();
    assert_eq!(tree.shadow_root(host), Some(shadow));
}

#[test]
fn wrong_node_types_and_namespaces_are_non_mutating() {
    let mut tree =
        Tree::parse("<body><svg><template></template></svg><math><template></template></math>");
    let html_host = tree.create_element("template");
    let svg_host = tree.create_element_ns("template", Namespace::Svg);
    let math_host = tree.create_element_ns("template", Namespace::MathMl);
    let null_host = tree.create_element_namespace(None, "template");
    let upper_host =
        tree.create_element_namespace(Some("http://www.w3.org/1999/xhtml"), "TEMPLATE");
    let ordinary = tree.create_element("div");
    let text = tree.create_text_node("template");
    let fragment = tree.create_document_fragment();
    let before = tree.version;
    for host in [
        tree.root(),
        html_host,
        svg_host,
        math_host,
        null_host,
        upper_host,
        ordinary,
        text,
        fragment,
    ] {
        assert_eq!(tree.take_template_contents(host), None);
        assert_eq!(tree.version, before);
    }
}

#[test]
fn extracted_descendant_pins_its_component_then_unpinned_component_is_reclaimed() {
    let mut tree = Tree::parse("<template><b>one</b><i>two</i></template>");
    let host = tree.query_selector("template").unwrap();
    let b = nth(&tree, "b", 0);
    let i = nth(&tree, "i", 0);
    let fragment = tree.take_template_contents(host).unwrap();
    tree.collect_unreachable(&[b], collection_budget()).unwrap();
    assert!(tree.is_live(fragment));
    assert!(tree.is_live(i));
    assert_eq!(tree.parent(b), Some(fragment));
    tree.collect_unreachable(&[], collection_budget()).unwrap();
    assert!(!tree.is_live(fragment));
    assert!(!tree.is_live(b));
    assert!(!tree.is_live(i));
    assert!(tree.is_live(host));
}

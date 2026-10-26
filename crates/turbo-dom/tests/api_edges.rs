//! Public Tree API edge cases (P5-cov): wrong node types, created vs parsed nodes,
//! attribute nodes through the UTF-16 surface, and the UTF-16 serializer — each asserted
//! value for value (web-observable behaviour is asserted against Chrome 156 in
//! `rtdom_core_chrome.rs`; these are the API contracts around it).
use turbo_dom::rtdom::mutations::MutationRecord;
use turbo_dom::rtdom::serialize::{
    serialize_inner, serialize_inner_utf16, serialize_outer, serialize_outer_utf16,
};
use turbo_dom::rtdom::tree::{Namespace, NodeType};
use turbo_dom::{Handle, Tree};

fn u(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

const XLINK: &str = "http://www.w3.org/1999/xlink";

#[test]
fn namespace_ids_round_trip_and_default_to_html() {
    assert_eq!(Namespace::from_id(0), Namespace::Html);
    assert_eq!(Namespace::from_id(1), Namespace::Svg);
    assert_eq!(Namespace::from_id(2), Namespace::MathMl);
    assert_eq!(Namespace::from_id(3), Namespace::Other);
    assert_eq!(Namespace::from_id(4), Namespace::Html);
    assert_eq!(Namespace::from_id(255), Namespace::Html);
    for ns in [Namespace::Html, Namespace::Svg, Namespace::MathMl, Namespace::Other] {
        assert_eq!(Namespace::from_id(ns as u8), ns);
    }
}

#[test]
fn non_elements_read_as_the_html_namespace_with_no_uri() {
    let mut tree = Tree::parse("<p>x</p>");
    let parsed_text = tree.first_child(tree.query_selector("p").unwrap()).unwrap();
    let created_text = tree.create_text_node("y");
    let comment = tree.create_comment("c");
    for node in [parsed_text, created_text, comment] {
        assert_eq!(tree.namespace(node), Namespace::Html);
        assert_eq!(tree.namespace_id(node), 0);
        assert_eq!(tree.namespace_uri(node), None);
        assert_eq!(tree.local_name(node), None);
        assert_eq!(tree.get_attribute_ns(node, None, "id"), None);
    }
    // an element created in the `Other` namespace without a URI has none
    let other = tree.create_element_ns("x", Namespace::Other);
    assert_eq!(tree.namespace(other), Namespace::Other);
    assert_eq!(tree.namespace_uri(other), None);
    assert_eq!(tree.namespace_id(other), 3);
    let svg = tree.create_element_ns("rect", Namespace::Svg);
    assert_eq!(tree.namespace_uri(svg), Some("http://www.w3.org/2000/svg"));
}

#[test]
fn doctype_and_processing_instruction_fields_belong_to_their_nodes() {
    let mut tree = Tree::parse("<!DOCTYPE html PUBLIC \"p\" \"s\"><p></p>");
    let parsed = tree.first_child(tree.root()).unwrap();
    let element = tree.query_selector("p").unwrap();
    let created_element = tree.create_element("div");
    let doctype = tree.create_document_type("d", "pub", "sys");
    let pi = tree.create_processing_instruction("target", "data");
    // parsed and created doctypes, scalar and UTF-16
    assert_eq!(
        (tree.doctype_name(parsed), tree.doctype_public_id(parsed), tree.doctype_system_id(parsed)),
        (Some("html"), Some("p"), Some("s"))
    );
    assert_eq!(
        (
            tree.doctype_name_utf16(parsed),
            tree.doctype_public_id_utf16(parsed),
            tree.doctype_system_id_utf16(parsed)
        ),
        (Some(u("html")), Some(u("p")), Some(u("s")))
    );
    assert_eq!(
        (
            tree.doctype_name_utf16(doctype),
            tree.doctype_public_id_utf16(doctype),
            tree.doctype_system_id_utf16(doctype)
        ),
        (Some(u("d")), Some(u("pub")), Some(u("sys")))
    );
    // a doctype field of anything else is None, parsed or created
    for node in [element, created_element, pi] {
        assert_eq!(tree.doctype_name(node), None);
        assert_eq!(tree.doctype_public_id(node), None);
        assert_eq!(tree.doctype_system_id(node), None);
        assert_eq!(tree.doctype_name_utf16(node), None);
        assert_eq!(tree.doctype_public_id_utf16(node), None);
        assert_eq!(tree.doctype_system_id_utf16(node), None);
    }
    // only created processing instructions have a target
    assert_eq!(tree.processing_instruction_target(pi), Some("target"));
    assert_eq!(tree.processing_instruction_target_utf16(pi), Some(u("target")));
    assert_eq!(tree.node_value(pi).as_deref(), Some("data"));
    for node in [parsed, element, created_element, doctype] {
        assert_eq!(tree.processing_instruction_target(node), None);
        assert_eq!(tree.processing_instruction_target_utf16(node), None);
    }
    let lone = tree.create_processing_instruction_units(&[0xd800], &[0xdc00]);
    assert_eq!(tree.processing_instruction_target_utf16(lone), Some(vec![0xd800]));
    assert_eq!(tree.node_value_utf16(lone), Some(vec![0xdc00]));
}

#[test]
fn attribute_and_child_mutations_ignore_the_wrong_node_types() {
    let mut tree = Tree::parse("<p id=a>x</p>");
    let p = tree.query_selector("p").unwrap();
    let text = tree.first_child(p).unwrap();
    let attr = tree.create_attribute(None, "title", "t");
    let version = tree.version;
    // Attr nodes are never tree children or parents
    tree.append_child(p, attr);
    tree.append_child(attr, text);
    tree.insert_before(p, attr, None);
    tree.insert_before(attr, text, None);
    assert_eq!(tree.children(p), [text]);
    assert_eq!(tree.parent(attr), None);
    assert_eq!(tree.children(attr), []);
    assert_eq!(tree.version, version, "rejected mutations do not bump the version");
    // setAttributeNode on a non-element, a non-attribute, or setting an element's value
    assert_eq!(tree.set_attribute_node(text, attr), Err("TypeError"));
    assert_eq!(tree.set_attribute_node(p, text), Err("TypeError"));
    tree.set_attribute_value(p, "ignored");
    tree.set_attribute_value_utf16(p, &u("ignored"));
    assert_eq!(tree.attributes(p), [("id".to_string(), "a".to_string())]);
    assert_eq!(tree.attribute_owner(attr), None);
    assert_eq!(tree.version, version);
}

#[test]
fn node_value_setter_records_the_old_value_while_recording() {
    let mut tree = Tree::parse("<p>old</p>");
    let text = tree.first_child(tree.query_selector("p").unwrap()).unwrap();
    tree.start_recording();
    tree.set_node_value(text, "new");
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].record, MutationRecord::CharacterData { .. }));
    assert_eq!(events[0].record.target(), text);
    assert_eq!(events[0].record.old_value(), Some("old"));
    assert_eq!(tree.node_value(text).as_deref(), Some("new"));
}

#[test]
fn foreign_fragment_attributes_keep_their_prefix_and_namespace() {
    let mut tree = Tree::parse("");
    let div = tree.create_element("div");
    tree.set_inner_html(div, "<svg xlink:href=\"u\" xml:lang=\"en\"></svg>");
    let svg = tree.first_child(div).unwrap();
    assert_eq!(tree.namespace(svg), Namespace::Svg);
    let mut full = Vec::new();
    tree.for_each_attr_full(svg, |prefix, name, value| {
        full.push((prefix.to_string(), name.to_string(), value.to_string()));
    });
    full.sort();
    assert_eq!(
        full,
        [
            ("xlink".into(), "href".into(), "u".into()),
            ("xml".into(), "lang".into(), "en".into())
        ]
    );
    assert_eq!(tree.get_attribute_ns(svg, Some(XLINK), "href").as_deref(), Some("u"));
    assert_eq!(tree.get_attribute(svg, "xlink:href"), Some("u"));
}

#[test]
fn attribute_nodes_expose_their_utf16_names() {
    let mut tree = Tree::parse("");
    let el = tree.create_element("div");
    tree.set_attribute_ns_units(el, Some(&u("urn:x")), &u("p:lo\u{e9}"), &[0xd800]);
    let attr = tree.attribute_node(el, Some("urn:x"), "lo\u{e9}").unwrap();
    assert_eq!(tree.local_name_utf16(attr), Some(u("lo\u{e9}")));
    assert_eq!(tree.qualified_name_utf16(attr), Some(u("p:lo\u{e9}")));
    assert_eq!(tree.prefix_utf16(attr), Some(u("p")));
    assert_eq!(tree.namespace_uri_utf16(attr), Some(u("urn:x")));
    assert_eq!(tree.node_value_utf16(attr), Some(vec![0xd800]));
    // a no-namespace attribute keeps a colon in its local name and has no prefix
    tree.set_attribute_ns_units(el, None, &u("a:b"), &u("v"));
    let plain = tree.attribute_node(el, None, "a:b").unwrap();
    assert_eq!(tree.local_name_utf16(plain), Some(u("a:b")));
    assert_eq!(tree.prefix_utf16(plain), None);
    // elements: parsed ones have no prefix; tag names upper-case HTML only
    let parsed = Tree::parse("<svg><foreignObject></foreignObject></svg><p></p>");
    let p = parsed.query_selector("p").unwrap();
    let fo = parsed.query_selector("foreignObject").unwrap();
    assert_eq!(parsed.prefix_utf16(p), None);
    assert_eq!(parsed.tag_name_utf16(p), Some(u("P")));
    assert_eq!(parsed.tag_name_utf16(fo), Some(u("foreignObject")));
    assert!(parsed.local_name_is(p, "p"));
    assert!(!parsed.local_name_is(p, "P"));
    assert!(parsed.local_name_is(fo, "foreignObject"));
    // created element names lower-case ASCII only
    let created = tree.create_element_units(&u("A\u{c9}z"));
    assert_eq!(tree.local_name_utf16(created), Some(u("a\u{c9}z")));
    assert_eq!(tree.tag_name_utf16(created), Some(u("A\u{c9}Z")));
}

#[test]
fn utf16_attribute_values_survive_on_owned_and_detached_attributes() {
    let mut tree = Tree::parse("");
    let el = tree.create_element("div");
    // a detached attribute keeps its exact units
    let detached = tree.create_attribute_utf16(None, "data-x", &[0x61, 0xdc00]);
    assert_eq!(tree.node_value_utf16(detached), Some(vec![0x61, 0xdc00]));
    assert_eq!(tree.node_value(detached).as_deref(), Some("a\u{fffd}"));
    tree.set_attribute_value_utf16(detached, &[0xd800]);
    assert_eq!(tree.node_value_utf16(detached), Some(vec![0xd800]));
    // attaching it, then setting through textContent, reaches the owner
    tree.set_attribute_node(el, detached).unwrap();
    tree.set_text_content_utf16(detached, &[0x62, 0xd801]);
    assert_eq!(tree.get_attribute_units(el, &u("data-x")), Some(vec![0x62, 0xd801]));
    assert_eq!(tree.get_attribute(el, "data-x"), Some("b\u{fffd}"));
    // removing by units: a missing name is a no-op, the qualified name removes it
    tree.remove_attribute_units(el, &u("data-y"));
    assert_eq!(tree.attributes(el).len(), 1);
    tree.remove_attribute_units(el, &u("data-x"));
    assert_eq!(tree.attributes(el), []);
}

#[test]
fn utf16_serializer_matches_the_scalar_serializer_for_every_node_type() {
    let mut tree = Tree::parse("<!DOCTYPE html><p>x</p>");
    let holder = tree.create_element("div");
    let comment = tree.create_comment("c-d");
    let pi = tree.create_processing_instruction("t", "d");
    let doctype = tree.create_document_type("q", "", "");
    let text = tree.create_text_node("a<b");
    for child in [comment, pi, doctype, text] {
        tree.append_child(holder, child);
    }
    let inner = serialize_inner(&tree, holder);
    assert_eq!(inner, "<!--c-d--><?t d?><!DOCTYPE q>a&lt;b");
    assert_eq!(serialize_inner_utf16(&tree, holder), u(&inner));
    for node in [comment, pi, doctype, text, holder] {
        assert_eq!(serialize_outer_utf16(&tree, node), u(&serialize_outer(&tree, node)));
    }
    // the parsed doctype and the document itself
    let parsed_doctype = tree.first_child(tree.root()).unwrap();
    assert_eq!(serialize_outer_utf16(&tree, parsed_doctype), u("<!DOCTYPE html>"));
    assert_eq!(serialize_outer(&tree, tree.root()), "");
    assert_eq!(serialize_outer_utf16(&tree, tree.root()), []);
    let document_inner = serialize_inner(&tree, tree.root());
    assert!(document_inner.starts_with("<!DOCTYPE html><html><head></head><body><p>x</p>"));
    assert_eq!(serialize_inner_utf16(&tree, tree.root()), u(&document_inner));
    // lone units are kept (the scalar serializer repairs them)
    let lone = tree.create_comment_utf16(&[0xd800]);
    let box_ = tree.create_element("i");
    tree.append_child(box_, lone);
    assert_eq!(serialize_inner_utf16(&tree, box_), [0x3c, 0x21, 0x2d, 0x2d, 0xd800, 0x2d, 0x2d, 0x3e]);
    assert_eq!(serialize_inner(&tree, box_), "<!--\u{fffd}-->");
}

#[test]
fn split_text_without_a_parent_returns_a_detached_tail() {
    let mut tree = Tree::parse("");
    let text = tree.create_text_node_utf16(&[0x61, 0xd83d, 0xde00, 0x62]);
    // splitting inside a surrogate pair keeps both halves as lone units
    let tail = tree.split_text_utf16(text, 2);
    assert_eq!(tree.node_value_utf16(text), Some(vec![0x61, 0xd83d]));
    assert_eq!(tree.node_value_utf16(tail), Some(vec![0xde00, 0x62]));
    assert_eq!(tree.parent(tail), None);
    // past the end clamps: the tail is empty
    let empty = tree.split_text(text, 99);
    assert_eq!(tree.node_value(empty).as_deref(), Some(""));
    assert_eq!(tree.node_value_utf16(text), Some(vec![0x61, 0xd83d]));
    assert_eq!(tree.node_type(empty), NodeType::Text);
    let _: Handle = empty;
}

#[test]
fn pending_records_account_their_lone_unit_names_until_drained() {
    use turbo_dom::rtdom::tree::CollectionBudget;
    // (namespace, name): the drained bytes are the record's lossy strings plus 2 bytes per
    // retained lone code unit
    let cases: [(Option<Vec<u16>>, Vec<u16>, usize); 5] = [
        (None, u("a"), 1),                       // "a"
        (None, vec![0xd800], 3 + 2),             // "\u{fffd}" + one lone unit
        (None, vec![0x61, 0xdc01, 0xd800], 1 + 3 + 3 + 2 * 3), // every unit kept once one is lone
        (None, vec![0x61, 0xd800, 0xdc01], 1 + 4),               // a valid pair: no units kept
        (Some(vec![0xdc00]), u("b"), 1 + 3 + 2), // name "b", namespace "\u{fffd}" + one unit
    ];
    for (namespace, name, drained) in cases {
        let mut tree = Tree::parse("");
        let el = tree.create_element("div");
        tree.start_recording();
        tree.set_attribute_ns_units(el, namespace.as_deref(), &name, &[]);
        let pending = tree.storage_stats().payload_bytes;
        assert_eq!(tree.take_mutation_events().len(), 1);
        assert_eq!(
            pending - tree.storage_stats().payload_bytes,
            drained,
            "{namespace:?} {name:?}"
        );
    }
    // collection keeps parsed comments and changed processing-instruction data
    let mut tree = Tree::parse("<p><!--parsed--></p>");
    let p = tree.query_selector("p").unwrap();
    let comment = tree.first_child(p).unwrap();
    let pi = tree.create_processing_instruction("t", "d");
    tree.append_child(p, pi);
    tree.set_text_content(pi, "changed");
    let dropped = tree.create_comment("gone");
    let budget = CollectionBudget {
        max_nodes: 1000,
        max_edges: 10_000,
        max_payload_bytes: 100_000,
    };
    let report = tree.collect_unreachable(&[], budget).unwrap();
    assert_eq!(report.reclaimed_nodes, 1);
    assert!(!tree.is_live(dropped));
    assert_eq!(tree.node_value(comment).as_deref(), Some("parsed"));
    assert_eq!(tree.node_value(pi).as_deref(), Some("changed"));
    assert_eq!(tree.processing_instruction_target(pi), Some("t"));
    assert_eq!(serialize_inner(&tree, p), "<!--parsed--><?t changed?>");
}

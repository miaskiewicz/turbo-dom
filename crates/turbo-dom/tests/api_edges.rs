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
    for ns in [
        Namespace::Html,
        Namespace::Svg,
        Namespace::MathMl,
        Namespace::Other,
    ] {
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
        (
            tree.doctype_name(parsed),
            tree.doctype_public_id(parsed),
            tree.doctype_system_id(parsed)
        ),
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
    assert_eq!(
        tree.processing_instruction_target_utf16(pi),
        Some(u("target"))
    );
    assert_eq!(tree.node_value(pi).as_deref(), Some("data"));
    for node in [parsed, element, created_element, doctype] {
        assert_eq!(tree.processing_instruction_target(node), None);
        assert_eq!(tree.processing_instruction_target_utf16(node), None);
    }
    let lone = tree.create_processing_instruction_units(&[0xd800], &[0xdc00]);
    assert_eq!(
        tree.processing_instruction_target_utf16(lone),
        Some(vec![0xd800])
    );
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
    assert_eq!(
        tree.version, version,
        "rejected mutations do not bump the version"
    );
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
    assert!(matches!(
        events[0].record,
        MutationRecord::CharacterData { .. }
    ));
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
    assert_eq!(
        tree.get_attribute_ns(svg, Some(XLINK), "href").as_deref(),
        Some("u")
    );
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
    assert_eq!(
        tree.get_attribute_units(el, &u("data-x")),
        Some(vec![0x62, 0xd801])
    );
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
        assert_eq!(
            serialize_outer_utf16(&tree, node),
            u(&serialize_outer(&tree, node))
        );
    }
    // the parsed doctype and the document itself
    let parsed_doctype = tree.first_child(tree.root()).unwrap();
    assert_eq!(
        serialize_outer_utf16(&tree, parsed_doctype),
        u("<!DOCTYPE html>")
    );
    assert_eq!(serialize_outer(&tree, tree.root()), "");
    assert_eq!(serialize_outer_utf16(&tree, tree.root()), []);
    let document_inner = serialize_inner(&tree, tree.root());
    assert!(document_inner.starts_with("<!DOCTYPE html><html><head></head><body><p>x</p>"));
    assert_eq!(
        serialize_inner_utf16(&tree, tree.root()),
        u(&document_inner)
    );
    // lone units are kept (the scalar serializer repairs them)
    let lone = tree.create_comment_utf16(&[0xd800]);
    let box_ = tree.create_element("i");
    tree.append_child(box_, lone);
    assert_eq!(
        serialize_inner_utf16(&tree, box_),
        [0x3c, 0x21, 0x2d, 0x2d, 0xd800, 0x2d, 0x2d, 0x3e]
    );
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
        (None, u("a"), 1),                                     // "a"
        (None, vec![0xd800], 3 + 2),                           // "\u{fffd}" + one lone unit
        (None, vec![0x61, 0xdc01, 0xd800], 1 + 3 + 3 + 2 * 3), // every unit kept once one is lone
        (None, vec![0x61, 0xd800, 0xdc01], 1 + 4),             // a valid pair: no units kept
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

#[test]
fn stylesheet_selectors_keep_an_escape_at_the_end_as_a_replacement_character() {
    use turbo_dom::rtdom::query::{match_for_cascade, parse_for_cascade, LiveNavigator};
    let mut tree = Tree::parse("<p>x</p>");
    let p = tree.query_selector("p").unwrap();
    let replaced = tree.create_element("p\u{fffd}");
    // a prelude is not EOF-closed: `p\` is the name `p\u{FFFD}` (CSS Syntax)
    let list = parse_for_cascade("p\\").unwrap();
    let nav = LiveNavigator(&tree);
    let matches = |h| {
        list.selectors()
            .iter()
            .any(|s| match_for_cascade(&nav, h, s))
    };
    assert!(!matches(p));
    assert!(matches(replaced));
    // and an unterminated block or string in a prelude stays an error
    assert!(parse_for_cascade("[a=\"x").is_err());
    assert!(parse_for_cascade("[a").is_err());
    assert!(
        parse_for_cascade(":is(p").is_ok(),
        "an open function argument is tolerated"
    );
}

#[test]
fn the_lenient_utf16_query_skips_a_character_it_cannot_place() {
    let tree = Tree::parse("<p>x</p><a></a>");
    let p = tree.query_selector("p").unwrap();
    // `)` cannot start a compound: the lenient API skips it (the `_checked` / binding
    // validation rejects such a selector before any query)
    assert_eq!(tree.query_selector_all_units(&u("\\70)")).to_vec(), [p]);
    assert_eq!(
        tree.query_selector_all_checked("\\70)")
            .map(|l| l.len())
            .ok(),
        None
    );
    // garbage the parser can place nowhere matches nothing
    assert_eq!(tree.query_selector_all_units(&u("!\\70")).len(), 0);
}

#[test]
fn namespaced_attributes_account_lossy_strings_plus_two_bytes_per_kept_unit() {
    // payload = namespace + prefix + local + qualified (lossy UTF-8) + value, plus 2 bytes
    // per code unit of every name part that holds a lone surrogate
    let cases: [(Vec<u16>, Vec<u16>, usize); 4] = [
        (u("urn"), u("p:a"), 3 + 1 + 1 + 3),
        (vec![0xd800], u("a"), 3 + 1 + 1 + 2), // lone namespace: 1 unit
        (u("urn"), vec![0xdc00, 0x3a, 0x61], 3 + 3 + 1 + 5 + 2 + 6), // lone prefix, qualified
        (u("urn"), vec![0x70, 0x3a, 0xd800], 3 + 1 + 3 + 5 + 2 + 6), // lone local, qualified
    ];
    for (namespace, qualified, bytes) in cases {
        let mut tree = Tree::parse("");
        let el = tree.create_element("div");
        let before = tree.storage_stats().payload_bytes;
        tree.set_attribute_ns_units(el, Some(&namespace), &qualified, &[]);
        assert_eq!(
            tree.storage_stats().payload_bytes - before,
            bytes,
            "{namespace:?} {qualified:?}"
        );
    }
}

#[test]
fn utf16_attribute_updates_record_the_replaced_value() {
    let mut tree = Tree::parse("");
    let el = tree.create_element("div");
    tree.set_attribute(el, "b", "x");
    tree.set_attribute_units(el, &u("a"), &u("1"));
    tree.start_recording();
    tree.set_attribute_units(el, &u("a"), &[0x32, 0xd800]);
    tree.set_attribute_units(el, &u("c"), &u("new"));
    let events = tree.take_mutation_events();
    let olds: Vec<_> = events
        .iter()
        .map(|e| (e.record.attribute_name(), e.record.old_value()))
        .collect();
    assert_eq!(olds, [(Some("a"), Some("1")), (Some("c"), None)]);
    assert_eq!(
        tree.get_attribute_units(el, &u("a")),
        Some(vec![0x32, 0xd800])
    );
    assert_eq!(tree.get_attribute(el, "b"), Some("x"));
}

#[test]
fn utf16_serializer_escapes_text_and_attributes_differently() {
    let mut tree = Tree::parse("");
    let i = tree.create_element("i");
    tree.set_attribute(i, "title", "a\"<>&\u{a0}");
    let text = tree.create_text_node("\"<>&\u{a0}");
    tree.append_child(i, text);
    let holder = tree.create_element("div");
    tree.append_child(holder, i);
    let expected = "<i title=\"a&quot;<>&amp;&nbsp;\">\"&lt;&gt;&amp;&nbsp;</i>";
    assert_eq!(serialize_inner(&tree, holder), expected);
    assert_eq!(serialize_inner_utf16(&tree, holder), u(expected));
}

#[test]
fn namespace_uris_select_the_namespace_and_lone_unit_names_compare_by_units() {
    let mut tree = Tree::parse("");
    for (uri, ns) in [
        ("http://www.w3.org/1999/xhtml", Namespace::Html),
        ("http://www.w3.org/2000/svg", Namespace::Svg),
        ("http://www.w3.org/1998/Math/MathML", Namespace::MathMl),
        ("urn:other", Namespace::Other),
    ] {
        let el = tree.create_element_namespace_units(Some(&u(uri)), &u("x"));
        assert_eq!(tree.namespace(el), ns, "{uri}");
        assert_eq!(tree.namespace_uri_utf16(el), Some(u(uri)));
    }
    let none = tree.create_element_namespace_units(Some(&[]), &u("x"));
    assert_eq!(
        (tree.namespace(none), tree.namespace_uri(none)),
        (Namespace::Other, None)
    );
    // `*-of-type` compares lone-unit local names by their units
    let host = tree.create_element("div");
    let kids = [
        tree.create_element_units(&[0x78, 0xd800]),
        tree.create_element_units(&[0x78, 0xd800]),
        tree.create_element_units(&[0x78, 0xdc00]),
    ];
    for kid in kids {
        tree.append_child(host, kid);
    }
    let first: Vec<bool> = kids
        .iter()
        .map(|&k| tree.matches(k, ":first-of-type"))
        .collect();
    assert_eq!(first, [true, false, true]);
}

#[test]
fn source_positions_count_a_lone_carriage_return_as_a_line_break() {
    // a lone CR, a CRLF pair and an LF each end one line; the offset counts every unit
    let tree = Tree::parse_with_source_positions("a\r<span></span>\r\n<p></p>\n<div></div>");
    let at = |tag: &str| {
        let p = tree
            .source_position(tree.query_selector(tag).unwrap())
            .unwrap();
        (p.utf16_offset, p.line, p.column)
    };
    assert_eq!(at("span"), (8, 2, 7));
    assert_eq!(at("p"), (20, 3, 4));
    assert_eq!(at("div"), (30, 4, 6));
    // elements an end tag implies have no position (only start tags record one)
    let implied = Tree::parse_with_source_positions("<div></p></div>");
    let p = implied.query_selector("p").unwrap();
    assert_eq!(implied.source_position(p), None);
    let div = implied.query_selector("div").unwrap();
    assert_eq!(
        implied.source_position(div).map(|p| p.utf16_offset),
        Some(5)
    );
    let body = implied.query_selector("body").unwrap();
    assert_eq!(implied.source_position(body), None);
}

#[test]
fn utf16_fragment_insertion_positions_and_records() {
    let lone = [0x3c, 0x62, 0x3e, 0xd800, 0x3c, 0x2f, 0x62, 0x3e]; // <b>\u{d800}</b>
    let mut tree = Tree::parse("");
    let host = tree.create_element("div");
    let first = tree.create_element("i");
    tree.append_child(host, first);
    tree.insert_adjacent_html_utf16(host, "afterbegin", &lone);
    tree.insert_adjacent_html_utf16(host, "beforeend", &lone);
    let names: Vec<_> = tree
        .children(host)
        .iter()
        .map(|&c| tree.local_name(c).unwrap().to_string())
        .collect();
    assert_eq!(names, ["b", "i", "b"]);
    let b = tree.first_child(host).unwrap();
    assert_eq!(tree.text_content_utf16(b), [0xd800]);
    // set_inner_html_utf16 records one childList batch while recording, none for empty -> empty
    tree.start_recording();
    let empty = tree.create_element("p");
    tree.set_inner_html_utf16(empty, &[0xd800, 0x3c]);
    tree.set_inner_html_utf16(empty, &[0xdc00]);
    let full = tree.create_element("p");
    tree.set_inner_html_utf16(full, &[0xd800]);
    let events = tree.take_mutation_events();
    let shapes: Vec<_> = events
        .iter()
        .map(|e| {
            (
                e.record.target(),
                e.record.added().len(),
                e.record.removed().len(),
            )
        })
        .collect();
    assert_eq!(shapes, [(empty, 1, 0), (empty, 1, 1), (full, 1, 0)]);
    let silent = tree.create_element("p");
    tree.set_inner_html_utf16(silent, &[0xd800]);
    tree.set_inner_html_utf16(silent, &[]);
    let removed: Vec<_> = tree
        .take_mutation_events()
        .iter()
        .map(|e| (e.record.added().len(), e.record.removed().len()))
        .collect();
    assert_eq!(removed, [(1, 0), (0, 1)]);
    let none = tree.create_element("p");
    tree.set_inner_html_utf16(none, &[0xd800, 0x3c, 0x21, 0x2d, 0x2d]); // `<!--` comment only
    tree.set_inner_html_utf16(none, &[]);
    assert_eq!(tree.take_mutation_events().len(), 2);
    let quiet = tree.create_element("p");
    tree.set_inner_html_utf16(quiet, &[]);
    assert_eq!(
        tree.take_mutation_events().len(),
        0,
        "empty -> empty records nothing (scalar path)"
    );
}

/// Chrome 156 throws HierarchyRequestError, leaving the tree unchanged, when a node is
/// inserted into itself or one of its descendants; the Tree API (which has no errors on
/// these paths) must leave the tree unchanged instead of building a parent cycle.
#[test]
fn inserting_a_node_into_its_own_subtree_is_a_no_op() {
    fn setup() -> (Tree, Handle, Handle, Handle) {
        let tree = Tree::parse("<div id=p><a></a><b></b><i></i></div>");
        let p = tree.query_selector("#p").unwrap();
        let kids = tree.children(p);
        (tree, p, kids[0], kids[1])
    }
    type Case = fn(&mut Tree, Handle, Handle, Handle);
    let cases: [(&str, Case); 10] = [
        ("adjacent beforebegin of child", |t, p, a, _| {
            assert!(!t.insert_adjacent_element(a, "beforebegin", p));
        }),
        ("append self", |t, p, _, _| t.append_child(p, p)),
        ("append into child", |t, p, a, _| t.append_child(a, p)),
        ("insert into child", |t, p, a, _| {
            t.insert_before(a, p, None)
        }),
        ("before child", |t, p, a, _| t.before(a, &[p])),
        ("after child", |t, p, a, _| t.after(a, &[p])),
        ("replace child with ancestor", |t, p, _, b| {
            t.replace_with(b, &[p])
        }),
        ("replace children of child", |t, p, a, _| {
            t.replace_children(a, &[p])
        }),
        ("adjacent afterbegin", |t, p, a, _| {
            assert!(!t.insert_adjacent_element(a, "afterbegin", p));
        }),
        ("adjacent beforeend", |t, p, a, _| {
            assert!(!t.insert_adjacent_element(a, "beforeend", p));
        }),
    ];
    for (name, case) in cases {
        let (mut tree, p, a, b) = setup();
        let body = tree.parent(p);
        case(&mut tree, p, a, b);
        assert_eq!(tree.parent(p), body, "{name}");
        assert_eq!(serialize_inner(&tree, p), "<a></a><b></b><i></i>", "{name}");
    }
}

/// DOM pre-insert: a reference child that is the node itself becomes its next sibling, so
/// `p.insertBefore(b, b)` leaves the order alone (Chrome 156: `<a></a><b></b><i></i>`).
#[test]
fn insert_before_itself_keeps_the_node_in_place() {
    let mut tree = Tree::parse("<div id=p><a></a><b></b><i></i></div>");
    let p = tree.query_selector("#p").unwrap();
    let b = tree.children(p)[1];
    tree.insert_before(p, b, Some(b));
    assert_eq!(serialize_inner(&tree, p), "<a></a><b></b><i></i>");
    // ...and an element may still be placed next to itself (not an ancestor of the parent).
    assert!(tree.insert_adjacent_element(b, "beforebegin", b));
    assert!(tree.insert_adjacent_element(b, "afterend", b));
    assert_eq!(serialize_inner(&tree, p), "<a></a><b></b><i></i>");
}

/// Host-including: a shadow root cannot take its host or the host's ancestors (Chrome 156:
/// HierarchyRequestError, tree unchanged).
#[test]
fn a_shadow_root_cannot_take_an_ancestor_of_its_host() {
    let mut tree = Tree::parse("<div id=p><span></span></div>");
    let p = tree.query_selector("#p").unwrap();
    let host = tree.children(p)[0];
    let body = tree.parent(p);
    let shadow = tree.attach_shadow(host);
    tree.append_child(shadow, p);
    tree.append_child(shadow, host);
    assert_eq!(tree.parent(p), body);
    assert_eq!(tree.parent(host), Some(p));
    assert!(tree.children(shadow).is_empty());
}

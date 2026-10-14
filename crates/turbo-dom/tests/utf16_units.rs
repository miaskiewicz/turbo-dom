use turbo_dom::rtdom::{
    tree::{CollectionBudget, CollectionError},
    Tree,
};
const UNITS: &[u16] = &[65, 0xd800, 66, 0xdc00, 0xd83d, 0xde00, 0];
fn budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 1000,
        max_edges: 10000,
        max_payload_bytes: 100_000,
    }
}
#[test]
fn character_payloads_and_utf16_offsets_preserve_chrome_units() {
    let mut tree = Tree::parse("<body>");
    let text = tree.create_text_node_utf16(UNITS);
    let comment = tree.create_comment_utf16(UNITS);
    assert_eq!(tree.node_value_utf16(text).unwrap(), UNITS);
    assert_eq!(tree.node_value_utf16(comment).unwrap(), UNITS);
    assert_eq!(
        tree.substring_data_utf16(text, 1, 5),
        [0xd800, 66, 0xdc00, 0xd83d, 0xde00]
    );
    tree.replace_data_utf16(text, 2, 2, &[0xd888]);
    tree.replace_data_utf16(text, usize::MAX, 0, &[0xd999]);
    assert_eq!(
        tree.node_value_utf16(text).unwrap(),
        [65, 0xd800, 0xd888, 0xd83d, 0xde00, 0, 0xd999]
    );
    let pair = tree.create_text_node("😀");
    let tail = tree.split_text_utf16(pair, 1);
    assert_eq!(tree.node_value_utf16(pair).unwrap(), [0xd83d]);
    assert_eq!(tree.node_value_utf16(tail).unwrap(), [0xde00]);
}
#[test]
fn native_attribute_identity_and_detachment_keep_exact_values() {
    let mut tree = Tree::parse("<body>");
    let node = tree.create_element("div");
    tree.set_attribute_utf16(node, "data-u", UNITS);
    let attr = tree.attribute_node(node, None, "data-u").unwrap();
    assert_eq!(tree.node_value_utf16(attr).unwrap(), UNITS);
    tree.remove_attribute(node, "data-u");
    assert_eq!(tree.attribute_owner(attr), None);
    assert_eq!(tree.node_value_utf16(attr).unwrap(), UNITS);
    tree.set_attribute_node(node, attr).unwrap();
    assert_eq!(tree.get_attribute_utf16(node, "data-u").unwrap(), UNITS);
    tree.set_attribute_value_utf16(attr, &[0xdfff]);
    assert_eq!(tree.get_attribute_utf16(node, "data-u").unwrap(), [0xdfff]);
    tree.set_attribute_ns_utf16(node, Some("urn:u"), "u:data", UNITS);
    let namespaced = tree.attribute_node(node, Some("urn:u"), "data").unwrap();
    tree.remove_attribute_ns(node, Some("urn:u"), "data");
    assert_eq!(tree.node_value_utf16(namespaced).unwrap(), UNITS);
    tree.set_attribute_value(namespaced, "scalar");
    assert_eq!(
        tree.node_value_utf16(namespaced).unwrap(),
        "scalar".encode_utf16().collect::<Vec<_>>()
    );
}
#[test]
fn legacy_utf8_writes_keep_lossless_mutation_old_values_and_then_clear_units() {
    let mut tree = Tree::parse("<body>");
    let text = tree.create_text_node_utf16(UNITS);
    let owner = tree.create_element("div");
    tree.append_child(owner, text);
    tree.set_attribute_utf16(owner, "data-u", UNITS);
    tree.start_recording();
    tree.set_text_content(text, "scalar");
    tree.set_attribute(owner, "data-u", "scalar");
    let events = tree.take_mutation_events_utf16();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].old_value.as_deref(), Some(UNITS));
    assert_eq!(events[1].old_value.as_deref(), Some(UNITS));
    assert_eq!(
        tree.node_value_utf16(text).unwrap(),
        "scalar".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        tree.get_attribute_utf16(owner, "data-u").unwrap(),
        "scalar".encode_utf16().collect::<Vec<_>>()
    );
    tree.set_text_content(text, "next");
    assert_eq!(
        tree.take_mutation_events_utf16()[0].old_value.as_deref(),
        Some(&[115, 99, 97, 108, 97, 114][..])
    );
}
#[test]
fn collection_preserves_units_after_buffer_materialization_and_reclaims_all_native_payload() {
    let mut tree = Tree::parse("<body><p>parsed</p></body>");
    let p = tree.query_selector("p").unwrap();
    let parsed = tree.first_child(p).unwrap();
    tree.set_text_content_utf16(parsed, UNITS);
    tree.set_attribute_utf16(p, "data-u", UNITS);
    tree.collect_unreachable(&[], budget()).unwrap();
    assert_eq!(tree.node_value_utf16(parsed).unwrap(), UNITS);
    assert_eq!(tree.get_attribute_utf16(p, "data-u").unwrap(), UNITS);
    let detached = tree.create_text_node_utf16(&vec![0xd800; 1000]);
    let kept = tree.create_comment_utf16(UNITS);
    let before = tree.storage_stats();
    let report = tree.collect_unreachable(&[kept], budget()).unwrap();
    assert!(!tree.is_live(detached));
    assert!(report.after.payload_bytes + 2000 < before.payload_bytes);
    assert_eq!(tree.node_value_utf16(kept).unwrap(), UNITS);
}
#[test]
fn exact_unit_payload_and_pending_old_values_are_admitted_before_atomic_collection() {
    let mut tree = Tree::parse("<body>");
    let text = tree.create_text_node_utf16(&vec![0xd800; 1000]);
    tree.start_recording();
    tree.set_text_content(text, "");
    let before = tree.storage_stats();
    let version = tree.version;
    let mut tiny = budget();
    tiny.max_payload_bytes = 1900;
    assert_eq!(
        tree.collect_unreachable(&[], tiny),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert_eq!(tree.version, version);
    let events = tree.take_mutation_events_utf16();
    assert_eq!(events[0].old_value.as_ref().unwrap().len(), 1000);
    tree.collect_unreachable(&[], budget()).unwrap();
    assert!(!tree.is_live(text));
}

#[test]
fn utf16_serialization_preserves_lone_units_and_matches_scalar_serialization() {
    use turbo_dom::rtdom::serialize::{
        serialize_inner, serialize_inner_utf16, serialize_outer_utf16,
    };
    let mut tree=Tree::parse("<div><svg><use xlink:href='#value'></use></svg><script>a < b</script><input value='a &amp; b'></div>");
    let div = tree.query_selector("div").unwrap();
    assert_eq!(
        serialize_inner_utf16(&tree, div),
        serialize_inner(&tree, div)
            .encode_utf16()
            .collect::<Vec<_>>()
    );
    tree.set_text_content_utf16(div, &[0xd800, 38, 60, 62, 160, 0xdc00, 0]);
    tree.set_attribute_utf16(div, "data-u", &[0xd800, 38, 34, 160, 0xdc00, 0]);
    let mut expected = "<div data-u=\"".encode_utf16().collect::<Vec<_>>();
    expected.push(0xd800);
    expected.extend("&amp;&quot;&nbsp;".encode_utf16());
    expected.extend([0xdc00, 0]);
    expected.extend("\">".encode_utf16());
    expected.push(0xd800);
    expected.extend("&amp;&lt;&gt;&nbsp;".encode_utf16());
    expected.extend([0xdc00, 0]);
    expected.extend("</div>".encode_utf16());
    assert_eq!(serialize_outer_utf16(&tree, div), expected);
}

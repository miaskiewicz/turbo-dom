use turbo_dom::rtdom::{
    serialize::serialize_outer_utf16,
    tree::{CollectionBudget, CollectionError},
    Tree,
};
fn budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 1000,
        max_edges: 10_000,
        max_payload_bytes: 100_000,
    }
}
#[test]
fn actual_native_names_and_namespace_identities_keep_high_low_and_fffd_distinct() {
    let mut tree = Tree::parse("<body>");
    let values = [[0xd800], [0xdc00], [0xfffd]];
    for value in values {
        let node = tree.create_element_units(&value);
        assert_eq!(tree.local_name_utf16(node).unwrap(), value);
        assert_eq!(tree.tag_name_utf16(node).unwrap(), value);
        assert_eq!(tree.local_name_is(node, "�"), value == [0xfffd]);
        let foreign = tree.create_element_namespace_units(Some(&value), &[120]);
        assert_eq!(tree.namespace_uri_utf16(foreign).unwrap(), value);
        let attribute = tree.create_attribute_units(Some(&value), &[120], &[65]);
        assert_eq!(
            tree.attribute_data_units(attribute)
                .unwrap()
                .namespace
                .as_deref(),
            Some(&value[..])
        );
    }
    let node = tree.create_element_namespace_units(Some(&[0xd800]), &[0xdc00, 58, 0xd800]);
    assert_eq!(tree.prefix_utf16(node).unwrap(), [0xdc00]);
    assert_eq!(tree.local_name_utf16(node).unwrap(), [0xd800]);
    assert_eq!(
        tree.qualified_name_utf16(node).unwrap(),
        [0xdc00, 58, 0xd800]
    );
}
#[test]
fn namespace_attribute_identity_and_unit_names_coexist_and_survive_detach_and_collection() {
    let mut tree = Tree::parse("<body>");
    let owner = tree.create_element("div");
    tree.append_child(tree.root(), owner);
    let values = [[0xd800], [0xdc00], [0xfffd]];
    for (index, value) in values.iter().enumerate() {
        tree.set_attribute_ns_units(owner, Some(value), &[120], &[48 + index as u16]);
    }
    assert_eq!(tree.attributes_units(owner).len(), 3);
    for (index, value) in values.iter().enumerate() {
        assert_eq!(
            tree.get_attribute_ns_units(owner, Some(value), &[120])
                .unwrap(),
            [48 + index as u16]
        );
    }
    assert_eq!(tree.get_attribute_ns(owner, Some("�"), "x").unwrap(), "2");
    let first = tree
        .attribute_node_units(owner, Some(&values[0]), &[120])
        .unwrap();
    let second = tree
        .attribute_node_units(owner, Some(&values[1]), &[120])
        .unwrap();
    assert_ne!(first, second);
    tree.remove_attribute_node(owner, first).unwrap();
    assert_eq!(tree.namespace_uri_utf16(first).unwrap(), values[0]);
    tree.set_attribute_node(owner, first).unwrap();
    tree.set_attribute_units(owner, &[0xd800], &[65]);
    tree.set_attribute_units(owner, &[0xdc00], &[66]);
    tree.set_attribute_units(owner, &[0xfffd], &[67]);
    assert_eq!(tree.get_attribute_units(owner, &[0xd800]).unwrap(), [65]);
    assert_eq!(tree.get_attribute_units(owner, &[0xdc00]).unwrap(), [66]);
    assert_eq!(tree.get_attribute(owner, "�"), Some("C"));
    tree.collect_unreachable(&[], budget()).unwrap();
    assert_eq!(tree.namespace_uri_utf16(first).unwrap(), values[0]);
    assert_eq!(tree.attributes_units(owner).len(), 6);
    assert_eq!(tree.get_attribute_units(owner, &[0xd800]).unwrap(), [65]);
    let output = serialize_outer_utf16(&tree, owner);
    assert!(output.contains(&0xd800));
    assert!(output.contains(&0xdc00));
}
#[test]
fn native_unpaired_name_and_namespace_payload_is_admitted_before_collection() {
    let mut tree = Tree::parse("<body>");
    let units = vec![0xd800; 200];
    let node = tree.create_element_namespace_units(Some(&units), &units);
    tree.set_attribute_ns_units(node, Some(&units), &units, &[65]);
    let before = tree.storage_stats();
    let version = tree.version;
    let mut tiny = budget();
    tiny.max_payload_bytes = before.payload_bytes.saturating_sub(1);
    assert_eq!(
        tree.collect_unreachable(&[node], tiny),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert_eq!(tree.version, version);
    tree.collect_unreachable(&[node], budget()).unwrap();
    assert_eq!(tree.namespace_uri_utf16(node).unwrap(), units);
    assert_eq!(tree.local_name_utf16(node).unwrap(), units);
    let before = tree.storage_stats().payload_bytes;
    tree.collect_unreachable(&[], budget()).unwrap();
    assert!(!tree.is_live(node));
    assert!(tree.storage_stats().payload_bytes < before);
}

#[test]
fn mutation_names_and_namespaces_and_old_units_are_original_native_values() {
    let mut tree = Tree::parse("<body>");
    let node = tree.create_element("div");
    tree.set_attribute_ns_units(node, Some(&[0xd800]), &[0xdc00], &[0xd800]);
    tree.start_recording();
    tree.set_attribute_ns_units(node, Some(&[0xd800]), &[0xdc00], &[0xdc00]);
    tree.remove_attribute_ns_units(node, Some(&[0xd800]), &[0xdc00]);
    let events = tree.take_mutation_events_utf16();
    assert_eq!(events.len(), 2);
    for event in &events {
        assert_eq!(event.attribute_name.as_deref(), Some(&[0xdc00][..]));
        assert_eq!(event.attribute_namespace.as_deref(), Some(&[0xd800][..]));
    }
    assert_eq!(events[0].old_value.as_deref(), Some(&[0xd800][..]));
    assert_eq!(events[1].old_value.as_deref(), Some(&[0xdc00][..]));
    tree.set_attribute_ns_units(node, Some(&[0xd800]), &[120], &[65]);
    tree.take_mutation_events();
    tree.set_attribute_utf16(node, "x", &[0xdc00]);
    assert_eq!(
        tree.get_attribute_ns_units(node, Some(&[0xd800]), &[120])
            .unwrap(),
        [0xdc00]
    );
    let event = tree.take_mutation_events_utf16().pop().unwrap();
    assert_eq!(event.attribute_namespace.as_deref(), Some(&[0xd800][..]));
}
#[test]
fn native_selector_matching_preserves_string_units_and_element_type_identity() {
    let mut tree = Tree::parse("<body>");
    let body = tree.query_selector("body").unwrap();
    let values = [[0xd800], [0xdc00], [0xfffd]];
    let nodes = values.map(|units| {
        let node = tree.create_element_units(&units);
        tree.set_attribute_utf16(node, "id", &units);
        tree.set_attribute_utf16(node, "class", &units);
        tree.append_child(body, node);
        node
    });
    for (index, units) in values.iter().enumerate() {
        assert_eq!(tree.get_element_by_id_units(units), Some(nodes[index]));
        let source = [vec![91, 105, 100, 61, 34], units.to_vec(), vec![34, 93]].concat();
        assert_eq!(
            tree.query_selector_all_units(&source).as_ref(),
            &[nodes[index]]
        );
        assert!(tree.matches(nodes[index], ":only-of-type"));
    }
    assert_eq!(tree.get_element_by_id("�"), Some(nodes[2]));
    assert_eq!(tree.get_elements_by_tag_name("�"), vec![nodes[2]]);
    assert_eq!(
        tree.query_selector_all_units(&[92, 100, 56, 48, 48])
            .as_ref(),
        &[nodes[2]]
    );
    let mut value = "prefix ".encode_utf16().collect::<Vec<_>>();
    value.push(0xd800);
    value.extend(" suffix".encode_utf16());
    tree.set_attribute_utf16(nodes[0], "data-u", &value);
    assert!(tree.matches(nodes[0], "[data-u]"));
    assert!(tree.matches(nodes[0], "[data-u^='prefix']"));
    assert!(tree.matches(nodes[0], "[data-u$='suffix']"));
    assert!(!tree.matches(nodes[0], "[data-u='prefix � suffix']"));
}

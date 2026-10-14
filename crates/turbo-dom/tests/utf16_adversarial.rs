use serde_json::{json, Value};
use turbo_dom::rtdom::{
    tree::{CollectionBudget, CollectionError, Handle, NodeType},
    Tree,
};
fn units(source: &str) -> Vec<u16> {
    source.encode_utf16().collect()
}
fn budget(bytes: usize) -> CollectionBudget {
    CollectionBudget {
        max_nodes: 10_000,
        max_edges: 100_000,
        max_payload_bytes: bytes,
    }
}
fn control() -> Value {
    serde_json::from_str(include_str!("fixtures/utf16-adversarial-chrome155.json")).unwrap()
}
#[test]
fn id_class_and_css_attribute_lookup_use_exact_null_namespace_identity() {
    let oracle = control();
    for before in [false, true] {
        for value in [vec![0xd800], vec![0xdc00], units("�")] {
            let mut tree = Tree::parse("<body>");
            let body = tree.query_selector("body").unwrap();
            let node = tree.create_element("div");
            tree.append_child(body, node);
            for name in ["id", "class"] {
                if before {
                    tree.set_attribute_ns_utf16(node, Some("urn:adversarial"), name, &value);
                }
                tree.set_attribute_ns(node, None, name, "t");
                if !before {
                    tree.set_attribute_ns_utf16(node, Some("urn:adversarial"), name, &value);
                }
            }
            let expected = &oracle["result"][format!("namespacedId{before}")];
            for collected in [false, true] {
                if collected {
                    tree.collect_unreachable(&[], budget(100_000)).unwrap();
                }
                assert_eq!(tree.get_element_by_id("t") == Some(node), expected["byId"]);
                assert_eq!(tree.get_element_by_id_units(&units("t")), Some(node));
                for selector in ["#t", ".t", "[id='t']", "[class='t']"] {
                    assert!(tree.matches(node, selector), "{selector} {before}");
                    assert!(tree.matches_units(node, &units(selector)));
                }
                assert_eq!(tree.matches(node, "#t"), expected["css"]);
                assert_eq!(
                    tree.get_attribute_ns_utf16(node, Some("urn:adversarial"), "id"),
                    Some(value.clone())
                );
            }
        }
    }
    let mut tree = Tree::parse("<body>");
    let node = tree.create_element("div");
    tree.set_attribute_ns_utf16(node, Some("urn:adversarial"), "p:v", &[0xd800]);
    let selector = [units("[p\\:v='"), vec![0xd800], units("']")].concat();
    assert_eq!(
        tree.matches_units(node, &selector),
        control()["result"]["prefixed"]["actual"]
    );
    assert!(!tree.matches(node, "[p:v='�']"));
    let literal = tree.create_attribute_units(None, &units("p:v"), &[0xfffd]);
    tree.set_attribute_node(node, literal).unwrap();
    assert!(!tree.matches_units(node, &selector));
    assert!(tree.matches_units(node, &units("[p\\:v='�']")));
    // Attr identity remains distinct even when qualified names coincide.
    let attr = tree
        .attribute_node(node, Some("urn:adversarial"), "v")
        .unwrap();
    tree.remove_attribute_node(node, attr).unwrap();
    assert_eq!(tree.node_value_utf16(attr), Some(vec![0xd800]));
    tree.collect_unreachable(&[node, attr], budget(100_000))
        .unwrap();
    assert_eq!(tree.node_value_utf16(attr), Some(vec![0xd800]));
    assert!(tree.matches_units(node, &units("[p\\:v='�']")));
}
#[test]
fn collection_charges_all_doctype_fields_and_processing_instruction_units_atomically() {
    for value in [
        vec![0xd800; 100],
        vec![0xdc00; 100],
        units(&"A".repeat(100)),
    ] {
        for mask in 1..8 {
            let mut tree = Tree::parse("");
            let empty = Vec::new();
            let fields = [1, 2, 4].map(|bit| if mask & bit == 0 { &empty } else { &value });
            let node = tree.create_document_type_units(fields[0], fields[1], fields[2]);
            let before = tree.storage_stats();
            let version = tree.version;
            assert_eq!(
                tree.collect_unreachable(&[node], budget(before.payload_bytes - 1)),
                Err(CollectionError::BudgetExceeded)
            );
            assert_eq!(tree.storage_stats(), before);
            assert_eq!(tree.version, version);
            assert!(tree.is_live(node));
            assert_eq!(tree.doctype_name_utf16(node), Some(fields[0].clone()));
            assert_eq!(tree.doctype_public_id_utf16(node), Some(fields[1].clone()));
            assert_eq!(tree.doctype_system_id_utf16(node), Some(fields[2].clone()));
            let result = tree
                .collect_unreachable(&[node], budget(before.payload_bytes))
                .unwrap();
            assert!(result.after.payload_bytes <= before.payload_bytes);
            assert_eq!(tree.doctype_name_utf16(node), Some(fields[0].clone()));
        }
        for mask in 1..4 {
            let mut tree = Tree::parse("");
            let scalar = units("A");
            let target = if mask & 1 == 0 { &scalar } else { &value };
            let data = if mask & 2 == 0 { &scalar } else { &value };
            let node = tree.create_processing_instruction_units(target, data);
            let before = tree.storage_stats();
            let version = tree.version;
            assert_eq!(
                tree.collect_unreachable(&[node], budget(before.payload_bytes - 1)),
                Err(CollectionError::BudgetExceeded)
            );
            assert_eq!(tree.storage_stats(), before);
            assert_eq!(tree.version, version);
            assert_eq!(
                tree.processing_instruction_target_utf16(node),
                Some(target.clone())
            );
            assert_eq!(tree.node_value_utf16(node), Some(data.clone()));
            let result = tree
                .collect_unreachable(&[node], budget(before.payload_bytes))
                .unwrap();
            assert!(result.after.payload_bytes <= before.payload_bytes);
            assert_eq!(
                tree.processing_instruction_target_utf16(node),
                Some(target.clone())
            );
            assert_eq!(tree.node_value_utf16(node), Some(data.clone()));
        }
    }
}
fn projection(tree: &Tree, parent: Handle) -> Value {
    Value::Array(
        tree.children(parent)
            .into_iter()
            .map(|node| {
                let element = tree.node_type(node) == NodeType::Element;
                json!({"type":tree.node_type_id(node), "namespace":tree.namespace_uri(node),
            "name":tree.local_name(node), "value":tree.node_value_utf16(node),
            "attribute":element.then(||tree.get_attribute_utf16(node,"data-u").unwrap_or_default()),
            "children":projection(tree,node)})
            })
            .collect(),
    )
}
#[test]
fn fragment_context_namespace_attributes_and_adjacency_match_chrome() {
    let oracle = control();
    assert_eq!(oracle["errors"], json!([]));
    for case in oracle["result"]["fragments"].as_array().unwrap() {
        let mut tree = Tree::parse("<body>");
        let namespace = case["namespace"].as_str().unwrap();
        let name = case["name"].as_str().unwrap();
        let host = tree.create_element_namespace(Some(namespace), name);
        if let Some(encoding) = case["encoding"].as_str() {
            tree.set_attribute(host, "encoding", encoding);
        }
        let source = case["source"]
            .as_array()
            .unwrap()
            .iter()
            .map(|unit| u16::try_from(unit.as_u64().unwrap()).unwrap())
            .collect::<Vec<_>>();
        let position = case["position"].as_str().unwrap();
        if position == "inner" {
            tree.set_inner_html_utf16(host, &source);
        } else if position == "beforebegin" || position == "afterend" {
            let anchor = tree.create_element_namespace(
                Some(namespace),
                if namespace.contains("/svg") {
                    "circle"
                } else {
                    "mn"
                },
            );
            tree.append_child(host, anchor);
            tree.insert_adjacent_html_utf16(anchor, position, &source);
            tree.remove_child(host, anchor);
        } else {
            tree.insert_adjacent_html_utf16(host, position, &source);
        }
        assert_eq!(
            projection(&tree, host),
            case["output"],
            "{name} {position} {source:?}"
        );
        if let Ok(scalar) = String::from_utf16(&source) {
            let mut direct = Tree::parse("<body>");
            let target = direct.create_element_namespace(Some(namespace), name);
            if let Some(encoding) = case["encoding"].as_str() {
                direct.set_attribute(target, "encoding", encoding);
            }
            direct.set_inner_html(target, &scalar);
            assert_eq!(
                projection(&direct, target),
                case["output"],
                "scalar {name} {source:?}"
            );
        }
    }
}
#[test]
fn css_hex_escape_terminators_keep_identifier_and_following_delimiters() {
    let tree = Tree::parse("<ab id='ab' class='ab' ab='v'></ab>");
    let node = tree.query_selector("ab").unwrap();
    assert_eq!(
        tree.matches_units(node, &units(".\\61\r\nb")),
        control()["result"]["hexCRLF"]
    );
    for digits in ["61", "061", "0061", "00061", "000061"] {
        for terminator in ["\r\n", "\r", "\n", " ", "\t", "\u{c}"] {
            let escaped = format!("\\{digits}{terminator}b");
            for selector in [
                escaped.clone(),
                format!("#{escaped}"),
                format!(".{escaped}"),
                format!("[{escaped}='v']"),
                format!(".{escaped}[ab='v']"),
            ] {
                assert!(tree.matches_units(node, &units(&selector)), "{selector:?}");
            }
        }
    }
}

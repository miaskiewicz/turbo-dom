use serde_json::{json, Value};
use turbo_dom::rtdom::mutations::{MutationEvent, MutationObserver, MutationRecord, ObserverInit};
use turbo_dom::rtdom::{Handle, Tree};

fn identity(tree: &Tree, node: Option<Handle>) -> Value {
    node.map_or(Value::Null, |node| {
        json!(tree.get_attribute(node, "id").map_or_else(
            || if tree.node_type_id(node) == 3 {
                "#text".to_string()
            } else {
                tree.tag_name(node).unwrap_or_default().to_uppercase()
            },
            str::to_string,
        ))
    })
}
fn record(tree: &Tree, event: &MutationEvent) -> Value {
    let kind = match event.record {
        MutationRecord::ChildList { .. } => "childList",
        MutationRecord::Attributes { .. } => "attributes",
        MutationRecord::CharacterData { .. } => "characterData",
    };
    json!({"type":kind,"target":identity(tree,Some(event.record.target())),
      "added":event.record.added().iter().map(|node|identity(tree,Some(*node))).collect::<Vec<_>>(),
      "removed":event.record.removed().iter().map(|node|identity(tree,Some(*node))).collect::<Vec<_>>(),
      "previous":identity(tree,event.previous_sibling),"next":identity(tree,event.next_sibling),
      "attribute":event.record.attribute_name(),"namespace":event.attribute_namespace,"oldValue":event.record.old_value()})
}
fn read(tree: &mut Tree, rows: &mut Value, name: &str) {
    let events = tree.take_mutation_events();
    rows[name] = json!(events
        .iter()
        .map(|event| record(tree, event))
        .collect::<Vec<_>>());
}

#[test]
fn native_mutation_events_match_independently_captured_chrome_records() {
    let mut tree=Tree::parse("<section id=source><i id=first></i><b id=node></b><u id=last></u></section><aside id=target><em id=reference></em></aside>");
    let source = tree.query_selector("section").unwrap();
    let target = tree.query_selector("aside").unwrap();
    let node = tree.query_selector("b").unwrap();
    let reference = tree.query_selector("em").unwrap();
    let mut rows = json!({});
    tree.start_recording();
    tree.insert_before(target, node, Some(reference));
    read(&mut tree, &mut rows, "move");
    tree.set_attribute_ns(node, Some("urn:authored"), "p:value", "first");
    tree.set_attribute(node, "p:value", "second");
    tree.remove_attribute(node, "p:value");
    read(&mut tree, &mut rows, "attributes");
    tree.set_inner_html(source, "<p id=replacement>one</p>tail");
    let events = tree.take_mutation_events();
    rows["innerHTML"] = json!(events
        .iter()
        .filter(|event| event.record.target() == source
            && matches!(event.record, MutationRecord::ChildList { .. }))
        .map(|event| record(&tree, event))
        .collect::<Vec<_>>());
    let mut observer = MutationObserver::new();
    observer.observe(
        &mut tree,
        target,
        ObserverInit {
            attributes: true,
            subtree: true,
            attribute_old_value: true,
            ..ObserverInit::default()
        },
    );
    tree.set_attribute(node, "class", "changed");
    tree.append_child(source, node);
    rows["mutationTimeAncestry"] = json!(observer
        .take_records(&mut tree)
        .into_iter()
        .map(|payload| record(
            &tree,
            &MutationEvent {
                record: payload,
                inclusive_ancestors: Vec::new(),
                previous_sibling: None,
                next_sibling: None,
                attribute_namespace: None
            }
        ))
        .collect::<Vec<_>>());
    tree.set_text_content(target, "");
    tree.take_mutation_events();
    tree.remove_attribute(target, "absent");
    tree.set_text_content(target, "");
    tree.set_inner_html(target, "");
    read(&mut tree, &mut rows, "nonMutations");
    let host = tree.create_element("div");
    tree.set_attribute(host, "id", "host");
    let shadow = tree.attach_shadow(host);
    let inside = tree.create_element("b");
    tree.set_attribute(inside, "id", "inside");
    tree.append_child(shadow, inside);
    tree.take_mutation_events();
    tree.set_attribute(inside, "class", "changed");
    let events = tree.take_mutation_events();
    rows["shadow"] = json!(events
        .iter()
        .filter(|event| event.inclusive_ancestors.contains(&shadow))
        .map(|event| record(&tree, event))
        .collect::<Vec<_>>());
    rows["shadowHost"] = json!(events
        .iter()
        .filter(|event| event.inclusive_ancestors.contains(&host))
        .map(|event| record(&tree, event))
        .collect::<Vec<_>>());
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/mutation-events-chrome155.json")).unwrap();
    assert_eq!(rows, fixture["result"]);
}

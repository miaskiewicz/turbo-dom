use turbo_dom::rtdom::mutations::{MutationEvent, MutationObserver, MutationRecord, ObserverInit};
use turbo_dom::rtdom::Tree;

#[test]
fn moving_nodes_records_removal_and_insertion_with_original_sibling_positions() {
    let mut tree = Tree::parse("<section><i></i><b></b><u></u></section><aside><em></em></aside>");
    let source = tree.query_selector("section").unwrap();
    let target = tree.query_selector("aside").unwrap();
    let first = tree.query_selector("i").unwrap();
    let node = tree.query_selector("b").unwrap();
    let last = tree.query_selector("u").unwrap();
    let reference = tree.query_selector("em").unwrap();
    tree.start_recording();
    tree.insert_before(target, node, Some(reference));
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(events[0].record.target(), source);
    assert_eq!(events[0].record.removed(), &[node]);
    assert_eq!(events[0].previous_sibling, Some(first));
    assert_eq!(events[0].next_sibling, Some(last));
    assert_eq!(events[1].record.target(), target);
    assert_eq!(events[1].record.added(), &[node]);
    assert_eq!(events[1].previous_sibling, None);
    assert_eq!(events[1].next_sibling, Some(reference));
    assert!(events[0].inclusive_ancestors.contains(&source));
    assert!(!events[0].inclusive_ancestors.contains(&target));
    assert!(events[1].inclusive_ancestors.contains(&target));
    assert_eq!(tree.parent(node), Some(target));
}

#[test]
fn attribute_namespace_and_ancestry_are_captured_before_later_detachment() {
    let mut tree = Tree::parse("<section><b></b></section>");
    let section = tree.query_selector("section").unwrap();
    let node = tree.query_selector("b").unwrap();
    tree.start_recording();
    tree.set_attribute_ns(node, Some("urn:authored"), "p:value", "first");
    tree.set_attribute(node, "p:value", "second");
    tree.remove_attribute(node, "p:value");
    tree.remove_child(section, node);
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 4, "{events:?}");
    for event in &events[..3] {
        assert_eq!(event.record.attribute_name(), Some("value"));
        assert_eq!(event.attribute_namespace.as_deref(), Some("urn:authored"));
        assert!(event.inclusive_ancestors.contains(&section));
        assert_eq!(event.previous_sibling, None);
        assert_eq!(event.next_sibling, None);
    }
    assert_eq!(events[1].record.old_value(), Some("first"));
    assert_eq!(events[2].record.old_value(), Some("second"));
    assert_eq!(tree.parent(node), None);
}

#[test]
fn inner_html_reports_one_child_replacement_with_retained_removed_nodes() {
    let mut tree = Tree::parse("<section><b>old</b>text</section>");
    let section = tree.query_selector("section").unwrap();
    let removed = tree.children(section);
    tree.start_recording();
    tree.set_inner_html(section, "<p>one</p>tail");
    let events = tree.take_mutation_events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(matches!(events[0].record, MutationRecord::ChildList { .. }));
    assert_eq!(events[0].record.target(), section);
    assert_eq!(events[0].record.removed(), removed);
    assert_eq!(events[0].record.added(), tree.children(section));
    assert_eq!(events[0].previous_sibling, None);
    assert_eq!(events[0].next_sibling, None);
    for node in removed {
        assert_eq!(tree.parent(node), None);
    }
}

#[test]
fn shadow_mutation_ancestry_stops_at_the_shadow_root() {
    let mut tree = Tree::parse("<div></div>");
    let host = tree.query_selector("div").unwrap();
    let shadow = tree.attach_shadow(host);
    let node = tree.create_element("b");
    tree.append_child(shadow, node);
    tree.start_recording();
    tree.set_attribute(node, "class", "changed");
    let events = tree.take_mutation_events();
    assert_eq!(events[0].inclusive_ancestors, [node, shadow]);
    assert_eq!(events[0].attribute_namespace, None);
}

#[test]
fn legacy_observer_filters_using_mutation_time_ancestry_and_shares_the_drain() {
    let mut tree = Tree::parse("<section><b class=first></b></section><aside></aside>");
    let source = tree.query_selector("section").unwrap();
    let target = tree.query_selector("aside").unwrap();
    let node = tree.query_selector("b").unwrap();
    let mut observer = MutationObserver::new();
    observer.observe(
        &mut tree,
        source,
        ObserverInit {
            attributes: true,
            subtree: true,
            attribute_old_value: true,
            ..ObserverInit::default()
        },
    );
    tree.set_attribute(node, "class", "second");
    tree.append_child(target, node);
    let records = observer.take_records(&mut tree);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].old_value(), Some("first"));
    assert_eq!(tree.take_mutation_events(), [] as [MutationEvent; 0]);
    tree.set_attribute(node, "class", "third");
    assert_eq!(tree.take_mutation_records().len(), 1);
    assert_eq!(tree.take_mutation_events(), [] as [MutationEvent; 0]);
}

#[test]
fn non_mutations_do_not_emit_events_and_recording_remains_opt_in() {
    let mut tree = Tree::parse("<section><b></b></section><aside></aside>");
    let section = tree.query_selector("section").unwrap();
    let aside = tree.query_selector("aside").unwrap();
    let node = tree.query_selector("b").unwrap();
    tree.set_attribute(node, "id", "before-recording");
    assert_eq!(tree.take_mutation_events(), [] as [MutationEvent; 0]);
    tree.start_recording();
    tree.remove_child(aside, node);
    tree.remove_attribute(node, "absent");
    tree.remove_attribute_ns(node, Some("urn:absent"), "value");
    tree.set_text_content(aside, "");
    tree.set_inner_html(aside, "");
    assert_eq!(tree.take_mutation_events(), [] as [MutationEvent; 0]);
    assert_eq!(tree.parent(node), Some(section));
    tree.stop_recording();
    tree.remove_child(section, node);
    assert_eq!(tree.take_mutation_events(), [] as [MutationEvent; 0]);
}

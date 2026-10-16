use sha2::{Digest, Sha256};
use turbo_dom::rtdom::{
    serialize::serialize_outer,
    tree::{CollectionBudget, CollectionError, Handle},
    Tree,
};

fn units(value: &serde_json::Value) -> Vec<u16> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| u16::try_from(unit.as_u64().unwrap()).unwrap())
        .collect()
}
fn budget() -> CollectionBudget {
    CollectionBudget {
        max_nodes: 100_000,
        max_edges: 1_000_000,
        max_payload_bytes: 16_000_000,
    }
}
#[test]
fn exact_state_membership_and_css_identifiers_match_independent_chrome() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/custom-state/chrome155.json")).unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(include_bytes!("fixtures/custom-state/probe.js"))
    );
    assert_eq!(hash, fixture["probeSha256"]);
    assert_eq!(fixture["errors"], serde_json::json!([]));
    let mut tree = Tree::parse("<body><x-native-state-control></x-native-state-control>");
    let element = tree.query_selector("x-native-state-control").unwrap();
    let original = serialize_outer(&tree, element);
    tree.start_recording();
    for row in fixture["result"]["rows"].as_array().unwrap() {
        tree.clear_custom_states(element);
        let state = units(&row["units"]);
        assert_eq!(row["error"], serde_json::Value::Null);
        assert!(tree.set_custom_state_utf16(element, &state, true));
        let version = tree.version;
        assert!(!tree.set_custom_state_utf16(element, &state, true));
        assert_eq!(tree.version, version);
        assert!(tree.has_custom_state_utf16(element, &state));
        assert_eq!(tree.custom_states_utf16(element), vec![state]);
        let selector = units(&row["selectorUnits"]);
        let expected = row["match"]["value"].as_bool().unwrap_or(false);
        assert_eq!(tree.matches_units(element, &selector), expected, "{row}");
        let scalar = String::from_utf16_lossy(&selector);
        assert_eq!(tree.matches(element, &scalar), expected, "{row}");
        assert_eq!(
            tree.query_selector(&scalar),
            expected.then_some(element),
            "{row}"
        );
        assert_eq!(
            tree.matches(element, &scalar.replacen(":state(", ":STATE(", 1)),
            expected,
            "{row}"
        );
    }
    tree.clear_custom_states(element);
    tree.set_custom_state_utf16(element, &"persist".encode_utf16().collect::<Vec<_>>(), true);
    for row in fixture["result"]["selectors"].as_array().unwrap() {
        let selector = row["selector"].as_str().unwrap();
        let expected = row["match"]["value"].as_bool().unwrap_or(false);
        assert_eq!(tree.matches(element, selector), expected, "{row}");
        assert_eq!(
            tree.matches_units(element, &selector.encode_utf16().collect::<Vec<_>>()),
            expected,
            "{row}"
        );
        if row["match"]["error"] == "SyntaxError" {
            assert!(tree.matches_checked(element, selector).is_err(), "{row}");
        } else {
            assert_eq!(
                tree.matches_checked(element, selector),
                Ok(expected),
                "{row}"
            );
        }
    }
    assert_eq!(serialize_outer(&tree, element), original);
    assert_eq!(tree.take_mutation_events(), []);
}
#[test]
fn state_queries_and_styles_invalidate_only_on_membership_changes() {
    let mut tree=Tree::parse("<style>x-control{color:red}x-control:state(ready){color:blue}</style><x-control></x-control>");
    let element = tree.query_selector("x-control").unwrap();
    let ready: Vec<_> = "ready".encode_utf16().collect();
    assert_eq!(
        tree.query_selector_all(":state(ready)").as_ref(),
        &[] as &[Handle]
    );
    assert_eq!(
        turbo_dom::rtdom::cascade::get_property_value(
            &turbo_dom::rtdom::cascade::computed_style(&tree, element),
            "color"
        ),
        "rgb(255, 0, 0)"
    );
    let version = tree.version;
    assert!(!tree.clear_custom_states(element));
    assert!(!tree.set_custom_state_utf16(element, &ready, false));
    assert_eq!(tree.version, version);
    assert!(tree.set_custom_state_utf16(element, &ready, true));
    assert_eq!(tree.query_selector(":state(ready)"), Some(element));
    assert!(tree.matches(element, ":is(:state(ready)):not(:state(missing))"));
    assert_eq!(
        turbo_dom::rtdom::cascade::get_property_value(
            &turbo_dom::rtdom::cascade::computed_style(&tree, element),
            "color"
        ),
        "rgb(0, 0, 255)"
    );
    assert!(tree.set_custom_state_utf16(element, &ready, false));
    assert_eq!(
        tree.query_selector_all(":state(ready)").as_ref(),
        &[] as &[Handle]
    );
    assert_eq!(
        turbo_dom::rtdom::cascade::get_property_value(
            &turbo_dom::rtdom::cascade::computed_style(&tree, element),
            "color"
        ),
        "rgb(255, 0, 0)"
    );
    assert!(tree.set_custom_state_utf16(element, &ready, true));
    assert!(tree.clear_custom_states(element));
    assert!(!tree.has_custom_state_utf16(element, &ready));
    assert!(!tree.set_custom_state_utf16(tree.root(), &ready, true));
    assert!(!tree.set_custom_state_utf16(Handle::from_raw(u32::MAX), &ready, true));
}
#[test]
fn exact_units_do_not_alias_replacement_characters_and_adoption_uses_public_state_seam() {
    let mut source = Tree::parse("<x-state></x-state>");
    let old = source.query_selector("x-state").unwrap();
    for key in [
        &[0xd800][..],
        &[0xdc00][..],
        &[0xfffd][..],
        &[][..],
        &[0][..],
    ] {
        assert!(source.set_custom_state_utf16(old, key, true));
    }
    assert_eq!(source.custom_states_utf16(old).len(), 5);
    let mut target = Tree::parse("<x-state></x-state><x-state></x-state>");
    let nodes = target.query_selector_all("x-state");
    let adopted = nodes[0];
    let cloned = nodes[1];
    for key in source.custom_states_utf16(old) {
        assert!(target.set_custom_state_utf16(adopted, &key, true));
    }
    assert!(source.clear_custom_states(old));
    for key in [
        &[0xd800][..],
        &[0xdc00][..],
        &[0xfffd][..],
        &[][..],
        &[0][..],
    ] {
        assert!(target.has_custom_state_utf16(adopted, key));
        assert!(!target.has_custom_state_utf16(cloned, key));
    }
    assert!(target.matches_units(adopted, &[58, 115, 116, 97, 116, 101, 40, 0xd800, 41]));
    assert!(target.set_custom_state_utf16(adopted, &[0xfffd], false));
    assert!(!target.matches_units(adopted, &[58, 115, 116, 97, 116, 101, 40, 0xd800, 41]));
    assert!(target.has_custom_state_utf16(adopted, &[0xd800]));
}
#[test]
fn custom_state_payload_respects_collection_admission_and_reclaimed_handles() {
    let mut tree = Tree::parse("<main><x-state></x-state></main>");
    let element = tree.query_selector("x-state").unwrap();
    let before = tree.storage_stats();
    let large = vec![65; 65536];
    assert!(tree.set_custom_state_utf16(element, &large, true));
    assert_eq!(
        tree.storage_stats().payload_bytes,
        before.payload_bytes + large.len() * 2
    );
    let bounds = CollectionBudget {
        max_payload_bytes: 1000,
        ..budget()
    };
    assert_eq!(
        tree.collect_unreachable(&[], bounds),
        Err(CollectionError::BudgetExceeded)
    );
    assert!(tree.has_custom_state_utf16(element, &large));
    let parent = tree.parent(element).unwrap();
    tree.remove_child(parent, element);
    tree.collect_unreachable(&[element], budget()).unwrap();
    assert!(tree.is_live(element));
    assert!(tree.has_custom_state_utf16(element, &large));
    let report = tree.collect_unreachable(&[], budget()).unwrap();
    assert!(report.reclaimed_nodes >= 1);
    assert!(!tree.is_live(element));
    assert!(!tree.has_custom_state_utf16(element, &large));
    assert_eq!(tree.custom_states_utf16(element), [] as [Vec<u16>; 0]);
    assert!(!tree.set_custom_state_utf16(element, &large, true));
    assert!(!tree.clear_custom_states(element));
    assert!(tree.storage_stats().payload_bytes < 1000);
}

#[test]
fn state_record_work_is_admitted_before_collection_changes_storage() {
    let mut tree = Tree::parse("<x-state></x-state>");
    let element = tree.query_selector("x-state").unwrap();
    let bounds = CollectionBudget {
        max_edges: 500,
        ..budget()
    };
    tree.collect_unreachable(&[], bounds).unwrap();
    for index in 0..1000 {
        tree.set_custom_state_utf16(
            element,
            &format!("state-{index}").encode_utf16().collect::<Vec<_>>(),
            true,
        );
    }
    let version = tree.version;
    let before = tree.storage_stats();
    assert_eq!(
        tree.collect_unreachable(&[], bounds),
        Err(CollectionError::BudgetExceeded)
    );
    assert_eq!(tree.storage_stats(), before);
    assert_eq!(tree.version, version);
    assert_eq!(tree.custom_states_utf16(element).len(), 1000);
}

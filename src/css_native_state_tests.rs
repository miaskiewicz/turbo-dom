use super::*;
use std::collections::BTreeMap;
fn corpus() -> Vec<Vec<Value>> {
    serde_json::from_str(include_str!("css_native_state_corpus.json")).unwrap()
}
fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn verify_pair(owner: &PreparedHandle, call: &Value, token: &str) -> (usize, usize) {
    let before = owner.matching.as_ref().unwrap().parse_calls();
    crate::css_resolve::take_transport_work();
    let actual = owner
        .resolve_style_state(text(&call["args"][0]), token, text(&call["projection"]))
        .unwrap();
    let misses = owner.matching.as_ref().unwrap().parse_calls() - before;
    assert_eq!(crate::css_resolve::take_transport_work(), (0, 0));
    let actual: Value = serde_json::from_str(&actual).unwrap();
    let cssom: Value = serde_json::from_str(text(&call["cssom"])).unwrap();
    let old: Value = serde_json::from_str(text(&call["result"])).unwrap();
    let id = cssom["targets"][0]["id"].as_u64().unwrap();
    let selected = old["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let pairs: Vec<(String, Value)> =
        serde_json::from_value(cssom["targets"][0]["values"].clone()).unwrap();
    let pairs: BTreeMap<_, _> = pairs.into_iter().collect();
    assert_eq!(actual["targets"][0]["values"], json!(pairs));
    assert_eq!(actual["targets"][0]["custom"], selected["custom"]);
    assert_eq!(
        actual["targets"][0]["usedProperties"],
        cssom["targets"][0]["usedProperties"]
    );
    let slot = owner.matching.as_ref().unwrap();
    let state = slot
        .resolve_state(owner.model().unwrap(), text(&call["args"][0]))
        .unwrap();
    assert_eq!(state.legacy_wire().unwrap(), text(&call["result"]));
    assert_eq!(
        crate::css_serialize::state_cssom_wire(&state, text(&call["projection"])).unwrap(),
        text(&call["cssom"])
    );
    assert!(slot.payload_bytes() <= 256 * 1024);
    (misses, slot.payload_bytes())
}
#[test]
fn authentic_118_pairs_skip_all_full_transport_rows_and_preserve_cssom_wire() {
    let mut count = 0;
    let mut misses = 0;
    let mut maximum = 0;
    for calls in corpus() {
        let mut compiled = BTreeMap::new();
        let mut prepared = BTreeMap::new();
        let mut tokens = BTreeMap::new();
        for call in calls {
            let owner = call["owner"].as_u64().unwrap_or(0);
            match text(&call["method"]) {
                "compile" => {
                    compiled.insert(owner, CompiledHandle::new(text(&call["input"])).unwrap());
                }
                "prepare" => {
                    prepared.insert(
                        call["result"].as_u64().unwrap(),
                        compiled[&owner].prepare(text(&call["input"])).unwrap(),
                    );
                }
                "storeMatchingV1" => {
                    let token = prepared
                        .get_mut(&owner)
                        .unwrap()
                        .store_matching(text(&call["args"][0]), text(&call["args"][1]))
                        .unwrap();
                    tokens.insert((owner, text(&call["result"]).to_owned()), token);
                }
                "resolveMatchingV1" => {
                    let (changed, retained) = verify_pair(
                        &prepared[&owner],
                        &call,
                        &tokens[&(owner, text(&call["args"][1]).to_owned())],
                    );
                    misses += changed;
                    maximum = maximum.max(retained);
                    count += 1;
                }
                "metadata" | "serializeCssomV1" => {}
                other => panic!("unknown {other}"),
            }
        }
        for owner in prepared.values_mut() {
            owner.dispose();
        }
        for owner in compiled.values_mut() {
            owner.dispose();
        }
    }
    assert_eq!(count, 118);
    println!("STATE_COUNTERS calls={count} misses={misses} hits={} max_matching_and_state_payload={maximum} full_rows=0 full_wires=0",count-misses);
}
#[test]
fn state_errors_do_not_switch_modes_or_poison_success() {
    let groups = corpus();
    let calls = &groups[0];
    let find = |name| calls.iter().find(|c| c["method"] == name).unwrap();
    let compiled = CompiledHandle::new(text(&find("compile")["input"])).unwrap();
    let mut owner = compiled.prepare(text(&find("prepare")["input"])).unwrap();
    let mut other = compiled.prepare(text(&find("prepare")["input"])).unwrap();
    let facts = find("storeMatchingV1");
    let args = &facts["args"];
    let token = owner
        .store_matching(text(&args[0]), text(&args[1]))
        .unwrap();
    let foreign = other
        .store_matching(text(&args[0]), text(&args[1]))
        .unwrap();
    let c = find("resolveMatchingV1");
    let wire = text(&c["args"][0]);
    let projection = text(&c["projection"]);
    let old = owner.resolve_matching(wire, &token).unwrap();
    let bytes = owner.retained_bytes();
    assert!(owner
        .resolve_style_state("bad", &token, projection)
        .is_err());
    assert_eq!(owner.retained_bytes(), bytes);
    assert!(owner
        .resolve_style_state("bad", &foreign, "bad")
        .unwrap_err()
        .contains("StaleMatching"));
    let actual = owner.resolve_style_state(wire, &token, projection).unwrap();
    let bytes = owner.retained_bytes();
    assert!(owner.resolve_style_state(wire, &token, "bad").is_err());
    assert_eq!(owner.retained_bytes(), bytes);
    assert!(owner.resolve_matching("bad", &token).is_err());
    assert_eq!(owner.retained_bytes(), bytes);
    assert_eq!(
        owner.resolve_style_state(wire, &token, projection).unwrap(),
        actual
    );
    assert_eq!(owner.resolve_matching(wire, &token).unwrap(), old);
    let next = owner
        .store_matching(text(&args[0]), text(&args[1]))
        .unwrap();
    assert!(owner
        .resolve_style_state(wire, &token, projection)
        .unwrap_err()
        .contains("StaleMatching"));
    assert_eq!(
        owner.resolve_style_state(wire, &next, projection).unwrap(),
        actual
    );
    owner.dispose();
    assert!(owner
        .resolve_style_state("bad", &next, "bad")
        .unwrap_err()
        .contains("Disposed"));
    other.dispose();
}
fn owner_fixture() -> (CompiledHandle, PreparedHandle, String, Value, Value) {
    let calls = corpus().remove(0);
    let find = |name| calls.iter().find(|c| c["method"] == name).unwrap();
    let compiled = CompiledHandle::new(text(&find("compile")["input"])).unwrap();
    let mut owner = compiled.prepare(text(&find("prepare")["input"])).unwrap();
    let args = &find("storeMatchingV1")["args"];
    let token = owner
        .store_matching(text(&args[0]), text(&args[1]))
        .unwrap();
    (
        compiled,
        owner,
        token,
        find("resolveMatchingV1").clone(),
        find("serializeCssomV1").clone(),
    )
}
#[test]
fn typed_projection_limits_match_original_serializer_errors() {
    let (_compiled, owner, token, call, serialized) = owner_fixture();
    let wire = text(&call["args"][0]);
    for field in [
        "targets",
        "inputUnits",
        "retainedUnits",
        "maxNesting",
        "maxOutputUnits",
        "maxTotalUnits",
    ] {
        for limit in [0, 1, 8, 32, 4096] {
            let mut projection: Value = serde_json::from_str(text(&call["projection"])).unwrap();
            let mut original: Value = serde_json::from_str(text(&serialized["args"][0])).unwrap();
            let path = if field.starts_with("max") {
                "/limits/computation"
            } else {
                "/limits"
            };
            projection.pointer_mut(path).unwrap()[field] = json!(limit);
            original.pointer_mut(path).unwrap()[field] = json!(limit);
            let expected = crate::css_serialize::serialize_batch(&original.to_string());
            let actual = owner.resolve_style_state(wire, &token, &projection.to_string());
            assert_eq!(
                actual.as_ref().err(),
                expected.as_ref().err(),
                "{field}={limit}"
            );
        }
    }
}
#[test]
fn typed_cache_oversize_success_clears_retention_and_threads_are_independent() {
    let (_compiled, owner, token, call, _) = owner_fixture();
    let wire = text(&call["args"][0]);
    let projection = text(&call["projection"]);
    let initial = owner.matching.as_ref().unwrap().payload_bytes();
    let expected = owner.resolve_style_state(wire, &token, projection).unwrap();
    assert!(owner.matching.as_ref().unwrap().payload_bytes() > initial);
    std::thread::scope(|scope| {
        let mut readers = Vec::new();
        for _ in 0..4 {
            readers
                .push(scope.spawn(|| owner.resolve_style_state(wire, &token, projection).unwrap()));
        }
        for reader in readers {
            assert_eq!(reader.join().unwrap(), expected);
        }
    });
    let oversized = format!("{}{wire}", " ".repeat(64 * 1024));
    assert_eq!(
        owner
            .resolve_style_state(&oversized, &token, projection)
            .unwrap(),
        expected
    );
    assert_eq!(owner.matching.as_ref().unwrap().payload_bytes(), initial);
    assert_eq!(
        owner.resolve_style_state(wire, &token, projection).unwrap(),
        expected
    );
}
#[test]
fn typed_cache_eight_entries_keep_current_lru_and_payload_bounds() {
    let (_compiled, owner, token, call, _) = owner_fixture();
    let wire = text(&call["args"][0]);
    let projection = text(&call["projection"]);
    let expected = owner.resolve_style_state(wire, &token, projection).unwrap();
    let slot = owner.matching.as_ref().unwrap();
    let before = slot.parse_calls();
    assert_eq!(
        owner.resolve_style_state(wire, &token, projection).unwrap(),
        expected
    );
    assert_eq!(slot.parse_calls(), before);
    for index in 1..9 {
        let changed = format!("{}{wire}", " ".repeat(index));
        assert_eq!(
            owner
                .resolve_style_state(&changed, &token, projection)
                .unwrap(),
            expected
        );
        assert!(slot.payload_bytes() <= 256 * 1024);
    }
    let before = slot.parse_calls();
    assert_eq!(
        owner.resolve_style_state(wire, &token, projection).unwrap(),
        expected
    );
    assert_eq!(slot.parse_calls(), before + 1);
}

// Synthetic post-cascade inputs exercise the native/JSON numeric boundary.
fn numeric_state_fixture() -> (crate::css_resolve::native_state::NativeStyleState, Value) {
    let (_compiled, mut owner, _token, call, _) = owner_fixture();
    let state = owner
        .matching
        .as_ref()
        .unwrap()
        .resolve_state(owner.model().unwrap(), text(&call["args"][0]))
        .unwrap();
    owner.dispose();
    let state = std::sync::Arc::try_unwrap(state)
        .ok()
        .expect("disposed owner releases cached state");
    (
        state,
        serde_json::from_str(text(&call["projection"])).unwrap(),
    )
}
fn set_numeric_style(
    state: &mut crate::css_resolve::native_state::NativeStyleState,
    id: u32,
    font: f64,
    origin: turbo_dom_core::rtdom::computed_css::prepared::CascadeOrigin,
) {
    use turbo_dom_core::rtdom::computed_css::{resolve::DeclaredTextDecoration, CssText};
    let style = state.styles.get_mut(&id).unwrap();
    style.values.font_size_px = font;
    style.values.root_font_size_px = font;
    style.values.values = BTreeMap::from([
        (
            "line-height".into(),
            CssText::from_units("1.5".encode_utf16().collect()),
        ),
        (
            "color".into(),
            CssText::from_units("red".encode_utf16().collect()),
        ),
    ]);
    style.public_keys = ["line-height".into(), "color".into()].into();
    style.declared_text_decoration = Some(DeclaredTextDecoration {
        value: CssText::from_units("none".encode_utf16().collect()),
        origin,
        important: true,
        inline: false,
        order: (2, 3),
    });
    state.extras.get_mut(&id).unwrap().clear();
    assert!(state.payload_bytes() >= 4 * std::mem::size_of::<u16>());
}
fn original_state_projection(
    state: &crate::css_resolve::native_state::NativeStyleState,
    projection: &Value,
) -> Value {
    // Materialize the old response first, then reconstruct the old JS DTO.
    let legacy: Value = serde_json::from_str(&state.legacy_wire().unwrap()).unwrap();
    let row = legacy["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == projection["targetId"])
        .unwrap();
    let pairs: Vec<_> = row["rendererValues"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| json!([name, value]))
        .collect();
    let marker = row["declaredTextDecoration"]
        .as_object()
        .map(|m| json!({"value":m["value"],"origin":m["origin"]}));
    let mut font = row["fontSizePx"].clone();
    // JSON.stringify(-0) is 0 on the original JS route; nonfinite is already null.
    if font.as_f64() == Some(0.0) {
        font = json!(0);
    }
    json!({"abiVersion":1,"targets":[{"id":row["id"],"values":pairs,
        "publicKeys":row["publicKeys"],"fontSizePx":font,"declaredTextDecoration":marker}],
        "textDecoration":projection["textDecoration"],"includeUsedProperties":true,
        "limits":projection["limits"]})
}
fn assert_state_projection_matches_original(
    state: &crate::css_resolve::native_state::NativeStyleState,
    projection: &Value,
) {
    let original = original_state_projection(state, projection);
    assert_eq!(
        crate::css_serialize::state_cssom_wire(state, &projection.to_string()),
        crate::css_serialize::serialize_batch(&original.to_string())
    );
}
#[test]
fn typed_state_font_dependent_projection_preserves_json_numeric_boundaries() {
    use turbo_dom_core::rtdom::computed_css::prepared::CascadeOrigin;
    let (mut state, projection) = numeric_state_fixture();
    let id = projection["targetId"].as_u64().unwrap() as u32;
    for font in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        0.0,
        13.123_456_789_012_345,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::from(f32::MAX) * 2.0,
    ] {
        for origin in [
            CascadeOrigin::Author,
            CascadeOrigin::User,
            CascadeOrigin::UserAgent,
        ] {
            set_numeric_style(&mut state, id, font, origin);
            assert_state_projection_matches_original(&state, &projection);
        }
    }
    set_numeric_style(&mut state, id, 20.0, CascadeOrigin::Author);
    let projected: Value = serde_json::from_str(
        &crate::css_serialize::state_cssom_wire(&state, &projection.to_string()).unwrap(),
    )
    .unwrap();
    assert!(projected["targets"][0]["values"]
        .as_array()
        .unwrap()
        .contains(&json!([
            "line-height",
            "30px".encode_utf16().collect::<Vec<_>>()
        ])));
}
#[test]
fn typed_state_used_property_fallback_keeps_ancestor_rows_and_old_snapshot() {
    use turbo_dom_core::rtdom::computed_css::{prepared::CascadeOrigin, CssText};
    let (mut state, projection) = numeric_state_fixture();
    let id = projection["targetId"].as_u64().unwrap() as u32;
    set_numeric_style(&mut state, id, 20.0, CascadeOrigin::Author);
    let style = state.styles.get_mut(&id).unwrap();
    style.values.values.insert(
        "width".into(),
        CssText::from_units("50%".encode_utf16().collect()),
    );
    style.public_keys.insert("width".into());
    let before: Value = serde_json::from_str(&state.legacy_wire().unwrap()).unwrap();
    let snapshot = crate::css_serialize::project_state(&state, &projection.to_string()).unwrap();
    let projected: Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(
        projected["targets"].as_array().unwrap().len(),
        state.order.len()
    );
    for row in projected["targets"].as_array().unwrap() {
        let old = before["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == row["id"])
            .unwrap();
        if row["id"] != id {
            assert_eq!(row, old);
            continue;
        }
        assert!(row["usedProperties"]
            .as_array()
            .unwrap()
            .contains(&json!("width")));
        assert_eq!(row["rendererValues"], old["rendererValues"]);
    }
    state.styles.get_mut(&id).unwrap().values.values.insert(
        "width".into(),
        CssText::from_units("25%".encode_utf16().collect()),
    );
    let later = crate::css_serialize::project_state(&state, &projection.to_string()).unwrap();
    assert_ne!(snapshot, later);
    assert_eq!(serde_json::from_str::<Value>(&snapshot).unwrap(), projected);
}
#[test]
fn typed_state_competing_marker_input_and_output_quotas_keep_old_errors() {
    use turbo_dom_core::rtdom::computed_css::prepared::CascadeOrigin;
    let (mut state, mut projection) = numeric_state_fixture();
    let id = projection["targetId"].as_u64().unwrap() as u32;
    set_numeric_style(&mut state, id, 20.0, CascadeOrigin::UserAgent);
    // Crossing each input boundary exercises marker/origin, ordinary names/values,
    // then public provenance before the simultaneously restricted result budget.
    for input in 0..96 {
        for output in [0, 1, 8, 32, 4096] {
            projection["limits"]["inputUnits"] = json!(input);
            projection["limits"]["retainedUnits"] = json!(output);
            assert_state_projection_matches_original(&state, &projection);
        }
    }
}

#[test]
fn typed_state_projection_protocol_rejects_incompatible_metadata_in_order() {
    let (mut state, mut projection) = numeric_state_fixture();
    state.geometry_only = true;
    assert!(crate::css_serialize::project_state(&state, "bad")
        .unwrap_err()
        .contains("StyleStateGeometryOnly"));
    state.geometry_only = false;
    projection["abiVersion"] = json!(2);
    projection["includeUsedProperties"] = json!(false);
    assert!(
        crate::css_serialize::project_state(&state, &projection.to_string())
            .unwrap_err()
            .contains("AbiVersion")
    );
    projection["abiVersion"] = json!(1);
    assert!(
        crate::css_serialize::project_state(&state, &projection.to_string())
            .unwrap_err()
            .contains("StyleStateUsedMetadata")
    );
    projection["includeUsedProperties"] = json!(true);
    projection["targetId"] = json!(u32::MAX);
    assert!(
        crate::css_serialize::project_state(&state, &projection.to_string())
            .unwrap_err()
            .contains("StyleStateTargetMissing")
    );
}

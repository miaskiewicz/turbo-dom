use super::tests::{retained_fixture, retained_input};
use super::*;
fn install(owner: &PreparedHandle, matching: &str, input: &Value) -> (String, String) {
    let result: Value = serde_json::from_str(
        &owner
            .resolve_request_template(&retained_input(input), matching)
            .unwrap(),
    )
    .unwrap();
    (
        result["response"].as_str().unwrap().to_owned(),
        result["templateToken"].as_str().unwrap().to_owned(),
    )
}
fn compact(input: &Value) -> String {
    let mut next: Value = serde_json::from_str(&retained_input(input)).unwrap();
    for (index, target) in next["targets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        target["context"] = json!(index);
    }
    next.to_string()
}
#[test]
fn template_contexts_preserve_exact_full_results_and_current_budgets() {
    let (owner, matching, input) = retained_fixture();
    let expected = owner
        .resolve_matching(&retained_input(&input), &matching)
        .unwrap();
    let (first, token) = install(&owner, &matching, &input);
    assert_eq!(first, expected);
    assert_eq!(
        owner
            .resolve_template(&compact(&input), &matching, &token)
            .unwrap(),
        expected
    );
    for name in [
        "targets",
        "answerRows",
        "answerIds",
        "pathNodes",
        "inputUnits",
        "retainedResultUnits",
        "matchingDecisions",
        "routingContexts",
        "inlineDeclarations",
        "inlineProperties",
        "customRollbacks",
        "valueRetainedUnits",
    ] {
        let mut changed = input.clone();
        changed["limits"][name] = json!(0);
        assert_eq!(
            owner.resolve_template(&compact(&changed), &matching, &token),
            owner.resolve_matching(&retained_input(&changed), &matching),
            "{name}"
        );
    }
}
#[test]
fn template_tokens_preserve_owner_replacement_disposal_and_error_order() {
    let (mut owner, matching, input) = retained_fixture();
    let (expected, token) = install(&owner, &matching, &input);
    let (other, other_matching, _) = retained_fixture();
    assert!(other
        .resolve_template(&compact(&input), &matching, &token)
        .unwrap_err()
        .contains("StaleMatching"));
    assert!(owner
        .resolve_template("bad", &other_matching, &token)
        .unwrap_err()
        .contains("StaleMatching"));
    assert!(owner
        .resolve_template("bad", &matching, "wrong")
        .unwrap_err()
        .contains("StaleRequestTemplate"));
    assert!(owner.resolve_request_template("bad", &matching).is_err());
    assert_eq!(
        owner
            .resolve_template(&compact(&input), &matching, &token)
            .unwrap(),
        expected
    );
    let (_, replacement) = install(&owner, &matching, &input);
    assert_ne!(token, replacement);
    assert!(owner
        .resolve_template(&compact(&input), &matching, &token)
        .is_err());
    owner.dispose();
    assert_eq!(owner.retained_bytes(), 0.0);
    assert!(owner
        .resolve_template("bad", &matching, &replacement)
        .is_err());
}
#[test]
fn template_failures_and_cache_namespaces_are_independent() {
    let (owner, matching, input) = retained_fixture();
    let (expected, token) = install(&owner, &matching, &input);
    let wire = compact(&input);
    assert_eq!(
        owner.resolve_template(&wire, &matching, &token).unwrap(),
        expected
    );
    // The compact wire is invalid under the original full protocol, even cached.
    assert!(owner.resolve_matching(&wire, &matching).is_err());
    let mut bad: Value = serde_json::from_str(&wire).unwrap();
    bad["targets"][0]["context"] = json!(16);
    assert!(owner
        .resolve_template(&bad.to_string(), &matching, &token)
        .unwrap_err()
        .contains("ContextReference"));
    bad["targets"][0]["context"] = json!(0);
    bad["abiVersion"] = json!(2);
    assert!(owner
        .resolve_template(&bad.to_string(), &matching, &token)
        .unwrap_err()
        .contains("AbiVersion"));
    assert_eq!(
        owner.resolve_template(&wire, &matching, &token).unwrap(),
        expected
    );
}
#[test]
fn template_and_success_payloads_share_the_existing_aggregate_cap() {
    let (owner, matching, input) = retained_fixture();
    let original = owner.retained_bytes();
    let (_, token) = install(&owner, &matching, &input);
    let compact = compact(&input);
    for index in 0..12 {
        owner
            .resolve_template(
                &format!("{compact}{}", " ".repeat(index * 3000)),
                &matching,
                &token,
            )
            .unwrap();
        assert!(owner.retained_bytes() <= original + f64::from(64 * 1024));
    }
    owner
        .resolve_template(
            &format!("{compact}{}", " ".repeat(64 * 1024)),
            &matching,
            &token,
        )
        .unwrap();
    assert!(owner.retained_bytes() < original + 1000.0);
}
#[test]
fn template_threaded_readers_preserve_owned_contexts() {
    let (owner, matching, input) = retained_fixture();
    let (expected, token) = install(&owner, &matching, &input);
    let compact = compact(&input);
    std::thread::scope(|scope| {
        let mut readers = Vec::new();
        for _ in 0..4 {
            readers
                .push(scope.spawn(|| owner.resolve_template(&compact, &matching, &token).unwrap()));
        }
        for reader in readers {
            assert_eq!(reader.join().unwrap(), expected);
        }
    });
}
#[test]
fn template_installation_preserves_the_original_full_success_cache_hit() {
    let (owner, matching, input) = retained_fixture();
    let (expected, token) = install(&owner, &matching, &input);
    let slot = owner.matching.as_ref().unwrap();
    assert_eq!(slot.cascade_calls(), 1);
    assert_eq!(
        owner
            .resolve_matching(&retained_input(&input), &matching)
            .unwrap(),
        expected
    );
    assert_eq!(slot.cascade_calls(), 1);
    assert_eq!(slot.parse_calls(), 1);
    let mut changed = input.clone();
    changed["targets"][0]["inline"] = json!("color:blue".encode_utf16().collect::<Vec<_>>());
    assert_ne!(
        owner
            .resolve_template(&compact(&changed), &matching, &token)
            .unwrap(),
        expected
    );
    assert_eq!(
        owner
            .resolve_matching(&retained_input(&input), &matching)
            .unwrap(),
        expected
    );
    assert_eq!(slot.parse_calls(), 1);
}
fn generous_limits(input: &mut Value) {
    for value in input["limits"].as_object_mut().unwrap().values_mut() {
        if value.is_number() {
            *value = json!(1_000_000);
        } else {
            for unit in value.as_object_mut().unwrap().values_mut() {
                *unit = json!(1_000_000);
            }
        }
    }
}
#[test]
fn successful_large_target_batch_declines_only_template_retention() {
    let (mut owner, _, mut input) = retained_fixture();
    let example = input["targets"][0].clone();
    input["targets"] = Value::Array(
        (2..19)
            .map(|id| {
                let mut row = example.clone();
                row["id"] = json!(id);
                row["subjectId"] = json!(id);
                row["path"] = json!([id, 1]);
                row
            })
            .collect(),
    );
    input["domains"][0]["nodeIds"] = json!((1..19).collect::<Vec<_>>());
    generous_limits(&mut input);
    let facts = json!({"domains":input["domains"],"selectorAnswers":input["selectorAnswers"],"scopeAnswers":[],"contextRoots":input["contextRoots"]});
    let token = owner
        .store_matching(&facts.to_string(), &input["limits"].to_string())
        .unwrap();
    let wire = retained_input(&input);
    let fresh: Value =
        serde_json::from_str(&owner.resolve_request_template(&wire, &token).unwrap()).unwrap();
    assert!(fresh["templateToken"].is_null());
    let expected = owner.resolve_matching(&wire, &token).unwrap();
    assert_eq!(fresh["response"], expected);
    let receipt: Value =
        serde_json::from_str(&owner.resolve_request_template(&wire, &token).unwrap()).unwrap();
    assert_eq!(receipt["response"], expected);
    assert!(receipt["templateToken"].is_null());
}
#[test]
fn successful_oversized_context_declines_retention_without_changing_response() {
    let (owner, token, mut input) = retained_fixture();
    input["targets"][0]["context"]["initialFontFamily"] = json!("x".repeat(70000));
    generous_limits(&mut input);
    let before = owner.retained_bytes();
    let wire = retained_input(&input);
    let expected = owner.resolve_matching(&wire, &token).unwrap();
    let receipt: Value =
        serde_json::from_str(&owner.resolve_request_template(&wire, &token).unwrap()).unwrap();
    assert_eq!(receipt["response"], expected);
    assert!(receipt["templateToken"].is_null());
    assert_eq!(owner.retained_bytes(), before);
}
#[test]
fn installing_context_payload_evicts_full_successes_before_exceeding_shared_cap() {
    let (preview, preview_token, mut larger) = retained_fixture();
    larger["targets"][0]["context"]["initialFontFamily"] = json!("x".repeat(500));
    generous_limits(&mut larger);
    let large_wire = retained_input(&larger);
    let large_response = preview
        .resolve_matching(&large_wire, &preview_token)
        .unwrap();
    let (owner, token, input) = retained_fixture();
    let base_wire = retained_input(&input);
    let base_response = preview
        .resolve_matching(&base_wire, &preview_token)
        .unwrap();
    let fill_bytes = 64 * 1024 - large_wire.len() - large_response.len() - 128;
    let padding = fill_bytes - base_wire.len() - base_response.len();
    let fill_wire = format!("{base_wire}{}", " ".repeat(padding));
    owner.resolve_matching(&fill_wire, &token).unwrap();
    let (response, _) = install(&owner, &token, &larger);
    assert_eq!(response, large_response);
    let slot = owner.matching.as_ref().unwrap();
    assert_eq!(slot.parse_calls(), 2);
    owner.resolve_matching(&large_wire, &token).unwrap();
    assert_eq!(slot.parse_calls(), 2);
    owner.resolve_matching(&fill_wire, &token).unwrap();
    assert_eq!(slot.parse_calls(), 3);
}

#[test]
fn fresh_template_installation_parses_once_and_cached_installation_parses_once() {
    let (owner, matching, input) = retained_fixture();
    let slot = owner.matching.as_ref().unwrap();
    let (response, _) = install(&owner, &matching, &input);
    assert_eq!(slot.parse_calls(), 1);
    assert_eq!(slot.cascade_calls(), 1);
    let (again, _) = install(&owner, &matching, &input);
    assert_eq!(response, again);
    assert_eq!(slot.parse_calls(), 2);
    assert_eq!(slot.cascade_calls(), 1);
}
#[test]
fn later_target_failure_drops_partial_context_receipt_without_publication() {
    let (owner, matching, input) = retained_fixture();
    let (response, token) = install(&owner, &matching, &input);
    let before = owner.retained_bytes();
    let mut invalid = input.clone();
    let mut later = invalid["targets"][0].clone();
    later["context"]["initialColor"] = json!("new-context-must-not-be-published");
    invalid["targets"].as_array_mut().unwrap().push(later);
    let wire = retained_input(&invalid);
    let original = owner.resolve_matching(&wire, &matching).unwrap_err();
    assert!(original.contains("DuplicateTarget"));
    assert_eq!(
        owner.resolve_request_template(&wire, &matching),
        Err(original)
    );
    assert_eq!(owner.retained_bytes(), before);
    assert_eq!(
        owner
            .resolve_template(&compact(&input), &matching, &token)
            .unwrap(),
        response
    );
}
#[test]
fn direct_state_publication_retires_template_but_failed_resolve_preserves_it() {
    let (owner, matching, input) = retained_fixture();
    let wire = retained_input(&input);
    let (expected, token) = install(&owner, &matching, &input);
    let before = owner.retained_bytes();
    assert!(owner.resolve_style_state("bad", &matching, "bad").is_err());
    assert_eq!(owner.retained_bytes(), before);
    assert_eq!(
        owner
            .resolve_template(&compact(&input), &matching, &token)
            .unwrap(),
        expected
    );
    // Resolve success publishes state before a later projection error.
    assert!(owner.resolve_style_state(&wire, &matching, "bad").is_err());
    assert!(owner
        .resolve_template(&compact(&input), &matching, &token)
        .unwrap_err()
        .contains("StaleRequestTemplate"));
    let slot = owner.matching.as_ref().unwrap();
    let state = slot.resolve_state(owner.model().unwrap(), &wire).unwrap();
    assert_eq!(state.legacy_wire().unwrap(), expected);
    let (again, fresh) = install(&owner, &matching, &input);
    assert_eq!(again, expected);
    assert_ne!(fresh, token);
    assert_eq!(
        owner
            .resolve_template(&compact(&input), &matching, &fresh)
            .unwrap(),
        expected
    );
    assert!(slot.payload_bytes() <= 256 * 1024);
}
#[test]
fn concurrent_state_and_template_calls_preserve_exact_results_and_single_cap() {
    let (owner, matching, input) = retained_fixture();
    let wire = retained_input(&input);
    let (expected, token) = install(&owner, &matching, &input);
    let compact = compact(&input);
    std::thread::scope(|scope| {
        let state = scope.spawn(|| {
            for _ in 0..8 {
                let slot = owner.matching.as_ref().unwrap();
                assert_eq!(
                    slot.resolve_state(owner.model().unwrap(), &wire)
                        .unwrap()
                        .legacy_wire()
                        .unwrap(),
                    expected
                );
                assert!(slot.payload_bytes() <= 256 * 1024);
            }
        });
        let template = scope.spawn(|| {
            for _ in 0..8 {
                match owner.resolve_template(&compact, &matching, &token) {
                    Ok(result) => assert_eq!(result, expected),
                    Err(error) => assert!(error.contains("StaleRequestTemplate")),
                }
            }
        });
        state.join().unwrap();
        template.join().unwrap();
    });
    let (fresh, _) = install(&owner, &matching, &input);
    assert_eq!(fresh, expected);
}

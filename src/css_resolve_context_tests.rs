use super::*;
use serde_json::{json, Value};
use turbo_dom_core::rtdom::computed_css::{
    conditions::ConditionEnvironment,
    model::{CompileLimits, CompiledStyles, SheetSource},
    prepared::{CascadeOrigin, PreparationEnvironment, PreparationLimits, SheetUse, StylePhase},
    ComputationLimits,
};
fn computation() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 8192,
    }
}
fn prepared(source: &str) -> PreparedStyles {
    prepared_disabled(source, false)
}
fn prepared_disabled(source: &str, disabled: bool) -> PreparedStyles {
    let compiled = CompiledStyles::compile(
        &[SheetSource {
            id: 7,
            text: CssText::from_units(source.encode_utf16().collect()),
        }],
        CompileLimits {
            sources: 1,
            source_units: source.len(),
            rules: 16,
            source_rule_preludes: 16,
            selectors: 16,
            declarations: 32,
            properties: 128,
            group_depth: 8,
            token_nesting: 32,
        },
    )
    .unwrap();
    let features = BTreeMap::new();
    let support = BTreeMap::new();
    let lengths = LengthContext {
        unit_bases: &[("rem", 16.0)],
    };
    let conditions = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &support,
    };
    let uses: Vec<_> = [1, 2]
        .into_iter()
        .map(|scope| SheetUse {
            source_id: 7,
            origin: CascadeOrigin::Author,
            tree_scope: scope,
            scope_depth: scope - 1,
            disabled,
            media: None,
            implicit_scope_root: None,
        })
        .collect();
    compiled
        .prepare(
            &uses,
            &PreparationEnvironment {
                conditions: &conditions,
                style_phase: Some(StylePhase::Steady),
            },
            PreparationLimits {
                uses: 2,
                rules: 32,
                layers: 16,
                computation: computation(),
            },
        )
        .unwrap()
}
fn request() -> Value {
    json!({"abiVersion":1,"targets":[{"id":11,"parentId":null,"path":[11,10],
        "implicitScopeRoot":10,"treeScope":1,"scopeDepth":0,"pseudo":null,"inline":null,
        "hostOf":2,"context":{"initialColor":"rgb(0, 0, 0)","initialFontFamily":"serif",
        "initialFontSize":"16px","unitBases":[["rem",16.0]],"fontSizeUnitBases":[],"isRoot":false}}],
        "contextRoots":[{"treeScope":1,"root":10,"host":null,"parentTreeScope":null},
            {"treeScope":2,"root":20,"host":11,"parentTreeScope":1}],
        "domains":[{"id":0,"nodeIds":[11]}],"selectorAnswers":[],"scopeAnswers":[],
        "contextAnswers":{"selectorAnswers":[
            {"treeScope":1,"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[11]},
            {"treeScope":1,"selectorId":1,"itemId":0,"rootId":null,"domainId":0,"matches":[]},
            {"treeScope":2,"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[]},
            {"treeScope":2,"selectorId":1,"itemId":0,"rootId":null,"domainId":0,"matches":[]}],"scopeAnswers":[]},
        "limits":{"targets":2,"answerRows":16,"answerIds":32,"pathNodes":16,"routingContexts":2,
            "inputUnits":1024,"retainedResultUnits":8192,"matchingDecisions":128,
            "inlineDeclarations":16,"inlineProperties":128,"customRollbacks":16,"valueRetainedUnits":4096,
            "computation":{"maxNesting":32,"maxOutputUnits":4096,"maxTotalUnits":8192}}})
}
fn model() -> PreparedStyles {
    prepared("#host{color:red}:host(.active){color:blue!important}")
}
fn resolve(prepared: &PreparedStyles, request: &Value) -> Value {
    serde_json::from_str(&resolve_batch(prepared, &request.to_string()).unwrap()).unwrap()
}
#[test]
fn contextual_selector_rows_preserve_shared_source_context_and_old_output_shape() {
    let prepared = model();
    let request = request();
    let output = resolve(&prepared, &request);
    assert_eq!(output["abiVersion"], 1);
    assert_eq!(output["geometryOnly"], false);
    assert_eq!(
        output["targets"][0]["values"]["color"],
        json!("rgb(255, 0, 0)".encode_utf16().collect::<Vec<_>>())
    );
    let mut other = request;
    other["contextAnswers"]["selectorAnswers"][3]["matches"] = json!([11]);
    assert_eq!(
        resolve(&prepared, &other)["targets"][0]["values"]["color"],
        json!("rgb(0, 0, 255)".encode_utf16().collect::<Vec<_>>())
    );
}
#[test]
fn absent_and_null_context_field_keep_legacy_payloads_byte_equivalent() {
    let prepared = model();
    let mut legacy = request();
    legacy.as_object_mut().unwrap().remove("contextAnswers");
    legacy["selectorAnswers"] = json!([
        {"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[11]},
        {"selectorId":1,"itemId":0,"rootId":null,"domainId":0,"matches":[]}]);
    let expected = resolve_batch(&prepared, &legacy.to_string()).unwrap();
    legacy["contextAnswers"] = Value::Null;
    assert_eq!(
        resolve_batch(&prepared, &legacy.to_string()).unwrap(),
        expected
    );
}
#[test]
fn mixed_context_missing_answers_and_partial_shapes_never_fall_back() {
    let prepared = model();
    let mut input = request();
    input["selectorAnswers"] =
        json!([{"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[11]}]);
    assert_eq!(
        resolve_batch(&prepared, &input.to_string()).unwrap_err(),
        fail("MixedAnswerModes")
    );
    let mut input = request();
    input["contextAnswers"] = json!({"selectorAnswers":[],"scopeAnswers":[]});
    assert!(resolve_batch(&prepared, &input.to_string())
        .unwrap_err()
        .contains("MissingSelectorAnswer"));
    for field in ["selectorAnswers", "scopeAnswers"] {
        let mut input = request();
        input["contextAnswers"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(resolve_batch(&prepared, &input.to_string())
            .unwrap_err()
            .starts_with("CSSBackendTransport:"));
    }
    let mut input = request();
    input["contextAnswers"]["unexpected"] = json!(1);
    assert!(resolve_batch(&prepared, &input.to_string())
        .unwrap_err()
        .starts_with("CSSBackendTransport:"));
}
#[test]
fn contextual_duplicates_wrong_owner_and_missing_domains_stay_explicit() {
    let prepared = model();
    let mut input = request();
    let row = input["contextAnswers"]["selectorAnswers"][0].clone();
    input["contextAnswers"]["selectorAnswers"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert_eq!(
        resolve_batch(&prepared, &input.to_string()).unwrap_err(),
        fail("DuplicateContextSelectorAnswer")
    );
    let mut input = request();
    input["contextAnswers"]["selectorAnswers"][0]["treeScope"] = json!(99);
    assert_eq!(
        resolve_batch(&prepared, &input.to_string()).unwrap_err(),
        fail("UnknownAnswerContext")
    );
    let mut input = request();
    input["contextAnswers"]["selectorAnswers"][0]["domainId"] = json!(9);
    assert_eq!(
        resolve_batch(&prepared, &input.to_string()).unwrap_err(),
        fail("MissingDomain")
    );
    let mut input = request();
    input["contextAnswers"]["selectorAnswers"][0]["matches"] = json!([99]);
    assert!(resolve_batch(&prepared, &input.to_string())
        .unwrap_err()
        .contains("OutsideDomain"));
}
#[test]
fn contextual_rows_and_match_ids_share_existing_exact_cumulative_quotas() {
    let prepared = model();
    let mut input = request();
    input["limits"]["answerRows"] = json!(5);
    input["limits"]["answerIds"] = json!(2);
    resolve(&prepared, &input);
    for (field, value) in [("answerRows", 4), ("answerIds", 1)] {
        let mut limited = input.clone();
        limited["limits"][field] = json!(value);
        assert_eq!(
            resolve_batch(&prepared, &limited.to_string()).unwrap_err(),
            fail("ResourceLimit")
        );
    }
}
#[test]
fn contextual_scope_boundaries_are_keyed_without_a_virtual_scope_host_dependency() {
    let prepared = prepared("@scope(.root)to(.limit){.x{color:red}}");
    let mut input = request();
    input["domains"][0]["nodeIds"] = json!([11, 10]);
    input["contextAnswers"] = json!({"selectorAnswers":[
        {"treeScope":1,"selectorId":0,"itemId":0,"rootId":11,"domainId":0,"matches":[11]},
        {"treeScope":2,"selectorId":0,"itemId":0,"rootId":11,"domainId":0,"matches":[11]}],"scopeAnswers":[
        {"treeScope":1,"groupId":0,"boundary":"start","rootId":10,"domainId":0,"matches":[11]},
        {"treeScope":2,"groupId":0,"boundary":"start","rootId":20,"domainId":0,"matches":[11]},
        {"treeScope":1,"groupId":0,"boundary":"end","rootId":11,"domainId":0,"matches":[]},
        {"treeScope":2,"groupId":0,"boundary":"end","rootId":11,"domainId":0,"matches":[11]}]});
    assert!(resolve(&prepared, &input)["targets"][0]["values"]
        .get("color")
        .is_some());
    input["contextAnswers"]["scopeAnswers"][0]["matches"] = json!([]);
    assert!(resolve(&prepared, &input)["targets"][0]["values"]
        .get("color")
        .is_none());
    input["contextAnswers"]["scopeAnswers"][0]["matches"] = json!([11]);
    input["contextAnswers"]["scopeAnswers"][2]["matches"] = json!([11]);
    assert!(resolve(&prepared, &input)["targets"][0]["values"]
        .get("color")
        .is_none());
    input["contextAnswers"]["scopeAnswers"][1]["treeScope"] = json!(1);
    input["contextAnswers"]["scopeAnswers"][1]["rootId"] = json!(10);
    assert_eq!(
        resolve_batch(&prepared, &input.to_string()).unwrap_err(),
        fail("DuplicateContextScopeAnswer")
    );
}

#[test]
fn inactive_and_disabled_uses_accept_known_context_answers_but_still_charge_unused_rows() {
    let disabled = prepared_disabled("#host{color:red}:host(.active){color:blue!important}", true);
    let inactive = prepared("@media print{#host{color:red}:host(.active){color:blue!important}}");
    for prepared in [disabled, inactive] {
        assert!(prepared.metadata().rules.is_empty());
        let mut input = request();
        input["limits"]["answerRows"] = json!(5);
        input["limits"]["answerIds"] = json!(2);
        assert!(resolve(&prepared, &input)["targets"][0]["values"]
            .get("color")
            .is_none());
        input["limits"]["answerRows"] = json!(4);
        assert_eq!(
            resolve_batch(&prepared, &input.to_string()).unwrap_err(),
            fail("ResourceLimit")
        );
    }
}

#[test]
fn native_origin_names_and_legacy_duplicate_fact_errors_are_exact() {
    assert_eq!(origin_name(CascadeOrigin::UserAgent), "user-agent");
    assert_eq!(origin_name(CascadeOrigin::User), "user");
    let mut duplicate = request();
    duplicate["domains"] = json!([{"id":0,"nodeIds":[11]},{"id":0,"nodeIds":[11]}]);
    assert_eq!(
        resolve_batch(&model(), &duplicate.to_string()),
        Err(fail("DuplicateDomain"))
    );
    duplicate = request();
    duplicate.as_object_mut().unwrap().remove("contextAnswers");
    duplicate["selectorAnswers"] = json!([
        {"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[11]},
        {"selectorId":0,"itemId":0,"rootId":null,"domainId":0,"matches":[11]}]);
    assert_eq!(
        resolve_batch(&model(), &duplicate.to_string()),
        Err(fail("DuplicateSelectorAnswer"))
    );
}
#[test]
fn legacy_scope_boundaries_charge_ids_and_reject_duplicate_boundary_keys() {
    let domain = domains(vec![DomainRow {
        id: 0,
        node_ids: vec![11],
    }])
    .unwrap();
    let rows = json!([
        {"groupId":0,"boundary":"start","rootId":10,"domainId":0,"matches":[11]},
        {"groupId":0,"boundary":"end","rootId":10,"domainId":0,"matches":[]}]);
    let answers = boundary_answers(serde_json::from_value(rows.clone()).unwrap(), &domain).unwrap();
    assert_eq!(answers.len(), 2);
    let mut facts: Matching = serde_json::from_value(
        json!({"domains":[{"id":0,"nodeIds":[11]}],"selectorAnswers":[],"scopeAnswers":rows}),
    )
    .unwrap();
    let mut usage = Usage::default();
    usage
        .common(
            &facts,
            serde_json::from_value(request()["limits"].clone()).unwrap(),
        )
        .unwrap();
    assert_eq!(usage.ids, 2);
    facts
        .scope_answers
        .push(serde_json::from_value(rows[0].clone()).unwrap());
    assert!(
        matches!(boundary_answers(facts.scope_answers, &domain), Err(error) if error == fail("DuplicateScopeAnswer"))
    );
}
#[test]
fn native_route_pseudo_and_missing_context_errors_keep_their_order() {
    let mut input = request();
    input["contextRoots"] = json!([]);
    input.as_object_mut().unwrap().remove("contextAnswers");
    assert_eq!(
        resolve_batch(&model(), &input.to_string()),
        Err(fail("MissingRouteContext"))
    );
    input = request();
    input["targets"][0]["pseudo"] = json!("unsupported");
    assert_eq!(
        resolve_batch(&model(), &input.to_string()),
        Err(fail("UnsupportedRoutePseudo"))
    );
    input["targets"][0]["pseudo"] = json!("after");
    assert!(resolve_batch(&model(), &input.to_string()).is_ok());
}

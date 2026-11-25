#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    conditions::{ConditionEnvironment, FeatureValue},
    facts::{AnswerDomain, MechanicalAnswer},
    model::{CompileLimits, CompiledStyles, SheetSource},
    prepared::{
        CascadeOrigin, PreparationEnvironment, PreparationLimits, PreparedStyles, SheetUse,
        StylePhase,
    },
    rule_matches::{
        matching_rules, matching_rules_with_implicit_policy, ImplicitRootPolicy, MatchError,
        SelectorAnswers, TargetFacts,
    },
    shadow_routes::{AssignedSlot, ContextRoot, RouteBudget, RouteFacts, ValidatedRouteFacts},
    target_scope::{Boundary, BoundaryAnswers},
    ComputationLimits, CssText,
};
use turbo_dom::rtdom::css_shorthand::LengthContext;
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 16384,
        max_total_units: 32768,
    }
}
fn model(css: &str, scopes: &[u32]) -> PreparedStyles {
    model_with_owners(
        css,
        &scopes
            .iter()
            .map(|scope| (*scope, None))
            .collect::<Vec<_>>(),
    )
}
fn model_with_owners(css: &str, uses: &[(u32, Option<u32>)]) -> PreparedStyles {
    let compiled = CompiledStyles::compile(
        &[SheetSource {
            id: 1,
            text: text(css),
        }],
        CompileLimits {
            sources: 4,
            source_units: 16384,
            rules: 128,
            source_rule_preludes: 128,
            selectors: 256,
            declarations: 256,
            properties: 2048,
            group_depth: 16,
            token_nesting: 32,
        },
    )
    .unwrap();
    let bases = [("em", 16.0), ("rem", 16.0)];
    let lengths = LengthContext { unit_bases: &bases };
    let features = BTreeMap::from([("width".to_owned(), FeatureValue::LengthPx(800.0))]);
    let selectors = BTreeMap::new();
    let conditions = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    compiled
        .prepare(
            &uses
                .iter()
                .map(|(scope, owner)| SheetUse {
                    source_id: 1,
                    origin: CascadeOrigin::Author,
                    tree_scope: *scope,
                    implicit_scope_root: *owner,
                    scope_depth: scope.saturating_sub(1),
                    disabled: false,
                    media: None,
                })
                .collect::<Vec<_>>(),
            &PreparationEnvironment {
                conditions: &conditions,
                style_phase: Some(StylePhase::Steady),
            },
            PreparationLimits {
                uses: 4,
                rules: 128,
                layers: 128,
                computation: limits(),
            },
        )
        .unwrap()
}
const ROOTS: [ContextRoot; 2] = [
    ContextRoot {
        tree_scope: 1,
        root: 10,
        host: None,
        parent_tree_scope: None,
    },
    ContextRoot {
        tree_scope: 2,
        root: 20,
        host: Some(11),
        parent_tree_scope: Some(1),
    },
];
fn route_facts(path: &[u32], scope: u32) -> RouteFacts<'_> {
    RouteFacts {
        target: path[0],
        own_tree_scope: scope,
        own_path: path,
        host_of: None,
        assigned_slot: None,
        context_roots: &ROOTS,
        requested_pseudo: None,
    }
}
fn validated<'a>(input: &RouteFacts<'a>) -> ValidatedRouteFacts<'a> {
    ValidatedRouteFacts::new(
        input,
        &mut RouteBudget {
            contexts: 2,
            path_units: 16,
        },
    )
    .unwrap()
}
fn answers(prepared: &PreparedStyles, subject: u32, root: Option<u32>) -> SelectorAnswers {
    let domain = AnswerDomain::new(vec![subject]).unwrap();
    prepared
        .metadata()
        .compiled
        .metadata()
        .selectors
        .iter()
        .enumerate()
        .flat_map(|(id, metadata)| {
            metadata
                .compiled
                .as_ref()
                .unwrap()
                .selectors()
                .iter()
                .enumerate()
                .map(move |(item, _)| (id, item))
        })
        .map(|(id, item)| {
            (
                (id, item, root),
                MechanicalAnswer::new(domain.clone(), vec![subject]).unwrap(),
            )
        })
        .collect()
}
fn facts<'a>(
    routes: &'a ValidatedRouteFacts<'a>,
    answers: &'a SelectorAnswers,
    boundaries: &'a BoundaryAnswers,
) -> TargetFacts<'a> {
    TargetFacts {
        containers: None,
        path: routes.facts().own_path,
        implicit_scope_root: *routes.facts().own_path.last().unwrap(),
        pseudo: None,
        selectors: answers,
        routes: Some(routes),
        boundaries,
    }
}
#[test]
fn ordinary_rules_use_only_original_own_tree_scope() {
    let prepared = model(".x{color:red}", &[1, 2]);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let answers = answers(&prepared, 21, None);
    let boundaries = BTreeMap::new();
    let matched = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(
        prepared.metadata().rules[matched.rules()[0].prepared_rule()].tree_scope,
        2
    );
}
#[test]
fn host_sheet_uses_exact_original_host_and_canonical_specificity() {
    let prepared = model(":host(.active){color:red}", &[2]);
    let mut input = route_facts(&[11, 10], 1);
    input.host_of = Some(2);
    let routes = validated(&input);
    let answers = answers(&prepared, 11, None);
    let boundaries = BTreeMap::new();
    let matched = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].specificity(), (0, 2, 0));
    let input = route_facts(&[12, 11, 10], 1);
    let routes = validated(&input);
    assert!(
        matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128)
            .unwrap()
            .rules()
            .is_empty()
    );
}
#[test]
fn slotted_sheet_uses_slot_path_but_answer_domain_remains_original_light_subject() {
    let prepared = model(".picked::slotted(.hl){color:red}", &[1, 2]);
    let mut input = route_facts(&[12, 11, 10], 1);
    input.assigned_slot = Some(AssignedSlot {
        slot: 22,
        tree_scope: 2,
        path: &[22, 23, 20],
    });
    let routes = validated(&input);
    let answers = answers(&prepared, 12, None);
    let boundaries = BTreeMap::new();
    let matched = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(
        prepared.metadata().rules[matched.rules()[0].prepared_rule()].tree_scope,
        2
    );
}
#[test]
fn unscoped_none_and_actual_scope_some_keys_are_distinct() {
    let prepared = model(".x{color:red}", &[1]);
    let input = route_facts(&[11, 10], 1);
    let routes = validated(&input);
    let wrong = answers(&prepared, 11, Some(10));
    let boundaries = BTreeMap::new();
    assert!(matches!(
        matching_rules(&prepared, &facts(&routes, &wrong, &boundaries), 128),
        Err(MatchError::MissingSelectorAnswer)
    ));
    let prepared = model("@scope(.root){.x{color:red}}", &[2]);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let domain = AnswerDomain::new(vec![21, 20]).unwrap();
    let boundaries = BTreeMap::from([(
        (0, Boundary::Start, 20),
        MechanicalAnswer::new(domain, vec![20]).unwrap(),
    )]);
    let answers = answers(&prepared, 21, Some(20));
    let matched = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128).unwrap();
    assert_eq!(matched.rules()[0].proximity(), 1);
}
#[test]
fn missing_shadow_provenance_and_mismatched_paths_fail_conservatively() {
    let prepared = model(".x{color:red}", &[2]);
    let answers = answers(&prepared, 21, None);
    let boundaries = BTreeMap::new();
    let plain = TargetFacts {
        containers: None,
        path: &[21, 20],
        implicit_scope_root: 20,
        pseudo: None,
        selectors: &answers,
        routes: None,
        boundaries: &boundaries,
    };
    assert!(matches!(
        matching_rules(&prepared, &plain, 128),
        Err(MatchError::MissingRouteFacts(2))
    ));
    let input = route_facts(&[11, 10], 1);
    let routes = validated(&input);
    let wrong = TargetFacts {
        routes: Some(&routes),
        ..plain
    };
    assert!(matches!(
        matching_rules(&prepared, &wrong, 128),
        Err(MatchError::InvalidRouteFacts)
    ));
}

#[test]
fn implicit_owner_is_per_stylesheet_use_and_known_nonancestor_is_empty() {
    let prepared = model_with_owners("@scope{.x{color:red}}", &[(1, Some(11)), (1, Some(14))]);
    assert_eq!(prepared.metadata().rules[0].implicit_scope_root, Some(11));
    assert_eq!(prepared.metadata().rules[1].implicit_scope_root, Some(14));
    let input = route_facts(&[12, 11, 10], 1);
    let routes = validated(&input);
    let answers = answers(&prepared, 12, Some(11));
    let boundaries = BTreeMap::new();
    let matched = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].prepared_rule(), 0);
    assert_eq!(matched.rules()[0].proximity(), 1);
}
#[test]
fn implicit_owner_missing_errors_but_explicit_root_ignores_unavailable_or_other_owner() {
    let input = route_facts(&[12, 11, 10], 1);
    let routes = validated(&input);
    let implicit = model("@scope{.x{color:red}}", &[1]);
    let empty = BTreeMap::new();
    let answers_implicit = answers(&implicit, 12, Some(10));
    let target = facts(&routes, &answers_implicit, &empty);
    assert!(matches!(
        matching_rules(&implicit, &target, 128),
        Err(MatchError::MissingImplicitRoot)
    ));
    assert_eq!(
        matching_rules_with_implicit_policy(
            &implicit,
            &target,
            128,
            ImplicitRootPolicy::LegacyTarget
        )
        .unwrap()
        .rules()
        .len(),
        1
    );
    for owner in [None, Some(14)] {
        let explicit = model_with_owners("@scope(.root){.x{color:red}}", &[(1, owner)]);
        let domain = AnswerDomain::new(vec![12, 11, 10]).unwrap();
        let boundaries = BTreeMap::from([(
            (0, Boundary::Start, 10),
            MechanicalAnswer::new(domain, vec![11]).unwrap(),
        )]);
        let selectors = answers(&explicit, 12, Some(11));
        let matched =
            matching_rules(&explicit, &facts(&routes, &selectors, &boundaries), 128).unwrap();
        assert_eq!(matched.rules().len(), 1);
    }
}
#[test]
fn relevant_scoped_slotted_route_remains_a_typed_unsupported_capability() {
    let prepared = model_with_owners("@scope{::slotted(.hl){color:red}}", &[(2, Some(20))]);
    let mut input = route_facts(&[12, 11, 10], 1);
    input.assigned_slot = Some(AssignedSlot {
        slot: 22,
        tree_scope: 2,
        path: &[22, 20],
    });
    let routes = validated(&input);
    let selectors = answers(&prepared, 12, Some(20));
    let boundaries = BTreeMap::new();
    assert!(matches!(
        matching_rules(&prepared, &facts(&routes, &selectors, &boundaries), 128),
        Err(MatchError::UnsupportedScopedSlotted)
    ));
    let input = route_facts(&[12, 11, 10], 1);
    let routes = validated(&input);
    assert!(
        matching_rules(&prepared, &facts(&routes, &selectors, &boundaries), 128)
            .unwrap()
            .rules()
            .is_empty()
    );
}

#[test]
fn selector_answer_present_but_original_subject_outside_its_domain_is_missing_capability() {
    let prepared = model(".x{color:red}", &[2]);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let selectors = answers(&prepared, 99, None);
    let boundaries = BTreeMap::new();
    assert!(matches!(
        matching_rules(&prepared, &facts(&routes, &selectors, &boundaries), 128),
        Err(MatchError::MissingSelectorAnswer)
    ));
}
#[test]
fn generated_before_after_routes_use_original_subject_and_reject_unrepresented_pseudo_context() {
    use turbo_dom::rtdom::computed_css::selector_policy::GeneratedPseudo;
    for (name, pseudo) in [
        ("before", GeneratedPseudo::Before),
        ("after", GeneratedPseudo::After),
    ] {
        let prepared = model(&format!(".x::{name}{{color:red}}"), &[2]);
        let mut input = route_facts(&[21, 20], 2);
        input.requested_pseudo = Some(pseudo);
        let routes = validated(&input);
        let selectors = answers(&prepared, 21, None);
        let boundaries = BTreeMap::new();
        let target = TargetFacts {
            pseudo: Some(name),
            ..facts(&routes, &selectors, &boundaries)
        };
        assert_eq!(
            matching_rules(&prepared, &target, 128)
                .unwrap()
                .rules()
                .len(),
            1
        );
    }
    let prepared = model(".x::marker{color:red}", &[2]);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let selectors = answers(&prepared, 21, None);
    let boundaries = BTreeMap::new();
    let target = TargetFacts {
        pseudo: Some("marker"),
        ..facts(&routes, &selectors, &boundaries)
    };
    assert!(matches!(
        matching_rules(&prepared, &target, 128),
        Err(MatchError::InvalidRouteFacts)
    ));
}
#[test]
fn zero_scope_legacy_pseudo_contract_preserves_unknown_generated_names_and_filters_other_subjects()
{
    let prepared = model(".x::marker{color:red}", &[0]);
    let selectors = answers(&prepared, 21, None);
    let boundaries = BTreeMap::new();
    let target = TargetFacts {
        containers: None,
        path: &[21, 20],
        implicit_scope_root: 20,
        pseudo: Some("marker"),
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
    };
    assert_eq!(
        matching_rules(&prepared, &target, 128)
            .unwrap()
            .rules()
            .len(),
        1
    );
    let ordinary = TargetFacts {
        pseudo: None,
        ..target
    };
    assert!(matching_rules(&prepared, &ordinary, 128)
        .unwrap()
        .rules()
        .is_empty());
}
#[test]
fn assigned_scope_relevance_does_not_admit_ordinary_rules_without_host_reference() {
    let prepared = model(".x{color:red}", &[2]);
    let mut input = route_facts(&[12, 11, 10], 1);
    input.assigned_slot = Some(AssignedSlot {
        slot: 22,
        tree_scope: 2,
        path: &[22, 20],
    });
    let routes = validated(&input);
    let selectors = answers(&prepared, 12, None);
    let boundaries = BTreeMap::new();
    assert!(
        matching_rules(&prepared, &facts(&routes, &selectors, &boundaries), 128)
            .unwrap()
            .rules()
            .is_empty()
    );
}
#[test]
fn scoped_boundary_answer_missing_one_original_ancestor_is_not_a_false_negative() {
    let prepared = model("@scope(.root){.x{color:red}}", &[2]);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let selectors = answers(&prepared, 21, Some(20));
    let boundaries = BTreeMap::from([(
        (0, Boundary::Start, 20),
        MechanicalAnswer::new(AnswerDomain::new(vec![20]).unwrap(), vec![20]).unwrap(),
    )]);
    assert!(matches!(
        matching_rules(&prepared, &facts(&routes, &selectors, &boundaries), 128),
        Err(MatchError::Scope(
            turbo_dom::rtdom::computed_css::target_scope::ScopeError::MissingAnswer
        ))
    ));
}

#[test]
fn malformed_author_scope_and_host_family_rules_are_discarded_without_fabricated_metadata() {
    use turbo_dom::rtdom::computed_css::selector_policy::SelectorPolicyInspection;
    let invalid = model("@scope broken{.x{color:red}}", &[2]);
    assert!(invalid.metadata().compiled.metadata().groups[0].invalid_scope);
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let selectors = answers(&invalid, 21, Some(20));
    let boundaries = BTreeMap::new();
    assert!(
        matching_rules(&invalid, &facts(&routes, &selectors, &boundaries), 128)
            .unwrap()
            .rules()
            .is_empty()
    );
    let invalid = model(":host(.a,.b){color:red}", &[2]);
    assert_eq!(
        invalid.metadata().compiled.metadata().selectors[0].policy,
        SelectorPolicyInspection::Invalid
    );
    let selectors = BTreeMap::new();
    assert!(
        matching_rules(&invalid, &facts(&routes, &selectors, &boundaries), 128)
            .unwrap()
            .rules()
            .is_empty()
    );
}
#[test]
fn empty_original_ancestor_path_has_no_valid_scope_membership() {
    use turbo_dom::rtdom::computed_css::target_scope::{scope_roots_with_owner_budget, ScopeError};
    assert_eq!(
        scope_roots_with_owner_budget(&[], &[], &[], None, &BTreeMap::new(), &mut 128),
        Err(ScopeError::InvalidPath)
    );
}

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

#[test]
fn mixed_host_and_ordinary_selector_items_keep_their_distinct_tree_routes() {
    let prepared = model(".bad.bad.bad,:host(.active){color:red}", &[2]);
    let mut host = route_facts(&[11, 10], 1);
    host.host_of = Some(2);
    let host_routes = validated(&host);
    let host_answers = answers(&prepared, 11, None);
    let boundaries = BTreeMap::new();
    // The core refuses an ordinary shadow-tree route even when the mechanical
    // adapter supplied an overbroad original-subject answer for that list item.
    let matched = matching_rules(
        &prepared,
        &facts(&host_routes, &host_answers, &boundaries),
        128,
    )
    .unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].specificity(), (0, 2, 0));
    let child = route_facts(&[21, 20], 2);
    let child_routes = validated(&child);
    let child_answers = answers(&prepared, 21, None);
    let matched = matching_rules(
        &prepared,
        &facts(&child_routes, &child_answers, &boundaries),
        128,
    )
    .unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].specificity(), (0, 3, 0));
}

#[test]
fn shared_source_host_answers_require_exact_binding_context_without_legacy_fallback() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers, ContextSelectorAnswers,
    };
    let prepared = model("#host,:host{color:red}", &[1, 2]);
    let mut input = route_facts(&[11, 10], 1);
    input.host_of = Some(2);
    let routes = validated(&input);
    let legacy = answers(&prepared, 11, None);
    let boundaries = BTreeMap::new();
    let target = facts(&routes, &legacy, &boundaries);
    assert_eq!(
        matching_rules(&prepared, &target, 128)
            .unwrap()
            .rules()
            .len(),
        2
    );
    let mut selectors = ContextSelectorAnswers::new();
    for key in legacy.keys() {
        for scope in [1, 2] {
            selectors.insert(
                (scope, key.0, key.1, key.2),
                MechanicalAnswer::new(
                    AnswerDomain::new(vec![11]).unwrap(),
                    if scope == 1 { vec![11] } else { vec![] },
                )
                .unwrap(),
            );
        }
    }
    let contextual_boundaries = BTreeMap::new();
    let contextual = ContextAnswers {
        selectors: &selectors,
        boundaries: &contextual_boundaries,
    };
    let matched = matching_rules_with_context_answers(&prepared, &target, contextual, 128).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(
        prepared.metadata().rules[matched.rules()[0].prepared_rule()].tree_scope,
        1
    );
    selectors.retain(|key, _| key.0 != 2);
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &contextual_boundaries
            },
            128
        ),
        Err(MatchError::MissingSelectorAnswer)
    ));
    for key in legacy.keys() {
        selectors.insert(
            (2, key.0, key.1, key.2),
            MechanicalAnswer::new(AnswerDomain::new(vec![99]).unwrap(), vec![]).unwrap(),
        );
    }
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &contextual_boundaries
            },
            128
        ),
        Err(MatchError::MissingSelectorAnswer)
    ));
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &contextual_boundaries
            },
            0
        ),
        Err(MatchError::ResourceLimit)
    ));
}

fn context_answer(domain: &[u32], matches: &[u32]) -> MechanicalAnswer {
    MechanicalAnswer::new(
        AnswerDomain::new(domain.to_vec()).unwrap(),
        matches.to_vec(),
    )
    .unwrap()
}
#[test]
fn contextual_shadow_scope_derives_host_without_replacing_physical_ancestry_or_owner() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let physical = [21, 20];
    let input = route_facts(&physical, 2);
    let routes = validated(&input);
    let legacy = BTreeMap::new();
    let legacy_boundaries = BTreeMap::new();
    let target = facts(&routes, &legacy, &legacy_boundaries);
    for css in ["@scope(:host){.x{color:red}}", "@scope{.x{color:red}}"] {
        let prepared = model_with_owners(css, &[(2, Some(20))]);
        let selectors = BTreeMap::from([((2, 0, 0, Some(11)), context_answer(&[21], &[21]))]);
        let boundaries = BTreeMap::from([(
            (2, 0, Boundary::Start, 20),
            context_answer(&[21, 11, 20], &[11]),
        )]);
        let matched = matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &boundaries,
            },
            128,
        )
        .unwrap();
        assert_eq!(matched.rules().len(), 1);
        assert_eq!(matched.rules()[0].proximity(), 1);
        assert_eq!(prepared.metadata().rules[0].implicit_scope_root, Some(20));
        assert_eq!(routes.facts().own_path, &physical);
        if css.contains(":host") {
            assert!(matches!(
                matching_rules(&prepared, &target, 128),
                Err(MatchError::Scope(
                    turbo_dom::rtdom::computed_css::target_scope::ScopeError::MissingAnswer
                ))
            ));
        }
    }
}
#[test]
fn contextual_scoped_scope_reference_admits_only_the_authentic_host_route() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let mut input = route_facts(&[11, 10], 1);
    input.host_of = Some(2);
    let routes = validated(&input);
    let legacy = BTreeMap::new();
    let legacy_boundaries = BTreeMap::new();
    let target = facts(&routes, &legacy, &legacy_boundaries);
    let selectors = BTreeMap::from([((2, 0, 0, Some(11)), context_answer(&[11], &[11]))]);
    let boundaries = BTreeMap::from([(
        (2, 0, Boundary::Start, 20),
        context_answer(&[11, 20], &[11]),
    )]);
    let prepared = model_with_owners("@scope(:host){:scope{color:red}}", &[(2, Some(20))]);
    let matched = matching_rules_with_context_answers(
        &prepared,
        &target,
        ContextAnswers {
            selectors: &selectors,
            boundaries: &boundaries,
        },
        128,
    )
    .unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].proximity(), 0);
    assert!(matching_rules(&prepared, &target, 128)
        .unwrap()
        .rules()
        .is_empty());
}

#[test]
fn contextual_scope_host_allocation_and_matching_budgets_are_exact_and_domains_required() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let legacy = BTreeMap::new();
    let legacy_boundaries = BTreeMap::new();
    let target = facts(&routes, &legacy, &legacy_boundaries);
    let prepared = model_with_owners("@scope(:host){.x{color:red}}", &[(2, Some(20))]);
    let selectors = BTreeMap::from([((2, 0, 0, Some(11)), context_answer(&[21], &[21]))]);
    let mut boundaries = BTreeMap::from([(
        (2, 0, Boundary::Start, 20),
        context_answer(&[21, 11, 20], &[11]),
    )]);
    for budget in [0, 2, 6] {
        assert!(matches!(
            matching_rules_with_context_answers(
                &prepared,
                &target,
                ContextAnswers {
                    selectors: &selectors,
                    boundaries: &boundaries
                },
                budget
            ),
            Err(MatchError::ResourceLimit)
        ));
    }
    assert_eq!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &boundaries
            },
            7
        )
        .unwrap()
        .rules()
        .len(),
        1
    );
    boundaries.insert((2, 0, Boundary::Start, 20), context_answer(&[21, 20], &[]));
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &boundaries
            },
            128
        ),
        Err(MatchError::Scope(
            turbo_dom::rtdom::computed_css::target_scope::ScopeError::MissingAnswer
        ))
    ));
}
#[test]
fn contextual_scope_host_does_not_alias_missing_foreign_owners_or_broaden_other_selectors() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let input = route_facts(&[21, 20], 2);
    let routes = validated(&input);
    let legacy = BTreeMap::new();
    let legacy_boundaries = BTreeMap::new();
    let target = facts(&routes, &legacy, &legacy_boundaries);
    let selectors = BTreeMap::new();
    let boundaries = BTreeMap::new();
    let unknown = model_with_owners("@scope{.x{color:red}}", &[(2, None)]);
    assert!(matches!(
        matching_rules_with_context_answers(
            &unknown,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &boundaries
            },
            128
        ),
        Err(MatchError::MissingImplicitRoot)
    ));
    let foreign = model_with_owners("@scope{.x{color:red}}", &[(2, Some(99))]);
    assert!(matching_rules_with_context_answers(
        &foreign,
        &target,
        ContextAnswers {
            selectors: &selectors,
            boundaries: &boundaries
        },
        128
    )
    .unwrap()
    .rules()
    .is_empty());
    let mut input = route_facts(&[11, 10], 1);
    input.host_of = Some(2);
    let routes = validated(&input);
    let target = facts(&routes, &legacy, &legacy_boundaries);
    for css in [":scope{color:red}", "@scope(:host){.x{color:red}}"] {
        let prepared = model_with_owners(css, &[(2, Some(20))]);
        assert!(matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &selectors,
                boundaries: &boundaries
            },
            128
        )
        .unwrap()
        .rules()
        .is_empty());
    }
}

fn oracle_id(value: &serde_json::Value) -> u32 {
    u32::try_from(value.as_u64().unwrap()).unwrap()
}
fn oracle_ids(value: &serde_json::Value) -> Vec<u32> {
    value.as_array().unwrap().iter().map(oracle_id).collect()
}
fn oracle_context_answers(
    fixture: &serde_json::Value,
) -> (
    turbo_dom::rtdom::computed_css::rule_matches::ContextSelectorAnswers,
    turbo_dom::rtdom::computed_css::target_scope::ContextBoundaryAnswers,
) {
    let mut selectors = BTreeMap::new();
    let mut boundaries = BTreeMap::new();
    let domain = oracle_ids(&fixture["domain"]["nodeIds"]);
    for context in fixture["contexts"].as_array().unwrap() {
        for answer in context["selectorAnswers"].as_array().unwrap() {
            let root = answer["rootId"]
                .as_u64()
                .map(|id| u32::try_from(id).unwrap());
            selectors.insert(
                (
                    oracle_id(&answer["treeScope"]),
                    answer["selectorId"].as_u64().unwrap() as usize,
                    answer["itemId"].as_u64().unwrap() as usize,
                    root,
                ),
                context_answer(&domain, &oracle_ids(&answer["matches"])),
            );
        }
        for answer in context["boundaryAnswers"].as_array().unwrap() {
            let boundary = match answer["boundary"].as_str().unwrap() {
                "start" => Boundary::Start,
                "end" => Boundary::End,
                _ => panic!("unknown fixture boundary"),
            };
            boundaries.insert(
                (
                    oracle_id(&answer["treeScope"]),
                    answer["groupId"].as_u64().unwrap() as usize,
                    boundary,
                    oracle_id(&answer["rootId"]),
                ),
                context_answer(&domain, &oracle_ids(&answer["matches"])),
            );
        }
    }
    (selectors, boundaries)
}
fn oracle_path(fixture: &serde_json::Value, subject: u32) -> Vec<u32> {
    oracle_ids(
        &fixture["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| oracle_id(&row["id"]) == subject)
            .unwrap()["physicalPath"],
    )
}
fn oracle_style(
    fixture: &serde_json::Value,
    prepared: &PreparedStyles,
    subject: u32,
) -> turbo_dom::rtdom::computed_css::resolve::ComputedStyle {
    use turbo_dom::rtdom::computed_css::{
        computed_values::{ValueContext, ValueLimits},
        custom_cascade::CustomCascadeLimits,
        inline_style::InlineLimits,
        properties::InitialContext,
        resolve::{compute_style_with_context_answers, ResolveLimits, TargetContext},
        rule_matches::ContextAnswers,
    };
    let ids = &fixture["ids"];
    let document = oracle_id(&ids["document"]);
    let host = oracle_id(&ids["host"]);
    let shadow = oracle_id(&ids["shadow"]);
    let slot = oracle_id(&ids["slot"]);
    let light = oracle_id(&ids["light"]);
    let path = oracle_path(fixture, subject);
    let slot_path = oracle_path(fixture, slot);
    let roots = [
        ContextRoot {
            tree_scope: document,
            root: document,
            host: None,
            parent_tree_scope: None,
        },
        ContextRoot {
            tree_scope: shadow,
            root: shadow,
            host: Some(host),
            parent_tree_scope: Some(document),
        },
    ];
    let scope = if path.last() == Some(&shadow) {
        shadow
    } else {
        document
    };
    let input = RouteFacts {
        target: subject,
        own_tree_scope: scope,
        own_path: &path,
        host_of: (subject == host).then_some(shadow),
        assigned_slot: (subject == light).then_some(AssignedSlot {
            slot,
            tree_scope: shadow,
            path: &slot_path,
        }),
        context_roots: &roots,
        requested_pseudo: None,
    };
    let routes = validated(&input);
    let empty = BTreeMap::new();
    let boundaries = BTreeMap::new();
    let target = facts(&routes, &empty, &boundaries);
    let (selectors, boundaries) = oracle_context_answers(fixture);
    let context = TargetContext {
        tree_scope: scope,
        scope_depth: scope.saturating_sub(1),
        values: ValueContext {
            initial: InitialContext {
                color: "rgb(0, 0, 0)",
                font_family: "serif",
                font_size: "16px",
            },
            unit_bases: &[("rem", 16.0)],
            font_size_unit_bases: &[("rem", 16.0)],
            is_root: false,
        },
    };
    compute_style_with_context_answers(
        prepared,
        &target,
        None,
        None,
        &context,
        ResolveLimits {
            matching_decisions: 4096,
            inline: InlineLimits {
                computation: limits(),
                declarations: 128,
                properties: 2048,
            },
            custom: CustomCascadeLimits {
                computation: limits(),
                rollbacks: 128,
            },
            values: ValueLimits {
                computation: limits(),
                retained_units: 32768,
            },
        },
        ContextAnswers {
            selectors: &selectors,
            boundaries: &boundaries,
        },
    )
    .unwrap()
}
fn oracle_value(
    style: &turbo_dom::rtdom::computed_css::resolve::ComputedStyle,
    name: &str,
) -> String {
    use turbo_dom::rtdom::computed_css::{
        computed_values::{compute_values, ValueContext, ValueLimits},
        properties::{initial, InitialContext},
    };
    if let Some(value) = style.values.values.get(name) {
        return String::from_utf16(value.as_units()).unwrap();
    }
    let context = ValueContext {
        initial: InitialContext {
            color: "rgb(0, 0, 0)",
            font_family: "serif",
            font_size: "16px",
        },
        unit_bases: &[("rem", 16.0)],
        font_size_unit_bases: &[("rem", 16.0)],
        is_root: false,
    };
    let values = compute_values(
        None,
        &BTreeMap::from([(name.to_owned(), initial(name, &context.initial))]),
        &context,
        ValueLimits {
            computation: limits(),
            retained_units: 32768,
        },
    )
    .unwrap();
    String::from_utf16(values.values[name].as_units()).unwrap()
}
#[test]
fn contextual_scope_host_matches_all_46_actual_chrome159_scope_and_proximity_rows() {
    use sha2::{Digest, Sha256};
    let source = include_str!("fixtures/computed_scope_host_chrome159.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(source)),
        "3aa1280ee8d51d229a66b452e55e9f9c503dc0ed44300040041a40877739a1e7"
    );
    let corpus: serde_json::Value = serde_json::from_str(source).unwrap();
    let mut count = 0;
    for fixture in corpus["fixtures"].as_array().unwrap() {
        let css = fixture["source"].as_str().unwrap();
        let ids = &fixture["ids"];
        assert_eq!(
            format!("{:x}", Sha256::digest(css)),
            fixture["sourceSha256"].as_str().unwrap()
        );
        let document = oracle_id(&ids["document"]);
        let shadow = oracle_id(&ids["shadow"]);
        for expected in fixture["expected"].as_array().unwrap() {
            let uses = match expected["mode"].as_str() {
                Some("document") => vec![(document, None)],
                Some("shadow") => vec![(shadow, None)],
                Some("both") => vec![(document, None), (shadow, None)],
                None => vec![(shadow, Some(shadow))],
                _ => panic!("unknown fixture mode"),
            };
            let prepared = model_with_owners(css, &uses);
            let host = oracle_style(fixture, &prepared, oracle_id(&ids["host"]));
            let child = oracle_style(fixture, &prepared, oracle_id(&ids["shadowB"]));
            if expected["mode"].is_string() {
                let light = oracle_style(fixture, &prepared, oracle_id(&ids["light"]));
                assert_eq!(
                    oracle_value(&host, "background-color"),
                    expected["background"].as_str().unwrap(),
                    "host {expected}"
                );
                assert_eq!(
                    oracle_value(&child, "background-color"),
                    expected["childBackground"].as_str().unwrap(),
                    "child {expected}"
                );
                assert_eq!(
                    oracle_value(&light, "background-color"),
                    expected["lightBackground"].as_str().unwrap(),
                    "light {expected}"
                );
            } else {
                assert_eq!(
                    oracle_value(&child, "color"),
                    expected["childColor"].as_str().unwrap(),
                    "child {expected}"
                );
                assert_eq!(
                    oracle_value(&host, "background-color"),
                    expected["hostBackground"].as_str().unwrap(),
                    "host {expected}"
                );
            }
            count += 1;
        }
    }
    assert_eq!(count, 46);
}

#[test]
fn flat_scoped_context_migration_preserves_legacy_rank_but_never_falls_back_to_legacy_maps() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let prepared = model_with_owners("@scope(.root){.x{color:red}}", &[(0, Some(1))]);
    let selectors = BTreeMap::from([((0, 0, Some(2)), context_answer(&[2], &[2]))]);
    let boundaries = BTreeMap::from([((0, Boundary::Start, 1), context_answer(&[2, 1], &[2]))]);
    let target = TargetFacts {
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
        containers: None,
    };
    let contextual_selectors = BTreeMap::from([((0, 0, 0, Some(2)), context_answer(&[2], &[2]))]);
    let contextual_boundaries =
        BTreeMap::from([((0, 0, Boundary::Start, 1), context_answer(&[2, 1], &[2]))]);
    let legacy = matching_rules(&prepared, &target, 128).unwrap();
    let contextual = matching_rules_with_context_answers(
        &prepared,
        &target,
        ContextAnswers {
            selectors: &contextual_selectors,
            boundaries: &contextual_boundaries,
        },
        128,
    )
    .unwrap();
    assert_eq!(legacy.rules(), contextual.rules());
    assert_eq!(contextual.rules().len(), 1);
    let wrong = BTreeMap::from([((1, 0, 0, Some(2)), context_answer(&[2], &[2]))]);
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &wrong,
                boundaries: &contextual_boundaries
            },
            128
        ),
        Err(MatchError::MissingSelectorAnswer)
    ));
}
#[test]
fn inconsistent_physical_ancestry_containing_its_declared_shadow_host_is_rejected() {
    use turbo_dom::rtdom::computed_css::rule_matches::{
        matching_rules_with_context_answers, ContextAnswers,
    };
    let input = route_facts(&[21, 11, 20], 2);
    let routes = validated(&input);
    let selectors = BTreeMap::new();
    let boundaries = BTreeMap::new();
    let target = facts(&routes, &selectors, &boundaries);
    let contextual_selectors = BTreeMap::new();
    let contextual_boundaries = BTreeMap::new();
    let prepared = model_with_owners("@scope(:host){.x{color:red}}", &[(2, Some(20))]);
    assert!(matches!(
        matching_rules_with_context_answers(
            &prepared,
            &target,
            ContextAnswers {
                selectors: &contextual_selectors,
                boundaries: &contextual_boundaries
            },
            128
        ),
        Err(MatchError::InvalidRouteFacts)
    ));
}

#[test]
fn ordinary_invalid_and_terminal_pseudo_rules_need_no_route_answers() {
    for selector in [
        "div[data-x=]",
        "#target, div[data-x=]",
        "div:not(.absent, [data-x=])",
        "::selection",
        "::-webkit-scrollbar",
        "#target::before",
        "#target::after",
    ] {
        let prepared = model(
            &format!("#target{{color:red}} {selector}{{color:blue}}"),
            &[1],
        );
        let input = route_facts(&[12, 11, 10], 1);
        let routes = validated(&input);
        let domain = AnswerDomain::new(vec![12]).unwrap();
        // Only the real element selector has a mechanical answer. Ignored CSS
        // must not consume an answer or turn missing data into a false match.
        let answers = BTreeMap::from([(
            (0, 0, None),
            MechanicalAnswer::new(domain, vec![12]).unwrap(),
        )]);
        let boundaries = BTreeMap::new();
        let result = matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128);
        assert!(result.is_ok(), "{selector}: {:?}", result.as_ref().err());
        assert_eq!(result.unwrap().rules().len(), 1, "{selector}");
    }
}

#[test]
fn composed_pseudo_and_missing_mechanical_answers_stay_errors() {
    for selector in ["::part(label)", ":host-context(.outer)"] {
        let prepared = model(&format!("{selector}{{color:blue}}"), &[1]);
        let input = route_facts(&[12, 11, 10], 1);
        let routes = validated(&input);
        let answers = BTreeMap::new();
        let boundaries = BTreeMap::new();
        assert!(matches!(
            matching_rules(&prepared, &facts(&routes, &answers, &boundaries), 128),
            Err(MatchError::Route(_))
        ));
    }
    let prepared = model("#target{color:blue}", &[1]);
    let input = route_facts(&[12, 11, 10], 1);
    let routes = validated(&input);
    assert!(matching_rules(
        &prepared,
        &facts(&routes, &BTreeMap::new(), &BTreeMap::new()),
        128
    )
    .is_err());
}

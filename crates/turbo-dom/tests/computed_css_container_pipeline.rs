#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    conditions::{ConditionEnvironment, FeatureValue},
    container::{ContainerAncestor, ContainerError, ContentSize, InlineAxis, SizeKind},
    facts::{AnswerDomain, MechanicalAnswer},
    model::{CompileLimits, CompiledStyles, SheetSource},
    prepared::{
        CascadeOrigin, PreparationEnvironment, PreparationLimits, PreparedStyles, SheetUse,
        StylePhase,
    },
    rule_matches::{
        matching_rules, ContainerMatchContext, MatchError, MatchedRules, SelectorAnswers,
        TargetFacts,
    },
    target_scope::{Boundary, BoundaryAnswers},
    ComputationLimits, ComputedCssError, CssText,
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
fn model(css: &str) -> PreparedStyles {
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
            &[SheetUse {
                source_id: 1,
                origin: CascadeOrigin::Author,
                tree_scope: 0,
                implicit_scope_root: Some(1),
                scope_depth: 0,
                disabled: false,
                media: None,
            }],
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
fn selectors(prepared: &PreparedStyles, root: Option<u32>) -> SelectorAnswers {
    let domain = AnswerDomain::new(vec![1, 2]).unwrap();
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
                MechanicalAnswer::new(domain.clone(), vec![2]).unwrap(),
            )
        })
        .collect()
}
fn run(
    prepared: &PreparedStyles,
    target: u32,
    context: Option<ContainerMatchContext<'_>>,
) -> Result<MatchedRules, MatchError> {
    let selectors = selectors(prepared, None);
    let boundaries = BoundaryAnswers::new();
    let path = if target == 1 { vec![1] } else { vec![2, 1] };
    let facts = TargetFacts {
        path: &path,
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
        containers: context,
    };
    matching_rules(prepared, &facts, 64)
}
fn ancestor<'a>(lengths: &'a LengthContext<'a>, width: Option<f64>) -> ContainerAncestor<'a> {
    ContainerAncestor {
        id: 42,
        names: &[],
        kind: SizeKind::InlineSize,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: width.map(|width_px| ContentSize {
            width_px,
            height_px: 100.0,
        }),
        length_context: lengths,
    }
}
#[test]
fn actual_container_content_size_activates_only_positive_target_selector() {
    let prepared = model("@container (width > 300px){#target{color:red}}");
    assert!(run(&prepared, 1, None).unwrap().rules().is_empty());
    assert!(matches!(
        run(&prepared, 2, None),
        Err(MatchError::RequiresContainer(0))
    ));
    let lengths = LengthContext { unit_bases: &[] };
    for (width, count) in [(400.0, 1), (200.0, 0)] {
        let ancestors = [ancestor(&lengths, Some(width))];
        let matched = run(
            &prepared,
            2,
            Some(ContainerMatchContext {
                measurements: None,
                ancestors: &ancestors,
                limits: limits(),
            }),
        )
        .unwrap();
        assert_eq!(matched.rules().len(), count);
    }
}
#[test]
fn selected_container_missing_measurement_is_typed_instead_of_inactive() {
    let prepared = model("@container (width > 300px){#target{color:red}}");
    let lengths = LengthContext { unit_bases: &[] };
    let ancestors = [ancestor(&lengths, None)];
    assert!(matches!(
        run(
            &prepared,
            2,
            Some(ContainerMatchContext {
                measurements: None,
                ancestors: &ancestors,
                limits: limits()
            })
        ),
        Err(MatchError::Container(ContainerError::RequiresMeasurement(
            42
        )))
    ));
}
#[test]
fn invalid_container_syntax_is_inactive_but_caller_limits_are_not() {
    let lengths = LengthContext { unit_bases: &[] };
    let ancestors = [ancestor(&lengths, Some(400.0))];
    let context = ContainerMatchContext {
        measurements: None,
        ancestors: &ancestors,
        limits: limits(),
    };
    let invalid = model("@container (width >){#target{color:red}}");
    assert!(run(&invalid, 2, Some(context)).unwrap().rules().is_empty());
    let valid = model("@container (width > 300px){#target{color:red}}");
    let mut policy = limits();
    policy.max_output_units = 2;
    assert!(matches!(
        run(
            &valid,
            2,
            Some(ContainerMatchContext {
                limits: policy,
                ..context
            })
        ),
        Err(MatchError::Container(ContainerError::Computation(
            ComputedCssError::ResourceLimit
        )))
    ));
}
#[test]
fn style_and_scroll_state_capabilities_remain_typed_after_selector_admission() {
    let lengths = LengthContext { unit_bases: &[] };
    let mut ancestor = ancestor(&lengths, Some(400.0));
    ancestor.scroll_state = true;
    let ancestors = [ancestor];
    let context = Some(ContainerMatchContext {
        measurements: None,
        ancestors: &ancestors,
        limits: limits(),
    });
    assert!(matches!(
        run(
            &model("@container style(--theme:dark){#target{color:red}}"),
            2,
            context
        ),
        Err(MatchError::Container(ContainerError::RequiresStyleQuery(
            42
        )))
    ));
    assert!(matches!(
        run(
            &model("@container scroll-state(stuck:top){#target{color:red}}"),
            2,
            context
        ),
        Err(MatchError::Container(ContainerError::RequiresScrollState(
            42
        )))
    ));
}
#[test]
fn scoped_container_keeps_scope_owner_answers_and_proximity() {
    let prepared = model("@scope (.root){@container (width > 300px){#target{color:red}}}");
    let metadata = prepared.metadata();
    let group = metadata
        .compiled
        .metadata()
        .groups
        .iter()
        .position(|group| group.scope.is_some())
        .unwrap();
    let owner = metadata.rules[0].implicit_scope_root.unwrap();
    assert_eq!(owner, 1);
    let selectors = selectors(&prepared, Some(owner));
    let domain = AnswerDomain::new(vec![1, 2]).unwrap();
    let boundaries = BTreeMap::from([(
        (group, Boundary::Start, owner),
        MechanicalAnswer::new(domain, vec![owner]).unwrap(),
    )]);
    let lengths = LengthContext { unit_bases: &[] };
    let ancestors = [ancestor(&lengths, Some(400.0))];
    let facts = TargetFacts {
        path: &[2, 1],
        implicit_scope_root: 1,
        pseudo: None,
        selectors: &selectors,
        routes: None,
        boundaries: &boundaries,
        containers: Some(ContainerMatchContext {
            measurements: None,
            ancestors: &ancestors,
            limits: limits(),
        }),
    };
    let matched = matching_rules(&prepared, &facts, 64).unwrap();
    assert_eq!(matched.rules().len(), 1);
    assert_eq!(matched.rules()[0].proximity(), 1);
    let wrong = TargetFacts {
        boundaries: &BTreeMap::new(),
        ..facts
    };
    assert!(matches!(
        matching_rules(&prepared, &wrong, 64),
        Err(MatchError::Scope(_))
    ));
}
#[test]
fn native_namespace_matching_is_retained_before_container_admission() {
    let tree = turbo_dom::rtdom::Tree::parse("<svg><rect id=target></rect></svg>");
    let rect = tree.query_selector("#target").unwrap();
    for selector in ["*|rect", "|rect"] {
        let prepared = model(&format!(
            "@container (width > 300px){{{selector}{{color:red}}}}"
        ));
        let native_match = tree.matches_checked(rect, selector).unwrap();
        let domain = AnswerDomain::new(vec![2]).unwrap();
        let selectors = BTreeMap::from([(
            (0, 0, None),
            MechanicalAnswer::new(domain, if native_match { vec![2] } else { vec![] }).unwrap(),
        )]);
        let boundaries = BoundaryAnswers::new();
        let facts = TargetFacts {
            path: &[2, 1],
            implicit_scope_root: 1,
            pseudo: None,
            selectors: &selectors,
            routes: None,
            boundaries: &boundaries,
            containers: None,
        };
        if native_match {
            assert!(matches!(
                matching_rules(&prepared, &facts, 64),
                Err(MatchError::RequiresContainer(0))
            ));
        } else {
            assert!(matching_rules(&prepared, &facts, 64)
                .unwrap()
                .rules()
                .is_empty());
        }
    }
}

#[test]
fn partial_actual_content_axes_flow_through_rule_matching_without_invented_height() {
    use turbo_dom::rtdom::computed_css::container::{ContainerMeasurements, PartialContentSize};
    let lengths = LengthContext { unit_bases: &[] };
    let mut value = ancestor(&lengths, None);
    value.kind = SizeKind::Size;
    let ancestors = [value];
    let measurements = ContainerMeasurements::from([(
        42,
        PartialContentSize {
            width_px: Some(400.0),
            height_px: None,
        },
    )]);
    let context = Some(ContainerMatchContext {
        ancestors: &ancestors,
        measurements: Some(&measurements),
        limits: limits(),
    });
    assert_eq!(
        run(
            &model("@container (width > 300px){#target{color:red}}"),
            2,
            context
        )
        .unwrap()
        .rules()
        .len(),
        1
    );
    assert!(matches!(
        run(
            &model("@container (height > 1px){#target{color:red}}"),
            2,
            context
        ),
        Err(MatchError::Container(ContainerError::RequiresMeasurement(
            42
        )))
    ));
}

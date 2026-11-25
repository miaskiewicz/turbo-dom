//! Public semantic and typed-conversion contracts of the optional CSS evaluator.
#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment, ConditionEvaluationError, FeatureValue},
        container::{
            container_matches, ContainerAncestor, ContainerError, ContentSize, InlineAxis, SizeKind,
        },
        model::{CompileLimits, CompiledStyles, SheetSource},
        target_scope::{scope_roots, BoundaryAnswers},
        ComputationLimits, ComputedCssError, CssText,
    },
    css_shorthand::LengthContext,
};
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 8192,
    }
}
#[test]
fn reversed_media_equality_compares_the_measured_feature() {
    let lengths = LengthContext { unit_bases: &[] };
    let selectors = BTreeMap::new();
    for (width, expected) in [(800.0, true), (801.0, false)] {
        let features = BTreeMap::from([("width".into(), FeatureValue::LengthPx(width))]);
        let environment = ConditionEnvironment {
            media_type: "screen",
            features: &features,
            length_context: &lengths,
            selector_support: &selectors,
        };
        assert_eq!(
            media_matches(&text("(800px = width)"), &environment, limits()),
            Ok(expected)
        );
    }
}
#[test]
fn unknown_container_features_are_not_measured_width_aliases() {
    let lengths = LengthContext { unit_bases: &[] };
    let ancestor = ContainerAncestor {
        id: 7,
        names: &[],
        kind: SizeKind::Size,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: Some(ContentSize {
            width_px: 800.0,
            height_px: 600.0,
        }),
        length_context: &lengths,
    };
    for query in ["(--future: 800px)", "(future-width: 800px)"] {
        assert!(
            !container_matches(&text(query), &[ancestor], limits())
                .unwrap()
                .matches,
            "{query}"
        );
    }
    let cold = ContainerAncestor {
        size: None,
        ..ancestor
    };
    assert_eq!(
        container_matches(&text("(width: 800px)"), &[cold], limits()),
        Err(ContainerError::RequiresMeasurement(7))
    );
    let mut bounded = limits();
    bounded.max_output_units = 1;
    assert_eq!(
        container_matches(&text("(width: 800px)"), &[ancestor], bounded),
        Err(ContainerError::Computation(ComputedCssError::ResourceLimit))
    );
}
#[test]
fn container_error_conversion_preserves_resource_and_missing_feature_identity() {
    // This is an explicit conversion contract, not fabricated parser output.
    assert_eq!(
        ContainerError::from(ConditionEvaluationError::Computation(
            ComputedCssError::ResourceLimit
        )),
        ContainerError::Computation(ComputedCssError::ResourceLimit)
    );
    assert_eq!(
        ContainerError::from(ConditionEvaluationError::RequiresFeature(
            "future-width".into()
        )),
        ContainerError::Condition(ConditionEvaluationError::RequiresFeature(
            "future-width".into()
        ))
    );
}
#[test]
fn malformed_scope_prelude_has_no_eligible_target_root() {
    let compiled = CompiledStyles::compile(
        &[SheetSource {
            id: 1,
            text: text("@scope (.root) to () { #target { color:red } }"),
        }],
        CompileLimits {
            sources: 1,
            source_units: 4096,
            rules: 16,
            source_rule_preludes: 16,
            selectors: 16,
            declarations: 16,
            properties: 128,
            group_depth: 8,
            token_nesting: 32,
        },
    )
    .unwrap();
    let data = compiled.metadata();
    assert_eq!(data.groups.len(), 1);
    assert!(data.groups[0].invalid_scope);
    assert_eq!(
        scope_roots(&data.groups, &[0], &[2, 1], 1, &BoundaryAnswers::new(), 16),
        Ok(Vec::new())
    );
}

#[test]
fn unrelated_shadow_stylesheet_context_has_no_selector_route() {
    use turbo_dom::rtdom::computed_css::{
        selector_policy::{inspect_selector_policy, SelectorPolicyInspection},
        shadow_routes::{
            selector_route, ContextRoot, RouteBudget, RouteFacts, ValidatedRouteFacts,
        },
    };
    // These are request-local numeric context facts, not fabricated selector ASTs.
    let roots = [
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
    let facts = RouteFacts {
        target: 12,
        own_tree_scope: 1,
        own_path: &[12, 10],
        host_of: None,
        assigned_slot: None,
        context_roots: &roots,
        requested_pseudo: None,
    };
    let validated = ValidatedRouteFacts::new(
        &facts,
        &mut RouteBudget {
            contexts: 2,
            path_units: 2,
        },
    )
    .unwrap();
    let SelectorPolicyInspection::Items(policies) =
        inspect_selector_policy(&text(".target, :host"), limits()).unwrap()
    else {
        panic!("ordinary class selector must be classified")
    };
    assert!(selector_route(2, &policies[0], &validated)
        .unwrap()
        .is_none());
    assert!(selector_route(1, &policies[0], &validated)
        .unwrap()
        .is_some());
}

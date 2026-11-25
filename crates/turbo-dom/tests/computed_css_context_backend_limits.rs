#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{
            media_matches, supports_matches, ConditionEnvironment, ConditionEvaluationError,
        },
        container::{container_matches, ContainerError},
        grammar::{parse_supports_condition, parse_supports_condition_context, ConditionError},
        ComputationLimits, ComputedCssError, CssText,
    },
    css_shorthand::LengthContext,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 600,
        max_output_units: 4096,
        max_total_units: 8192,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn evaluate(
    source: &CssText,
    supports: bool,
    policy: ComputationLimits,
) -> Result<bool, ConditionEvaluationError> {
    let features = BTreeMap::new();
    let selectors = BTreeMap::new();
    let lengths = LengthContext { unit_bases: &[] };
    let context = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    if supports {
        supports_matches(source, &context, policy)
    } else {
        media_matches(source, &context, policy)
    }
}
#[test]
fn whole_contexts_reject_backend_excess_before_any_recursive_ast_parser() {
    let raw = format!("{}red{}", "f(".repeat(501), ")".repeat(501));
    assert!(matches!(
        parse_supports_condition_context(&raw, limits()),
        Err(ConditionError::NestingLimit)
    ));
    for supports in [true, false] {
        assert_eq!(
            evaluate(&text(&raw), supports, limits()),
            Err(ConditionEvaluationError::Computation(
                ComputedCssError::NestingLimit
            ))
        );
    }
    assert_eq!(
        container_matches(&text(&raw), &[], limits()),
        Err(ContainerError::Computation(ComputedCssError::NestingLimit))
    );
}
#[test]
fn explicit_context_unit_and_stricter_nesting_errors_keep_their_identity() {
    let mut policy = limits();
    policy.max_output_units = 2;
    assert!(matches!(
        parse_supports_condition_context("(display:grid)", policy),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    ));
    for supports in [true, false] {
        assert_eq!(
            evaluate(&text("(width:1px)"), supports, policy),
            Err(ConditionEvaluationError::Computation(
                ComputedCssError::ResourceLimit
            ))
        );
    }
    assert_eq!(
        container_matches(&text("(width:1px)"), &[], policy),
        Err(ContainerError::Computation(ComputedCssError::ResourceLimit))
    );
    policy = limits();
    policy.max_nesting = 2;
    assert!(matches!(
        parse_supports_condition_context("(((display:grid)))", policy),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    ));
}
#[test]
fn short_context_grammar_retains_original_strict_supports_contract() {
    assert_eq!(
        parse_supports_condition_context("(display:grid)", limits()).unwrap(),
        parse_supports_condition("(display:grid)").unwrap()
    );
    assert_eq!(
        parse_supports_condition_context("not", limits()).unwrap_err(),
        ConditionError::InvalidSyntax
    );
    assert_eq!(evaluate(&text("(display:grid)"), true, limits()), Ok(true));
    assert!(
        !container_matches(&text("(width:1px)"), &[], limits())
            .unwrap()
            .matches
    );
}

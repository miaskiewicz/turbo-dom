//! Explicit-limit APIs preserve caller quotas before conditional syntax recovery.
#![cfg(feature = "computed-style-grammar")]
use cssparser::{Parser, ParserInput};
use turbo_dom::rtdom::computed_css::{
    grammar::{
        parse_media_condition_context, parse_supports_condition_context, zoom_declaration,
        ConditionError,
    },
    ComputationLimits, ComputedCssError,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 8,
        max_output_units: 64,
        max_total_units: 0,
    }
}
#[test]
fn invalid_conditional_syntax_cannot_discharge_original_unit_quota() {
    let mut bounded = limits();
    bounded.max_output_units = 1;
    assert!(matches!(
        parse_media_condition_context("print and, screen", bounded),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    ));
    assert!(matches!(
        parse_supports_condition_context("(color: red) or (future(foo))", bounded),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    ));
    assert!(parse_media_condition_context("print and, screen", limits()).is_ok());
    assert!(parse_supports_condition_context("(color: red)", limits()).is_ok());
}
#[test]
fn shallow_caller_bound_precedes_general_enclosed_recovery() {
    let mut bounded = limits();
    bounded.max_nesting = 1;
    assert!(matches!(
        parse_media_condition_context("(future(foo))", bounded),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    ));
    assert!(matches!(
        parse_supports_condition_context("(future(foo))", bounded),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    ));
}
fn zoom(source: &str) -> Result<String, ()> {
    let mut source = ParserInput::new(source);
    let mut parser = Parser::new(&mut source);
    let result = zoom_declaration(&mut parser).map_err(|_| ())?;
    parser.expect_exhausted().map_err(|_| ())?;
    Ok(result)
}
#[test]
fn raw_legacy_zoom_keeps_deferred_values_and_rejects_unrelated_tokens() {
    for source in [
        "var(--factor)",
        "env(scale-factor)",
        "inherit",
        "revert-layer",
    ] {
        assert_eq!(zoom(source), Ok(source.to_owned()));
    }
    for source in ["garbage", "1px", "var()"] {
        assert!(zoom(source).is_err(), "{source}");
    }
}

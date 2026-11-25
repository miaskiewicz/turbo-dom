#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::grammar::{
    parse_media_condition, parse_supports_condition, ConditionError,
};
#[test]
fn raw_ast_entrypoints_reject_backend_excess_before_recursion_without_a_size_budget() {
    let source = format!("{}red{}", "f(".repeat(501), ")".repeat(501));
    assert!(matches!(
        parse_media_condition(&source),
        Err(ConditionError::NestingLimit)
    ));
    assert!(matches!(
        parse_supports_condition(&source),
        Err(ConditionError::NestingLimit)
    ));
}
#[test]
fn raw_ast_shallow_large_sources_have_no_invented_source_unit_floor() {
    let media = "screen".repeat(1000);
    assert!(parse_media_condition(&media).is_ok());
    let supports = format!("(unknown-property:{})", "x".repeat(10000));
    assert!(parse_supports_condition(&supports).is_ok());
    assert!(parse_media_condition("screen or (future(foo))").is_err());
    assert!(parse_supports_condition("not").is_err());
}

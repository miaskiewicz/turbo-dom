#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{supports_matches, ConditionEnvironment, ConditionEvaluationError},
        grammar::{supports_condition_declaration, supports_declaration, ConditionError},
        ComputationLimits, ComputedCssError, CssText,
    },
    css_shorthand::LengthContext,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn evaluate(value: &str, policy: ComputationLimits) -> Result<bool, ConditionEvaluationError> {
    let features = BTreeMap::new();
    let selectors = BTreeMap::new();
    let lengths = LengthContext { unit_bases: &[] };
    let environment = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    supports_matches(&text(value), &environment, policy)
}
#[test]
fn supports_condition_and_two_argument_syntax_are_distinct_in_actual_chrome159() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_supports_context_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    for row in control["results"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let value = row["value"].as_str().unwrap();
        let expected = row["condition"].as_bool().unwrap();
        assert_eq!(
            supports_condition_declaration(name, value, limits()).unwrap(),
            expected,
            "{name}:{value}"
        );
        assert_eq!(
            evaluate(&format!("({name}:{value})"), limits()),
            Ok(expected),
            "{name}:{value}"
        );
    }
    assert_eq!(
        supports_declaration("position-anchor", "--card", limits()),
        Ok(false)
    );
    assert_eq!(
        supports_condition_declaration("position-anchor", "--card", limits()),
        Ok(true)
    );
}
#[test]
fn missing_grammar_is_typed_and_only_decisive_boolean_branches_discharge_it() {
    let missing = Err(ConditionEvaluationError::RequiresGrammar(
        "trigger-scope".into(),
    ));
    assert_eq!(evaluate("(trigger-scope:--card)", limits()), missing);
    assert_eq!(evaluate("not (trigger-scope:--card)", limits()), missing);
    assert_eq!(
        evaluate("(trigger-scope:--card) or (display:grid)", limits()),
        Ok(true)
    );
    assert_eq!(
        evaluate(
            "(trigger-scope:--card) and (future-property:value)",
            limits()
        ),
        Ok(false)
    );
    assert_eq!(
        supports_condition_declaration("trigger-scope", "--card", limits()),
        Err(ConditionError::RequiresGrammar("trigger-scope".into()))
    );
}
#[test]
fn resource_and_nesting_failures_do_not_become_unsupported_css() {
    let mut policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        evaluate("(trigger-scope:--card) or (display:grid)", policy),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::ResourceLimit
        ))
    );
    policy = limits();
    policy.max_nesting = 2;
    assert_eq!(
        evaluate("(display:grid) or (((trigger-scope:--card)))", policy),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
}

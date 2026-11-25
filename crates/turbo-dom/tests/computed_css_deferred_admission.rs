#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    grammar::{computed_declaration_value, ConditionError, DeclarationAdmission},
    ComputationLimits, ComputedCssError,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn deferred_admission_validates_registry_only_and_legacy_families() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/css_deferred_admission_chrome159.json"
    ))
    .unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    for row in control["results"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let value = row["value"].as_str().unwrap();
        if value.ends_with(" !important") {
            assert_eq!(
                computed_declaration_value(name, value, limits()).unwrap(),
                DeclarationAdmission::Invalid
            );
            continue;
        }
        let expected = row["supports"].as_bool().unwrap();
        let actual = computed_declaration_value(name, value, limits()).unwrap();
        assert_eq!(
            matches!(actual, DeclarationAdmission::Valid(_)),
            expected,
            "{name}: {value}: {actual:?}"
        );
        assert!(
            !matches!(actual, DeclarationAdmission::RequiresGrammar(_)),
            "{name}: {value}"
        );
    }
}
#[test]
fn deferred_limits_remain_typed_even_in_unused_fallbacks() {
    let mut policy = limits();
    policy.max_nesting = 2;
    assert_eq!(
        computed_declaration_value("position-anchor", "var(--x, f(f(f(1))))", policy),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    );
    policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        computed_declaration_value("position-anchor", "var(--x)", policy),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    );
}

#[test]
fn anchor_scope_positive_inverse_case_and_escape_controls() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_anchor_scope_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    for row in control["results"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let value = row["value"].as_str().unwrap();
        let expected = row["supports"].as_bool().unwrap();
        let actual = computed_declaration_value(name, value, limits()).unwrap();
        assert_eq!(
            matches!(actual, DeclarationAdmission::Valid(_)),
            expected,
            "{name}: {value}: {actual:?}"
        );
        assert!(!matches!(actual, DeclarationAdmission::RequiresGrammar(_)));
    }
}

#[test]
fn strict_shorthand_whitespace_preserves_legacy_wrapper_and_css_tokens() {
    use turbo_dom::rtdom::css_shorthand::{expand, expand_css};
    assert!(expand("margin", "\u{a0}1px\u{a0}").is_some());
    for space in ['\u{a0}', '\u{85}', '\u{2003}'] {
        assert!(expand_css("margin", &format!("{space}1px{space}")).is_none());
    }
    assert!(expand_css("margin", " \t1px\n\r\u{c}").is_some());
}

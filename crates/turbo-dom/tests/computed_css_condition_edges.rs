#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment, ConditionEvaluationError, FeatureValue},
        ComputationLimits, CssText,
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
fn media(query: &str, facts: &[(&str, FeatureValue)]) -> Result<bool, ConditionEvaluationError> {
    let features = facts
        .iter()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect();
    let selectors = BTreeMap::new();
    let lengths = LengthContext { unit_bases: &[] };
    let environment = ConditionEnvironment {
        media_type: "screen",
        features: &features,
        length_context: &lengths,
        selector_support: &selectors,
    };
    media_matches(
        &CssText::from_units(query.encode_utf16().collect()),
        &environment,
        limits(),
    )
}
#[test]
fn media_intervals_reverse_each_comparison_without_reversing_truth() {
    let facts = [("width", FeatureValue::LengthPx(200.0))];
    for query in [
        "(100px < width < 300px)",
        "(100px <= width <= 200px)",
        "(300px > width > 100px)",
        "(300px >= width >= 200px)",
    ] {
        assert_eq!(media(query, &facts), Ok(true), "{query}");
    }
    assert_eq!(media("(width < 200px)", &facts), Ok(false));
    assert_eq!(media("(300px < width < 400px)", &facts), Ok(false));
}
#[test]
fn numeric_boolean_ratio_and_resolution_facts_keep_their_types() {
    assert_eq!(
        media("(color:8)", &[("color", FeatureValue::Number(8.0))]),
        Ok(true)
    );
    assert_eq!(
        media(
            "(-webkit-device-pixel-ratio:1.5)",
            &[("-webkit-device-pixel-ratio", FeatureValue::Number(1.5))]
        ),
        Ok(true)
    );
    assert_eq!(
        media("(grid:1)", &[("grid", FeatureValue::Boolean(true))]),
        Ok(true)
    );
    assert_eq!(
        media(
            "(aspect-ratio)",
            &[("aspect-ratio", FeatureValue::Ratio(16.0, 9.0))]
        ),
        Ok(true)
    );
    assert_eq!(
        media(
            "(resolution > 1dpcm)",
            &[("resolution", FeatureValue::ResolutionDppx(1.0))]
        ),
        Ok(true)
    );
}
#[test]
fn invalid_measured_values_and_wrong_host_types_are_errors() {
    assert!(matches!(
        media("(hover)", &[("hover", FeatureValue::Ident(String::new()))]),
        Err(ConditionEvaluationError::InvalidFeature(_))
    ));
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(matches!(
            media("(width:1px)", &[("width", FeatureValue::LengthPx(value))]),
            Err(ConditionEvaluationError::InvalidFeature(_))
        ));
    }
    assert!(matches!(
        media("(width:1px)", &[("width", FeatureValue::Boolean(true))]),
        Err(ConditionEvaluationError::InvalidFeature(_))
    ));
    assert!(matches!(
        media(
            "(aspect-ratio)",
            &[("aspect-ratio", FeatureValue::Ratio(1.0, -1.0))]
        ),
        Err(ConditionEvaluationError::InvalidFeature(_))
    ));
}
#[test]
fn unknown_features_do_not_turn_into_missing_known_measurements() {
    assert_eq!(media("(future-feature)", &[]), Ok(false));
    assert!(
        matches!(media("(--host-feature)",&[]),Err(ConditionEvaluationError::RequiresFeature(name))if name=="--host-feature")
    );
    assert_eq!(media("(future(foo)) and (width:1px)", &[]), Ok(false));
    assert_eq!(
        media(
            "not ((future(foo)) and (width:1px))",
            &[("width", FeatureValue::LengthPx(2.0))]
        ),
        Ok(true)
    );
}
#[test]
fn extreme_and_invalid_ratios_are_not_division_rounding_shortcuts() {
    assert_eq!(
        media(
            "(aspect-ratio:1e20/1e20)",
            &[("aspect-ratio", FeatureValue::Ratio(1e308, 1e308))]
        ),
        Ok(true)
    );
    assert_eq!(
        media(
            "(aspect-ratio:-1/1)",
            &[("aspect-ratio", FeatureValue::Ratio(1.0, 1.0))]
        ),
        Err(ConditionEvaluationError::InvalidSyntax)
    );
}
#[test]
fn query_environment_variables_require_a_provider_instead_of_guessing() {
    assert!(matches!(
        media(
            "(width:env(viewport-segment-width 0 0))",
            &[("width", FeatureValue::LengthPx(200.0))]
        ),
        Err(ConditionEvaluationError::UnsupportedValue(_))
    ));
}

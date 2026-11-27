//! Host measurements and capability errors are distinct from CSS unknown conditions.
#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{
            media_matches, supports_matches, ConditionEnvironment, ConditionEvaluationError,
            FeatureValue,
        },
        ComputationLimits, ComputedCssError, CssText,
    },
    css_shorthand::{LengthContext, LengthError},
};

fn text(source: &str) -> CssText {
    CssText::from_units(source.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
fn facts() -> BTreeMap<String, FeatureValue> {
    BTreeMap::from([
        ("width".into(), FeatureValue::LengthPx(800.0)),
        ("height".into(), FeatureValue::LengthPx(600.0)),
        (
            "orientation".into(),
            FeatureValue::Ident("landscape".into()),
        ),
        ("resolution".into(), FeatureValue::ResolutionDppx(1.0)),
        ("aspect-ratio".into(), FeatureValue::Ratio(4.0, 3.0)),
        ("hover".into(), FeatureValue::Ident("hover".into())),
        (
            "prefers-reduced-motion".into(),
            FeatureValue::Ident("no-preference".into()),
        ),
        ("color".into(), FeatureValue::Number(8.0)),
        ("grid".into(), FeatureValue::Boolean(false)),
    ])
}

fn evaluate(
    source: &str,
    media: bool,
    features: &BTreeMap<String, FeatureValue>,
    selectors: &BTreeMap<String, bool>,
    bases: &[(&str, f64)],
    limits: ComputationLimits,
) -> Result<bool, ConditionEvaluationError> {
    let context = LengthContext { unit_bases: bases };
    let environment = ConditionEnvironment {
        media_type: "screen",
        features,
        length_context: &context,
        selector_support: selectors,
    };
    if media {
        media_matches(&text(source), &environment, limits)
    } else {
        supports_matches(&text(source), &environment, limits)
    }
}

#[test]
fn known_media_features_use_real_measurements_and_canonical_query_units() {
    let features = facts();
    let selectors = BTreeMap::new();
    let bases = [("em", 16.0), ("rem", 16.0), ("vw", 8.0), ("vh", 6.0)];
    for (source, expected) in [
        ("", true),
        ("screen and (min-width:1px)", true),
        ("(min-width:10000px)", false),
        ("print", false),
        ("not print", true),
        ("only screen", true),
        ("(orientation:LANDSCAPE)", true),
        ("(orientation:portrait)", false),
        ("(resolution:1dppx)", true),
        ("(resolution:96dpi)", true),
        ("(resolution:2dppx)", false),
        ("(aspect-ratio:4/3)", true),
        ("(aspect-ratio:0/0)", false),
        ("(max-aspect-ratio:16/0)", true),
        ("(10px < width <= 800px)", true),
        ("(800px < width < 1000px)", false),
        ("(min-width:-1px)", true),
        ("(max-width:-1px)", false),
        ("(width:50em)", true),
        ("(width:50rem)", true),
        ("(width:100vw)", true),
        ("(prefers-reduced-motion)", false),
        ("(hover)", true),
        ("(color)", true),
        ("(grid)", false),
    ] {
        assert_eq!(
            evaluate(source, true, &features, &selectors, &bases, limits()),
            Ok(expected),
            "{source}"
        );
    }
}

#[test]
fn unknown_css_features_do_not_replace_missing_known_host_capabilities() {
    let empty = BTreeMap::new();
    let selectors = BTreeMap::new();
    for source in ["(future-feature:1)", "(future(foo))", "speech"] {
        assert_eq!(
            evaluate(source, true, &empty, &selectors, &[], limits()),
            Ok(false),
            "{source}"
        );
    }
    assert_eq!(
        evaluate("(width:1px)", true, &empty, &selectors, &[], limits()),
        Err(ConditionEvaluationError::RequiresFeature("width".into()))
    );
    assert_eq!(
        evaluate("(width:50em)", true, &facts(), &selectors, &[], limits()),
        Err(ConditionEvaluationError::Length(
            LengthError::RequiresBasis("em".into())
        ))
    );
    assert_eq!(
        evaluate(
            "(width:50em)",
            true,
            &facts(),
            &selectors,
            &[("em", -1.0)],
            limits()
        ),
        Err(ConditionEvaluationError::Length(LengthError::InvalidBasis(
            "em".into()
        )))
    );
}

#[test]
fn media_short_circuit_can_discharge_missing_capabilities_in_either_order() {
    let empty = BTreeMap::new();
    let selectors = BTreeMap::new();
    for (source, expected) in [
        ("screen, (width:1px)", true),
        ("(width:1px), screen", true),
        ("(width:1px) and (future-feature:1)", false),
        ("(future-feature:1) and (width:1px)", false),
        ("print and (width:1px)", false),
    ] {
        assert_eq!(
            evaluate(source, true, &empty, &selectors, &[], limits()),
            Ok(expected),
            "{source}"
        );
    }
}

#[test]
fn forgiving_media_syntax_preserves_host_and_supports_errors() {
    let mut features = facts();
    let selectors = BTreeMap::new();
    features.insert("width".into(), FeatureValue::LengthPx(f64::NAN));
    assert_eq!(
        evaluate("(width:1px)", true, &features, &selectors, &[], limits()),
        Err(ConditionEvaluationError::InvalidFeature("width".into()))
    );
    features.insert("width".into(), FeatureValue::LengthPx(-1.0));
    assert_eq!(
        evaluate("(width:1px)", true, &features, &selectors, &[], limits()),
        Err(ConditionEvaluationError::InvalidFeature("width".into()))
    );
    assert_eq!(
        evaluate("???", true, &features, &selectors, &[], limits()),
        Ok(false)
    );
    assert!(turbo_dom::rtdom::computed_css::grammar::parse_media_condition("???").is_err());
    assert_eq!(
        evaluate("not", false, &features, &selectors, &[], limits()),
        Err(ConditionEvaluationError::InvalidSyntax)
    );
}

#[test]
fn supports_uses_canonical_declarations_and_explicit_selector_support() {
    let features = facts();
    let selectors = BTreeMap::from([(".known".into(), true), (":made-up-pseudo".into(), false)]);
    for (source, expected) in [
        ("(display:grid)", true),
        ("(made-up-property:value)", false),
        ("not (made-up-property:value)", true),
        ("future(foo)", false),
        ("(display:grid) or future(foo)", true),
        ("(display:grid) or (color:var())", true),
        ("(color:var())", false),
        ("(color:red!important)", true),
        ("(color:red ! bad)", false),
        ("(--x:any)", true),
        ("(--x:)", true),
        ("(--x:any!important)", true),
        ("(--x:var(--missing))", true),
        ("(--x:var())", false),
        ("(--x:x;y)", false),
        ("(--x:x!bad)", false),
        ("selector(.known)", true),
        ("selector(:made-up-pseudo)", false),
    ] {
        assert_eq!(
            evaluate(source, false, &features, &selectors, &[], limits()),
            Ok(expected),
            "{source}"
        );
    }
    assert_eq!(
        evaluate(
            "selector(:hover)",
            false,
            &features,
            &selectors,
            &[],
            limits()
        ),
        Err(ConditionEvaluationError::RequiresSelector(":hover".into()))
    );
}

#[test]
fn supports_short_circuit_does_not_turn_unclassified_selectors_into_false() {
    let features = facts();
    let selectors = BTreeMap::new();
    for (source, expected) in [
        ("selector(:hover) or (display:grid)", true),
        ("(display:grid) or selector(:hover)", true),
        ("selector(:hover) and (made-up-property:value)", false),
        ("(made-up-property:value) and selector(:hover)", false),
    ] {
        assert_eq!(
            evaluate(source, false, &features, &selectors, &[], limits()),
            Ok(expected),
            "{source}"
        );
    }
    for source in [
        "not selector(:hover)",
        "selector(:hover) or (made-up-property:value)",
    ] {
        assert_eq!(
            evaluate(source, false, &features, &selectors, &[], limits()),
            Err(ConditionEvaluationError::RequiresSelector(":hover".into())),
            "{source}"
        );
    }
}

#[test]
fn original_caller_limits_apply_before_library_recursion_and_short_circuit() {
    let features = facts();
    let selectors = BTreeMap::new();
    let exact = "(display:grid)";
    let units = exact.encode_utf16().count();
    let mut policy = limits();
    policy.max_nesting = 2;
    policy.max_output_units = units;
    assert_eq!(
        evaluate(exact, false, &features, &selectors, &[], policy),
        Ok(true)
    );
    policy.max_nesting = 1;
    assert_eq!(
        evaluate(exact, false, &features, &selectors, &[], policy),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
    policy = limits();
    policy.max_output_units = units - 1;
    assert_eq!(
        evaluate(exact, false, &features, &selectors, &[], policy),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::ResourceLimit
        ))
    );
    policy = limits();
    policy.max_nesting = 0;
    assert_eq!(
        evaluate("screen", true, &features, &selectors, &[], policy),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
    policy = limits();
    policy.max_nesting = 2;
    assert_eq!(
        evaluate(
            "(display:grid) or (color:var(--x,f(deep)))",
            false,
            &features,
            &selectors,
            &[],
            policy
        ),
        Err(ConditionEvaluationError::Computation(
            ComputedCssError::NestingLimit
        ))
    );
}

#[test]
fn general_enclosed_media_retains_unknown_through_not_and_boolean_context() {
    let features = facts();
    let selectors = BTreeMap::new();
    for (source, expected) in [
        ("future(foo)", false),
        ("(future(foo))", false),
        ("not (future(foo))", false),
        ("not future(foo)", false),
        ("not(future(foo))", false),
        ("not ((future(foo)))", false),
        ("(future(foo)) or (width:800px)", true),
        ("(width:800px) or (future(foo))", true),
        ("(future(foo)) and (width:800px)", false),
        ("(future(foo)), screen", true),
        ("screen, (future(foo)),", true),
        ("not ((future(foo)) and (width:0px))", true),
        ("not ((future(foo)) or (width:800px))", false),
        ("not (future-feature:1)", false),
        ("not ((future-feature:1) and (width:0px))", true),
        ("not print and (future(foo))", true),
        ("not screen and (future(foo))", false),
        ("not futuretype", true),
        ("(foo bar)", false),
        ("not (foo bar)", false),
        ("(width:)", false),
        ("not (width:)", false),
        ("()", false),
        ("not ()", false),
        ("(123) or (width:800px)", true),
        ("(future(foo)", false),
        ("future(var()) or (width:800px)", true),
        ("future(\"and ) , not\") or (width:800px)", true),
        ("future({x:[y]}) or (width:800px)", true),
    ] {
        assert_eq!(
            evaluate(source, true, &features, &selectors, &[], limits()),
            Ok(expected),
            "{source}"
        );
    }
}

#[test]
fn forgiving_media_context_never_matches_malformed_queries() {
    let features = facts();
    let selectors = BTreeMap::new();
    for source in [
        "???",
        "screen or future(foo)",
        "not future(foo) and (width:800px)",
        "(future(foo)) and",
        "(future(foo)) or",
        "(future(foo)))",
        "(future(foo)) and (width:800px) or (height:600px)",
        "(future([bad))",
        "future(foo), screen and (width:800px) or (height:600px)",
    ] {
        assert_eq!(
            evaluate(source, true, &features, &selectors, &[], limits()),
            Ok(false),
            "{source}"
        );
        assert!(
            turbo_dom::rtdom::computed_css::grammar::parse_media_condition(source).is_err(),
            "strict parser: {source}"
        );
    }
}

#[test]
fn unknown_and_missing_fact_domains_are_collapsed_only_at_the_final_match() {
    let empty = BTreeMap::new();
    let selectors = BTreeMap::new();
    assert_eq!(
        evaluate(
            "(future(foo)) and (width:0px)",
            true,
            &empty,
            &selectors,
            &[],
            limits()
        ),
        Ok(false)
    );
    assert_eq!(
        evaluate(
            "not ((future(foo)) and (width:0px))",
            true,
            &empty,
            &selectors,
            &[],
            limits()
        ),
        Err(ConditionEvaluationError::RequiresFeature("width".into()))
    );
    assert_eq!(
        evaluate(
            "(future(foo)) or (width:800px)",
            true,
            &empty,
            &selectors,
            &[],
            limits()
        ),
        Err(ConditionEvaluationError::RequiresFeature("width".into()))
    );
    assert_eq!(
        evaluate(
            "screen, (future(foo)) or (width:800px)",
            true,
            &empty,
            &selectors,
            &[],
            limits()
        ),
        Ok(true)
    );
}

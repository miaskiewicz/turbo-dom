//! Optional grammar behavior shared by native CSSOM and the Node/WASM CSS kernel.
#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::grammar::{
    declaration_value, is_supported_property, parse_media_condition, parse_supports_condition,
    MediaCondition, MediaFeatureComparison, MediaFeatureId, MediaFeatureName, MediaType, Qualifier,
    QueryFeature, SupportsCondition,
};

#[test]
fn known_and_vendor_properties_keep_native_admission_policy() {
    for name in [
        "color",
        "zoom",
        "interest-delay",
        "scroll-snap-align",
        "-WEBKIT-LINE-CLAMP",
        "-webkit-column-rule",
    ] {
        assert!(is_supported_property(name), "{name}");
    }
    for name in [
        "-moz-appearance",
        "-MOZ-appearance",
        "-webkit-backdrop-filter",
        "made-up-property",
        "--Custom",
    ] {
        assert!(!is_supported_property(name), "{name}");
        assert_eq!(declaration_value(name, "red"), None);
    }
}

#[test]
fn ordinary_values_are_validated_without_computed_value_rewriting() {
    for (name, raw) in [
        ("color", "  RED  "),
        ("width", "2em"),
        ("width", "var(--measure)"),
        ("width", "inherit"),
        ("box-shadow", "none"),
        ("writing-mode", "vertical-rl"),
        ("font-variant", "small-caps"),
        ("interest-delay", "normal 20ms"),
    ] {
        assert_eq!(
            declaration_value(name, raw).as_deref(),
            Some(raw.trim()),
            "{name}: {raw}"
        );
    }
    for (name, raw) in [
        ("width", "garbage"),
        ("color", "red !important"),
        ("interest-delay", "-1s"),
        ("interest-delay", "normal normal normal"),
        ("interest-delay-start", "1s 2s"),
        ("writing-mode", "left"),
        ("font-variant", "invented"),
    ] {
        assert_eq!(declaration_value(name, raw), None, "{name}: {raw}");
    }
}

#[test]
fn special_property_spelling_reuses_the_existing_native_paths() {
    for (name, raw, expected) in [
        ("zoom", "+2", "2"),
        ("zoom", "NORMAL", "normal"),
        ("zoom", "INHERIT", "inherit"),
        ("zoom", "var(--z)", "var(--z)"),
        ("zoom", "calc(50% + 50%)", "calc(100%)"),
        ("scroll-behavior", "SMOOTH", "smooth"),
        ("scroll-snap-type", "Y Proximity", "y"),
        ("scroll-snap-type", "both mandatory", "both mandatory"),
        ("scroll-snap-align", "center center", "center"),
        ("scroll-snap-align", "START end", "start end"),
        ("scroll-snap-stop", "Normal", "normal"),
        ("scroll-snap-type", "initial", "initial"),
    ] {
        assert_eq!(
            declaration_value(name, raw).as_deref(),
            Some(expected),
            "{name}: {raw}"
        );
    }
    for (name, raw) in [
        ("zoom", "reset"),
        ("zoom", "-1"),
        ("scroll-behavior", "instant"),
        ("scroll-snap-type", "none mandatory"),
        ("scroll-snap-type", "y x"),
        ("scroll-snap-align", "start end center"),
        ("scroll-snap-align", "left"),
        ("scroll-snap-align", ""),
        ("scroll-snap-stop", "never"),
    ] {
        assert_eq!(declaration_value(name, raw), None, "{name}: {raw}");
    }
}

#[test]
fn deferred_if_grammar_keeps_native_condition_validation() {
    for (raw, admitted) in [
        ("if(style(--x: 1): red; else: blue)", true),
        (
            "if((style(--x) or (media(print))) and supports(display: grid): red)",
            true,
        ),
        ("if(not media(print): red;)", true),
        ("if(else:)", true),
        ("if(red)", false),
        ("if()", false),
        ("if(: red)", false),
        ("if(style(--x); red)", false),
        ("if(not style(--x) and media(print): red)", false),
        (
            "if(style(--x) and media(print) or supports(a: b): red)",
            false,
        ),
        ("if(style(--x) and: red)", false),
        ("if(style(--x) red: blue)", false),
        ("if((style(--x): red)", false),
        ("if((): red)", false),
        ("if(else: red;;)", false),
    ] {
        assert_eq!(declaration_value("color", raw).is_some(), admitted, "{raw}");
    }
}

#[test]
fn media_ast_exposes_type_ranges_and_unknown_features_without_evaluation() {
    let source = String::from(
        "not screen and (min-width:10px), (10px < width <= 20px), (future-feature: 1)",
    );
    let media = parse_media_condition(&source).unwrap();
    assert_eq!(media.media_queries.len(), 3);
    assert!(matches!(
        media.media_queries[0].qualifier,
        Some(Qualifier::Not)
    ));
    assert!(matches!(
        media.media_queries[0].media_type,
        MediaType::Screen
    ));
    assert!(matches!(
        &media.media_queries[0].condition,
        Some(MediaCondition::Feature(QueryFeature::Range {
            name: MediaFeatureName::Standard(MediaFeatureId::Width),
            operator: MediaFeatureComparison::GreaterThanEqual,
            ..
        }))
    ));
    assert!(matches!(
        &media.media_queries[1].condition,
        Some(MediaCondition::Feature(QueryFeature::Interval { .. }))
    ));
    assert!(matches!(
        &media.media_queries[2].condition,
        Some(MediaCondition::Feature(QueryFeature::Plain {
            name: MediaFeatureName::Unknown(_),
            ..
        }))
    ));
    assert!(parse_media_condition("???").is_err());
    assert!(parse_media_condition("screen trailing").is_err());
}

#[test]
fn supports_ast_preserves_declaration_selector_and_general_enclosed_leaves() {
    let source = String::from("(display:grid) and selector(:hover)");
    let SupportsCondition::And(parts) = parse_supports_condition(&source).unwrap() else {
        panic!("and condition")
    };
    assert!(
        matches!(&parts[0], SupportsCondition::Declaration {value,..} if value.as_ref()=="grid")
    );
    assert!(matches!(&parts[1], SupportsCondition::Selector(source) if source.as_ref()==":hover"));
    assert!(matches!(
        parse_supports_condition("future(foo)").unwrap(),
        SupportsCondition::Unknown(_)
    ));
    assert!(parse_supports_condition("not").is_err());
    assert!(parse_supports_condition("(display:grid) trailing").is_err());
}

#[test]
fn layer_names_keep_canonical_identifier_boundaries_and_chrome159_keywords() {
    use turbo_dom::rtdom::computed_css::grammar::parse_layer_names;
    for (source, expected) in [
        (
            "reset, app.colors",
            vec![vec!["reset"], vec!["app", "colors"]],
        ),
        ("initial", vec![vec!["initial"]]),
        ("default", vec![vec!["default"]]),
        ("revert-layer", vec![vec!["revert-layer"]]),
        ("foo.inherit", vec![vec!["foo", "inherit"]]),
        ("\\69 nitial", vec![vec!["initial"]]),
        ("foo\\.bar", vec![vec!["foo.bar"]]),
    ] {
        assert_eq!(parse_layer_names(source).unwrap(), expected, "{source}");
    }
    for source in ["", "foo,", "foo .bar", "foo..bar", "foo bar", "foo;"] {
        assert!(parse_layer_names(source).is_err(), "{source}");
    }
}

#[test]
fn checked_ordinary_admission_preserves_upstream_nesting_failure() {
    use turbo_dom::rtdom::computed_css::grammar::{declaration_value_checked, ConditionError};
    assert_eq!(
        declaration_value_checked("color", "red", computation_limits()),
        Ok(Some("red".into()))
    );
    assert_eq!(
        declaration_value_checked("color", "not-a-color", computation_limits()),
        Ok(None)
    );
    assert_eq!(
        declaration_value_checked("width", "var()", computation_limits()),
        Ok(None)
    );
    let deeply_nested = format!("{}x{}", "f(".repeat(502), ")".repeat(502));
    assert_eq!(
        declaration_value_checked("color", &deeply_nested, computation_limits()),
        Err(ConditionError::Computation(
            turbo_dom::rtdom::computed_css::ComputedCssError::NestingLimit
        ))
    );
    assert_eq!(
        declaration_value_checked("unknown-property", "f(x)", computation_limits()),
        Ok(None)
    );
}

#[test]
fn supports_declarations_use_valid_priority_and_custom_token_grammar() {
    use turbo_dom::rtdom::computed_css::grammar::supports_declaration;
    for (name, source, expected) in [
        ("color", "red!important", true),
        ("display", "grid !important", true),
        ("color", "red ! bad", false),
        ("color", "var()", false),
        ("--x", "", true),
        ("--x", "any!important", true),
        ("--x", "var(--missing)", true),
        ("--x", "var()", false),
        ("--x", "x;y", false),
        ("--x", ";", false),
        ("--x", "x!bad", false),
    ] {
        assert_eq!(
            supports_declaration(name, source, computation_limits()),
            Ok(expected),
            "{name}: {source}"
        );
    }
}

fn computation_limits() -> turbo_dom::rtdom::computed_css::ComputationLimits {
    turbo_dom::rtdom::computed_css::ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 0,
    }
}

#[test]
fn checked_admission_checks_source_limits_before_legacy_grammar() {
    use turbo_dom::rtdom::computed_css::{
        grammar::{declaration_value_checked, supports_declaration, ConditionError},
        ComputedCssError,
    };
    let mut policy = computation_limits();
    policy.max_output_units = 2;
    assert_eq!(
        declaration_value_checked("color", "red", policy),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    );
    assert_eq!(
        supports_declaration("color", "red", policy),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    );
    policy = computation_limits();
    let deeply_nested = format!("{}x{}", "f(".repeat(502), ")".repeat(502));
    assert_eq!(
        supports_declaration("color", &deeply_nested, policy),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    );
}

//! Existing Chrome controls exercise the optional shared grammar, not a CSSOM serializer.
#![cfg(feature = "computed-style-grammar")]
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::computed_css::{
    grammar::{
        computed_declaration_value, declaration_value_checked, is_supported_property,
        parse_layer_names, parse_media_condition, parse_supports_condition, ConditionError,
        DeclarationAdmission,
    },
    ComputationLimits, ComputedCssError,
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 1_000_000,
        max_total_units: 0,
    }
}
fn json(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap()
}
fn full_admitted(name: &str, raw: &str) -> Result<bool, String> {
    match computed_declaration_value(name, raw, limits()).map_err(|error| format!("{error:?}"))? {
        DeclarationAdmission::Valid(_) => Ok(true),
        DeclarationAdmission::Invalid => Ok(false),
        DeclarationAdmission::RequiresGrammar(name) => Err(format!("RequiresGrammar({name})")),
    }
}
fn admission(name: &str, raw: &str, expected: bool) {
    assert_eq!(full_admitted(name, raw), Ok(expected), "{name}: {raw}");
}
#[test]
fn existing_longhand_control_preserves_ordinary_admission() {
    let control = json(include_str!("fixtures/css_longhand_chrome156.json"));
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        control["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-longhand-probe.js"))
        )
    );
    assert_eq!(
        control["casesSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/css-longhand-cases.json"))
        )
    );
    let mut mismatches = Vec::new();
    for row in control["result"][0]["result"].as_array().unwrap() {
        let name = row[0].as_str().unwrap();
        if name.starts_with("--") {
            continue;
        }
        let raw = row[1].as_str().unwrap();
        let expected = !row[2].as_str().unwrap().is_empty();
        let actual = full_admitted(name, raw);
        if actual.as_ref().copied() != Ok(expected) {
            mismatches.push(format!(
                "{name}: {raw} expected={expected} actual={actual:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} corpus mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
#[test]
fn prefixed_property_policy_reuses_existing_native_control() {
    let control = json(include_str!("fixtures/css_webkit_prefix_chrome156.json"));
    for row in control["prefixed"].as_array().unwrap() {
        let name = row[0].as_str().unwrap();
        let expected = row[2].as_bool().unwrap();
        assert_eq!(is_supported_property(name), expected, "{name}");
        admission(name, row[1].as_str().unwrap(), expected);
    }
    let current = json(include_str!("fixtures/css_vendor_admission_chrome159.json"));
    assert_eq!(current["version"], "159.0.8154.6");
    for row in current["output"].as_array().unwrap() {
        let name = row["property"].as_str().unwrap();
        if name.starts_with("--") {
            assert!(!is_supported_property(name));
            continue;
        }
        admission(name, "rotate(1deg)", row["supports"].as_bool().unwrap());
    }
}
#[test]
fn tree_counting_shape_and_geometry_box_control_reaches_shared_grammar() {
    let control = json(include_str!(
        "fixtures/css_tree_counting_shape_chrome156.json"
    ));
    assert_eq!(control["browser"], "Chrome/156.0.8078.80");
    for family in ["tree", "shapes", "boxes"] {
        for row in control["result"][0]["result"][family].as_array().unwrap() {
            admission(
                row[0].as_str().unwrap(),
                row[1].as_str().unwrap(),
                row[2].as_bool().unwrap(),
            );
        }
    }
}
#[test]
fn supports_fixture_distinguishes_declaration_admission_from_supports_wrapper() {
    let control = json(include_str!("fixtures/css_supports_chrome156.json"));
    assert_eq!(control["browser"], "Chrome/156.0.8078.80");
    let mut mismatches = Vec::new();
    for row in control["result"][0]["result"]["declarations"]
        .as_array()
        .unwrap()
    {
        let declaration = row[0].as_str().unwrap();
        let (name, value) = declaration.split_once(':').unwrap();
        let expected = row[3].as_bool().unwrap();
        let actual = full_admitted(name, value.trim());
        if actual != Ok(expected) {
            mismatches.push(format!(
                "{name}: {value} expected={expected} actual={actual:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} supports mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
#[test]
fn native_zoom_scroll_interest_and_deferred_raw_contracts_remain_explicit() {
    for (name, raw, expected) in [
        ("zoom", "+2", "2"),
        ("zoom", "NORMAL", "normal"),
        ("zoom", "calc(1 + 1)", "calc(2)"),
        ("scroll-snap-type", "Y Proximity", "y"),
        ("scroll-snap-align", "center center", "center"),
        ("scroll-behavior", "SMOOTH", "smooth"),
        ("interest-delay", "normal 20ms", "normal 20ms"),
        ("interest-delay-start", "1s", "1s"),
        ("width", "var(--x)", "var(--x)"),
        (
            "width",
            "env(safe-area-inset-top)",
            "env(safe-area-inset-top)",
        ),
        (
            "color",
            "if(style(--x: 1): red; else: blue)",
            "if(style(--x: 1): red; else: blue)",
        ),
        ("writing-mode", "VERTICAL-RL", "VERTICAL-RL"),
        ("font-variant", "small-caps", "small-caps"),
    ] {
        assert_eq!(
            declaration_value_checked(name, raw, limits())
                .unwrap()
                .as_deref(),
            Some(expected),
            "{name}: {raw}"
        );
    }
    for (name, raw) in [
        ("zoom", "-1"),
        ("zoom", "calc(1 / 0)"),
        ("scroll-snap-align", "left"),
        ("scroll-snap-type", "none mandatory"),
        ("interest-delay", "normal normal normal"),
        ("interest-delay-start", "1s 2s"),
        ("color", "if(red)"),
        ("color", "if(style(--x) and: red)"),
        ("width", "var()"),
        ("writing-mode", "left"),
    ] {
        admission(name, raw, false);
    }
}
#[test]
fn conditions_layers_and_checked_preflight_do_not_conceal_resource_errors() {
    for media in [
        "screen and (min-width: 1em)",
        "(width >= 100px)",
        "(future(foo))",
        "not (future(foo))",
    ] {
        assert!(parse_media_condition(media).is_ok(), "{media}");
    }
    for supports in [
        "(display: grid)",
        "not (future: value)",
        "(display: grid) or (future: value)",
    ] {
        assert!(parse_supports_condition(supports).is_ok(), "{supports}");
    }
    assert_eq!(
        parse_layer_names("base, theme.widgets").unwrap(),
        [
            vec!["base".to_string()],
            vec!["theme".to_string(), "widgets".to_string()]
        ]
    );
    for layer in ["a..b", ",a", "a,"] {
        assert!(parse_layer_names(layer).is_err(), "{layer}");
    }
    let nested = format!("{}1{}", "f(".repeat(502), ")".repeat(502));
    let mut policy = limits();
    policy.max_nesting = 32;
    assert_eq!(
        declaration_value_checked("width", &nested, policy),
        Err(ConditionError::Computation(ComputedCssError::NestingLimit))
    );
    policy = limits();
    policy.max_output_units = 2;
    assert_eq!(
        declaration_value_checked("color", "red", policy),
        Err(ConditionError::Computation(ComputedCssError::ResourceLimit))
    );
}

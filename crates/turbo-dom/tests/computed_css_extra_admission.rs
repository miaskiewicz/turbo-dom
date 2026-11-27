#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        grammar::{computed_declaration_value, DeclarationAdmission},
        ComputationLimits,
    },
    css_shorthand::{longhand_value, longhand_value_css, Spelled},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn actual_chrome159_extra_property_grammar_controls() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_shared_extra_chrome159.json")).unwrap();
    assert_eq!(control["version"], "159.0.8154.6");
    let mut failures = Vec::new();
    for row in control["results"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let value = row["value"].as_str().unwrap();
        let expected = row["supports"].as_bool().unwrap();
        let actual = computed_declaration_value(name, value, limits()).unwrap();
        if matches!(actual, DeclarationAdmission::Valid(_)) != expected
            || matches!(actual, DeclarationAdmission::RequiresGrammar(_))
        {
            failures.push(format!(
                "{name}: {value} expected={expected} actual={actual:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn css_whitespace_preserves_identifier_boundaries_and_legacy_contract() {
    for boundary in ['\u{a0}', '\u{85}', '\u{2003}'] {
        let color = format!("{boundary}red");
        assert!(matches!(longhand_value("color", &color), Spelled::Valid(_)));
        assert_eq!(longhand_value_css("color", &color), Spelled::Invalid);
        assert_eq!(
            computed_declaration_value("color", &color, limits()).unwrap(),
            DeclarationAdmission::Invalid
        );
        let animation = format!("{boundary}foo");
        assert_eq!(
            longhand_value_css("animation-name", &animation),
            Spelled::Valid(animation.clone())
        );
        assert_eq!(
            computed_declaration_value("animation-name", &animation, limits()).unwrap(),
            DeclarationAdmission::Valid(animation)
        );
    }
}

//! Keep extracted policy aligned with validated producer effect fix f7e74b275db7f23649db9dc01b7f2ab4a3dddeb1.
#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::grammar::{
    declaration_value_checked, is_supported_property, supports_declaration,
};
#[test]
fn canonical_admission_retains_chrome159_legacy_vendor_webkit_and_custom_controls() {
    let limits = turbo_dom::rtdom::computed_css::ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 8192,
    };
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/css_vendor_admission_chrome159.json")).unwrap();
    assert_eq!(fixture["version"], "159.0.8154.6");
    assert_eq!(
        fixture["probeSha256"],
        "0c36b0d2b24e9d9d38c6b79c26640dcb21366ff125fc8e20f3c3242a4cdfb368"
    );
    for row in fixture["output"].as_array().unwrap() {
        let name = row["property"].as_str().unwrap();
        let expected = row["supports"].as_bool().unwrap();
        assert_eq!(
            supports_declaration(name, "rotate(1deg)", limits).unwrap(),
            expected,
            "{name}"
        );
        if !name.starts_with("--") {
            assert_eq!(is_supported_property(name), expected, "{name}");
            assert_eq!(
                declaration_value_checked(name, "rotate(1deg)", limits)
                    .unwrap()
                    .is_some(),
                expected,
                "{name}"
            );
        }
    }
}

#![cfg(feature = "computed-style-grammar")]
use sha2::{Digest, Sha256};
use turbo_dom::rtdom::{
    computed_css::{
        grammar::{computed_declaration_value, DeclarationAdmission},
        property_registry, ComputationLimits,
    },
    css_shorthand::{longhand_value, Spelled},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn registry_is_the_original_measured_single_source() {
    let registry: serde_json::Value =
        serde_json::from_str(property_registry::REGISTRY_JSON).unwrap();
    assert_eq!(registry["source"]["browser"], "Chrome/156.0.8078.47");
    let names = registry["properties"].as_object().unwrap();
    assert_eq!(names.len(), 724);
    assert_eq!(property_registry::names().len(), names.len());
    assert_eq!(
        property_registry::SOURCE_SHA256,
        format!(
            "{:x}",
            Sha256::digest(property_registry::REGISTRY_JSON.as_bytes())
        )
    );
    for name in names.keys() {
        assert!(property_registry::is_known(name));
    }
    assert!(!property_registry::is_known("future-property"));
}
#[test]
fn full_admission_reuses_strict_longhand_not_legacy_name_subset() {
    for (name, raw) in [
        ("anchor-name", "--a"),
        ("-webkit-border-horizontal-spacing", "1px"),
        ("offset-path", "shape(from 0 0, line to 1px 1px)"),
    ] {
        assert_eq!(
            computed_declaration_value(name, raw, limits()).unwrap(),
            DeclarationAdmission::Valid(raw.into())
        );
    }
    for raw in ["--a", "--a, --b", "none", "--"] {
        assert!(
            matches!(longhand_value("anchor-name", raw), Spelled::Valid(_)),
            "{raw}"
        );
    }
    for raw in ["a", "--a --b", "--a,", "none,--a"] {
        assert_eq!(
            longhand_value("anchor-name", raw),
            Spelled::Invalid,
            "{raw}"
        );
        assert_eq!(
            computed_declaration_value("anchor-name", raw, limits()).unwrap(),
            DeclarationAdmission::Invalid
        );
    }
}
#[test]
fn registry_does_not_override_vendor_or_unknown_value_policy() {
    for name in [
        "-ms-transform",
        "-MS-transform",
        "-moz-transform",
        "-o-transform",
        "future-property",
        "--Custom",
    ] {
        assert_eq!(
            computed_declaration_value(name, "red", limits()).unwrap(),
            DeclarationAdmission::Invalid
        );
    }
}

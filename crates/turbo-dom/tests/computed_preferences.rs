use std::borrow::Cow;
use turbo_dom::rtdom::computed_preferences::{
    computed_font_preferences, default_computed_font_preferences, default_initial_color,
    FontDefaultsPlatform, FontPreferenceError, PREFERENCE_SOURCE_SHA256,
};

#[test]
fn generated_preferences_reuse_canonical_registry_and_explicit_platform() {
    use sha2::{Digest, Sha256};
    let raw = include_bytes!("../src/rtdom/computed_css/property_registry.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(raw)),
        PREFERENCE_SOURCE_SHA256
    );
    let registry: serde_json::Value = serde_json::from_slice(raw).unwrap();
    let mac = default_computed_font_preferences(FontDefaultsPlatform::MacOs);
    assert_eq!(
        mac.family.as_ref(),
        registry["properties"]["font-family"]["initial"][0]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        default_initial_color(),
        registry["properties"]["color"]["initial"][0]
            .as_str()
            .unwrap()
    );
    assert_eq!(mac.family, "Times");
    assert_eq!(mac.size_px, 16.0);
    assert!(matches!(mac.family, Cow::Borrowed(_)));
    let other = default_computed_font_preferences(FontDefaultsPlatform::Other);
    assert_eq!(other.family, "Times New Roman");
    assert_eq!(other.size_px, mac.size_px);
}

#[test]
fn raw_host_override_validation_preserves_native_contract_without_css_policy() {
    for family in ["", "  ", "\u{00a0}"] {
        assert_eq!(
            computed_font_preferences(Cow::Borrowed(family), 16.0),
            Err(FontPreferenceError::EmptyFamily)
        );
    }
    for size in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            computed_font_preferences(Cow::Borrowed("serif"), size),
            Err(FontPreferenceError::InvalidSize)
        );
    }
    for size in [f64::MIN_POSITIVE, 16.0, 1e300] {
        let value = computed_font_preferences(Cow::Borrowed("  serif  "), size).unwrap();
        assert_eq!(value.family, "  serif  ");
        assert_eq!(value.size_px, size);
        let owned = value.into_owned();
        assert!(matches!(owned.family, Cow::Owned(_)));
        assert_eq!(owned.family, "  serif  ");
    }
    let value = computed_font_preferences(Cow::Owned("var(--literal-host-name)".into()), 24.0)
        .unwrap()
        .into_owned();
    assert_eq!(value.family, "var(--literal-host-name)");
    assert_eq!(value.size_px, 24.0);
}

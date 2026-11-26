#![cfg(feature = "computed-style-grammar")]
use std::borrow::Cow;
use turbo_dom::rtdom::{
    computed_css::{
        preferences::{initial_preference_values, PreferenceBuildError},
        ComputationLimits, ComputedCssError,
    },
    computed_preferences::{
        computed_font_preferences, default_initial_color, ComputedFontPreferences,
        FontPreferenceError,
    },
};
fn limits(output: usize, total: usize) -> ComputationLimits {
    ComputationLimits {
        max_nesting: 0,
        max_output_units: output,
        max_total_units: total,
    }
}
#[test]
fn initial_projection_preserves_raw_data_and_charges_utf16_before_copy() {
    let fonts = computed_font_preferences(Cow::Borrowed("A💩"), 1e300).unwrap();
    let color = default_initial_color().encode_utf16().count();
    let value =
        initial_preference_values(&fonts, limits(color, color + 3 + 2 * "10000px".len())).unwrap();
    assert_eq!(
        value.font_family.as_units(),
        "A💩".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(value.font_size_px, 1e300);
    assert_eq!(value.computed_font_size_px, 10000.0);
    assert_eq!(
        value.computed_font_size.as_units(),
        "10000px".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        value.color.as_units(),
        default_initial_color().encode_utf16().collect::<Vec<_>>()
    );
    for budget in [
        limits(0, 0),
        limits(2, 100),
        limits(3, 100),
        limits(100, 2),
        limits(100, color + 2),
        limits(100, color + 3 + 2 * "10000px".len() - 1),
    ] {
        assert_eq!(
            initial_preference_values(&fonts, budget),
            Err(PreferenceBuildError::Computation(
                ComputedCssError::ResourceLimit
            ))
        );
    }
}
#[test]
fn bypassed_invalid_host_data_is_not_promoted_to_a_context() {
    let fonts = ComputedFontPreferences {
        family: Cow::Borrowed(""),
        size_px: 16.0,
    };
    assert_eq!(
        initial_preference_values(&fonts, limits(100, 100)),
        Err(PreferenceBuildError::Font(FontPreferenceError::EmptyFamily))
    );
    let fonts = ComputedFontPreferences {
        family: Cow::Borrowed("serif"),
        size_px: 0.0,
    };
    assert_eq!(
        initial_preference_values(&fonts, limits(100, 100)),
        Err(PreferenceBuildError::Font(FontPreferenceError::InvalidSize))
    );
}

#[test]
fn renderer_preferences_are_private_sparse_defaults_and_never_override_inheritance() {
    use std::collections::BTreeMap;
    use turbo_dom::rtdom::computed_css::{
        preferences::renderer_initial_preferences, properties::InitialContext, CssText,
    };
    let initial = InitialContext {
        font_family: "Times",
        color: "rgb(0, 0, 0)",
        font_size: "16px",
    };
    let mut values = BTreeMap::new();
    let limits = ComputationLimits {
        max_nesting: 0,
        max_output_units: 12,
        max_total_units: 33,
    };
    let defaults = renderer_initial_preferences(&values, &initial, limits).unwrap();
    assert_eq!(
        defaults.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        ["font-family", "color"]
    );
    assert!(values.is_empty());
    values.insert(
        "font-family".into(),
        CssText::from_units("monospace".encode_utf16().collect()),
    );
    let defaults = renderer_initial_preferences(&values, &initial, limits).unwrap();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].0, "color");
    let insufficient = ComputationLimits {
        max_total_units: 16,
        ..limits
    };
    assert!(renderer_initial_preferences(&values, &initial, insufficient).is_err());
    values.insert(
        "color".into(),
        CssText::from_units("red".encode_utf16().collect()),
    );
    assert!(renderer_initial_preferences(
        &values,
        &initial,
        ComputationLimits {
            max_output_units: 0,
            max_total_units: 0,
            ..limits
        }
    )
    .unwrap()
    .is_empty());
}

#[test]
fn internal_font_input_roundtrips_the_canonical_dimension_basis_without_cssom_rounding() {
    use turbo_dom::rtdom::css_shorthand::{computed_length, LengthContext};
    for raw in [16.123_456_789, 1e-300, 1e300] {
        let fonts = computed_font_preferences(Cow::Borrowed("A"), raw).unwrap();
        let projected = initial_preference_values(&fonts, limits(64, 256)).unwrap();
        let input = String::from_utf16(projected.font_size_input.as_units()).unwrap();
        let actual = computed_length(
            &input,
            &LengthContext { unit_bases: &[] },
            ComputationLimits {
                max_nesting: 32,
                max_output_units: 64,
                max_total_units: 256,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(actual, projected.computed_font_size_px);
        assert_eq!(projected.font_size_px, raw);
        let retained = projected.font_family.as_units().len()
            + projected.color.as_units().len()
            + projected.computed_font_size.as_units().len()
            + projected.font_size_input.as_units().len();
        assert_eq!(
            initial_preference_values(&fonts, limits(64, retained)).unwrap(),
            projected
        );
        assert_eq!(
            initial_preference_values(&fonts, limits(64, retained - 1)),
            Err(PreferenceBuildError::Computation(
                ComputedCssError::ResourceLimit
            ))
        );
    }
}

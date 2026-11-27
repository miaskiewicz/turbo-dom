#![cfg(feature = "computed-style-grammar")]
use std::{borrow::Cow, collections::BTreeMap};
use turbo_dom::rtdom::{
    computed_css::{
        conditions::{media_matches, ConditionEnvironment, ConditionEvaluationError, FeatureValue},
        environment::{
            build_environment_facts, ComputedEnvironmentFacts, EnvironmentError, ViewportAxis,
            ViewportFacts,
        },
        preferences::{initial_preference_values, InitialPreferenceValues},
        ComputationLimits, ComputedCssError, CssText,
    },
    computed_preferences::computed_font_preferences,
    css_shorthand::{computed_length, LengthContext, LengthError},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 128,
        max_total_units: 4096,
    }
}
fn initial(raw: f64) -> InitialPreferenceValues {
    initial_preference_values(
        &computed_font_preferences(Cow::Borrowed("Times"), raw).unwrap(),
        limits(),
    )
    .unwrap()
}
fn view(width: ViewportAxis, height: ViewportAxis) -> ViewportFacts {
    ViewportFacts { width, height }
}
fn bases(facts: &ComputedEnvironmentFacts, media: bool) -> Vec<(&str, f64)> {
    let source = if media {
        &facts.media_unit_bases
    } else {
        &facts.value_unit_bases
    };
    source
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect()
}
fn media(query: &str, facts: &ComputedEnvironmentFacts) -> Result<bool, ConditionEvaluationError> {
    let owned = bases(facts, true);
    let context = LengthContext { unit_bases: &owned };
    media_matches(
        &CssText::from_units(query.encode_utf16().collect()),
        &ConditionEnvironment {
            media_type: "screen",
            features: &facts.features,
            length_context: &context,
            selector_support: &BTreeMap::new(),
        },
        limits(),
    )
}
#[test]
fn known_viewport_uses_canonical_media_and_lengths_without_target_font_bases() {
    use ViewportAxis::Known;
    let facts =
        build_environment_facts(view(Known(1024.0), Known(768.0)), &initial(1e300), limits())
            .unwrap();
    assert_eq!(facts.features["width"], FeatureValue::LengthPx(1024.0));
    assert_eq!(facts.features["height"], FeatureValue::LengthPx(768.0));
    assert_eq!(
        facts.features["orientation"],
        FeatureValue::Ident("landscape".into())
    );
    assert_eq!(
        facts.features["aspect-ratio"],
        FeatureValue::Ratio(1024.0, 768.0)
    );
    assert!(media("(width:1024px) and (orientation:landscape)", &facts).unwrap());
    assert!(media("(width < 1em)", &facts).unwrap()); // normalized initial font=10000, not raw1e300
    let owned = bases(&facts, false);
    let context = LengthContext { unit_bases: &owned };
    assert_eq!(computed_length("50vw", &context, limits()), Ok(Some(512.0)));
    assert_eq!(computed_length("50vh", &context, limits()), Ok(Some(384.0)));
    assert_eq!(
        computed_length("100vmin", &context, limits()),
        Ok(Some(768.0))
    );
    assert_eq!(
        computed_length("100vmax", &context, limits()),
        Ok(Some(1024.0))
    );
    for unit in ["em", "rem", "vi", "vb", "ch", "ex"] {
        assert_eq!(
            computed_length(&format!("1{unit}"), &context, limits()),
            Err(LengthError::RequiresBasis(unit.into()))
        );
    }
}
#[test]
fn missing_axes_stay_unknown_and_existing_truth_evaluator_can_be_decisive() {
    use ViewportAxis::{Known, Unknown};
    let facts =
        build_environment_facts(view(Unknown, Known(768.0)), &initial(16.0), limits()).unwrap();
    assert!(!facts.features.contains_key("width"));
    assert!(!facts.features.contains_key("orientation"));
    assert!(!facts.features.contains_key("aspect-ratio"));
    assert_eq!(
        media("not (width:0px)", &facts),
        Err(ConditionEvaluationError::RequiresFeature("width".into()))
    );
    assert!(media("(height:768px) or (width:0px)", &facts).unwrap());
    let owned = bases(&facts, false);
    let context = LengthContext { unit_bases: &owned };
    assert_eq!(computed_length("50vh", &context, limits()), Ok(Some(384.0)));
    for unit in ["vw", "vmin", "vmax"] {
        assert_eq!(
            computed_length(&format!("1{unit}"), &context, limits()),
            Err(LengthError::RequiresBasis(unit.into()))
        );
    }
    let other =
        build_environment_facts(view(Known(0.0), Unknown), &initial(16.0), limits()).unwrap();
    assert_eq!(other.features["width"], FeatureValue::LengthPx(0.0));
    assert!(!other.features.contains_key("height"));
    let none = build_environment_facts(view(Unknown, Unknown), &initial(16.0), limits()).unwrap();
    assert!(none.features.is_empty());
    assert!(none.value_unit_bases.is_empty());
    assert_eq!(
        none.media_unit_bases,
        vec![("em".into(), 16.0), ("rem".into(), 16.0)]
    );
}
#[test]
fn zero_equal_and_huge_finite_axes_preserve_ratio_inputs_without_overflow_guesses() {
    use ViewportAxis::Known;
    for (width, height, orientation) in [
        (0.0, 0.0, "portrait"),
        (1.0, 1.0, "portrait"),
        (1.0, 0.0, "landscape"),
        (f64::MAX, 1.0, "landscape"),
    ] {
        let facts =
            build_environment_facts(view(Known(width), Known(height)), &initial(16.0), limits())
                .unwrap();
        assert_eq!(
            facts.features["aspect-ratio"],
            FeatureValue::Ratio(width, height)
        );
        assert_eq!(
            facts.features["orientation"],
            FeatureValue::Ident(orientation.into())
        );
        assert!(facts
            .value_unit_bases
            .iter()
            .all(|(_, value)| value.is_finite()));
    }
}
#[test]
fn external_public_struct_and_axes_are_validated_before_any_result_is_promoted() {
    use ViewportAxis::{Known, Unknown};
    for (viewport, name) in [
        (view(Known(f64::NAN), Unknown), "width"),
        (view(Unknown, Known(f64::INFINITY)), "height"),
        (view(Known(-1.0), Unknown), "width"),
    ] {
        assert_eq!(
            build_environment_facts(viewport, &initial(16.0), limits()),
            Err(EnvironmentError::InvalidViewportAxis(name))
        );
    }
    for basis in [f64::NAN, f64::INFINITY, -1.0, 10_001.0, 16.1] {
        let mut fabricated = initial(16.0);
        fabricated.computed_font_size_px = basis;
        assert_eq!(
            build_environment_facts(view(Unknown, Unknown), &fabricated, limits()),
            Err(EnvironmentError::InvalidInitialFontBasis)
        );
    }
}
fn retained_units(facts: &ComputedEnvironmentFacts) -> usize {
    facts
        .features
        .iter()
        .map(|(name, value)| {
            name.encode_utf16().count()
                + match value {
                    FeatureValue::Ident(value) => value.encode_utf16().count(),
                    _ => 0,
                }
        })
        .sum::<usize>()
        + facts
            .media_unit_bases
            .iter()
            .chain(&facts.value_unit_bases)
            .map(|(name, _)| name.encode_utf16().count())
            .sum::<usize>()
}
#[test]
fn exact_per_string_and_retained_quotas_cover_all_copied_keys_idents_and_units() {
    use ViewportAxis::Known;
    let viewport = view(Known(1024.0), Known(768.0));
    let initial = initial(16.0);
    let expected = build_environment_facts(viewport, &initial, limits()).unwrap();
    let total = retained_units(&expected);
    let exact = ComputationLimits {
        max_output_units: 12,
        max_total_units: total,
        ..limits()
    };
    assert_eq!(
        build_environment_facts(viewport, &initial, exact).unwrap(),
        expected
    );
    for limits in [
        ComputationLimits {
            max_output_units: 11,
            ..exact
        },
        ComputationLimits {
            max_total_units: total - 1,
            ..exact
        },
        ComputationLimits {
            max_total_units: 0,
            ..exact
        },
    ] {
        assert_eq!(
            build_environment_facts(viewport, &initial, limits),
            Err(EnvironmentError::Computation(
                ComputedCssError::ResourceLimit
            ))
        );
    }
}

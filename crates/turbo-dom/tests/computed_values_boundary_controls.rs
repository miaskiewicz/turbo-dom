#![cfg(feature = "computed-style-grammar")]
//! Behavioral boundaries between computed inheritance and host/layout used values.
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    computed_values::{compute_values, ComputedValues, ValueContext, ValueError, ValueLimits},
    properties::InitialContext,
    ComputationLimits, ComputedCssError, CssText,
};
use turbo_dom::rtdom::css_shorthand::{computed_length, LengthContext, LengthError};
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn chosen(values: &[(&str, &str)]) -> BTreeMap<String, Option<CssText>> {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), Some(text(value))))
        .collect()
}
fn context() -> ValueContext<'static> {
    ValueContext {
        initial: InitialContext {
            color: "red",
            font_family: "serif",
            font_size: "16px",
        },
        unit_bases: &[("rem", 16.0)],
        font_size_unit_bases: &[],
        is_root: false,
    }
}
fn limits() -> ValueLimits {
    ValueLimits {
        computation: ComputationLimits {
            max_nesting: 32,
            max_output_units: 4096,
            max_total_units: 4096,
        },
        retained_units: 8192,
    }
}
fn value(style: &ComputedValues, name: &str) -> String {
    String::from_utf16(style.values[name].as_units()).unwrap()
}
#[test]
fn computed_relative_lengths_inherit_after_font_resolution_but_percentage_waits_for_child_basis() {
    let parent = compute_values(
        None,
        &chosen(&[
            ("font-size", "10px"),
            ("width", "20em"),
            ("height", "50%"),
            ("line-height", "1.5"),
        ]),
        &context(),
        limits(),
    )
    .unwrap();
    let child = compute_values(
        Some(&parent),
        &chosen(&[
            ("font-size", "20px"),
            ("width", "inherit"),
            ("height", "inherit"),
        ]),
        &context(),
        limits(),
    )
    .unwrap();
    assert_eq!(value(&child, "width"), "200px");
    assert_eq!(value(&child, "height"), "50%");
    assert_eq!(value(&child, "line-height"), "1.5");
    let used = |basis| {
        computed_length(
            &value(&child, "height"),
            &LengthContext {
                unit_bases: &[("%", basis)],
            },
            limits().computation,
        )
        .unwrap()
        .unwrap()
    };
    assert_eq!((used(8.0), used(4.0)), (400.0, 200.0));
    assert_eq!(parent.font_size_px * 1.5, 15.0);
    assert_eq!(child.font_size_px * 1.5, 30.0);
}
#[test]
fn explicit_font_inheritance_materializes_known_parent_number_and_initial_has_no_parent_dependency()
{
    let parent = ComputedValues {
        values: BTreeMap::new(),
        font_size_px: 23.0,
        root_font_size_px: 16.0,
    };
    for keyword in ["inherit", "unset"] {
        let child = compute_values(
            Some(&parent),
            &chosen(&[("font-size", keyword)]),
            &context(),
            limits(),
        )
        .unwrap();
        assert_eq!(value(&child, "font-size"), "23px");
    }
    for keyword in ["initial", "inherit", "unset", "medium"] {
        let child = compute_values(
            None,
            &chosen(&[("font-size", keyword)]),
            &context(),
            limits(),
        )
        .unwrap();
        assert_eq!(value(&child, "font-size"), "16px");
    }
    let mut root = context();
    root.is_root = true;
    let style = compute_values(None, &chosen(&[("font-size", "2rem")]), &root, limits()).unwrap();
    assert_eq!((style.font_size_px, style.root_font_size_px), (32.0, 32.0));
}
#[test]
fn currentcolor_without_parent_uses_actual_initial_preference_and_rejects_recursive_initial() {
    let style = compute_values(
        None,
        &chosen(&[
            ("color", "currentcolor"),
            ("border-top-color", "currentcolor"),
            ("display", "block"),
            ("line-height", "normal"),
        ]),
        &context(),
        limits(),
    )
    .unwrap();
    assert_eq!(value(&style, "color"), "rgb(255, 0, 0)");
    assert_eq!(value(&style, "border-top-color"), "currentcolor");
    assert_eq!(value(&style, "display"), "block");
    assert_eq!(value(&style, "line-height"), "normal");
    let mut recursive = context();
    recursive.initial.color = "currentcolor";
    assert_eq!(
        compute_values(
            None,
            &chosen(&[("color", "currentcolor")]),
            &recursive,
            limits()
        ),
        Err(ValueError::RequiresInitial("color".into()))
    );
}
#[test]
fn invalid_ordinary_values_and_missing_actual_metrics_remain_distinct() {
    for (name, source) in [
        ("line-height", "-1"),
        ("line-height", "not-a-height"),
        ("border-top-style", "not-a-style"),
        ("color", "not-a-color"),
    ] {
        assert_eq!(
            compute_values(None, &chosen(&[(name, source)]), &context(), limits()),
            Err(ValueError::UnsupportedValue(name.into())),
            "{name}:{source}"
        );
    }
    assert_eq!(
        compute_values(None, &chosen(&[("width", "2ch")]), &context(), limits()),
        Err(ValueError::Length(LengthError::RequiresBasis("ch".into())))
    );
    assert_eq!(
        ValueError::from(LengthError::Computation(ComputedCssError::NestingLimit)),
        ValueError::Computation(ComputedCssError::NestingLimit)
    );
    assert_eq!(
        ValueError::from(LengthError::Computation(ComputedCssError::ResourceLimit)),
        ValueError::Computation(ComputedCssError::ResourceLimit)
    );
}

#[cfg(feature = "computed-style-grammar")]
#[test]
fn selected_admission_rejects_invalid_display_before_value_computation_against_chrome159() {
    use sha2::{Digest, Sha256};
    use turbo_dom::rtdom::computed_css::{
        candidate_set::CandidateSet,
        custom_cascade::CustomCascadeLimits,
        inline_style::{InlineLimits, InlineStyle},
        selection::select_style,
    };
    let probe = include_bytes!("fixtures/computed-display-admission-159/invalid-display-probe.mjs");
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/computed-display-admission-159/INVALID-DISPLAY-ORACLE.json"
    ))
    .unwrap();
    assert_eq!(oracle["browser"], "159.0.8154.6");
    assert_eq!(
        oracle["sourceSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    assert_eq!(oracle["result"]["supports"], false);
    assert_eq!(
        oracle["result"]["invalidParsed"]["declared"],
        "inline-block"
    );
    assert_eq!(
        oracle["result"]["invalidSubstituted"]["declared"],
        "var(--bad)"
    );
    // Literal invalid declarations are ignored. Invalid var winners become
    // initial and must never expose the lower inline-block candidate.
    for (case, expected) in [
        ("invalidParsed", "inline-block"),
        ("invalidSubstituted", "inline"),
    ] {
        let control = &oracle["result"][case];
        assert_eq!(control["computed"], expected);
        let source = control["css"]
            .as_str()
            .unwrap()
            .strip_prefix(".x{")
            .unwrap()
            .strip_suffix('}')
            .unwrap();
        let expected = control["computed"].as_str().unwrap();
        let inline = InlineStyle::compile(
            &text(source),
            InlineLimits {
                computation: limits().computation,
                declarations: 8,
                properties: 8,
            },
        )
        .unwrap();
        let mut candidates = CandidateSet {
            ordinary: BTreeMap::new(),
            custom: BTreeMap::new(),
        };
        inline.append(&mut candidates, 0, 0);
        let selected = select_style(
            &BTreeMap::new(),
            &candidates.custom,
            &candidates.ordinary,
            CustomCascadeLimits {
                computation: limits().computation,
                rollbacks: 8,
            },
        )
        .unwrap();
        let computed = compute_values(None, &selected.ordinary, &context(), limits()).unwrap();
        assert_eq!(value(&computed, "display"), expected, "{source}");
    }
}

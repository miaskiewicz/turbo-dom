//! Computed inheritance retains percentages/currentcolor/numbers for later used values.
#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    computed_values::{compute_values, ComputedValues, ValueContext, ValueError, ValueLimits},
    properties::InitialContext,
    ComputationLimits, ComputedCssError, CssText,
};
use turbo_dom::rtdom::css_shorthand::LengthError;

fn text(source: &str) -> CssText {
    CssText::from_units(source.encode_utf16().collect())
}
fn selected(entries: &[(&str, &str)]) -> BTreeMap<String, Option<CssText>> {
    entries
        .iter()
        .map(|(name, value)| ((*name).into(), Some(text(value))))
        .collect()
}
fn limits() -> ValueLimits {
    ValueLimits {
        computation: ComputationLimits {
            max_nesting: 32,
            max_output_units: 4096,
            max_total_units: 0,
        },
        retained_units: 16384,
    }
}
fn context<'a>(
    is_root: bool,
    initial_font: &'a str,
    bases: &'a [(&'a str, f64)],
    font_bases: &'a [(&'a str, f64)],
) -> ValueContext<'a> {
    ValueContext {
        initial: InitialContext {
            color: "rgb(0, 0, 0)",
            font_family: "serif",
            font_size: initial_font,
        },
        unit_bases: bases,
        font_size_unit_bases: font_bases,
        is_root,
    }
}
fn get(values: &ComputedValues, name: &str) -> String {
    String::from_utf16(values.values[name].as_units()).unwrap()
}
fn root() -> ComputedValues {
    compute_values(
        None,
        &selected(&[]),
        &context(true, "16px", &[], &[]),
        limits(),
    )
    .unwrap()
}
fn compute(parent: &ComputedValues, entries: &[(&str, &str)]) -> ComputedValues {
    compute_values(
        Some(parent),
        &selected(entries),
        &context(false, "16px", &[], &[]),
        limits(),
    )
    .unwrap()
}

#[test]
fn no_default_style_is_materialized_and_actual_initial_font_has_no_fixed_fallback() {
    let style = compute_values(
        None,
        &selected(&[]),
        &context(true, "19px", &[], &[]),
        limits(),
    )
    .unwrap();
    assert!(style.values.is_empty());
    assert_eq!(style.font_size_px, 19.0);
    assert_eq!(style.root_font_size_px, 19.0);
    let child = compute_values(
        Some(&style),
        &selected(&[]),
        &context(false, "19px", &[], &[]),
        limits(),
    )
    .unwrap();
    assert!(child.values.is_empty());
    assert_eq!(child.font_size_px, 19.0);
    assert_eq!(
        compute_values(
            None,
            &selected(&[]),
            &context(true, "medium", &[], &[]),
            limits()
        ),
        Err(ValueError::RequiresInitial("font-size".into()))
    );
}

#[test]
fn declared_em_lengths_compute_once_before_inheritance() {
    let root = root();
    let parent = compute(
        &root,
        &[
            ("font-size", "10px"),
            ("width", "20em"),
            ("margin-left", "2em"),
        ],
    );
    let child = compute(
        &parent,
        &[
            ("font-size", "20px"),
            ("width", "inherit"),
            ("margin-left", "inherit"),
        ],
    );
    assert_eq!(get(&parent, "width"), "200px");
    assert_eq!(get(&child, "width"), "200px");
    assert_eq!(get(&child, "margin-left"), "20px");
    assert_eq!(child.font_size_px, 20.0);
    assert_eq!(child.root_font_size_px, 16.0);
}

#[test]
fn percentages_and_mixed_math_remain_computed_expressions_across_inherit() {
    let root = root();
    for source in ["50%", "calc(50% + 1em)"] {
        let parent = compute(
            &root,
            &[
                ("font-size", "10px"),
                ("width", source),
                ("margin-left", "50%"),
            ],
        );
        let child = compute(
            &parent,
            &[
                ("font-size", "20px"),
                ("width", "inherit"),
                ("margin-left", "inherit"),
            ],
        );
        assert_eq!(get(&child, "width"), get(&parent, "width"));
        assert_eq!(get(&child, "margin-left"), "50%");
        assert!(get(&child, "width").contains("50%"));
        if source.starts_with("calc") {
            assert!(get(&child, "width").contains("10px"));
            assert!(!get(&child, "width").contains("20px"));
        }
    }
}

#[test]
fn line_height_numbers_inherit_as_numbers_but_percentages_and_lengths_are_absolute() {
    let root = root();
    for (source, expected) in [("1.5", "1.5"), ("2em", "20px"), ("150%", "15px")] {
        let parent = compute(&root, &[("font-size", "10px"), ("line-height", source)]);
        let child = compute(
            &parent,
            &[("font-size", "20px"), ("line-height", "inherit")],
        );
        assert_eq!(get(&parent, "line-height"), expected);
        assert_eq!(get(&child, "line-height"), expected);
    }
}

#[test]
fn other_currentcolor_tokens_survive_inherit_while_color_itself_uses_parent_color() {
    let root = root();
    let parent = compute(
        &root,
        &[
            ("color", "red"),
            ("border-left-color", "currentcolor"),
            ("background-color", "currentcolor"),
        ],
    );
    let child = compute(
        &parent,
        &[
            ("color", "blue"),
            ("border-left-color", "inherit"),
            ("background-color", "inherit"),
        ],
    );
    assert_eq!(get(&parent, "color"), "rgb(255, 0, 0)");
    assert_eq!(get(&child, "color"), "rgb(0, 0, 255)");
    assert_eq!(get(&child, "border-left-color"), "currentcolor");
    assert_eq!(get(&child, "background-color"), "currentcolor");
    let same = compute(&parent, &[("color", "currentcolor")]);
    assert_eq!(get(&same, "color"), get(&parent, "color"));
}

#[test]
fn root_font_size_rem_uses_initial_but_other_rem_uses_computed_root() {
    let bases = [("em", 999.0), ("rem", 999.0), ("%", 999.0)];
    let root = compute_values(
        None,
        &selected(&[("font-size", "2rem"), ("width", "1rem")]),
        &context(true, "20px", &bases, &bases),
        limits(),
    )
    .unwrap();
    assert_eq!(root.font_size_px, 40.0);
    assert_eq!(root.root_font_size_px, 40.0);
    assert_eq!(get(&root, "width"), "40px");
    let child = compute_values(
        Some(&root),
        &selected(&[
            ("font-size", "50%"),
            ("width", "1rem"),
            ("margin-left", "1em"),
        ]),
        &context(false, "20px", &bases, &bases),
        limits(),
    )
    .unwrap();
    assert_eq!(child.font_size_px, 20.0);
    assert_eq!(get(&child, "width"), "40px");
    assert_eq!(get(&child, "margin-left"), "20px");
}

#[test]
fn font_size_and_other_font_metric_units_use_distinct_actual_contexts() {
    let root = root();
    let parent = compute(&root, &[("font-size", "20px")]);
    let child = compute_values(
        Some(&parent),
        &selected(&[("font-size", "1ex"), ("width", "1ex")]),
        &context(false, "16px", &[("ex", 3.0)], &[("ex", 8.0)]),
        limits(),
    )
    .unwrap();
    assert_eq!(child.font_size_px, 8.0);
    assert_eq!(get(&child, "width"), "3px");
    assert_eq!(
        compute_values(
            Some(&parent),
            &selected(&[("width", "1ex")]),
            &context(false, "16px", &[], &[]),
            limits()
        ),
        Err(ValueError::Length(LengthError::RequiresBasis("ex".into())))
    );
}

#[test]
fn implicit_inheritance_is_limited_to_known_parent_keys_and_wide_values_share_metadata() {
    let root = root();
    let parent = compute(
        &root,
        &[
            ("color", "red"),
            ("opacity", "0.5"),
            ("margin-left", "5px"),
            ("future-property", "raw"),
        ],
    );
    let child = compute(&parent, &[]);
    assert_eq!(get(&child, "color"), get(&parent, "color"));
    assert!(!child.values.contains_key("opacity"));
    assert!(!child.values.contains_key("margin-left"));
    assert!(!child.values.contains_key("future-property"));
    let mut entries = selected(&[
        ("color", "unset"),
        ("margin-left", "unset"),
        ("word-spacing", "initial"),
    ]);
    entries.insert("width".into(), None);
    let reset = compute_values(
        Some(&parent),
        &entries,
        &context(false, "16px", &[], &[]),
        limits(),
    )
    .unwrap();
    assert_eq!(get(&reset, "color"), get(&parent, "color"));
    assert_eq!(get(&reset, "margin-left"), "0px");
    assert_eq!(get(&reset, "word-spacing"), "0px");
    assert_eq!(get(&reset, "width"), "auto");
    assert_eq!(
        compute_values(
            Some(&parent),
            &selected(&[("future-property", "initial")]),
            &context(false, "16px", &[], &[]),
            limits()
        ),
        Err(ValueError::RequiresInitial("future-property".into()))
    );
}

#[test]
fn rollback_and_unavailable_math_are_not_silently_invalidated_or_reselected() {
    let root = root();
    for source in ["revert", "revert-layer"] {
        assert_eq!(
            compute_values(
                Some(&root),
                &selected(&[("width", source)]),
                &context(false, "16px", &[], &[]),
                limits()
            ),
            Err(ValueError::Computation(ComputedCssError::RequiresCascade))
        );
    }
    assert!(matches!(
        compute_values(
            Some(&root),
            &selected(&[("width", "calc(sibling-index() * 1px)")]),
            &context(false, "16px", &[], &[]),
            limits()
        ),
        Err(ValueError::Length(LengthError::UnsupportedMath(_)))
    ));
}

#[test]
fn raw_context_free_dom_strings_are_not_repaired_through_scalar_transport() {
    let root = root();
    let mut entries = selected(&[]);
    let value = CssText::from_units(vec![34, 0xd800, 34]);
    entries.insert("content".into(), Some(value.clone()));
    let computed = compute_values(
        Some(&root),
        &entries,
        &context(false, "16px", &[], &[]),
        limits(),
    )
    .unwrap();
    assert_eq!(computed.values["content"], value);
}

#[test]
fn inherited_payloads_obey_per_output_and_cumulative_quotas_before_copy() {
    let mut units = vec![97; 5000];
    units[0] = 0xd800;
    let parent = ComputedValues {
        values: BTreeMap::from([("font-family".into(), CssText::from_units(units.clone()))]),
        font_size_px: 16.0,
        root_font_size_px: 16.0,
    };
    let mut policy = limits();
    policy.computation.max_output_units = 6000;
    policy.retained_units = 4999;
    assert_eq!(
        compute_values(
            Some(&parent),
            &selected(&[]),
            &context(false, "16px", &[], &[]),
            policy
        ),
        Err(ValueError::Computation(ComputedCssError::ResourceLimit))
    );
    policy.retained_units = 5000;
    let copied = compute_values(
        Some(&parent),
        &selected(&[]),
        &context(false, "16px", &[], &[]),
        policy,
    )
    .unwrap();
    assert_eq!(copied.values["font-family"].as_units(), units);
    policy.computation.max_output_units = 4999;
    assert_eq!(
        compute_values(
            Some(&parent),
            &selected(&[]),
            &context(false, "16px", &[], &[]),
            policy
        ),
        Err(ValueError::Computation(ComputedCssError::ResourceLimit))
    );
}

#[test]
fn nonroot_without_parent_requires_actual_root_basis_and_metadata_is_validated() {
    assert_eq!(
        compute_values(
            None,
            &selected(&[]),
            &context(false, "16px", &[], &[]),
            limits()
        ),
        Err(ValueError::Length(LengthError::RequiresBasis("rem".into())))
    );
    let style = compute_values(
        None,
        &selected(&[("width", "1rem")]),
        &context(false, "16px", &[("rem", 23.0)], &[]),
        limits(),
    )
    .unwrap();
    assert_eq!(style.root_font_size_px, 23.0);
    assert_eq!(get(&style, "width"), "23px");
    let parent = ComputedValues {
        values: BTreeMap::new(),
        font_size_px: f64::NAN,
        root_font_size_px: 16.0,
    };
    assert_eq!(
        compute_values(
            Some(&parent),
            &selected(&[]),
            &context(false, "16px", &[], &[]),
            limits()
        ),
        Err(ValueError::InvalidFontContext("parent font-size".into()))
    );
}

#[test]
fn generated_content_uses_shared_cssom_string_serialization() {
    let result = compute_values(
        None,
        &selected(&[("content", "'x'")]),
        &context(false, "16px", &[("rem", 16.0)], &[]),
        limits(),
    )
    .unwrap();
    assert_eq!(get(&result, "content"), "\"x\"");
}

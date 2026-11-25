//! Public font error adapters preserve resource identity and sparse inheritance.
#![cfg(feature = "computed-style-grammar")]
use std::collections::BTreeMap;
use turbo_dom::rtdom::computed_css::{
    computed_values::{compute_values, ValueContext, ValueError, ValueLimits},
    font_values::FontValueError,
    grammar::GrammarLimitError,
    properties::InitialContext,
    ComputationLimits, ComputedCssError, CssText,
};

#[test]
fn font_error_conversion_preserves_resource_identity_and_property_errors() {
    for error in [
        ComputedCssError::ResourceLimit,
        ComputedCssError::NestingLimit,
    ] {
        assert_eq!(
            ValueError::from(FontValueError::Computation(error)),
            ValueError::Computation(error)
        );
    }
    for (error, property) in [
        (FontValueError::InvalidLineHeight, "line-height"),
        (FontValueError::InvalidFontWeight, "font-weight"),
    ] {
        assert_eq!(
            ValueError::from(error),
            ValueError::UnsupportedValue(property.into())
        );
    }
    assert_eq!(
        ValueError::from(FontValueError::RequiresParentFontWeight),
        ValueError::Font(FontValueError::RequiresParentFontWeight)
    );
}

#[test]
fn backend_nesting_conversion_stays_typed_without_recursive_parser_execution() {
    assert_eq!(
        FontValueError::from(GrammarLimitError::NestingLimit),
        FontValueError::Computation(ComputedCssError::NestingLimit)
    );
    assert_eq!(
        FontValueError::from(GrammarLimitError::Computation(
            ComputedCssError::ResourceLimit
        )),
        FontValueError::Computation(ComputedCssError::ResourceLimit)
    );
}

#[test]
fn relative_weight_uses_canonical_initial_when_parent_sparse_key_is_missing() {
    let context = ValueContext {
        initial: InitialContext {
            color: "rgb(0, 0, 0)",
            font_family: "serif",
            font_size: "16px",
        },
        unit_bases: &[("rem", 16.0)],
        font_size_unit_bases: &[("rem", 16.0)],
        is_root: false,
    };
    let limits = ValueLimits {
        computation: ComputationLimits {
            max_nesting: 32,
            max_output_units: 4096,
            max_total_units: 8192,
        },
        retained_units: 8192,
    };
    let parent = compute_values(None, &BTreeMap::new(), &context, limits).unwrap();
    assert!(!parent.values.contains_key("font-weight"));
    for (source, expected) in [("bolder", "700"), ("lighter", "100")] {
        let selected = BTreeMap::from([(
            "font-weight".into(),
            Some(CssText::from_units(source.encode_utf16().collect())),
        )]);
        let child = compute_values(Some(&parent), &selected, &context, limits).unwrap();
        assert_eq!(
            String::from_utf16(child.values["font-weight"].as_units()).unwrap(),
            expected
        );
        assert!(!parent.values.contains_key("font-weight"));
        // The same CSS initial supplies the root/no-parent relative-weight basis.
        let standalone = compute_values(None, &selected, &context, limits).unwrap();
        assert_eq!(
            standalone.values["font-weight"],
            child.values["font-weight"]
        );
    }
}

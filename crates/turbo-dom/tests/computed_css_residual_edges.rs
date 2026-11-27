#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        grammar::{declaration_value, supports_condition_declaration},
        ComputationLimits, ComputedCssError,
    },
    css_shorthand::{computed_length, computed_length_expression, LengthContext, LengthError},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn native_tree_counting_does_not_reinterpret_comment_payload() {
    for value in [
        "calc(sibling-index() * 1px) /* sibling-other() */",
        "calc(sibling-count() * 1px) /* sibling-index(1) */",
    ] {
        assert_eq!(
            declaration_value("width", value).as_deref(),
            Some(value),
            "{value}"
        );
    }
}
#[test]
fn computed_math_gaps_and_deferred_values_remain_explicit() {
    let context = LengthContext { unit_bases: &[] };
    assert!(matches!(
        computed_length_expression("round(up,1px,2px)", &context, limits()),
        Err(LengthError::UnsupportedMath(_))
    ));
    assert!(matches!(
        computed_length("calc-size(auto, size + 1px)", &context, limits()),
        Err(LengthError::UnsupportedMath(_))
    ));
    for value in ["env(safe-area-inset-top)", "if(else:2px)"] {
        assert_eq!(
            computed_length(value, &context, limits()),
            Err(LengthError::Computation(ComputedCssError::RequiresCascade)),
            "{value}"
        );
    }
}
#[test]
fn supports_raw_property_names_require_one_complete_identifier() {
    assert_eq!(
        supports_condition_declaration("--x y", "red", limits()),
        Ok(false)
    );
    assert_eq!(
        supports_condition_declaration("--x:bad", "red", limits()),
        Ok(false)
    );
}

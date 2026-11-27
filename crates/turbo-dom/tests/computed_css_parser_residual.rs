#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        conditions::ConditionEvaluationError,
        container::{
            container_matches, ContainerAncestor, ContainerError, ContentSize, InlineAxis, SizeKind,
        },
        grammar::declaration_value,
        ComputationLimits, CssText,
    },
    css_shorthand::{LengthContext, LengthError},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 32,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
#[test]
fn tree_counting_transform_validation_keeps_comment_text_inert() {
    for value in [
        "translateX(calc(sibling-index() * 1px)) /* sibling-other() */",
        "translateX(calc(sibling-count() * 1px)) /* sibling-index(1) */",
    ] {
        assert_eq!(
            declaration_value("transform", value).as_deref(),
            Some(value),
            "{value}"
        );
    }
}
#[test]
fn invalid_font_provider_data_cannot_be_discharged_as_missing_container_capability() {
    let lengths = LengthContext {
        unit_bases: &[("em", f64::NAN)],
    };
    let ancestors = [ContainerAncestor {
        id: 42,
        names: &[],
        kind: SizeKind::Size,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: Some(ContentSize {
            width_px: 400.0,
            height_px: 100.0,
        }),
        length_context: &lengths,
    }];
    for source in ["(width:1em)", "(width:1em) or (width:400px)"] {
        let text = CssText::from_units(source.encode_utf16().collect());
        assert_eq!(
            container_matches(&text, &ancestors, limits()),
            Err(ContainerError::Condition(ConditionEvaluationError::Length(
                LengthError::InvalidBasis("em".into())
            ))),
            "{source}"
        );
    }
}

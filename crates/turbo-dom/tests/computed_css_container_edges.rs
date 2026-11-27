#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        conditions::ConditionEvaluationError,
        container::{
            container_matches, ContainerAncestor, ContainerError, ContentSize, InlineAxis, SizeKind,
        },
        ComputationLimits, CssText,
    },
    css_shorthand::{LengthContext, LengthError},
};
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn container<'a>(context: &'a LengthContext<'a>) -> ContainerAncestor<'a> {
    ContainerAncestor {
        id: 1,
        names: &[],
        kind: SizeKind::Size,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: Some(ContentSize {
            width_px: 200.0,
            height_px: 100.0,
        }),
        length_context: context,
    }
}
#[test]
fn orientation_logical_axes_and_bounded_ranges_use_actual_content_size() {
    let context = LengthContext { unit_bases: &[] };
    let mut ancestor = container(&context);
    for query in [
        "(orientation:landscape)",
        "(inline-size:200px)",
        "(100px < width <= 200px)",
        "(200px >= width > 100px)",
    ] {
        assert!(
            container_matches(&text(query), &[ancestor], limits())
                .unwrap()
                .matches,
            "{query}"
        );
    }
    assert!(
        !container_matches(&text("(300px < width < 400px)"), &[ancestor], limits())
            .unwrap()
            .matches
    );
    ancestor.inline_axis = InlineAxis::Vertical;
    assert!(
        container_matches(&text("(block-size:200px)"), &[ancestor], limits())
            .unwrap()
            .matches
    );
    ancestor.size = Some(ContentSize {
        width_px: 100.0,
        height_px: 200.0,
    });
    assert!(
        container_matches(&text("(orientation:portrait)"), &[ancestor], limits())
            .unwrap()
            .matches
    );
}
#[test]
fn unknown_container_features_are_not_assigned_a_size_basis() {
    let context = LengthContext { unit_bases: &[] };
    let mut ancestor = container(&context);
    ancestor.kind = SizeKind::Normal;
    ancestor.size = None;
    for query in ["(future-feature)", "(future-feature:1)"] {
        assert!(
            !container_matches(&text(query), &[ancestor], limits())
                .unwrap()
                .matches
        );
    }
    assert!(
        !container_matches(
            &text("(future-feature) and (width:1px)"),
            &[ancestor],
            limits()
        )
        .unwrap()
        .matches
    );
}
#[test]
fn missing_font_basis_is_typed_and_boolean_decision_can_discharge_it() {
    let context = LengthContext { unit_bases: &[] };
    let ancestor = container(&context);
    assert_eq!(
        container_matches(&text("(width:1em)"), &[ancestor], limits()),
        Err(ContainerError::Condition(ConditionEvaluationError::Length(
            LengthError::RequiresBasis("em".into())
        )))
    );
    assert!(
        container_matches(
            &text("(width:1em) or (height:100px)"),
            &[ancestor],
            limits()
        )
        .unwrap()
        .matches
    );
}

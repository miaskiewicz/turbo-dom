//! Size facts are actual container content boxes, never authored target widths.
#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::{
    computed_css::{
        container::{
            container_matches, container_names, ContainerAncestor, ContainerError, ContentSize,
            InlineAxis, SizeKind,
        },
        ComputationLimits, ComputedCssError, CssText,
    },
    css_shorthand::LengthContext,
};
fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}
fn limits() -> ComputationLimits {
    ComputationLimits {
        max_nesting: 64,
        max_output_units: 4096,
        max_total_units: 0,
    }
}
fn ancestor<'a>(
    id: u64,
    kind: SizeKind,
    width: f64,
    height: f64,
    context: &'a LengthContext<'a>,
) -> ContainerAncestor<'a> {
    ContainerAncestor {
        id,
        names: &[],
        kind,
        inline_axis: InlineAxis::Horizontal,
        scroll_state: false,
        size: Some(ContentSize {
            width_px: width,
            height_px: height,
        }),
        length_context: context,
    }
}
fn result(query: &str, ancestors: &[ContainerAncestor<'_>]) -> bool {
    container_matches(&text(query), ancestors, limits())
        .unwrap()
        .matches
}
#[test]
fn nearest_eligible_container_and_all_boolean_branch_axes() {
    let context = LengthContext { unit_bases: &[] };
    let inner = ancestor(2, SizeKind::InlineSize, 200.0, 100.0, &context);
    let outer = ancestor(1, SizeKind::Size, 50.0, 50.0, &context);
    let ancestors = [inner, outer];
    assert!(result("(width > 100px)", &ancestors));
    assert!(!result("(width > 100px) or (height > 300px)", &ancestors));
    assert!(result(
        "not ((width > 100px) or (height > 300px))",
        &ancestors
    ));
    assert_eq!(
        container_matches(&text("(height > 10px)"), &ancestors, limits())
            .unwrap()
            .container,
        Some(1)
    );
}
#[test]
fn named_containers_and_name_only_preserve_case() {
    let context = LengthContext { unit_bases: &[] };
    let names = vec!["box".into()];
    let mut inner = ancestor(2, SizeKind::InlineSize, 200.0, 100.0, &context);
    let mut outer = ancestor(1, SizeKind::Size, 400.0, 400.0, &context);
    outer.names = &names;
    assert!(result("box (width > 300px)", &[inner, outer]));
    inner = ancestor(2, SizeKind::Normal, 0.0, 0.0, &context);
    inner.names = &names;
    inner.size = None;
    assert!(result("box", &[inner]));
    assert!(!result("Box", &[inner]));
    assert_eq!(
        container_names(&text("b\\6f x second"), limits()).unwrap(),
        ["box", "second"]
    );
    assert!(container_names(&text("none"), limits()).unwrap().is_empty());
    for source in ["", "none other", "initial"] {
        assert_eq!(
            container_names(&text(source), limits()),
            Err(ContainerError::InvalidSyntax)
        );
    }
}
#[test]
fn vertical_inline_axis_does_not_claim_physical_width() {
    let context = LengthContext { unit_bases: &[] };
    let mut inner = ancestor(2, SizeKind::InlineSize, 200.0, 100.0, &context);
    inner.inline_axis = InlineAxis::Vertical;
    let outer = ancestor(1, SizeKind::Size, 400.0, 400.0, &context);
    let ancestors = [inner, outer];
    assert!(result("(width > 300px)", &ancestors));
    assert!(!result("(height > 300px)", &ancestors));
    assert!(!result("(inline-size > 150px)", &ancestors));
    assert!(result("(block-size > 300px)", &ancestors));
}
#[test]
fn selected_cold_container_does_not_fall_through() {
    let context = LengthContext { unit_bases: &[] };
    let mut inner = ancestor(2, SizeKind::InlineSize, 200.0, 100.0, &context);
    inner.size = None;
    let outer = ancestor(1, SizeKind::Size, 400.0, 400.0, &context);
    assert_eq!(
        container_matches(&text("(width > 300px)"), &[inner, outer], limits()),
        Err(ContainerError::RequiresMeasurement(2))
    );
    let normal = ancestor(3, SizeKind::Normal, 400.0, 400.0, &context);
    assert!(!result("not (width > 300px)", &[normal]));
}
#[test]
fn container_font_and_actual_content_box_are_used() {
    let context = LengthContext {
        unit_bases: &[("em", 10.0), ("rem", 16.0)],
    };
    let outer = ancestor(1, SizeKind::Size, 400.0, 100.0, &context);
    assert!(result("(width > 30em)", &[outer]));
    assert!(!result("(width > 30rem)", &[outer]));
    let border_box = ancestor(1, SizeKind::Size, 174.0, 100.0, &context);
    assert!(result("(width:174px)", &[border_box]));
}
#[test]
fn zero_size_ratios_follow_comparison_cross_products() {
    let context = LengthContext { unit_bases: &[] };
    let zero = ancestor(1, SizeKind::Size, 0.0, 0.0, &context);
    for query in [
        "(aspect-ratio:1/1)",
        "(aspect-ratio:0/1)",
        "(aspect-ratio:16/0)",
        "not (aspect-ratio > 1/1)",
    ] {
        assert!(result(query, &[zero]));
    }
    assert!(!result("(aspect-ratio > 1/1)", &[zero]));
    let flat = ancestor(2, SizeKind::Size, 100.0, 0.0, &context);
    assert!(result("(aspect-ratio > 1/1)", &[flat]));
    assert!(!result("(width)", &[zero]));
    assert!(result("not (width)", &[zero]));
}
#[test]
fn style_and_scroll_capabilities_remain_explicit() {
    let context = LengthContext { unit_bases: &[] };
    let mut outer = ancestor(1, SizeKind::Size, 400.0, 100.0, &context);
    assert_eq!(
        container_matches(&text("style(--tone:red)"), &[outer], limits()),
        Err(ContainerError::RequiresStyleQuery(1))
    );
    assert!(result("(width > 300px) or style(--tone:red)", &[outer]));
    assert!(!result("scroll-state(stuck:top)", &[outer]));
    outer.scroll_state = true;
    assert_eq!(
        container_matches(&text("scroll-state(stuck:top)"), &[outer], limits()),
        Err(ContainerError::RequiresScrollState(1))
    );
}
#[test]
fn invalid_measurements_and_limits_are_not_css_false() {
    let context = LengthContext { unit_bases: &[] };
    let invalid = ancestor(1, SizeKind::Size, -1.0, 10.0, &context);
    assert_eq!(
        container_matches(&text("(width)"), &[invalid], limits()),
        Err(ContainerError::InvalidMeasurement(1))
    );
    let mut limit = limits();
    limit.max_nesting = 1;
    assert_eq!(
        container_matches(&text("not (((width)))"), &[], limit),
        Err(ContainerError::Computation(ComputedCssError::NestingLimit))
    );
    limit = limits();
    limit.max_output_units = 2;
    assert_eq!(
        container_matches(&text("(width)"), &[], limit),
        Err(ContainerError::Computation(ComputedCssError::ResourceLimit))
    );
    assert_eq!(
        container_matches(&text("(width >)"), &[], limits()),
        Err(ContainerError::InvalidSyntax)
    );
}

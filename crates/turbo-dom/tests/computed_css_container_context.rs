#![cfg(feature = "computed-style-grammar")]
use turbo_dom::rtdom::computed_css::{
    container::{parse_container_context, ContainerError, InlineAxis, SizeKind},
    ComputationLimits, ComputedCssError, CssText,
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
#[test]
fn canonical_context_decodes_tokens_and_keeps_axes_independent() {
    let context = parse_container_context(
        &text("b\\6f x second"),
        &text("INLINE-SIZE"),
        &text("vertical-rl"),
        limits(),
    )
    .unwrap();
    assert_eq!(context.names, ["box", "second"]);
    assert_eq!(context.kind, SizeKind::InlineSize);
    assert_eq!(context.inline_axis, InlineAxis::Vertical);
    assert!(!context.scroll_state);
    let scroll = parse_container_context(
        &text("none"),
        &text("scroll-state"),
        &text("horizontal-tb"),
        limits(),
    )
    .unwrap();
    assert!(scroll.names.is_empty());
    assert_eq!(scroll.kind, SizeKind::Normal);
    assert!(scroll.scroll_state);
    for mode in ["vertical-lr", "sideways-lr", "sideways-rl"] {
        assert_eq!(
            parse_container_context(&text("none"), &text("size"), &text(mode), limits())
                .unwrap()
                .inline_axis,
            InlineAxis::Vertical
        );
    }
}
#[test]
fn unresolved_or_invalid_context_is_not_guessed() {
    for mode in [
        "inherit",
        "initial",
        "revert-layer",
        "future-mode",
        "vertical-rl extra",
    ] {
        assert_eq!(
            parse_container_context(&text("none"), &text("size"), &text(mode), limits()),
            Err(ContainerError::InvalidSyntax)
        );
    }
    for kind in ["inherit", "future", "size scroll-state"] {
        assert_eq!(
            parse_container_context(&text("none"), &text(kind), &text("horizontal-tb"), limits()),
            Err(ContainerError::InvalidSyntax)
        );
    }
    let mut limit = limits();
    limit.max_output_units = 1;
    assert_eq!(
        parse_container_context(
            &text("none"),
            &text("normal"),
            &text("horizontal-tb"),
            limit
        ),
        Err(ContainerError::Computation(ComputedCssError::ResourceLimit))
    );
}

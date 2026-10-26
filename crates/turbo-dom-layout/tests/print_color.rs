//! Print-colour tests (AC-7.x): `cmyk(...)` colours parse only when the style /
//! layout entries run with `accept_cmyk` (html2pdf's `print-color` feature sets it).
//! Before the consolidation's S2 step this was the crate's compile-time
//! `print-color` feature (tests then compiled only with it; now they always run).
//!
//! Split at the S1 relocation: the cases that need `t:` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/print_color.rs`.

use turbo_dom_layout::layout::value::{parse_color, parse_color_with, Rgba};
use turbo_dom_layout::{
    build_cascade, layout_with_options, style_input, FontRegistry, FragmentContent, ImageCtx,
    LayoutOptions, NoImages, Roots, StyleInput, StyleOptions, TreeView,
};

fn cmyk(s: &str) -> Option<Rgba> {
    parse_color_with(s, true)
}

#[test]
fn cmyk_function_parses_percentages_and_fractions() {
    // Pure cyan ink: cmyk(100%,0,0,0) -> rgb(0,255,255).
    assert_eq!(
        cmyk("cmyk(100%, 0, 0, 0)"),
        Some(Rgba::new(0, 255, 255, 255))
    );
    // Fractional form, key ink only.
    assert_eq!(cmyk("cmyk(0, 0, 0, 1)"), Some(Rgba::new(0, 0, 0, 255)));
    // Slash-separated, magenta.
    assert_eq!(
        cmyk("cmyk(0 / 1 / 0 / 0)"),
        Some(Rgba::new(255, 0, 255, 255))
    );
    // No ink at all -> white.
    assert_eq!(cmyk("cmyk(0,0,0,0)"), Some(Rgba::new(255, 255, 255, 255)));
}

#[test]
fn cmyk_function_rejects_malformed_input() {
    assert_eq!(cmyk("cmyk(1, 2, 3)"), None); // too few components
    assert_eq!(cmyk("cmyk(1, 2, 3, x)"), None); // non-numeric
    assert_eq!(cmyk("cmyk 1,0,0,0"), None); // no parens
}

#[test]
fn cmyk_components_clamp_out_of_range() {
    // Over-range components clamp to [0,1]: 200% -> 1.0 (full ink).
    assert_eq!(
        cmyk("cmyk(200%, 0, 0, 0)"),
        Some(Rgba::new(0, 255, 255, 255))
    );
    // Negative key clamps to 0 (no key ink).
    assert_eq!(
        cmyk("cmyk(0, 0, 0, -1)"),
        Some(Rgba::new(255, 255, 255, 255))
    );
}

#[test]
fn cmyk_is_rejected_unless_accepted() {
    assert_eq!(parse_color("cmyk(100%, 0, 0, 0)"), None);
    assert_eq!(parse_color_with("cmyk(100%, 0, 0, 0)", false), None);
    // Ordinary colours parse either way.
    assert_eq!(parse_color_with("#00ffff", false), cmyk("#00ffff"));
}

/// The background of the first box fragment of a one-div document laid out with
/// `accept` for both the style (border-color shorthand validation) and layout
/// (colour resolution) entries.
fn laid_out_colors(accept: bool) -> (Option<Rgba>, u16) {
    let tree = turbo_dom_layout::turbo_dom::Tree::parse(
        r#"<body><div style="background-color: cmyk(100%, 0, 0, 0); border: 2px solid; border-color: cmyk(0, 0, 0, 1); height: 10px"></div></body>"#,
    );
    let body = TreeView::new(&tree).find_element("body").unwrap();
    let roots = tree.children(body);
    let input = StyleInput {
        view: TreeView::new(&tree),
        roots: Roots::Detached(&roots),
    };
    let cascade = build_cascade("", "", Default::default());
    let styled = style_input(
        &input,
        &cascade,
        &StyleOptions {
            accept_cmyk: accept,
        },
    );
    let options = LayoutOptions {
        accept_cmyk: accept,
        ..LayoutOptions::default()
    };
    let galley = layout_with_options(
        &styled,
        200.0,
        &FontRegistry::new(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &options,
        &mut Default::default(),
    );
    let div = &galley.children[0];
    match &div.content {
        FragmentContent::Box {
            background, border, ..
        } => (*background, border.top.width),
        other => panic!("box fragment expected, got {other:?}"),
    }
}

#[test]
fn style_and_layout_entries_accept_cmyk_only_when_asked() {
    let (background, border) = laid_out_colors(true);
    assert_eq!(background, Some(Rgba::new(0, 255, 255, 255)));
    assert_eq!(border, 2);
    let (background, _) = laid_out_colors(false);
    assert_eq!(background, None);
    // The scope ends with the entry: the plain parser rejects cmyk again.
    assert_eq!(parse_color("cmyk(0,0,0,0)"), None);
}

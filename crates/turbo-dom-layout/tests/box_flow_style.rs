//! Public flow facts agree bit-for-bit with full style resolution.
use turbo_dom_layout::layout::value::{
    resolve_box_flow_style, resolve_box_style, BoxFlowStyle, Edges, LengthPct, Position, ResolveCtx,
};
use turbo_dom_layout::style::{
    root_font_scope, set_media_viewport_height, set_media_viewport_width, ComputedStyle,
};

fn length_bits(length: LengthPct) -> (u8, u32, u32) {
    match length {
        LengthPct::Auto => (0, 0, 0),
        LengthPct::Px(value) => (1, value.to_bits(), 0),
        LengthPct::Pct(value) => (2, value.to_bits(), 0),
        LengthPct::Calc { pct, px } => (3, pct.to_bits(), px.to_bits()),
    }
}

fn edges_bits(edges: Edges) -> [u32; 4] {
    [edges.top, edges.right, edges.bottom, edges.left].map(f32::to_bits)
}

fn compare(style: &ComputedStyle, ctx: ResolveCtx) {
    let full = resolve_box_style(style, ctx);
    let flow = resolve_box_flow_style(style, ctx);
    assert_eq!(flow.position, full.position);
    assert_eq!(length_bits(flow.inset_top), length_bits(full.inset_top));
    assert_eq!(
        length_bits(flow.inset_bottom),
        length_bits(full.inset_bottom)
    );
    assert_eq!(edges_bits(flow.margin), edges_bits(full.margin));
    assert_eq!(edges_bits(flow.padding), edges_bits(full.padding));
    assert_eq!(
        edges_bits(flow.border_widths),
        edges_bits(full.border.widths())
    );
    assert_eq!(length_bits(flow.height), length_bits(full.height));
    assert_eq!(flow.font_size.to_bits(), full.font_size.to_bits());
}

#[test]
fn frame_facts_preserve_relative_units_border_precedence_and_context_changes() {
    let cases = [
        vec![],
        vec![
            ("font-size", "150%"),
            ("position", "relative"),
            ("top", "10%"),
            ("bottom", "calc(25% - 2px)"),
            ("height", "40%"),
            ("padding", "2% 1em"),
            ("margin", "auto 2rem -3px 4vw"),
        ],
        vec![
            ("font-size", "2rem"),
            ("position", "sticky"),
            ("top", "1vh"),
            ("border", "3px inset currentcolor"),
            ("border-left", "5px outset red"),
            ("border-right-width", "0.5em"),
            ("border-width", "thin"),
            ("color", "#123456"),
        ],
        vec![
            ("font-size", "larger"),
            ("height", "bad-value"),
            ("top", "auto"),
            ("margin", "1em"),
            ("margin-block-end", "7%"),
            ("padding-inline-start", "2rem"),
            ("border", "invalid"),
            ("border-bottom-width", "thick"),
        ],
        vec![
            ("font-size", "12px"),
            ("position", "fixed"),
            ("height", "0"),
            ("-internal-ua-padding", "1em 2px"),
            ("-internal-ua-margin-block-end", "1em"),
            ("writing-mode", "vertical-rl"),
        ],
    ];
    for (root, viewport) in [(16.0, (800.0, 600.0)), (23.0, (1200.0, 900.0))] {
        let _scope = root_font_scope(root);
        set_media_viewport_width(viewport.0);
        set_media_viewport_height(viewport.1);
        for pairs in &cases {
            let style = ComputedStyle::from_pairs(pairs.iter().copied());
            for (parent_font_size, cb_width) in [(12.0, 0.0), (20.0, 120.0), (32.0, 503.25)] {
                compare(
                    &style,
                    ResolveCtx {
                        parent_font_size,
                        cb_width,
                    },
                );
            }
        }
    }
    set_media_viewport_width(800.0);
    set_media_viewport_height(600.0);
}

#[test]
fn flow_facts_have_independent_expected_geometry_for_each_style() {
    let style = ComputedStyle::from_pairs([
        ("font-size", "150%"),
        ("position", "absolute"),
        ("top", "25%"),
        ("padding", "10%"),
        ("margin", "2px"),
        ("border", "3px solid red"),
        ("height", "2em"),
    ]);
    let ctx = ResolveCtx {
        parent_font_size: 20.0,
        cb_width: 200.0,
    };
    assert_eq!(
        resolve_box_flow_style(&style, ctx),
        BoxFlowStyle {
            position: Position::Absolute,
            inset_top: LengthPct::Pct(25.0),
            inset_bottom: LengthPct::Auto,
            padding: Edges::all(20.0),
            margin: Edges::all(2.0),
            border_widths: Edges::all(3.0),
            height: LengthPct::Px(60.0),
            font_size: 30.0,
        }
    );
    let style =
        ComputedStyle::from_pairs([("font-size", "10px"), ("padding", "1em"), ("height", "2em")]);
    assert_eq!(
        resolve_box_flow_style(&style, ctx).padding,
        Edges::all(10.0)
    );
    assert_eq!(
        resolve_box_flow_style(&style, ctx).height,
        LengthPct::Px(20.0)
    );
}

#[cfg(feature = "perf")]
#[test]
fn projection_never_invokes_full_box_resolution() {
    let style = ComputedStyle::from_pairs([
        ("font-family", "'an unused family', serif"),
        ("box-shadow", "2px 3px 4px red"),
        ("background", "linear-gradient(red, blue)"),
        ("transform", "rotate(45deg)"),
    ]);
    turbo_dom_layout::perf::reset();
    let _ = resolve_box_flow_style(
        &style,
        ResolveCtx {
            parent_font_size: 20.0,
            cb_width: 200.0,
        },
    );
    assert_eq!(
        turbo_dom_layout::perf::count("layout.resolve_box_flow_style"),
        1
    );
    assert_eq!(turbo_dom_layout::perf::count("layout.resolve_box_style"), 0);
}

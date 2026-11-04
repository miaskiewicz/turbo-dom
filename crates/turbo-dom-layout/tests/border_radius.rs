//! Chrome 156's `border-radius` model in the layout galley (miaskiewicz/turbo-dom#253):
//! the used radii of each corner (per-corner, elliptical, percentage, the overlap rule,
//! logical corners under RTL and vertical writing modes, the cascade order of the
//! shorthand and its longhands) and the `overflow` clip region with its inner radii. The
//! pixels are checked against Chrome by turbo-dom-raster's `paint_chrome` `radius-*`
//! cases, laid out from the same pages.
use turbo_dom_layout::layout::value::{resolve_radii, CornerRadius, LengthPct};
use turbo_dom_layout::{
    layout_html, BorderRadii, CornerRadii, Diagnostics, Fragment, FragmentContent, OverflowClip,
};

/// Every fragment of `body` (laid out at 1280px with `body { margin: 0 }`), depth first.
fn fragments(body: &str) -> Vec<Fragment> {
    fn walk(f: &Fragment, out: &mut Vec<Fragment>) {
        out.push(f.clone());
        for c in &f.children {
            walk(c, out);
        }
    }
    let html = format!("<!DOCTYPE html><style>body{{margin:0}}</style><body>{body}</body>");
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let root = layout_html(&html, "", 1280.0, &fonts, &mut Diagnostics::default()).unwrap();
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

/// The fragment whose border box is at `(x, y)` with size `w` x `h`.
fn at(frags: &[Fragment], (x, y, w, h): (f32, f32, f32, f32)) -> Fragment {
    frags
        .iter()
        .find(|f| f.x == x && f.y == y && f.width == w && f.height == h)
        .unwrap_or_else(|| panic!("no fragment at {x},{y} {w}x{h}"))
        .clone()
}

fn radii(f: &Fragment) -> BorderRadii {
    match &f.content {
        FragmentContent::Box { border_radius, .. } => *border_radius,
        other => panic!("not a box: {other:?}"),
    }
}

fn c(h: f32, v: f32) -> CornerRadii {
    CornerRadii { h, v }
}

fn corners(tl: CornerRadii, tr: CornerRadii, br: CornerRadii, bl: CornerRadii) -> BorderRadii {
    BorderRadii::from_corners([tl, tr, br, bl])
}

const ABS: &str = "position:absolute;left:20px;top:20px";

/// The used radii of a 100x60 box at (20, 20) with the extra style `style`.
fn box_radii(style: &str) -> BorderRadii {
    let frags = fragments(&format!(
        "<div style=\"{ABS};width:100px;height:60px;background:red;{style}\"></div>"
    ));
    radii(&at(&frags, (20.0, 20.0, 100.0, 60.0)))
}

#[test]
fn shorthand_values_set_each_corner() {
    let zero = c(0.0, 0.0);
    assert_eq!(
        box_radii("border-radius:0 10px 30px 20px"),
        corners(zero, c(10.0, 10.0), c(30.0, 30.0), c(20.0, 20.0))
    );
    assert_eq!(
        box_radii("border-radius:10px 30px / 20px 5px"),
        corners(c(10.0, 20.0), c(30.0, 5.0), c(10.0, 20.0), c(30.0, 5.0))
    );
    // Percentages resolve per axis: an ellipse on a non-square box, not a pill.
    assert_eq!(
        box_radii("border-radius:50%"),
        BorderRadii::from_corners([c(50.0, 30.0); 4])
    );
    // Invalid values.
    assert_eq!(box_radii("border-radius:auto"), BorderRadii::ZERO);
    assert_eq!(box_radii("border-radius:-4px"), BorderRadii::ZERO);
}

#[test]
fn longhands_win_over_the_shorthand_in_cascade_order() {
    assert_eq!(
        box_radii("border-radius:4px;border-top-left-radius:20px 10px"),
        corners(c(20.0, 10.0), c(4.0, 4.0), c(4.0, 4.0), c(4.0, 4.0))
    );
    // A later shorthand resets an earlier longhand.
    assert_eq!(
        box_radii("border-top-left-radius:20px;border-radius:4px"),
        BorderRadii::uniform(4.0)
    );
    // Invalid longhand values and a third value are square corners.
    assert_eq!(
        box_radii("border-top-left-radius:1px 2px 3px;border-top-right-radius:auto"),
        BorderRadii::ZERO
    );
}

#[test]
fn overlapping_radii_scale_down_together() {
    let frags = fragments(&format!(
        "<div style=\"{ABS};width:100px;height:200px;border-top-left-radius:150px;border-top-right-radius:50px\"></div>"
    ));
    assert_eq!(
        radii(&at(&frags, (20.0, 20.0, 100.0, 200.0))),
        corners(c(75.0, 75.0), c(25.0, 25.0), c(0.0, 0.0), c(0.0, 0.0))
    );
}

#[test]
fn logical_corners_follow_the_direction_and_writing_mode() {
    let r = c(40.0, 40.0);
    let o = c(0.0, 0.0);
    let logical = "border-start-start-radius:40px;border-end-end-radius:40px";
    assert_eq!(box_radii(logical), corners(r, o, r, o));
    assert_eq!(
        box_radii(&format!("direction:rtl;{logical}")),
        corners(o, r, o, r)
    );
    let mixed = "border-start-end-radius:40px;border-end-start-radius:40px";
    assert_eq!(box_radii(mixed), corners(o, r, o, r));
    assert_eq!(
        box_radii(&format!("direction:rtl;{mixed}")),
        corners(r, o, r, o)
    );
    // A physical longhand stays put under RTL; a flow-relative one wins over it.
    assert_eq!(
        box_radii("direction:rtl;border-top-left-radius:40px"),
        corners(r, o, o, o)
    );
    assert_eq!(
        box_radii("border-top-left-radius:10px;border-start-start-radius:40px"),
        corners(r, o, o, o)
    );
    // Vertical modes: block-start is the right (rl) or left (lr) side, inline-start the
    // top (or the bottom under RTL).
    for (mode, want) in [
        ("vertical-rl", corners(o, r, o, r)),
        ("vertical-lr", corners(r, o, r, o)),
        ("vertical-rl;direction:rtl", corners(r, o, r, o)),
        ("vertical-lr;direction:rtl", corners(o, r, o, r)),
    ] {
        let got = box_radii(&format!("writing-mode:{mode};{logical}"));
        assert_eq!(got, want, "{mode}");
    }
}

#[test]
fn vertical_boxes_keep_physical_corners_axes_and_border_sides() {
    let frags = fragments(&format!(
        "<div style=\"{ABS};width:100px;height:60px;background:red;writing-mode:vertical-rl;border-start-end-radius:20px;border-top-left-radius:30px 10px;border-right:10px solid blue\"></div>"
    ));
    let f = at(&frags, (20.0, 20.0, 110.0, 60.0));
    assert_eq!(
        radii(&f),
        corners(c(30.0, 10.0), c(0.0, 0.0), c(20.0, 20.0), c(0.0, 0.0))
    );
    let FragmentContent::Box { border, .. } = &f.content else {
        unreachable!("a box")
    };
    assert_eq!((border.right.width, border.top.width), (10, 0));
}

#[test]
fn overflow_clips_the_padding_box_with_the_inner_radii() {
    let clip = |style: &str| {
        let frags = fragments(&format!(
            "<div style=\"{ABS};width:80px;height:60px;border:10px solid blue;border-radius:40px 20px;{style}\"><div style=\"height:200px\"></div></div>"
        ));
        at(&frags, (20.0, 20.0, 100.0, 80.0)).overflow_clip
    };
    let inner = corners(c(30.0, 30.0), c(10.0, 10.0), c(30.0, 30.0), c(10.0, 10.0));
    let padding = Some(OverflowClip {
        x: 10.0,
        y: 10.0,
        width: 80.0,
        height: 60.0,
        radii: inner,
    });
    assert_eq!(clip("overflow:hidden"), padding);
    assert_eq!(clip("overflow:auto"), padding);
    assert_eq!(clip("overflow:scroll visible"), padding);
    assert_eq!(clip("overflow-y:hidden"), padding);
    assert_eq!(clip("overflow:clip"), padding);
    assert_eq!(clip(""), None);
    assert_eq!(clip("overflow:visible"), None);
    // `clip` beside `visible` clips one axis only, without the radii.
    let x_only = clip("overflow-x:clip").unwrap();
    assert_eq!((x_only.x, x_only.width), (10.0, 80.0));
    assert!(x_only.y < -1000.0 && x_only.height > 2000.0);
    assert_eq!(x_only.radii, BorderRadii::ZERO);
    let y_only = clip("overflow:visible clip").unwrap();
    assert_eq!((y_only.y, y_only.height), (10.0, 60.0));
    assert!(y_only.x < -1000.0 && y_only.width > 2000.0);
}

#[test]
fn background_layers_of_a_rounded_box_clip_to_its_painting_area() {
    let c = |h, v| CornerRadii { h, v };
    // A colour clipped to the content box paints with the content edge's radii, each
    // corner less its own adjacent border and padding widths.
    let frags = fragments(&format!(
        "<div style=\"{ABS};width:80px;height:80px;padding:10px 4px 10px 20px;border-radius:40px 30px / 40px 20px;background:blue;background-clip:content-box\"></div>"
    ));
    let colour = &at(&frags, (20.0, 20.0, 104.0, 100.0)).children[0];
    assert_eq!(
        radii(colour),
        corners(c(20.0, 30.0), c(26.0, 10.0), c(36.0, 30.0), c(10.0, 10.0))
    );
    // Image / gradient tiles sit in a container clipped to the painting area's radii.
    let frags = fragments(&format!(
        "<div style=\"{ABS};width:80px;height:80px;border:10px solid blue;border-radius:40px 0;background:linear-gradient(red,red) 0 0 / 20px 20px\"></div>"
    ));
    let container = &at(&frags, (20.0, 20.0, 100.0, 100.0)).children[0];
    assert_eq!(
        container.clip.unwrap().radii,
        corners(c(40.0, 40.0), c(0.0, 0.0), c(40.0, 40.0), c(0.0, 0.0))
    );
}

#[test]
fn radii_values_resolve_fit_inset_and_compare() {
    let spec = [
        CornerRadius {
            h: LengthPct::Pct(50.0),
            v: LengthPct::Px(10.0),
        },
        CornerRadius::ZERO,
        CornerRadius {
            h: LengthPct::Auto,
            v: LengthPct::Px(-3.0),
        },
        CornerRadius::ZERO,
    ];
    let r = resolve_radii(&spec, 200.0, 100.0);
    assert_eq!(r.top_left, c(100.0, 10.0));
    assert_eq!(r.bottom_right, c(0.0, 0.0));
    assert!(r.is_rounded() && r.as_uniform().is_none());
    assert_eq!(BorderRadii::from(5.0).as_uniform(), Some(5.0));
    assert!(!BorderRadii::uniform(0.5).is_rounded());
    assert!(!c(10.0, 0.4).is_rounded());
    // Square corners never scale; a zero-size box scales every radius to zero.
    assert_eq!(BorderRadii::ZERO.fit(0.0, 0.0), BorderRadii::ZERO);
    assert_eq!(BorderRadii::uniform(4.0).fit(0.0, 10.0), BorderRadii::ZERO);
    assert_eq!(
        BorderRadii::uniform(10.0).inset(2.0, 4.0, 12.0, 0.0),
        corners(c(10.0, 8.0), c(6.0, 8.0), c(6.0, 0.0), c(10.0, 0.0))
    );
    assert_eq!(
        BorderRadii::uniform(3.0).map(|k| c(k.v * 2.0, k.h)),
        BorderRadii::from_corners([c(6.0, 3.0); 4])
    );
}

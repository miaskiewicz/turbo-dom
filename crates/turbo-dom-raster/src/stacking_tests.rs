use std::collections::HashMap;
use turbo_dom_layout::{
    BorderRadii, ClipRect, Fragment, FragmentContent, NodeId, OverflowClip, Rgba,
};

const GREEN: Rgba = Rgba {
    r: 0,
    g: 255,
    b: 0,
    a: 255,
};
const RED: Rgba = Rgba {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

fn boxed(id: u32, color: Option<Rgba>) -> Fragment {
    Fragment::new(
        NodeId(id),
        0.0,
        0.0,
        100.0,
        100.0,
        FragmentContent::Box {
            background: color,
            border: turbo_dom_layout::layout::value::BorderEdges::default(),
            border_radius: BorderRadii::ZERO,
            shadow: None,
            gradient: None,
            transform: None,
        },
    )
}

fn fixture(context: bool, clip: Option<OverflowClip>) -> Fragment {
    let mut inner = boxed(2, Some(GREEN));
    inner.is_positioned = true;
    inner.z_index = Some(100);
    let mut subject = boxed(1, None);
    subject.is_positioned = true;
    subject.establishes_stacking_context = context;
    subject.overflow_clip = clip;
    subject.children.push(inner);
    let mut cover = boxed(3, Some(RED));
    cover.is_positioned = true;
    cover.z_index = Some(1);
    let mut root = boxed(0, None);
    root.children = vec![subject, cover];
    root
}

fn verify(root: &Fragment, point: (usize, usize), expected: Rgba) {
    let assets = HashMap::new();
    let png = crate::paint_png::paint(
        root,
        100,
        100,
        Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        &assets,
    )
    .unwrap();
    let png = turbo_dom_layout::image::decode_png_rgba(&png).unwrap();
    let svg = crate::paint_svg::paint(
        root,
        100,
        100,
        Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        &assets,
    );
    let svg = turbo_dom_layout::svg::rasterize(
        svg.as_bytes(),
        1.0,
        turbo_dom_layout::svg::Oversize::Reject,
    )
    .unwrap();
    let at = 4 * (point.1 * 100 + point.0);
    for image in [png, svg] {
        assert_eq!(
            &image.rgba[at..at + 4],
            &[expected.r, expected.g, expected.b, expected.a]
        );
    }
}

#[test]
fn positioned_auto_descendant_paints_above_outside_lower_z_in_png_and_svg() {
    verify(&fixture(false, None), (50, 50), GREEN);
    verify(&fixture(true, None), (50, 50), RED);
}

#[test]
fn promoted_descendant_retains_non_context_overflow_clip_in_png_and_svg() {
    let clip = OverflowClip {
        x: 0.0,
        y: 0.0,
        width: 40.0,
        height: 100.0,
        radii: BorderRadii::ZERO,
    };
    let root = fixture(false, Some(clip));
    verify(&root, (20, 50), GREEN);
    verify(&root, (50, 50), RED);
}

#[test]
fn promoted_descendant_retains_rounded_clip_in_png_and_svg() {
    let clip = OverflowClip {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        radii: BorderRadii::uniform(40.0),
    };
    let root = fixture(false, Some(clip));
    verify(&root, (50, 50), GREEN);
    verify(&root, (0, 0), RED);
}

#[test]
fn promoted_descendant_retains_legacy_rect_clip_in_png_and_svg() {
    let mut root = fixture(false, None);
    root.children[0].clip = Some(ClipRect {
        x: 0.0,
        y: 0.0,
        width: 40.0,
        height: 100.0,
        radii: BorderRadii::ZERO,
    });
    verify(&root, (20, 50), GREEN);
    verify(&root, (50, 50), RED);
    root.children[0].clip.as_mut().unwrap().width = 0.0;
    verify(&root, (20, 50), RED);
}

#[test]
fn empty_promoted_ancestor_overflow_does_not_leave_unbalanced_svg_groups() {
    let clip = OverflowClip {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 100.0,
        radii: BorderRadii::ZERO,
    };
    let mut root = fixture(false, Some(clip));
    root.children[0].clip = Some(ClipRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        radii: BorderRadii::ZERO,
    });
    verify(&root, (20, 50), RED);
    root.children[0].overflow_clip.as_mut().unwrap().width = 100.0;
    verify(&root, (20, 50), GREEN);
}

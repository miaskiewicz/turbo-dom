//! Capture bounds in CSS pixels, retaining positioned descendants and CSS clipping.
use tiny_skia::{Rect, Transform};
use turbo_dom_layout::{Fragment, FragmentContent};

pub(crate) fn content_bottom(root: &Fragment) -> f32 {
    subtree_bottom(root, Transform::identity(), None)
}

fn box_transform(f: &Fragment, parent: Transform) -> Transform {
    let FragmentContent::Box {
        transform: Some(t), ..
    } = &f.content
    else {
        return parent;
    };
    let (x, y) = (f.x + t.origin_x, f.y + t.origin_y);
    let [a, b, c, d, e, g] = t.matrix;
    parent.pre_concat(
        Transform::from_translate(x, y)
            .pre_concat(Transform::from_row(a, b, c, d, e, g))
            .pre_concat(Transform::from_translate(-x, -y)),
    )
}

fn transformed_rect(x: f32, y: f32, width: f32, height: f32, xf: Transform) -> Option<Rect> {
    Rect::from_xywh(x, y, width, height)?.transform(xf)
}

fn intersect(a: Option<Rect>, b: Rect) -> Option<Rect> {
    a.map_or(Some(b), |a| a.intersect(&b))
}

fn subtree_bottom(f: &Fragment, parent: Transform, inherited_clip: Option<Rect>) -> f32 {
    let xf = box_transform(f, parent);
    let Ok(clip) = clip_region(
        f,
        xf,
        inherited_clip,
        f.clip.map(|c| (c.x, c.y, c.width, c.height)),
    ) else {
        return 0.0;
    };
    let mut bottom = transformed_rect(f.x, f.y, f.width, f.height, xf)
        .and_then(|r| intersect(clip, r))
        .map_or(0.0, |r| r.bottom());
    let Ok(child_clip) = clip_region(
        f,
        xf,
        clip,
        f.overflow_clip.map(|c| (c.x, c.y, c.width, c.height)),
    ) else {
        return bottom;
    };
    for child in &f.children {
        bottom = bottom.max(subtree_bottom(child, xf, child_clip));
    }
    bottom
}

fn clip_region(
    f: &Fragment,
    xf: Transform,
    inherited: Option<Rect>,
    local: Option<(f32, f32, f32, f32)>,
) -> Result<Option<Rect>, ()> {
    let Some((x, y, w, h)) = local else {
        return Ok(inherited);
    };
    let bounds = transformed_rect(f.x + x, f.y + y, w, h, xf).ok_or(())?;
    intersect(inherited, bounds).map(Some).ok_or(())
}

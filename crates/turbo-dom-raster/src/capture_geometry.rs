//! Scroll bounds in CSS pixels and shared transform/clip geometry for capture.
use tiny_skia::{Rect, Transform};
use turbo_dom_layout::{Fragment, FragmentContent};

pub(crate) fn content_bottom(root: &Fragment) -> f32 {
    subtree_bottom(root, Transform::identity(), None)
}

pub(crate) fn box_transform(f: &Fragment, parent: Transform) -> Transform {
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

pub(crate) fn transformed_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    xf: Transform,
) -> Option<Rect> {
    Rect::from_xywh(x, y, width, height)?.transform(xf)
}

pub(crate) fn intersect(a: Option<Rect>, b: Rect) -> Option<Rect> {
    a.map_or(Some(b), |a| a.intersect(&b))
}

fn subtree_bottom(f: &Fragment, parent: Transform, inherited_clip: Option<Rect>) -> f32 {
    let xf = box_transform(f, parent);
    // CSS clip hides ink without changing scrollable overflow. Only ancestor
    // overflow clipping constrains the positioned boxes in a full-page capture.
    let mut bottom = transformed_rect(f.x, f.y, f.width, f.height, xf)
        .and_then(|r| intersect(inherited_clip, r))
        .map_or(0.0, |r| r.bottom());
    let Ok(child_clip) = clip_region(
        f,
        xf,
        inherited_clip,
        f.overflow_clip.map(|c| (c.x, c.y, c.width, c.height)),
    ) else {
        return bottom;
    };
    for child in &f.children {
        bottom = bottom.max(subtree_bottom(child, xf, child_clip));
    }
    bottom
}

pub(crate) fn clip_region(
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

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_layout::layout::value::BorderEdges;
    use turbo_dom_layout::{BorderRadii, ClipRect, NodeId, OverflowClip};

    fn fragment(y: f32, height: f32) -> Fragment {
        Fragment::new(
            NodeId(0),
            0.0,
            y,
            20.0,
            height,
            FragmentContent::Box {
                background: None,
                border: BorderEdges::default(),
                border_radius: BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            },
        )
    }

    fn clip(width: f32, height: f32) -> ClipRect {
        ClipRect {
            x: 0.0,
            y: 0.0,
            width,
            height,
            radii: BorderRadii::ZERO,
        }
    }

    #[test]
    fn empty_overflow_clip_suppresses_children_but_retains_own_box() {
        let mut root = fragment(30.0, 20.0);
        root.children.push(fragment(200.0, 40.0));
        assert_eq!(content_bottom(&root), 240.0);
        root.overflow_clip = Some(OverflowClip {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 0.0,
            radii: BorderRadii::ZERO,
        });
        assert_eq!(content_bottom(&root), 50.0);
    }

    #[test]
    fn scroll_extent_ignores_css_clip_but_respects_ancestor_overflow() {
        let inherited = Rect::from_xywh(0.0, 0.0, 20.0, 20.0);
        let mut root = fragment(0.0, 100.0);
        root.children.push(fragment(0.0, 200.0));
        let mut outside = clip(20.0, 10.0);
        outside.y = 40.0;
        root.clip = Some(outside);
        assert_eq!(content_bottom(&root), 200.0);
        assert_eq!(
            subtree_bottom(&root, Transform::identity(), inherited),
            20.0
        );
        root.clip = None;
        root.overflow_clip = Some(OverflowClip {
            x: outside.x,
            y: outside.y,
            width: outside.width,
            height: outside.height,
            radii: outside.radii,
        });
        assert_eq!(
            subtree_bottom(&root, Transform::identity(), inherited),
            20.0
        );
    }
}

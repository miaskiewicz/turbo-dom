//! Rounded-rectangle geometry the PNG and SVG painters share (miaskiewicz/turbo-dom#253):
//! a box's elliptical corner radii as one outline, its padding box with the inner radii,
//! a shadow's grown radii and the region each border side paints at a rounded corner.
use turbo_dom_layout::layout::value::BorderEdges;
use turbo_dom_layout::{BorderRadii, CornerRadii, Fragment};

/// One outline step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// The cubic-Bézier control ratio of a quarter ellipse.
const KAPPA: f32 = 0.552_284_7;

/// A rectangle `(x, y, width, height)`.
pub(crate) type Rect = (f32, f32, f32, f32);

/// The outline of `rect` with corner radii `radii` (scaled down to fit the rectangle;
/// a corner with either radius at zero is square), clockwise from the top edge.
pub(crate) fn outline((x, y, w, h): Rect, radii: &BorderRadii) -> Vec<Seg> {
    let square = |c: CornerRadii| {
        if c.h > 0.0 && c.v > 0.0 {
            c
        } else {
            CornerRadii::default()
        }
    };
    let r = radii.map(square).fit(w, h);
    let (tl, tr, br, bl) = (r.top_left, r.top_right, r.bottom_right, r.bottom_left);
    let (right, bottom) = (x + w, y + h);
    vec![
        Seg::Move(x + tl.h, y),
        Seg::Line(right - tr.h, y),
        Seg::Cubic(
            right - tr.h + tr.h * KAPPA,
            y,
            right,
            y + tr.v - tr.v * KAPPA,
            right,
            y + tr.v,
        ),
        Seg::Line(right, bottom - br.v),
        Seg::Cubic(
            right,
            bottom - br.v + br.v * KAPPA,
            right - br.h + br.h * KAPPA,
            bottom,
            right - br.h,
            bottom,
        ),
        Seg::Line(x + bl.h, bottom),
        Seg::Cubic(
            x + bl.h - bl.h * KAPPA,
            bottom,
            x,
            bottom - bl.v + bl.v * KAPPA,
            x,
            bottom - bl.v,
        ),
        Seg::Line(x, y + tl.v),
        Seg::Cubic(
            x,
            y + tl.v - tl.v * KAPPA,
            x + tl.h - tl.h * KAPPA,
            y,
            x + tl.h,
            y,
        ),
        Seg::Close,
    ]
}

/// A closed polygon through `points`.
pub(crate) fn polygon(points: &[(f32, f32)]) -> Vec<Seg> {
    let mut segs: Vec<Seg> = points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            if i == 0 {
                Seg::Move(x, y)
            } else {
                Seg::Line(x, y)
            }
        })
        .collect();
    segs.push(Seg::Close);
    segs
}

/// The border widths of `b` as `[top, right, bottom, left]`.
pub(crate) fn widths(b: &BorderEdges) -> [f32; 4] {
    let w = b.widths();
    [w.top, w.right, w.bottom, w.left]
}

/// The padding box of `f` (its border box less the border widths) and its inner radii:
/// each outer radius less the adjacent border width (CSS Backgrounds 3 §5.2).
pub(crate) fn padding_box(
    f: &Fragment,
    b: &BorderEdges,
    radii: &BorderRadii,
) -> (Rect, BorderRadii) {
    let [t, r, bo, l] = widths(b);
    (
        (f.x + l, f.y + t, f.width - l - r, f.height - t - bo),
        radii.inset(t, r, bo, l),
    )
}

/// An outer shadow's corner radius for a box radius `r` and a `spread` (CSS Backgrounds 3
/// §7.1, as Chrome paints it): the radius plus the spread, floored at zero — except that
/// below the spread the spread is first scaled by `1 + (r/spread - 1)^3`, so a square
/// corner keeps a square shadow and small radii grow continuously.
pub(crate) fn shadow_radius(r: f32, spread: f32) -> f32 {
    if spread > 0.0 && r < spread {
        let t = r / spread - 1.0;
        r + spread * (1.0 + t * t * t)
    } else {
        (r + spread).max(0.0)
    }
}

/// An outer shadow's radii: [`shadow_radius`] per corner and axis.
pub(crate) fn shadow_radii(radii: &BorderRadii, spread: f32) -> BorderRadii {
    radii.map(|c| CornerRadii {
        h: shadow_radius(c.h, spread),
        v: shadow_radius(c.v, spread),
    })
}

/// The region border side `side` (`0` top, `1` right, `2` bottom, `3` left) of a box
/// `rect` with border `widths` paints: the box cut at each of its two corners by the line
/// from the outer corner through the inner (padding-box) corner, so adjacent sides of
/// different widths or colours meet on that diagonal, as in Chrome.
pub(crate) fn side_region((x, y, w, h): Rect, widths: [f32; 4], side: usize) -> Vec<(f32, f32)> {
    let [t, r, b, l] = widths;
    // Outer corners (CSS order) and the direction towards their inner corners.
    let corners = [
        ((x, y), (l, t)),
        ((x + w, y), (-r, t)),
        ((x + w, y + h), (-r, -b)),
        ((x, y + h), (l, -b)),
    ];
    let reference = [
        (x + w / 2.0, y),
        (x + w, y + h / 2.0),
        (x + w / 2.0, y + h),
        (x, y + h / 2.0),
    ][side];
    let mut region = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    for corner in [corners[side], corners[(side + 1) % 4]] {
        region = keep_side(&region, corner, reference);
    }
    region
}

/// The part of convex polygon `points` on `reference`'s side of the line through
/// `origin` along `dir` (Sutherland–Hodgman against one half-plane).
fn keep_side(
    points: &[(f32, f32)],
    (origin, dir): ((f32, f32), (f32, f32)),
    reference: (f32, f32),
) -> Vec<(f32, f32)> {
    let raw = |(px, py): (f32, f32)| dir.0 * (py - origin.1) - dir.1 * (px - origin.0);
    let sign = raw(reference).signum();
    let side = |p: (f32, f32)| raw(p) * sign;
    let mut out = Vec::with_capacity(points.len() + 1);
    for (i, &p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
            let k = sp / (sp - sq);
            out.push((p.0 + (q.0 - p.0) * k, p.1 + (q.1 - p.1) * k));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlines_follow_each_corner_and_square_degenerate_ones() {
        let radii = BorderRadii {
            top_left: CornerRadii { h: 10.0, v: 0.0 },
            top_right: CornerRadii { h: 20.0, v: 10.0 },
            bottom_right: CornerRadii::circle(0.0),
            bottom_left: CornerRadii::circle(5.0),
        };
        let segs = outline((0.0, 0.0, 100.0, 50.0), &radii);
        // The half-zero top-left corner is square; the top-right one is elliptical.
        assert_eq!(segs[0], Seg::Move(0.0, 0.0));
        assert_eq!(segs[1], Seg::Line(80.0, 0.0));
        assert_eq!(segs[3], Seg::Line(100.0, 50.0));
        assert_eq!(
            segs[6],
            Seg::Cubic(5.0 - 5.0 * KAPPA, 50.0, 0.0, 45.0 + 5.0 * KAPPA, 0.0, 45.0)
        );
        assert_eq!(segs[9], Seg::Close);
        // Radii past the box scale down together.
        let big = outline((0.0, 0.0, 10.0, 10.0), &BorderRadii::uniform(10.0));
        assert_eq!(big[1], Seg::Line(5.0, 0.0));
    }

    #[test]
    fn polygons_close_through_their_points() {
        assert_eq!(
            polygon(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]),
            vec![
                Seg::Move(0.0, 0.0),
                Seg::Line(1.0, 0.0),
                Seg::Line(1.0, 1.0),
                Seg::Close
            ]
        );
    }

    #[test]
    fn side_regions_meet_on_the_corner_diagonals() {
        let rect = (0.0, 0.0, 100.0, 100.0);
        // Equal widths: the top side is the triangle above both diagonals.
        assert_eq!(
            side_region(rect, [10.0; 4], 0),
            vec![(0.0, 0.0), (100.0, 0.0), (50.0, 50.0)]
        );
        // A zero-width left side gives the top side the whole top-left corner.
        let top = side_region(rect, [10.0, 10.0, 10.0, 0.0], 0);
        assert!(
            top.contains(&(0.0, 0.0)) && top.contains(&(0.0, 100.0)),
            "{top:?}"
        );
        // The bottom side of a box with a thin top.
        let bottom = side_region(rect, [0.0, 10.0, 10.0, 10.0], 2);
        assert!(bottom.contains(&(100.0, 100.0)) && bottom.contains(&(50.0, 50.0)));
    }

    #[test]
    fn shadow_radii_grow_each_corner_axis() {
        let radii = BorderRadii {
            top_left: CornerRadii { h: 20.0, v: 0.0 },
            ..BorderRadii::ZERO
        };
        let grown = shadow_radii(&radii, 10.0);
        assert_eq!(grown.top_left, CornerRadii { h: 30.0, v: 0.0 });
        assert_eq!(grown.top_right, CornerRadii::circle(0.0));
        assert!((shadow_radius(5.0, 10.0) - 13.75).abs() < 1e-5);
        assert_eq!(shadow_radius(5.0, -10.0), 0.0);
    }
}

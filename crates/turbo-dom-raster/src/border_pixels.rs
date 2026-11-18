//! Composite coloured border joins once, using convex pixel-area coverage.
//!
//! Square sides follow Blink's opaque side ordering inside nested opacity groups.
//! Rounded sides partition the ring before one shared coverage mask is applied;
//! masking and source-over compositing each side separately would darken a join.
use tiny_skia::{FillRule, Mask, Path, Pixmap, Point, Rect, Transform};
use turbo_dom_layout::layout::value::BorderEdges;
use turbo_dom_layout::{Fragment, Rgba};

use crate::rrect;

struct Side {
    polygon: Vec<Point>,
    bounds: Rect,
    color: Rgba,
}

pub(crate) fn paint(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, xf: Transform) {
    paint_sides(pm, &sides(f, b, xf), None);
}

pub(crate) fn paint_ring(
    pm: &mut Pixmap,
    f: &Fragment,
    b: &BorderEdges,
    xf: Transform,
    ring: &Path,
) {
    let Some(bounds) = ring.clone().transform(xf).map(|p| p.bounds()) else {
        return;
    };
    let left = bounds.left().floor().max(0.0) as u32;
    let top = bounds.top().floor().max(0.0) as u32;
    let right = bounds.right().ceil().min(pm.width() as f32) as u32;
    let bottom = bounds.bottom().ceil().min(pm.height() as f32) as u32;
    let Some(mut mask) = Mask::new(right.saturating_sub(left), bottom.saturating_sub(top)) else {
        return;
    };
    let local = Transform::from_translate(-(left as f32), -(top as f32)).pre_concat(xf);
    mask.fill_path(ring, FillRule::EvenOdd, true, local);
    let sides = partition_sides(f, b, xf);
    paint_sides(pm, &sides, Some(&RingMask { mask, left, top }));
}

struct RingMask {
    mask: Mask,
    left: u32,
    top: u32,
}

impl RingMask {
    fn coverage(&self, x: u32, y: u32) -> f32 {
        let (Some(x), Some(y)) = (x.checked_sub(self.left), y.checked_sub(self.top)) else {
            return 0.0;
        };
        if x >= self.mask.width() || y >= self.mask.height() {
            return 0.0;
        }
        f32::from(self.mask.data()[(y * self.mask.width() + x) as usize]) / 255.0
    }
}

fn partition_sides(f: &Fragment, b: &BorderEdges, xf: Transform) -> Vec<Side> {
    [&b.top, &b.right, &b.bottom, &b.left]
        .into_iter()
        .enumerate()
        .filter_map(|(i, edge)| {
            let color = edge.color.filter(|c| c.a > 0 && edge.width > 0)?;
            let mut polygon: Vec<_> =
                rrect::side_region((f.x, f.y, f.width, f.height), rrect::widths(b), i)
                    .into_iter()
                    .map(|(x, y)| Point::from_xy(x, y))
                    .collect();
            xf.map_points(&mut polygon);
            let bounds = Rect::from_points(&polygon)?;
            Some(Side {
                polygon,
                bounds,
                color,
            })
        })
        .collect()
}

fn paint_sides(pm: &mut Pixmap, sides: &[Side], ring: Option<&RingMask>) {
    let Some(bounds) = bounds(sides) else { return };
    let top = bounds.top().floor().max(0.0) as u32;
    let bottom = bounds.bottom().ceil().min(pm.height() as f32) as u32;
    let width = pm.width();
    for y in top..bottom {
        for (left, right) in row_spans(sides, y, width) {
            for x in left..right {
                let coverage = ring.as_ref().map_or(1.0, |m| m.coverage(x, y));
                if coverage == 0.0 {
                    continue;
                }
                let mut source = if ring.is_some() {
                    sample_partition(sides, x as f32, y as f32)
                } else {
                    sample(sides, x as f32, y as f32)
                };
                for c in &mut source {
                    *c *= coverage;
                }
                let offset = ((y * width + x) * 4) as usize;
                composite(&mut pm.data_mut()[offset..offset + 4], source);
            }
        }
    }
}

fn sample_partition(sides: &[Side], x: f32, y: f32) -> [f32; 4] {
    let mut source = [0.0; 4];
    for side in sides {
        let alpha = pixel_coverage(side, x, y) * f32::from(side.color.a) / 255.0;
        for (out, color) in source
            .iter_mut()
            .zip([side.color.r, side.color.g, side.color.b, 255])
        {
            *out += f32::from(color) * alpha;
        }
    }
    source
}

fn bounds(sides: &[Side]) -> Option<Rect> {
    let mut rect = sides.first()?.bounds;
    for side in &sides[1..] {
        rect = Rect::from_ltrb(
            rect.left().min(side.bounds.left()),
            rect.top().min(side.bounds.top()),
            rect.right().max(side.bounds.right()),
            rect.bottom().max(side.bounds.bottom()),
        )?;
    }
    Some(rect)
}

fn sides(f: &Fragment, b: &BorderEdges, xf: Transform) -> Vec<Side> {
    let edges = [&b.top, &b.right, &b.bottom, &b.left];
    // Blink's BoxBorderPainter orders solid sides top, bottom, right, left,
    // within increasing-alpha groups, allowing later sides to cover earlier ones.
    let mut order: Vec<_> = [0, 2, 1, 3]
        .into_iter()
        .filter(|&i| edges[i].width > 0 && edges[i].color.is_some_and(|c| c.a > 0))
        .collect();
    order.sort_by_key(|&i| edges[i].color.unwrap().a);
    let mut completed =
        std::array::from_fn(|i| edges[i].width == 0 || edges[i].color.is_none_or(|c| c.a == 0));
    let mut result = Vec::new();
    for i in order {
        let mut polygon = side_polygon(f, rrect::widths(b), i, completed);
        xf.map_points(&mut polygon);
        if let Some(bounds) = Rect::from_points(&polygon) {
            result.push(Side {
                polygon,
                bounds,
                color: edges[i].color.unwrap(),
            });
        }
        completed[i] = true;
    }
    result
}

fn side_polygon(f: &Fragment, widths: [f32; 4], side: usize, completed: [bool; 4]) -> Vec<Point> {
    let [t, r, b, l] = widths;
    let (x, y, w, h) = (f.x, f.y, f.width, f.height);
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
    let mut polygon = vec![
        Point::from_xy(x, y),
        Point::from_xy(x + w, y),
        Point::from_xy(x + w, y + h),
        Point::from_xy(x, y + h),
    ];
    polygon = clip_rect(&polygon, band(f, widths, side));
    for (corner, adjacent) in [(side, (side + 3) % 4), ((side + 1) % 4, (side + 1) % 4)] {
        if completed[adjacent] {
            let ((ox, oy), (dx, dy)) = corners[corner];
            let sign = (dx * (reference.1 - oy) - dy * (reference.0 - ox)).signum();
            polygon = clip_by(&polygon, |p| (dx * (p.y - oy) - dy * (p.x - ox)) * sign);
        }
    }
    polygon
}

fn band(f: &Fragment, widths: [f32; 4], side: usize) -> [f32; 4] {
    let [t, r, b, l] = widths;
    match side {
        0 => [f.x, f.y, f.x + f.width, f.y + t],
        1 => [f.x + f.width - r, f.y, f.x + f.width, f.y + f.height],
        2 => [f.x, f.y + f.height - b, f.x + f.width, f.y + f.height],
        _ => [f.x, f.y, f.x + l, f.y + f.height],
    }
}

fn clip_rect(points: &[Point], [l, t, r, b]: [f32; 4]) -> Vec<Point> {
    let mut polygon = points.to_vec();
    for (axis, bound, sign) in [(0, l, 1.0), (0, r, -1.0), (1, t, 1.0), (1, b, -1.0)] {
        polygon = clip(&polygon, axis, bound, sign);
    }
    polygon
}

fn clip(points: &[Point], axis: usize, bound: f32, sign: f32) -> Vec<Point> {
    clip_by(points, |p| {
        ((if axis == 0 { p.x } else { p.y }) - bound) * sign
    })
}

fn clip_by(points: &[Point], distance: impl Fn(Point) -> f32) -> Vec<Point> {
    let mut output = Vec::with_capacity(points.len() + 1);
    for (index, &p) in points.iter().enumerate() {
        let q = points[(index + 1) % points.len()];
        let (a, b) = (distance(p), distance(q));
        if a >= 0.0 {
            output.push(p);
        }
        if (a > 0.0 && b < 0.0) || (a < 0.0 && b > 0.0) {
            let ratio = a / (a - b);
            output.push(Point::from_xy(
                p.x + (q.x - p.x) * ratio,
                p.y + (q.y - p.y) * ratio,
            ));
        }
    }
    output
}

fn area(points: &[Point]) -> f32 {
    let Some(origin) = points.first() else {
        return 0.0;
    };
    points[1..]
        .windows(2)
        .map(|pair| {
            let (p, q) = (pair[0], pair[1]);
            (p.x - origin.x) * (q.y - origin.y) - (p.y - origin.y) * (q.x - origin.x)
        })
        .sum::<f32>()
        .abs()
        * 0.5
}

fn row_spans(sides: &[Side], y: u32, width: u32) -> Vec<(u32, u32)> {
    let mut spans: Vec<_> = sides
        .iter()
        .filter(|s| s.bounds.bottom() > y as f32 && s.bounds.top() < (y + 1) as f32)
        .map(|s| {
            (
                s.bounds.left().floor().max(0.0) as u32,
                s.bounds.right().ceil().min(width as f32) as u32,
            )
        })
        .collect();
    spans.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(spans.len());
    for (left, right) in spans {
        match merged.last_mut() {
            Some(last) if left <= last.1 => last.1 = last.1.max(right),
            _ => merged.push((left, right)),
        }
    }
    merged
}

fn sample(sides: &[Side], x: f32, y: f32) -> [f32; 4] {
    let mut source = [0.0; 4];
    let mut previous_alpha = 0;
    for side in sides {
        let alpha = side.color.a;
        // Closing the preceding nested group scales its source by the ratio
        // of the two opacities. The sides themselves always paint opaquely.
        if alpha != previous_alpha {
            let scale = f32::from(previous_alpha) / f32::from(alpha);
            for c in &mut source {
                *c *= scale;
            }
            previous_alpha = alpha;
        }
        let coverage = pixel_coverage(side, x, y);
        for (out, color) in source
            .iter_mut()
            .zip([side.color.r, side.color.g, side.color.b, 255])
        {
            *out = f32::from(color) * coverage + *out * (1.0 - coverage);
        }
    }
    let scale = f32::from(previous_alpha) / 255.0;
    for c in &mut source {
        *c *= scale;
    }
    source
}

fn pixel_coverage(side: &Side, x: f32, y: f32) -> f32 {
    if side.bounds.right() <= x
        || side.bounds.left() >= x + 1.0
        || side.bounds.bottom() <= y
        || side.bounds.top() >= y + 1.0
    {
        return 0.0;
    }
    area(&clip_rect(&side.polygon, [x, y, x + 1.0, y + 1.0])).min(1.0)
}

fn composite(destination: &mut [u8], source: [f32; 4]) {
    let inverse = 1.0 - source[3] / 255.0;
    for (destination, source) in destination.iter_mut().zip(source) {
        *destination = (source + f32::from(*destination) * inverse)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_quad_coverage_is_its_pixel_intersection_area() {
        // The independent Chrome SVG diagnostic reports 72/255 coverage here.
        // The polygon intersects this pixel in a 1/4×1/2 rectangle plus a
        // right triangle of side 1/4: 1/8 + 1/32 = 5/32, not 72/255.
        let polygon = vec![
            Point::from_xy(0.25, 0.5),
            Point::from_xy(2.25, 2.5),
            Point::from_xy(2.25, 6.5),
            Point::from_xy(0.25, 8.5),
        ];
        let side = Side {
            bounds: Rect::from_points(&polygon).unwrap(),
            polygon,
            color: Rgba::BLACK,
        };
        assert!((pixel_coverage(&side, 2.0, 2.0) - 0.15625).abs() < f32::EPSILON);
        assert_eq!(pixel_coverage(&side, 4.0, 2.0), 0.0);
    }
}

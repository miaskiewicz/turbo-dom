//! Border sides overdraw opaquely within Blink-style nested opacity groups.
use std::fmt::Write;

use turbo_dom_layout::layout::value::BorderEdges;
use turbo_dom_layout::{BorderRadii, Fragment, Rgba};

use super::path_data;
use crate::rrect::{self, Rect};

#[derive(Clone, Copy)]
struct Side {
    index: usize,
    color: Rgba,
}

struct Group {
    alpha: u8,
    sides: Vec<Side>,
}

struct Shape {
    rect: Rect,
    widths: [f32; 4],
    ring: Option<String>,
    depths: [f32; 4],
    alphas: [u8; 4],
}

pub(super) fn needed(b: &BorderEdges) -> bool {
    rrect::mixed_colours(b)
        && [&b.top, &b.right, &b.bottom, &b.left]
            .iter()
            .any(|s| s.width > 0 && s.color.is_some_and(|c| c.a > 0 && c.a < 255))
}

pub(super) fn paint(svg: &mut String, f: &Fragment, b: &BorderEdges) {
    let shape = Shape {
        rect: (f.x, f.y, f.width, f.height),
        widths: rrect::widths(b),
        ring: None,
        depths: rrect::widths(b),
        alphas: side_alphas(b),
    };
    paint_shape(svg, &shape, b);
}

// Keep the existing outline geometry. SVG represents the rounded border's
// coverage with an even-odd path mask; Blink uses native RRect intersection /
// difference clips, whose antialiased edge pixels differ between renderers.
pub(super) fn paint_round(svg: &mut String, f: &Fragment, b: &BorderEdges, radii: &BorderRadii) {
    let rect = (f.x, f.y, f.width, f.height);
    let (inner, inner_radii) = rrect::padding_box(f, b, radii);
    let mut ring = path_data(&rrect::outline(rect, radii));
    if inner.2 > 0.0 && inner.3 > 0.0 {
        ring.push(' ');
        ring.push_str(&path_data(&rrect::outline(inner, &inner_radii)));
    }
    let widths = rrect::widths(b);
    let r = radii.fit(f.width, f.height);
    let shape = Shape {
        rect,
        widths,
        ring: Some(ring),
        depths: [
            widths[0].max(r.top_left.v).max(r.top_right.v),
            widths[1].max(r.top_right.h).max(r.bottom_right.h),
            widths[2].max(r.bottom_left.v).max(r.bottom_right.v),
            widths[3].max(r.top_left.h).max(r.bottom_left.h),
        ],
        alphas: side_alphas(b),
    };
    paint_shape(svg, &shape, b);
}

fn paint_shape(svg: &mut String, shape: &Shape, b: &BorderEdges) {
    if let Some(ring) = &shape.ring {
        let id = svg.len();
        let (x, y, w, h) = shape.rect;
        let _ = write!(svg, "<mask id=\"border{id}\" maskUnits=\"userSpaceOnUse\" mask-type=\"alpha\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><path d=\"{ring}\" fill-rule=\"evenodd\" fill=\"white\"/></mask><g mask=\"url(#border{id})\">",x-2.0,y-2.0,w+4.0,h+4.0);
    }
    let groups = groups(b);
    let visible = groups
        .iter()
        .flat_map(|g| &g.sides)
        .fold(0, |v, s| v | (1 << s.index));
    let mut completed = !visible & 15;
    paint_groups(svg, shape, &groups, 1.0, &mut completed);
    if shape.ring.is_some() {
        svg.push_str("</g>");
    }
}

fn side_alphas(b: &BorderEdges) -> [u8; 4] {
    [&b.top, &b.right, &b.bottom, &b.left].map(|s| s.color.map_or(0, |c| c.a))
}

fn groups(b: &BorderEdges) -> Vec<Group> {
    let mut sides: Vec<_> = [&b.top, &b.right, &b.bottom, &b.left]
        .into_iter()
        .enumerate()
        .filter(|(_, s)| s.width > 0)
        .filter_map(|(index, s)| {
            s.color
                .filter(|c| c.a > 0)
                .map(|color| Side { index, color })
        })
        .collect();
    let priority = [0, 2, 1, 3];
    sides.sort_by_key(|s| (s.color.a, priority[s.index]));
    let mut groups: Vec<Group> = Vec::new();
    for side in sides {
        if groups.last().is_none_or(|g| g.alpha != side.color.a) {
            groups.push(Group {
                alpha: side.color.a,
                sides: Vec::new(),
            });
        }
        // The first side and every new alpha create a group above.
        groups.last_mut().unwrap().sides.push(side);
    }
    groups
}

fn paint_groups(
    svg: &mut String,
    shape: &Shape,
    groups: &[Group],
    effective: f32,
    completed: &mut u8,
) {
    let Some((group, lower)) = groups.split_last() else {
        return;
    };
    let flags = group.sides.iter().fold(0u8, |v, s| v | (1 << s.index));
    let adjacent = flags & ((flags << 1) | (flags >> 3)) != 0;
    let layer = group.alpha < 255 && (adjacent || !lower.is_empty());
    let alpha = f32::from(group.alpha) / 255.0;
    let next = if layer { alpha } else { effective };
    if layer {
        let _ = write!(svg, "<g opacity=\"{}\">", alpha / effective);
    }
    paint_groups(svg, shape, lower, next, completed);
    for side in &group.sides {
        paint_side(svg, shape, *side, alpha / next, *completed);
        *completed |= 1 << side.index;
    }
    if layer {
        svg.push_str("</g>");
    }
}

fn paint_side(svg: &mut String, shape: &Shape, side: Side, alpha: f32, completed: u8) {
    let c = side.color;
    let attributes = format!(
        "fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{alpha}\"",
        c.r, c.g, c.b
    );
    let d = path_data(&rrect::polygon(&side_polygon(shape, side.index, completed)));
    let _ = writeln!(svg, "<path d=\"{d}\" {attributes}/>");
}

fn side_polygon(shape: &Shape, side: usize, completed: u8) -> Vec<(f32, f32)> {
    let (x, y, w, h) = shape.rect;
    let [t, r, b, l] = shape.widths;
    let depth = shape.depths[side];
    let strip = match side {
        0 => (x, y, w, depth),
        1 => (x + w - depth, y, depth, h),
        2 => (x, y + h - depth, w, depth),
        _ => (x, y, depth, h),
    };
    let (sx, sy, sw, sh) = strip;
    let mut points = vec![(sx, sy), (sx + sw, sy), (sx + sw, sy + sh), (sx, sy + sh)];
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
    for (corner, adjacent) in [(side, (side + 3) % 4), ((side + 1) % 4, (side + 1) % 4)] {
        // A shared-opacity group can overdraw before its single blend.
        // Other groups retain independent rounded-corner coverage.
        let different_group = shape.ring.is_some() && shape.alphas[adjacent] != shape.alphas[side];
        if different_group || completed & (1 << adjacent) != 0 {
            points = rrect::keep_side(&points, corners[corner], reference);
        }
    }
    points
}

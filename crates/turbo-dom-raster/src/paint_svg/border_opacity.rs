//! Border sides overdraw opaquely within Blink-style nested opacity groups.
use std::fmt::Write;

use turbo_dom_layout::layout::value::BorderEdges;
use turbo_dom_layout::{Fragment, Rgba};

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
    };
    let groups = groups(b);
    let visible = groups
        .iter()
        .flat_map(|g| &g.sides)
        .fold(0, |v, s| v | (1 << s.index));
    let mut completed = !visible & 15;
    paint_groups(svg, &shape, &groups, 1.0, &mut completed);
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
        let d = path_data(&rrect::polygon(&side_polygon(
            shape, side.index, *completed,
        )));
        let c = side.color;
        let _ = writeln!(
            svg,
            "<path d=\"{d}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{}\"/>",
            c.r,
            c.g,
            c.b,
            alpha / next
        );
        *completed |= 1 << side.index;
    }
    if layer {
        svg.push_str("</g>");
    }
}

fn side_polygon(shape: &Shape, side: usize, completed: u8) -> Vec<(f32, f32)> {
    let (x, y, w, h) = shape.rect;
    let [t, r, b, l] = shape.widths;
    let depth = shape.widths[side];
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
        if completed & (1 << adjacent) != 0 {
            points = rrect::keep_side(&points, corners[corner], reference);
        }
    }
    points
}

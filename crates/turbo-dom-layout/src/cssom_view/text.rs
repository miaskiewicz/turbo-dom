//! Document caret candidates use actual native font/run boxes, then source queries.
use crate::live_layout_scroll::{scroll_translation, SourceStyles};
use std::collections::BTreeMap;
use turbo_dom_layout::layout_snapshot::{LayoutSnapshot, Point, Rect, SourceId};

fn distance(point: [f64; 2], rect: Rect) -> [f64; 2] {
    let gap = |value: f64, start: f32, length: f32| {
        (f64::from(start) - value)
            .max(0.0)
            .max(value - f64::from(start + length))
    };
    [
        gap(point[1], rect.y, rect.height),
        gap(point[0], rect.x, rect.width),
    ]
}
pub(crate) fn world_rect(rect: Rect, matrix: [f64; 6]) -> Rect {
    let [a, b, c, d, e, f] = matrix;
    let mut xs = [0.0; 4];
    let mut ys = [0.0; 4];
    for (index, (x, y)) in [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ]
    .into_iter()
    .enumerate()
    {
        xs[index] = (a * f64::from(x) + c * f64::from(y) + e) as f32;
        ys[index] = (b * f64::from(x) + d * f64::from(y) + f) as f32;
    }
    let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
    let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
    Rect {
        x: min(xs),
        y: min(ys),
        width: max(xs) - min(xs),
        height: max(ys) - min(ys),
    }
}
pub(crate) fn caret(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<Option<(SourceId, usize, Rect)>, String> {
    let elements = crate::live_layout_hit::elements_at_point(snapshot, styles, offsets, point)?;
    let Some(&owner) = elements.first() else {
        return Ok(None);
    };
    if styles.get(&owner).is_some_and(|style| style.tag == "svg") {
        return Err("native SVG caret hit testing is unsupported".into());
    }
    let mut best = None;
    let mut examined = 0usize;
    for run in snapshot.text_runs() {
        if !run.source_ancestors.contains(&owner) {
            continue;
        }
        let Some(source) = run.source_id else {
            continue;
        };
        if run.source_ancestors.last().is_some_and(|id| {
            crate::live_layout_hit::property(styles, *id, "visibility")
                .is_some_and(|value| value != "visible")
        }) {
            continue;
        }
        examined = examined
            .checked_add(run.glyphs.len())
            .ok_or("native caret query size overflow")?;
        if examined > 1_000_000 {
            return Err("native caret query exceeds glyph budget".into());
        }
        let fragment = &snapshot.fragments()[run.fragment_index];
        let Some(local) = run_rect(snapshot, run) else {
            continue;
        };
        let translation = scroll_translation(snapshot, styles, source, offsets)?;
        if crate::live_layout_hit::clipped(snapshot, styles, &run.source_ancestors, offsets, point)?
        {
            continue;
        }
        let mut rect = world_rect(
            local,
            snapshot
                .native_hit_transform(fragment.index)
                .ok_or("native caret transform unavailable")?,
        );
        rect.x -= translation[0];
        rect.y -= translation[1];
        let gap = distance(point, rect);
        if best.as_ref().is_none_or(
            |(_, previous, _, _): &(SourceId, [f64; 2], usize, [f32; 2])| {
                gap[0] < previous[0] || (gap[0] == previous[0] && gap[1] < previous[1])
            },
        ) {
            best = Some((source, gap, run.fragment_index, translation));
        }
    }
    let Some((source, _, _, translation)) = best else {
        return Ok(None);
    };
    let query = Point {
        x: point[0] as f32 + translation[0],
        y: point[1] as f32 + translation[1],
    };
    let offset = snapshot
        .text_caret_offset_from_point(source, query)
        .map_err(|error| error.to_string())?;
    let boxes = snapshot
        .text_range_boxes(source, offset, offset)
        .map_err(|error| error.to_string())?;
    let rect = boxes
        .into_iter()
        .map(|cell| cell.transformed_bounds)
        .min_by(|a, b| {
            let a = distance([f64::from(query.x), f64::from(query.y)], *a);
            let b = distance([f64::from(query.x), f64::from(query.y)], *b);
            a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
        })
        .ok_or("native caret edge is unplaced")?;
    Ok(Some((
        source,
        offset,
        Rect {
            x: rect.x - translation[0],
            y: rect.y - translation[1],
            ..rect
        },
    )))
}

pub(crate) fn run_rect(
    snapshot: &LayoutSnapshot,
    run: &turbo_dom_layout::layout_snapshot::SnapshotTextRun,
) -> Option<Rect> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for glyph in &run.glyphs {
        let cluster = glyph.source;
        left = left.min(cluster.pen_x.min(cluster.pen_x + cluster.advance));
        right = right.max(cluster.pen_x.max(cluster.pen_x + cluster.advance));
    }
    if !left.is_finite() || !right.is_finite() {
        return None;
    }
    let fragment = &snapshot.fragments()[run.fragment_index];
    let font = run.font_box;
    Some(Rect {
        x: fragment.rect.x + left,
        y: fragment.rect.y + font.baseline - font.ascent,
        width: right - left,
        height: font.ascent + font.descent,
    })
}

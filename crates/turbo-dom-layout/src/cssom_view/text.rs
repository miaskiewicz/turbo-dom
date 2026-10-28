//! Document caret candidates use actual native font/run boxes, then source queries.
use super::scroll::{scroll_translation, SourceStyles};
use crate::layout_snapshot::{LayoutSnapshot, Point, Rect, SourceId};
use std::collections::BTreeMap;

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
#[must_use]
pub fn world_rect(rect: Rect, matrix: [f64; 6]) -> Rect {
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
pub fn caret(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    offsets: &BTreeMap<SourceId, [f64; 2]>,
    point: [f64; 2],
) -> Result<Option<(SourceId, usize, Rect)>, String> {
    let elements = super::hit::elements_at_point(snapshot, styles, offsets, point)?;
    let Some(&owner) = elements.first() else {
        return Ok(None);
    };
    if styles.get(&owner).is_some_and(|style| style.tag == "svg") {
        return Err("native SVG caret hit testing is unsupported".into());
    }
    let mut best = None;
    // No glyph budget here: a snapshot captures at most 1,000,000 clusters in all its
    // runs (`layout_snapshot::text::TextCapture::record`, else the snapshot fails with
    // its text budget error), and one glyph per cluster.
    for run in snapshot.text_runs() {
        if !run.source_ancestors.contains(&owner) {
            continue;
        }
        let Some(source) = run.source_id else {
            continue;
        };
        // visibility:hidden text still takes the caret, like Chrome 156
        // (miaskiewicz/turbo-dom#126).
        let fragment = &snapshot.fragments()[run.fragment_index];
        let Some(local) = run_rect(snapshot, run) else {
            continue;
        };
        let translation = scroll_translation(snapshot, styles, source, offsets)?;
        if super::hit::hidden_by_clip(
            snapshot,
            styles,
            (&run.source_ancestors, None),
            offsets,
            point,
        )? {
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

#[must_use]
pub fn run_rect(
    snapshot: &LayoutSnapshot,
    run: &crate::layout_snapshot::SnapshotTextRun,
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

#[cfg(test)]
#[cfg(feature = "bundled-fonts")]
mod tests {
    use super::*;
    use crate::cssom_view::scroll::chrome::{case, close, rect as chrome_rect, Case};
    use crate::live::{source_id, testing::DOCUMENT};
    use serde_json::Value;

    /// The text node (or element) Chrome's caret readout names.
    fn chrome_node(case: &Case, want: &Value) -> SourceId {
        let tree = &case.tree;
        let cid = want["cid"].as_str().unwrap();
        let mut owner = tree
            .query_selector(&format!("[data-cid=\"{cid}\"]"))
            .unwrap();
        for step in want["path"].as_array().unwrap() {
            owner = tree.children(owner)[step.as_u64().unwrap() as usize];
        }
        match want["child"].as_u64() {
            Some(child) => source_id(DOCUMENT, tree.children(owner)[child as usize]),
            None => source_id(DOCUMENT, owner),
        }
    }

    /// miaskiewicz/turbo-dom#127: a point beside a transformed line (Chrome descends with
    /// the untransformed layout position), a point in an element without text (Chrome:
    /// a caret in the element) and a point in the gap below a block (Chrome: the next
    /// block's first line).
    const GAPS: &[[f64; 2]] = &[[50.0, 85.0], [40.0, 105.0], [5.0, 13.0]];

    fn caret_failures(id: &str, select: impl Fn([f64; 2]) -> bool) -> Vec<String> {
        let case = case(id);
        let offsets = case.offsets();
        let mut failures = Vec::new();
        let mut compared = 0;
        for want in case.reads("carets") {
            let point = [want["x"].as_f64().unwrap(), want["y"].as_f64().unwrap()];
            if !select(point) {
                continue;
            }
            compared += 1;
            if want.get("cid").is_none() {
                // Chrome: no caret (the point hits nothing).
                let got = caret(&case.snapshot, &case.styles, &offsets, point).unwrap();
                if got.is_some() {
                    failures.push(format!("{point:?}: {got:?}, Chrome null"));
                }
                continue;
            }
            if want["child"].is_null() {
                failures.push(format!("{point:?}: Chrome {want} is an element caret"));
                continue;
            }
            let got = caret(&case.snapshot, &case.styles, &offsets, point).unwrap();
            let chrome = chrome_rect(&want["rect"]);
            let expected = (
                chrome_node(&case, want),
                want["offset"].as_u64().unwrap() as usize,
            );
            match got {
                Some((source, offset, r))
                    if (source, offset) == expected
                        && close(r.x, chrome[0])
                        && close(r.y, chrome[1])
                        && close(r.height, chrome[3]) => {}
                got => failures.push(format!("{point:?}: {got:?}, Chrome {want}")),
            }
        }
        assert!(compared > 0);
        failures
    }

    #[test]
    fn caret_positions_from_points_match_chrome_156() {
        let mut failures = caret_failures("caret-from-point", |point| !GAPS.contains(&point));
        failures.extend(caret_failures("caret-wrap-and-markers", |_| true));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Until miaskiewicz/turbo-dom#127: an element without text answers no text caret.
    #[test]
    fn an_element_without_text_has_no_text_caret() {
        let case = case("caret-from-point");
        let empty = case.bounds("e");
        let point = [f64::from(empty.x) + 40.0, f64::from(empty.y) + 5.0];
        assert_eq!(
            caret(&case.snapshot, &case.styles, &case.offsets(), point).unwrap(),
            None
        );
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The axis-aligned bounds of a transformed rect, as `getBoundingClientRect()` bounds a
    /// transformed box.
    #[test]
    fn world_rect_bounds_every_transformed_corner() {
        let r = rect(1.0, 2.0, 3.0, 4.0);
        assert_eq!(
            world_rect(r, [2.0, 0.5, 0.25, 3.0, 10.0, 20.0]),
            rect(12.5, 26.5, 7.0, 13.5)
        );
        assert_eq!(
            world_rect(r, [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
            rect(-6.0, 1.0, 4.0, 3.0)
        );
        assert_eq!(world_rect(r, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]), r);
    }

    /// `[vertical, horizontal]` gaps from a point to a rect (0 inside or on an edge).
    #[test]
    fn distance_is_the_per_axis_gap_outside_the_rect() {
        let r = rect(10.0, 20.0, 30.0, 40.0);
        assert_eq!(distance([15.0, 30.0], r), [0.0, 0.0]);
        assert_eq!(distance([4.0, 30.0], r), [0.0, 6.0]);
        assert_eq!(distance([47.0, 30.0], r), [0.0, 7.0]);
        assert_eq!(distance([15.0, 12.0], r), [8.0, 0.0]);
        assert_eq!(distance([15.0, 65.0], r), [5.0, 0.0]);
        assert_eq!(distance([40.0, 60.0], r), [0.0, 0.0]);
        assert_eq!(distance([1.0, 2.0], r), [18.0, 9.0]);
    }

    /// A run's rect is its text's content area: Chrome's client rect of a Range over the
    /// whole Text node.
    #[test]
    fn run_rects_match_chrome_156_text_client_rects() {
        let case = case("range-client-rects");
        let all = &case.reads("ranges")[0]["rects"];
        let got: Vec<Rect> = case
            .snapshot
            .text_runs()
            .iter()
            .map(|run| run_rect(&case.snapshot, run).unwrap())
            .collect();
        // [r: 0..4] lists div a, "abc", div b, "de", "fg" (the span's box is the same
        // rect, listed once), "hi", div x.
        let chrome: Vec<[f32; 4]> = [1, 3, 4, 5].map(|i| chrome_rect(&all[i])).into();
        assert_eq!(got.len(), chrome.len());
        for (r, c) in got.iter().zip(&chrome) {
            assert!(
                close(r.x, c[0])
                    && close(r.y, c[1])
                    && close(r.width, c[2])
                    && close(r.height, c[3]),
                "{r:?}, Chrome {c:?}"
            );
        }
    }

    #[test]
    fn svg_carets_are_unsupported() {
        let (_, snapshot, styles) = crate::live::testing::layout(
            "<body style='margin:0'><svg width=50 height=50><text y=20>a</text></svg></body>",
            "",
            (800.0, 600.0),
            Some(1.0),
        );
        assert_eq!(
            caret(&snapshot, &styles, &BTreeMap::new(), [10.0, 10.0]).unwrap_err(),
            "native SVG caret hit testing is unsupported"
        );
    }

    #[test]
    #[ignore = "turbo-dom#127: fails until the fix"]
    fn carets_beside_transformed_lines_and_in_empty_elements_match_chrome_156() {
        let failures = caret_failures("caret-from-point", |point| GAPS.contains(&point));
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

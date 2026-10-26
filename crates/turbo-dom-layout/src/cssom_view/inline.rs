//! A bounded plain-inline seam over genuine source-owned native font/run boxes.
//! Decorations and empty inline placement need actual inline-border production.
use crate::live_layout_metrics::{ClientMetrics, ElementGeometry, OffsetSize};
use crate::live_layout_scroll::SourceStyles;
use std::collections::BTreeMap;
use turbo_dom_layout::layout_snapshot::{LayoutSnapshot, Rect, SourceId};

fn check_style(styles: &SourceStyles, id: SourceId) -> Result<(), String> {
    let style = &styles
        .get(&id)
        .ok_or("native inline style is missing")?
        .style;
    for property in [
        "padding",
        "padding-left",
        "padding-right",
        "padding-top",
        "padding-bottom",
        "border",
        "border-width",
        "border-left",
        "border-right",
        "border-top",
        "border-bottom",
        "margin",
        "margin-left",
        "margin-right",
        "margin-top",
        "margin-bottom",
    ] {
        if style.get(property).is_some_and(|value| {
            !value
                .split_whitespace()
                .all(|token| matches!(token, "0" | "0px"))
        }) {
            return Err("native decorated inline border production is pending".into());
        }
    }
    for (property, default) in [
        ("position", "static"),
        ("vertical-align", "baseline"),
        ("writing-mode", "horizontal-tb"),
        ("direction", "ltr"),
    ] {
        if style.get(property).is_some_and(|value| value != default) {
            return Err("native positioned/vertical inline border production is pending".into());
        }
    }
    Ok(())
}
fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect {
        x,
        y,
        width: (a.x + a.width).max(b.x + b.width) - x,
        height: (a.y + a.height).max(b.y + b.height) - y,
    }
}
pub(crate) fn geometry(
    snapshot: &LayoutSnapshot,
    styles: &SourceStyles,
    id: SourceId,
) -> Result<ElementGeometry, String> {
    check_style(styles, id)?;
    let own = &styles[&id].style;
    let mut lines = BTreeMap::<(u32, [u64; 6]), (Rect, [f64; 6])>::new();
    let mut font = None;
    for run in snapshot
        .text_runs()
        .iter()
        .filter(|run| run.source_ancestors.contains(&id))
    {
        let start = run
            .source_ancestors
            .iter()
            .position(|source| *source == id)
            .unwrap();
        for ancestor in &run.source_ancestors[start..] {
            check_style(styles, *ancestor)?;
            let child = &styles[ancestor].style;
            for property in [
                "font-family",
                "font-size",
                "font-weight",
                "font-style",
                "line-height",
            ] {
                if own.get(property) != child.get(property) {
                    return Err("native mixed-font inline border production is pending".into());
                }
            }
        }
        if font.is_some_and(|font| font != run.font_box) {
            return Err("native fallback-font inline border production is pending".into());
        }
        font = Some(run.font_box);
        let Some(rect) = crate::live_layout_text::run_rect(snapshot, run) else {
            continue;
        };
        let fragment = &snapshot.fragments()[run.fragment_index];
        let matrix = snapshot
            .native_world_transform(fragment.index)
            .ok_or("native inline transform unavailable")?;
        let key = (fragment.rect.y.to_bits(), matrix.map(f64::to_bits));
        lines
            .entry(key)
            .and_modify(|(old, _)| *old = union(*old, rect))
            .or_insert((rect, matrix));
    }
    if lines.is_empty() {
        return Err("native empty inline placement is pending".into());
    }
    let raw: Vec<_> = lines.values().map(|(rect, _)| *rect).collect();
    let rects: Vec<_> = lines
        .values()
        .map(|(rect, matrix)| crate::live_layout_text::world_rect(*rect, *matrix))
        .collect();
    let bounds = crate::live_layout_metrics::bounding_box(&rects);
    let local = crate::live_layout_metrics::bounding_box(&raw);
    Ok(ElementGeometry {
        bounds,
        rects,
        client: ClientMetrics {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        },
        offset: OffsetSize {
            width: local.width.round() as i32,
            height: local.height.round() as i32,
        },
    })
}

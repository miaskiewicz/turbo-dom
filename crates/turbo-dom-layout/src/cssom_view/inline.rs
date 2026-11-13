//! A bounded plain-inline seam over genuine source-owned native font/run boxes.
//! Decorations and empty inline placement need actual inline-border production.
use super::metrics::{ClientMetrics, ElementGeometry, OffsetSize};
use super::scroll::SourceStyles;
use crate::layout_snapshot::{LayoutSnapshot, Rect, SourceId};
use std::collections::BTreeMap;

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
fn check_inline_font_consistency(
    styles: &SourceStyles,
    own: &crate::ComputedStyle,
    run: &crate::layout_snapshot::SnapshotTextRun,
    id: SourceId,
) -> Result<(), String> {
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
    Ok(())
}

pub fn geometry(
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
        check_inline_font_consistency(styles, own, run, id)?;
        if font.is_some_and(|font| font != run.font_box) {
            return Err("native fallback-font inline border production is pending".into());
        }
        font = Some(run.font_box);
        let Some(rect) = super::text::run_rect(snapshot, run) else {
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
        .map(|(rect, matrix)| super::text::world_rect(*rect, *matrix))
        .collect();
    let bounds = super::metrics::bounding_box(&rects);
    let local = super::metrics::bounding_box(&raw);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::testing;

    #[cfg(feature = "bundled-fonts")]
    fn read(body: &str) -> Result<ElementGeometry, String> {
        let html = format!("<body style='margin:0;font-family:\"Liberation Serif\"'>{body}</body>");
        let (tree, snapshot, styles) = testing::layout(&html, "", (400.0, 300.0), Some(1.0));
        geometry(&snapshot, &styles, testing::id(&tree, "#t"))
    }

    // Text in the bundled Liberation Serif.
    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn decorated_positioned_and_vertical_inlines_are_explicit_gaps() {
        const DECORATED: &str = "native decorated inline border production is pending";
        const POSITIONED: &str = "native positioned/vertical inline border production is pending";
        for (style, error) in [
            ("padding-left:2px", DECORATED),
            ("border:1px solid", DECORATED),
            ("margin:0 3px", DECORATED),
            ("position:relative", POSITIONED),
            ("vertical-align:top", POSITIONED),
            ("writing-mode:vertical-rl", POSITIONED),
            ("direction:rtl", POSITIONED),
        ] {
            let got = read(&format!("<p>a <span id=t style='{style}'>b</span></p>"));
            assert_eq!(got.unwrap_err(), error, "{style}");
        }
        // Zero decorations are plain.
        let plain =
            read("<p>a <span id=t style='padding:0;margin:0px 0;border-width:0'>b</span></p>");
        assert_eq!(plain.unwrap().rects.len(), 1);
        // A decorated descendant fails its ancestor's read too.
        let nested = read("<p><span id=t>a <b style='padding:1px'>b</b></span></p>");
        assert_eq!(nested.unwrap_err(), DECORATED);
    }

    // Text in the bundled Liberation Serif.
    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn mixed_fonts_and_empty_inlines_are_explicit_gaps() {
        for property in [
            "font-size:20px",
            "font-weight:700",
            "font-style:italic",
            "line-height:30px",
            "font-family:Inter",
        ] {
            let got = read(&format!(
                "<p><span id=t>a <i style='{property}'>b</i></span></p>"
            ));
            assert_eq!(
                got.unwrap_err(),
                "native mixed-font inline border production is pending",
                "{property}"
            );
        }
        assert_eq!(
            read("<p>a<span id=t></span>b</p>").unwrap_err(),
            "native empty inline placement is pending"
        );
    }

    /// miaskiewicz/turbo-dom#386, follow-up to #377: the live inline y of a web-font line
    /// at DPR 2 (`LiveOptions { layout_scale: 2.0 }`) matches Chrome 156's. 20px Fira Code
    /// has an odd content height (25 CSS px) at DPR 2, so the line's top lands at a
    /// fractional CSS px; Chrome snaps its `getBoundingClientRect` / `getClientRects` y to
    /// the device-pixel grid, and the binding must do the same.
    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn webfont_inline_y_at_dpr_2_matches_chrome_156() {
        use crate::text::FontFace;
        use crate::FontRegistry;
        let fira = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/fira-code/FiraCode-Regular.ttf"
        ))
        .unwrap();
        let mut fonts = FontRegistry::new();
        fonts.add(FontFace::from_web_font_bytes(fira, "Fira Code", 400, false).unwrap());
        let css = "body{margin:0;font-family:\"Fira Code\";\
            font-size:20px;line-height:25px}";
        let html = "<!doctype html><div data-cid=\"r\"><div data-cid=\"a\">abc</div>\
            <div data-cid=\"b\">de<span id=t data-cid=\"s\">fg</span>hi</div></div>";
        let (tree, snapshot, styles) = crate::live::testing::layout_with_fonts(
            html,
            css,
            (1280.0, 800.0),
            Some(2.0),
            |_| {},
            &fonts,
        );
        let id = crate::live::source_id(
            crate::live::testing::DOCUMENT,
            tree.query_selector("#t").unwrap(),
        );
        let got = geometry(&snapshot, &styles, id).unwrap();
        // Chrome 156 `cssom_view_chrome156_dpr2.json`: span s bounds = [24.6171875, 25.0,
        // 24.6171875, 24.5] (one rect). Line-height 25px aligns the line box with the
        // font's 24.5-CSS-px content box at DPR 2, so the inline y is the line-box top
        // snapped to the device-pixel grid.
        assert!(
            (got.bounds.y - 25.0).abs() <= 0.01,
            "inline y {} vs Chrome 25.0",
            got.bounds.y
        );
        assert!(
            (got.bounds.height - 24.5).abs() <= 0.01,
            "inline height {} vs Chrome 24.5",
            got.bounds.height
        );
    }

    #[test]
    fn a_receiver_without_a_style_is_an_error() {
        let html = "<body><p id=t>text</p></body>";
        let (tree, snapshot, styles) = testing::layout(html, "", (400.0, 300.0), Some(1.0));
        let text = tree.children(tree.query_selector("#t").unwrap())[0];
        let id = crate::live::source_id(testing::DOCUMENT, text);
        assert_eq!(
            geometry(&snapshot, &styles, id).unwrap_err(),
            "native inline style is missing"
        );
    }
}

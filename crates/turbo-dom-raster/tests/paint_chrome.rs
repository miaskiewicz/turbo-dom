//! Screenshot pixels against Chrome 156 (P5b-W5).
//!
//! The control is `tests/fixtures/paint_chrome156.json` (turbo-dom's
//! `scripts/paint-chrome-control.mjs`): pages over a `#00ffff` backdrop with Chrome's colour
//! class (R/G/B/Y/W/T = backdrop/`?`) at each case's probe points and the
//! `getBoundingClientRect()` of every `[data-cid]` element. Each case is screenshot here
//! (its `<img src="data:…">` bytes supplied as fetched assets keyed by the raw `src`) and
//! laid out through turbo-dom-layout's `layout_boxes` (the same static layout the
//! screenshot paints, plus the zero-area `clip: rect()` boxes it leaves out:
//! miaskiewicz/turbo-dom#75).
//! - `caption-*`: miaskiewicz/turbo-dom#51, table captions in the table wrapper box; the
//!   table's background and border paint the grid box only.
//! - `clip-*`: miaskiewicz/turbo-dom#52, an absolutely positioned box's `clip: rect()`
//!   clips it and its descendants (it keeps its border box); a zero-area clip paints
//!   nothing (its box still has its border box, #75); `clip` does not apply to other boxes.
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};
use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, ImageAssets, Viewport};

fn class([r, g, b]: [u8; 3]) -> char {
    let hi = |v: u8| v > 160;
    let lo = |v: u8| v < 96;
    if hi(r) && hi(g) && hi(b) {
        'W'
    } else if lo(r) && hi(g) && hi(b) {
        'T'
    } else if hi(r) && lo(g) && lo(b) {
        'R'
    } else if lo(r) && hi(g) && lo(b) {
        'G'
    } else if lo(r) && lo(g) && hi(b) {
        'B'
    } else if hi(r) && hi(g) && lo(b) {
        'Y'
    } else {
        '?'
    }
}

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/paint_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

fn assets(case: &serde_json::Value) -> ImageAssets {
    let mut assets: ImageAssets = HashMap::new();
    for src in case["assets"].as_array().unwrap() {
        let src = src.as_str().unwrap();
        assets.insert(src.to_string(), decode_data_url(src).unwrap().bytes);
    }
    assets
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

/// Every difference from Chrome of the cases `select` picks: probe colour classes of the
/// screenshot and the `layout_boxes` geometry.
fn differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let png =
            screenshot_png_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("png");
        let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds");
            let got = class([c.red(), c.green(), c.blue()]);
            let want = probe["class"].as_str().unwrap();
            if got.to_string() != want {
                failures.push(format!("{id} ({x},{y}): {got}, Chrome {want}"));
            }
        }
        let mut diags = Diagnostics::default();
        let boxes = layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).unwrap();
        for want in case["boxes"].as_array().unwrap() {
            let cid = want["cid"].as_str().unwrap();
            let Some(b) = boxes.iter().find(|b| b.cid == cid) else {
                failures.push(format!("{id}: no box for data-cid={cid}"));
                continue;
            };
            for (field, got) in [
                ("x", b.x),
                ("y", b.y),
                ("width", b.width),
                ("height", b.height),
            ] {
                let chrome = want[field].as_f64().unwrap() as f32;
                if (got - chrome).abs() > 0.01 {
                    failures.push(format!("{id}: {cid}.{field} = {got}, Chrome {chrome}"));
                }
            }
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// Caption cases that also need another fix: `caption-narrow`'s `width` is the table's
/// border box (miaskiewicz/turbo-dom#62, [`tables_are_border_box_sized_like_chrome_156`]).
fn pending_case(id: &str) -> bool {
    id == "caption-narrow"
}

#[test]
fn table_captions_paint_outside_the_grid_like_chrome_156() {
    let failures = differences(|id| id.starts_with("caption-") && !pending_case(id));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn tables_are_border_box_sized_like_chrome_156() {
    let failures = differences(pending_case);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn clip_rect_clips_the_box_and_its_descendants_like_chrome_156() {
    let failures = differences(|id| id.starts_with("clip-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The painter's box decoration paths (P5-cov): a rounded border, a gradient in a rounded
/// box, unblurred / collapsed / spread shadows, transparent text.
#[test]
fn box_decorations_paint_like_chrome_156() {
    let failures = differences(|id| id.starts_with("paint-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The PNG and (resvg-rasterized) SVG screenshot's raw colour at each probe of `id`.
fn probe_rgb(id: &str) -> Vec<([u8; 3], [u8; 3], [u8; 3])> {
    let fixture = fixture();
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap();
    let html = case["html"].as_str().unwrap();
    let png = screenshot_png_with_opts(html, "", VIEWPORT, &assets(case), false, false).unwrap();
    let png = tiny_skia::Pixmap::decode_png(&png).unwrap();
    let svg = screenshot_svg_with_opts(html, "", VIEWPORT, &assets(case), false, false).unwrap();
    let raster = rasterize_svg(&svg);
    case["probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|probe| {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let p = png.pixel(x, y).unwrap();
            let s = raster.pixel(x, y).unwrap().demultiply();
            let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
            (
                [p.red(), p.green(), p.blue()],
                [s.red(), s.green(), s.blue()],
                chrome,
            )
        })
        .collect()
}

/// A half-transparent background and shadow composite over the backdrop like Chrome's in
/// both painters (the SVG one as `fill-opacity`), exactly.
#[test]
fn translucent_colours_composite_like_chrome_156() {
    for (png, svg, chrome) in probe_rgb("paint-alpha") {
        assert_eq!(png, chrome, "png");
        assert_eq!(svg, chrome, "svg");
    }
}

/// A blurred shadow of a box far wider than the viewport (9000px) paints like Chrome's
/// (miaskiewicz/turbo-dom#95: it was skipped past an 8192px scratch).
#[test]
fn wide_box_shadows_paint_like_chrome_156() {
    let failures = differences(|id| id == "shadow-wide");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A spread enlarges a shadow's corner radius by the spread scaled by 1 + (r/s - 1)^3 below
/// it (CSS Backgrounds 3 §7.1): a square box's shadow stays square, in both painters (found
/// by the P5-cov mutation pass).
#[test]
fn shadow_spread_radii_follow_css_like_chrome_156() {
    let mut failures = differences(|id| id == "shadow-spread-small-radius");
    failures.extend(svg_differences(|id| id == "shadow-spread-small-radius"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Chrome paints every shadow blur from 256px up identically (the control's 256px and 400px
/// cases have equal pixels); so does the raster (miaskiewicz/turbo-dom#95).
#[test]
fn shadow_blur_is_capped_at_256px_like_chrome_156() {
    let (at256, at400) = (probe_rgb("shadow-blur-256"), probe_rgb("shadow-blur-400"));
    for ((png256, svg256, chrome256), (png400, svg400, chrome400)) in at256.into_iter().zip(at400) {
        assert_eq!(chrome256, chrome400, "the control");
        assert_eq!(png256, png400, "png");
        assert_eq!(svg256, svg400, "svg");
    }
    let failures = differences(|id| id.starts_with("shadow-blur-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The SVG screenshot rasterized by resvg. This resvg build has no `raster-images` decoders
/// (turning them on in the dev-dependency would also change turbo-dom-layout's rasterizer
/// under test), so each embedded PNG `data:` image is first re-embedded as an SVG of its
/// pixels (one rect per run of equal pixels, crisp edges) — the same image, drawn the same
/// way into its `<image>` box.
fn rasterize_svg(svg: &str) -> resvg::tiny_skia::Pixmap {
    use resvg::{tiny_skia as rsk, usvg};
    use std::fmt::Write;
    const PNG: &str = "href=\"data:image/png;base64,";
    let mut svg = svg.to_string();
    while let Some(at) = svg.find(PNG) {
        let start = at + "href=\"".len();
        let end = start + svg[start..].find('"').unwrap();
        let png = decode_data_url(&svg[start..end]).unwrap().bytes;
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        let mut pixels = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{0}' height='{1}' viewBox='0 0 {0} {1}' preserveAspectRatio='none' shape-rendering='crispEdges'>",
            img.width(),
            img.height()
        );
        for y in 0..img.height() {
            let mut x = 0;
            while x < img.width() {
                let p = img.get_pixel(x, y).0;
                let run = (x..img.width())
                    .take_while(|&e| img.get_pixel(e, y).0 == p)
                    .count() as u32;
                let _ = write!(
                    pixels,
                    "<rect x='{x}' y='{y}' width='{run}' height='1' fill='rgb({},{},{})' fill-opacity='{}'/>",
                    p[0],
                    p[1],
                    p[2],
                    f32::from(p[3]) / 255.0
                );
                x += run;
            }
        }
        pixels.push_str("</svg>");
        let escaped = pixels.replace('<', "&lt;").replace('>', "&gt;");
        svg.replace_range(start..end, &format!("data:image/svg+xml;utf8,{escaped}"));
    }
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid svg");
    let mut pm = rsk::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    pm
}

/// Every probe difference from Chrome of the vector screenshot of the cases `select` picks,
/// rasterized by resvg (the SVG painter emits the same boxes, borders, shadows, gradients,
/// transforms and clips as the raster one, as SVG).
fn svg_differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let svg =
            screenshot_svg_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("svg");
        let pm = rasterize_svg(&svg);
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds").demultiply();
            let got = class([c.red(), c.green(), c.blue()]);
            let want = probe["class"].as_str().unwrap();
            if got.to_string() != want {
                failures.push(format!("{id} ({x},{y}): svg {got}, Chrome {want}"));
            }
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// A rounded box's border follows its radius in the vector screenshot too: the corners
/// outside the radius stay the backdrop (miaskiewicz/turbo-dom#95).
#[test]
fn svg_rounded_borders_follow_the_radius_like_chrome_156() {
    let failures = svg_differences(|id| id == "paint-round-border");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The vector screenshot of every case, rasterized, against the same Chrome probes.
#[test]
fn svg_screenshots_paint_like_chrome_156() {
    let failures = svg_differences(|id| !pending_case(id));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The vector screenshot clips a `clip: rect()` box with a `<clipPath>` of its region and
/// leaves out a box whose region is empty (miaskiewicz/turbo-dom#52).
#[test]
fn svg_screenshot_clips_with_a_clip_path() {
    let fixture = fixture();
    let svg = |id: &str| {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let html = case["html"].as_str().unwrap();
        screenshot_svg_with_opts(html, "", VIEWPORT, &assets(case), false, false).expect("svg")
    };
    let partial = svg("clip-partial");
    assert_eq!(partial.matches("<clipPath").count(), 1, "{partial}");
    assert!(
        partial.contains(r#"<rect x="10.00" y="10.00" width="30.00" height="30.00"/></clipPath>"#)
    );
    assert!(partial.contains(r##"fill="#ff0000""##));
    let empty = svg("clip-zero-area-auto");
    assert!(
        !empty.contains("<clipPath") && !empty.contains(r##"fill="#ff0000""##),
        "{empty}"
    );
}

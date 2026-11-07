//! Screenshot pixels against Chrome 156 (P5b-W5).
//!
//! The control is `tests/fixtures/paint_chrome156.json` (turbo-dom's
//! `scripts/paint-chrome-control.mjs`): pages over a `#00ffff` backdrop with Chrome's colour
//! class (R/G/B/Y/W/T = backdrop/`?`) and raw RGB at each case's probe points and the
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
//! - `radius-*`: miaskiewicz/turbo-dom#253, the full `border-radius` model.
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};
use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, ImageAssets, Viewport};

fn class([r, g, b]: [u8; 3]) -> char {
    // Classify a pixel into a colour class by whether each channel is bright (>160) or
    // dark (<96). The match form keeps cyclomatic complexity low (one match, no chained
    // boolean operators) while preserving the original six-way classification.
    match (r > 160, g > 160, b > 160, r < 96, g < 96, b < 96) {
        (true, true, true, _, _, _) => 'W',
        (_, true, true, true, _, _) => 'T',
        (true, _, _, _, true, true) => 'R',
        (_, true, _, true, _, true) => 'G',
        (_, _, true, true, true, _) => 'B',
        (true, true, _, _, _, true) => 'Y',
        _ => '?',
    }
}

/// The widest per-channel difference from Chrome's RGB a probe outside the six classes
/// (`?`: a grey, a dark, a mixed colour) may have.
const RGB_TOLERANCE: u8 = 8;

/// Probes that sit on an anti-aliased edge, with the per-channel tolerance each needs:
/// (case, x, y, tolerance). Their RGB is compared with Chrome's within it, whatever
/// class the edge pixel falls in; every other `?` probe matches Chrome's RGB within
/// [`RGB_TOLERANCE`].
/// - A radio button's 1px ring: Chrome's circle covers about three quarters of these
///   pixels (its grey blends with the white inside and the backdrop outside), ours all
///   of it.
/// - `overflow-glyph-ink-*` (0.7.0 round-2 review, finding 3; miaskiewicz/turbo-dom#266):
///   edge pixels of the 40px "gy". The PNG painter grows the outline by a box and lifts
///   it like Chrome's CoreText smoothing, with a luminance-dependent edge transfer
///   (`src/glyph_raster.rs`, #331); the SVG one emits the same grown outline with Skia's
///   pre-blend as a filter (#294; resvg anti-aliases it in quarter steps). Measured green
///   channel (red and blue match), Chrome / PNG / SVG: (22,25) 46 / 49 / 53 (7, the SVG;
///   11 before #331, 17 before #294), (22,44) 118 / 104 / 150 (32, the SVG's quarter
///   steps; 41 before #294), (28,44) 139 / 163 / 134 (24, PNG: the `g` bar sits about
///   0.2 px higher in Chrome than in the outline, a glyph-specific vertical offset no
///   outline offset reproduces, while the 40px `H`, `I` and `l` match; the previous
///   painter measured 154, tolerance 15, and before #266 the bare outline missed this
///   pixel, 255, tolerance 116). The
///   descender's left part is also pinned by the solid-ink probes (23,42) and (24,44),
///   compared by class; the glyph-wide residual is measured in
///   `tests/glyph_coverage_chrome.rs`.
const ANTIALIASED_EDGES: &[(&str, u32, u32, u8)] = &[
    ("form-native-radio", 26, 20, 36),
    ("form-native-radio", 20, 26, 36),
    ("form-native-radio", 32, 26, 36),
    ("form-native-radio", 26, 32, 36),
    ("form-native-radio-checked", 26, 20, 45),
    ("overflow-glyph-ink-hidden", 22, 25, 7),
    ("overflow-glyph-ink-visible", 22, 25, 7),
    ("overflow-glyph-ink-visible", 22, 44, 32),
    ("overflow-glyph-ink-visible", 28, 44, 24),
    // The 90% thumb's (centre 67.6, 28; radius 7) top arc crosses this pixel: measured
    // rgb(0,126,255) against Chrome's rgb(0,117,255), PNG and SVG (round-2 review #9).
    ("form-native-range-step-max", 66, 21, 9),
    // The blue top / grey left mitre diagonal halves the corner pixel (#283): Chrome
    // blends it rgb(84,84,211), exactly half; the painters' coverage of the diagonal
    // measures rgb(105,105,200), PNG and SVG.
    ("author-border-button-top-color", 20, 20, 21),
    ("author-border-button-top-color-bg", 20, 20, 21),
];

/// Why `got` (the screenshot's RGB at `probe`) differs from Chrome's, if it does: a
/// different colour class, or, at a `?` probe, a channel more than the probe's tolerance
/// from Chrome's stored RGB.
fn probe_difference(id: &str, probe: &serde_json::Value, got: [u8; 3]) -> Option<String> {
    let (x, y) = (
        probe["x"].as_u64().unwrap() as u32,
        probe["y"].as_u64().unwrap() as u32,
    );
    let want = probe["class"].as_str().unwrap();
    let edge = ANTIALIASED_EDGES
        .iter()
        .find(|&&(case, ex, ey, _)| case == id && (ex, ey) == (x, y))
        .map(|e| e.3);
    let class = class(got);
    if class.to_string() != want && edge.is_none() {
        return Some(format!("({x},{y}): {class} {got:?}, Chrome {want}"));
    }
    if want != "?" && edge.is_none() {
        return None;
    }
    let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
    let tolerance = edge.unwrap_or(RGB_TOLERANCE);
    let far = got
        .iter()
        .zip(chrome)
        .any(|(&g, c)| g.abs_diff(c) > tolerance);
    far.then(|| format!("({x},{y}): {got:?}, Chrome {chrome:?} (tolerance {tolerance})"))
}

#[test]
fn unclassed_probes_compare_chromes_rgb_within_a_tolerance() {
    let probe = serde_json::json!({"x": 1, "y": 2, "class": "?", "rgb": [100, 100, 100]});
    assert_eq!(probe_difference("c", &probe, [108, 92, 100]), None);
    assert!(probe_difference("c", &probe, [109, 100, 100]).is_some());
    assert!(probe_difference("c", &probe, [100, 100, 91]).is_some());
    // A classed pixel is never a `?` match.
    assert!(probe_difference("c", &probe, [0, 0, 255]).is_some());
    let blue = serde_json::json!({"x": 1, "y": 2, "class": "B", "rgb": [0, 0, 255]});
    assert_eq!(probe_difference("c", &blue, [20, 20, 200]), None);
    assert!(probe_difference("c", &blue, [100, 100, 100]).is_some());
    // An anti-aliased edge compares the RGB within its tolerance, across classes.
    let edge = serde_json::json!({"x": 22, "y": 25, "class": "B", "rgb": [0, 46, 255]});
    assert_eq!(
        probe_difference("overflow-glyph-ink-visible", &edge, [0, 53, 255]),
        None
    );
    assert!(probe_difference("overflow-glyph-ink-visible", &edge, [0, 54, 255]).is_some());
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
            if let Some(d) = probe_difference(id, probe, [c.red(), c.green(), c.blue()]) {
                failures.push(format!("{id} {d}"));
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

/// miaskiewicz/turbo-dom#238: the screenshot paints bidi-reordered runs — an override's
/// atoms reversed, an RTL paragraph's atoms right to left, reordered inline boxes with
/// their backgrounds.
#[test]
fn bidi_reordered_runs_paint_like_chrome_156() {
    let failures = differences(|id| id.starts_with("bidi-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#244: an inline box paints its background, padding and border per
/// line fragment (`box-decoration-break: slice`): the start edge on its first fragment,
/// the end edge on its last, also when bidi reordering splits it; its padding, border and
/// margin push its neighbours. `layout_boxes` gives the union of its fragments. The SVG
/// screenshot is checked by [`svg_screenshots_paint_like_chrome_156`].
#[test]
fn inline_box_decorations_paint_per_fragment_like_chrome_156() {
    let failures = differences(|id| id.starts_with("inline-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#248: `background-size` / `-position` / `-repeat` / `-origin` /
/// `-clip` place and tile a block's background image or gradient like Chrome (the inline
/// case is `inline-bg-position`). SVG: [`svg_screenshots_paint_like_chrome_156`].
#[test]
fn background_layers_are_sized_positioned_and_tiled_like_chrome_156() {
    let failures = differences(|id| id.starts_with("bg-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#264: gradient colour stops at lengths, hard and double-position
/// stops, and repeating gradients paint like Chrome, in both painters.
#[test]
fn gradient_stops_paint_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("gradient-"));
    failures.extend(svg_differences(|id| id.starts_with("gradient-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#256: the `background` shorthand and its longhands cascade by
/// origin, `!important`, specificity and order: an author shorthand beats the UA button's
/// `background-color`, the later declaration wins, and a shorthand resets every longhand
/// (image, position, size, repeat, origin, clip) an earlier rule set.
#[test]
fn background_shorthand_and_longhands_cascade_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("cascade-bg-"));
    failures.extend(svg_differences(|id| id.starts_with("cascade-bg-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#81: the screenshot's initial containing block is its viewport.
#[test]
fn fixed_and_icb_boxes_anchor_to_the_viewport_like_chrome_156() {
    let failures = differences(|id| id.starts_with("icb-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#252: `<canvas>`, `<video>`, `<audio controls>` and `<object>` are
/// replaced boxes painting their own background and border, in horizontal and vertical
/// writing modes, and a `<video>` its poster.
#[test]
fn media_boxes_paint_their_frame_and_poster_like_chrome_156() {
    let failures = differences(|id| id.starts_with("media-"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#239: form controls paint their field / button background at
/// Chrome's sizes.
#[test]
fn form_controls_paint_their_fields_like_chrome_156() {
    let failures = differences(|id| id.starts_with("form-"));
    assert!(
        failures.is_empty(),
        "{}",
        failures.join(
            "
"
        )
    );
}

/// miaskiewicz/turbo-dom#255: a control with its default appearance paints Chrome's
/// native look (frames, menu-list chevron, checkbox / radio glyphs, range track and
/// thumb, colour swatch) in the PNG and SVG screenshots; `appearance: none` or an author
/// border / background keeps the CSS painting.
#[test]
fn form_controls_paint_chrome_native_appearance() {
    let mut failures = differences(|id| id.starts_with("form-native-"));
    failures.extend(svg_differences(|id| id.starts_with("form-native-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#283: any author border declaration (the shorthand, a width equal
/// to the UA's, a style, a colour, one side's colour, `border: none`) on a `<button>` or an
/// `<input>` button type turns its native appearance off: the CSS border and background
/// paint, with and without an author background, in the PNG and SVG screenshots.
#[test]
fn an_author_border_turns_a_buttons_native_appearance_off_like_chrome_156() {
    let pick = |id: &str| id.starts_with("author-border-");
    let mut failures = differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#303: a border without a colour, or with `currentcolor` spelled
/// out (shorthand, `border-color`, per side), paints in the element's `color` (its own,
/// inherited, the initial black, a button's buttontext; an `outset` one shaded from it), in
/// the PNG and SVG screenshots.
#[test]
fn a_currentcolor_border_paints_in_the_elements_color_like_chrome_156() {
    let pick = |id: &str| id.starts_with("currentcolor-");
    let mut failures = differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#257: `inset` / `outset` borders shade their sides like Chrome
/// (darkened top and left of an inset border, lightened bottom and right; a colour too
/// dark to darken lightened instead), in the PNG and SVG screenshots.
#[test]
fn inset_and_outset_borders_shade_their_sides_like_chrome_156() {
    let pick = |id: &str| id.starts_with("border-inset-") || id.starts_with("border-outset-");
    let mut failures = differences(pick);
    failures.extend(svg_differences(pick));
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
/// under test), so each embedded PNG `data:` image element is first replaced by an inline
/// `<svg>` of its pixels (one rect per run of equal pixels, crisp edges) with the same box
/// — the same image, drawn the same way; inline, so a `<use>`d image is parsed once.
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
            "viewBox='0 0 {0} {1}' shape-rendering='crispEdges'>",
            img.width(),
            img.height()
        );
        // Identical rows share their rects (one rect per run, as tall as the rows): an
        // image drawn smaller than its pixels then has no seams between rows.
        let row = |y: u32| {
            (0..img.width())
                .map(|x| img.get_pixel(x, y).0)
                .collect::<Vec<_>>()
        };
        let mut y = 0;
        while y < img.height() {
            let rows = (y..img.height()).take_while(|&r| row(r) == row(y)).count() as u32;
            let mut x = 0;
            while x < img.width() {
                let p = img.get_pixel(x, y).0;
                let run = (x..img.width())
                    .take_while(|&e| img.get_pixel(e, y).0 == p)
                    .count() as u32;
                let _ = write!(
                    pixels,
                    "<rect x='{x}' y='{y}' width='{run}' height='{rows}' fill='rgb({},{},{})' fill-opacity='{}'/>",
                    p[0],
                    p[1],
                    p[2],
                    f32::from(p[3]) / 255.0
                );
                x += run;
            }
            y += rows;
        }
        pixels.push_str("</svg>");
        let open = svg[..at].rfind("<image ").unwrap();
        let close = end + svg[end..].find("/>").unwrap() + 2;
        let attrs = svg[open + "<image ".len()..at].to_string();
        svg.replace_range(open..close, &format!("<svg {attrs}{pixels}"));
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
            if let Some(d) = probe_difference(id, probe, [c.red(), c.green(), c.blue()]) {
                failures.push(format!("{id} svg {d}"));
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
    // `img-scaled-4x` (miaskiewicz/turbo-dom#269) embeds the source image as it is; how
    // smoothly it is drawn four times larger is the viewer's filter, and
    // [`rasterize_svg`] draws embedded images crisp by construction.
    let failures = svg_differences(|id| !pending_case(id) && id != "img-scaled-4x");
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

/// Inset shadows paint inside the padding box, over the background and under the border,
/// with their offset, spread (following the padding-box radius) and blur, in both painters
/// (miaskiewicz/turbo-dom#192: they were skipped).
#[test]
fn inset_box_shadows_paint_inside_the_padding_box_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("inset-"));
    failures.extend(svg_differences(|id| id.starts_with("inset-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Chrome 156's full `border-radius` model (miaskiewicz/turbo-dom#253: it was one scalar):
/// per-corner, elliptical and percentage radii, the overlap scale-down, inner radii,
/// logical corners under RTL and vertical writing modes, and the radius clipping
/// backgrounds (with #248's layers), borders with per-side widths and colours, shadows
/// and `overflow` — in both painters.
#[test]
fn border_radii_paint_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("radius-"));
    failures.extend(svg_differences(|id| id.starts_with("radius-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Round-1 review of 0.7.0, finding 7: `overflow: hidden` clips glyph ink past the line box
/// (a 40px "gy" in a 10px-high line) in both painters; unclipped, it reaches below the box.
#[test]
fn overflow_clips_glyph_ink_past_the_line_box_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("overflow-glyph-ink-"));
    failures.extend(svg_differences(|id| id.starts_with("overflow-glyph-ink-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Round-1 review of 0.7.0, finding 18: `box-decoration-break: clone` repeats a multi-line
/// inline box's border radius, background image and inset shadow on every fragment, in
/// both painters.
#[test]
fn cloned_inline_decorations_paint_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("inline-clone-"));
    failures.extend(svg_differences(|id| id.starts_with("inline-clone-")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// 0.7.0 round-3 review, finding 7: a pattern of more than 4096 tiles without a
/// whole-pixel period (2.5px tiles snap to alternating 3px and 2px) paints the same
/// snapped tiles in both painters: the vector screenshot, rasterized, matches the PNG
/// pixel for pixel (an unsnapped `<pattern>` of the exact tile put edges up to 1px off).
#[test]
fn fractional_tile_patterns_paint_alike_in_png_and_svg() {
    let html = r#"<body style="margin:0;background:#0ff"><div style="position:absolute;left:3px;top:5px;width:203px;height:201px;background:#0f0 linear-gradient(to right,#f00 40%,#00f 40%) 0 0/2.5px 2.5px"></div>"#;
    let none = ImageAssets::new();
    let png = screenshot_png_with_opts(html, "", VIEWPORT, &none, false, false).expect("png");
    let png = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let svg = screenshot_svg_with_opts(html, "", VIEWPORT, &none, false, false).expect("svg");
    assert!(
        svg.matches("<rect").count() < 100,
        "one pattern, not one rect per tile"
    );
    let raster = rasterize_svg(&svg);
    let mut differences = Vec::new();
    for y in 0..220 {
        for x in 0..220 {
            let p = png.pixel(x, y).unwrap();
            let s = raster.pixel(x, y).unwrap().demultiply();
            let far = [
                p.red().abs_diff(s.red()),
                p.green().abs_diff(s.green()),
                p.blue().abs_diff(s.blue()),
            ]
            .into_iter()
            .any(|d| d > 2);
            if far {
                differences.push(format!("({x},{y}): png {p:?}, svg {s:?}"));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "{} pixels differ:\n{}",
        differences.len(),
        differences[..differences.len().min(20)].join("\n")
    );
}

/// miaskiewicz/turbo-dom#269: an `<img>` and image tiles under `scale(4)` sample the
/// source at device resolution (their edges blend over a few device px), not a 1x
/// resample stretched. SVG: [`svg_screenshots_paint_like_chrome_156`].
#[test]
fn scaled_images_sample_their_source_at_device_resolution_like_chrome_156() {
    let scaled = |id: &str| id == "img-scaled-4x" || id == "bg-image-tiles-scaled-4x";
    let mut failures = differences(scaled);
    // Every probe, classed or not, within the RGB tolerance of Chrome's: the blend
    // across an edge is the point here.
    for case in fixture()["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !scaled(id) {
            continue;
        }
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
            let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
            let got = [c.red(), c.green(), c.blue()];
            if got
                .iter()
                .zip(chrome)
                .any(|(&g, c)| g.abs_diff(c) > RGB_TOLERANCE)
            {
                failures.push(format!("{id} ({x},{y}): {got:?}, Chrome {chrome:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#270: fractional tiles (2.5px, 1.5px) are drawn once and repeated
/// every fractional step, blending the pixels their edges cross, like Chrome, in both
/// painters.
#[test]
fn fractional_tiles_blend_their_edges_like_chrome_156() {
    let mut failures = differences(|id| id.starts_with("bg-tiles-fractional-blend"));
    failures.extend(svg_differences(|id| {
        id.starts_with("bg-tiles-fractional-blend")
    }));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// miaskiewicz/turbo-dom#272: a `transform` on an `<img>` itself scales the painted image
/// and its `getBoundingClientRect()` (through `layout_boxes`) like Chrome. SVG:
/// [`svg_screenshots_paint_like_chrome_156`].
#[test]
fn a_transformed_img_paints_and_measures_like_chrome_156() {
    let failures = differences(|id| id == "img-transformed");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

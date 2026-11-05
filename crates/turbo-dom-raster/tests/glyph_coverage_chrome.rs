//! Glyph coverage against Chrome 156 (miaskiewicz/turbo-dom#266, #294).
//!
//! The control is `tests/fixtures/glyph_coverage_chrome156.json` (turbo-dom's
//! `scripts/glyph-coverage-chrome-control.mjs`): Chrome's pixels around a line of fixture
//! Liberation Serif, `text-*` at 12-64px in three colour pairs and `edge-*`, a 64px `l` /
//! `-` at Skia's four subpixel pen phases. Each case is screenshot here and compared
//! pixel by pixel (the largest channel difference) over the pixels either engine inks.
//!
//! Chrome's glyphs come from CoreText with font smoothing, which this painter models as
//! an outline grown by a `0.0045 * size` px box and lifted, with a luminance- and
//! size-dependent edge transfer (`src/glyph_raster.rs`, #331). The model is not
//! CoreText's exact rasterizer, so the
//! limits below are the measured residual (each a little above it), not zero: a change
//! that moves glyph edges away from Chrome fails here. The SVG screenshot (rasterized by
//! resvg) emits the same outline with the pre-blend as an `feComponentTransfer` filter
//! (#294) and is held to its own measured residual.
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, Viewport};

/// The comparison of one case: the mean and largest channel difference over the
/// pixels either engine inks.
#[derive(Debug)]
struct Residual {
    mean: f64,
    max: u8,
    inked: usize,
}

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/glyph_coverage_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

/// The PNG screenshot of `html`.
fn png(html: &str) -> tiny_skia::Pixmap {
    let png =
        screenshot_png_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).expect("png");
    tiny_skia::Pixmap::decode_png(&png).expect("decode")
}

/// The SVG screenshot of `html`, rasterized by resvg (these pages have no images).
fn svg(html: &str) -> tiny_skia::Pixmap {
    use resvg::usvg;
    let svg =
        screenshot_svg_with_opts(html, "", VIEWPORT, &HashMap::new(), false, false).expect("svg");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("valid svg");
    let mut pm = resvg::tiny_skia::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pm.as_mut(),
    );
    let mut out = tiny_skia::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    out.data_mut().copy_from_slice(pm.data());
    out
}

fn residual(case: &serde_json::Value, render: fn(&str) -> tiny_skia::Pixmap) -> Residual {
    let crop: Vec<u32> = case["crop"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    let chrome = decode_data_url(&format!(
        "data:application/octet-stream;base64,{}",
        case["rgb"].as_str().unwrap()
    ))
    .unwrap()
    .bytes;
    let ours = render(case["html"].as_str().unwrap());
    let backdrop = &chrome[..3];
    let (mut sum, mut max, mut inked) = (0u64, 0u8, 0);
    for y in 0..crop[3] {
        for x in 0..crop[2] {
            let at = ((y * crop[2] + x) * 3) as usize;
            let want = &chrome[at..at + 3];
            let p = ours.pixel(crop[0] + x, crop[1] + y).unwrap();
            let got = [p.red(), p.green(), p.blue()];
            if want == backdrop && got == backdrop {
                continue;
            }
            let d = got
                .iter()
                .zip(want)
                .map(|(g, w)| g.abs_diff(*w))
                .max()
                .unwrap();
            inked += 1;
            sum += u64::from(d);
            max = max.max(d);
        }
    }
    Residual {
        mean: sum as f64 / inked.max(1) as f64,
        max,
        inked,
    }
}

/// Each case's limits `(mean, max)`: its measured residual (rounded up a little), with
/// the previous model's limits (before #331: a disc offset with Skia's gamma-1.8
/// pre-blend) and the bare-outline painter's residual (before #266) in the comment.
/// Below about 20px Chrome's vertical geometry follows the size's pixel grid, which no
/// outline offset does, so 12-16px keep the largest residual; the stem / bar phases
/// show the x and y edge responses are alike.
const LIMITS: &[(&str, f64, u8)] = &[
    ("text-12-000000", 11.0, 62), // before #331: 12.7 / 69; before #266: mean 28.5, max 98
    ("text-12-0000ff", 11.0, 62), // before #331: 13.2 / 71; 28.5, 98
    ("text-12-ffffff", 14.2, 74), // before #331: 20.5 / 83; 57.4, 149
    ("text-16-000000", 14.4, 85), // before #331: 16.5 / 86; 31.6, 138
    ("text-16-0000ff", 14.4, 84), // before #331: 16.8 / 84; 31.6, 138
    ("text-16-ffffff", 15.9, 102), // before #331: 21.3 / 112; 59.3, 202
    ("text-20-000000", 6.3, 58),  // before #331: 7.8 / 60; 24.4, 98
    ("text-20-0000ff", 6.3, 58),  // before #331: 8.1 / 62; 24.4, 98
    ("text-20-ffffff", 8.8, 60),  // before #331: 13.3 / 65; 52.1, 145
    ("text-32-000000", 5.1, 66),  // before #331: 6.4 / 66; 27.6, 146
    ("text-32-0000ff", 5.1, 66),  // before #331: 6.5 / 68; 27.6, 146
    ("text-32-ffffff", 7.0, 70),  // before #331: 10.4 / 74; 48.4, 204
    ("text-40-000000", 5.7, 87),  // before #331: 6.4 / 98; 32.7, 186
    ("text-40-0000ff", 5.7, 87),  // before #331: 6.5 / 97; 32.7, 186
    ("text-40-ffffff", 7.4, 91),  // before #331: 11.3 / 104; 51.7, 237
    ("text-64-000000", 4.0, 65),  // before #331: 5.7 / 74; 29.6, 217
    ("text-64-0000ff", 4.0, 65),  // before #331: 5.8 / 74; 29.6, 217
    ("text-64-ffffff", 4.5, 91),  // before #331: 6.8 / 92; 43.1, 249
    ("edge-l-0", 2.5, 18),        // before #331: 4.7 / 35; 23.3, 176
    ("edge-l-0.25", 2.5, 18),     // before #331: 4.7 / 35; 23.2, 176
    ("edge-l-0.5", 3.5, 19),      // before #331: 6.6 / 38; 22.1, 183
    ("edge-l-0.75", 3.6, 19),     // before #331: 6.6 / 38; 22.0, 183
    ("edge-hyphen-0", 1.5, 13),   // before #331: 6.1 / 29; 27.3, 169
    ("edge-hyphen-0.25", 1.6, 14), // before #331: 7.1 / 33; 26.4, 128
    ("edge-hyphen-0.5", 1.5, 14), // before #331: 6.8 / 32; 26.6, 128
    ("edge-hyphen-0.75", 1.1, 10), // before #331: 5.9 / 26; 26.1, 159
];

/// The SVG screenshot's limits `(mean, max)` per text case: its measured residual
/// (rounded up a little) with the glyph pre-blend filter (#294), and before it in the
/// comment, when its edges kept a plain source-over blend. resvg's anti-aliasing comes in
/// quarter steps, so the vector residual stays above the raster one.
const SVG_LIMITS: &[(&str, f64, u8)] = &[
    ("text-12-000000", 15.8, 60), // before #331: 16.0 / 60; before #294: mean 14.8, max 52
    ("text-12-0000ff", 16.1, 62), // before #331: 16.2 / 62; before #294: mean 14.8, max 52
    ("text-12-ffffff", 20.4, 78), // before #331: 21.0 / 78; before #294: mean 38.3, max 115
    ("text-16-000000", 17.5, 83), // before #331: 18.2 / 87; before #294: mean 17.6, max 86
    ("text-16-0000ff", 17.7, 85), // before #331: 18.3 / 89; before #294: mean 17.6, max 86
    ("text-16-ffffff", 20.2, 100), // before #331: 21.7 / 100; before #294: mean 38.6, max 150
    ("text-20-000000", 9.9, 54),  // before #331: 10.5 / 54; before #294: mean 10.1, max 44
    ("text-20-0000ff", 10.1, 56), // before #331: 10.6 / 56; before #294: mean 10.1, max 44
    ("text-20-ffffff", 13.7, 72), // before #331: 14.3 / 72; before #294: mean 31.7, max 103
    ("text-32-000000", 6.6, 57),  // before #331: 7.4 / 57; before #294: mean 8.0, max 57
    ("text-32-0000ff", 6.7, 59),  // before #331: 7.4 / 59; before #294: mean 8.0, max 57
    ("text-32-ffffff", 9.9, 84),  // before #331: 11.0 / 84; before #294: mean 25.6, max 121
    ("text-40-000000", 6.4, 78),  // before #331: 7.7 / 78; before #294: mean 9.0, max 84
    ("text-40-0000ff", 6.3, 77),  // before #331: 7.5 / 77; before #294: mean 9.0, max 84
    ("text-40-ffffff", 12.0, 100), // before #331: 13.4 / 100; before #294: mean 26.7, max 137
    ("text-64-000000", 5.0, 69),  // before #331: 5.0 / 80; before #294: mean 4.4, max 72
    ("text-64-0000ff", 5.1, 70),  // before #331: 5.1 / 81; before #294: mean 4.4, max 72
    ("text-64-ffffff", 5.9, 82),  // before #331: 6.5 / 87; before #294: mean 14.6, max 133
];

/// Every case `select` picks whose residual from `render` exceeds its `limits`.
fn failures(
    select: impl Fn(&str) -> bool,
    render: fn(&str) -> tiny_skia::Pixmap,
    limits: &[(&str, f64, u8)],
) -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let r = residual(case, render);
        println!("{id}: {r:?}");
        assert!(r.inked > 20, "{id}: nothing inked");
        let &(_, mean, max) = limits
            .iter()
            .find(|l| l.0 == id)
            .unwrap_or_else(|| panic!("{id}: no limit"));
        if r.mean > mean || r.max > max {
            failures.push(format!("{id}: {r:?}, limit mean {mean} max {max}"));
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// Text at 12-64px in black, blue and white within its measured residual.
#[test]
fn glyph_edges_cover_like_chrome_156() {
    let failures = failures(|id| id.starts_with("text-"), png, LIMITS);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A 64px stem (x edges) and bar (y edges) at Skia's four subpixel pen phases within
/// their measured residual.
#[test]
fn stem_and_bar_edges_cover_like_chrome_156_at_every_subpixel_phase() {
    let failures = failures(|id| id.starts_with("edge-"), png, LIMITS);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The SVG screenshot's text in black, blue and white within its measured residual: its
/// glyph edges take the raster painter's colour-dependent pre-blend through a filter.
#[test]
fn svg_glyph_edges_take_the_pre_blend_like_chrome_156() {
    let failures = failures(|id| id.starts_with("text-"), svg, SVG_LIMITS);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

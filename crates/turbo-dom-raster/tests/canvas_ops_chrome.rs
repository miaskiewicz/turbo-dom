//! 2D-canvas op-log replay against Chrome 156 (P5-cov).
//!
//! The control is `tests/fixtures/canvas_ops_chrome156.json` (turbo-dom's
//! `scripts/canvas-ops-chrome-control.mjs`): op logs in the render tier's `ctx._ops` tuple
//! format that Chrome for Testing 156 replayed into a real `getContext('2d')`, with
//! `getImageData` at each probe point. Each log is replayed here through
//! [`canvas_ops_rgba`] (the straight-alpha bytes `getImageData` returns) and
//! [`canvas_ops_png`] (the same pixels, PNG-encoded): fill-style colour syntaxes,
//! save/restore and every transform op, paths (closed and open, rects, arcs in both
//! directions, a bare `lineTo`), `strokeRect`, `clearRect` under a rotation and
//! `globalAlpha`.
use turbo_dom_raster::{canvas_ops_png, canvas_ops_rgba};

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/canvas_ops_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

/// Every probe of the cases `select` picks whose replayed RGBA (or PNG pixel) is not
/// Chrome's exactly.
fn differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        let (w, h) = (
            case["size"][0].as_u64().unwrap() as u32,
            case["size"][1].as_u64().unwrap() as u32,
        );
        let ops = case["ops"].to_string();
        let rgba = canvas_ops_rgba(w, h, &ops).expect("rgba");
        assert_eq!(
            rgba.len(),
            (w * h * 4) as usize,
            "{id}: tightly packed RGBA"
        );
        let png = canvas_ops_png(w, h, &ops).expect("png");
        let png = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(png.dimensions(), (w, h), "{id}: png size");
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let at = ((y * w + x) * 4) as usize;
            let got = &rgba[at..at + 4];
            let want: Vec<u8> = serde_json::from_value(probe["rgba"].clone()).unwrap();
            let near = |a: &[u8]| a == want.as_slice();
            if !near(got) {
                failures.push(format!("{id} ({x},{y}): rgba {got:?}, Chrome {want:?}"));
            }
            let px = png.get_pixel(x, y).0;
            if !near(&px) {
                failures.push(format!("{id} ({x},{y}): png {px:?}, Chrome {want:?}"));
            }
        }
    }
    failures
}

#[test]
fn op_logs_replay_like_chrome_156() {
    let failures = differences(|id| {
        !["fill-style-css-colours", "closed-strokes", "unpremultiply"].contains(&id)
            && id != "global-alpha"
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A stroked closed path strokes its closing edge, and after `closePath` a `lineTo` starts
/// from the closed subpath's first point (found by the P5-cov mutation pass).
#[test]
fn closed_paths_restart_at_their_first_point_like_chrome_156() {
    let failures = differences(|id| id == "closed-strokes");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `hsl()`, percentage `rgb()`, fractional channels and every named colour resolve like
/// Chrome's `fillStyle` (miaskiewicz/turbo-dom#95).
#[test]
fn css_colour_syntaxes_fill_like_chrome_156() {
    let failures = differences(|id| id == "fill-style-css-colours");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `getImageData`'s straight alpha: each premultiplied channel is divided by the alpha the
/// way Chrome's (Skia's) pipeline does it, in single precision with a round-to-even, over a
/// 16 x 16 grid of colour / alpha pairs and the translucent `global-alpha` cases (found by
/// the P5-cov mutation pass).
#[test]
fn straight_alpha_readback_rounds_like_chrome_156() {
    let failures = differences(|id| id == "unpremultiply" || id == "global-alpha");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

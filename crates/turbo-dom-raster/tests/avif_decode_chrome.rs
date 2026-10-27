//! AVIF screenshot pixels against Chrome 156 over every YUV path of `src/avif.rs` (P5-cov).
//!
//! The control is `tests/fixtures/avif_decode_chrome156.json` (turbo-dom's
//! `scripts/raster-avif-chrome-control.mjs`) over the AVIFs of
//! `scripts/make_raster_avif_fixtures.py` (`tests/fixtures/images/avif`): the 64x40
//! red / green / blue / yellow quadrant image as 4:4:4 identity (lossless), 4:4:4 BT.709
//! limited range, 4:2:2 BT.601, 4:2:0 FCC, SMPTE 240M and BT.2020 (limited), and without an
//! `nclx` box (the AV1 sequence header's colour description applies), plus 10-bit limited and
//! 12-bit full range. Chrome's raw colour at the four quadrant centres equals the
//! screenshot's exactly for the 8-bit images (within 2 levels for the deep ones). Without the `avif`
//! feature each image still lays out at its size and paints as the light-grey placeholder.
use std::collections::HashMap;

use turbo_dom_layout::image::decode_data_url;
use turbo_dom_raster::{screenshot_png_with_opts, ImageAssets, Viewport};

#[test]
fn avif_yuv_paths_match_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/avif_decode_chrome156.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut assets: ImageAssets = HashMap::new();
    for case in cases {
        let src = case["src"].as_str().unwrap();
        assets.insert(src.to_string(), decode_data_url(src).unwrap().bytes);
    }
    let viewport = Viewport {
        width: 1280,
        height: 800,
    };
    let html = fixture["html"].as_str().unwrap();
    let png = screenshot_png_with_opts(html, "", viewport, &assets, false, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        assert_eq!(case["natural"], serde_json::json!([64, 40]), "{id}");
        let chrome: Vec<[u8; 3]> = serde_json::from_value(case["rgb"].clone()).unwrap();
        for (point, want) in case["points"].as_array().unwrap().iter().zip(chrome) {
            let (x, y) = (
                point[0].as_u64().unwrap() as u32,
                point[1].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds");
            let got = [c.red(), c.green(), c.blue()];
            let want = if cfg!(feature = "avif") {
                want
            } else {
                [220, 220, 220]
            };
            // 8-bit images equal Chrome exactly. Chrome converts 10/12-bit images through
            // its own fixed-point pipeline (not H.273's exact arithmetic, which the raster
            // implements — `avif::tests::deep_limited_range_is_the_eight_bit_range_shifted`):
            // a deep quadrant may differ by up to 2 levels.
            let slack = if cfg!(feature = "avif") && id.contains("bit-") {
                2
            } else {
                0
            };
            if got.iter().zip(&want).any(|(g, w)| g.abs_diff(*w) > slack) {
                failures.push(format!("{id} ({x},{y}): {got:?}, want {want:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

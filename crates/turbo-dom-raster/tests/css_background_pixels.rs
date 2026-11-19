//! Square CSS backgrounds snap their paint bounds while layout remains fractional.
use sha2::{Digest, Sha256};
use turbo_dom_raster::{screenshot_png_with_options, ImageAssets, ScreenshotOptions, Viewport};

#[test]
fn css_background_pixels_match_chromium_159() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/css_background_pixels_chrome159.json"
    ))
    .unwrap();
    assert_eq!(fixture["browser"], "159.0.8154.6");
    assert_eq!(
        fixture["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/css-background-pixels-chrome-control.mjs"
            ))
        )
    );
    for case in fixture["cases"].as_array().unwrap() {
        let png = screenshot_png_with_options(
            case["html"].as_str().unwrap(),
            "",
            Viewport {
                width: 8,
                height: 8,
            },
            &ImageAssets::new(),
            &ScreenshotOptions::default(),
        )
        .unwrap();
        let actual = image::load_from_memory(&png).unwrap().to_rgba8();
        let expected: Vec<u8> = serde_json::from_value(case["rgba"].clone()).unwrap();
        assert_eq!(actual.as_raw(), &expected, "{}", case["id"]);
    }
}

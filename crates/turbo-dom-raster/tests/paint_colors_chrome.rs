//! Functional border colors and inherited inline SVG colors retain the document cascade.
use sha2::{Digest, Sha256};
use turbo_dom_raster::{screenshot_png_with_options, ImageAssets, ScreenshotOptions, Viewport};

#[test]
fn border_colors_match_chromium_159() {
    check_cases("border-");
}

#[test]
fn inline_svg_colors_match_chromium_159() {
    check_cases("svg-");
}

fn check_cases(prefix: &str) {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/paint_colors_chrome159.json")).unwrap();
    assert_eq!(fixture["browser"], "159.0.8154.6");
    assert_eq!(
        fixture["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/paint-colors-chrome-control.mjs"
            ))
        )
    );
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["id"].as_str().unwrap().starts_with(prefix))
    {
        let html = case["html"].as_str().unwrap();
        let png = screenshot_png_with_options(
            case["html"].as_str().unwrap(),
            case["css"].as_str().unwrap(),
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
        let tree = turbo_dom_layout::parse_html_tree(html);
        let nodes = tree.query_selector_all("*");
        let div = tree.query_selector("div").unwrap();
        let index = nodes.iter().position(|node| *node == div).unwrap();
        let rects = turbo_dom_raster::screenshot_element_rects(
            html,
            case["css"].as_str().unwrap(),
            Viewport {
                width: 8,
                height: 8,
            },
            &ImageAssets::new(),
            &ScreenshotOptions::default(),
            &[index],
        )
        .unwrap();
        let rect = rects[0].unwrap();
        let expected_rect: Vec<f32> = serde_json::from_value(case["rect"].clone()).unwrap();
        assert_eq!(
            [rect.x, rect.y, rect.width, rect.height].as_slice(),
            expected_rect,
            "{} geometry",
            case["id"]
        );
    }
}

use turbo_dom_raster::{screenshot_png_with_options, ImageAssets, ScreenshotOptions, Viewport};

#[test]
fn translucent_solid_border_regions_match_chrome() {
    let controls: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/border_corners_chrome159.json")).unwrap();
    for case in controls["cases"].as_array().unwrap() {
        let png = screenshot_png_with_options(
            case["html"].as_str().unwrap(),
            "",
            Viewport {
                width: 16,
                height: 16,
            },
            &ImageAssets::new(),
            &ScreenshotOptions::default(),
        )
        .unwrap();
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        for point in case["pixels"].as_array().unwrap() {
            let expected: Vec<u8> = point["rgba"]
                .as_array()
                .unwrap()
                .iter()
                .map(|channel| u8::try_from(channel.as_u64().unwrap()).unwrap())
                .collect();
            let x = u32::try_from(point["x"].as_u64().unwrap()).unwrap();
            let y = u32::try_from(point["y"].as_u64().unwrap()).unwrap();
            assert_eq!(
                pixels.get_pixel(x, y).0.as_slice(),
                expected,
                "{} at {x},{y}",
                case["id"]
            );
        }
    }
}

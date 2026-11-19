use sha2::{Digest, Sha256};
use turbo_dom_raster::{screenshot_png_with_options, ImageAssets, ScreenshotOptions, Viewport};

fn render(html: &str, transparent: bool) -> Vec<u8> {
    let mut options = ScreenshotOptions::default();
    options.omit_background = transparent;
    screenshot_png_with_options(
        html,
        "",
        Viewport {
            width: 16,
            height: 16,
        },
        &ImageAssets::new(),
        &options,
    )
    .unwrap()
}

#[test]
fn mixed_border_chrome_controls_check_colors_and_coverage_semantics() {
    let controls: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mixed_border_chrome159.json")).unwrap();
    assert_eq!(controls["chrome"], "159.0.8154.6");
    assert_eq!(controls["viewport"], serde_json::json!([16, 16]));
    assert_eq!(controls["cases"].as_array().unwrap().len(), 13);
    assert_eq!(
        controls["collectorSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/mixed-border-chrome-control.mjs"
            ))
        )
    );
    for case in controls["cases"].as_array().unwrap() {
        assert_grid(case);
        let html = case["html"].as_str().unwrap();
        let png = render(html, false);
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(pixels.dimensions(), (16, 16));
        match case["comparison"].as_str().unwrap() {
            "integer-color" => {
                for point in case["pixels"].as_array().unwrap() {
                    let x = u32::try_from(point["x"].as_u64().unwrap()).unwrap();
                    let y = u32::try_from(point["y"].as_u64().unwrap()).unwrap();
                    for (channel, expected) in pixels
                        .get_pixel(x, y)
                        .0
                        .iter()
                        .zip(point["rgba"].as_array().unwrap())
                    {
                        let expected = u8::try_from(expected.as_u64().unwrap()).unwrap();
                        assert!(
                            channel.abs_diff(expected) <= 2,
                            "{} at {x},{y}: {:?} versus {}",
                            case["id"],
                            pixels.get_pixel(x, y),
                            point["rgba"]
                        );
                    }
                }
            }
            "analytic-coverage" => {
                assert_single_opacity(case);
                assert_color_interiors(case, &pixels);
            }
            mode => panic!("unknown comparison mode {mode}"),
        }
    }
}

fn assert_single_opacity(case: &serde_json::Value) {
    let transparent = |html: &str| html.replace("background:white", "background:transparent");
    let actual =
        tiny_skia::Pixmap::decode_png(&render(&transparent(case["html"].as_str().unwrap()), true))
            .unwrap();
    let opaque = tiny_skia::Pixmap::decode_png(&render(
        &transparent(case["opaqueHtml"].as_str().unwrap()),
        true,
    ))
    .unwrap();
    // Applying opacity once scales every premultiplied channel, including coverage.
    // This detects both double alpha and gaps at every edge/corner pixel, without
    // mistaking Skia's approximate curve/quad AA for exact polygon intersection.
    for (pixel, full) in actual
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(opaque.data().as_chunks::<4>().0.iter())
    {
        assert!(pixel[3] <= 128, "{}: alpha {}", case["id"], pixel[3]);
        for (&channel, &full) in pixel.iter().zip(full) {
            let expected = u8::try_from((u16::from(full) * 128 + 127) / 255).unwrap();
            assert!(
                channel.abs_diff(expected) <= 1,
                "{}: {pixel:?} versus {full}",
                case["id"]
            );
        }
    }
}

fn assert_color_interiors(case: &serde_json::Value, pixels: &image::RgbaImage) {
    for point in case["interiorPoints"].as_array().unwrap() {
        let x = u32::try_from(point[0].as_u64().unwrap()).unwrap();
        let y = u32::try_from(point[1].as_u64().unwrap()).unwrap();
        let expected = &case["pixels"][(y * 16 + x) as usize]["rgba"];
        for (&actual, expected) in pixels
            .get_pixel(x, y)
            .0
            .iter()
            .zip(expected.as_array().unwrap())
        {
            assert!(
                actual.abs_diff(u8::try_from(expected.as_u64().unwrap()).unwrap()) <= 2,
                "{} interior at{x},{y}",
                case["id"]
            );
        }
    }
}

fn assert_grid(case: &serde_json::Value) {
    assert_eq!(case["dimensions"], serde_json::json!([16, 16]));
    let points = case["pixels"].as_array().unwrap();
    assert_eq!(points.len(), 256);
    for (index, point) in points.iter().enumerate() {
        assert_eq!(point["x"], index % 16);
        assert_eq!(point["y"], index / 16);
        assert_eq!(point["rgba"].as_array().unwrap().len(), 4);
    }
}

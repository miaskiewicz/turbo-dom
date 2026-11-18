//! Independent Chrome 159 pixel controls: CSS list order, paint phases and ownership.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_raster::{ImageAssets, PreparedScreenshot, ScreenshotOptions, Viewport};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/multiple_box_shadow_chrome159.json")).unwrap()
}

fn svg_pixels(svg: &str) -> resvg::tiny_skia::Pixmap {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
    let size = tree.size().to_int_size();
    let mut pixels = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    pixels
}

fn scene(html: &str) -> PreparedScreenshot {
    PreparedScreenshot::new(
        html,
        "",
        Viewport {
            width: 200,
            height: 160,
        },
        &ImageAssets::new(),
        &ScreenshotOptions::default(),
    )
}

fn check_probes(
    case: &Value,
    pixels: &tiny_skia::Pixmap,
    backend: &str,
    failures: &mut Vec<String>,
) {
    // Solid probes have no colour-class shortcuts. The blurred case uses the
    // existing screenshot oracle's 8/255 RGB allowance for Skia/resvg blur kernels.
    let tolerance = if matches!(
        case["id"].as_str(),
        Some("spread-and-blur" | "partner-original-shadow-list")
    ) {
        8
    } else {
        3
    };
    for probe in case["probes"].as_array().unwrap() {
        let (x, y) = (
            probe["x"].as_u64().unwrap() as u32,
            probe["y"].as_u64().unwrap() as u32,
        );
        let pixel = pixels.pixel(x, y).unwrap().demultiply();
        let actual = [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()];
        let expected: [u8; 4] = serde_json::from_value(probe["rgba"].clone()).unwrap();
        let delta = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| a.abs_diff(b))
            .max()
            .unwrap();
        if delta > tolerance {
            failures.push(format!(
                "{} {backend} {x},{y}: {actual:?} != Chrome {expected:?}",
                case["id"]
            ));
        }
    }
}

#[test]
fn all_layers_paint_in_chrome_order_in_both_public_screenshot_backends() {
    let fixture = fixture();
    let source_text = include_str!("fixtures/multiple_box_shadow_cases.json");
    assert_eq!(
        fixture["source_sha256"],
        format!("{:x}", Sha256::digest(source_text.as_bytes()))
    );
    let source: Value =
        serde_json::from_str(include_str!("fixtures/multiple_box_shadow_cases.json")).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        source["cases"].as_array().unwrap().len(),
        fixture["cases"].as_array().unwrap().len()
    );
    let mut failures = Vec::new();
    for (input, case) in source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fixture["cases"].as_array().unwrap())
    {
        assert_eq!(input["id"], case["id"]);
        assert_eq!(input["html"], case["html"]);
        assert_ne!(case["computed"], "none");
        let scene = scene(case["html"].as_str().unwrap());
        let png = tiny_skia::Pixmap::decode_png(&scene.png().unwrap()).unwrap();
        check_probes(case, &png, "PNG", &mut failures);
        let svg = svg_pixels(&scene.svg().unwrap());
        check_probes(case, &svg, "SVG", &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn immutable_scenes_do_not_share_shadow_ids_or_caller_style_storage() {
    let html = "<!doctype html><body style='margin:0;background:white'><div style='position:absolute;left:30px;top:30px;width:40px;height:40px;background:white;box-shadow:10px 10px red,14px 14px blue'></div>";
    let first = scene(html);
    let original_png = first.png().unwrap();
    let original_svg = first.svg().unwrap();
    let second = scene(&html.replace("14px 14px blue", "14px 14px green"));
    assert_ne!(original_png, second.png().unwrap());
    assert_ne!(original_svg, second.svg().unwrap());
    drop(second);
    assert_eq!(first.png().unwrap(), original_png);
    assert_eq!(first.svg().unwrap(), original_svg);
}

#[test]
fn recording_keeps_full_page_clipped_bounds_and_scaled_masks() {
    use turbo_dom_raster::{ScreenshotMask, ScreenshotRect};
    let html = "<!doctype html><body style='margin:0;background:white'><div style='position:absolute;left:30px;top:30px;width:40px;height:40px;background:white;box-shadow:10px 10px red,14px 14px blue'></div><div style='position:absolute;top:300px;width:2px;height:20px;clip:rect(0,0,0,0);background:lime'></div>";
    let masks = [ScreenshotMask::new(ScreenshotRect {
        x: 80.0,
        y: 80.0,
        width: 5.0,
        height: 5.0,
    })];
    let mut options = ScreenshotOptions::default();
    options.full_page = true;
    options.device_scale = 2.0;
    options.masks = &masks;
    let scene = PreparedScreenshot::new(
        html,
        "",
        Viewport {
            width: 200,
            height: 160,
        },
        &ImageAssets::new(),
        &options,
    );
    let png = tiny_skia::Pixmap::decode_png(&scene.png().unwrap()).unwrap();
    assert_eq!((png.width(), png.height()), (400, 640));
    let svg = svg_pixels(&scene.svg().unwrap());
    assert_eq!((svg.width(), svg.height()), (400, 640));
    for pixels in [&png, &svg] {
        let p = pixels.pixel(164, 164).unwrap().demultiply();
        assert_eq!(
            [p.red(), p.green(), p.blue(), p.alpha()],
            [255, 0, 255, 255]
        );
        let p = pixels.pixel(150, 150).unwrap().demultiply();
        assert_eq!([p.red(), p.green(), p.blue(), p.alpha()], [255, 0, 0, 255]);
    }
}

#[test]
fn offscreen_shadow_extents_never_serialize_nonfinite_mask_geometry() {
    let capture = scene("<!doctype html><body style='margin:0;background:white'><div style='position:absolute;left:3e38px;top:10px;width:3e38px;height:10px;box-shadow:1px 1px red,2px 2px blue'></div>");
    let svg = capture.svg().unwrap();
    assert!(!svg.contains("inf"), "{svg}");
    assert!(!svg.contains("NaN"), "{svg}");
    let png = tiny_skia::Pixmap::decode_png(&capture.png().unwrap()).unwrap();
    let vector = svg_pixels(&svg);
    assert_eq!(png.data(), vector.data());
}

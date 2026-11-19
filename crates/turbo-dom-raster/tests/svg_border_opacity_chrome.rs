use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_raster::{screenshot_svg_with_options, ImageAssets, ScreenshotOptions, Viewport};

fn rasterize(svg: &str) -> tiny_skia::Pixmap {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    let mut pixels = tiny_skia::Pixmap::new(16, 16).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    pixels
}

fn capture(html: &str, transparent: bool) -> String {
    let mut options = ScreenshotOptions::default();
    options.omit_background = transparent;
    screenshot_svg_with_options(
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

fn verify_opacity(case: &Value, white: &tiny_skia::Pixmap, transparent: &tiny_skia::Pixmap) {
    let id = case["id"].as_str().unwrap();
    let ceiling = u8::try_from(
        case["alpha"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .max()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        transparent.pixels().iter().map(|p| p.alpha()).max(),
        Some(ceiling),
        "{id}: side joins compounded opacity"
    );
    for (index, (w, p)) in white.pixels().iter().zip(transparent.pixels()).enumerate() {
        for (actual, foreground) in
            [w.red(), w.green(), w.blue()]
                .into_iter()
                .zip([p.red(), p.green(), p.blue()])
        {
            let expected = foreground + (255 - p.alpha());
            assert!(
                actual.abs_diff(expected) <= 1,
                "{id} at pixel{index}: premultiplied color/alpha scaling {actual} vs {expected}"
            );
        }
    }
    if id.starts_with("common-") {
        assert_eq!(
            transparent.pixel(2, 2).unwrap().alpha(),
            u8::try_from(case["alpha"][34].as_u64().unwrap()).unwrap(),
            "{id}: diagonal seam differs from Chrome"
        );
    }
}

fn verify_rounded_interiors(case: &Value, pixels: &tiny_skia::Pixmap) {
    let id = case["id"].as_str().unwrap();
    // The Chrome harness checks all nine rounded corner/interior probes.
    // resvg uses a different AA rasterizer for diagonal clips, so native RGB
    // comparisons use the straight bands and empty center; opacity and
    // premultiplied color scaling still cover every pixel above.
    for (x, y) in [(6, 1), (1, 6), (12, 6), (6, 12), (6, 6)] {
        let p = pixels.pixel(x, y).unwrap();
        let index = usize::try_from(y * 16 + x).unwrap();
        for (channel, actual) in [p.red(), p.green(), p.blue()].into_iter().enumerate() {
            let expected = u8::try_from(case["rgb"][index][channel].as_u64().unwrap()).unwrap();
            assert!(
                actual.abs_diff(expected) <= 1,
                "{id} at{x},{y}, channel{channel}: {actual} vs {expected}"
            );
        }
    }
}

#[test]
fn translucent_borders_preserve_opacity_color_and_interior_coverage() {
    let controls: Value =
        serde_json::from_str(include_str!("fixtures/svg_border_opacity_chrome159.json")).unwrap();
    assert_eq!(controls["chrome"], "159.0.8154.6");
    assert_eq!(controls["width"], 16);
    assert_eq!(controls["height"], 16);
    assert_eq!(controls["cases"].as_array().unwrap().len(), 11);
    assert_eq!(
        controls["probeSha256"].as_str().unwrap(),
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/svg-border-opacity-chrome-control.mjs"
            ))
        )
    );
    for case in controls["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        assert_eq!(case["rgb"].as_array().unwrap().len(), 256);
        assert_eq!(case["rgba"].as_array().unwrap().len(), 256);
        let html = case["html"].as_str().unwrap();
        let white_svg = capture(html, false);
        let transparent_svg = capture(
            &html.replace("background:white", "background:transparent"),
            true,
        );
        if let Some(dir) = std::env::var_os("SVG_BORDER_PROBE_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            let dir = std::path::Path::new(&dir);
            std::fs::write(dir.join(format!("{id}.svg")), &white_svg).unwrap();
            std::fs::write(dir.join(format!("{id}-transparent.svg")), &transparent_svg).unwrap();
        }
        let white = rasterize(&white_svg);
        let transparent = rasterize(&transparent_svg);
        verify_opacity(case, &white, &transparent);
        if case["mode"] == "rounded" {
            verify_rounded_interiors(case, &white);
        }
    }
}

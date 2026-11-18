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

#[test]
fn translucent_borders_never_multiply_their_opacity_at_side_joins() {
    let controls: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/svg_border_opacity_chrome159.json")).unwrap();
    assert_eq!(
        controls["probeSha256"].as_str().unwrap(),
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/svg-border-opacity-chrome-control.mjs"
            ))
        )
    );
    let mut options = ScreenshotOptions::default();
    let assets = ImageAssets::new();
    for case in controls["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let html = case["html"].as_str().unwrap();
        options.omit_background = false;
        let svg = screenshot_svg_with_options(
            html,
            "",
            Viewport {
                width: 16,
                height: 16,
            },
            &assets,
            &options,
        )
        .unwrap();
        if let Some(dir) = std::env::var_os("SVG_BORDER_PROBE_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(std::path::Path::new(&dir).join(format!("{id}.svg")), &svg).unwrap();
        }
        // Rendering engines approximate antialiased edges differently. Opacity
        // composition has an independent invariant across every pixel: no join
        // exceeds the largest authored side alpha. Chrome supplies the ceiling.
        options.omit_background = true;
        let svg = screenshot_svg_with_options(
            &html.replace("background:white", "background:transparent"),
            "",
            Viewport {
                width: 16,
                height: 16,
            },
            &assets,
            &options,
        )
        .unwrap();
        let pixels = rasterize(&svg);
        let expected = u8::try_from(
            case["alpha"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .max()
                .unwrap(),
        )
        .unwrap();
        let actual = pixels
            .pixels()
            .iter()
            .map(|pixel| pixel.alpha())
            .max()
            .unwrap();
        assert_eq!(actual, expected, "{id}: a border corner compounded opacity");
        if id.starts_with("common-") {
            assert_eq!(
                pixels.pixel(2, 2).unwrap().alpha(),
                expected,
                "{id}: opaque underpainting fills the diagonal seam"
            );
        }
    }
}

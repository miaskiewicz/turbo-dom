use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, ImageAssets, ScreenshotOptions,
    Viewport,
};

#[test]
fn fractional_canvas_background_covers_every_output_pixel() {
    let viewport = Viewport {
        width: 101,
        height: 81,
    };
    let assets = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.device_scale = 1.25;
    for (omit_background, html, expected) in [
        (false, "", [255, 255, 255, 255]),
        (
            false,
            "<html style='background:rgba(255,0,0,0.5)'><body style='margin:0'>",
            [255, 127, 127, 255],
        ),
        (true, "", [0, 0, 0, 0]),
        (
            true,
            "<html style='background:rgba(255,0,0,0.5)'><body style='margin:0'>",
            [255, 0, 0, 128],
        ),
    ] {
        options.omit_background = omit_background;
        let png = screenshot_png_with_options(html, "", viewport, &assets, &options).unwrap();
        let png = image::load_from_memory(&png).unwrap().to_rgba8();
        let svg = screenshot_svg_with_options(html, "", viewport, &assets, &options).unwrap();
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut vector = tiny_skia::Pixmap::new(127, 102).unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut vector.as_mut(),
        );
        assert_eq!(png.dimensions(), (127, 102));
        for (x, y) in [(126, 0), (0, 101), (126, 101), (126, 50), (50, 101)] {
            assert_eq!(png.get_pixel(x, y).0, expected, "PNG at {x},{y}");
            let pixel = vector.pixel(x, y).unwrap().demultiply();
            assert_eq!(
                [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
                expected,
                "SVG at {x},{y} for {html}"
            );
        }
    }
}

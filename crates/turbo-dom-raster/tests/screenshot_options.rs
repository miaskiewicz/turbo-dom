use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, ImageAssets, ScreenshotOptions,
    Viewport,
};

#[test]
fn omitted_canvas_preserves_author_backgrounds() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.omit_background = true;
    for (html, expected) in [
        ("<body style='margin:0'>", [0, 0, 0, 0]),
        ("<body style='margin:0;background:red'>", [255, 0, 0, 255]),
        (
            "<html style='background:rgba(255,0,0,0.5)'><body style='margin:0'>",
            [255, 0, 0, 128],
        ),
    ] {
        let png = screenshot_png_with_options(html, "", viewport, &images, &options).unwrap();
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(pixels.get_pixel(35, 25).0, expected);
        let svg = screenshot_svg_with_options(html, "", viewport, &images, &options).unwrap();
        assert!(!svg.contains("fill=\"rgb(255,255,255)\""));
    }
    options.omit_background = false;
    let png = screenshot_png_with_options("", "", viewport, &images, &options).unwrap();
    assert_eq!(
        image::load_from_memory(&png)
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)
            .0,
        [255; 4]
    );
}

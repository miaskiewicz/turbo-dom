use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, ImageAssets, ScreenshotOptions,
    Viewport,
};

#[test]
fn device_scale_preserves_css_layout_at_integer_and_fractional_scales() {
    let html = "<body style='margin:0'><div style='width:50vw;height:20px;background:red'></div><div style='width:25px;height:10px;background:blue'></div>";
    let viewport = Viewport {
        width: 100,
        height: 80,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    for scale in [1.0, 2.0, 1.5, 0.5] {
        options.device_scale = scale;
        let png = screenshot_png_with_options(html, "", viewport, &images, &options).unwrap();
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(
            pixels.dimensions(),
            ((100.0 * scale) as u32, (80.0 * scale) as u32)
        );
        for (x, y, color) in [
            (10.0, 10.0, [255, 0, 0, 255]),
            (60.0, 10.0, [255; 4]),
            (10.0, 25.0, [0, 0, 255, 255]),
        ] {
            assert_eq!(
                pixels.get_pixel((x * scale) as u32, (y * scale) as u32).0,
                color
            );
        }
        let svg = screenshot_svg_with_options(html, "", viewport, &images, &options).unwrap();
        assert!(svg.contains(&format!(
            "width=\"{}\" height=\"{}\"",
            (100.0 * scale) as u32,
            (80.0 * scale) as u32
        )));
        assert!(svg.contains("viewBox=\"0 0 100 80\""));
        let raster = turbo_dom_layout::svg::rasterize(
            svg.as_bytes(),
            1.0,
            turbo_dom_layout::svg::Oversize::Reject,
        )
        .unwrap();
        assert_eq!((raster.width, raster.height), pixels.dimensions());
        for (x, y) in [(10.0, 10.0), (60.0, 10.0), (10.0, 25.0)] {
            let (x, y) = ((x * scale) as u32, (y * scale) as u32);
            let at = ((y * raster.width + x) * 4) as usize;
            assert_eq!(&raster.rgba[at..at + 4], &pixels.get_pixel(x, y).0);
        }
    }
}

#[test]
fn fractional_dimensions_round_outward_and_invalid_scales_reject() {
    let viewport = Viewport {
        width: 101,
        height: 81,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.device_scale = 1.25;
    options.omit_background = true;
    options.full_page = true;
    let png = screenshot_png_with_options("", "", viewport, &images, &options).unwrap();
    let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(pixels.dimensions(), (127, 102));
    assert_eq!(pixels.get_pixel(126, 101).0, [0; 4]);
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        options.device_scale = scale;
        assert!(screenshot_png_with_options("", "", viewport, &images, &options).is_err());
        assert!(screenshot_svg_with_options("", "", viewport, &images, &options).is_err());
    }
}

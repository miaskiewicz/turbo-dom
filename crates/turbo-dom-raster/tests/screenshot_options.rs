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

#[test]
fn full_page_includes_positioned_overflow_and_viewport_minimum() {
    let viewport = Viewport {
        width: 100,
        height: 80,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.full_page = true;
    for (html, height) in [
        ("<body style='margin:0'>", 80),
        ("<body style='margin:0'><div style='width:20px;height:20px;background:red;transform:translateY(220px)'></div>", 240),
        ("<body style='margin:0'><div style='height:20px;overflow:hidden'><div style='position:relative;top:220px;width:20px;height:20px;background:red'></div></div>", 80),
        ("<body style='margin:0'><div style='position:absolute;top:220px;width:20px;height:20px;background:red'></div>", 240),
        ("<body style='margin:0'><div style='position:relative;top:220px;width:20px;height:20px;background:red'></div>", 240),
    ] {
        let png = screenshot_png_with_options(html, "", viewport, &images, &options).unwrap();
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(pixels.dimensions(), (100, height));
        if height == 240 { assert_eq!(pixels.get_pixel(5,235).0, [255,0,0,255]); }
        let svg = screenshot_svg_with_options(html, "", viewport, &images, &options).unwrap();
        assert!(svg.contains(&format!("height=\"{height}\"")));
    }
}

#[test]
fn full_page_extent_retains_the_safety_cap_and_viewport_capture() {
    let html = "<body style='margin:0'><div style='position:absolute;top:30000px;width:1px;height:20px;background:red'></div>";
    let viewport = Viewport {
        width: 1,
        height: 80,
    };
    let mut options = ScreenshotOptions::default();
    for (full_page, height) in [(false, 80), (true, 24_000)] {
        options.full_page = full_page;
        let png =
            screenshot_png_with_options(html, "", viewport, &ImageAssets::new(), &options).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().height(), height);
    }
}

#[test]
fn full_page_scroll_extent_survives_css_clip_without_painting_clipped_ink() {
    let mut options = ScreenshotOptions::default();
    options.full_page = true;
    let viewport = Viewport {
        width: 100,
        height: 80,
    };
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/full-page-clip-chrome156.json")).unwrap();
    for case in control["cases"].as_array().unwrap() {
        let height = case["height"].as_u64().unwrap() as u32;
        assert_eq!(case["height"], case["scrollHeight"]);
        let scene = turbo_dom_raster::PreparedScreenshot::new(
            case["html"].as_str().unwrap(),
            "",
            viewport,
            &ImageAssets::new(),
            &options,
        );
        let png = scene.png().unwrap();
        let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(pixels.dimensions(), (100, height));
        assert!(pixels.pixels().all(|p| p.0 == [255; 4]));
        assert!(scene
            .svg()
            .unwrap()
            .contains(&format!("height=\"{height}\"")));
    }
}

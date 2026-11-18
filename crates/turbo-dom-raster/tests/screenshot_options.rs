use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, ImageAssets, PreparedScreenshot,
    ScreenshotOptions, Viewport,
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

#[test]
fn propagated_canvas_color_is_painted_once_under_body_layers() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let images = ImageAssets::new();
    for (omit_background, body_color, inside, outside) in [
        (true, "blue", [0, 0, 255, 255], [255, 0, 0, 128]),
        (
            true,
            "rgba(0,0,255,0.5)",
            [85, 0, 170, 192],
            [255, 0, 0, 128],
        ),
        (false, "blue", [0, 0, 255, 255], [255, 127, 127, 255]),
    ] {
        let html = format!(
            "<html style='background:rgba(255,0,0,0.5)'><body style='margin:0;height:10px;background:{body_color}'>"
        );
        let mut options = ScreenshotOptions::default();
        options.omit_background = omit_background;
        let scene = PreparedScreenshot::new(&html, "", viewport, &images, &options);
        let png = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .to_rgba8();
        let svg = scene.svg().unwrap();
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut vector = tiny_skia::Pixmap::new(40, 30).unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::identity(),
            &mut vector.as_mut(),
        );
        for (x, y, expected) in [(20, 5, inside), (20, 25, outside)] {
            assert_eq!(
                png.get_pixel(x, y).0,
                expected,
                "PNG {body_color} omit={omit_background} at{x},{y}"
            );
            let pixel = vector.pixel(x, y).unwrap().demultiply();
            assert_eq!(
                [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
                expected,
                "SVG {body_color} omit={omit_background} at{x},{y}"
            );
        }
    }
}

fn assert_prepared_pixels(
    scene: &PreparedScreenshot,
    size: (u32, u32),
    samples: &[(u32, u32, [u8; 4])],
) {
    let png = image::load_from_memory(&scene.png().unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(png.dimensions(), size);
    let svg = scene.svg().unwrap();
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    let mut vector = tiny_skia::Pixmap::new(size.0, size.1).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut vector.as_mut(),
    );
    for &(x, y, expected) in samples {
        assert_eq!(png.get_pixel(x, y).0, expected, "PNG at {x},{y}");
        let pixel = vector.pixel(x, y).unwrap().demultiply();
        assert_eq!(
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
            expected,
            "SVG at {x},{y}"
        );
    }
}

#[test]
fn transparent_root_propagates_body_color_beyond_the_body_box() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let images = ImageAssets::new();
    for doctype in ["", "<!doctype html>"] {
        for (omit_background, expected) in [(true, [255, 0, 0, 128]), (false, [255, 127, 127, 255])]
        {
            let html = format!(
                "{doctype}<html style='background:transparent'><body style='margin:0;height:10px;background:rgba(255,0,0,0.5)'>"
            );
            let mut options = ScreenshotOptions::default();
            options.omit_background = omit_background;
            let scene = PreparedScreenshot::new(&html, "", viewport, &images, &options);
            assert_prepared_pixels(&scene, (40, 30), &[(5, 5, expected), (35, 25, expected)]);
        }
    }
}

#[test]
fn prepared_translucent_canvas_survives_svg_first_and_repeated_capture_order() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let html = "<html style='background:rgba(255,0,0,0.5)'><body style='margin:0;height:10px;background:blue'>";
    let mut options = ScreenshotOptions::default();
    options.omit_background = true;
    let scene = PreparedScreenshot::new(html, "", viewport, &ImageAssets::new(), &options);
    let svg = scene.svg().unwrap();
    let png = scene.png().unwrap();
    for _ in 0..3 {
        assert_eq!(scene.svg().unwrap(), svg);
        assert_eq!(scene.png().unwrap(), png);
    }
    assert_prepared_pixels(
        &scene,
        (40, 30),
        &[(20, 5, [0, 0, 255, 255]), (20, 25, [255, 0, 0, 128])],
    );
}

#[test]
fn body_canvas_color_reaches_full_page_overflow_without_repainting_the_viewport() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let html = "<html style='background:transparent'><body style='margin:0;height:10px;background:rgba(255,0,0,0.5)'><div style='position:absolute;left:0;top:80px;width:10px;height:10px;background:blue'></div>";
    let mut options = ScreenshotOptions::default();
    options.omit_background = true;
    options.full_page = true;
    let scene = PreparedScreenshot::new(html, "", viewport, &ImageAssets::new(), &options);
    assert_prepared_pixels(
        &scene,
        (40, 90),
        &[
            (35, 25, [255, 0, 0, 128]),
            (39, 89, [255, 0, 0, 128]),
            (5, 85, [0, 0, 255, 255]),
        ],
    );
}

use turbo_dom_raster::{ImageAssets, PreparedScreenshot, ScreenshotOptions, Viewport};

#[test]
fn decimal_dpr_keeps_integral_edges_integral() {
    let mut options = ScreenshotOptions::default();
    for (scale, width, height) in [
        (1.2, 120, 96),
        (1.1, 110, 88),
        (1.3, 130, 104),
        (1.4, 140, 112),
    ] {
        options.device_scale = scale;
        let scene = PreparedScreenshot::new(
            "<body style='margin:0'>",
            "",
            Viewport {
                width: 100,
                height: 80,
            },
            &ImageAssets::new(),
            &options,
        );
        let png = scene.png().unwrap();
        assert_eq!(
            image::load_from_memory(&png)
                .unwrap()
                .to_rgba8()
                .dimensions(),
            (width, height)
        );
        assert_eq!(scene.png().unwrap(), png);
        let svg = scene.svg().unwrap();
        assert!(svg.contains(&format!("width=\"{width}\" height=\"{height}\"")));
        let view = svg
            .split("viewBox=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let edges: Vec<f64> = view
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert!((edges[2] - 100.0).abs() < 1e-12 && (edges[3] - 80.0).abs() < 1e-12);
    }
}

#[test]
fn fractional_edges_round_to_nearest_with_positive_ties_up() {
    for (scale, width) in [
        (100.0_f64, 100),
        (100.0_f64.next_up(), 100),
        (100.0_f64.next_up().next_up(), 100),
        (100.0_f64.next_down(), 100),
        (100.000_000_1, 100),
        (100.49, 100),
        (100.5, 101),
        (100.5_f64.next_down(), 100),
        (100.5_f64.next_up(), 101),
    ] {
        let mut options = ScreenshotOptions::default();
        options.device_scale = scale;
        let scene = PreparedScreenshot::new(
            "",
            "",
            Viewport {
                width: 1,
                height: 1,
            },
            &ImageAssets::new(),
            &options,
        );
        assert_eq!(
            image::load_from_memory(&scene.png().unwrap())
                .unwrap()
                .width(),
            width
        );
    }
}

#[test]
fn unsupported_double_scales_reject_before_native_narrowing() {
    for scale in [
        f64::NAN,
        f64::INFINITY,
        f64::MAX,
        1e10,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        0.0,
        -1.0,
    ] {
        let mut options = ScreenshotOptions::default();
        options.device_scale = scale;
        let scene = PreparedScreenshot::new(
            "",
            "",
            Viewport {
                width: 100,
                height: 80,
            },
            &ImageAssets::new(),
            &options,
        );
        assert!(scene.png().is_err());
        assert!(scene.svg().is_err());
    }
}

#[test]
fn chrome_dimension_controls_match_for_viewport_and_full_page() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/screenshot-scale-chrome156.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut options = ScreenshotOptions::default();
        options.device_scale = case["requestedDpr"].as_f64().unwrap();
        options.full_page = case["fullPage"].as_bool().unwrap();
        let viewport = Viewport {
            width: case["viewport"]["width"].as_u64().unwrap() as u32,
            height: case["viewport"]["height"].as_u64().unwrap() as u32,
        };
        let scene = PreparedScreenshot::new(
            fixture["html"].as_str().unwrap(),
            "",
            viewport,
            &ImageAssets::new(),
            &options,
        );
        let pixels = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .to_rgba8();
        let expected = (
            case["cdp"]["width"].as_u64().unwrap() as u32,
            case["cdp"]["height"].as_u64().unwrap() as u32,
        );
        assert_eq!(pixels.dimensions(), expected, "{}", case["id"]);
        assert_eq!(case["cdp"]["width"], case["playwright"]["width"]);
        assert_eq!(case["cdp"]["height"], case["playwright"]["height"]);
        let svg = scene.svg().unwrap();
        let rendered = turbo_dom_layout::svg::rasterize(
            svg.as_bytes(),
            1.0,
            turbo_dom_layout::svg::Oversize::Reject,
        )
        .unwrap();
        assert_eq!((rendered.width, rendered.height), expected);
        if options.full_page {
            let y = (230.0 * options.device_scale).round() as u32;
            assert_eq!(pixels.get_pixel(2, y).0, [0, 0, 255, 255]);
        }
    }
}

#[test]
fn decimal_full_page_masks_and_fractional_clips_keep_css_geometry() {
    use turbo_dom_raster::{ScreenshotMask, ScreenshotRect};
    let html="<body style='margin:0'><div style='position:absolute;left:10.25px;top:100.5px;width:20.5px;height:19.5px;overflow:hidden'><div style='width:40px;height:40px;background:red'></div></div>";
    let masks = [ScreenshotMask::new(ScreenshotRect {
        x: 11.25,
        y: 102.5,
        width: 10.5,
        height: 10.25,
    })];
    let mut options = ScreenshotOptions::default();
    options.device_scale = 1.2;
    options.full_page = true;
    options.masks = &masks;
    let scene = PreparedScreenshot::new(
        html,
        "",
        Viewport {
            width: 100,
            height: 80,
        },
        &ImageAssets::new(),
        &options,
    );
    let pixels = image::load_from_memory(&scene.png().unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(pixels.dimensions(), (120, 144));
    let svg = scene.svg().unwrap();
    let rendered = turbo_dom_layout::svg::rasterize(
        svg.as_bytes(),
        1.0,
        turbo_dom_layout::svg::Oversize::Reject,
    )
    .unwrap();
    assert_eq!((rendered.width, rendered.height), (120, 144));
    for (x, y, color) in [
        (16, 128, [255, 0, 255, 255]),
        (0, 128, [255; 4]),
        (35, 128, [255, 0, 0, 255]),
        (20, 142, [255, 0, 0, 255]),
    ] {
        assert_eq!(pixels.get_pixel(x, y).0, color);
        let at = ((y * rendered.width + x) * 4) as usize;
        assert_eq!(&rendered.rgba[at..at + 4], &color);
    }
    options.full_page = false;
    options.masks = &[];
    let fractional = PreparedScreenshot::new(
        "",
        "",
        Viewport {
            width: 101,
            height: 81,
        },
        &ImageAssets::new(),
        &options,
    );
    assert_eq!(
        image::load_from_memory(&fractional.png().unwrap())
            .unwrap()
            .to_rgba8()
            .dimensions(),
        (121, 97)
    );
    options.device_scale = 1.000_000_001;
    let fractional = PreparedScreenshot::new(
        "",
        "",
        Viewport {
            width: 100,
            height: 80,
        },
        &ImageAssets::new(),
        &options,
    );
    assert_eq!(
        image::load_from_memory(&fractional.png().unwrap())
            .unwrap()
            .to_rgba8()
            .dimensions(),
        (100, 80)
    );
}

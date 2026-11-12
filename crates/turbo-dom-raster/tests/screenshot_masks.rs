use turbo_dom_raster::{
    screenshot_element_rects, screenshot_png_with_options, screenshot_svg_with_options,
    ImageAssets, ScreenshotMask, ScreenshotOptions, ScreenshotRect, Viewport,
};

#[test]
fn masks_cover_resolved_transformed_and_clipped_elements() {
    let html = "<!doctype html><body style='margin:0'><div style='position:absolute;left:10px;top:10px;width:20px;height:20px;background:red'></div><div style='position:absolute;left:40px;top:10px;width:20px;height:20px;overflow:hidden'><div style='width:30px;height:20px;background:blue;transform:translateX(10px)'></div></div><div hidden></div>";
    let viewport = Viewport {
        width: 100,
        height: 80,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    let rects =
        screenshot_element_rects(html, "", viewport, &images, &options, &[3, 5, 6, 999]).unwrap();
    assert_eq!(
        rects,
        vec![
            Some(ScreenshotRect {
                x: 10.,
                y: 10.,
                width: 20.,
                height: 20.
            }),
            Some(ScreenshotRect {
                x: 50.,
                y: 10.,
                width: 10.,
                height: 20.
            }),
            None,
            None
        ]
    );
    let masks = [
        ScreenshotMask::new(rects[0].unwrap()),
        ScreenshotMask {
            rect: rects[1].unwrap(),
            color: [0, 255, 0, 255],
        },
    ];
    options.masks = &masks;
    let bytes = screenshot_png_with_options(html, "", viewport, &images, &options).unwrap();
    let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(pixels.get_pixel(15, 15).0, [255, 0, 255, 255]);
    assert_eq!(pixels.get_pixel(55, 15).0, [0, 255, 0, 255]);
    assert_eq!(pixels.get_pixel(65, 15).0, [255; 4]);
    assert_eq!(pixels.get_pixel(5, 5).0, [255; 4]);
    let svg = screenshot_svg_with_options(html, "", viewport, &images, &options).unwrap();
    assert!(svg.contains("fill=\"rgb(255,0,255)\""));
    assert!(svg.contains("fill=\"rgb(0,255,0)\""));
    let rendered = turbo_dom_layout::svg::rasterize(
        svg.as_bytes(),
        1.,
        turbo_dom_layout::svg::Oversize::Reject,
    )
    .unwrap();
    assert_eq!((rendered.width, rendered.height), (100, 80));
    for (x, expected) in [
        (15, [255, 0, 255, 255]),
        (55, [0, 255, 0, 255]),
        (65, [255; 4]),
    ] {
        let at = (15 * 100 + x) * 4;
        assert_eq!(&rendered.rgba[at..at + 4], &expected);
    }
}

#[test]
fn explicit_masks_preserve_order_alpha_and_default_output() {
    let viewport = Viewport {
        width: 20,
        height: 20,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    let baseline = screenshot_png_with_options("", "", viewport, &images, &options).unwrap();
    let masks = [
        ScreenshotMask::new(ScreenshotRect {
            x: 2.,
            y: 2.,
            width: 10.,
            height: 10.,
        }),
        ScreenshotMask {
            rect: ScreenshotRect {
                x: 4.,
                y: 4.,
                width: 4.,
                height: 4.,
            },
            color: [0, 0, 255, 128],
        },
        ScreenshotMask::new(ScreenshotRect {
            x: f32::NAN,
            y: 0.,
            width: 20.,
            height: 20.,
        }),
        ScreenshotMask::new(ScreenshotRect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 20.,
        }),
    ];
    options.masks = &masks;
    let bytes = screenshot_png_with_options("", "", viewport, &images, &options).unwrap();
    let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(pixels.get_pixel(3, 3).0, [255, 0, 255, 255]);
    assert_eq!(pixels.get_pixel(5, 5).0, [127, 0, 255, 255]);
    assert_eq!(pixels.get_pixel(15, 15).0, [255; 4]);
    let svg = screenshot_svg_with_options("", "", viewport, &images, &options).unwrap();
    let rendered = turbo_dom_layout::svg::rasterize(
        svg.as_bytes(),
        1.,
        turbo_dom_layout::svg::Oversize::Reject,
    )
    .unwrap();
    let at = (5 * 20 + 5) * 4;
    assert_eq!(&rendered.rgba[at..at + 4], &[127, 0, 255, 255]);
    assert!(!svg.contains("NaN"));
    options.masks = &[];
    assert_eq!(
        baseline,
        screenshot_png_with_options("", "", viewport, &images, &options).unwrap()
    );
}

#[test]
fn source_indices_survive_svg_replacement_and_snapshot_changes() {
    let html = "<!doctype html><body style='margin:0'><svg style='position:absolute;left:10px;top:10px;width:20px;height:20px'><rect width='20' height='20' fill='red'/></svg><div style='position:absolute;left:40px;top:10px;width:20px;height:20px;background:blue'></div>";
    let viewport = Viewport {
        width: 100,
        height: 80,
    };
    let images = ImageAssets::new();
    let options = ScreenshotOptions::default();
    let expected = vec![
        Some(ScreenshotRect {
            x: 10.,
            y: 10.,
            width: 20.,
            height: 20.,
        }),
        Some(ScreenshotRect {
            x: 10.,
            y: 10.,
            width: 20.,
            height: 20.,
        }),
        Some(ScreenshotRect {
            x: 40.,
            y: 10.,
            width: 20.,
            height: 20.,
        }),
    ];
    assert_eq!(
        screenshot_element_rects(html, "", viewport, &images, &options, &[3, 4, 5]).unwrap(),
        expected
    );
    let other = html.replace("left:40px", "left:60px");
    let result = screenshot_element_rects(&other, "", viewport, &images, &options, &[5]).unwrap();
    assert_eq!(result[0].unwrap().x, 60.);
    assert_eq!(
        screenshot_element_rects(html, "", viewport, &images, &options, &[3, 4, 5]).unwrap(),
        expected
    );
}

#[test]
fn resolved_geometry_handles_inline_fragments_rotation_and_empty_clips() {
    let html = "<!doctype html><body style='margin:0'><div style='width:45px;font-size:20px'><span>one two three</span></div><div style='position:absolute;left:40px;top:10px;width:20px;height:10px;transform-origin:0 0;transform:rotate(90deg)'></div><div style='position:absolute;left:0;top:0;width:10px;height:10px;clip:rect(0,0,0,0)'><div style='width:5px;height:5px'></div></div>";
    let rects = screenshot_element_rects(
        html,
        "",
        Viewport {
            width: 100,
            height: 80,
        },
        &ImageAssets::new(),
        &ScreenshotOptions::default(),
        &[4, 5, 6, 7],
    )
    .unwrap();
    let inline = rects[0].unwrap();
    assert!(inline.width > 0. && inline.height > 30.);
    let rotated = rects[1].unwrap();
    for (actual, expected) in [
        (rotated.x, 30.),
        (rotated.y, 10.),
        (rotated.width, 10.),
        (rotated.height, 20.),
    ] {
        assert!((actual - expected).abs() < 0.0001);
    }
    assert_eq!(&rects[2..], &[None, None]);
}

#[test]
fn masks_use_css_coordinates_at_device_scale_and_full_page_height() {
    let html="<!doctype html><body style='margin:0'><div style='position:absolute;left:10px;top:100px;width:20px;height:20px;background:red'></div>";
    let viewport = Viewport {
        width: 50,
        height: 40,
    };
    let images = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.device_scale = 1.5;
    options.full_page = true;
    let rect =
        screenshot_element_rects(html, "", viewport, &images, &options, &[3]).unwrap()[0].unwrap();
    assert_eq!(
        rect,
        ScreenshotRect {
            x: 10.,
            y: 100.,
            width: 20.,
            height: 20.
        }
    );
    let masks = [ScreenshotMask::new(rect)];
    options.masks = &masks;
    let png = screenshot_png_with_options(html, "", viewport, &images, &options).unwrap();
    let pixels = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(pixels.dimensions(), (75, 180));
    assert_eq!(pixels.get_pixel(20, 160).0, [255, 0, 255, 255]);
    assert_eq!(pixels.get_pixel(50, 160).0, [255; 4]);
    let svg = screenshot_svg_with_options(html, "", viewport, &images, &options).unwrap();
    assert!(svg.contains("width=\"75\" height=\"180\""));
    assert!(svg.contains("x=\"10\" y=\"100\" width=\"20\" height=\"20\" fill=\"rgb(255,0,255)\""));
    let rendered = turbo_dom_layout::svg::rasterize(
        svg.as_bytes(),
        1.,
        turbo_dom_layout::svg::Oversize::Reject,
    )
    .unwrap();
    assert_eq!((rendered.width, rendered.height), (75, 180));
    let at = (160 * 75 + 20) * 4;
    assert_eq!(&rendered.rgba[at..at + 4], &[255, 0, 255, 255]);
}

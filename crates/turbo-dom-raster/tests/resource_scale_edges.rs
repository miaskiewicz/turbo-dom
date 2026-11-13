use turbo_dom_layout::image::Intrinsic;
use turbo_dom_raster::{
    capture_resources, screenshot_png_with_options, ImageAssets, ImageMetadata, ScreenshotMask,
    ScreenshotOptions, ScreenshotRect, Viewport,
};

fn fixture() -> (ImageAssets, ImageMetadata) {
    let mut solid = tiny_skia::Pixmap::new(1, 1).unwrap();
    solid.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    (
        [("edge.png".into(), solid.encode_png().unwrap())]
            .into_iter()
            .collect(),
        [("edge.png".into(), Intrinsic::new(1, 1, false))]
            .into_iter()
            .collect(),
    )
}

fn compare_subset(html: &str, viewport: Viewport, options: &ScreenshotOptions) -> Vec<String> {
    let (assets, metadata) = fixture();
    let mut options = *options;
    options.image_metadata = Some(&metadata);
    let plan = capture_resources(html, "", viewport, &metadata, &options);
    let selected = assets
        .iter()
        .filter(|(key, _)| plan.pixel_urls.contains(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let full = screenshot_png_with_options(html, "", viewport, &assets, &options).unwrap();
    let reduced = screenshot_png_with_options(html, "", viewport, &selected, &options).unwrap();
    let full_pixels = tiny_skia::Pixmap::decode_png(&full).unwrap();
    let reduced_pixels = tiny_skia::Pixmap::decode_png(&reduced).unwrap();
    assert_eq!(
        (full_pixels.width(), full_pixels.height()),
        (reduced_pixels.width(), reduced_pixels.height())
    );
    for (index, (all, subset)) in full_pixels
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reduced_pixels.data().as_chunks::<4>().0.iter())
        .enumerate()
    {
        assert_eq!(
            all, subset,
            "pixel {index}, scale {}, selected {:?}",
            options.device_scale, plan.pixel_urls
        );
    }
    assert_eq!(full, reduced);
    plan.pixel_urls
}

#[test]
fn scaled_resource_plan_preserves_rounded_canvas_edge_pixels() {
    let viewport = Viewport {
        width: 101,
        height: 81,
    };
    for (scale, full_page, position) in [
        (0.5, false, "left:101px;top:0;width:1px;height:10px"),
        (1.5, false, "left:101px;top:0;width:1px;height:10px"),
        (0.5, false, "left:0;top:81px;width:10px;height:1px"),
        (1.5, false, "left:0;top:81px;width:10px;height:1px"),
        (0.5, true, "left:101px;top:0;width:1px;height:10px"),
    ] {
        let mut options = ScreenshotOptions::default();
        options.device_scale = scale;
        options.full_page = full_page;
        let html = format!(
            "<body style='margin:0'><img src='edge.png' style='position:absolute;{position}'>"
        );
        assert_eq!(compare_subset(&html, viewport, &options), ["edge.png"]);
    }
}

#[test]
fn rounded_capture_edges_survive_masks_and_full_page_safety_cap() {
    let mut options = ScreenshotOptions::default();
    options.device_scale = 0.5;
    options.omit_background = true;
    let masks = [ScreenshotMask {
        rect: ScreenshotRect {
            x: 100.0,
            y: 0.0,
            width: 2.0,
            height: 10.0,
        },
        color: [0, 0, 255, 128],
    }];
    options.masks = &masks;
    let html = "<body style='margin:0'><img src='edge.png' style='position:absolute;left:101px;top:0;width:1px;height:10px'>";
    assert_eq!(
        compare_subset(
            html,
            Viewport {
                width: 101,
                height: 81
            },
            &options
        ),
        ["edge.png"]
    );
    options.device_scale = 0.50003;
    options.full_page = true;
    options.masks = &[];
    let html = "<body style='margin:0'><img src='edge.png' style='position:absolute;top:24000px;width:4px;height:1px'>";
    assert_eq!(
        compare_subset(
            html,
            Viewport {
                width: 4,
                height: 5
            },
            &options
        ),
        ["edge.png"]
    );
}

#[test]
fn integral_and_inward_rounded_edges_keep_offscreen_pixels_unselected() {
    let html = "<body style='margin:0'><img src='edge.png' style='position:absolute;left:101px;top:0;width:1px;height:10px'>";
    for scale in [1.0, 1.2, 1.25] {
        let mut options = ScreenshotOptions::default();
        options.device_scale = scale;
        assert!(compare_subset(
            html,
            Viewport {
                width: 101,
                height: 81
            },
            &options
        )
        .is_empty());
    }
    let (_, metadata) = fixture();
    let mut options = ScreenshotOptions::default();
    options.device_scale = f64::NAN;
    assert_eq!(
        capture_resources(
            "<img src='edge.png' width=1 height=1>",
            "",
            Viewport::default(),
            &metadata,
            &options
        )
        .pixel_urls,
        ["edge.png"]
    );
}

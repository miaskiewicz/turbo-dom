use std::fmt::Write as _;
use turbo_dom_layout::image::Intrinsic;
use turbo_dom_raster::{
    capture_resources, screenshot_png_with_options, ImageAssets, ImageMetadata, ScreenshotOptions,
    Viewport,
};

fn fixture(count: usize) -> (String, ImageAssets, ImageMetadata) {
    let mut html = String::from("<body style='margin:0'>");
    let mut assets = ImageAssets::new();
    let mut metadata = ImageMetadata::new();
    for i in 0..count {
        write!(
            html,
            "<img src='{i}.png' style='display:block;width:300px;height:200px'>"
        )
        .unwrap();
        let mut pm = tiny_skia::Pixmap::new(3, 2).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(
            if i % 2 == 0 { 255 } else { 0 },
            0,
            255,
            255,
        ));
        assets.insert(format!("{i}.png"), pm.encode_png().unwrap());
        metadata.insert(format!("{i}.png"), Intrinsic::new(3, 2, false));
    }
    (html, assets, metadata)
}

#[test]
fn selects_only_pixel_contributors_with_independent_metadata() {
    let (html, assets, metadata) = fixture(6);
    let vp = Viewport {
        width: 300,
        height: 200,
    };
    let mut options = ScreenshotOptions::default();
    options.image_metadata = Some(&metadata);
    let plan = capture_resources(&html, "", vp, &metadata, &options);
    assert_eq!(plan.pixel_urls, ["0.png"]);
    let selected = assets
        .iter()
        .filter(|(k, _)| plan.pixel_urls.contains(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let all = screenshot_png_with_options(&html, "", vp, &assets, &options).unwrap();
    let pixels = tiny_skia::Pixmap::decode_png(&all).unwrap();
    assert_eq!(pixels.pixel(150, 100).unwrap().red(), 255);
    assert_eq!(
        all,
        screenshot_png_with_options(&html, "", vp, &selected, &options).unwrap()
    );
    options.full_page = true;
    assert_eq!(
        capture_resources(&html, "", vp, &metadata, &options)
            .pixel_urls
            .len(),
        6
    );
}

#[test]
fn preserves_intrinsic_layout_and_pixel_equivalence() {
    let (mut html, assets, metadata) = fixture(6);
    html = html.replace("width:300px;height:200px", "width:300px");
    let vp = Viewport {
        width: 300,
        height: 200,
    };
    let mut options = ScreenshotOptions::default();
    options.image_metadata = Some(&metadata);
    let plan = capture_resources(&html, "", vp, &metadata, &options);
    assert_eq!(plan.pixel_urls, ["0.png"]);
    let selected = [("0.png".into(), assets["0.png"].clone())]
        .into_iter()
        .collect();
    assert_eq!(
        screenshot_png_with_options(&html, "", vp, &assets, &options).unwrap(),
        screenshot_png_with_options(&html, "", vp, &selected, &options).unwrap()
    );
}

#[test]
fn keeps_positioned_overflow_and_extending_effects_conservatively() {
    let (_, _, metadata) = fixture(6);
    let vp = Viewport {
        width: 300,
        height: 200,
    };
    let options = ScreenshotOptions::default();
    for style in [
        "position:absolute;top:0",
        "transform:translateY(-400px)",
        "box-shadow:0 -400px 20px black",
        "filter:blur(200px)",
    ] {
        let html = format!("<body style='margin:0'><div style='height:400px'></div><div style='{style}'><img src='0.png' width='300' height='200'></div>");
        assert!(
            capture_resources(&html, "", vp, &metadata, &options)
                .pixel_urls
                .contains(&"0.png".into()),
            "{style}"
        );
    }
    let html = "<body style='margin:0'><div style='height:0'><img src='0.png' style='position:absolute;top:0;width:300px;height:200px'></div>";
    assert_eq!(
        capture_resources(html, "", vp, &metadata, &options).pixel_urls,
        ["0.png"]
    );
}

#[test]
fn keeps_unknown_intrinsic_dependencies_and_backgrounds() {
    let vp = Viewport {
        width: 300,
        height: 200,
    };
    let options = ScreenshotOptions::default();
    let html = "<body style='margin:0'><div style='height:400px'></div><img src='unknown.png'>";
    assert_eq!(
        capture_resources(html, "", vp, &ImageMetadata::new(), &options).pixel_urls,
        ["unknown.png"]
    );
    let (_, _, metadata) = fixture(6);
    let html = "<body style='margin:0'><div class='first' style='width:300px;height:200px'></div><div style='background:url(1.png);height:200px'></div>";
    assert_eq!(
        capture_resources(
            html,
            ".first{background:url(0.png)}",
            vp,
            &metadata,
            &options
        )
        .pixel_urls,
        ["0.png"]
    );
}

#[test]
fn plans_are_owned_and_recomputed_for_changed_inputs() {
    let (_, _, mut metadata) = fixture(2);
    let html = "<body style='margin:0'><img src='0.png' style='display:block;width:300px'><img src='1.png' style='display:block;width:300px'>";
    let vp = Viewport {
        width: 300,
        height: 200,
    };
    let options = ScreenshotOptions::default();
    let first = capture_resources(html, "", vp, &metadata, &options);
    assert_eq!(first.pixel_urls, ["0.png"]);
    metadata.insert("0.png".into(), Intrinsic::new(300, 10, false));
    assert_eq!(
        capture_resources(html, "", vp, &metadata, &options).pixel_urls,
        ["0.png", "1.png"]
    );
    assert_eq!(first.pixel_urls, ["0.png"]);
    assert_eq!(
        capture_resources(html, "img{height:200px}", vp, &metadata, &options).pixel_urls,
        ["0.png"]
    );
    assert_eq!(
        capture_resources(
            html,
            "",
            Viewport {
                width: 300,
                height: 1
            },
            &metadata,
            &options
        )
        .pixel_urls,
        ["0.png"]
    );
    assert_eq!(
        capture_resources("<body>", "", vp, &metadata, &options).pixel_urls,
        Vec::<String>::new()
    );
    let inline = "<img src='data:image/png;base64,broken' width='10' height='10'>";
    assert_eq!(
        capture_resources(inline, "", vp, &metadata, &options).pixel_urls,
        Vec::<String>::new()
    );
}

#[test]
fn internally_generated_svg_images_are_not_fetch_requests() {
    let html = "<svg width='10' height='10'><rect width='10' height='10' fill='red'/></svg>";
    assert_eq!(
        capture_resources(
            html,
            "",
            Viewport::DEFAULT,
            &ImageMetadata::new(),
            &ScreenshotOptions::default()
        )
        .pixel_urls,
        Vec::<String>::new()
    );
}

#[test]
fn visible_metadata_without_a_pixel_body_uses_missing_image_placeholder() {
    let metadata = [("missing".into(), Intrinsic::new(10, 20, false))]
        .into_iter()
        .collect();
    let mut options = ScreenshotOptions::default();
    options.image_metadata = Some(&metadata);
    let html = "<body style='margin:0'><img src=missing style='display:block'><div style='width:10px;height:5px;background:red'></div>";
    let viewport = Viewport {
        width: 40,
        height: 40,
    };
    let png =
        screenshot_png_with_options(html, "", viewport, &ImageAssets::new(), &options).unwrap();
    let pixels = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!(pixels.pixel(5, 5).unwrap().red(), 220);
    assert_eq!(pixels.pixel(5, 5).unwrap().green(), 220);
    assert_eq!(pixels.pixel(5, 22).unwrap().red(), 255);
    assert_eq!(pixels.pixel(5, 22).unwrap().green(), 0);
    let svg = turbo_dom_raster::screenshot_svg_with_options(
        html,
        "",
        viewport,
        &ImageAssets::new(),
        &options,
    )
    .unwrap();
    assert!(svg.contains("#dcdcdc"));
    let mut body = tiny_skia::Pixmap::new(10, 20).unwrap();
    body.fill(tiny_skia::Color::from_rgba8(0, 255, 0, 255));
    let assets = [("missing".into(), body.encode_png().unwrap())]
        .into_iter()
        .collect();
    let pixels = tiny_skia::Pixmap::decode_png(
        &screenshot_png_with_options(html, "", viewport, &assets, &options).unwrap(),
    )
    .unwrap();
    assert_eq!(pixels.pixel(5, 5).unwrap().green(), 255);
    assert_eq!(pixels.pixel(5, 5).unwrap().red(), 0);
}

//! Repeatable offline capture planning harness. No wall-clock assertions.
use std::fmt::Write as _;
use std::time::Instant;
use turbo_dom_layout::image::Intrinsic;
use turbo_dom_raster::{
    capture_resources, screenshot_png_with_options, ImageAssets, ImageMetadata, ScreenshotOptions,
    Viewport,
};

fn main() {
    let rows: usize = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1000".into())
        .parse()
        .unwrap();
    let mut html = String::from("<body style='margin:0'>");
    let mut metadata = ImageMetadata::new();
    let mut images = ImageAssets::new();
    let mut image = tiny_skia::Pixmap::new(3, 2).unwrap();
    image.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    let bytes = image.encode_png().unwrap();
    for i in 0..rows {
        write!(
            html,
            "<img src='{i}.png' style='display:block;width:300px;height:200px'>"
        )
        .unwrap();
        metadata.insert(format!("{i}.png"), Intrinsic::new(3, 2, false));
        images.insert(format!("{i}.png"), bytes.clone());
    }
    let viewport = Viewport {
        width: 300,
        height: 200,
    };
    let mut options = ScreenshotOptions::default();
    options.image_metadata = Some(&metadata);
    let start = Instant::now();
    let plan = capture_resources(&html, "", viewport, &metadata, &options);
    let elapsed = start.elapsed();
    assert_eq!(plan.pixel_urls.len(), usize::from(rows > 0));
    let selected = images
        .iter()
        .filter(|(key, _)| plan.pixel_urls.contains(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    assert_eq!(
        screenshot_png_with_options(&html, "", viewport, &images, &options).unwrap(),
        screenshot_png_with_options(&html, "", viewport, &selected, &options).unwrap()
    );
    println!(
        "rows={rows} fragments={} selected={} supplied={} plan_us={} pixel_equivalent=true",
        plan.fragments_examined,
        plan.pixel_urls.len(),
        images.len(),
        elapsed.as_micros()
    );
}

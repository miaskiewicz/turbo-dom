//! Offline, scalable correctness/performance control for retained screenshots.
//! Run: `cargo run --release -p turbo-dom-raster --example retained_capture -- 5000 10`
use std::fmt::Write;
use std::time::Instant;
use turbo_dom_raster::{
    screenshot_png_with_options, ImageAssets, PreparedScreenshot, ScreenshotOptions, Viewport,
};

fn fixture(rows: usize) -> String {
    let mut html = String::from("<body style='margin:0'>");
    for row in 0..rows {
        let color = if row % 2 == 0 { "red" } else { "blue" };
        write!(html, "<div style='height:10px;background:{color}'></div>").unwrap();
    }
    html
}

fn main() {
    let mut args = std::env::args().skip(1);
    let rows = args
        .next()
        .map_or(5000, |v| v.parse::<usize>().unwrap())
        .max(20);
    let repeats = args
        .next()
        .map_or(10, |v| v.parse::<usize>().unwrap())
        .max(1);
    let html = fixture(rows);
    let assets = ImageAssets::new();
    let viewport = Viewport {
        width: 300,
        height: 200,
    };
    let options = ScreenshotOptions::default();
    let start = Instant::now();
    let scene = PreparedScreenshot::new(&html, "", viewport, &assets, &options);
    let prepare = start.elapsed();
    let expected = scene.png().unwrap();
    let start = Instant::now();
    for _ in 0..repeats {
        assert_eq!(scene.png().unwrap(), expected);
    }
    let retained = start.elapsed();
    let start = Instant::now();
    for _ in 0..repeats {
        assert_eq!(
            screenshot_png_with_options(&html, "", viewport, &assets, &options).unwrap(),
            expected
        );
    }
    let stateless = start.elapsed();
    let changed =
        PreparedScreenshot::new(&html, "body{background:green}", viewport, &assets, &options);
    // All rows fill the viewport, so a canvas background change alone is occluded.
    assert_eq!(changed.png().unwrap(), expected);
    let changed = PreparedScreenshot::new(
        &html,
        "div{background:green!important}",
        viewport,
        &assets,
        &options,
    );
    assert_ne!(changed.png().unwrap(), expected);
    assert_eq!(scene.png().unwrap(), expected);
    let pixels = tiny_skia::Pixmap::decode_png(&expected).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (300, 200));
    assert_eq!(pixels.pixel(1, 1).unwrap().red(), 255);
    assert_eq!(pixels.pixel(1, 11).unwrap().blue(), 255);
    println!("rows={rows} repeats={repeats} retained_layout_passes=1 stateless_layout_passes={repeats} prepare_ms={:.3} retained_total_ms={:.3} stateless_total_ms={:.3} bytes={} correctness=pass", prepare.as_secs_f64() * 1000.0, retained.as_secs_f64() * 1000.0, stateless.as_secs_f64() * 1000.0, expected.len());
}

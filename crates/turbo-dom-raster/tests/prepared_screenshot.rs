use std::fmt::Write;
use turbo_dom_raster::{
    screenshot_png_with_options, screenshot_svg_with_options, CanvasContent, CanvasContents,
    IdlStates, ImageAssets, PreparedScreenshot, ScreenshotOptions, Viewport,
};

fn rows(count: usize) -> String {
    let mut html = String::from("<body style='margin:0'>");
    for row in 0..count {
        let color = if row % 2 == 0 { "red" } else { "blue" };
        write!(html, "<div style='height:10px;background:{color}'></div>").unwrap();
    }
    html
}

#[test]
fn repeated_prepared_captures_preserve_pixels_without_preparing_again() {
    let html = rows(5000);
    let viewport = Viewport {
        width: 300,
        height: 200,
    };
    let assets = ImageAssets::new();
    let options = ScreenshotOptions::default();
    let scene = PreparedScreenshot::new(&html, "", viewport, &assets, &options);
    let png = scene.png().unwrap();
    assert_eq!(
        png,
        screenshot_png_with_options(&html, "", viewport, &assets, &options).unwrap()
    );
    for _ in 0..3 {
        assert_eq!(scene.png().unwrap(), png);
    }
    assert_eq!(
        scene.svg().unwrap(),
        screenshot_svg_with_options(&html, "", viewport, &assets, &options).unwrap()
    );
    let pixels = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!(pixels.pixel(5, 5).unwrap().red(), 255);
    assert_eq!(pixels.pixel(5, 15).unwrap().blue(), 255);
}

#[test]
fn prepared_scene_owns_all_inputs_and_replacement_observes_changes() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let mut html = String::from("<body style='margin:0'><img src='asset'><canvas width=2 height=2></canvas><input type=checkbox>");
    let mut css = String::from("img{width:10px;height:10px}");
    let mut images = ImageAssets::new();
    let mut bitmap = tiny_skia::Pixmap::new(2, 2).unwrap();
    bitmap.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    images.insert("asset".into(), bitmap.encode_png().unwrap());
    let mut canvases = CanvasContents::new();
    canvases.insert(
        0,
        CanvasContent::Rgba {
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        },
    );
    let mut states = IdlStates::default();
    let mut options = ScreenshotOptions::default();
    options.canvases = Some(&canvases);
    options.idl_states = Some(&states);
    let original = PreparedScreenshot::new(&html, &css, viewport, &images, &options);
    let original_png = original.png().unwrap();
    html.push_str("<div style='height:200px;background:blue'></div>");
    css.push_str("body{background:green}img{width:20px}");
    bitmap.fill(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
    images.insert("asset".into(), bitmap.encode_png().unwrap());
    canvases.clear();
    states.indeterminate_inputs.insert(0);
    let mut options = ScreenshotOptions::default();
    options.canvases = Some(&canvases);
    options.idl_states = Some(&states);
    options.full_page = true;
    let replacement = PreparedScreenshot::new(
        &html,
        &css,
        Viewport {
            width: 50,
            height: 40,
        },
        &images,
        &options,
    );
    assert_eq!(original.png().unwrap(), original_png);
    assert_ne!(replacement.png().unwrap(), original_png);
    assert_eq!(
        replacement.png().unwrap(),
        screenshot_png_with_options(
            &html,
            &css,
            Viewport {
                width: 50,
                height: 40
            },
            &images,
            &options
        )
        .unwrap()
    );
}

#[test]
fn replacing_each_layout_input_and_font_registry_is_explicit() {
    use turbo_dom_layout::{FontFace, FontRegistry};
    let viewport = Viewport {
        width: 120,
        height: 50,
    };
    let html = "<body style='margin:0'><div style='font:20px WebTest'>iiii WWWW</div>";
    let assets = ImageAssets::new();
    let options = ScreenshotOptions::default();
    let old = PreparedScreenshot::new(html, "", viewport, &assets, &options);
    let original = old.png().unwrap();
    for (markup, css, viewport) in [
        ("<body style='background:red'>new markup", "", viewport),
        (html, "body{background:blue}", viewport),
        (
            html,
            "",
            Viewport {
                width: 130,
                height: 60,
            },
        ),
    ] {
        let replacement = PreparedScreenshot::new(markup, css, viewport, &assets, &options);
        assert_ne!(replacement.png().unwrap(), original);
        assert_eq!(old.png().unwrap(), original);
    }
    let custom = {
        let mut fonts = FontRegistry::new();
        fonts.add(
            FontFace::from_web_font_bytes(
                include_bytes!("../../turbo-dom-layout/assets/fonts/Go-Regular.ttf").to_vec(),
                "WebTest",
                400,
                false,
            )
            .unwrap(),
        );
        let mut options = ScreenshotOptions::default();
        options.fonts = Some(&fonts);
        PreparedScreenshot::new(html, "", viewport, &assets, &options)
    };
    assert_ne!(custom.png().unwrap(), original);
    assert_eq!(old.png().unwrap(), original);
}

#[test]
fn full_page_retains_offscreen_rows_and_independent_live_state() {
    let viewport = Viewport {
        width: 30,
        height: 20,
    };
    let assets = ImageAssets::new();
    let mut options = ScreenshotOptions::default();
    options.full_page = true;
    let page = PreparedScreenshot::new(&rows(25), "", viewport, &assets, &options);
    let png = page.png().unwrap();
    let pixels = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (30, 250));
    assert_eq!(pixels.pixel(2, 245).unwrap().red(), 255);
    let html = "<body style='margin:0'><input type=checkbox><canvas width=10 height=10></canvas>";
    options.full_page = false;
    let base = PreparedScreenshot::new(html, "", viewport, &assets, &options);
    let mut states = IdlStates::default();
    states.indeterminate_inputs.insert(0);
    options.idl_states = Some(&states);
    let state = PreparedScreenshot::new(html, "", viewport, &assets, &options);
    assert_ne!(base.png().unwrap(), state.png().unwrap());
    options.idl_states = None;
    let mut canvases = CanvasContents::new();
    canvases.insert(
        0,
        CanvasContent::Rgba {
            width: 1,
            height: 1,
            pixels: vec![255, 0, 0, 255],
        },
    );
    options.canvases = Some(&canvases);
    let canvas = PreparedScreenshot::new(html, "", viewport, &assets, &options);
    assert_ne!(base.png().unwrap(), canvas.png().unwrap());
    assert_ne!(canvas.png().unwrap(), state.png().unwrap());
}

#[test]
fn intrinsic_image_changes_are_isolated_from_existing_snapshot() {
    let viewport = Viewport {
        width: 40,
        height: 30,
    };
    let options = ScreenshotOptions::default();
    let html = "<body style='margin:0'><img src='asset'>";
    let mut images = ImageAssets::new();
    let mut small = tiny_skia::Pixmap::new(2, 2).unwrap();
    small.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    images.insert("asset".into(), small.encode_png().unwrap());
    let original = PreparedScreenshot::new(html, "", viewport, &images, &options);
    let old = original.png().unwrap();
    let mut large = tiny_skia::Pixmap::new(20, 20).unwrap();
    large.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    images.insert("asset".into(), large.encode_png().unwrap());
    let replacement = PreparedScreenshot::new(html, "", viewport, &images, &options);
    assert_ne!(replacement.png().unwrap(), old);
    images.clear();
    assert_eq!(original.png().unwrap(), old);
    let pixels = tiny_skia::Pixmap::decode_png(&replacement.png().unwrap()).unwrap();
    assert_eq!(pixels.pixel(10, 10).unwrap().red(), 255);
    assert_eq!(pixels.pixel(10, 10).unwrap().green(), 0);
}

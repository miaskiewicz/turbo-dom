//! Web fonts in screenshots and canvas replay against a Chrome 156 pixel control
//! (miaskiewicz/turbo-dom#304): a caller `FontRegistry` carrying `@font-face` faces reaches
//! the screenshot layout/paint and the canvas text painter. Oracle:
//! `tests/fixtures/web_font_paint_chrome156.json` from `scripts/web-font-paint-chrome-control.mjs`
//! (Go-Regular as `WebGo`, which the raster does not bundle, and IBM Plex Mono as `WebPlex`,
//! whose web-font metrics give 25px lines at 20px). Each case's ink column and row runs must
//! agree to a pixel (anti-aliasing differs between Skia and tiny-skia; a fallback face or a
//! wrong line pitch moves the runs by many pixels).

use turbo_dom_layout::{FontFace, FontRegistry};
use turbo_dom_raster::{canvas_ops_rgba_with_fonts, screenshot_png_with_fonts, Viewport};

fn control() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/web_font_paint_chrome156.json")).unwrap()
}

/// The bundled registry plus the control's `@font-face` faces.
fn web_fonts(control: &serde_json::Value) -> FontRegistry {
    let mut fonts = FontRegistry::new();
    for (family, face) in control["faces"].as_object().unwrap() {
        let path = format!(
            "{}/../turbo-dom-layout/assets/fonts/{}",
            env!("CARGO_MANIFEST_DIR"),
            face["file"].as_str().unwrap()
        );
        let bytes = std::fs::read(path).unwrap();
        fonts.add(FontFace::from_web_font_bytes(bytes, family, 400, false).unwrap());
    }
    fonts
}

fn runs(count: usize, ink: impl Fn(usize) -> bool) -> Vec<[usize; 2]> {
    let mut out = Vec::new();
    let mut start = None;
    for i in 0..=count {
        match (i < count && ink(i), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push([s, i]);
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// `(columns, rows)` ink runs of a `width x height` image whose pixel `(x, y)` is ink.
fn ink_runs(
    width: usize,
    height: usize,
    ink: impl Fn(usize, usize) -> bool,
) -> (Vec<[usize; 2]>, Vec<[usize; 2]>) {
    (
        runs(width, |x| (0..height).any(|y| ink(x, y))),
        runs(height, |y| (0..width).any(|x| ink(x, y))),
    )
}

fn within_a_pixel(got: &[[usize; 2]], want: &serde_json::Value) -> bool {
    let want: Vec<[usize; 2]> = serde_json::from_value(want.clone()).unwrap();
    got.len() == want.len()
        && got
            .iter()
            .flatten()
            .zip(want.iter().flatten())
            .all(|(a, b)| a.abs_diff(*b) <= 1)
}

#[test]
fn screenshots_paint_web_fonts_from_the_caller_registry_like_chrome() {
    let control = control();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let fonts = web_fonts(&control);
    let [width, height] = [0, 1].map(|i| control["viewport"][i].as_u64().unwrap() as u32);
    let style = control["pageStyle"].as_str().unwrap();
    let cases = control["screenshots"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    let mut failures = Vec::new();
    for case in cases {
        let html = format!(
            "<!doctype html><html><head><style>{style}</style></head><body>{}</body></html>",
            case["body"].as_str().unwrap()
        );
        let png = screenshot_png_with_fonts(
            &html,
            "",
            Viewport { width, height },
            &std::collections::HashMap::new(),
            false,
            &fonts,
        )
        .unwrap();
        let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
        let (columns, rows) = ink_runs(width as usize, height as usize, |x, y| {
            let p = pm.pixel(x as u32, y as u32).unwrap();
            p.red() < 128 || p.green() < 128 || p.blue() < 128
        });
        if !within_a_pixel(&columns, &case["columns"]) || !within_a_pixel(&rows, &case["rows"]) {
            failures.push(format!(
                "{}: columns {columns:?} rows {rows:?}\n  chrome columns {} rows {}",
                case["name"], case["columns"], case["rows"]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn canvas_text_paints_web_fonts_from_the_caller_registry_like_chrome() {
    let control = control();
    let fonts = web_fonts(&control);
    let case = &control["canvas"];
    let [width, height] = ["width", "height"].map(|k| case[k].as_u64().unwrap() as usize);
    let ops = serde_json::json!([[
        "fillText",
        [case["text"], case["baseline"][0], case["baseline"][1]],
        "#000000",
        null,
        case["font"]
    ]]);
    let rgba =
        canvas_ops_rgba_with_fonts(width as u32, height as u32, &ops.to_string(), &fonts).unwrap();
    let (columns, rows) = ink_runs(width, height, |x, y| rgba[(y * width + x) * 4 + 3] >= 128);
    assert!(
        within_a_pixel(&columns, &case["columns"]) && within_a_pixel(&rows, &case["rows"]),
        "columns {columns:?} rows {rows:?}\nchrome columns {} rows {}",
        case["columns"],
        case["rows"]
    );
    // The PNG entry draws the same pixels.
    let png = turbo_dom_raster::canvas_ops_png_with_fonts(
        width as u32,
        height as u32,
        &ops.to_string(),
        &fonts,
    )
    .unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    let (png_columns, _) = ink_runs(width, height, |x, y| {
        pm.pixel(x as u32, y as u32).unwrap().alpha() >= 128
    });
    assert_eq!(png_columns, columns);
    // Canvas measureText reads the same registry.
    let measured = turbo_dom_raster::measure_canvas_text_with_fonts(
        case["text"].as_str().unwrap(),
        case["font"].as_str().unwrap(),
        &fonts,
    );
    // Blink's double summation order differs by ~1e-14; one 16.16 unit is 1.5e-5.
    assert!((measured - case["measureText"].as_f64().unwrap()).abs() < 1e-9);
}

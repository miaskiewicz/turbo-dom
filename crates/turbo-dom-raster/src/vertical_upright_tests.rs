//! Upright text in vertical writing modes painted like Chrome 156
//! (miaskiewicz/turbo-dom#239): the `ic-*` cases of
//! `tests/fixtures/vertical_text_chrome156.json` (`scripts/vertical-text-chrome-control.mjs`),
//! in the generated box-glyph Ic Fixture face (which no screenshot entry point resolves,
//! so the galley is laid out with it here and painted through `paint_png` / `paint_svg`).
//! CJK under `text-orientation: mixed` and every character under `upright` stand upright,
//! centred on the line and advancing by the 1em vertical advance; Latin in a mixed run
//! stays turned. The ink inside each box is compared with Chrome's within 1px.
use std::collections::HashMap;

use turbo_dom_layout::{layout_html, Diagnostics, FontFace, FontRegistry, Fragment, Rgba};

const WHITE: Rgba = Rgba {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

fn fonts() -> FontRegistry {
    let bytes = include_bytes!("../../turbo-dom-layout/assets/fonts/ic-fixture/IcFixture.ttf");
    let face = FontFace::from_bytes(bytes.to_vec(), "Ic Fixture", 400, false).unwrap();
    let mut registry = FontRegistry::new();
    registry.add(face);
    registry
}

type Pixels = (u32, Vec<u8>);

fn png(galley: &Fragment) -> Pixels {
    let assets = crate::image_paint::decode_all(&HashMap::new());
    let png = crate::paint_png::paint(galley, 800, 600, WHITE, &assets).unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    (pm.width(), pm.data().to_vec())
}

fn svg(galley: &Fragment) -> Pixels {
    use resvg::{tiny_skia as rsk, usvg};
    let assets = crate::image_paint::decode_all(&HashMap::new());
    let svg = crate::paint_svg::paint(galley, 800, 600, WHITE, &assets);
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let mut pm = rsk::Pixmap::new(800, 600).unwrap();
    pm.fill(rsk::Color::WHITE);
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    (pm.width(), pm.data().to_vec())
}

/// The ink box within `rect` (pixels more than 96 below white in a channel).
fn ink((width, rgba): &Pixels, rect: [i64; 4]) -> Option<[i64; 4]> {
    let mut found: Option<[i64; 4]> = None;
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            let i = ((y * i64::from(*width) + x) * 4) as usize;
            if rgba[i..i + 3].iter().any(|&v| 255 - i64::from(v) > 96) {
                found = Some(found.map_or([x, y, x + 1, y + 1], |b| {
                    [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)]
                }));
            }
        }
    }
    found
}

fn differences(paint: fn(&Fragment) -> Pixels) -> Vec<String> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/vertical_text_chrome156.json"
    ))
    .unwrap();
    let fonts = fonts();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        if case["ic"].as_bool() != Some(true) {
            continue;
        }
        let id = case["id"].as_str().unwrap();
        let mut diags = Diagnostics::default();
        let galley = layout_html(
            case["html"].as_str().unwrap(),
            "",
            800.0,
            &fonts,
            &mut diags,
        )
        .unwrap();
        let pixels = paint(&galley);
        for chrome in case["boxes"].as_array().unwrap() {
            let r = [0, 1, 2, 3].map(|i| chrome["rect"][i].as_f64().unwrap());
            let rect = [
                r[0].floor() as i64,
                r[1].floor() as i64,
                (r[0] + r[2]).ceil() as i64 + 4,
                (r[1] + r[3]).ceil() as i64 + 4,
            ];
            let want = [0, 1, 2, 3].map(|i| chrome["ink"]["box"][i].as_i64().unwrap());
            let got = ink(&pixels, rect);
            compared += 1;
            if !got.is_some_and(|got| got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1)) {
                failures.push(format!("{id}: ink {got:?}, Chrome {want:?}"));
            }
        }
    }
    assert_eq!(compared, 4);
    failures
}

#[test]
fn upright_text_paints_like_chrome_156_png() {
    let failures = differences(png);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn upright_text_paints_like_chrome_156_svg() {
    let failures = differences(svg);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

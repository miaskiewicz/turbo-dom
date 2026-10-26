//! EXIF image orientation in screenshots against Chrome 156 (miaskiewicz/turbo-html2pdf#30).
//!
//! The control is turbo-dom-layout's `tests/fixtures/image_orientation_chrome156.json`
//! (turbo-dom's `scripts/image-orientation-chrome-control.mjs`): one page showing every
//! `tests/fixtures/images/orientation` image as `<img>` and as a background, with and
//! without `image-orientation: none`, plus Chrome's colour class (R/G/B/Y/W) at sample
//! points read from its screenshot. The same page is screenshotted here (the `<img>`
//! `data:` sources supplied as fetched assets, as turbo-surf passes them) and every
//! sample point compared.
use std::collections::HashMap;

use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, ImageAssets, Viewport};

fn class([r, g, b]: [u8; 3]) -> char {
    let hi = |v: u8| v > 160;
    let lo = |v: u8| v < 96;
    match (hi(r), hi(g), hi(b), lo(r), lo(g), lo(b)) {
        (true, true, true, ..) => 'W',
        (true, _, _, _, true, true) => 'R',
        (_, true, _, true, _, true) => 'G',
        (_, _, true, true, true, _) => 'B',
        (true, true, _, _, _, true) => 'Y',
        _ => '?',
    }
}

fn b64(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        buf = buf << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    out
}

/// Every `src="data:…;base64,…"` of the page, keyed as the layout resolves it.
fn img_assets(html: &str) -> ImageAssets {
    let mut assets = HashMap::new();
    for part in html.split("src=\"").skip(1) {
        let uri = &part[..part.find('"').unwrap()];
        let payload = &uri[uri.find(',').unwrap() + 1..];
        assets.insert(uri.to_string(), b64(payload));
    }
    assets
}

#[test]
fn oriented_pixels_match_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../turbo-dom-layout/tests/fixtures/image_orientation_chrome156.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    let html = fixture["html"].as_str().unwrap();
    let viewport = Viewport {
        width: 1280,
        height: 1300,
    };
    let png =
        screenshot_png_with_opts(html, "", viewport, &img_assets(html), false, false).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let want: Vec<char> = case["samples"].as_str().unwrap().chars().collect();
        let got: String = case["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let (x, y) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
                let c = pm.pixel(x, y).expect("in bounds");
                class([c.red(), c.green(), c.blue()])
            })
            .collect();
        if got.chars().collect::<Vec<_>>() != want {
            failures.push(format!("{}: {got}, Chrome {}", case["id"], case["samples"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn svg_screenshots_embed_the_displayed_pixels() {
    // The SVG painter embeds each image as a PNG `data:` URI: an orientation-6 JPEG
    // is embedded turned (40x64), not as stored (64x40).
    let jpeg = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../turbo-dom-layout/tests/fixtures/images/orientation/jpeg-o6.jpg"
    ))
    .unwrap();
    let html = r#"<html><body style="margin:0"><img src="o6.jpg"></body></html>"#;
    let assets: ImageAssets = HashMap::from([("o6.jpg".to_string(), jpeg)]);
    let viewport = Viewport {
        width: 200,
        height: 100,
    };
    let svg = screenshot_svg_with_opts(html, "", viewport, &assets, false, false).unwrap();
    let uri = svg
        .split("href=\"data:image/png;base64,")
        .nth(1)
        .expect("embedded image");
    let png = b64(&uri[..uri.find('"').unwrap()]);
    let size = (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    );
    assert_eq!(size, (40, 64));
    assert!(
        svg.contains(r#"width="40.00" height="64.00""#),
        "the box is turned too"
    );
}

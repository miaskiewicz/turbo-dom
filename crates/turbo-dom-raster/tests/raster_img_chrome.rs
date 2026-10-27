//! `<img>` handling against Chrome 156 (miaskiewicz/turbo-dom#27).
//!
//! The control is `tests/fixtures/raster_img_chrome156.json` (turbo-dom's
//! `scripts/raster-img-chrome-control.mjs`): pages of absolutely positioned boxes over a
//! `#00ffff` backdrop, with Chrome's colour class (R/G/B/Y/W/T = backdrop) at the four
//! quadrant centres of each `[data-cid]` box, and the attributes of every element of
//! Chrome's `document.images`.
//!
//! Item 1: an `<img src="data:…">` (base64 / percent-encoded PNG, SVG, BMP) paints with no
//! caller-supplied bytes. Item 2: only the parsed document's HTML `<img>` elements count —
//! `<img` text in an attribute value, a `<script>` string, a comment or `<template>`
//! content is not one: the screenshot (and its lazy-image fill) leaves it alone,
//! `image_urls` returns the URLs of exactly Chrome's images, and `delazy_images` changes
//! nothing but those images.
//!
//! Item 3 (miaskiewicz/turbo-dom#107): an image Chrome sizes but whose pixels do not decode
//! (a frame-corrupt AVIF, a PNG cut inside IDAT, a GIF with an empty LZW stream) paints
//! nothing — not the grey placeholder — and an AVIF whose primary item runs past the end of
//! the data is broken (naturalWidth 0): it renders like an `<img>` that does not resolve.
use turbo_dom_layout::parse_html_tree;
use turbo_dom_layout::turbo_dom::rtdom::tree::NodeType;
use turbo_dom_layout::turbo_dom::{Handle, Tree};
use turbo_dom_raster::{delazy_images, image_urls, screenshot_png, screenshot_svg, Viewport};

fn class([r, g, b]: [u8; 3]) -> char {
    let hi = |v: u8| v > 160;
    let lo = |v: u8| v < 96;
    if hi(r) && hi(g) && hi(b) {
        'W'
    } else if lo(r) && hi(g) && hi(b) {
        'T'
    } else if hi(r) && lo(g) && lo(b) {
        'R'
    } else if lo(r) && hi(g) && lo(b) {
        'G'
    } else if lo(r) && lo(g) && hi(b) {
        'B'
    } else if hi(r) && hi(g) && lo(b) {
        'Y'
    } else {
        '?'
    }
}

fn cases(prefix: &str) -> Vec<serde_json::Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/raster_img_chrome156.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    let all = fixture["cases"].as_array().unwrap().clone();
    all.into_iter()
        .filter(|c| c["id"].as_str().unwrap().starts_with(prefix))
        .collect()
}

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

/// Each `[data-cid]` box's colour classes in the PNG against Chrome's, pushed to
/// `failures` on mismatch.
fn compare_pixels(case: &serde_json::Value, failures: &mut Vec<String>) {
    let id = case["id"].as_str().unwrap();
    let html = case["html"].as_str().unwrap();
    let png = screenshot_png(html, VIEWPORT).expect("png");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
    for b in case["boxes"].as_array().unwrap() {
        let got: String = b["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let (x, y) = (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32);
                let c = pm.pixel(x, y).expect("in bounds");
                class([c.red(), c.green(), c.blue()])
            })
            .collect();
        let want = b["samples"].as_str().unwrap();
        if got != want {
            failures.push(format!("{id} {}: {got}, Chrome {want}", b["cid"]));
        }
    }
}

/// The SVG screenshot embeds one image per `data:` image of Chrome's `document.images`.
fn compare_svg_images(case: &serde_json::Value, failures: &mut Vec<String>) {
    let id = case["id"].as_str().unwrap();
    let html = case["html"].as_str().unwrap();
    let painted = screenshot_svg(html, VIEWPORT)
        .expect("svg")
        .matches("<image")
        .count();
    let images = case["images"].as_array().unwrap();
    let data_images = images
        .iter()
        .filter(|a| {
            a["src"]
                .as_str()
                .is_some_and(|s| s.to_ascii_lowercase().starts_with("data:"))
        })
        .count();
    if painted != data_images {
        failures.push(format!(
            "{id}: {painted} painted images, Chrome {data_images}"
        ));
    }
}

#[test]
fn data_url_images_paint_like_chrome_156() {
    let mut failures = Vec::new();
    for case in cases("data-") {
        compare_pixels(&case, &mut failures);
        compare_svg_images(&case, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// An `<img>` with exactly `attrs` (Chrome's attributes of one image element).
fn img(attrs: &serde_json::Map<String, serde_json::Value>) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("<img");
    for (k, v) in attrs {
        let v = v
            .as_str()
            .unwrap()
            .replace('&', "&amp;")
            .replace('"', "&quot;");
        write!(out, " {k}=\"{v}\"").unwrap();
    }
    out + ">"
}

/// Every node of `tree` in document order (template content included) as text: an
/// element's name and attributes (an `<img>`'s `src`/`style` left out — what the lazy fill
/// writes), a text node's or comment's data.
fn shape(tree: &Tree) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack: Vec<Handle> = vec![tree.root()];
    while let Some(h) = stack.pop() {
        match tree.node_type(h) {
            NodeType::Element => {
                let name = tree.local_name(h).unwrap_or_default().to_string();
                let attrs: Vec<(String, String)> = tree
                    .attributes(h)
                    .into_iter()
                    .filter(|(k, _)| name != "img" || (k != "src" && k != "style"))
                    .collect();
                out.push(format!("{name} {attrs:?}"));
            }
            NodeType::Text | NodeType::Comment => out.push(tree.node_value(h).unwrap_or_default()),
            _ => {}
        }
        stack.extend(tree.children(h).into_iter().rev());
    }
    out
}

/// The `src` of every `<img>` element of `tree` (template content excluded).
fn img_srcs(tree: &Tree) -> Vec<Option<String>> {
    let mut out = Vec::new();
    let mut stack: Vec<Handle> = vec![tree.root()];
    while let Some(h) = stack.pop() {
        if tree.node_type(h) == NodeType::Element && tree.local_name(h) == Some("img") {
            out.push(tree.get_attribute(h, "src").map(str::to_string));
        }
        let kids = tree.children(h).into_iter().rev();
        stack.extend(kids.filter(|&c| tree.node_type(c) != NodeType::Fragment));
    }
    out
}

#[test]
fn img_text_is_not_an_image_like_chrome_156() {
    let mut failures = Vec::new();
    for case in cases("img-text-") {
        let id = case["id"].as_str().unwrap();
        let html = case["html"].as_str().unwrap();
        compare_pixels(&case, &mut failures);
        let images: Vec<String> = case["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| img(a.as_object().unwrap()))
            .collect();
        // The fetch list is the one of Chrome's image elements alone.
        let want = image_urls(&images.concat());
        if image_urls(html) != want {
            failures.push(format!(
                "{id}: image_urls {:?}, Chrome's images {want:?}",
                image_urls(html)
            ));
        }
        // The lazy fill changes Chrome's images only, as it does them on their own.
        let (before, after) = (parse_html_tree(html), parse_html_tree(&delazy_images(html)));
        if shape(&before) != shape(&after) {
            failures.push(format!("{id}: delazy_images changed more than the images"));
        }
        let alone = parse_html_tree(&delazy_images(&images.concat()));
        if img_srcs(&after) != img_srcs(&alone) {
            failures.push(format!(
                "{id}: delazy srcs {:?}, Chrome's images {:?}",
                img_srcs(&after),
                img_srcs(&alone)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Chrome's `[naturalWidth, naturalHeight]` of the case's one image.
fn natural(case: &serde_json::Value) -> (u64, u64) {
    let n = &case["natural"][0];
    (n[0].as_u64().unwrap(), n[1].as_u64().unwrap())
}

#[test]
#[ignore = "turbo-dom#107: fails until the fix"]
fn undecodable_images_paint_nothing_like_chrome_156() {
    let mut failures = Vec::new();
    let undecodable = cases("undecodable-");
    assert_eq!(undecodable.len(), 5);
    for case in undecodable {
        let id = case["id"].as_str().unwrap();
        let html = case["html"].as_str().unwrap();
        if natural(&case) == (0, 0) {
            // Broken: the screenshots equal those of an `<img>` no decoder reads.
            let src = case["images"][0]["src"].as_str().unwrap();
            let unresolved = html.replace(src, "data:image/png;base64,AAAA");
            assert_ne!(unresolved, html);
            if screenshot_png(html, VIEWPORT) != screenshot_png(&unresolved, VIEWPORT) {
                failures.push(format!("{id}: PNG differs from an unresolved <img>"));
            }
            if screenshot_svg(html, VIEWPORT) != screenshot_svg(&unresolved, VIEWPORT) {
                failures.push(format!("{id}: SVG differs from an unresolved <img>"));
            }
            continue;
        }
        assert_eq!(natural(&case), (64, 40), "{id}");
        // Without the `avif` feature there is no AVIF decoder: the sized box paints as the
        // light-grey placeholder (class W).
        if id.contains("avif") && !cfg!(feature = "avif") {
            let mut placeholder = case.clone();
            placeholder["boxes"][0]["samples"] = "WWWW".into();
            compare_pixels(&placeholder, &mut failures);
            continue;
        }
        compare_pixels(&case, &mut failures);
        let svg = screenshot_svg(html, VIEWPORT).expect("svg");
        let painted = svg.matches("<image").count();
        let want = usize::from(id == "undecodable-intact-avif");
        if painted != want || svg.contains("#dcdcdc") {
            failures.push(format!(
                "{id}: SVG has {painted} <image> (want {want}), placeholder {}",
                svg.contains("#dcdcdc")
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

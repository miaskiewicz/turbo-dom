//! Native form-control and media-control looks painted like Chrome 156: the cases of
//! `tests/fixtures/form_native_chrome156.json` (`scripts/form-native-chrome-control.mjs`).
//! - `progress-*` / `meter-*`: `<progress>` and `<meter>` laid out and painted as native
//!   controls (miaskiewicz/turbo-dom#293);
//! - `indeterminate-*`: a script-set indeterminate checkbox (#293). A screenshot has no
//!   script, so the page is styled here and each `data-indeterminate` element is handed
//!   the flag as [`turbo_dom_layout::INDETERMINATE_ATTR`], as a live embedder does from
//!   its `post_cascade` hook;
//! - `vertical-*`: every control turned in `vertical-lr` / `vertical-rl` (#293);
//! - `vline-*`: every control on a `vertical-lr` / `vertical-rl` line, in an absolutely
//!   positioned and a static wrapper (miaskiewicz/turbo-dom#345);
//! - `audio-*`: the `<audio controls>` panel (miaskiewicz/turbo-dom#292).
//!
//! Each case is laid out at Chrome's 1280x800 viewport and painted through `paint_png` and
//! `paint_svg` (rasterized by resvg); every probe's colour class must be Chrome's, and a
//! probe outside the six classes (`?`) Chrome's RGB within [`RGB_TOLERANCE`]. The boxes
//! come from `layout_boxes`, within 0.01px of Chrome's.
use std::collections::HashMap;

use turbo_dom_layout::html_layout::propagate_canvas_background;
use turbo_dom_layout::text::FontRegistry;
use turbo_dom_layout::{
    document_root_font_px, layout_boxes, layout_in_viewport, parse_html_tree, root_font_scope,
    style_html_tree, Attr, Diagnostics, Fragment, FragmentContent, HtmlOptions, ImageCtx,
    LayoutOptions, NoImages, StyledNode, INDETERMINATE_ATTR,
};

const VIEWPORT: (u32, u32) = (1280, 800);

/// The widest per-channel difference from Chrome's RGB a `?` probe may have.
const RGB_TOLERANCE: u8 = 8;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../tests/fixtures/form_native_chrome156.json")).unwrap()
}

/// Chrome's colour class (the control script's `classOf`): a channel above 160 is high,
/// below 96 low.
fn class([r, g, b]: [u8; 3]) -> char {
    let level = |v: u8| {
        if v > 160 {
            2
        } else {
            u8::from(v >= 96)
        }
    };
    match (level(r), level(g), level(b)) {
        (2, 2, 2) => 'W',
        (0, 2, 2) => 'T',
        (2, 0, 0) => 'R',
        (0, 2, 0) => 'G',
        (0, 0, 2) => 'B',
        (2, 2, 0) => 'Y',
        _ => '?',
    }
}

/// Why `got` differs from Chrome's probe, if it does.
fn probe_difference(probe: &serde_json::Value, got: [u8; 3]) -> Option<String> {
    let (x, y) = (probe["x"].as_u64().unwrap(), probe["y"].as_u64().unwrap());
    let want = probe["class"].as_str().unwrap();
    let class = class(got);
    if class.to_string() != want {
        return Some(format!("({x},{y}): {class} {got:?}, Chrome {want}"));
    }
    if want != "?" {
        return None;
    }
    let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
    let far = got
        .iter()
        .zip(chrome)
        .any(|(&g, c)| g.abs_diff(c) > RGB_TOLERANCE);
    far.then(|| format!("({x},{y}): {got:?}, Chrome {chrome:?}"))
}

/// Mark every element with a `data-indeterminate` attribute indeterminate.
fn mark_indeterminate(nodes: &mut [StyledNode]) {
    for node in nodes {
        let StyledNode::Element(el) = node else {
            continue;
        };
        if el.attrs.iter().any(|a| a.name == "data-indeterminate") {
            el.attrs.push(Attr {
                name: INDETERMINATE_ATTR.into(),
                value: String::new(),
            });
        }
        mark_indeterminate(&mut el.children);
    }
}

/// The case's galley, laid out as the screenshot entries lay a page out (its canvas
/// background propagated), with the indeterminate flags set.
fn galley(html: &str) -> Fragment {
    let tree = parse_html_tree(html);
    let options = HtmlOptions {
        uncapped_images: true,
        ..HtmlOptions::default()
    };
    let width = VIEWPORT.0 as f32;
    let mut styled = style_html_tree(&tree, "", width, &options);
    mark_indeterminate(&mut styled);
    let canvas = propagate_canvas_background(&mut styled);
    let _root_font = root_font_scope(document_root_font_px(&styled));
    let layout = LayoutOptions {
        uncapped_images: true,
        ..LayoutOptions::default()
    };
    let mut diags = Diagnostics::default();
    let mut galley = layout_in_viewport(
        &styled,
        (width, VIEWPORT.1 as f32),
        &FontRegistry::new(),
        &ImageCtx {
            resolver: &NoImages,
            body_height: None,
        },
        &layout,
        &mut diags,
    );
    if let FragmentContent::Box { background, .. } = &mut galley.content {
        *background = canvas;
    }
    galley
}

type Pixels = Box<dyn Fn(u32, u32) -> [u8; 3]>;

fn png(galley: &Fragment) -> Pixels {
    let assets = crate::image_paint::decode_all(&HashMap::new());
    let bg = crate::canvas_background(galley);
    let png = crate::paint_png::paint(galley, VIEWPORT.0, VIEWPORT.1, bg, &assets).unwrap();
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    Box::new(move |x, y| {
        let c = pm.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue()]
    })
}

fn svg(galley: &Fragment) -> Pixels {
    use resvg::{tiny_skia as rsk, usvg};
    let assets = crate::image_paint::decode_all(&HashMap::new());
    let bg = crate::canvas_background(galley);
    let svg = crate::paint_svg::paint(galley, VIEWPORT.0, VIEWPORT.1, bg, &assets);
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let mut pm = rsk::Pixmap::new(VIEWPORT.0, VIEWPORT.1).unwrap();
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    Box::new(move |x, y| {
        let c = pm.pixel(x, y).unwrap().demultiply();
        [c.red(), c.green(), c.blue()]
    })
}

/// Every probe and box difference from Chrome of the cases whose id starts with `prefix`.
fn differences(prefix: &str) -> Vec<String> {
    let fixture = fixture();
    let fonts = FontRegistry::new();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !id.starts_with(prefix) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let galley = galley(html);
        for (painter, pixels) in [("png", png(&galley)), ("svg", svg(&galley))] {
            for probe in case["probes"].as_array().unwrap() {
                let (x, y) = (probe["x"].as_u64().unwrap(), probe["y"].as_u64().unwrap());
                if let Some(d) = probe_difference(probe, pixels(x as u32, y as u32)) {
                    failures.push(format!("{id} {painter} {d}"));
                }
            }
        }
        let mut diags = Diagnostics::default();
        let boxes = layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).unwrap();
        for want in case["boxes"].as_array().unwrap() {
            let cid = want["cid"].as_str().unwrap();
            let Some(b) = boxes.iter().find(|b| b.cid == cid) else {
                failures.push(format!("{id}: no box for data-cid={cid}"));
                continue;
            };
            for (field, got) in [
                ("x", b.x),
                ("y", b.y),
                ("width", b.width),
                ("height", b.height),
            ] {
                let chrome = want[field].as_f64().unwrap() as f32;
                if (got - chrome).abs() > 0.01 {
                    failures.push(format!("{id}: {cid}.{field} = {got}, Chrome {chrome}"));
                }
            }
        }
    }
    assert!(compared > 0, "no case starts with {prefix}");
    failures
}

fn assert_like_chrome(prefix: &str) {
    let failures = differences(prefix);
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn progress_bars_are_native_controls_like_chrome_156() {
    assert_like_chrome("progress-");
}

#[test]
fn meters_are_native_controls_like_chrome_156() {
    assert_like_chrome("meter-");
}

#[test]
fn indeterminate_checkboxes_paint_a_dash_like_chrome_156() {
    assert_like_chrome("indeterminate-");
}

#[test]
fn vertical_controls_turn_like_chrome_156() {
    assert_like_chrome("vertical-");
}

#[test]
fn controls_sit_on_a_vertical_lines_central_baseline_like_chrome_156() {
    assert_like_chrome("vline-");
}

#[test]
fn audio_controls_paint_chromes_panel() {
    assert_like_chrome("audio-");
}

#[test]
fn probes_compare_classes_then_chromes_rgb() {
    let probe = serde_json::json!({"x": 1, "y": 2, "class": "?", "rgb": [100, 100, 100]});
    assert_eq!(probe_difference(&probe, [108, 92, 100]), None);
    assert!(probe_difference(&probe, [109, 100, 100]).is_some());
    let blue = serde_json::json!({"x": 1, "y": 2, "class": "B", "rgb": [0, 0, 255]});
    assert_eq!(probe_difference(&blue, [20, 20, 200]), None);
    assert!(probe_difference(&blue, [100, 100, 100]).is_some());
}

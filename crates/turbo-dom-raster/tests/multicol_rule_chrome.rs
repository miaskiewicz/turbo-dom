//! Column-rule (CSS Multicol 1 §5) paint against Chrome 156
//! (miaskiewicz/turbo-dom#398 PR-D).
//!
//! The fixture `tests/fixtures/multicol_rule_chrome156.json` (captured by
//! `scripts/multicol-rule-chrome-control.mjs`) lists seven `column-rule-*`
//! cases, each with a page rendered over the cyan backdrop and probe points
//! whose colour class (`R`/`G`/`B`/`Y`/`W`/`T`/`?`) and raw RGB Chrome 156
//! records. Each case is screenshot here (PNG and SVG, the latter rasterized
//! through resvg) and the probes compared with the same tolerance the rest of
//! the paint tests use (`paint_chrome.rs` `probe_difference` contract).
//!
//! Styles covered: `none`, `solid`, `dotted`, `dashed`, `double`, plus
//! `currentcolor` colour and a gap-clamp case. The `groove` / `ridge` /
//! `inset` / `outset` family is deferred to miaskiewicz/turbo-dom#419.
use std::collections::HashSet;

use turbo_dom_raster::{screenshot_png_with_opts, screenshot_svg_with_opts, ImageAssets, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

/// The widest per-channel difference from Chrome's RGB a `?` probe may have —
/// the same tolerance the main paint control uses.
const RGB_TOLERANCE: u8 = 8;

fn fixture() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/multicol_rule_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

/// Classify a pixel into the same six colour classes the oracle records: `W`
/// (white-ish), `T` (cyan backdrop), `R`, `G`, `B`, `Y`, else `?`.
fn class([r, g, b]: [u8; 3]) -> char {
    match (r > 160, g > 160, b > 160, r < 96, g < 96, b < 96) {
        (true, true, true, _, _, _) => 'W',
        (_, true, true, true, _, _) => 'T',
        (true, _, _, _, true, true) => 'R',
        (_, true, _, true, _, true) => 'G',
        (_, _, true, true, true, _) => 'B',
        (true, true, _, _, _, true) => 'Y',
        _ => '?',
    }
}

/// `Some(reason)` when `got` (the screenshot's RGB) differs from Chrome's at
/// `probe`: a different colour class, or — for `?` probes — a channel more
/// than [`RGB_TOLERANCE`] away from Chrome's stored RGB.
fn probe_difference(probe: &serde_json::Value, got: [u8; 3]) -> Option<String> {
    let (x, y) = (
        probe["x"].as_u64().unwrap() as u32,
        probe["y"].as_u64().unwrap() as u32,
    );
    let want = probe["class"].as_str().unwrap();
    let c = class(got);
    if c.to_string() != want {
        return Some(format!("({x},{y}): {c} {got:?}, Chrome {want}"));
    }
    if want != "?" {
        return None;
    }
    let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
    let far = got
        .iter()
        .zip(chrome)
        .any(|(&g, c)| g.abs_diff(c) > RGB_TOLERANCE);
    far.then(|| format!("({x},{y}): {got:?}, Chrome {chrome:?} (tolerance {RGB_TOLERANCE})"))
}

/// Every PNG paint difference from Chrome of the cases `select` picks.
fn png_differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let png = screenshot_png_with_opts(html, "", VIEWPORT, &ImageAssets::new(), false, false)
            .expect("png");
        let pm = tiny_skia::Pixmap::decode_png(&png).expect("decode");
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds");
            if let Some(d) = probe_difference(probe, [c.red(), c.green(), c.blue()]) {
                failures.push(format!("{id} png {d}"));
            }
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// The SVG painter's output, rasterized through resvg, probed like the PNG.
fn rasterize_svg(svg: &str) -> resvg::tiny_skia::Pixmap {
    use resvg::{tiny_skia as rsk, usvg};
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("valid svg");
    let mut pm = rsk::Pixmap::new(VIEWPORT.width, VIEWPORT.height).unwrap();
    resvg::render(&tree, rsk::Transform::identity(), &mut pm.as_mut());
    pm
}

fn svg_differences(select: impl Fn(&str) -> bool) -> Vec<String> {
    let fixture = fixture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !select(id) {
            continue;
        }
        compared += 1;
        let html = case["html"].as_str().unwrap();
        let svg = screenshot_svg_with_opts(html, "", VIEWPORT, &ImageAssets::new(), false, false)
            .expect("svg");
        let pm = rasterize_svg(&svg);
        for probe in case["probes"].as_array().unwrap() {
            let (x, y) = (
                probe["x"].as_u64().unwrap() as u32,
                probe["y"].as_u64().unwrap() as u32,
            );
            let c = pm.pixel(x, y).expect("in bounds").demultiply();
            if let Some(d) = probe_difference(probe, [c.red(), c.green(), c.blue()]) {
                failures.push(format!("{id} svg {d}"));
            }
        }
    }
    assert!(compared > 0, "no case selected");
    failures
}

/// `column-rule-style: solid` paints a filled rule band centred in the
/// `column-gap`, in both painters.
#[test]
fn solid_column_rules_paint_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-solid-1col-gap";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `column-rule-style: dotted` stamps a dot of diameter `column-rule-width`
/// at the rule top in both painters (Chrome's exact dot phase past the first
/// dot is a Blink `LayoutUnit` arithmetic detail outside PR-D's scope).
#[test]
fn dotted_column_rules_paint_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-dotted-3col";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `column-rule-style: double` paints two bands of `floor(width / 3)` thick
/// at the rule's long edges with a hollow centre, in both painters.
#[test]
fn double_column_rules_paint_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-double-thick";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `column-rule-style: dashed` stamps a dash at the rule top in both
/// painters.
#[test]
fn dashed_column_rules_paint_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-dashed";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `column-rule-color: currentcolor` resolves to the container's own `color`
/// (initial black; here overridden to red), in both painters.
#[test]
fn currentcolor_column_rules_paint_in_container_color_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-currentcolor";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `column-rule-style: none` suppresses the rule — the gap paints what is
/// behind it, in both painters.
#[test]
fn no_column_rule_is_painted_for_style_none_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-none";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A `column-rule-width` wider than the gap is clamped by Chrome to the gap
/// (Blink's multicol painter never paints outside the gap), so the rule is
/// only the gap wide. Both painters match.
#[test]
fn wider_than_gap_column_rules_are_clamped_to_the_gap_like_chrome_156() {
    let pick = |id: &str| id == "multicol-rule-wider-than-gap";
    let mut failures = png_differences(pick);
    failures.extend(svg_differences(pick));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every case in the fixture is picked by exactly one of the suites above,
/// so `cargo test` never silently skips a Chrome case the capture script
/// writes. If a new case is added, add a pattern matcher for it (or extend
/// an existing one) so it is asserted.
#[test]
fn every_fixture_case_is_covered() {
    let pick = |_id: &str| true;
    let cases: Vec<String> = fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(cases.iter().collect::<HashSet<_>>().len(), cases.len());
    assert_ne!(cases.len(), 0, "fixture lists at least one case");
    // Every case must be addressed by one of the test selectors above.
    let covered: Vec<&str> = cases
        .iter()
        .filter(|id| {
            matches!(
                id.as_str(),
                "multicol-rule-solid-1col-gap"
                    | "multicol-rule-dotted-3col"
                    | "multicol-rule-double-thick"
                    | "multicol-rule-none"
                    | "multicol-rule-wider-than-gap"
                    | "multicol-rule-dashed"
                    | "multicol-rule-currentcolor"
            )
        })
        .map(String::as_str)
        .collect();
    assert_eq!(covered.len(), cases.len(), "uncovered cases: {cases:?}");
    assert!(pick("any"));
}

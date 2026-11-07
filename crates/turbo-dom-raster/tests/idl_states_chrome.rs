//! `ScreenshotOptions::idl_states` carries the IDL-state flags a live embedder
//! (turbo-surf) sets at runtime into a static screenshot (miaskiewicz/turbo-dom#371).
//! A snapshot has no script to toggle `input.indeterminate`, so the markup alone
//! cannot pick the Chrome indeterminate dash; the caller names the input by its
//! tree-order index and the raster paints the dash in that box.
//!
//! The pixel probes come from Chrome 156's `indeterminate-*` cases in
//! `tests/fixtures/form_native_chrome156.json` (`scripts/form-native-chrome-control.mjs`),
//! captured with `el.indeterminate = true` set from JS — exactly the IDL state
//! `idl_states` encodes.

use std::collections::HashSet;

use turbo_dom_raster::{
    screenshot_png_with_options, IdlStates, ImageAssets, ScreenshotOptions, Viewport,
};

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
};

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

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/form_native_chrome156.json")).unwrap()
}

fn case(id: &str) -> serde_json::Value {
    let fixture = fixture();
    let entries = fixture["cases"].as_array().unwrap();
    entries
        .iter()
        .find(|e| e["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("case {id} not found"))
        .clone()
}

fn pixels(png: &[u8]) -> impl Fn(u32, u32) -> [u8; 3] {
    let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
    move |x, y| {
        let c = pm.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue()]
    }
}

fn render_indeterminate_case(id: &str) -> Vec<u8> {
    let entry = case(id);
    let html = entry["html"].as_str().unwrap().to_string();
    // The one `<input>` in each `indeterminate-*` case: index 0 among
    // `document.querySelectorAll('input')`.
    let mut inputs = HashSet::new();
    inputs.insert(0);
    let idl = IdlStates {
        indeterminate_inputs: inputs,
    };
    let mut options = ScreenshotOptions::default();
    options.idl_states = Some(&idl);
    let assets = ImageAssets::new();
    screenshot_png_with_options(&html, "", VIEWPORT, &assets, &options).unwrap()
}

fn assert_probes_match(id: &str) {
    let png = render_indeterminate_case(id);
    let read = pixels(&png);
    let entry = case(id);
    let mut mismatches: Vec<String> = Vec::new();
    for probe in entry["probes"].as_array().unwrap() {
        let x = probe["x"].as_u64().unwrap() as u32;
        let y = probe["y"].as_u64().unwrap() as u32;
        let want = probe["class"].as_str().unwrap();
        let got = read(x, y);
        let got_class = class(got);
        if got_class.to_string() != want {
            mismatches.push(format!(
                "({x},{y}): got {got_class} {got:?}, want Chrome {want}"
            ));
            continue;
        }
        if want == "?" {
            let chrome: [u8; 3] = serde_json::from_value(probe["rgb"].clone()).unwrap();
            let far = got.iter().zip(chrome).any(|(&g, c)| g.abs_diff(c) > 8);
            if far {
                mismatches.push(format!("({x},{y}): got {got:?}, Chrome {chrome:?}"));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{id} mismatches: {}",
        mismatches.join("; ")
    );
}

#[test]
fn indeterminate_checkbox_paints_with_idl_state() {
    assert_probes_match("indeterminate-checkbox");
}

#[test]
fn indeterminate_checked_paints_dash_over_check() {
    assert_probes_match("indeterminate-checked");
}

#[test]
fn indeterminate_radio_ignores_the_flag_like_chrome() {
    assert_probes_match("indeterminate-radio");
}

#[test]
fn without_idl_states_a_bare_checkbox_is_unchecked() {
    // The same HTML as `indeterminate-checkbox` (its `data-indeterminate` attribute
    // is inert in Chrome and in the raster) with `idl_states` left `None` must
    // paint an unchecked box: the inner probe (22,22) is the control's white
    // interior, not the Chrome indeterminate blue.
    let entry = case("indeterminate-checkbox");
    let html = entry["html"].as_str().unwrap();
    let options = ScreenshotOptions::default();
    let assets = ImageAssets::new();
    let png = screenshot_png_with_options(html, "", VIEWPORT, &assets, &options).unwrap();
    let read = pixels(&png);
    let inside = read(22, 22);
    let is_blue = inside[2] > 160 && inside[0] < 96 && inside[1] < 160;
    assert!(
        !is_blue,
        "no IDL state should leave the checkbox unchecked, got {inside:?}"
    );
}

#[test]
fn an_empty_idl_state_set_renders_like_no_idl_states() {
    // `Some` with no indeterminate index is the same no-op as `None`: the render is
    // byte-for-byte the markup-only screenshot.
    let entry = case("indeterminate-checkbox");
    let html = entry["html"].as_str().unwrap();
    let assets = ImageAssets::new();
    let plain =
        screenshot_png_with_options(html, "", VIEWPORT, &assets, &ScreenshotOptions::default())
            .unwrap();
    let idl = IdlStates::default();
    let mut options = ScreenshotOptions::default();
    options.idl_states = Some(&idl);
    let empty = screenshot_png_with_options(html, "", VIEWPORT, &assets, &options).unwrap();
    assert_eq!(plain, empty);
}

#[test]
fn idl_state_indices_count_inputs_in_tree_order_past_text() {
    // Two `indeterminate-checkbox` boxes (the second 40px to the right) with page text
    // before, between and inside a wrapper: only input #1 (the second in
    // `querySelectorAll('input')` order) carries the flag, so only its box paints the
    // Chrome dash (probe offsets as in the `indeterminate-checkbox` control).
    let html = "<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}\
                input{position:absolute;top:20px;margin:0}</style></head><body>a\
                <input type=checkbox style=\"left:20px\"> b <div>c\
                <input type=checkbox style=\"left:60px\"></div></body></html>";
    let mut inputs = HashSet::new();
    inputs.insert(1);
    let idl = IdlStates {
        indeterminate_inputs: inputs,
    };
    let mut options = ScreenshotOptions::default();
    options.idl_states = Some(&idl);
    let assets = ImageAssets::new();
    let png = screenshot_png_with_options(html, "", VIEWPORT, &assets, &options).unwrap();
    let read = pixels(&png);
    let near_blue = |p: [u8; 3]| {
        p.iter()
            .zip([0u8, 117, 255])
            .all(|(&g, c)| g.abs_diff(c) <= 8)
    };
    // Input #0: unchecked, a white interior.
    assert_eq!(class(read(22, 22)), 'W', "input #0 got {:?}", read(22, 22));
    // Input #1: Chrome's indeterminate fill with the white dash across the middle.
    assert!(near_blue(read(62, 22)), "input #1 got {:?}", read(62, 22));
    assert_eq!(
        class(read(67, 26)),
        'W',
        "input #1 dash got {:?}",
        read(67, 26)
    );
}

//! CanvasRenderingContext2D `fillStyle` / `strokeStyle` / `shadowColor` against Chrome 156
//! (miaskiewicz/turbo-dom#108): `canvas_style_probe.js`, recorded by
//! `scripts/invokers-chrome-control.mjs` into `canvas_style_chrome156.json`.
use super::reference_target::probe_harness::{differences, run};
use serde_json::{json, Value};
use turbo_dom_parser::rtdom::color::parse_color;

const PROBE: &str = include_str!("canvas_style_probe.js");
const CONTROL: &str = include_str!("canvas_style_chrome156.json");

/// Chrome's `getImageData` pixel for a fillRect; offline the probe returns the colour the op
/// log recorded for that fillRect instead, which turbo-dom-raster's replay paints through
/// the same `parse_color` (opaque colours only, so no premultiplication rounding).
fn as_pixels(actual: &mut Value, expected: &Value) {
    let is_pixel = |value: &Value| {
        value
            .as_array()
            .is_some_and(|pixel| pixel.len() == 4 && pixel.iter().all(Value::is_u64))
    };
    if let (Some(colour), true) = (actual.as_str(), is_pixel(expected)) {
        *actual = match parse_color(colour) {
            Some(c) => json!([c.r, c.g, c.b, c.a * 255.0].map(|channel| channel.round() as u64)),
            None => json!(format!("unparsable op-log colour {colour:?}")),
        };
        return;
    }
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            for (key, value) in e {
                if let Some(a) = a.get_mut(key) {
                    as_pixels(a, value);
                }
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            for (a, e) in a.iter_mut().zip(e) {
                as_pixels(a, e);
            }
        }
        _ => {}
    }
}

/// Reads serialize like Chrome (`#rrggbb`, else `rgba(r, g, b, a)` with Chrome's alpha
/// digits), invalid and non-colour values are ignored, gradients are kept by identity,
/// `currentcolor` resolves at assignment against the connected canvas's inline `color`
/// (else black) and the op log records the resolved colour.
#[test]
fn canvas_styles_match_chrome156() {
    use sha2::{Digest, Sha256};
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    let digest = Sha256::digest(PROBE.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    let expected = &control["result"];
    let mut actual = run(PROBE);
    as_pixels(&mut actual, expected);
    let mut out = Vec::new();
    differences("result", &actual, expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

/// save() / restore() keep a drawing-state stack like Chrome (miaskiewicz/turbo-dom#110):
/// `canvas_state_probe.js`, recorded by `scripts/invokers-chrome-control.mjs` into
/// `canvas_state_chrome156.json`. Every state attribute and the line dash read back at each
/// level of a nested save/restore, restore() on an empty stack changes nothing, a gradient
/// comes back by identity and the next fillRect paints the restored colour.
#[test]
fn canvas_save_restore_matches_chrome156() {
    use sha2::{Digest, Sha256};
    const PROBE: &str = include_str!("canvas_state_probe.js");
    let control: Value = serde_json::from_str(include_str!("canvas_state_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    let digest = Sha256::digest(PROBE.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    let expected = &control["result"];
    let mut actual = run(PROBE);
    as_pixels(&mut actual, expected);
    let mut out = Vec::new();
    differences("result", &actual, expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

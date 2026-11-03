//! CanvasRenderingContext2D `measureText` against Chrome 156 (miaskiewicz/turbo-dom#237):
//! `canvas_measure_text_probe.js`, recorded by `scripts/fixture-font-chrome-controls.mjs`
//! (the repo's fixture fonts through `@font-face`) into `canvas_measure_text_chrome156.json`.
//! Offline the host registers the same fixture faces with [`set_canvas_fonts`].
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use serde_json::Value;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("canvas_measure_text_probe.js");
const CONTROL: &str = include_str!("canvas_measure_text_chrome156.json");

/// Chrome's glyph ink bounds come from Skia's outline bounds in `f32` (the control carries
/// their rounding noise, about 1e-5 px); widths and font metrics match exactly.
const INK_TOLERANCE: f64 = 1e-4;

fn run(fonts: Option<turbo_dom_layout::FontRegistry>, probe: &str) -> Value {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    if let Some(fonts) = fonts {
        set_canvas_fonts(context, fonts);
    }
    let code = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let value = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("probe threw: {}", thrown(scope.exception(), scope)))
        .to_rust_string_lossy(scope);
    reset();
    serde_json::from_str(&value).unwrap()
}

fn compare(label: &str, keys: &[Value], actual: &Value, expected: &Value, out: &mut Vec<String>) {
    for (index, key) in keys.iter().enumerate() {
        let (a, e) = (&actual[index], &expected[index]);
        let ink = key.as_str().is_some_and(|key| key.starts_with("actual"));
        let same = match (a.as_f64(), e.as_f64()) {
            (Some(a), Some(e)) if ink => (a - e).abs() <= INK_TOLERANCE,
            (Some(a), Some(e)) => a == e,
            _ => a == e,
        };
        if !same {
            out.push(format!("{label} {key}: got {a}, Chrome {e}"));
        }
    }
}

/// Width (Blink's word-by-word 16.16 advances), the ink box, the font box, and the
/// baselines for each canvas `font`, then the textAlign / textBaseline offsets; the
/// `emHeight*` fields are not exposed by Chrome 156 (undefined).
#[test]
fn measure_text_matches_chrome156() {
    let control: Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let digest = Sha256::digest(PROBE.as_bytes());
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    let expected = &control["result"];
    let actual = run(Some(turbo_dom_layout::test_support::fixture_fonts()), PROBE);
    let keys = expected["keys"].as_array().unwrap();
    assert_eq!(&actual["keys"], &expected["keys"]);
    let mut out = Vec::new();
    for section in ["metrics", "aligned"] {
        let (a, e) = (
            actual[section].as_array().unwrap(),
            expected[section].as_array().unwrap(),
        );
        assert_eq!(a.len(), e.len(), "{section} rows");
        for (a, e) in a.iter().zip(e) {
            assert_eq!(a[0], e[0]);
            assert_eq!(a[1], e[1]);
            compare(
                &format!("{section} {} {}", e[0], e[1]),
                keys,
                &a[2],
                &e[2],
                &mut out,
            );
        }
    }
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
}

/// Without host fonts the canvas measures with turbo-dom-layout's bundled faces (the
/// generic families resolve to them): a real advance, never a per-character constant.
#[test]
fn measure_text_without_host_fonts_uses_the_bundled_faces() {
    let probe = "(() => { const c = document.createElement('canvas').getContext('2d');
        const w = (t) => c.measureText(t).width;
        return [w('iiii'), w('WWWW'), c.measureText('x').fontBoundingBoxAscent > 0]; })()";
    let actual = run(None, probe);
    let narrow = actual[0].as_f64().unwrap();
    let wide = actual[1].as_f64().unwrap();
    assert!(narrow > 0.0 && wide > narrow * 2.0, "{actual}");
    assert_eq!(actual[2], true);
}

/// The control's result, after checking it was captured from `probe` by Chrome 156.
fn checked_control(control: &str, probe: &str) -> Value {
    let control: Value = serde_json::from_str(control).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let digest = Sha256::digest(probe.as_bytes());
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    control["result"].clone()
}

/// `TextMetrics` is a global interface like Chrome 156's: not constructible, its fields are
/// enumerable prototype getters (Chrome's order and names) that reject other receivers, and
/// every measureText() returns a fresh instance with no own properties.
#[test]
fn text_metrics_interface_matches_chrome156() {
    const PROBE: &str = include_str!("canvas_text_metrics_interface_probe.js");
    let expected = checked_control(
        include_str!("canvas_text_metrics_interface_chrome156.json"),
        PROBE,
    );
    let actual = run(Some(turbo_dom_layout::test_support::fixture_fonts()), PROBE);
    let mut out = Vec::new();
    super::reference_target::probe_harness::differences("result", &actual, &expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

/// letterSpacing / wordSpacing parse CSS lengths and ignore invalid values; fontKerning,
/// fontStretch and textRendering accept only their enum values; save/restore keeps them;
/// and measureText widths follow Chrome 156: spacing after every character (and every
/// space), em against the current font, no kerning for `fontKerning: none`, whole-run
/// shaping (kerning across spaces) for `normal` / optimizeLegibility / geometricPrecision.
#[test]
fn text_attributes_match_chrome156() {
    const PROBE: &str = include_str!("canvas_text_attributes_probe.js");
    let mut expected =
        checked_control(include_str!("canvas_text_attributes_chrome156.json"), PROBE);
    let mut actual = run(Some(turbo_dom_layout::test_support::fixture_fonts()), PROBE);
    let rows = |value: &mut Value| value["widths"].take();
    let (actual_rows, expected_rows) = (rows(&mut actual), rows(&mut expected));
    let mut out = Vec::new();
    super::reference_target::probe_harness::differences("result", &actual, &expected, &mut out);
    let keys = ["width", "actualBoundingBoxLeft", "actualBoundingBoxRight"].map(Value::from);
    let (a, e) = (
        actual_rows.as_array().unwrap(),
        expected_rows.as_array().unwrap(),
    );
    assert_eq!(a.len(), e.len(), "width rows");
    for (a, e) in a.iter().zip(e) {
        assert_eq!((&a[0], &a[1], &a[2]), (&e[0], &e[1], &e[2]));
        compare(
            &format!("{} {} {:?}", e[0], e[1], e[2]),
            &keys,
            &a[3],
            &e[3],
            &mut out,
        );
    }
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
}

/// Every CSS length unit Chrome 156 accepts for letterSpacing / wordSpacing (and nothing
/// else), and what one unit adds per character: Chrome resolves the root-relative units
/// against the canvas font itself (rem = em, rex = ex, ...), `ic` without a CJK glyph is
/// 1em, and the line-height, viewport and container units resolve to 0 for a canvas,
/// attached to the document or not.
#[test]
fn spacing_units_match_chrome156() {
    const PROBE: &str = include_str!("canvas_spacing_units_probe.js");
    let mut expected = checked_control(include_str!("canvas_spacing_units_chrome156.json"), PROBE);
    let mut actual = run(Some(turbo_dom_layout::test_support::fixture_fonts()), PROBE);
    assert_eq!(actual["accepted"], expected["accepted"]);
    let mut out = Vec::new();
    for placement in ["attached", "detached"] {
        let rows = |value: &mut Value| value["perUnit"][placement].take();
        let (a, e) = (rows(&mut actual), rows(&mut expected));
        for (unit, want) in e.as_object().unwrap() {
            let got = a[unit].as_f64();
            if got.is_none_or(|got| (got - want.as_f64().unwrap()).abs() > INK_TOLERANCE) {
                out.push(format!(
                    "{placement} 1{unit}: got {:?}, Chrome {want}",
                    a[unit]
                ));
            }
        }
    }
    assert!(out.is_empty(), "{}", out.join("\n"));
}

/// `ic` with a face that has U+6C34 (`test_support::ic_fixture_face`, 0.9em) is that
/// glyph's advance, as in Chrome 156 (`canvas_ic_unit_chrome156.json`); `ric` resolves
/// against the canvas font too.
#[test]
fn ic_unit_measures_the_water_glyph_like_chrome156() {
    const PROBE: &str = include_str!("canvas_ic_unit_probe.js");
    let expected = checked_control(include_str!("canvas_ic_unit_chrome156.json"), PROBE);
    let mut fonts = turbo_dom_layout::test_support::fixture_fonts();
    fonts.add(turbo_dom_layout::test_support::ic_fixture_face());
    let actual = run(Some(fonts), PROBE);
    let mut out = Vec::new();
    for (size, row) in expected.as_object().unwrap() {
        for (key, want) in row.as_object().unwrap() {
            let got = actual[size][key].as_f64();
            if got.is_none_or(|got| (got - want.as_f64().unwrap()).abs() > INK_TOLERANCE) {
                out.push(format!(
                    "{size}px {key}: got {:?}, Chrome {want}",
                    actual[size][key]
                ));
            }
        }
    }
    assert!(out.is_empty(), "{}", out.join("\n"));
}

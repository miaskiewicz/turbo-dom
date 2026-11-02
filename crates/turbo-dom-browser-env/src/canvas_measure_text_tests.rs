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
    let isolate = &mut v8::Isolate::new(Default::default());
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

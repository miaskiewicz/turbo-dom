//! Canvas `measureText` widths against Chrome 156 (miaskiewicz/turbo-dom#191): the canvas
//! `font` shorthand's weight and style pick distinct faces (700 vs 400, italic vs upright),
//! and the width is Blink's canvas measure: the text shaped word by word (a space is its own
//! word, so no kerning crosses it) with each glyph advance cast to a 16.16 `HarfBuzz`
//! position. Oracle: the `canvas` rows of turbo-dom-layout's
//! `tests/fixtures/text-advances/chrome156.json` (Chrome 156.0.8078.47 headless, the fixture
//! fonts through `@font-face`; canvas text resolves the same font files from the bundled set).

use turbo_dom_raster::measure_canvas_text;

#[test]
fn canvas_measure_text_matches_chrome_156_per_weight_and_style() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "../../turbo-dom-layout/tests/fixtures/text-advances/chrome156.json"
    ))
    .unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let rows = control["result"]["canvas"].as_array().unwrap();
    assert_eq!(rows.len(), 36);
    let mut failures = Vec::new();
    for row in rows {
        let font = row["font"].as_str().unwrap();
        let text = row["text"].as_str().unwrap();
        let want = row["width"].as_f64().unwrap();
        let got = measure_canvas_text(text, font);
        if got != want {
            failures.push(format!("{font} {text:?}: got {got}, Chrome {want}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn canvas_bold_and_regular_resolve_distinct_faces() {
    for family in ["\"Liberation Serif\"", "Inter", "serif", "sans-serif"] {
        let regular = measure_canvas_text("Typography", &format!("16px {family}"));
        let bold = measure_canvas_text("Typography", &format!("bold 16px {family}"));
        let numeric = measure_canvas_text("Typography", &format!("700 16px {family}"));
        assert!(bold > regular, "{family}: bold {bold} vs regular {regular}");
        assert_eq!(bold, numeric, "{family}: bold == 700");
    }
}

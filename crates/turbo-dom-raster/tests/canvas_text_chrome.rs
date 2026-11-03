//! Canvas `fillText` glyph positions against a Chrome 156 pixel control
//! (miaskiewicz/turbo-dom#236): the pen advances word by word with Blink's 16.16 glyph
//! advances, the widths `measureText` reports, so no kerning crosses a space. Oracle:
//! `tests/fixtures/canvas_text_chrome156.json` from `scripts/canvas-text-chrome-control.mjs`
//! (Chrome 156.0.8078.47 headless, the fixture fonts through `@font-face`; the raster draws
//! the same files from its bundled set): the ink column runs of large text, whose edges must
//! agree to a pixel (anti-aliasing differs between Skia and tiny-skia; a pen error moves
//! whole runs by several pixels at these sizes).

use turbo_dom_raster::canvas_ops_rgba;

fn ink_runs(rgba: &[u8], width: usize, height: usize) -> Vec<[usize; 2]> {
    let mut runs = Vec::new();
    let mut start = None;
    for x in 0..=width {
        let ink = x < width && (0..height).any(|y| rgba[(y * width + x) * 4 + 3] >= 128);
        match (ink, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                runs.push([s, x]);
                start = None;
            }
            _ => {}
        }
    }
    runs
}

#[test]
fn fill_text_glyph_runs_match_chrome_156() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/canvas_text_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let [width, height] = [0, 1].map(|i| control["canvas"][i].as_u64().unwrap() as usize);
    let baseline = &control["baseline"];
    let cases = control["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    let mut failures = Vec::new();
    for case in cases {
        let (font, text) = (
            case["font"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
        );
        let ops = serde_json::json!([[
            "fillText",
            [text, baseline[0], baseline[1]],
            "#000000",
            null,
            font
        ]]);
        let rgba = canvas_ops_rgba(width as u32, height as u32, &ops.to_string()).unwrap();
        let got = ink_runs(&rgba, width, height);
        let want: Vec<[usize; 2]> = serde_json::from_value(case["runs"].clone()).unwrap();
        let close = got.len() == want.len()
            && got
                .iter()
                .flatten()
                .zip(want.iter().flatten())
                .all(|(a, b)| a.abs_diff(*b) <= 1);
        if !close {
            failures.push(format!(
                "{font} {text:?}:\n  got   {got:?}\n  want  {want:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

//! Vertical metrics of `@font-face` web fonts against Chrome 156 (miaskiewicz/turbo-dom#304).
//! Oracle: `tests/fixtures/web-font-metrics/chrome156.json` from
//! `scripts/web-font-metrics-chrome-control.mjs`: every fixture font in `assets/fonts`, loaded
//! as a web font, at 6..64px in quarter-pixel steps and device scale factors 1 and 2. Each row
//! is the inline content box height, its ascent (span top to baseline) and the
//! `line-height: normal` line box height (not compared for `IcFixture`, which has no `x`: its
//! line box takes a fallback font's metrics too). Chrome reads the `hhea` ascender, descender
//! and line gap (the OS/2 `USE_TYPO_METRICS` flag is ignored, see Evolventa); for a TrueType
//! font each is a 16.16 fixed-point fraction of the em before scaling (IBM Plex Mono's
//! 1025/1000 em ascent is 20px at 20px, not 20.5 rounded up), for a CFF font the exact ratio.

use sha2::{Digest, Sha256};
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, NativeTextOptions, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::parse_html;
use turbo_dom_layout::{build_cascade, Diagnostics, FontFace, FontRegistry, ImageCtx, NoImages};

fn control() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/web-font-metrics/chrome156.json")).unwrap()
}

fn asset(path: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/assets/fonts/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn web_font_html_metrics_match_chrome() {
    let control = control();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let fonts = control["fonts"].as_array().unwrap();
    assert_eq!(fonts.len(), 33);
    let mut failures = Vec::new();
    for font in fonts {
        let path = font["path"].as_str().unwrap();
        let bytes = asset(path);
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            font["sha256"].as_str().unwrap()
        );
        let face = FontFace::from_web_font_bytes(bytes, "wf", 400, false).unwrap();
        for dsf in [1.0_f32, 2.0] {
            let rows = font[format!("dsf{dsf}")]["rows"].as_array().unwrap();
            assert_eq!(rows.len(), 233, "{path}");
            for row in rows {
                let n = |i: usize| row[i].as_f64().unwrap() as f32;
                let (size, want) = (n(0), (n(2), n(1), n(3)));
                let m = face.html_metrics(size * dsf);
                let mut got = (
                    m.ascent / dsf,
                    (m.ascent + m.descent) / dsf,
                    (m.ascent + m.descent + m.line_gap) / dsf,
                );
                if !face.has_glyph('x') {
                    got.2 = want.2;
                }
                if got != want {
                    failures.push(format!(
                        "{path} dsf{dsf} {size}px: (ascent, height, line height) {got:?} vs chrome {want:?}"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The inline content height of `text` laid out in the web face `family` at `size`px, at
/// device scale `scale`, read from its Range box (the text's content area).
fn content_height(fonts: &FontRegistry, size: f32, scale: f32) -> f32 {
    let nodes = parse_html(&format!(
        "<div style='font-family:\"wf\";font-size:{size}px;white-space:nowrap'>x</div>"
    ))
    .unwrap();
    let id = |id, path: &[usize]| SourceIdentity {
        id: SourceId(id),
        path: path.to_vec(),
    };
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &[id(1, &[0]), id(2, &[0, 0])],
        &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
        &NativeTextOptions {
            original_text: &[],
            svg_viewports: &[],
            layout_scale: scale,
        },
        &SnapshotInputs {
            width: 800.0,
            height: 600.0,
            fonts,
            images: &ImageCtx {
                resolver: &NoImages,
                body_height: None,
            },
        },
        &mut Diagnostics::default(),
    )
    .unwrap();
    let boxes = snapshot.text_range_boxes(SourceId(2), 0, 1).unwrap();
    boxes[0].rect.height
}

#[test]
fn a_web_face_lays_text_out_at_chromes_content_height() {
    // turbo-surf#130's case: IBM Plex Mono as a web font at 20px is 25px tall in Chrome 156.
    let control = control();
    let font = control["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "ibm-plex-mono/IBMPlexMono-Regular.ttf")
        .unwrap();
    let mut fonts = FontRegistry::new();
    fonts.add(
        FontFace::from_web_font_bytes(
            asset("ibm-plex-mono/IBMPlexMono-Regular.ttf"),
            "wf",
            400,
            false,
        )
        .unwrap(),
    );
    for (dsf, sizes) in [(1.0_f32, [20.0, 60.0]), (2.0, [10.0, 30.0])] {
        let rows = font[format!("dsf{dsf}")]["rows"].as_array().unwrap();
        for size in sizes {
            let row = rows.iter().find(|r| r[0].as_f64() == Some(size)).unwrap();
            let want = row[1].as_f64().unwrap() as f32;
            assert_eq!(
                content_height(&fonts, size as f32, dsf),
                want,
                "dsf{dsf} {size}px"
            );
        }
    }
}

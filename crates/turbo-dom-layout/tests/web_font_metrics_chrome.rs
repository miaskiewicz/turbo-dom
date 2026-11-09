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
use turbo_dom_layout::test_support::fixture_fonts;
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
    // turbo-surf#130's case: IBM Plex Mono as a web font at 20px is 25 CSS px tall in
    // Chrome 156 at DSF 1, and the live path at `layout_scale: 2.0` matches Chrome at
    // `--force-device-scale-factor=2` (miaskiewicz/turbo-dom#386, follow-up to #377):
    // the metrics read at `size * scale` and divided back land ascent and descent on
    // the device-pixel grid, so content height at DPR 2 lands on a half-CSS-px grain
    // (IBM Plex Mono 10px -> 12.5, 30px -> 38.5; probe-captured against real Chrome at
    // `--force-device-scale-factor=2`, not Playwright's emulated DPR, which rounds in
    // CSS pixels and would return 13/39 — CLAUDE.md "Text-geometry tests name a repo
    // font"). The DSF-2 20px live path returned 26 before #377; 25 after it; the
    // CSS-px case (DSF 1) is unchanged.
    let _control = control();
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
    for (dsf, size, want) in [
        (1.0_f32, 20.0_f32, 25.0_f32),
        (1.0, 60.0, 77.0),
        (2.0, 10.0, 12.5),
        (2.0, 30.0, 38.5),
    ] {
        assert_eq!(content_height(&fonts, size, dsf), want, "dsf{dsf} {size}px");
    }
}

/// The content height of the inline `<span>` box wrapping `text` laid out in the
/// web face `family` at `size`px and device scale `scale`, read from its
/// `UsedBoxMetrics`. This reaches the live-layout "ready-height" probe path
/// (miaskiewicz/turbo-dom#387), distinct from the `text_range_boxes` path #377
/// fixed: an inline box's fragment `content_height` is set from
/// `face.html_metrics(size * scale) / scale`, which for an `@font-face` face
/// double-quantizes at DSF 2 (26 vs Chrome's 25 for IBM Plex Mono 20px).
fn inline_box_content_height(fonts: &FontRegistry, size: f32, scale: f32) -> f32 {
    let nodes = parse_html(&format!(
        "<div><span style='font-family:\"wf\";font-size:{size}px;white-space:nowrap'>x</span></div>"
    ))
    .unwrap();
    let id = |id, path: &[usize]| SourceIdentity {
        id: SourceId(id),
        path: path.to_vec(),
    };
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        // 1: <div>, 2: <span>, 3: text "x"
        &[id(1, &[0]), id(2, &[0, 0]), id(3, &[0, 0, 0])],
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
    let span = SourceId(2);
    for frag in snapshot.fragments() {
        if frag.source_id == Some(span) {
            if let Some(used) = frag.used_box {
                return used.content_height;
            }
        }
    }
    panic!("no used_box for the <span>");
}

/// miaskiewicz/turbo-dom#387: an inline `<span>` box laid out in a web face at
/// DSF 2 sizes its content box from the CSS-size web-font metrics, not the
/// device-scaled ones divided back (Chrome's per-device-pixel rounding). This
/// is the live-layout path turbo-surf's `tests/font_face.rs` "plain"/"csp"
/// ready-height cases exercise; #377 fixed the sibling `text_range_boxes`
/// path, not this one.
#[test]
fn an_inline_box_with_a_web_face_takes_chromes_content_height() {
    let _control = control();
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
    // DSF-2 values share the real-DPR metric path unified in #386. 10px / 30px are
    // cross-checked against the direct Chrome 156 control captured by #386's
    // `a_web_face_lays_text_out_at_chromes_content_height`; 20px / 60px are captured
    // separately by `scripts/webfont-inline-dpr2-chrome-control.mjs` (span's
    // `getBoundingClientRect().height` at `--force-device-scale-factor=2`: 26 at 20px,
    // 78 at 60px).
    for (dsf, size, want) in [
        (1.0_f32, 20.0_f32, 25.0_f32),
        (1.0, 60.0, 77.0),
        (2.0, 10.0, 12.5),
        (2.0, 20.0, 26.0),
        (2.0, 30.0, 38.5),
        (2.0, 60.0, 78.0),
    ] {
        assert_eq!(
            inline_box_content_height(&fonts, size, dsf),
            want,
            "dsf{dsf} {size}px"
        );
    }
}

/// Inline-box content height of an `x` glyph in `family` at `size` px laid out
/// at device scale `scale`. Mirrors [`inline_box_content_height`] but names the
/// CSS family so it can walk `fixture_fonts()` by generic.
fn inline_box_content_height_in(fonts: &FontRegistry, family: &str, size: f32, scale: f32) -> f32 {
    let nodes = parse_html(&format!(
        "<div><span style='font-family:\"{family}\";font-size:{size}px;white-space:nowrap'>x</span></div>"
    ))
    .unwrap();
    let id = |id, path: &[usize]| SourceIdentity {
        id: SourceId(id),
        path: path.to_vec(),
    };
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &[id(1, &[0]), id(2, &[0, 0]), id(3, &[0, 0, 0])],
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
    let span = SourceId(2);
    for frag in snapshot.fragments() {
        if frag.source_id == Some(span) {
            if let Some(used) = frag.used_box {
                return used.content_height;
            }
        }
    }
    panic!("no used_box for the <span>");
}

/// miaskiewicz/turbo-dom#446: at DSF 2 the inline-box content height for the
/// `fixture_fonts()` faces at 11 / 13 / 15 / 17 / 19 px (the rounding
/// boundary sizes between the already-pinned 10 / 20 metrics) matches Chrome
/// 156 under `--force-device-scale-factor=2`. Fixture captured by
/// `scripts/webfont-inline-dpr2-chrome-control.mjs`. The fixture names each
/// case by `font` (the file-stem id below) and `size` px; the test maps each
/// font id to its `fixture_fonts()` CSS family (serif / sans-serif /
/// monospace generics all resolve to the bundled face).
#[test]
fn fixture_font_dpr2_metrics_at_the_11_19_px_boundary_match_chrome_156() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/web-font-metrics/dpr2_rounding_chrome156.json"
    ))
    .unwrap();
    assert!(
        control["chrome"]
            .as_str()
            .expect("chrome")
            .starts_with("156."),
        "Chrome version: {}",
        control["chrome"]
    );
    let dsf = control["environment"]["deviceScaleFactor"]
        .as_f64()
        .expect("dsf");
    assert_eq!(dsf, 2.0, "fixture is a DSF 2 capture");
    let cases = control["cases"].as_array().expect("cases");
    // 3 faces (Liberation Serif, Inter, Fira Code) x 5 sizes (11, 13, 15, 17, 19).
    assert_eq!(cases.len(), 15, "fixture covers every face x size pair");
    let fonts = fixture_fonts();
    let mut failures: Vec<String> = Vec::new();
    for case in cases {
        let font = case["font"].as_str().expect("font");
        let size = case["size"].as_f64().expect("size") as f32;
        let want = case["spanHeight"].as_f64().expect("spanHeight") as f32;
        let family = match font {
            "liberation-serif-regular" => "Liberation Serif",
            "inter-regular" => "Inter",
            "fira-code-regular" => "Fira Code",
            other => panic!("unexpected font id in fixture: {other}"),
        };
        let got = inline_box_content_height_in(&fonts, family, size, 2.0);
        // Exact match: both sides are produced on the same device-pixel grid
        // (Chrome under `--force-device-scale-factor=2` and the live-layout
        // path at `layout_scale: 2.0` both land on a half-CSS-px grain). R1
        // M2: fixture values carry `.0` / `.5` which are exact in f32, so
        // an exact comparison matches the docstring — `f32::EPSILON` would
        // mask a 1-ULP drift that the "exact" claim promises to catch.
        if got != want {
            failures.push(format!(
                "{font} {size}px: inline box content height {got}, Chrome {want}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

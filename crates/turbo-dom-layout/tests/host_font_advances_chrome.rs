//! Text run advances with the macOS system Times faces against Chrome 156
//! (miaskiewicz/turbo-dom#236). A test of the host-font policy itself, so it reads the
//! installed fonts and runs only on macOS (CLAUDE.md "Text-geometry tests name a repo font"):
//! the system Times faces carry AAT `morx`/`trak` tables that none of the fixture fonts has.
//! Oracle: `tests/fixtures/host-font-advances/chrome156.json`, captured on macOS by
//! `scripts/host-font-chrome-control.mjs` (Chrome 156.0.8078.47 headless, DSF 2, no injected
//! fonts). Every prefix Range width of every case is compared exactly.
#![cfg(target_os = "macos")]
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, LayoutSnapshot, NativeTextOptions, SnapshotInputs,
    SourceId, SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::parse_html;
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, NoImages};

fn fonts() -> &'static FontRegistry {
    static HOST: OnceLock<FontRegistry> = OnceLock::new();
    HOST.get_or_init(|| {
        let mut fonts = FontRegistry::new();
        fonts.load_system_fonts_for_html();
        fonts
    })
}

fn snapshot(style: &str, text: &str) -> LayoutSnapshot {
    let nodes = parse_html(&format!(
        "<div style='width:1200px;line-height:normal;{style}'><span>{text}</span></div>"
    ))
    .unwrap();
    let identity = |id, path: &[usize]| SourceIdentity {
        id: SourceId(id),
        path: path.to_vec(),
    };
    snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &[identity(1, &[0]), identity(2, &[0, 0]), identity(3, &[0, 0, 0])],
        &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
        &NativeTextOptions {
            original_text: &[],
            svg_viewports: &[],
            layout_scale: 2.0,
        },
        &SnapshotInputs {
            width: 800.0,
            height: 600.0,
            fonts: fonts(),
            images: &ImageCtx {
                resolver: &NoImages,
                body_height: None,
            },
        },
        &mut Diagnostics::default(),
    )
    .unwrap()
}

#[test]
fn times_range_widths_match_chrome_on_macos() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/host-font-advances/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/host-font-advances/probe.js"))
        )
    );
    let mut failures = Vec::new();
    let cases = control["result"].as_array().unwrap();
    for case in cases {
        let (style, text) = (case["style"].as_str().unwrap(), case["text"].as_str().unwrap());
        let snapshot = snapshot(style, text);
        for range in case["ranges"].as_array().unwrap() {
            let end = range["end"].as_u64().unwrap() as usize;
            let boxes = snapshot.text_range_boxes(SourceId(3), 0, end).unwrap();
            let left = boxes.iter().map(|b| b.rect.x).fold(f32::INFINITY, f32::min);
            let right = boxes
                .iter()
                .map(|b| b.rect.x + b.rect.width)
                .fold(f32::NEG_INFINITY, f32::max);
            let actual = f64::from(right - left);
            let expected = range["width"].as_f64().unwrap();
            if actual != expected {
                failures.push(format!("{style} {text:?}[..{end}]: {actual} != {expected}"));
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(cases.len(), 40);
}

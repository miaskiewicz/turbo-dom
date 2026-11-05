//! The `line-height: normal` line box of a `<p>` at device scale factors 1 and 2 against
//! Chrome 156 (miaskiewicz/turbo-dom#319, turbo-surf#158). Blink rounds the font's ascent
//! and descent in device pixels (and adds the macOS Times / Helvetica / Courier ascent hack
//! there), so a default 16px Times `<p>` is 18px tall at DSF 1 and 18.5px at DSF 2: the
//! live drive's `layout_scale` is that device scale factor, and a host that lays out at 2
//! while reporting `devicePixelRatio` 1 gets the DSF-2 box.
//! Oracle: `tests/fixtures/default_line_box_chrome156.json`, captured by
//! `scripts/default-line-box-chrome-control.mjs` (Chrome 156.0.8078.47 headless, real
//! `--force-device-scale-factor` 1 and 2). The fixture-font rows load the repo's font files
//! in Chrome through `@font-face` and run everywhere; the `host` rows (Chrome's default
//! font, Times) are a host-font policy check and run on macOS only.
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, NativeTextOptions, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::test_support::fixture_fonts;
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::parse_html;
use turbo_dom_layout::{build_cascade, Diagnostics, FontRegistry, ImageCtx, NoImages};

fn rows(source: &str) -> Vec<serde_json::Value> {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/default_line_box_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    control["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"] == source)
        .cloned()
        .collect()
}

/// The height of a `Hello world` `<p>` in `family` ("" = the default) at `size`, laid out
/// at `layout_scale`.
fn p_height(fonts: &FontRegistry, family: &str, size: f64, layout_scale: f32) -> f64 {
    let family = if family.is_empty() {
        String::new()
    } else {
        format!("font-family:'{family}';")
    };
    let nodes = parse_html(&format!(
        "<p style=\"{family}font-size:{size}px\">Hello world</p>"
    ))
    .unwrap();
    let snapshot = snapshot_html_live_tree_with_native_text(
        &literal::tree(&nodes).input(),
        &[SourceIdentity {
            id: SourceId(1),
            path: vec![0],
        }],
        &build_cascade("", "", turbo_dom_layout::style::TokenSet::default()),
        &NativeTextOptions {
            original_text: &[],
            svg_viewports: &[],
            layout_scale,
        },
        &SnapshotInputs {
            width: 784.0,
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
    let source = snapshot.source(SourceId(1)).unwrap();
    f64::from(snapshot.fragments()[source.fragment_indices[0]].rect.height)
}

fn compare(fonts: &FontRegistry, rows: &[serde_json::Value]) {
    let mut failures = Vec::new();
    for row in rows {
        let family = row["family"].as_str().unwrap();
        let size = row["size"].as_f64().unwrap();
        let dsf = row["dsf"].as_f64().unwrap();
        let want = row["height"].as_f64().unwrap();
        let got = p_height(fonts, family, size, dsf as f32);
        if got != want {
            failures.push(format!(
                "{family:?} {size}px DSF {dsf}: {got}, Chrome {want}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn fixture_font_line_boxes_follow_the_device_scale_like_chrome() {
    let rows = rows("fixture");
    assert_eq!(rows.len(), 18);
    compare(&fixture_fonts(), &rows);
}

#[cfg(target_os = "macos")]
#[test]
fn default_times_line_box_is_18px_at_dsf_1_and_18_5px_at_dsf_2() {
    let rows = rows("host");
    assert_eq!(rows.len(), 12);
    let default_16 = |dsf: f64| {
        rows.iter()
            .find(|r| r["family"] == "" && r["size"] == 16.0 && r["dsf"] == dsf)
            .unwrap()["height"]
            .as_f64()
            .unwrap()
    };
    assert_eq!((default_16(1.0), default_16(2.0)), (18.0, 18.5));
    let mut fonts = FontRegistry::new();
    fonts.load_system_fonts_for_html();
    compare(&fonts, &rows);
}

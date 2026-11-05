//! `@font-face` faces rank above installed faces of the same family (miaskiewicz/turbo-dom#304).
//! Oracle: `tests/fixtures/web-font-priority/chrome156.json` from
//! `scripts/web-font-priority-chrome-control.mjs`: Go-Regular.ttf declared as `@font-face`
//! "Helvetica" (installed on the control host with Regular and Bold faces) measures exactly
//! as the same face under a private name, at weight 400 and at 700 (synthetic bold keeps the
//! advances), while the installed Helvetica measures differently. Here Liberation Serif
//! Regular/Bold stand in for the installed faces (`FontFace::from_bytes`, fixture fonts only),
//! registered before and after the web face.

use sha2::{Digest, Sha256};
use turbo_dom_layout::layout_snapshot::{
    snapshot_html_live_tree_with_native_text, NativeTextOptions, SnapshotInputs, SourceId,
    SourceIdentity,
};
use turbo_dom_layout::test_support::literal;
use turbo_dom_layout::test_support::literal::parse_html;
use turbo_dom_layout::{build_cascade, Diagnostics, FontFace, FontRegistry, ImageCtx, NoImages};

fn asset(path: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/assets/fonts/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn range_width(fonts: &FontRegistry, family: &str, weight: u64, size: f64, text: &str) -> f64 {
    let nodes = parse_html(&format!(
        "<div style='font-family:\"{family}\";font-size:{size}px;font-weight:{weight};width:2000px'>{text}</div>"
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
            layout_scale: 2.0,
        },
        &SnapshotInputs {
            width: 2400.0,
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
    let boxes = snapshot
        .text_range_boxes(SourceId(2), 0, text.len())
        .unwrap();
    boxes.iter().map(|b| f64::from(b.rect.width)).sum()
}

fn installed(family: &str) -> [FontFace; 2] {
    let face = |file, weight| {
        FontFace::from_bytes(
            asset(&format!("liberation-serif/{file}")),
            family,
            weight,
            false,
        )
        .unwrap()
    };
    [
        face("LiberationSerif-Regular.ttf", 400),
        face("LiberationSerif-Bold.ttf", 700),
    ]
}

/// A registry with the stand-in installed "Helvetica" faces and the web face, in either order.
fn registry(web_first: bool) -> FontRegistry {
    let go = asset("Go-Regular.ttf");
    let web = |family| FontFace::from_web_font_bytes(go.clone(), family, 400, false).unwrap();
    let mut fonts = FontRegistry::new();
    if web_first {
        fonts.add(web("Helvetica"));
    }
    for face in installed("Helvetica") {
        fonts.add(face);
    }
    if !web_first {
        fonts.add(web("Helvetica"));
    }
    fonts.add(web("WebGo"));
    fonts
}

#[test]
fn web_faces_shadow_installed_faces_of_the_same_family_like_chrome() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/web-font-priority/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        format!("{:x}", Sha256::digest(asset("Go-Regular.ttf"))),
        control["fontSha256"].as_str().unwrap()
    );
    let size = control["size"].as_f64().unwrap();
    let rows = control["web"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    // The control's installed Helvetica differs from the web face, at both weights.
    for (web, installed) in rows.iter().zip(control["installed"].as_array().unwrap()) {
        assert_eq!(
            (&web["weight"], &web["text"]),
            (&installed["weight"], &installed["text"])
        );
        assert_ne!(web["width"], installed["width"], "{web}");
    }
    let mut failures = Vec::new();
    for web_first in [false, true] {
        let fonts = registry(web_first);
        for row in rows {
            let (family, weight, text) = (
                row["family"].as_str().unwrap(),
                row["weight"].as_u64().unwrap(),
                row["text"].as_str().unwrap(),
            );
            let face = fonts.select(&[family], weight as u16, false).unwrap();
            if !face.is_web_font() {
                failures.push(format!("web_first={web_first} {family} {weight}: {face:?}"));
            }
            let got = range_width(&fonts, family, weight, size, text);
            let want = row["width"].as_f64().unwrap();
            if got != want {
                failures.push(format!(
                    "web_first={web_first} {family} {weight} {text:?}: {got} vs {want}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn installed_faces_serve_families_no_web_face_declares() {
    // The stand-in installed faces still serve their own family when no web face shadows it,
    // and a glyph the web family lacks falls through to the next family in the list, never to
    // an installed face of the shadowed name.
    let mut fonts = registry(false);
    for face in installed("Times") {
        fonts.add(face);
    }
    let times = fonts.select(&["Times"], 700, false).unwrap();
    assert!(!times.is_web_font());
    assert_eq!(times.weight(), 700);
    let hebrew = '\u{05d0}';
    let go = fonts.select(&["Helvetica"], 400, false).unwrap();
    assert!(!go.has_glyph(hebrew));
    let mut fonts = fonts;
    let hebrew_face = FontFace::from_bytes(
        asset("noto-sans-hebrew/NotoSansHebrew-Regular.ttf"),
        "Helvetica",
        400,
        false,
    )
    .unwrap();
    fonts.add(hebrew_face);
    let next = FontFace::from_bytes(
        asset("noto-sans-hebrew/NotoSansHebrew-Regular.ttf"),
        "Next",
        400,
        false,
    )
    .unwrap();
    fonts.add(next);
    let resolved = fonts
        .resolve_glyph(&["Helvetica", "Next"], 400, false, hebrew)
        .unwrap();
    assert_eq!(resolved.family(), "Next");
    assert!(fonts
        .resolve_glyph(&["Helvetica"], 400, false, 'a')
        .unwrap()
        .is_web_font());
}

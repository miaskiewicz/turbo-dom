//! Legacy `kern` tables in web fonts against Chrome 156 (miaskiewicz/turbo-dom#240).
//! Oracle: `tests/fixtures/web-font-kern/chrome156.json` from
//! `scripts/web-font-kern-chrome-control.mjs`: four @font-face variants crossing the outline
//! format (CFF / TrueType) with the `kern` table version (Apple 1.0 / OpenType 0), none with
//! GPOS. Chrome kerns only the TrueType font with a version-0 table; a CFF font's `kern` table
//! and any version-1.0 table are ignored. `FontFace::from_web_font_bytes` applies that rule;
//! canvas widths and the HTML Range width of each text match exactly.
//! The variants are rebuilt here byte for byte (checked against the control's hashes).

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
fn be16(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_be_bytes([b[at], b[at + 1]]))
}
fn be32(b: &[u8], at: usize) -> usize {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}
/// `(directory entry offset, table offset, table length)` of `tag`.
fn table(b: &[u8], tag: [u8; 4]) -> Option<(usize, usize, usize)> {
    (0..be16(b, 4))
        .map(|i| 12 + 16 * i)
        .find_map(|at| (b[at..at + 4] == tag).then(|| (at, be32(b, at + 8), be32(b, at + 12))))
}
fn hide(mut b: Vec<u8>, tag: [u8; 4]) -> Vec<u8> {
    let (at, ..) = table(&b, tag).unwrap();
    b[at..at + 4].copy_from_slice(b"zzzz");
    b
}
fn pad(mut b: Vec<u8>) -> Vec<u8> {
    b.resize(b.len().div_ceil(4) * 4, 0);
    b
}
/// The control's `rewriteKern`: the same format-0 pair data under the other header version,
/// appended as a new table the `kern` directory entry points at.
fn rewrite_kern(b: &[u8], to_version: u16) -> Vec<u8> {
    let (at, offset, length) = table(b, *b"kern").unwrap();
    let kern = &b[offset..offset + length];
    let mut subs = Vec::new();
    if be16(kern, 0) == 0 {
        let mut p = 4;
        for _ in 0..be16(kern, 2) {
            let (len, coverage) = (be16(kern, p + 2), be16(kern, p + 4));
            subs.push((coverage >> 8, coverage & 1 == 1, &kern[p + 6..p + len]));
            p += len;
        }
    } else {
        let mut p = 8;
        for _ in 0..be32(kern, 4) {
            let (len, coverage) = (be32(kern, p), be16(kern, p + 4));
            subs.push((
                coverage & 0xff,
                coverage & 0x8000 == 0,
                &kern[p + 8..p + len],
            ));
            p += len;
        }
    }
    let mut out = Vec::new();
    let n = subs.len() as u32;
    if to_version == 0 {
        out.extend([0, 0]);
        out.extend((n as u16).to_be_bytes());
        for (format, horizontal, data) in subs {
            out.extend([0, 0]);
            out.extend(((data.len() + 6) as u16).to_be_bytes());
            out.extend(((format << 8) as u16 | u16::from(horizontal)).to_be_bytes());
            out.extend(data);
        }
    } else {
        out.extend(0x0001_0000_u32.to_be_bytes());
        out.extend(n.to_be_bytes());
        for (format, horizontal, data) in subs {
            out.extend(((data.len() + 8) as u32).to_be_bytes());
            out.extend((if horizontal { 0 } else { 0x8000 } | format as u16).to_be_bytes());
            out.extend([0, 0]);
            out.extend(data);
        }
    }
    let mut base = pad(b.to_vec());
    let start = base.len() as u32;
    base[at + 8..at + 12].copy_from_slice(&start.to_be_bytes());
    base[at + 12..at + 16].copy_from_slice(&(out.len() as u32).to_be_bytes());
    base.extend(pad(out));
    base
}
fn variant(name: &str) -> Vec<u8> {
    let warsaw = asset("WarsawGothic-BnBV.otf");
    let liberation = hide(
        asset("liberation-serif/LiberationSerif-Regular.ttf"),
        *b"GPOS",
    );
    match name {
        "cff-v1" => warsaw,
        "cff-v0" => rewrite_kern(&warsaw, 0),
        "tt-v0" => liberation,
        "tt-v1" => rewrite_kern(&liberation, 1),
        _ => unreachable!("{name}"),
    }
}

fn range_width(fonts: &FontRegistry, family: &str, size: f64, text: &str) -> f64 {
    let nodes = parse_html(&format!(
        "<div style='font-family:\"{family}\";font-size:{size}px;width:2000px'>{text}</div>"
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

#[test]
fn web_fonts_apply_only_truetype_version_0_kern_tables_like_chrome() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/web-font-kern/chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    let size = control["size"].as_f64().unwrap();
    let mut fonts = FontRegistry::new();
    for (name, hash) in control["variantSha256"].as_object().unwrap() {
        let bytes = variant(name);
        assert_eq!(&format!("{:x}", Sha256::digest(&bytes)), hash, "{name}");
        fonts.add(FontFace::from_web_font_bytes(bytes, name, 400, false).unwrap());
    }
    let rows = control["result"].as_array().unwrap();
    assert_eq!(rows.len(), 12);
    let mut failures = Vec::new();
    for row in rows {
        let (name, text) = (
            row["variant"].as_str().unwrap(),
            row["text"].as_str().unwrap(),
        );
        let face = fonts.select(&[name], 400, false).unwrap();
        let canvas = face.measure_canvas(text, size as f32);
        let want = row["canvas_normal"].as_f64().unwrap();
        // Blink's double summation order differs by ~1e-14; one 16.16 unit is 1.5e-5.
        if (canvas - want).abs() > 1e-9 {
            failures.push(format!("{name} {text:?} canvas: {canvas} vs {want}"));
        }
        let html = range_width(&fonts, name, size, text);
        let want = row["html_normal"].as_f64().unwrap();
        if html != want {
            failures.push(format!("{name} {text:?} html: {html} vs {want}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

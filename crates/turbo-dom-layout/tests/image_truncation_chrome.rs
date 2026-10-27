//! Where Blink first reports an image's size, against its Chrome 156 control
//! (miaskiewicz/turbo-dom#59).
//!
//! The control is `tests/fixtures/image_truncation_chrome156.json` (turbo-dom's
//! `scripts/image-truncation-chrome-control.mjs`): every byte prefix of each image in
//! `tests/fixtures/images/truncation` (`scripts/make_truncation_fixtures.py`) loaded as a
//! `data:` `<img>` — `loads` has one `L` (load, full natural size) or `E` (error) per prefix
//! length; the AVIF images of `tests/fixtures/images/formats` too (`formats/…` names) — plus `image/svg+xml` documents with and without an SVG-namespace root. Blink's
//! decoders know the size of a JPEG at the end of its first `SOS` header, of a GIF once the
//! first image descriptor's position and size are read, of a WebP only with the whole RIFF
//! payload (the demuxer validates it once all data arrived) and of a PNG at its first `IDAT`
//! header, of an AVIF once the primary item's `iloc` extents lie within the data (libavif
//! rejects an extent past the end with all data received; the alpha item's need not:
//! miaskiewicz/turbo-dom#107); before that the image is broken. An SVG image's root must be an `<svg>` in the SVG
//! namespace (XML, so case-sensitive), never gzip-compressed.
//!
//! Here: [`probe_natural`] (the live `<img>` load decision, turbo-surf) and [`probe`] (layout,
//! the PDF and the raster painter) answer Chrome's load / natural size at every prefix, and the
//! HTML drive lays a loadable prefix out at its natural size and a broken one like an image
//! that does not resolve.
use std::fmt::Write as _;

use serde_json::Value;
use turbo_dom_layout::image::{probe, probe_natural};
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/image_truncation_chrome156.json"
    );
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    fixture
}

/// A control image: `formats/…` names are relative to `tests/fixtures/images`, the others
/// to its `truncation` directory.
fn read(file: &str) -> Vec<u8> {
    let dir = if file.contains('/') {
        ""
    } else {
        "truncation/"
    };
    std::fs::read(format!(
        "{}/tests/fixtures/images/{dir}{file}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn is_avif(file: &str) -> bool {
    file.ends_with(".avif")
}

/// The control's images `pick` selects.
fn images(fixture: &Value, pick: fn(&str) -> bool) -> Vec<Value> {
    let all = fixture["images"].as_array().unwrap();
    let picked: Vec<Value> = all
        .iter()
        .filter(|i| pick(i["file"].as_str().unwrap()))
        .cloned()
        .collect();
    assert!(!picked.is_empty());
    picked
}

fn size(i: Option<turbo_dom_layout::image::Intrinsic>) -> Option<(u32, u32)> {
    i.map(|i| (i.width, i.height))
}

#[test]
fn the_probes_report_a_size_at_chromes_prefix_lengths() {
    probes_follow_chrome(|f| !is_avif(f));
}

#[test]
#[ignore = "turbo-dom#107: fails until the fix"]
fn the_avif_probes_report_a_size_once_the_primary_item_is_in_the_data() {
    probes_follow_chrome(is_avif);
}

fn probes_follow_chrome(pick: fn(&str) -> bool) {
    let fixture = fixture();
    let mut failures = Vec::new();
    for image in &images(&fixture, pick) {
        let file = image["file"].as_str().unwrap();
        let bytes = read(file);
        assert_eq!(
            bytes.len() as u64,
            image["length"].as_u64().unwrap(),
            "{file}"
        );
        let natural = image["natural"].as_array().unwrap();
        let natural = (
            natural[0].as_u64().unwrap() as u32,
            natural[1].as_u64().unwrap() as u32,
        );
        let loads = image["loads"].as_str().unwrap().as_bytes();
        assert_eq!(loads.len(), bytes.len() + 1, "{file}");
        for (n, &chrome) in loads.iter().enumerate() {
            assert!(
                chrome == b'L' || chrome == b'E',
                "{file}@{n}: {}",
                chrome as char
            );
            let want = (chrome == b'L').then_some(natural);
            let head = &bytes[..n];
            for (name, got) in [
                ("probe_natural", size(probe_natural(head))),
                ("probe", size(probe(head))),
            ] {
                if got != want {
                    failures.push(format!("{name} {file}@{n}: {got:?}, Chrome {want:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[cfg(feature = "svg-rasterize")]
#[test]
fn svg_images_need_an_svg_namespace_root() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for case in fixture["svgs"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let bytes = if id == "svgz" {
            // The control's gzip of the first document (any gzip stream: Blink never inflates).
            let doc = fixture["svgs"][0]["document"].as_str().unwrap();
            gzip_stored(doc.as_bytes())
        } else {
            case["document"].as_str().unwrap().as_bytes().to_vec()
        };
        let want = (case["event"] == "load").then(|| {
            let n = case["natural"].as_array().unwrap();
            (n[0].as_u64().unwrap() as u32, n[1].as_u64().unwrap() as u32)
        });
        let natural = size(probe_natural(&bytes));
        // The raster painter's 1x rasterizer decides the same.
        let raster =
            turbo_dom_layout::svg::rasterize(&bytes, 1.0, turbo_dom_layout::svg::Oversize::Reject)
                .map(|i| (i.width, i.height));
        if natural != want || raster != want {
            failures.push(format!(
                "{id}: probe_natural {natural:?}, rasterize {raster:?}, Chrome {want:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A gzip member of `data` with one stored (uncompressed) deflate block.
#[cfg(feature = "svg-rasterize")]
fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    let len = data.len() as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    let mut crc = 0xffff_ffff_u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    out.extend_from_slice(&(!crc).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                A[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

#[test]
fn truncated_img_boxes_follow_chromes_load_decision() {
    img_boxes_follow_chrome(|f| !is_avif(f));
}

#[test]
#[ignore = "turbo-dom#107: fails until the fix"]
fn truncated_avif_img_boxes_follow_chromes_load_decision() {
    img_boxes_follow_chrome(is_avif);
}

fn img_boxes_follow_chrome(pick: fn(&str) -> bool) {
    // Per image: the last broken prefix and the first loadable one, as absolutely positioned
    // `<img>`s, next to an `<img>` whose source no decoder reads (the not-loaded box).
    let fixture = fixture();
    let mime = |file: &str| match file.rsplit('.').next().unwrap() {
        "jpg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        _ => "image/png",
    };
    let mut body = String::from(
        r#"<img data-cid="none" src="data:image/png;base64,AAAA" style="position:absolute;left:0;top:0">"#,
    );
    let mut expected = Vec::new();
    for (i, image) in images(&fixture, pick).iter().enumerate() {
        let file = image["file"].as_str().unwrap();
        let bytes = read(file);
        let loads = image["loads"].as_str().unwrap();
        let first = loads.find('L').unwrap();
        let natural = image["natural"].as_array().unwrap();
        let natural = (
            natural[0].as_f64().unwrap() as f32,
            natural[1].as_f64().unwrap() as f32,
        );
        for (kind, n) in [("broken", first - 1), ("loads", first)] {
            let id = format!("{file}-{kind}");
            let top = 20 + i * 40;
            let left = if kind == "broken" { 100 } else { 300 };
            write!(
                body,
                r#"<img data-cid="{id}" src="data:{};base64,{}" style="position:absolute;left:{left}px;top:{top}px">"#,
                mime(file),
                base64(&bytes[..n])
            )
            .unwrap();
            expected.push((id, kind == "loads", natural));
        }
    }
    let html = format!("<!DOCTYPE html><html><head><style>body{{margin:0}}</style></head><body>{body}</body></html>");
    let mut diags = Diagnostics::default();
    let boxes = layout_boxes(&html, "", 800.0, 600.0, &FontRegistry::new(), &mut diags).unwrap();
    let size_of = |id: &str| {
        boxes
            .iter()
            .find(|b| b.cid == id)
            .map(|b| (b.width, b.height))
    };
    let unresolved = size_of("none");
    let mut failures = Vec::new();
    for (id, loads, natural) in expected {
        let got = size_of(&id);
        let want = if loads { Some(natural) } else { unresolved };
        if got != want {
            failures.push(format!("{id}: {got:?}, want {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

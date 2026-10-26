//! The image formats Chrome 156 decodes, against its control (miaskiewicz/turbo-surf#30).
//!
//! The control is `tests/fixtures/image_formats_chrome156.json` (turbo-dom's
//! `scripts/image-formats-chrome-control.mjs` over `tests/fixtures/images/formats`, the
//! 64x40 four-colour image of `scripts/make_format_fixtures.py` as AVIF, BMP, ICO, CUR,
//! SVG and TIFF, every one an `<img src="data:…">`): which sources load, their
//! `naturalWidth`/`naturalHeight` and their boxes. Here: the shared `data:` URL decoder
//! reads every source, [`probe_natural`] answers Chrome's load / natural size (SVG under
//! `svg-rasterize`), and the HTML drive lays every raster-format `<img>` out at Chrome's
//! rectangle.
use serde_json::Value;
use turbo_dom_layout::image::{decode_data_url, probe, probe_natural, sniff, Format};
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/image_formats_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

fn is_svg(id: &str) -> bool {
    id.starts_with("svg")
}

#[test]
fn every_data_url_form_decodes_to_the_fixture_bytes() {
    let svg = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/images/formats/q.svg"
    ))
    .unwrap();
    let svg = String::from_utf8(svg).unwrap();
    for case in fixture()["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let url = decode_data_url(case["src"].as_str().unwrap()).expect(id);
        if matches!(id, "svg-base64" | "svg-percent" | "svg-utf8") {
            assert!(url.mime.starts_with("image/svg+xml"), "{id}: {}", url.mime);
            assert_eq!(
                String::from_utf8(url.bytes).unwrap().trim(),
                svg.trim(),
                "{id}"
            );
        }
    }
    // The Fetch standard's defaults and edge cases.
    let plain = decode_data_url("data:,a%2Cb").unwrap();
    assert_eq!(
        (plain.mime.as_str(), plain.bytes.as_slice()),
        ("text/plain;charset=US-ASCII", &b"a,b"[..])
    );
    assert_eq!(decode_data_url("DATA:;base64,QQ").unwrap().bytes, b"A");
    assert_eq!(
        decode_data_url("data:x/y;base64, Q Q = =").unwrap().bytes,
        b"A"
    );
    assert_eq!(
        decode_data_url("data:x;base64,QQ%3D%3D").unwrap().bytes,
        b"A"
    );
    assert!(decode_data_url("data:x;base64,Q").is_none(), "1 mod 4");
    assert!(
        decode_data_url("data:x;base64,Q=Q=").is_none(),
        "padding inside"
    );
    assert_eq!(decode_data_url("data:x,%zz%4").unwrap().bytes, b"%zz%4");
    assert!(decode_data_url("http://x/a.png").is_none());
    assert!(decode_data_url("data:no-comma").is_none());
}

#[test]
fn probe_natural_answers_chromes_load_and_natural_size() {
    let mut checked = 0;
    for case in fixture()["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if is_svg(id) && !cfg!(feature = "svg-rasterize") {
            continue;
        }
        let bytes = decode_data_url(case["src"].as_str().unwrap())
            .unwrap()
            .bytes;
        let natural = probe_natural(&bytes).map(|i| (u64::from(i.width), u64::from(i.height)));
        let want = (case["event"] == "load").then(|| {
            let n = case["natural"].as_array().unwrap();
            (n[0].as_u64().unwrap(), n[1].as_u64().unwrap())
        });
        assert_eq!(natural, want, "{id}");
        checked += 1;
    }
    assert!(checked >= 10);
}

#[test]
fn the_new_formats_sniff_and_probe() {
    let read = |f: &str| {
        std::fs::read(format!(
            "{}/tests/fixtures/images/formats/{f}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    };
    for (file, format, alpha) in [
        ("q.avif", Format::Avif, false),
        ("q-alpha.avif", Format::Avif, true),
        ("q.bmp", Format::Bmp, false),
        ("q-alpha.bmp", Format::Bmp, true),
        ("q-8bit.bmp", Format::Bmp, false),
        ("q.ico", Format::Ico, true),
        ("q-png.ico", Format::Ico, true),
        ("q.cur", Format::Ico, true),
    ] {
        let bytes = read(file);
        assert_eq!(sniff(&bytes), Some(format), "{file}");
        let i = probe(&bytes).expect(file);
        assert_eq!((i.width, i.height, i.has_alpha), (64, 40, alpha), "{file}");
        // Not embeddable in a PDF: decode emits nothing rather than a wrong image.
        assert!(turbo_dom_layout::image::decode(&bytes).is_none(), "{file}");
    }
    let sizes = probe(&read("q-sizes.ico")).unwrap();
    assert_eq!((sizes.width, sizes.height), (48, 48), "the largest entry");
    let avif = turbo_dom_layout::image::avif_payload(&read("q-alpha.avif")).expect("demux");
    assert!(!avif.color.is_empty() && avif.alpha.as_ref().is_some_and(|a| !a.is_empty()));
    assert!(
        sniff(&read("q.tiff")).is_none(),
        "TIFF stays unsupported, as in Chrome"
    );
    // Truncated / malformed inputs never panic.
    for file in ["q.avif", "q.bmp", "q.ico", "q-sizes.ico"] {
        let bytes = read(file);
        for cut in [3, 8, 13, 20, 30, 60, 100] {
            let head = &bytes[..cut.min(bytes.len())];
            let _ = (
                probe(head),
                probe_natural(head),
                turbo_dom_layout::image::avif_payload(head),
            );
        }
    }
}

#[test]
fn raster_format_img_boxes_match_chrome_156() {
    let fixture = fixture();
    let html = fixture["html"].as_str().unwrap();
    let mut diags = Diagnostics::default();
    let boxes = layout_boxes(html, "", 1280.0, 800.0, &FontRegistry::new(), &mut diags).unwrap();
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        // SVG boxes are the PDF pipeline's (2x, `svg` feature) / the raster's business;
        // TIFF is Chrome's broken-image icon.
        if is_svg(id) || case["event"] != "load" {
            continue;
        }
        let want: Vec<f32> = case["rect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let got = boxes
            .iter()
            .find(|b| b.cid == id)
            .map(|b| [b.x, b.y, b.width, b.height]);
        if got.is_none_or(|g| g.iter().zip(&want).any(|(g, w)| (g - w).abs() > 0.01)) {
            failures.push(format!("{id}: {got:?}, Chrome {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

//! EXIF image orientation against Chrome 156 (miaskiewicz/turbo-html2pdf#30).
//!
//! The control is `tests/fixtures/image_orientation_chrome156.json` (turbo-dom's
//! `scripts/image-orientation-chrome-control.mjs` over the
//! `tests/fixtures/images/orientation` images of `scripts/make_orientation_fixtures.py`):
//! Chrome applies the EXIF Orientation of a JPEG `APP1` segment and of a PNG `eXIf`
//! chunk (not of a WebP `EXIF` chunk) unless `image-orientation: none`, swapping the
//! box for orientations 5-8. Here: the shared probe reads the same orientation, and the
//! HTML drive lays every `<img>` of the control page out at Chrome's rectangle.
use serde_json::Value;
use turbo_dom_layout::image::{probe, Orientation};
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/image_orientation_chrome156.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).unwrap()
}

fn image(file: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/images/orientation/{file}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("orientation image")
}

#[test]
fn the_probe_reads_the_orientation_chrome_applies() {
    for case in fixture()["cases"].as_array().unwrap() {
        if case["mode"] != "img" {
            continue;
        }
        let file = case["file"].as_str().unwrap();
        let intrinsic = probe(&image(file)).expect("probes");
        let natural = case["natural"].as_array().unwrap();
        let (w, h) = intrinsic
            .orientation
            .displayed_size(intrinsic.width, intrinsic.height);
        assert_eq!(
            (u64::from(w), u64::from(h)),
            (natural[0].as_u64().unwrap(), natural[1].as_u64().unwrap()),
            "{file}: Chrome's naturalWidth/naturalHeight"
        );
        // Chrome applies EXIF orientation from JPEG and PNG only.
        let applied = if file.starts_with("webp-") {
            1
        } else {
            u8::try_from(case["orientation"].as_u64().unwrap()).unwrap()
        };
        assert_eq!(intrinsic.orientation.exif(), applied, "{file}");
    }
}

#[test]
fn img_boxes_match_chrome_156() {
    let fixture = fixture();
    let html = fixture["html"].as_str().unwrap();
    let mut diags = Diagnostics::default();
    let boxes = layout_boxes(html, "", 1280.0, 800.0, &FontRegistry::new(), &mut diags).unwrap();
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let Some(b) = boxes.iter().find(|b| b.cid == id) else {
            failures.push(format!("{id}: no box"));
            continue;
        };
        let want: Vec<f32> = case["rect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        let got = [b.x, b.y, b.width, b.height];
        if got.iter().zip(&want).any(|(g, w)| (g - w).abs() > 0.01) {
            failures.push(format!("{id}: {got:?}, Chrome {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn orientation_maps_and_relays_pixels() {
    // A 3x2 stored image, pixels numbered by their stored index.
    let rgba: Vec<u8> = (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect();
    let shown = |o: u16| {
        let (w, h, px) = Orientation::from_exif(o).apply_rgba(3, 2, &rgba);
        (w, h, px.chunks(4).map(|p| p[0]).collect::<Vec<_>>())
    };
    assert_eq!(shown(1), (3, 2, vec![0, 1, 2, 3, 4, 5]));
    assert_eq!(shown(2), (3, 2, vec![2, 1, 0, 5, 4, 3]));
    assert_eq!(shown(3), (3, 2, vec![5, 4, 3, 2, 1, 0]));
    assert_eq!(shown(4), (3, 2, vec![3, 4, 5, 0, 1, 2]));
    assert_eq!(shown(5), (2, 3, vec![0, 3, 1, 4, 2, 5]));
    assert_eq!(shown(6), (2, 3, vec![3, 0, 4, 1, 5, 2]));
    assert_eq!(shown(7), (2, 3, vec![5, 2, 4, 1, 3, 0]));
    assert_eq!(shown(8), (2, 3, vec![2, 5, 1, 4, 0, 3]));
    for bad in [0, 9, 300] {
        assert!(Orientation::from_exif(bad).is_identity());
    }
    assert_eq!(Orientation::default(), Orientation::IDENTITY);
}

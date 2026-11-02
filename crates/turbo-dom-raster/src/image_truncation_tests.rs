//! The raster painter's image decode follows Blink's load decision for truncated bodies and
//! SVG roots (miaskiewicz/turbo-dom#59), against turbo-dom-layout's Chrome 156 control
//! `tests/fixtures/image_truncation_chrome156.json` (`scripts/image-truncation-chrome-control.mjs`):
//! a prefix Chrome loads is laid out at its natural size (painted from whatever the decoder
//! reads, else painting nothing), a prefix Chrome rejects is not an image. The control's AVIF
//! images (`formats/…`, relative to its `images` directory) load once the primary item's
//! `iloc` extents lie within the data (miaskiewicz/turbo-dom#107).
use super::decode;

fn layout_fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../turbo-dom-layout/tests/fixtures")
        .join(path)
}

#[test]
fn truncated_images_decode_like_chrome_156() {
    decode_like_chrome(|file| !is_avif(file), true);
}

#[test]
fn truncated_avifs_decode_like_chrome_156() {
    decode_like_chrome(is_avif, false);
}

fn is_avif(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e == "avif")
}

/// The control's images `pick` selects (and, with `svgs`, its SVG documents) decode as
/// Chrome loads them.
fn decode_like_chrome(pick: fn(&str) -> bool, svgs: bool) {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(layout_fixture("image_truncation_chrome156.json")).unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for image in fixture["images"].as_array().unwrap() {
        let file = image["file"].as_str().unwrap();
        if !pick(file) {
            continue;
        }
        checked += 1;
        let dir = if file.contains('/') {
            ""
        } else {
            "truncation/"
        };
        let bytes = std::fs::read(layout_fixture(&format!("images/{dir}{file}"))).unwrap();
        let natural = &image["natural"];
        let natural = (
            natural[0].as_u64().unwrap() as u32,
            natural[1].as_u64().unwrap() as u32,
        );
        for (n, chrome) in image["loads"].as_str().unwrap().bytes().enumerate() {
            let want = (chrome == b'L').then_some(natural);
            let got = decode(&bytes[..n]).map(|d| (d.intrinsic.width, d.intrinsic.height));
            failures.extend(mismatch(format_args!("{file}@{n}"), got, want));
        }
    }
    assert!(checked > 0);
    for case in fixture["svgs"].as_array().unwrap().iter().filter(|_| svgs) {
        let id = case["id"].as_str().unwrap();
        if id == "svgz" {
            continue; // covered by turbo-dom-layout's rasterizer test
        }
        let want = (case["event"] == "load").then(|| {
            let n = &case["natural"];
            (n[0].as_u64().unwrap() as u32, n[1].as_u64().unwrap() as u32)
        });
        let got = decode(case["document"].as_str().unwrap().as_bytes())
            .map(|d| (d.intrinsic.width, d.intrinsic.height));
        failures.extend(mismatch(id, got, want));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The failure line for `label` when the decoded size differs from Chrome's (formatted
/// eagerly, so the passing run executes it too).
fn mismatch(
    label: impl std::fmt::Display,
    got: Option<(u32, u32)>,
    want: Option<(u32, u32)>,
) -> Option<String> {
    let line = format!("{label}: {got:?}, Chrome {want:?}");
    (got != want).then_some(line)
}

#[test]
fn a_size_differing_from_chromes_is_a_failure_line() {
    assert_eq!(mismatch("a.png@3", Some((4, 5)), Some((4, 5))), None);
    assert_eq!(
        mismatch("a.png@3", Some((4, 5)), None).as_deref(),
        Some("a.png@3: Some((4, 5)), Chrome None")
    );
    assert_eq!(
        mismatch("svg", None, Some((1, 2))).as_deref(),
        Some("svg: None, Chrome Some((1, 2))")
    );
}

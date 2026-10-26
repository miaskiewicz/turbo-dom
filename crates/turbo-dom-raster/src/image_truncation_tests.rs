//! The raster painter's image decode follows Blink's load decision for truncated bodies and
//! SVG roots (miaskiewicz/turbo-dom#59), against turbo-dom-layout's Chrome 156 control
//! `tests/fixtures/image_truncation_chrome156.json` (`scripts/image-truncation-chrome-control.mjs`):
//! a prefix Chrome loads is laid out at its natural size (painted from whatever the decoder
//! reads, else as the placeholder), a prefix Chrome rejects is not an image.
use super::decode;

fn layout_fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../turbo-dom-layout/tests/fixtures")
        .join(path)
}

#[test]
#[ignore = "turbo-dom#59: fails until the fix"]
fn truncated_images_decode_like_chrome_156() {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(layout_fixture("image_truncation_chrome156.json")).unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    for image in fixture["images"].as_array().unwrap() {
        let file = image["file"].as_str().unwrap();
        let bytes = std::fs::read(layout_fixture(&format!("images/truncation/{file}"))).unwrap();
        let natural = &image["natural"];
        let natural = (
            natural[0].as_u64().unwrap() as u32,
            natural[1].as_u64().unwrap() as u32,
        );
        for (n, chrome) in image["loads"].as_str().unwrap().bytes().enumerate() {
            let want = (chrome == b'L').then_some(natural);
            let got = decode(&bytes[..n]).map(|d| (d.intrinsic.width, d.intrinsic.height));
            if got != want {
                failures.push(format!("{file}@{n}: {got:?}, Chrome {want:?}"));
            }
        }
    }
    for case in fixture["svgs"].as_array().unwrap() {
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
        if got != want {
            failures.push(format!("{id}: {got:?}, Chrome {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

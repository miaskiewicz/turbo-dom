//! Grid item box sizing on the HTML path (0.7.0 review R3 #2): the `grid-item-box-*` cases of
//! the live-geometry Chrome 156 control (`tests/fixtures/live_geometry_chrome156.json`,
//! `scripts/live-geometry-chrome-control.mjs`) laid out through [`layout_boxes`] (the
//! `data-cid` geometry seam of `layout_html`) at Chrome's 1280x800 viewport. A grid item's
//! `width` / `height` / `min-*` / `max-*` are content-box lengths under `box-sizing:
//! content-box`: its padding and border add to them, start-aligned or stretched (and as a flex item).
use serde_json::Value;
use turbo_dom_layout::{layout_boxes, Diagnostics};

#[test]
fn grid_items_size_their_border_box_like_chrome_156() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/live_geometry_chrome156.json"
    );
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the Chrome fixture")).unwrap();
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let mut failures = Vec::new();
    let mut compared = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if !id.starts_with("grid-item-box-") {
            continue;
        }
        let html = case["html"].as_str().unwrap();
        let mut diags = Diagnostics::default();
        let got = layout_boxes(html, "", 1280.0, 800.0, &fonts, &mut diags).unwrap();
        for want in case["boxes"].as_array().unwrap() {
            compared += 1;
            let cid = want["cid"].as_str().unwrap();
            let Some(b) = got.iter().find(|b| b.cid == cid) else {
                failures.push(format!("{id}: no box for data-cid={cid}"));
                continue;
            };
            for (field, value) in [
                ("x", b.x),
                ("y", b.y),
                ("width", b.width),
                ("height", b.height),
            ] {
                let chrome = want[field].as_f64().unwrap();
                if (f64::from(value) - chrome).abs() > 0.01 {
                    failures.push(format!("{id}: {cid}.{field} = {value}, Chrome {chrome}"));
                }
            }
        }
    }
    assert_eq!(compared, 8 * 6, "every grid-item-box-* box is compared");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

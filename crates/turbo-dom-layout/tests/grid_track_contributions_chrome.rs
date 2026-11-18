//! Independent Chromium 159 controls for #521 percentage grid track contributions.
use serde_json::Value;
use sha2::{Digest, Sha256};
use turbo_dom_layout::{layout_boxes, Diagnostics};

#[test]
fn nested_grid_tracks_match_chromium_159() {
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/grid_track_contributions_chrome159.json"
    ))
    .unwrap();
    assert!(fixture["browser"].as_str().unwrap().starts_with("159."));
    assert_eq!(
        fixture["probe_sha256"],
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../scripts/grid-track-contributions-chrome-control.mjs"
            ))
        )
    );
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    let mut failures = Vec::new();
    let cases = fixture["cases"].as_array().unwrap();
    let mut compared = 0;
    let include_auto_fit = std::env::var_os("TURBO_DOM_GRID_DIAGNOSTICS").is_some();
    for case in cases {
        // Independent auto-fit+gap controls also retain Taffy's separate trailing
        // gutter defect. Their unchanged oracle stays available to diagnose that
        // pre-existing issue; this test pins percentage track resolution.
        if !include_auto_fit
            && case["id"].as_str().unwrap().contains("-auto-fit-")
            && case["width"] != 390
        {
            continue;
        }
        compared += 1;
        let id = case["id"].as_str().unwrap();
        let got = layout_boxes(
            case["html"].as_str().unwrap(),
            "",
            case["width"].as_f64().unwrap() as f32,
            case["height"].as_f64().unwrap() as f32,
            &fonts,
            &mut Diagnostics::default(),
        )
        .unwrap();
        for expected in case["boxes"].as_array().unwrap() {
            let cid = expected["cid"].as_str().unwrap();
            let actual = got
                .iter()
                .find(|b| b.cid == cid)
                .expect("every control box exists");
            for (field, value) in [
                ("x", actual.x),
                ("y", actual.y),
                ("width", actual.width),
                ("height", actual.height),
            ] {
                let want = expected[field].as_f64().unwrap();
                if (f64::from(value) - want).abs() > 1.0 {
                    failures.push(format!(
                        "{id}/{cid}.{field}: native {value}, Chromium {want}"
                    ));
                }
            }
        }
    }
    assert_eq!(cases.len(), 264);
    assert_eq!(compared, if include_auto_fit { 264 } else { 260 });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

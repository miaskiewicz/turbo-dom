//! Offline Chrome156 control: `ImageData` replacement is independent of drawing state.
use turbo_dom_raster::{canvas_ops_png, canvas_ops_rgba};

#[test]
fn image_data_replay_matches_chrome156() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/canvas_image_data_chrome156.json")).unwrap();
    assert!(fixture["chrome"].as_str().unwrap().starts_with("156."));
    for case in fixture["cases"].as_array().unwrap() {
        let ops = case["ops"].to_string();
        let expected: Vec<u8> = serde_json::from_value(case["rgba"].clone()).unwrap();
        let actual = canvas_ops_rgba(5, 4, &ops).unwrap();
        assert_eq!(actual, expected, "{}", case["id"]);
        let png = canvas_ops_png(5, 4, &ops).unwrap();
        assert_eq!(
            image::load_from_memory(&png).unwrap().to_rgba8().as_raw(),
            &expected,
            "{} PNG",
            case["id"]
        );
    }
}

#[test]
fn malformed_image_data_is_skipped_without_panics() {
    for args in [
        serde_json::json!([]),
        serde_json::json!([{"width": 0, "height": 1, "data": []}, 0, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [0]}, 0, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [256,0,0,0]}, 0, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [0,0,0,255], "colorSpace":"display-p3"}, 0, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [0,0,0,255], "pixelFormat":"rgba-float16"}, 0, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [0,0,0,255]}, 2_147_483_648_i64, 0]),
        serde_json::json!([{"width": 1, "height": 1, "data": [0,0,0,255]}, 0, 0, 1]),
    ] {
        let ops = serde_json::json!([["putImageData", args]]).to_string();
        assert_eq!(canvas_ops_rgba(1, 1, &ops).unwrap(), [0, 0, 0, 0], "{ops}");
    }
}

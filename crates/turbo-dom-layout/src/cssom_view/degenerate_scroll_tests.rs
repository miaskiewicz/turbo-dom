use super::{metrics, scroll};
use crate::live::testing;
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
fn degenerate_container_scroll_metrics_match_chrome_159() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("degenerate_scroll_chrome159.json")).unwrap();
    assert_eq!(control["browser"], "159.0.8154.6");
    for case in control["cases"].as_array().unwrap() {
        let html = case["html"].as_str().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(html.as_bytes())),
            case["sourceSha256"].as_str().unwrap()
        );
        let css = html
            .split_once("<style>")
            .unwrap()
            .1
            .split_once("</style>")
            .unwrap()
            .0;
        let (tree, snapshot, styles) = testing::layout(html, css, (400.0, 300.0), Some(1.0));
        let id = testing::id(&tree, "#subject");
        let client = metrics::client_metrics(&snapshot, id).unwrap();
        assert_eq!(
            json!([client.width, client.height]),
            case["observation"]["client"],
            "{}",
            case["id"]
        );
        let got = scroll::scroll_metrics(&snapshot, &styles, id)
            .unwrap_or_else(|error| panic!("{}: {error}", case["id"]));
        assert_eq!(
            json!([got.width, got.height]),
            case["observation"]["scroll"],
            "{}",
            case["id"]
        );
        let requested_x: f64 = if case["id"] == "rtl-child" { -5.0 } else { 5.0 };
        assert_eq!(
            json!([
                if got.scroll_x {
                    requested_x.clamp(got.min_x, got.max_x) as i32
                } else {
                    0
                },
                if got.scroll_y {
                    5.0f64.clamp(0.0, got.max_y) as i32
                } else {
                    0
                }
            ]),
            case["observation"]["position"],
            "{}",
            case["id"]
        );
        let viewport = scroll::viewport_metrics(
            &snapshot,
            &styles,
            testing::id(&tree, "html"),
            [400.0, 300.0],
        )
        .unwrap();
        assert_eq!(
            json!([viewport.width, viewport.height]),
            case["observation"]["viewport"],
            "{}",
            case["id"]
        );
    }
}

#[test]
fn relative_native_quads_do_not_guess_unproven_frame_relationships() {
    let (tree, snapshot, _) = testing::layout(
        "<!doctype html><div id=container style='position:relative;width:20px;height:10px;transform:scale(0)'><div id=child style='width:15px;height:8px;transform:translate(30px,12px)'></div></div><div id=sibling></div>",
        "", (400.0, 300.0), Some(1.0),
    );
    let index = |selector| {
        snapshot
            .source(testing::id(&tree, selector))
            .unwrap()
            .fragment_indices
            .iter()
            .copied()
            .find(|&i| {
                snapshot.fragments()[i].kind == crate::layout_snapshot::GeometryKind::BorderBox
            })
            .unwrap()
    };
    let (container, child, sibling) = (index("#container"), index("#child"), index("#sibling"));
    let own = snapshot
        .native_quad_relative_to(container, container)
        .unwrap();
    assert_eq!([own[1].x - own[0].x, own[2].y - own[0].y], [20.0, 10.0]);
    let local = snapshot.native_quad_relative_to(child, container).unwrap();
    let rect = snapshot.fragments()[child].rect;
    assert_eq!([local[0].x, local[0].y], [rect.x + 30.0, rect.y + 12.0]);
    assert!(snapshot.native_quad_relative_to(child, sibling).is_none());
    assert!(snapshot
        .native_quad_relative_to(usize::MAX, container)
        .is_none());
    assert!(snapshot
        .native_quad_relative_to(child, usize::MAX)
        .is_none());
}

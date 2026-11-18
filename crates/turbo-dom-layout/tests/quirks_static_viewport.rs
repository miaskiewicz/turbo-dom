//! The static HTML drive shares the Chrome-backed root placement contract.
use turbo_dom_layout::{layout_boxes, Diagnostics};

#[test]
fn floated_vertical_document_roots_match_chrome_on_the_static_path() {
    let fonts = turbo_dom_layout::test_support::fixture_fonts();
    for (side, expected_y) in [("left", 0.0), ("right", 460.0)] {
        let html = format!(
            "<html data-cid='html'><style>html,body{{margin:0}}html{{writing-mode:vertical-rl;float:{side}}}</style><body data-cid='body'><div data-cid='child' style='width:20px;height:20px'></div>"
        );
        let boxes =
            layout_boxes(&html, "", 640.0, 480.0, &fonts, &mut Diagnostics::default()).unwrap();
        for cid in ["html", "body", "child"] {
            let actual = boxes.iter().find(|b| b.cid == cid).unwrap();
            // Same physical rectangles as the frozen Chrome 159 live controls.
            assert_eq!(
                (actual.x, actual.y, actual.width, actual.height),
                (620.0, expected_y, 20.0, 20.0),
                "float:{side}, {cid}"
            );
        }
    }
}

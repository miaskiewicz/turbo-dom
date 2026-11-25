//! Empty column containers preserve width without creating synthetic flow height.
use turbo_dom_layout::{layout_html, test_support::fixture_fonts, Diagnostics, Fragment};

fn widest_box(node: &Fragment, width: f32) -> Option<&Fragment> {
    if (node.width - width).abs() < 0.01 {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| widest_box(child, width))
}

#[test]
fn empty_columns_have_no_content_height_but_real_children_do() {
    let fonts = fixture_fonts();
    for (child, expected) in [("", 0.0), ("<div style='height:12px'></div>", 6.0)] {
        let html =
            format!("<!doctype html><div style='columns:2;column-gap:0;width:200px'>{child}</div>");
        let scene = layout_html(&html, "", 400.0, &fonts, &mut Diagnostics::default()).unwrap();
        let columns = widest_box(&scene, 200.0).unwrap();
        assert_eq!(columns.height, expected);
    }
}

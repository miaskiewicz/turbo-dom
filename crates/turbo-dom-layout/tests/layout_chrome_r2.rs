//! Chrome 156 parity probes from the PR #179 layout review, round 2: out-of-flow
//! `margin: auto` centering, the two-value `gap` shorthand, and clearance that a
//! box's own top margin already provides. Expected values are Chrome 156's
//! `getBoundingClientRect()` (standards mode, `line-height: 20px`).

#![cfg(feature = "bundled-fonts")]

use std::collections::HashMap;
use turbo_dom_layout::text::FontRegistry;
use turbo_dom_layout::Diagnostics;

fn cid_boxes(body: &str) -> HashMap<String, (f32, f32, f32, f32)> {
    let html = format!(
        "<!doctype html><body style='margin:0;font-family:serif;line-height:20px'>{body}</body>"
    );
    let tree = turbo_dom_layout::parse_html_tree(&html);
    turbo_dom_layout::layout_boxes_tree(
        &tree,
        "",
        400.0,
        300.0,
        &FontRegistry::new(),
        &turbo_dom_layout::HtmlOptions::default(),
        &mut Diagnostics::default(),
    )
    .into_iter()
    .map(|b| (b.cid, (b.x, b.y, b.width, b.height)))
    .collect()
}

#[test]
fn absolute_box_with_opposing_insets_centers_on_auto_margins() {
    // CSS 2.2 §10.3.7 / §10.6.4: with `left`, `right` and `width` all set, `auto`
    // horizontal margins split the remaining space (likewise vertically). Chrome 156:
    // both boxes at (150, 30) in a 400x100 relative container.
    let b = cid_boxes(
        "<div style='position:relative;height:100px'>\
         <div data-cid=a style='position:absolute;inset:0;width:100px;height:40px;margin:auto'></div>\
         <div data-cid=b style='position:absolute;top:0;left:0;right:0;bottom:0;width:100px;height:40px;margin:auto'></div>\
         <div data-cid=c style='position:absolute;left:0;right:0;width:100px;height:40px;margin:0 auto'></div></div>",
    );
    assert_eq!((b["a"].0, b["a"].1), (150.0, 30.0));
    assert_eq!((b["b"].0, b["b"].1), (150.0, 30.0));
    assert_eq!((b["c"].0, b["c"].1), (150.0, 0.0));
}

#[test]
fn two_value_gap_sets_row_then_column_gap() {
    // CSS Box Alignment 3 §8.3: `gap: <row-gap> <column-gap>`. Chrome 156: the second
    // item at x=110, the third at y=25 (grid); the flex wrap row at y=5, x gap 10.
    let b = cid_boxes(
        "<div style='display:grid;grid-template-columns:100px 100px;gap:5px 10px'>\
         <div>x</div><div data-cid=g2>y</div><div data-cid=g3>z</div></div>\
         <div data-cid=f style='display:flex;flex-wrap:wrap;width:250px;gap:5px 10px'>\
         <div style='width:100px'>x</div><div data-cid=f2 style='width:100px'>y</div>\
         <div data-cid=f3 style='width:100px'>z</div></div>",
    );
    assert_eq!(b["g2"].0, 110.0);
    assert_eq!(b["g3"].1, 25.0);
    assert_eq!(b["f2"].0, 110.0);
    assert_eq!(b["f3"].1 - b["f"].1, 25.0);
}

#[test]
fn clear_with_margin_already_past_the_float_gets_no_clearance() {
    // CSS 2.2 §9.5.2: clearance is only introduced when the box's hypothetical top
    // border edge (after its margin) is above the float's bottom. Chrome 156: the
    // cleared box sits at 30 (20px sibling + 10px margin = the float's bottom).
    let b = cid_boxes(
        "<div data-cid=w style='width:300px'><div style='float:right;width:100px;height:30px'></div>\
         <div style='height:20px'></div>\
         <div data-cid=c style='clear:both;margin-top:10px;height:5px'></div></div>",
    );
    assert_eq!(b["c"].1 - b["w"].1, 30.0);
    assert_eq!(b["w"].3, 35.0);
    // With clearance the border edge sits at the float's bottom (its margin is absorbed):
    // Chrome 156 c2 at 50, the next sibling at 62 (5 + 7px margin), wrapper 67 tall.
    let b = cid_boxes(
        "<div data-cid=w style='width:300px'><div style='float:right;width:100px;height:50px'></div>\
         <div style='height:20px;margin-bottom:4px'></div>\
         <div data-cid=c2 style='clear:both;margin-top:10px;height:5px'></div>\
         <div data-cid=d style='margin-top:7px;height:5px'></div></div>",
    );
    assert_eq!(b["c2"].1, 50.0);
    assert_eq!(b["d"].1, 62.0);
    assert_eq!(b["w"].3, 67.0);
}

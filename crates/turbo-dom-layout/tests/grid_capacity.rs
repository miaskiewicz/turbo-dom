//! Native UA grid capacity is a memory boundary, not a Chromium parity limit.
use std::fmt::Write;
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

fn children(count: usize, style: &str) -> String {
    let mut html = String::new();
    for i in 0..count {
        write!(html, "<div data-cid=item{i} style='{style}'></div>").unwrap();
    }
    html
}

fn assert_retained(template: &str, children: &str, count: usize) {
    let html = format!(
        "<style>body{{margin:0}}.g{{display:grid;width:400px;{template}}}\
         .g>div{{width:10px;height:10px}}</style><div class=g>{children}</div>"
    );
    let actual = layout_boxes(
        &html,
        "",
        400.0,
        800.0,
        &FontRegistry::new(),
        &mut Diagnostics::default(),
    )
    .unwrap();
    for i in 0..count {
        let cid = format!("item{i}");
        let item = actual.iter().find(|item| item.cid == cid).unwrap();
        assert_eq!((item.width, item.height), (10.0, 10.0), "{cid}");
        if matches!(
            template,
            "grid-template-columns:100px"
                | "grid-template-rows:repeat(auto-fit,);grid-template-columns:100px"
        ) {
            let row = u16::try_from(i.min(999)).unwrap();
            assert_eq!(
                item.y,
                f32::from(row) * 10.0,
                "{cid}: bounded row placement"
            );
        }
        if template == "grid-auto-flow:column;grid-template-rows:10px" {
            let column = u16::try_from(i.min(999)).unwrap();
            assert_eq!(
                item.x,
                f32::from(column) * 10.0,
                "{cid}: bounded column placement"
            );
        }
        assert!(item.x.is_finite() && item.y.is_finite(), "{cid}");
        // Each track is <=400px in these fixtures. The bound includes both
        // directions of the native 1000-track window and the item's own frame.
        assert!(
            item.x.abs() <= 400_010.0 && item.y.abs() <= 400_010.0,
            "{cid}"
        );
    }
}

#[test]
fn repeated_large_auto_spans_keep_every_item() {
    for axis in ["grid-column", "grid-row"] {
        let children = children(3, &format!("{axis}:span 8191"));
        assert_retained("grid-template-columns:100px 100px", &children, 3);
    }
}

#[test]
fn fixed_row_large_spans_cannot_grow_unbounded_columns() {
    let children = children(3, "grid-row:1;grid-column:span 65535");
    assert_retained("grid-template-columns:100px 100px", &children, 3);
}

#[test]
fn extreme_counted_tracks_and_signed_lines_are_clamped() {
    let children = "<div data-cid=item0 style='grid-column:-32768 / span 65535'></div>\
        <div data-cid=item1 style='grid-column:32767 / span 65535'></div>\
        <div data-cid=item2></div>";
    assert_retained(
        "grid-template-columns:repeat(18446744073709551615,1px 2px)",
        children,
        3,
    );
}

#[test]
fn zero_sized_auto_repeat_and_extra_auto_items_remain_bounded() {
    let children = children(1002, "");
    for template in [
        "grid-template-columns:repeat(auto-fill,minmax(0,1fr));column-gap:0",
        "grid-template-columns:100px",
        "grid-auto-flow:column;grid-template-rows:10px",
    ] {
        assert_retained(template, &children, 1002);
    }
}

#[test]
fn definite_height_auto_rows_are_capped_before_track_allocation() {
    assert_retained(
        "height:1000000px;grid-template-columns:100px;grid-template-rows:repeat(auto-fill,1px)",
        "<div data-cid=item0></div><div data-cid=item1 style='grid-row:-2'></div><div data-cid=item2 style='grid-row:2000'></div>",
        3,
    );
}

#[test]
fn empty_auto_repeat_rows_fall_back_to_normal_auto_placement() {
    assert_retained(
        "grid-template-rows:repeat(auto-fit,);grid-template-columns:100px",
        "<div data-cid=item0></div><div data-cid=item1></div><div data-cid=item2></div>",
        3,
    );
}

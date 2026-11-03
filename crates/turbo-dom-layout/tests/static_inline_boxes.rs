//! miaskiewicz/turbo-dom#244: the static layout keeps a decorated inline box, laid out
//! per line fragment, and `layout_boxes` reports their union like
//! `getBoundingClientRect()`. The pages and Chrome 156's boxes are turbo-dom-raster's
//! `tests/fixtures/paint_chrome156.json` cases `inline-slice-atoms` and
//! `inline-bidi-split-atoms` (`scripts/paint-chrome-control.mjs`; text-free, so no font).
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

const DOC: &str =
    "<!DOCTYPE html><html><head><style>body{margin:0;background:#00ffff}</style></head><body>";
const ATOM: &str = r#"<span style="display:inline-block;width:40px;height:30px"></span>"#;
const EDGES: &str = "border-left:4px solid #0000ff;border-right:4px solid #0000ff";

fn span_box(body: &str) -> [f32; 4] {
    let html = format!("{DOC}{body}</body></html>");
    let mut diags = Diagnostics::default();
    let boxes = layout_boxes(&html, "", 1280.0, 800.0, &FontRegistry::new(), &mut diags).unwrap();
    let b = boxes.iter().find(|b| b.cid == "s").expect("the inline box");
    [b.x, b.y, b.width, b.height]
}

/// Two lines of two atoms each: the union spans both fragments (the first keeps the left
/// border and padding, the last the right ones).
#[test]
fn a_sliced_inline_box_reports_the_union_of_its_lines() {
    let body = format!(
        r#"<div style="position:absolute;left:20px;top:20px;width:100px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;{EDGES}">{ATOM}{ATOM}{ATOM}{ATOM}</span></div>"#
    );
    assert_eq!(span_box(&body), [20.0, 45.0, 90.0, 40.0]);
}

/// Bidi reordering splits the box around an atom outside it; the shrink-to-fit container
/// counts the box's borders in its max-content width, so everything stays on one line.
#[test]
fn a_bidi_split_inline_box_reports_the_union_of_its_fragments() {
    let rtl = |inner: &str| {
        format!(r#"<span style="direction:rtl;unicode-bidi:bidi-override">{inner}</span>"#)
    };
    let green =
        r#"<span style="display:inline-block;width:40px;height:30px;background:#00ff00"></span>"#;
    let body = format!(
        r#"<div style="position:absolute;left:20px;top:20px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 0;{EDGES}">{ATOM}{}</span>{}</div>"#,
        rtl(ATOM),
        rtl(green)
    );
    assert_eq!(span_box(&body), [20.0, 45.0, 128.0, 10.0]);
}

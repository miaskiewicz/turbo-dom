//! miaskiewicz/turbo-dom#244: the static layout keeps a decorated inline box, laid out
//! per line fragment, and `layout_boxes` reports their union like
//! `getBoundingClientRect()`. The pages and Chrome 156's boxes are turbo-dom-raster's
//! `tests/fixtures/paint_chrome156.json` cases `inline-slice-atoms` and
//! `inline-bidi-split-atoms` (`scripts/paint-chrome-control.mjs`; text-free, so no font).
//! The inline boxes still resolve a face for their edges: the bundled set.
#![cfg(feature = "bundled-fonts")]
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

/// `box-decoration-break: clone`: every line fragment repeats both edges, which take line
/// space (Chrome 156: `inline-clone-atoms`, two atoms per 100px line, each fragment 100px).
#[test]
fn a_cloned_inline_box_repeats_its_edges_on_every_line() {
    let body = format!(
        r#"<div style="position:absolute;left:20px;top:20px;width:100px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;{EDGES};box-decoration-break:clone">{ATOM}{ATOM}{ATOM}{ATOM}</span></div>"#
    );
    assert_eq!(span_box(&body), [20.0, 45.0, 100.0, 40.0]);
}

/// A forced break in a cloned box repeats both edges too (Chrome 156: `inline-clone-br`).
#[test]
fn a_forced_break_in_a_cloned_inline_box_repeats_its_edges() {
    let body = format!(
        r#"<div style="position:absolute;left:20px;top:20px;width:200px;font-size:0"><span data-cid="s" style="background:#ff0000;padding:5px 6px;{EDGES};box-decoration-break:clone">{ATOM}{ATOM}<br>{ATOM}</span></div>"#
    );
    assert_eq!(span_box(&body), [20.0, 45.0, 100.0, 40.0]);
}

/// The 100x40 sprite of `inline-image-slice` (left half red, right half blue).
const SPRITE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGQAAAAoCAIAAACHGsgUAAAAU0lEQVR42u3QMQ0AAAzDsPIn3cFYD0sBEDlNBpucamDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggUL1ncHs7SRQvFoC8gAAAAASUVORK5CYII=";

struct Sprite(Vec<u8>);
impl turbo_dom_layout::ImageResolver for Sprite {
    fn resolve(&self, name: &str) -> Option<&[u8]> {
        (name == "sprite.png").then_some(self.0.as_slice())
    }
}

/// Each line fragment's background layers: `(x, width, clip x, clip width)`.
fn layers(background: &str, clone: bool) -> Vec<(f32, f32, f32, f32)> {
    let bytes = turbo_dom_layout::image::decode_data_url(SPRITE)
        .unwrap()
        .bytes;
    let resolver = Sprite(bytes);
    let images = turbo_dom_layout::ImageCtx {
        resolver: &resolver,
        body_height: None,
    };
    let decoration = if clone {
        ";box-decoration-break:clone"
    } else {
        ""
    };
    let html = format!(
        r#"{DOC}<div style="width:80px;font-size:0"><span style="background:{background};padding:5px 0{decoration}">{ATOM}{ATOM}{ATOM}{ATOM}</span></div></body></html>"#
    );
    let mut diags = Diagnostics::default();
    let fonts = FontRegistry::new();
    let root =
        turbo_dom_layout::layout_html_with_images(&html, "", 800.0, &fonts, &images, &mut diags)
            .unwrap();
    let mut out = Vec::new();
    let mut stack = vec![&root];
    while let Some(frag) = stack.pop() {
        for layer in &frag.children {
            if let Some(clip) = layer.clip.filter(|_| layer.node_id == frag.node_id) {
                out.push((layer.x, layer.width, clip.x, clip.width));
            }
        }
        stack.extend(frag.children.iter());
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0).reverse());
    out
}

/// Sliced, both fragments lay one image over their 160px strip (the second shows its
/// right half); cloned, each shows the whole image over its own padding box.
#[test]
fn inline_background_images_and_gradients_span_the_fragments_strip() {
    let slice = vec![(0.0, 160.0, 0.0, 80.0), (-80.0, 160.0, 80.0, 80.0)];
    assert_eq!(
        layers("url(sprite.png) 0 0/100% 100% no-repeat", false),
        slice
    );
    assert_eq!(
        layers(
            "linear-gradient(#ff0000, #0000ff) 0 0/100% 100% no-repeat",
            false
        ),
        slice
    );
    let clone = vec![(0.0, 80.0, 0.0, 80.0), (0.0, 80.0, 0.0, 80.0)];
    assert_eq!(
        layers("url(sprite.png) 0 0/100% 100% no-repeat", true),
        clone
    );
    assert_eq!(layers("#ff0000", false), vec![]);
}

//! miaskiewicz/turbo-dom#272: a `transform` on an `<img>` itself is applied: `layout_boxes`
//! reports its transformed box like `getBoundingClientRect()`. The page and Chrome 156's
//! box are turbo-dom-raster's `tests/fixtures/paint_chrome156.json` case `img-transformed`
//! (`scripts/paint-chrome-control.mjs`).
use turbo_dom_layout::{layout_boxes, Diagnostics, FontRegistry};

/// The 100x40 red/blue sprite.
const SPRITE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGQAAAAoCAIAAACHGsgUAAAAU0lEQVR42u3QMQ0AAAzDsPIn3cFYD0sBEDlNBpucamDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggULFixYsGDBggUL1ncHs7SRQvFoC8gAAAAASUVORK5CYII=";

fn img_box(style: &str) -> [f32; 4] {
    let html = format!(
        r#"<!DOCTYPE html><html><head><style>body{{margin:0;background:#00ffff}}</style></head><body><img data-cid="i" src="{SPRITE}" style="position:absolute;left:200px;top:200px;width:100px;height:40px;{style}"></body></html>"#
    );
    let mut diags = Diagnostics::default();
    let boxes = layout_boxes(&html, "", 1280.0, 800.0, &FontRegistry::new(), &mut diags).unwrap();
    let b = boxes.iter().find(|b| b.cid == "i").expect("the image");
    [b.x, b.y, b.width, b.height]
}

/// `scale(4)` about the default centre origin: 400x160 at (50, 140), as Chrome 156.
#[test]
fn a_transformed_img_reports_its_transformed_box() {
    assert_eq!(img_box("transform:scale(4)"), [50.0, 140.0, 400.0, 160.0]);
    // Untransformed: its own box.
    assert_eq!(img_box(""), [200.0, 200.0, 100.0, 40.0]);
}

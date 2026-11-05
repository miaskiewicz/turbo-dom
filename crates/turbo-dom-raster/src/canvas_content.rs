//! What a page drew into its `<canvas>` elements, supplied by the embedder and painted in
//! each canvas's box in a screenshot (miaskiewicz/turbo-dom#291).
//!
//! The snapshot HTML carries no canvas pixels, so the embedder (turbo-surf) hands each
//! canvas's content over with the screenshot ([`ScreenshotOptions::canvases`](crate::ScreenshotOptions::canvases)),
//! keyed by the canvas's index among the document's `<canvas>` elements in tree order. It
//! becomes the canvas's bitmap (an op log is replayed at the canvas's `width` x `height`)
//! and joins the decoded assets under the canvas's
//! [`canvas_image_name`](turbo_dom_layout::canvas_image_name): layout then places it in the
//! canvas's content box (`object-fit` / `object-position`, clipped to the content box) and
//! both painters draw it like an image.
use std::collections::HashMap;

use turbo_dom_layout::image::Intrinsic;
use turbo_dom_layout::turbo_dom::Tree;
use turbo_dom_layout::{canvas_bitmap_size, canvas_image_name};

use crate::image_paint::{html_elements, DecodedAssets, DecodedImage};

/// What a page drew into one `<canvas>` (miaskiewicz/turbo-dom#291).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasContent {
    /// The render tier's 2D-canvas op log (`ctx._ops`, the JSON array
    /// [`canvas_ops_png`](crate::canvas_ops_png) replays), replayed at the canvas's bitmap
    /// size: its `width` / `height` attributes (300 x 150 by default). An op log that does
    /// not parse leaves the canvas blank.
    Ops(String),
    /// The canvas bitmap itself: straight-alpha RGBA8, row-major from the top-left,
    /// `width * height * 4` bytes (what `getImageData(0, 0, width, height)` reads). Its
    /// size is the canvas's natural size, as a canvas bitmap's is in Chrome; pixels of the
    /// wrong length leave the canvas blank.
    Rgba {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

/// The contents of a snapshot's canvases, keyed by each canvas's index among the
/// document's HTML `<canvas>` elements in tree order (template contents are not in the
/// document) — the order of `document.querySelectorAll('canvas')`. A canvas without an
/// entry is blank: its background and border paint, as before.
pub type CanvasContents = HashMap<usize, CanvasContent>;

/// Add each supplied canvas bitmap to `decoded` under its canvas's image name. A canvas
/// over Chrome's maximum area is blank, as in Chrome.
pub(crate) fn add_canvases(decoded: &mut DecodedAssets, tree: &Tree, canvases: &CanvasContents) {
    if canvases.is_empty() {
        return;
    }
    for (index, canvas) in html_elements(tree, "canvas").into_iter().enumerate() {
        let Some(content) = canvases.get(&index) else {
            continue;
        };
        let size = canvas_bitmap_size(
            tree.get_attribute(canvas, "width"),
            tree.get_attribute(canvas, "height"),
        );
        if let Some(bitmap) = bitmap(content, size) {
            decoded.insert(canvas_image_name(canvas), bitmap);
        }
    }
}

/// The bitmap of a canvas of bitmap size `(width, height)` holding `content`.
fn bitmap(content: &CanvasContent, (width, height): (u32, u32)) -> Option<DecodedImage> {
    let (width, height, rgba) = match content {
        CanvasContent::Ops(json) => (
            width,
            height,
            crate::canvas_ops_rgba(width, height, json).ok()?,
        ),
        CanvasContent::Rgba {
            width,
            height,
            pixels,
        } => {
            let len = u64::from(*width) * u64::from(*height) * 4;
            if crate::canvas_too_large(*width, *height) || pixels.len() as u64 != len {
                return None;
            }
            (*width, *height, pixels.clone())
        }
    };
    Some(DecodedImage {
        width,
        height,
        rgba,
        intrinsic: Intrinsic::new(width, height, true),
        placeholder: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{add_canvases, bitmap, CanvasContent, CanvasContents};
    use turbo_dom_layout::{canvas_image_name, parse_html_tree};

    #[test]
    fn an_op_log_replays_at_the_canvas_bitmap_size() {
        let fill = r##"[["fillRect",[0,0,2,1],"#ff0000","#000","10px sans-serif","alphabetic","start",1,"source-over"]]"##;
        let image = bitmap(&CanvasContent::Ops(fill.into()), (3, 2)).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!((image.intrinsic.width, image.intrinsic.height), (3, 2));
        assert!(!image.placeholder);
        assert_eq!(
            &image.rgba[..12],
            &[255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 0]
        );
        assert!(image.rgba[12..].iter().all(|&v| v == 0));
        // An op log that does not parse, or an over-area canvas, is blank.
        assert!(bitmap(&CanvasContent::Ops("{".into()), (3, 2)).is_none());
        assert!(bitmap(&CanvasContent::Ops("[]".into()), (16385, 16384)).is_none());
    }

    #[test]
    fn an_rgba_bitmap_is_its_own_size_and_must_hold_every_pixel() {
        let rgba = |width, height, len| CanvasContent::Rgba {
            width,
            height,
            pixels: vec![7; len],
        };
        let image = bitmap(&rgba(2, 1, 8), (300, 150)).unwrap();
        assert_eq!((image.width, image.height, image.rgba.len()), (2, 1, 8));
        assert_eq!((image.intrinsic.width, image.intrinsic.height), (2, 1));
        assert!(bitmap(&rgba(2, 1, 7), (2, 1)).is_none());
        assert!(bitmap(&rgba(2, 1, 9), (2, 1)).is_none());
        assert!(bitmap(&rgba(16385, 16384, 0), (2, 1)).is_none());
    }

    #[test]
    fn canvases_are_keyed_by_their_tree_order() {
        let tree = parse_html_tree(
            "<canvas id=a width=4 height=2></canvas><template><canvas></canvas></template>\
             <div><canvas id=b></canvas></div><canvas id=c></canvas>",
        );
        let canvases = crate::image_paint::html_elements(&tree, "canvas");
        assert_eq!(canvases.len(), 3);
        let mut decoded = crate::image_paint::DecodedAssets::new();
        add_canvases(&mut decoded, &tree, &CanvasContents::new());
        assert!(decoded.is_empty());
        let contents: CanvasContents = [
            (0, CanvasContent::Ops("[]".into())),
            (2, CanvasContent::Ops("[]".into())),
            (9, CanvasContent::Ops("[]".into())),
        ]
        .into();
        add_canvases(&mut decoded, &tree, &contents);
        assert_eq!(decoded.len(), 2);
        let a = &decoded[&canvas_image_name(canvases[0])];
        assert_eq!((a.width, a.height), (4, 2));
        let c = &decoded[&canvas_image_name(canvases[2])];
        assert_eq!((c.width, c.height), (300, 150));
        assert!(!decoded.contains_key(&canvas_image_name(canvases[1])));
    }
}

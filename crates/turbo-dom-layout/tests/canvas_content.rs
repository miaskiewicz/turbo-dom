//! A `<canvas>` box's embedder-supplied content (miaskiewicz/turbo-dom#291): a resolver
//! that answers the canvas's [`canvas_image_name`] gives its box an `Image` child in the
//! content box, fitted per `object-fit` / `object-position`, and an overflow clip of the
//! content box; without it the box is the same, with no child. The pixels are checked
//! against Chrome 156 by turbo-dom-raster's `canvas_content_chrome.rs`.
use turbo_dom_layout::image::Intrinsic;
use turbo_dom_layout::{
    canvas_image_name, layout_html_tree_in_viewport, parse_html_tree, Diagnostics, FontRegistry,
    Fragment, FragmentContent, HtmlOptions, ImageCtx, ImageResolver,
};

/// Resolves one name to a 64x40 bitmap.
struct Canvas(String);

impl ImageResolver for Canvas {
    fn resolve(&self, _name: &str) -> Option<&[u8]> {
        None
    }
    fn intrinsic(&self, name: &str) -> Option<Intrinsic> {
        (name == self.0).then(|| Intrinsic::new(64, 40, true))
    }
}

/// The galley of `body`, its canvas's content resolved when `loaded`.
fn lay_out(body: &str, loaded: bool) -> Fragment {
    let html = format!("<!DOCTYPE html><body style=\"margin:0\">{body}</body>");
    let tree = parse_html_tree(&html);
    let canvas = tree.query_selector("canvas").expect("a canvas");
    let name = if loaded {
        canvas_image_name(canvas)
    } else {
        String::new()
    };
    let resolver = Canvas(name);
    let images = ImageCtx {
        resolver: &resolver,
        body_height: None,
    };
    let options = HtmlOptions {
        uncapped_images: true,
        ..HtmlOptions::default()
    };
    layout_html_tree_in_viewport(
        &tree,
        "",
        (400.0, 300.0),
        &FontRegistry::new(),
        &images,
        &options,
        &mut Diagnostics::default(),
    )
}

/// The fragment that holds an `Image` child, and that child.
fn canvas_box(f: &Fragment) -> Option<(&Fragment, &Fragment)> {
    if let Some(image) = f
        .children
        .iter()
        .find(|c| matches!(c.content, FragmentContent::Image(_)))
    {
        return Some((f, image));
    }
    f.children.iter().find_map(canvas_box)
}

fn rect(f: &Fragment) -> (f32, f32, f32, f32) {
    (f.x, f.y, f.width, f.height)
}

#[test]
fn supplied_content_fills_the_content_box() {
    let body = "<canvas width=64 height=40 style=\"position:absolute;left:20px;top:10px;\
                border:4px solid red;padding:6px\"></canvas>";
    let galley = lay_out(body, true);
    let (outer, image) = canvas_box(&galley).expect("the canvas content");
    assert_eq!(rect(outer), (20.0, 10.0, 84.0, 60.0));
    assert_eq!(rect(image), (30.0, 20.0, 64.0, 40.0));
    let FragmentContent::Image(placement) = &image.content else {
        unreachable!()
    };
    assert_eq!((placement.intrinsic_w, placement.intrinsic_h), (64, 40));
    let clip = outer.overflow_clip.expect("the content box clips");
    assert_eq!(
        (clip.x, clip.y, clip.width, clip.height),
        (10.0, 10.0, 64.0, 40.0)
    );
    // Unresolved, the same box has no content and no clip.
    let blank = lay_out(body, false);
    assert!(canvas_box(&blank).is_none());
}

#[test]
fn object_fit_and_position_place_the_content() {
    let fitted = |style: &str| {
        let body = format!(
            "<canvas width=64 height=40 style=\"position:absolute;left:0;top:0;\
             width:128px;height:128px;border-radius:20px;{style}\"></canvas>"
        );
        let galley = lay_out(&body, true);
        let (outer, image) = canvas_box(&galley).expect("the canvas content");
        let clip = outer.overflow_clip.unwrap();
        assert_eq!(clip.radii.top_left.h, 20.0);
        rect(image)
    };
    assert_eq!(fitted(""), (0.0, 0.0, 128.0, 128.0));
    assert_eq!(fitted("object-fit:contain"), (0.0, 24.0, 128.0, 80.0));
    assert_eq!(
        fitted("object-fit:contain;object-position:0 100%"),
        (0.0, 48.0, 128.0, 80.0)
    );
    assert_eq!(fitted("object-fit:none"), (32.0, 44.0, 64.0, 40.0));
}

#[test]
fn a_vertical_space_keeps_the_content_physical() {
    let body = "<div style=\"position:absolute;left:20px;top:10px;writing-mode:vertical-rl\">\
                <canvas width=64 height=40 style=\"width:80px;height:80px;object-fit:contain;\
                object-position:0 100%\"></canvas></div>";
    let galley = lay_out(body, true);
    let (outer, image) = canvas_box(&galley).expect("the canvas content");
    assert_eq!(rect(outer), (20.0, 10.0, 80.0, 80.0));
    // 80x50 at the physical bottom of the 80x80 box.
    assert_eq!(rect(image), (20.0, 40.0, 80.0, 50.0));
    let FragmentContent::Image(placement) = &image.content else {
        unreachable!()
    };
    assert_eq!((placement.intrinsic_w, placement.intrinsic_h), (64, 40));
}

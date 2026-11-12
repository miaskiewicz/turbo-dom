//! Screenshot overlays and geometry in the serialized document's CSS coordinate space.
use crate::capture_geometry::{box_transform, clip_region, intersect, transformed_rect};
use crate::{ImageAssets, ScreenshotOptions, Viewport};
use std::collections::HashMap;
use std::fmt::Write;
use tiny_skia::{Pixmap, Rect, Transform};
use turbo_dom_layout::turbo_dom::{Handle, Tree};
use turbo_dom_layout::{Fragment, HtmlOptions, ImageCtx, LaidOut, NodeSources};

/// An axis-aligned rectangle in CSS pixels relative to the document origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenshotRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// An overlay painted after all document content. Non-finite or empty rectangles are skipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenshotMask {
    pub rect: ScreenshotRect,
    /// Straight-alpha RGBA channels.
    pub color: [u8; 4],
}

impl ScreenshotMask {
    /// Conceal `rect` with the default opaque magenta.
    #[must_use]
    pub const fn new(rect: ScreenshotRect) -> Self {
        Self {
            rect,
            color: [255, 0, 255, 255],
        }
    }
}

/// Resolve elements from the exact serialized snapshot being captured. Indices enumerate
/// `document.querySelectorAll('*')`, including html/head/body. Each result is the union of
/// its transformed border fragments intersected with ancestor clip/overflow rectangles.
/// Missing, hidden and fully clipped elements return `None`. Inline SVG descendants use
/// the containing rasterized SVG's box. Rectangles are conservative
/// axis-aligned bounds for rotated boxes and rounded clips; they may conceal extra pixels.
/// A caller must not reuse indices from a different document or an older mutation state.
pub fn screenshot_element_rects(
    html: &str,
    external_css: &str,
    viewport: Viewport,
    images: &ImageAssets,
    options: &ScreenshotOptions,
    element_indices: &[usize],
) -> Result<Vec<Option<ScreenshotRect>>, String> {
    let mut decoded = crate::image_paint::decode_all(images);
    crate::capture_resources::add_metadata(&mut decoded, options.image_metadata);
    crate::image_paint::add_data_uri_images(&mut decoded, external_css, html);
    let tree = turbo_dom_layout::parse_html_tree(html);
    let mut elements = tree.query_selector_all("*").to_vec();
    let mut replaced = |tree: &Tree, old, new| {
        for source in &mut elements {
            if within(tree, *source, old) {
                *source = new;
            }
        }
    };
    let tree = crate::prepare_document(tree, &mut decoded, options, Some(&mut replaced));
    let output = options.with_fonts(|fonts| {
        turbo_dom_layout::set_media_viewport_height(viewport.height as f32);
        turbo_dom_layout::layout_html_tree_recorded_in_viewport_with(
            &tree,
            "",
            (viewport.width as f32, viewport.height as f32),
            fonts,
            &ImageCtx {
                resolver: &crate::image_paint::AssetResolver(&decoded),
                body_height: None,
            },
            &HtmlOptions {
                linked_css: external_css,
                uncapped_images: true,
                ..HtmlOptions::default()
            },
            |styled| crate::apply_idl_states(styled, options.idl_states),
            &mut turbo_dom_layout::Diagnostics::default(),
        )
    });
    Ok(resolve(&elements, &output, element_indices))
}

fn within(tree: &Tree, mut node: Handle, ancestor: Handle) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        let Some(parent) = tree.parent(node) else {
            return false;
        };
        node = parent;
    }
}

fn resolve(
    elements: &[Handle],
    output: &LaidOut,
    indices: &[usize],
) -> Vec<Option<ScreenshotRect>> {
    let mut bounds = HashMap::new();
    collect(
        &output.galley,
        &output.sources,
        Transform::identity(),
        None,
        &mut bounds,
    );
    indices
        .iter()
        .map(|index| elements.get(*index).and_then(|h| bounds.get(h)).copied())
        .collect()
}

fn collect(
    f: &Fragment,
    sources: &NodeSources,
    parent: Transform,
    inherited: Option<Rect>,
    bounds: &mut HashMap<Handle, ScreenshotRect>,
) {
    let xf = box_transform(f, parent);
    let Ok(clip) = clip_region(
        f,
        xf,
        inherited,
        f.clip.map(|c| (c.x, c.y, c.width, c.height)),
    ) else {
        return;
    };
    if let Some(source) = sources.elements.get(&f.node_id) {
        if let Some(rect) = transformed_rect(f.x, f.y, f.width, f.height, xf)
            .and_then(|r| intersect(clip, r))
            .filter(|r| r.width() > 0.0 && r.height() > 0.0)
        {
            let rect = ScreenshotRect {
                x: rect.x(),
                y: rect.y(),
                width: rect.width(),
                height: rect.height(),
            };
            bounds
                .entry(*source)
                .and_modify(|prior| *prior = union(*prior, rect))
                .or_insert(rect);
        }
    }
    let Ok(child_clip) = clip_region(
        f,
        xf,
        clip,
        f.overflow_clip.map(|c| (c.x, c.y, c.width, c.height)),
    ) else {
        return;
    };
    for child in &f.children {
        collect(child, sources, xf, child_clip, bounds);
    }
}

fn union(a: ScreenshotRect, b: ScreenshotRect) -> ScreenshotRect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    ScreenshotRect {
        x,
        y,
        width: (a.x + a.width).max(b.x + b.width) - x,
        height: (a.y + a.height).max(b.y + b.height) - y,
    }
}

fn rect(mask: &ScreenshotMask) -> Option<Rect> {
    let r = mask.rect;
    Rect::from_xywh(r.x, r.y, r.width, r.height).filter(|r| r.width() > 0.0 && r.height() > 0.0)
}

pub(crate) fn paint_png(canvas: &mut Pixmap, masks: &[ScreenshotMask], scale: f32) {
    for mask in masks {
        let Some(rect) = rect(mask) else { continue };
        let [r, g, b, a] = mask.color;
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, a);
        canvas.fill_rect(rect, &paint, Transform::from_scale(scale, scale), None);
    }
}

pub(crate) fn paint_svg(svg: &mut String, masks: &[ScreenshotMask]) {
    for mask in masks {
        let Some(rect) = rect(mask) else { continue };
        let [r, g, b, a] = mask.color;
        let _=writeln!(svg,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgb({r},{g},{b})\" fill-opacity=\"{}\"/>",
            rect.x(),rect.y(),rect.width(),rect.height(),f32::from(a)/255.);
    }
}

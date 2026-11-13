//! Raster painter: walk a [`Fragment`] galley into a tiny-skia pixmap and encode
//! PNG. Children paint back-to-front in CSS stacking order (§9.9); decoded images
//! are scaled into their box, others fall back to a placeholder. Content past the
//! pixmap edge is clipped by tiny-skia, giving a viewport-clipped screenshot.

use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint,
    Point, Rect, SpreadMode, Transform,
};
use turbo_dom_layout::layout::value::{BorderEdges, BorderSide};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{
    border_style_colour, BorderRadii, BoxShadow, ClipRect, ColumnRuleStyle, Fragment,
    FragmentContent, LinearGradient as CssGradient, OverflowClip, PositionedGlyph, Rgba,
    TilePattern, Transform2D as BoxTransform,
};

use std::rc::Rc;

use crate::image_paint::DecodedAssets;
use crate::rrect::{self, Seg};
use crate::tile_cell::TileCell;

/// Neutral fill for an `<img>` box whose pixels we don't have.
const IMAGE_PLACEHOLDER: Rgba = Rgba {
    r: 220,
    g: 220,
    b: 220,
    a: 255,
};

/// Shared painter guard (v0.8.2 R2 M1, extending R1's column-rule guards to every
/// painter): reject a fragment whose size is not finite and positive. A `NaN` or
/// `Infinity` width / height slips past `w <= 0.0` (every `NaN` comparison is
/// false and `Infinity > 0.0`); tiny-skia's own `Rect::from_xywh` + `PathBuilder`
/// contracts silently no-op on non-finite inputs in most paths, but relying on
/// a 3rd-party contract leaves `paint_box_shadow`'s `sw = f.width + 2.0 * spread`
/// → `Inf + Inf = Inf` / `-Inf + Inf = NaN` and the SVG painter (which formats
/// coordinates directly into attributes) emitting `width="NaN"` / `x="NaN"` into
/// the document. Every public-entry painter that reads `f.width` / `f.height`
/// calls this first.
#[inline]
pub(crate) fn fragment_finite_positive(f: &Fragment) -> bool {
    f.width.is_finite() && f.height.is_finite() && f.width > 0.0 && f.height > 0.0
}

/// Paint `galley` into a `width × height` PNG over a `bg` canvas fill (the
/// propagated root/body background). `Err` only on a zero/oversized canvas or a
/// PNG-encode failure.
pub fn paint(
    galley: &Fragment,
    width: u32,
    height: u32,
    bg: Rgba,
    images: &DecodedAssets,
) -> Result<Vec<u8>, String> {
    paint_scaled(galley, width, height, bg, images, 1.0)
}

pub(crate) fn paint_scaled(
    galley: &Fragment,
    width: u32,
    height: u32,
    bg: Rgba,
    images: &DecodedAssets,
    scale: f64,
) -> Result<Vec<u8>, String> {
    paint_scaled_with_masks(galley, width, height, bg, images, scale, &[])
}

pub(crate) fn paint_scaled_with_masks(
    galley: &Fragment,
    width: u32,
    height: u32,
    bg: Rgba,
    images: &DecodedAssets,
    scale: f64,
    masks: &[crate::ScreenshotMask],
) -> Result<Vec<u8>, String> {
    let (width, height) = crate::screenshot_pixel_size(width, height, scale)?;
    let mut pm =
        Pixmap::new(width, height).ok_or_else(|| format!("bad canvas {width}x{height}"))?;
    pm.fill(tiny_skia::Color::from_rgba8(bg.r, bg.g, bg.b, bg.a));
    SCALED.with(|memo| memo.borrow_mut().take());
    paint_fragment(
        &mut pm,
        galley,
        images,
        Transform::from_scale(scale as f32, scale as f32),
    );
    crate::masks::paint_png(&mut pm, masks, scale as f32);
    pm.encode_png().map_err(|e| format!("png encode: {e}"))
}

/// The `window` (x, y, width, height) of the image scaled to `size` px, memoized for
/// the render's next tile of the same image (a background layer draws one image many
/// times; miaskiewicz/turbo-dom#248). Shared, so a hit costs no copy.
type ScaledKey = (
    usize,
    turbo_dom_layout::image::Orientation,
    (u32, u32),
    (u32, u32, u32, u32),
);
thread_local! {
    static SCALED: std::cell::RefCell<Option<(ScaledKey, Option<Rc<Pixmap>>)>> =
        const { std::cell::RefCell::new(None) };
}

fn scaled(
    image: &crate::image_paint::DecodedImage,
    orientation: turbo_dom_layout::image::Orientation,
    size: (u32, u32),
    window: (u32, u32, u32, u32),
) -> Option<Rc<Pixmap>> {
    let key = (
        std::ptr::from_ref(image) as usize,
        orientation,
        size,
        window,
    );
    SCALED.with(|memo| {
        let mut memo = memo.borrow_mut();
        if let Some((_, pixmap)) = memo.as_ref().filter(|(seen, _)| *seen == key) {
            return pixmap.clone();
        }
        let pixmap = image
            .oriented(orientation)
            .scaled_window(size, window)
            .map(Rc::new);
        *memo = Some((key, pixmap.clone()));
        pixmap
    })
}

/// The part `(x, y, width, height)` of a `size` image box drawn through `place` (box
/// px to device px) that can reach a `canvas`-sized pixmap, in whole box px: the
/// painter resamples only that window, so the scratch never exceeds the canvas's
/// reach however large the layout box (a 40000px `<img>` or `background-size`;
/// round-1 review of 0.7.0, finding 4). `None` when none of it can show.
fn visible_window(
    place: Transform,
    size: (u32, u32),
    canvas: (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let device = Rect::from_xywh(0.0, 0.0, canvas.0 as f32, canvas.1 as f32)?;
    let local = PathBuilder::from_rect(device)
        .transform(place.invert()?)?
        .bounds();
    let x0 = local.left().floor().max(0.0);
    let y0 = local.top().floor().max(0.0);
    let x1 = local.right().ceil().min(size.0 as f32);
    let y1 = local.bottom().ceil().min(size.1 as f32);
    (x1 > x0 && y1 > y0).then_some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// Compose a box's CSS `transform` about its absolute origin onto the inherited
/// `xf`: a point in the box's local space maps through `matrix` (about the box's
/// `transform-origin`) and then the ancestors' transform. Descendants inherit the
/// result, so a translated carousel slide (and its content) all move together.
fn compose_transform(xf: Transform, f: &Fragment, t: &BoxTransform) -> Transform {
    let (ox, oy) = (f.x + t.origin_x, f.y + t.origin_y);
    let [a, b, c, d, e, g] = t.matrix;
    let m = Transform::from_translate(ox, oy)
        .pre_concat(Transform::from_row(a, b, c, d, e, g))
        .pre_concat(Transform::from_translate(-ox, -oy));
    xf.pre_concat(m)
}

fn paint_fragment(pm: &mut Pixmap, f: &Fragment, images: &DecodedAssets, xf: Transform) {
    // CSS `opacity` (miaskiewicz/turbo-dom#206): the box and its subtree paint into
    // one canvas-sized layer composited at that alpha (a transparency group, so
    // overlapping descendants do not compound); `opacity: 0` paints nothing.
    if f.opacity < 1.0 {
        if f.opacity > 0.0 {
            let Some((x0, y0, w, h)) = group_layer(f, xf, pm.width(), pm.height()) else {
                return;
            };
            let mut layer = Pixmap::new(w, h).expect("layer within the canvas");
            let local = Transform::from_translate(-x0 as f32, -y0 as f32).pre_concat(xf);
            paint_opaque(&mut layer, f, images, local);
            let paint = PixmapPaint {
                opacity: f.opacity,
                ..PixmapPaint::default()
            };
            pm.draw_pixmap(x0, y0, layer.as_ref(), &paint, Transform::identity(), None);
        }
        return;
    }
    paint_opaque(pm, f, images, xf);
}

/// The device-pixel rectangle `(x, y, width, height)` an opacity group's layer covers on
/// a `w x h` canvas: a conservative bound on the subtree's ink, clamped to the canvas
/// (miaskiewicz/turbo-dom#211). `None` when nothing of it can show; the whole canvas when
/// the bound does not resolve (a degenerate transform), as before.
fn group_layer(f: &Fragment, xf: Transform, w: u32, h: u32) -> Option<(i32, i32, u32, u32)> {
    let Some(ink) = ink_bounds(f, xf) else {
        return Some((0, 0, w, h));
    };
    device_span(ink, w, h)
}

/// A device-space rectangle holding every pixel `f` and its subtree can paint under
/// `xf`, or `None` when a transform does not map it to finite bounds.
fn ink_bounds(f: &Fragment, xf: Transform) -> Option<Rect> {
    let xf = match &f.content {
        FragmentContent::Box {
            transform: Some(t), ..
        } => compose_transform(xf, f, t),
        _ => xf,
    };
    let (mut l, mut t, mut r, mut b) = (f.x, f.y, f.x + f.width, f.y + f.height);
    match &f.content {
        FragmentContent::Box {
            shadow: Some(sh), ..
        } => {
            // The shadow rect (offset, spread) plus the blur's reach on each side.
            let reach = sh.spread.max(0) as f32 + 3.0 * shadow_blur(sh) + 1.0;
            let (dx, dy) = (sh.offset_x as f32, sh.offset_y as f32);
            l = l.min(l + dx - reach);
            t = t.min(t + dy - reach);
            r = r.max(r + dx + reach);
            b = b.max(b + dy + reach);
        }
        FragmentContent::TextLine {
            glyphs,
            font_size,
            vertical,
            ..
        } => {
            // Glyph outlines sit around their pens, within a couple of ems.
            let m = 2.0 * font_size;
            let frame = glyph_frame(f, *vertical);
            for g in glyphs {
                let mut pen = [tiny_skia::Point::from_xy(f.x + g.x, f.y + g.y)];
                frame.map_points(&mut pen);
                let (px, py) = (pen[0].x, pen[0].y);
                l = l.min(px - m);
                t = t.min(py - m);
                r = r.max(px + m);
                b = b.max(py + m);
            }
        }
        // Images paint at their rounded box.
        FragmentContent::Image(_) => (l, t, r, b) = (l - 1.0, t - 1.0, r + 1.0, b + 1.0),
        _ => {}
    }
    let mut ink = PathBuilder::from_rect(Rect::from_ltrb(l, t, r.max(l + 1.0), b.max(t + 1.0))?)
        .transform(xf)?
        .bounds();
    for c in &f.children {
        let cb = ink_bounds(c, xf)?;
        ink = Rect::from_ltrb(
            ink.left().min(cb.left()),
            ink.top().min(cb.top()),
            ink.right().max(cb.right()),
            ink.bottom().max(cb.bottom()),
        )?;
    }
    Some(ink)
}

/// [`paint_fragment`] for a box painted at full opacity.
fn paint_opaque(pm: &mut Pixmap, f: &Fragment, images: &DecodedAssets, xf: Transform) {
    // A CSS `transform` on this box applies to it AND its whole subtree.
    let xf = match &f.content {
        FragmentContent::Box {
            transform: Some(t), ..
        } => compose_transform(xf, f, t),
        _ => xf,
    };
    match f.clip {
        Some(clip) => paint_clipped(pm, f, clip, images, xf),
        None => paint_subtree(pm, f, images, xf),
    }
}

/// Paint a fragment whose box has a `clip: rect()` (miaskiewicz/turbo-dom#52): it and its
/// subtree paint into a layer covering the clip region's device bounds (in the box's own,
/// possibly transformed, space), masked to that region and composited source-over. An
/// empty region paints nothing.
fn paint_clipped(
    pm: &mut Pixmap,
    f: &Fragment,
    clip: ClipRect,
    images: &DecodedAssets,
    xf: Transform,
) {
    let Some(region) = Rect::from_xywh(f.x + clip.x, f.y + clip.y, clip.width, clip.height) else {
        return;
    };
    // `region` is a valid rect (positive size), so its outline always builds.
    let path = rrect_path(
        (region.x(), region.y(), region.width(), region.height()),
        &clip.radii,
    )
    .expect("the outline of a valid region");
    paint_masked(pm, &path, xf, |layer, local| {
        paint_subtree(layer, f, images, local);
    });
}

/// Run `paint` into a layer covering `path`'s device bounds (`path` in the space `xf`
/// maps from), keep only what falls inside `path`, and composite the layer onto `pm`.
/// A path whose bounds miss the canvas paints nothing.
fn paint_masked(
    pm: &mut Pixmap,
    path: &Path,
    xf: Transform,
    paint: impl FnOnce(&mut Pixmap, Transform),
) {
    let span = path
        .clone()
        .transform(xf)
        .and_then(|p| device_span(p.bounds(), pm.width(), pm.height()));
    let Some((x0, y0, w, h)) = span else {
        return;
    };
    let mut layer = Pixmap::new(w, h).expect("clip layer within the canvas");
    let mut mask = Mask::new(w, h).expect("clip mask within the canvas");
    let local = Transform::from_translate(-x0 as f32, -y0 as f32).pre_concat(xf);
    paint(&mut layer, local);
    mask.fill_path(path, FillRule::Winding, true, local);
    layer.apply_mask(&mask);
    pm.draw_pixmap(
        x0,
        y0,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

/// The device pixels `(x, y, width, height)` of a `w x h` canvas that `bounds` touches:
/// a non-empty sub-rectangle of the canvas (which tiny-skia allocates at any size an
/// existing pixmap already has), or `None` when `bounds` misses the canvas.
fn device_span(bounds: Rect, w: u32, h: u32) -> Option<(i32, i32, u32, u32)> {
    let x0 = bounds.left().floor().max(0.0) as i32;
    let y0 = bounds.top().floor().max(0.0) as i32;
    let x1 = (bounds.right().ceil() as i32).min(w as i32);
    let y1 = (bounds.bottom().ceil() as i32).min(h as i32);
    (x1 > x0 && y1 > y0).then(|| (x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// Paint `f`'s children in stacking order, clipped to its `overflow` region
/// (miaskiewicz/turbo-dom#253) when it has one: an empty region paints none of them, and
/// children all inside a square region paint directly.
fn paint_children(pm: &mut Pixmap, f: &Fragment, images: &DecodedAssets, xf: Transform) {
    let paint = |pm: &mut Pixmap, xf: Transform| {
        // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
        // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
        for &i in &f.paint_order() {
            paint_fragment(pm, &f.children[i], images, xf);
        }
    };
    let Some(clip) = f.overflow_clip.filter(|_| !f.children.is_empty()) else {
        paint(pm, xf);
        return;
    };
    let region = overflow_region(f, &clip);
    if !clip.radii.is_rounded() && f.children.iter().all(|c| within(c, region)) {
        paint(pm, xf);
        return;
    }
    if let Some(path) = rrect_path(region, &clip.radii) {
        paint_masked(pm, &path, xf, paint);
    }
}

/// `f`'s overflow clip region as an absolute rectangle.
pub(crate) fn overflow_region(f: &Fragment, clip: &OverflowClip) -> rrect::Rect {
    (f.x + clip.x, f.y + clip.y, clip.width, clip.height)
}

/// Whether `f` and its whole subtree paint inside `(x, y, w, h)` (untransformed and
/// unshadowed: anything that could paint past its own box answers `false`, a text line
/// too, since its glyphs' ink is not bounded by the line box: descenders, a small
/// `line-height`, italics; round-1 review of 0.7.0, finding 7).
pub(crate) fn within(f: &Fragment, (x, y, w, h): rrect::Rect) -> bool {
    let plain = match &f.content {
        FragmentContent::Box {
            shadow, transform, ..
        } => shadow.is_none() && transform.is_none(),
        FragmentContent::TextLine { .. } => false,
        _ => true,
    };
    plain
        && f.x >= x
        && f.y >= y
        && f.x + f.width <= x + w
        && f.y + f.height <= y + h
        && f.children.iter().all(|c| within(c, (x, y, w, h)))
}

/// Paint a fragment's own content, then its children in stacking order.
fn paint_subtree(pm: &mut Pixmap, f: &Fragment, images: &DecodedAssets, xf: Transform) {
    match &f.content {
        FragmentContent::Box {
            background,
            border,
            border_radius,
            shadow,
            gradient,
            ..
        } => {
            // An outer shadow paints behind the box (before its fill/border) so the
            // card sits on it; an inset one paints over the background, under the border.
            if let Some(sh) = shadow.filter(|s| !s.inset && s.color.a > 0) {
                paint_box_shadow(pm, f, border_radius, &sh, xf);
            }
            if border_radius.is_rounded() {
                if let Some(bg) = background {
                    fill_rrect(pm, (f.x, f.y, f.width, f.height), border_radius, *bg, xf);
                }
                if let Some(g) = gradient {
                    paint_gradient(pm, f, border_radius, g, xf);
                }
                paint_inset_shadow(pm, f, border, border_radius, shadow.as_ref(), xf);
                paint_round_border(pm, f, border, border_radius, xf);
            } else {
                if let Some(bg) = background {
                    fill_rect(pm, f.x, f.y, f.width, f.height, *bg, xf);
                }
                if let Some(g) = gradient {
                    paint_gradient(pm, f, &BorderRadii::ZERO, g, xf);
                }
                paint_inset_shadow(pm, f, border, &BorderRadii::ZERO, shadow.as_ref(), xf);
                paint_border(pm, f, border, xf);
            }
        }
        FragmentContent::TextLine {
            glyphs,
            face,
            font_size,
            color,
            vertical,
            upright,
            ..
        } => paint_text(
            pm,
            (f, *vertical && *upright),
            glyphs,
            face,
            (*font_size, *color),
            xf.pre_concat(glyph_frame(f, *vertical)),
        ),
        FragmentContent::Image(placement) => paint_image(pm, f, placement, images, xf),
        FragmentContent::Tiles(pattern) => paint_tiles(pm, f, pattern, images, xf),
        FragmentContent::ColumnRule { style, color } => {
            paint_column_rule(pm, f, *style, *color, xf);
        }
        FragmentContent::Marker(_) => {}
    }
    paint_children(pm, f, images, xf);
}

/// Paint a multicol `column-rule` fragment ([`FragmentContent::ColumnRule`];
/// miaskiewicz/turbo-dom#398 PR-D) centred in its box: solid fills the rect,
/// dotted stamps circles of diameter `width` on period `2 * width`, dashed
/// stamps dashes `2 * width` long on period `3 * width`, double paints two
/// bands each `floor(width / 3)` thick at the long edges.
fn paint_column_rule(
    pm: &mut Pixmap,
    f: &Fragment,
    style: ColumnRuleStyle,
    color: Rgba,
    xf: Transform,
) {
    // Geometry admission belongs to each style's primitive; alpha admission
    // also avoids walking a long, wholly invisible dotted or dashed rule.
    if color.a == 0 {
        return;
    }
    match style {
        ColumnRuleStyle::Solid => fill_rect(pm, f.x, f.y, f.width, f.height, color, xf),
        ColumnRuleStyle::Dotted => paint_dotted_rule(pm, f, color, xf),
        ColumnRuleStyle::Dashed => paint_dashed_rule(pm, f, color, xf),
        ColumnRuleStyle::Double => paint_double_rule(pm, f, color, xf),
        ColumnRuleStyle::Groove
        | ColumnRuleStyle::Ridge
        | ColumnRuleStyle::Inset
        | ColumnRuleStyle::Outset => paint_shaded_rule(pm, f, style, color, xf),
    }
}

/// Stamp circular dots of diameter `f.width` down the rule's centerline, period
/// `2 * width` (Blink's `kDottedStroke` with cap-round). The painted diameter
/// is floored to 1 px to match Chrome 156's `kDottedStroke`, which rounds the
/// effective stroke to >= 1 device pixel so a sub-pixel `column-rule-width`
/// (e.g. `0.5px`) still produces visible dots instead of vanishing to
/// antialiased transparent; the rule is kept centred on `f.x + f.width / 2`
/// so gap allocation still reserves the declared width (#424).
///
/// When the declared width is < 1 px, the circle rasterization of a radius-0.5
/// disk spreads anti-aliased across two half-pixel columns (an ~20 % coverage
/// stamp that reads as grey-cyan, not black). Chrome's painter instead goes
/// through `GraphicsContext::StrokePath` with [`LineCap::Round`] and dash
/// `[0, 2 * stroke_width]`, which pixel-snaps the dot column and paints pure
/// black at the rule centre. We mirror that path for the floored case so the
/// probe at the rule centre reads `rgb(0, 0, 0)` like Chrome.
fn paint_dotted_rule(pm: &mut Pixmap, f: &Fragment, color: Rgba, xf: Transform) {
    // Guard `f.width <= 0.0` BEFORE flooring: a zero- or negative-width
    // fragment must paint nothing, and `max(1.0)` would otherwise coerce it
    // into the sub-pixel fallback below. (The outer `paint_column_rule`
    // already guards, but the helper's direct contract keeps the loop
    // protection pinned.) R1 M1: `is_finite()` also rejects `NaN` and
    // `Infinity`, which comparisons never catch.
    if !f.width.is_finite() || !f.height.is_finite() || f.width <= 0.0 {
        return;
    }
    let diameter = f.width.max(1.0);
    let cx = f.x + f.width / 2.0;
    let period = 2.0 * diameter;
    if f.width < 1.0 {
        paint_dotted_rule_stroked(pm, cx, f.y, f.height, diameter, color, xf);
        return;
    }
    let radius = diameter / 2.0;
    let (top, xf) = column_rule_space(f.y, period, xf);
    let mut cy = top + radius;
    let end = top + f.height;
    while cy - radius < end {
        if let Some(path) = circle_path(cx, cy, radius) {
            pm.fill_path(&path, &solid(color), FillRule::Winding, xf, None);
        }
        let Some(next) = next_column_rule_step(cy, period) else {
            break;
        };
        cy = next;
    }
}

/// Dotted-rule fallback for sub-pixel declared widths (`column-rule-width` < 1
/// px): Blink clamps `kDottedStroke`'s effective thickness to >= 1 device
/// pixel, then paints `GraphicsContext::DrawLineForBoxSide`'s dotted pattern
/// as pixel-aligned 1-px squares at the rule's centre column — a leading cap
/// at `y_top`, then one square per 2 px of rule length starting at `y_top +
/// 1`, which together matches Chrome 156's observed pattern (first two rows
/// solid, then one square on / one off; #424). An anti-aliased radius-0.5
/// disk would spread to adjacent columns at ~20 % coverage each, so pixel
/// snapping is load-bearing.
fn paint_dotted_rule_stroked(
    pm: &mut Pixmap,
    cx: f32,
    y_top: f32,
    height: f32,
    diameter: f32,
    color: Rgba,
    xf: Transform,
) {
    paint_pixel_dots(pm, cx.floor(), (y_top, height), diameter, color, xf);
}

fn paint_pixel_dots(
    pm: &mut Pixmap,
    px_left: f32,
    (y_top, height): (f32, f32),
    diameter: f32,
    color: Rgba,
    xf: Transform,
) {
    let (y_top, xf) = column_rule_space(y_top, 2.0 * diameter, xf);
    // Use the rasterizer's actual rectangle admission once for the whole
    // series. Localize first: a large finite origin can swallow a short height.
    let Some(bounds) = Rect::from_xywh(px_left, y_top, diameter, height) else {
        return;
    };
    let end = bounds.bottom();
    // Leading cap: one px above the first dash (Blink's round line-cap).
    // R1 L3: clamp the cap height to `(end - y_top).min(diameter)` so a
    // fragment shorter than `diameter` (e.g. `height = 0.3`) does not
    // leak past `end` — Blink's round cap collapses with the rule
    // height, and the probe on a 0.3 px rule never paints a full pixel.
    if y_top < end {
        let cap_h = (end - y_top).min(diameter);
        fill_rect(pm, px_left, y_top, diameter, cap_h, color, xf);
    }
    // Dash series: `[dash=diameter, gap=diameter]` starting at `y_top + 1`.
    let mut y = y_top + diameter;
    while y < end {
        let dash_h = (end - y).min(diameter);
        fill_rect(pm, px_left, y, diameter, dash_h, color, xf);
        let Some(next) = next_column_rule_step(y, 2.0 * diameter) else {
            break;
        };
        y = next;
    }
}

/// Kappa: the cubic control offset that approximates a quarter-circle to
/// within ~0.0003 units.
const CIRCLE_KAPPA: f32 = 0.552_284_8;

/// Build a tiny-skia path for a circle of radius `r` at `(cx, cy)` as four cubic
/// Béziers. `None` when `r < 0.5` pixel (tiny-skia ignores a degenerate path).
fn circle_path(cx: f32, cy: f32, r: f32) -> Option<Path> {
    if r <= 0.0 {
        return None;
    }
    let k = r * CIRCLE_KAPPA;
    let mut p = PathBuilder::new();
    p.move_to(cx + r, cy);
    p.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
    p.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
    p.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
    p.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
    p.close();
    p.finish()
}

/// Stamp dashes of length `2 * width` down the rule, period `3 * width` (Blink's
/// `kDashedStroke`: a `width` gap between each dash).
fn paint_dashed_rule(pm: &mut Pixmap, f: &Fragment, color: Rgba, xf: Transform) {
    // R1 M1: reject non-finite sizes — `period <= 0.0` is false for
    // `NaN` and `Infinity`, which would otherwise step the loop at
    // undefined coordinates (`dash_top += NaN` never terminates
    // `dash_top < end`).
    if !f.width.is_finite() || !f.height.is_finite() {
        return;
    }
    let width = f.width;
    let dash = 2.0 * width;
    let gap = width;
    let period = dash + gap;
    if period <= 0.0 {
        return;
    }
    let (top, xf) = column_rule_space(f.y, period, xf);
    let end = top + f.height;
    let mut dash_top = top;
    while dash_top < end {
        let dash_h = dash.min(end - dash_top);
        if dash_h > 0.0 {
            fill_rect(pm, f.x, dash_top, width, dash_h, color, xf);
        }
        let Some(next) = next_column_rule_step(dash_top, period) else {
            break;
        };
        dash_top = next;
    }
}

/// A rule cursor must move strictly forward; a swallowed period cannot spin.
fn next_column_rule_step(cursor: f32, period: f32) -> Option<f32> {
    let next = cursor + period;
    (next > cursor).then_some(next)
}

/// Keep ordinary coordinates unchanged, but iterate locally when a finite
/// origin swallows the rule's period. Composing that origin into the transform
/// preserves the full visible pattern when an ancestor translates it onscreen.
fn column_rule_space(y: f32, period: f32, xf: Transform) -> (f32, Transform) {
    if !y.is_finite() || y + period > y {
        (y, xf)
    } else {
        (0.0, xf.pre_concat(Transform::from_translate(0.0, y)))
    }
}

/// Paint two bands at the long edges, each `floor(width / 3)` thick, matching
/// the CSS `double` border style (CSS Backgrounds 3 §4.2). Below 3 px the
/// painter falls back to one solid band so a thin double rule still shows.
fn paint_double_rule(pm: &mut Pixmap, f: &Fragment, color: Rgba, xf: Transform) {
    // R1 M1 + L1: reject `NaN` / `Infinity` — `f.width < 3.0` is false
    // for either, so without this guard an `Infinity` width would run
    // the two-band path with Inf band widths, and a NaN would make
    // `f.width - band = NaN`.
    if !f.width.is_finite() || !f.height.is_finite() {
        return;
    }
    // R1 L2: the thin-double fallback below handles every `f.width < 3.0`
    // case, so from here on `f.width >= 3.0`, which forces
    // `(f.width / 3.0).floor() >= 1.0` — the earlier `.max(1.0)` was a
    // dead branch unreachable after the short-circuit. Computing `band`
    // AFTER the fallback also avoids a wasted divide + floor on the
    // thin path. Proof: f.width >= 3.0 && floor(x) <= x => floor(f.width / 3.0) >= 1.0.
    if f.width < 3.0 {
        fill_rect(pm, f.x, f.y, f.width, f.height, color, xf);
        return;
    }
    let band = (f.width / 3.0).floor();
    fill_rect(pm, f.x, f.y, band, f.height, color, xf);
    fill_rect(pm, f.x + f.width - band, f.y, band, f.height, color, xf);
}

/// Paint a 3-D column rule (`groove` / `ridge` / `inset` / `outset`;
/// miaskiewicz/turbo-dom#419) as two halves of the rule's inline axis,
/// shaded through [`border_style_colour`]. Chrome 156 splits the rule at
/// its inline midpoint: `groove` and `outset` paint the LEFT half with the
/// darkened shade and the RIGHT half with the lightened one; `ridge` and
/// `inset` swap the two (the rule is vertical, so the two "halves" are
/// vertical bands running the rule's full height).
fn paint_shaded_rule(
    pm: &mut Pixmap,
    f: &Fragment,
    style: ColumnRuleStyle,
    color: Rgba,
    xf: Transform,
) {
    // The outer `paint_column_rule` already rejects non-finite / non-positive
    // sizes; the helper's direct contract keeps a future caller from feeding
    // it NaN / Infinity (`is_finite` rejects both) or a zero-area fragment
    // (nothing to paint — `fill_rect` would early-return anyway).
    if !f.width.is_finite() || !f.height.is_finite() || f.width <= 0.0 || f.height <= 0.0 {
        return;
    }
    // Chrome 156: left band = darker shade for `groove` / `outset`, lighter
    // shade for `ridge` / `inset`; right band is the other. Split at the
    // rule's inline midpoint (`f.width / 2.0`); the right band takes the
    // trailing pixels so that an odd-pixel rule's extra column lands on the
    // right, mirroring Chrome's floor/ceil split.
    let left_dark = matches!(style, ColumnRuleStyle::Groove | ColumnRuleStyle::Outset);
    let left_color = border_style_colour(color, left_dark);
    let right_color = border_style_colour(color, !left_dark);
    let half = f.width / 2.0;
    fill_rect(pm, f.x, f.y, half, f.height, left_color, xf);
    fill_rect(
        pm,
        f.x + half,
        f.y,
        f.width - half,
        f.height,
        right_color,
        xf,
    );
}

/// Paint a background layer's tiles ([`TilePattern`]; 0.7.0 round-2 review, finding 1)
/// over the part of its box the canvas shows: one [`TileCell`] (a whole-pixel super-period
/// of the snapped tiles) drawn into a pixmap and repeated by a pattern shader, else (one
/// tile, or a cell larger than the canvas: big tiles, few of them) tile by tile.
fn paint_tiles(
    pm: &mut Pixmap,
    f: &Fragment,
    pattern: &TilePattern,
    images: &DecodedAssets,
    xf: Transform,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let Some(region) = visible_region(f, xf, (pm.width(), pm.height())) else {
        return;
    };
    let mut tiles = pattern.tiles(region);
    let Some(first) = tiles.next() else {
        return;
    };
    let canvas = u64::from(pm.width()) * u64::from(pm.height());
    if tiles.next().is_some() {
        if repeats_tiles(pattern) {
            if let Some(texture) = tile_texture(pattern, images, cell_scale(xf), canvas) {
                fill_texture(pm, &texture, pattern, (region, first), xf);
                return;
            }
        } else {
            let cell = TileCell::new(pattern, first);
            if let Some(tile) = cell_pixmap(pattern, &cell, images, (xf, canvas)) {
                fill_pattern(pm, &tile, (region, first, &cell), xf);
                return;
            }
        }
    }
    for (x, y, w, h) in pattern.tiles(region) {
        let tile = Fragment::new(f.node_id, x, y, w, h, (*pattern.tile).clone());
        paint_subtree(pm, &tile, images, xf);
    }
}

/// Whether every repeated axis of `pattern` steps by exactly its tile (`repeat`,
/// `round`, not `space`): Chrome then draws one tile image repeated every tile size,
/// fractional or not, bilinear-filtered, so the pixels a tile edge crosses blend
/// (miaskiewicz/turbo-dom#270).
pub(crate) fn repeats_tiles(pattern: &TilePattern) -> bool {
    let axis = |step: f32, tile: f32| step == 0.0 || step == tile;
    axis(pattern.step_x, pattern.tile_width) && axis(pattern.step_y, pattern.tile_height)
}

/// One tile's pixels, as Chrome rasterizes a background tile: `scale` px per CSS px
/// rounded up to whole pixels (a 2.5px gradient tile at 1x: 3px), an image at most at
/// its own (oriented) resolution, stencilled when it is a mask. `None` when the image
/// is missing or did not decode (painted tile by tile: the placeholder), or the tile
/// would be larger than `canvas` pixels.
pub(crate) fn tile_texture(
    pattern: &TilePattern,
    images: &DecodedAssets,
    scale: f32,
    canvas: u64,
) -> Option<Pixmap> {
    let (tw, th) = (pattern.tile_width, pattern.tile_height);
    let size = ((tw * scale).ceil() as u32, (th * scale).ceil() as u32);
    if let FragmentContent::Image(placement) = &*pattern.tile {
        let image = images.get(&placement.name)?;
        let (iw, ih) = {
            let oriented = image.oriented(placement.orientation);
            (oriented.width, oriented.height)
        };
        let size = (size.0.min(iw), size.1.min(ih));
        if u64::from(size.0) * u64::from(size.1) > canvas {
            return None;
        }
        let pixels = scaled(image, placement.orientation, size, (0, 0, size.0, size.1))?;
        return Some(match placement.tint {
            Some(tint) => tint_pixmap(&pixels, tint),
            None => (*pixels).clone(),
        });
    }
    if u64::from(size.0) * u64::from(size.1) > canvas {
        return None;
    }
    let mut texture = Pixmap::new(size.0, size.1)?;
    let content = Fragment::new(
        turbo_dom_layout::NodeId(0),
        0.0,
        0.0,
        tw,
        th,
        (*pattern.tile).clone(),
    );
    let at = Transform::from_scale(size.0 as f32 / tw, size.1 as f32 / th);
    paint_subtree(&mut texture, &content, images, at);
    Some(texture)
}

/// The least length along a repeated axis of a cell [`texture_cell`] draws.
const MIN_EMBEDDED_CELL: f32 = 64.0;

/// One whole-pixel cell of fractional tiles repeated every tile size, at 1x, as the PNG
/// painter fills it ([`tile_texture`] repeated and bilinear-filtered: the pixels a tile
/// edge crosses blend), with its corner: whole px at or before the pattern's origin
/// along a repeated axis (whole [`crate::tile_cell::cell_length`]s, at least
/// [`MIN_EMBEDDED_CELL`] long), the `first` tile's
/// snapped extent along one that does not repeat. `None` as for [`tile_texture`].
pub(crate) fn texture_cell(
    pattern: &TilePattern,
    images: &DecodedAssets,
    first: rrect::Rect,
    cap: u64,
) -> Option<(Pixmap, (f32, f32))> {
    let texture = tile_texture(pattern, images, 1.0, cap)?;
    let along = |step: f32, origin: f32, (at, size): (f32, f32)| {
        if step > 0.0 {
            // Whole cells up to at least MIN_EMBEDDED_CELL px: a viewer drawing the
            // pattern much smaller than 1x clips each cell on whole device pixels.
            let cell = crate::tile_cell::cell_length(step);
            (
                origin.floor(),
                cell * (MIN_EMBEDDED_CELL / cell).ceil().max(1.0),
            )
        } else {
            (at, size)
        }
    };
    let (x, w) = along(pattern.step_x, pattern.x, (first.0, first.2));
    let (y, h) = along(pattern.step_y, pattern.y, (first.1, first.3));
    let mut cell = Pixmap::new(w as u32, h as u32)?;
    let region = (x, y, w, h);
    fill_texture(
        &mut cell,
        &texture,
        pattern,
        (region, region),
        Transform::from_translate(-x, -y),
    );
    Some((cell, (x, y)))
}

/// Fill `region` with `texture` repeated every tile from the pattern's (unsnapped)
/// origin, bilinear-filtered (only the `first` tile's row or column on an axis that
/// does not repeat); the region's edges are anti-aliased under a scale, rotation or
/// skew.
fn fill_texture(
    pm: &mut Pixmap,
    texture: &Pixmap,
    pattern: &TilePattern,
    (region, first): (rrect::Rect, rrect::Rect),
    xf: Transform,
) {
    let span = |repeats: bool, (from, len): (f32, f32), (at, size): (f32, f32)| {
        if repeats {
            (from, len)
        } else {
            (at, size)
        }
    };
    let (x, w) = span(
        pattern.step_x > 0.0,
        (region.0, region.2),
        (first.0, first.2),
    );
    let (y, h) = span(
        pattern.step_y > 0.0,
        (region.1, region.3),
        (first.1, first.3),
    );
    let to_layer = Transform::from_translate(pattern.x, pattern.y).pre_scale(
        pattern.tile_width / texture.width() as f32,
        pattern.tile_height / texture.height() as f32,
    );
    let paint = Paint {
        shader: tiny_skia::Pattern::new(
            texture.as_ref(),
            SpreadMode::Repeat,
            tiny_skia::FilterQuality::Bilinear,
            1.0,
            to_layer,
        ),
        anti_alias: !only_translates(xf),
        ..Paint::default()
    };
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        pm.fill_rect(rect, &paint, xf, None);
    }
}

/// The part of `f`'s box a `canvas`-sized pixmap can show through `xf`, in `f`'s space
/// (whole px around it), or `None` when none of it can.
fn visible_region(f: &Fragment, xf: Transform, canvas: (u32, u32)) -> Option<rrect::Rect> {
    let device = Rect::from_xywh(0.0, 0.0, canvas.0 as f32, canvas.1 as f32)?;
    let local = PathBuilder::from_rect(device)
        .transform(xf.invert()?)?
        .bounds();
    let x0 = local.left().floor().max(f.x);
    let y0 = local.top().floor().max(f.y);
    let x1 = local.right().ceil().min(f.x + f.width);
    let y1 = local.bottom().ceil().min(f.y + f.height);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// Whether `xf` only moves (no scale, rotation or skew): a tile pattern then paints its
/// pixels 1:1, unfiltered (0.7.0 round-3 review, finding 3).
fn only_translates(xf: Transform) -> bool {
    xf.sx == 1.0 && xf.sy == 1.0 && xf.kx == 0.0 && xf.ky == 0.0
}

/// The most device pixels per CSS px a cell is drawn at.
const MAX_CELL_SCALE: f32 = 16.0;

/// How many pixmap pixels per CSS px a cell is drawn at under `xf`: the whole number at
/// or above the device scale (so the tiles' snapped edges stay on pixmap pixels and the
/// pattern shader only ever shrinks it), 1 when `xf` only moves or shrinks.
fn cell_scale(xf: Transform) -> f32 {
    let x = xf.sx.hypot(xf.ky);
    let y = xf.kx.hypot(xf.sy);
    x.max(y).ceil().clamp(1.0, MAX_CELL_SCALE)
}

/// One cell of the pattern drawn at `cell_scale` (or 1x when that would not fit the
/// canvas's pixel count): its tiles' content at their snapped places. `None` when even a
/// 1x cell would be larger than the canvas (big tiles: walked).
fn cell_pixmap(
    pattern: &TilePattern,
    cell: &TileCell,
    images: &DecodedAssets,
    (xf, canvas): (Transform, u64),
) -> Option<Pixmap> {
    let (w, h) = cell.size;
    let pixels = |s: f32| ((w * s) as u64).saturating_mul((h * s) as u64);
    let scale = Some(cell_scale(xf))
        .filter(|&s| pixels(s) <= canvas)
        .or_else(|| (pixels(1.0) <= canvas).then_some(1.0))?;
    let mut pixmap = Pixmap::new((w * scale) as u32, (h * scale) as u32)?;
    let at = Transform::from_scale(scale, scale);
    for (x, y, tw, th) in cell.tiles(pattern) {
        let content = Fragment::new(
            turbo_dom_layout::NodeId(0),
            x,
            y,
            tw,
            th,
            (*pattern.tile).clone(),
        );
        paint_subtree(&mut pixmap, &content, images, at);
    }
    Some(pixmap)
}

/// Fill `region` with the `cell` pixmap `tile` repeated from the cell's corner (only the
/// `first` tile's row or column on an axis that does not repeat). Under a transform that
/// only moves, the cell's pixels land 1:1 (nearest, no anti-aliasing); under a scale,
/// rotation or skew they are filtered and the region's edges anti-aliased, as Chrome
/// draws a scaled or rotated background (0.7.0 round-3 review, finding 3).
fn fill_pattern(
    pm: &mut Pixmap,
    tile: &Pixmap,
    (region, first, cell): (rrect::Rect, rrect::Rect, &TileCell),
    xf: Transform,
) {
    let span = |repeats: bool, (from, len): (f32, f32), (at, size): (f32, f32)| {
        if repeats {
            (from, len)
        } else {
            (at, size)
        }
    };
    let (x, w) = span(cell.repeats.0, (region.0, region.2), (first.0, first.2));
    let (y, h) = span(cell.repeats.1, (region.1, region.3), (first.1, first.3));
    let smooth = !only_translates(xf);
    let quality = if smooth {
        tiny_skia::FilterQuality::Bilinear
    } else {
        tiny_skia::FilterQuality::Nearest
    };
    let to_layer = Transform::from_translate(cell.origin.0, cell.origin.1).pre_scale(
        cell.size.0 / tile.width() as f32,
        cell.size.1 / tile.height() as f32,
    );
    let paint = Paint {
        shader: tiny_skia::Pattern::new(tile.as_ref(), SpreadMode::Repeat, quality, 1.0, to_layer),
        anti_alias: smooth,
        ..Paint::default()
    };
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        pm.fill_rect(rect, &paint, xf, None);
    }
}

/// Draw an image into its layout box: resample the decoded pixels to the box (one
/// kernel, [`DecodedImage::scaled_window`](crate::image_paint::DecodedImage::scaled_window),
/// over the part of the box the canvas can show) and blit them. An image
/// laid out at its size whose pixels did not decode paints nothing, as in Chrome
/// (miaskiewicz/turbo-dom#107); the neutral placeholder stands for an image the caller did
/// not supply or this build cannot decode (`DecodedImage::placeholder`).
/// A `mask-image` placement (`tint` set) is stencilled: the source's alpha selects
/// where the tint colour paints (Wikipedia's monochrome SVG UI glyphs).
fn paint_image(
    pm: &mut Pixmap,
    f: &Fragment,
    placement: &turbo_dom_layout::ImagePlacement,
    images: &DecodedAssets,
    xf: Transform,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let (w, h) = (f.width.round() as u32, f.height.round() as u32);
    // Fold the box's top-left into the transform so a transformed image (a rotated
    // thumbnail, a translated slide) maps correctly.
    let place = xf.pre_concat(Transform::from_translate(f.x.round(), f.y.round()));
    let Some(decoded) = images.get(&placement.name) else {
        if placement.tint.is_none() {
            fill_rect(pm, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER, xf);
        }
        return;
    };
    // Under a scale the source is resampled at (at least) the device resolution, a whole
    // number of pixels per box px, and drawn back down through `1/scale`: Chrome samples
    // a scaled image's source, not a 1x copy (miaskiewicz/turbo-dom#269).
    let scale = cell_scale(xf);
    let place = place.pre_scale(scale.recip(), scale.recip());
    let size = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
    let Some(window) = visible_window(place, size, (pm.width(), pm.height())) else {
        return;
    };
    match (
        scaled(decoded, placement.orientation, size, window),
        placement.tint,
    ) {
        (Some(src), tint) => {
            let stencilled = tint.map(|tint| tint_pixmap(&src, tint));
            let quality = if only_translates(xf) {
                tiny_skia::FilterQuality::Nearest
            } else {
                tiny_skia::FilterQuality::Bilinear
            };
            let paint = PixmapPaint {
                quality,
                ..PixmapPaint::default()
            };
            let src = stencilled.as_ref().unwrap_or(&src);
            let at = Transform::from_translate(window.0 as f32, window.1 as f32);
            pm.draw_pixmap(0, 0, src.as_ref(), &paint, place.pre_concat(at), None);
        }
        // A missing mask paints nothing (no placeholder box for a glyph), nor does an
        // image whose pixels did not decode; a missing real image shows the neutral
        // placeholder.
        (None, Some(_)) => {}
        (None, None) if !decoded.placeholder => {}
        (None, None) => fill_rect(pm, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER, xf),
    }
}

/// Stencil `tint` through a mask pixmap: each output pixel is `tint` with its alpha
/// scaled by the source pixel's alpha, premultiplied. Where the mask is transparent
/// nothing paints; where opaque the full tint shows.
fn tint_pixmap(src: &Pixmap, tint: Rgba) -> Pixmap {
    let mut out = Pixmap::new(src.width(), src.height()).expect("mask pixmap");
    let scale = |c: u8, a: u8| ((u32::from(c) * u32::from(a)) / 255) as u8;
    for (o, s) in out.pixels_mut().iter_mut().zip(src.pixels()) {
        let a = scale(s.alpha(), tint.a);
        *o = tiny_skia::PremultipliedColorU8::from_rgba(
            scale(tint.r, a),
            scale(tint.g, a),
            scale(tint.b, a),
            a,
        )
        .expect("premultiplied");
    }
    out
}

fn solid(c: Rgba) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(c.r, c.g, c.b, c.a);
    paint.anti_alias = true;
    paint
}

fn fill_rect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Rgba, xf: Transform) {
    if w <= 0.0 || h <= 0.0 || c.a == 0 {
        return;
    }
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        // The optimized AA rectangle scan assumes a full interior pixel between
        // fractional edges. A narrow border can cover two partial columns with
        // none between them; use the general path scan for that valid geometry.
        if xf.is_identity() && rect.width() < 2.0 {
            pm.fill_path(
                &PathBuilder::from_rect(rect),
                &solid(c),
                FillRule::Winding,
                xf,
                None,
            );
        } else {
            pm.fill_rect(rect, &solid(c), xf, None);
        }
    }
}

/// Paint each border side as a filled edge band (no line styles — width + colour only).
/// Sides that differ in colour ([`rrect::mixed_colours`]) paint the full-height left and
/// right bands first, then the top and bottom ones mitred on the corner diagonals.
fn border_side(s: BorderSide) -> Option<(f32, Rgba)> {
    s.color
        .filter(|_| s.width > 0)
        .map(|c| (f32::from(s.width), c))
}

fn paint_mitred_bands(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, xf: Transform) {
    for (top, s) in [(true, &b.top), (false, &b.bottom)] {
        let Some((_, c)) = border_side(*s) else {
            continue;
        };
        let band = rrect::mitred_band((f.x, f.y, f.width, f.height), rrect::widths(b), top);
        let mut path = PathBuilder::new();
        trace(&mut path, &rrect::polygon(&band));
        if let Some(path) = path.finish() {
            pm.fill_path(&path, &solid(c), FillRule::Winding, xf, None);
        }
    }
}

fn paint_border(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, xf: Transform) {
    if !fragment_finite_positive(f) {
        return;
    }
    let mitred = rrect::mixed_colours(b);
    if !mitred {
        if let Some((w, c)) = border_side(b.top) {
            fill_rect(pm, f.x, f.y, f.width, w, c, xf);
        }
        if let Some((w, c)) = border_side(b.bottom) {
            fill_rect(pm, f.x, f.y + f.height - w, f.width, w, c, xf);
        }
    }
    if let Some((w, c)) = border_side(b.left) {
        fill_rect(pm, f.x, f.y, w, f.height, c, xf);
    }
    if let Some((w, c)) = border_side(b.right) {
        fill_rect(pm, f.x + f.width - w, f.y, w, f.height, c, xf);
    }
    if mitred {
        paint_mitred_bands(pm, f, b, xf);
    }
}

/// Append outline steps to a path.
fn trace(p: &mut PathBuilder, segs: &[Seg]) {
    for seg in segs {
        match *seg {
            Seg::Move(x, y) => p.move_to(x, y),
            Seg::Line(x, y) => p.line_to(x, y),
            Seg::Cubic(a, b, c, d, x, y) => p.cubic_to(a, b, c, d, x, y),
            Seg::Close => p.close(),
        }
    }
}

/// A rectangle with elliptical corner radii (miaskiewicz/turbo-dom#253) as a path.
pub(crate) fn rrect_path(rect: rrect::Rect, radii: &BorderRadii) -> Option<Path> {
    let mut p = PathBuilder::new();
    trace(&mut p, &rrect::outline(rect, radii));
    p.finish()
}

/// Fill a rectangle with elliptical corner radii.
fn fill_rrect(pm: &mut Pixmap, rect: rrect::Rect, radii: &BorderRadii, c: Rgba, xf: Transform) {
    if rect.2 <= 0.0 || rect.3 <= 0.0 || c.a == 0 {
        return;
    }
    if let Some(path) = rrect_path(rect, radii) {
        pm.fill_path(&path, &solid(c), FillRule::Winding, xf, None);
    }
}

/// Fill a rounded rectangle with one circular radius `r`.
#[cfg(test)]
#[allow(clippy::too_many_arguments)] // geometry + colour + the inherited transform
fn fill_round_rect(
    pm: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    c: Rgba,
    xf: Transform,
) {
    fill_rrect(pm, (x, y, w, h), &BorderRadii::uniform(r.max(0.0)), c, xf);
}

/// Paint an outer `box-shadow` behind the box: stamp its rounded-rect silhouette
/// (the border box, offset by `(offset_x, offset_y)` and grown by `spread`) in the
/// shadow colour onto a scratch pixmap, box-blur it by `blur`, and composite it
/// under the box. A `blur` of 0 fills the silhouette directly. This is what makes
/// an overlay card/modal read as raised chrome rather than a flat rectangle.
fn paint_box_shadow(
    pm: &mut Pixmap,
    f: &Fragment,
    border_radius: &BorderRadii,
    sh: &BoxShadow,
    xf: Transform,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let spread = sh.spread as f32;
    let sx = f.x + sh.offset_x as f32 - spread;
    let sy = f.y + sh.offset_y as f32 - spread;
    let sw = f.width + 2.0 * spread;
    let sh_h = f.height + 2.0 * spread;
    if sw < 0.5 || sh_h < 0.5 {
        return;
    }
    let radii = rrect::shadow_radii(border_radius, spread);
    let blur = shadow_blur(sh);
    if blur < 0.5 {
        fill_rrect(pm, (sx, sy, sw, sh_h), &radii, sh.color, xf);
        return;
    }
    let silhouette = Silhouette {
        x: sx,
        y: sy,
        width: sw,
        height: sh_h,
        radii,
        color: sh.color,
    };
    blurred_shadow(pm, &silhouette, blur, xf, SHADOW_TILE, MAX_SHADOW_SCRATCH);
}

/// Paint an inset `box-shadow` (miaskiewicz/turbo-dom#192): the padding box in the shadow
/// colour with a hole cut out of it — the padding box offset by `(offset_x, offset_y)`,
/// shrunk by `spread` (its radius shrunk with it) and blurred by `blur`. Like Chrome it
/// paints over the background and under the border, never outside the padding box.
fn paint_inset_shadow(
    pm: &mut Pixmap,
    f: &Fragment,
    border: &BorderEdges,
    border_radius: &BorderRadii,
    shadow: Option<&BoxShadow>,
    xf: Transform,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let Some(sh) = shadow.filter(|s| s.inset && s.color.a > 0) else {
        return;
    };
    let ((px, py, pw, ph), pr) = rrect::padding_box(f, border, border_radius);
    // The shadow never paints outside the padding box, so its scratch layers cover only
    // the padding box's device bounds on the canvas (round-1 review of 0.7.0, finding 5:
    // they were canvas-sized, two per shadow).
    let Some((x0, y0, w, h)) = rrect_path((px, py, pw, ph), &pr)
        .and_then(|p| p.transform(xf))
        .and_then(|p| device_span(p.bounds(), pm.width(), pm.height()))
    else {
        return;
    };
    let xf = Transform::from_translate(-x0 as f32, -y0 as f32).pre_concat(xf);
    let mut layer = Pixmap::new(w, h).expect("a layer within the canvas");
    fill_rrect(&mut layer, (px, py, pw, ph), &pr, sh.color, xf);
    let spread = sh.spread as f32;
    let hole = Silhouette {
        x: px + sh.offset_x as f32 + spread,
        y: py + sh.offset_y as f32 + spread,
        width: pw - 2.0 * spread,
        height: ph - 2.0 * spread,
        radii: pr.inset(spread, spread, spread, spread),
        color: Rgba { a: 255, ..sh.color },
    };
    if hole.width >= 0.5 && hole.height >= 0.5 {
        let mut cut = Pixmap::new(w, h).expect("a layer within the canvas");
        let blur = shadow_blur(sh);
        if blur < 0.5 {
            fill_rrect(
                &mut cut,
                (hole.x, hole.y, hole.width, hole.height),
                &hole.radii,
                hole.color,
                xf,
            );
        } else {
            blurred_shadow(&mut cut, &hole, blur, xf, SHADOW_TILE, MAX_SHADOW_SCRATCH);
        }
        let out = PixmapPaint {
            blend_mode: tiny_skia::BlendMode::DestinationOut,
            ..PixmapPaint::default()
        };
        layer.draw_pixmap(0, 0, cut.as_ref(), &out, Transform::identity(), None);
    }
    pm.draw_pixmap(
        x0,
        y0,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

/// The blur radius Chrome paints a shadow with: every `blur` from 256px up paints the same
/// pixels in Chrome 156 (measured; miaskiewicz/turbo-dom#95).
pub(crate) const MAX_SHADOW_BLUR: u32 = 256;

/// A shadow's blur radius in px, capped like Chrome's ([`MAX_SHADOW_BLUR`]).
pub(crate) fn shadow_blur(sh: &BoxShadow) -> f32 {
    sh.blur.min(MAX_SHADOW_BLUR) as f32
}

/// The largest side (px) of one blurred-shadow scratch tile, before the blur's reach is
/// added on each side.
const SHADOW_TILE: i32 = 2048;

/// The largest side (px) of the one scratch a rotated or skewed shadow's visible part is
/// blurred in before it is tiled too.
const MAX_SHADOW_SCRATCH: i32 = 8192;

/// A shadow's rounded-rect silhouette, in the box's (local) space.
struct Silhouette {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radii: BorderRadii,
    color: Rgba,
}

/// Stamp `s` onto scratch pixmaps padded by the blur radius (so the feathered edge isn't
/// clipped), box-blur them by `blur`, and composite them under the box through `xf`. Only
/// the part of the padded silhouette whose blurred pixels can land on the canvas is drawn
/// (the canvas mapped back into the box's space; nothing under a singular transform), in
/// tiles of at most `tile` px a side, each rendered with the blur's reach of neighbours
/// (clamped to the padded silhouette) so every painted pixel is the one a single whole
/// scratch would give. A box far wider or taller than the canvas casts its shadow with
/// bounded memory (miaskiewicz/turbo-dom#95). Tiles meet exactly only along axis-aligned
/// edges, so under a rotation or skew the visible part is blurred in one scratch while it
/// is at most `max_scratch` px a side (beyond that it is tiled, the seams' pixels sampled
/// and covered per tile).
fn blurred_shadow(
    pm: &mut Pixmap,
    s: &Silhouette,
    blur: f32,
    xf: Transform,
    tile: i32,
    max_scratch: i32,
) {
    let Some(inv) = xf.invert() else {
        return;
    };
    let margin = blur.ceil() as i32;
    let (tw, th) = (
        s.width.ceil() as i32 + 2 * margin,
        s.height.ceil() as i32 + 2 * margin,
    );
    // The padded silhouette's origin in the box's space.
    let (ox, oy) = (s.x as i32 - margin, s.y as i32 - margin);
    // CSS blur radius ≈ 2σ; a 3-pass box blur of radius ≈ blur/2 approximates it. Three
    // passes reach `3 * r` pixels away.
    let r = (blur / 2.0).round().max(1.0) as usize;
    let reach = 3 * r as i32 + 1;
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let mut corners = [
        Point::from_xy(0.0, 0.0),
        Point::from_xy(w, 0.0),
        Point::from_xy(0.0, h),
        Point::from_xy(w, h),
    ];
    inv.map_points(&mut corners);
    // The visible span along one axis, in padded-silhouette pixels, grown by the reach.
    let span = |lo: f32, hi: f32, origin: i32, len: i32| {
        let lo = (f64::from(lo).floor() - f64::from(reach) - f64::from(origin)).max(0.0);
        let hi = (f64::from(hi).ceil() + f64::from(reach) - f64::from(origin)).min(f64::from(len));
        (lo as i32, hi as i32)
    };
    let fold = |pick: fn(&Point) -> f32| {
        corners
            .iter()
            .map(pick)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    };
    let ((lx0, lx1), (ly0, ly1)) = (fold(|p| p.x), fold(|p| p.y));
    let (x0, x1) = span(lx0, lx1, ox, tw);
    let (y0, y1) = span(ly0, ly1, oy, th);
    let whole = !xf.is_scale_translate() && x1 - x0 <= max_scratch && y1 - y0 <= max_scratch;
    let tile = if whole { max_scratch } else { tile };
    let step = usize::try_from(tile).expect("a positive tile size");
    for ty in (y0..y1).step_by(step) {
        for tx in (x0..x1).step_by(step) {
            let (cx1, cy1) = ((tx + tile).min(x1), (ty + tile).min(y1));
            // The tile plus the blur's reach on every side, within the padded silhouette.
            let (gx0, gy0) = ((tx - reach).max(0), (ty - reach).max(0));
            let (gx1, gy1) = ((cx1 + reach).min(tw), (cy1 + reach).min(th));
            // At most `tile + 2 * reach` (< 3000) px a side: within tiny-skia's limits.
            let mut scratch =
                Pixmap::new((gx1 - gx0) as u32, (gy1 - gy0) as u32).expect("bounded shadow tile");
            fill_rrect(
                &mut scratch,
                (
                    (margin - gx0) as f32,
                    (margin - gy0) as f32,
                    s.width,
                    s.height,
                ),
                &s.radii,
                s.color,
                Transform::identity(),
            );
            box_blur(&mut scratch, r);
            // Draw the tile's core from a copy with one more pixel of real neighbours, so a
            // device pixel sampled just past a core edge (under a rotation) reads the pixel a
            // whole scratch has there, not a padded edge.
            let (ax0, ay0) = ((tx - 1).max(0), (ty - 1).max(0));
            let (ax1, ay1) = ((cx1 + 1).min(tw), (cy1 + 1).min(th));
            let apron = tiny_skia::IntRect::from_xywh(
                ax0 - gx0,
                ay0 - gy0,
                (ax1 - ax0) as u32,
                (ay1 - ay0) as u32,
            )
            .and_then(|rect| scratch.clone_rect(rect))
            .expect("the apron lies inside its scratch");
            let paint = Paint {
                shader: tiny_skia::Pattern::new(
                    apron.as_ref(),
                    SpreadMode::Pad,
                    tiny_skia::FilterQuality::Nearest,
                    1.0,
                    Transform::identity(),
                ),
                anti_alias: false,
                ..Paint::default()
            };
            let core = Rect::from_xywh(
                (tx - ax0) as f32,
                (ty - ay0) as f32,
                (cx1 - tx) as f32,
                (cy1 - ty) as f32,
            )
            .expect("a non-empty core");
            // `PixmapMut::draw_pixmap` is this fill over the whole pixmap: one tile covering
            // the whole padded silhouette paints exactly what drawing it in one piece did.
            let place = xf.pre_concat(Transform::from_translate(
                (ox + ax0) as f32,
                (oy + ay0) as f32,
            ));
            pm.fill_rect(core, &paint, place, None);
        }
    }
}

/// Paint a `linear-gradient(...)` background into the box (clipped to its rounded
/// rect when `border_radius > 0`). The CSS gradient-line angle (0 = to top, 90 = to
/// right, clockwise) maps to start/end points across the box: the line runs through
/// the centre, its half-length the CSS `|w·sinθ| + |h·cosθ|` projection so the 0%
/// and 100% stops land at the box's extreme corners along the line.
fn paint_gradient(
    pm: &mut Pixmap,
    f: &Fragment,
    border_radius: &BorderRadii,
    g: &CssGradient,
    xf: Transform,
) {
    if !fragment_finite_positive(f)
        || f.width < 0.5
        || f.height < 0.5
        || g.stops.len() < 2
        || g.stops.iter().any(|s| !s.pos.is_finite())
    {
        return;
    }
    let (w, h) = (f.width, f.height);
    let theta = g.angle_deg.to_radians();
    let (dx, dy) = (theta.sin(), -theta.cos()); // 0°→(0,-1) up; 90°→(1,0) right
    let half = (w * theta.sin().abs() + h * theta.cos().abs()) / 2.0;
    let (cx, cy) = (f.x + w / 2.0, f.y + h / 2.0);
    let start = Point::from_xy(cx - dx * half, cy - dy * half);
    let end = Point::from_xy(cx + dx * half, cy + dy * half);
    let stops: Vec<GradientStop> = g
        .stops
        .iter()
        .map(|s| {
            GradientStop::new(
                s.pos.clamp(0.0, 1.0),
                tiny_skia::Color::from_rgba8(s.color.r, s.color.g, s.color.b, s.color.a),
            )
        })
        .collect();
    let Some(shader) =
        LinearGradient::new(start, end, stops, SpreadMode::Pad, Transform::identity())
    else {
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    };
    if border_radius.is_rounded() {
        if let Some(path) = rrect_path((f.x, f.y, w, h), border_radius) {
            pm.fill_path(&path, &paint, FillRule::Winding, xf, None);
        }
    } else if let Some(rect) = Rect::from_xywh(f.x, f.y, w, h) {
        pm.fill_rect(rect, &paint, xf, None);
    }
}

/// Approximate a Gaussian blur with three passes of a separable box blur over the
/// pixmap's premultiplied RGBA (edges clamp-extended). Only shadow scratch buffers
/// pass through here, so the O(passes·w·h) cost is bounded by the shadow's size.
/// `radius` is at least 1 (a radius of 0 would be the identity) and a `Pixmap` is never
/// empty, so both passes see a non-empty row and column.
fn box_blur(pm: &mut Pixmap, radius: usize) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let data = pm.data_mut();
    for _ in 0..3 {
        box_blur_horizontal(data, w, h, radius);
        box_blur_vertical(data, w, h, radius);
    }
}

/// One horizontal box-blur pass with a running window sum (clamp-extended edges).
fn box_blur_horizontal(data: &mut [u8], w: usize, h: usize, r: usize) {
    let win = (2 * r + 1) as u32;
    let mut src = vec![0u8; w * 4];
    for y in 0..h {
        let row = &mut data[y * w * 4..(y + 1) * w * 4];
        src.copy_from_slice(row);
        let get = |x: isize, c: usize| u32::from(src[x.clamp(0, w as isize - 1) as usize * 4 + c]);
        for c in 0..4 {
            let mut sum: u32 = (0..=r as isize).map(|x| get(x, c)).sum::<u32>()
                + (1..=r as isize).map(|_| get(0, c)).sum::<u32>();
            for x in 0..w {
                row[x * 4 + c] = (sum / win) as u8;
                sum = sum - get(x as isize - r as isize, c) + get(x as isize + r as isize + 1, c);
            }
        }
    }
}

/// One vertical box-blur pass, mirror of [`box_blur_horizontal`] over columns.
fn box_blur_vertical(data: &mut [u8], w: usize, h: usize, r: usize) {
    let win = (2 * r + 1) as u32;
    let mut src = vec![0u8; h * 4];
    for x in 0..w {
        for y in 0..h {
            let s = (y * w + x) * 4;
            src[y * 4..y * 4 + 4].copy_from_slice(&data[s..s + 4]);
        }
        let get = |y: isize, c: usize| u32::from(src[y.clamp(0, h as isize - 1) as usize * 4 + c]);
        for c in 0..4 {
            let mut sum: u32 = (0..=r as isize).map(|y| get(y, c)).sum::<u32>()
                + (1..=r as isize).map(|_| get(0, c)).sum::<u32>();
            for y in 0..h {
                data[(y * w + x) * 4 + c] = (sum / win) as u8;
                sum = sum - get(y as isize - r as isize, c) + get(y as isize + r as isize + 1, c);
            }
        }
    }
}

/// Paint a rounded box's border (miaskiewicz/turbo-dom#253): the ring between the
/// border-box outline and the padding-box outline (its inner radii), in one fill when every
/// painted side has one colour, else each side's part of the ring in its colour, the sides
/// meeting on each corner's diagonal ([`rrect::side_region`]).
fn paint_round_border(
    pm: &mut Pixmap,
    f: &Fragment,
    b: &BorderEdges,
    radii: &BorderRadii,
    xf: Transform,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let rect = (f.x, f.y, f.width, f.height);
    let (inner, inner_radii) = rrect::padding_box(f, b, radii);
    let mut ring = PathBuilder::new();
    trace(&mut ring, &rrect::outline(rect, radii));
    if inner.2 > 0.0 && inner.3 > 0.0 {
        trace(&mut ring, &rrect::outline(inner, &inner_radii));
    }
    let Some(ring) = ring.finish() else {
        return;
    };
    let sides = [&b.top, &b.right, &b.bottom, &b.left];
    let painted: Vec<(usize, Rgba)> = sides
        .iter()
        .enumerate()
        .filter(|(_, s)| s.width > 0)
        .filter_map(|(i, s)| s.color.map(|c| (i, c)))
        .collect();
    let widths = rrect::widths(b);
    let all = widths.iter().filter(|w| **w > 0.0).count();
    if painted.len() == all && painted.windows(2).all(|p| p[0].1 == p[1].1) {
        if let Some(&(_, c)) = painted.first() {
            pm.fill_path(&ring, &solid(c), FillRule::EvenOdd, xf, None);
        }
        return;
    }
    for (side, c) in painted {
        let region = rrect::side_region(rect, widths, side);
        let mut clip = PathBuilder::new();
        trace(&mut clip, &rrect::polygon(&region));
        if let Some(clip) = clip.finish() {
            paint_masked(pm, &clip, xf, |layer, local| {
                layer.fill_path(&ring, &solid(c), FillRule::EvenOdd, local, None);
            });
        }
    }
}

/// Fill a line's glyph outlines in `color` (a transparent colour paints nothing). The
/// face's `units_per_em` is non-zero: `ttf-parser` rejects a `head` outside 16..=16384.
/// Where a line's glyph pens land: as positioned, or for a line laid out in a vertical
/// writing mode turned a quarter clockwise about the fragment, a pen at `(f.x + x,
/// f.y + y)` landing at `(f.x + f.width - y, f.y + x)` (miaskiewicz/turbo-dom#234).
pub(crate) fn glyph_frame(f: &Fragment, vertical: bool) -> Transform {
    if vertical {
        Transform::from_row(0.0, 1.0, -1.0, 0.0, f.x + f.width + f.y, f.y - f.x)
    } else {
        Transform::identity()
    }
}

/// An upright glyph of a turned (vertical) run turns back a quarter about its pen
/// `(x, y)`, so it paints unturned (miaskiewicz/turbo-dom#239).
pub(crate) fn upright_turn(upright: bool, (x, y): (f32, f32)) -> Transform {
    if upright {
        Transform::from_rotate_at(-90.0, x, y)
    } else {
        Transform::identity()
    }
}

fn paint_text(
    pm: &mut Pixmap,
    (f, upright): (&Fragment, bool),
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    (font_size, color): (f32, Rgba),
    xf: Transform,
) {
    if color.a == 0 {
        return;
    }
    let run = crate::glyph_raster::run_outline((f, upright), glyphs, face, font_size, xf);
    crate::glyph_raster::fill(pm, &run, color, font_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_border_clips_to_the_viewport_and_ignores_empty_boxes() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        for x in [-0.5, 3.5] {
            fill_rect(
                &mut pixmap,
                x,
                0.0,
                1.0,
                4.0,
                Rgba::BLACK,
                Transform::identity(),
            );
        }
        fill_rect(
            &mut pixmap,
            1.0,
            0.0,
            0.0,
            4.0,
            Rgba::BLACK,
            Transform::identity(),
        );
        fill_rect(
            &mut pixmap,
            1.0,
            0.0,
            1.0,
            0.0,
            Rgba::BLACK,
            Transform::identity(),
        );
        for y in 0..4 {
            for x in [0, 3] {
                let alpha = pixmap.pixel(x, y).unwrap().alpha();
                assert!((120..=135).contains(&alpha), "edge coverage {alpha}");
            }
            for x in [1, 2] {
                assert_eq!(pixmap.pixel(x, y).unwrap().alpha(), 0);
            }
        }
    }

    #[test]
    fn fractional_single_pixel_border_preserves_partial_coverage() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        fill_rect(
            &mut pixmap,
            0.5,
            0.0,
            1.0,
            4.0,
            Rgba::BLACK,
            Transform::identity(),
        );
        for y in 0..4 {
            let left = pixmap.pixel(0, y).unwrap().alpha();
            let right = pixmap.pixel(1, y).unwrap().alpha();
            assert!((120..=135).contains(&left), "left coverage {left}");
            assert!((120..=135).contains(&right), "right coverage {right}");
            assert_eq!(pixmap.pixel(2, y).unwrap().alpha(), 0);
        }
    }

    use std::collections::HashMap;
    use turbo_dom_layout::image::Orientation;
    use turbo_dom_layout::{
        GradientStop, ImagePlacement, LinearGradient, MarkerKind, NodeId, Transform2D,
    };

    const RED: Rgba = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    /// A plain box fragment (no border, radius, shadow, gradient or transform).
    fn plain(x: f32, y: f32, w: f32, h: f32, background: Option<Rgba>) -> Fragment {
        Fragment::new(
            NodeId(0),
            x,
            y,
            w,
            h,
            FragmentContent::Box {
                background,
                border: BorderEdges::default(),
                border_radius: BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            },
        )
    }

    /// `f` with its box content's fields changed by `edit`.
    fn with_box(
        mut f: Fragment,
        edit: impl FnOnce(
            &mut f32,
            &mut Option<BoxShadow>,
            &mut Option<CssGradient>,
            &mut Option<BoxTransform>,
        ),
    ) -> Fragment {
        if let FragmentContent::Box {
            border_radius,
            shadow,
            gradient,
            transform,
            ..
        } = &mut f.content
        {
            let mut r = border_radius.as_uniform().unwrap_or(0.0);
            edit(&mut r, shadow, gradient, transform);
            *border_radius = r.into();
        }
        f
    }

    /// Paint `galley` over white at `w x h` and decode the PNG.
    fn render(galley: &Fragment, w: u32, h: u32, images: &DecodedAssets) -> Pixmap {
        let png = paint(galley, w, h, Rgba::new(255, 255, 255, 255), images).unwrap();
        Pixmap::decode_png(&png).unwrap()
    }

    fn rgb(pm: &Pixmap, x: u32, y: u32) -> [u8; 3] {
        let p = pm.pixel(x, y).unwrap();
        [p.red(), p.green(), p.blue()]
    }

    fn all_white(pm: &Pixmap) -> bool {
        pm.pixels()
            .iter()
            .all(|p| [p.red(), p.green(), p.blue()] == [255, 255, 255])
    }

    #[test]
    fn opacity_composites_the_box_and_its_subtree_as_one_group() {
        // miaskiewicz/turbo-dom#206: an `opacity:.5` red box with a red child over white
        // paints one half-transparent layer (where they overlap, still 50%, not 75%), as
        // Chrome's transparency group does; `opacity:0` paints nothing.
        let mut half = plain(0.0, 0.0, 8.0, 8.0, Some(RED));
        half.children.push(plain(0.0, 0.0, 4.0, 4.0, Some(RED)));
        half.opacity = 0.5;
        let pm = render(&half, 10, 10, &HashMap::new());
        assert_eq!(rgb(&pm, 2, 2), [255, 128, 128]);
        assert_eq!(rgb(&pm, 6, 6), [255, 128, 128]);
        assert_eq!(rgb(&pm, 9, 9), [255, 255, 255]);
        let mut gone = plain(0.0, 0.0, 8.0, 8.0, Some(RED));
        gone.opacity = 0.0;
        assert!(all_white(&render(&gone, 10, 10, &HashMap::new())));
        // An opacity group wholly off the canvas has no layer and paints nothing (#211).
        let mut away = plain(20.0, 20.0, 8.0, 8.0, Some(RED));
        away.opacity = 0.5;
        assert!(all_white(&render(&away, 10, 10, &HashMap::new())));
    }

    /// `f` (an opacity group) painted the pre-#206-follow-up way: one canvas-sized layer.
    fn full_layer_reference(f: &Fragment, w: u32, h: u32, images: &DecodedAssets) -> Pixmap {
        let mut pm = Pixmap::new(w, h).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        let mut layer = Pixmap::new(w, h).unwrap();
        paint_opaque(&mut layer, f, images, Transform::identity());
        let paint = PixmapPaint {
            opacity: f.opacity,
            ..PixmapPaint::default()
        };
        pm.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
        pm
    }

    #[test]
    fn column_rule_steps_require_strictly_forward_coordinates() {
        for (cursor, period, expected) in [
            (0.0, 3.0, Some(3.0)),
            (-3.0, 2.0, Some(-1.0)),
            (1_000_000_000.0, 3.0, None),
            (1.0, 0.0, None),
            (1.0, -1.0, None),
            (f32::NAN, 1.0, None),
            (f32::INFINITY, 1.0, None),
            (1.0, f32::NAN, None),
        ] {
            assert_eq!(next_column_rule_step(cursor, period), expected);
        }
        let transform = Transform::from_translate(7.0, 11.0);
        assert_eq!(column_rule_space(5.0, 3.0, transform), (5.0, transform));
        let (top, local) = column_rule_space(
            1_000_000_000.0,
            3.0,
            Transform::from_translate(0.0, -1_000_000_000.0),
        );
        assert_eq!(top, 0.0);
        assert_eq!(local, Transform::identity());
    }

    #[test]
    fn an_image_touching_the_canvas_edge_has_no_visible_resampling_window() {
        for (x, y) in [
            (12.0, 0.0),
            (0.0, 12.0),
            (12.0, 12.0),
            (-4.0, 0.0),
            (0.0, -4.0),
        ] {
            assert_eq!(
                visible_window(Transform::from_translate(x, y), (4, 4), (12, 12)),
                None,
                "an edge-only intersection at ({x}, {y}) contains no image pixels",
            );
        }
    }

    #[test]
    fn opacity_groups_keep_hard_shadow_spread_at_fractional_scaled_edges() {
        let mut groups = Vec::new();
        for (dx, dy) in [(6, 6), (-6, 6), (6, -6), (-6, -6)] {
            groups.push(with_box(
                plain(6.0, 6.0, 5.0, 5.0, None),
                |_, shadow, _, _| {
                    *shadow = Some(BoxShadow {
                        offset_x: dx,
                        offset_y: dy,
                        blur: 0,
                        spread: 4,
                        color: RED,
                        inset: false,
                    });
                },
            ));
        }
        // The silhouette width rounds up when its large spread is added. Its
        // actual right edge reaches device x=4; summing the box's right edge and
        // spread separately only reaches x=3. The conservative margin must keep
        // that real shadow pixel rather than trusting associative f32 addition.
        groups.push(with_box(
            plain(0.0, 0.0, 3.0 / 32.0, 1.0, None),
            |_, shadow, _, transform| {
                *shadow = Some(BoxShadow {
                    offset_x: 0,
                    offset_y: 0,
                    blur: 0,
                    spread: 262_144,
                    color: RED,
                    inset: false,
                });
                *transform = Some(BoxTransform {
                    matrix: [32.0, 0.0, 0.0, 32.0, -8_388_608.0, 0.0],
                    origin_x: 0.0,
                    origin_y: 0.0,
                });
            },
        ));
        for (index, mut group) in groups.into_iter().enumerate() {
            group.opacity = 0.5;
            let expected = full_layer_reference(&group, 24, 24, &HashMap::new());
            assert!(
                !all_white(&expected),
                "reference case {index} must actually paint"
            );
            if index == 4 {
                assert_ne!(
                    rgb(&expected, 3, 8),
                    [255; 3],
                    "the rounded shadow edge paints x=3"
                );
            }
            let actual = render(&group, 24, 24, &HashMap::new());
            assert_eq!(actual.data(), expected.data(), "shadow group {index}");
        }
    }

    fn check_opacity_ink(mut group: Fragment, images: &DecodedAssets, label: &str) {
        group.opacity = 0.5;
        let expected = full_layer_reference(&group, 256, 256, images);
        assert!(
            !all_white(&expected),
            "{label}: the independent reference paints"
        );
        let actual = render(&group, 256, 256, images);
        let first_difference = actual
            .pixels()
            .iter()
            .zip(expected.pixels())
            .position(|(a, b)| a != b);
        assert!(
            first_difference.is_none(),
            "{label}: first differing pixel {first_difference:?}"
        );
    }

    #[test]
    fn public_large_spread_blur_preserves_rounded_visible_shadow_edge() {
        let mut group = with_box(
            plain(0.0, 0.0, 4.0, 4.0, None),
            |_, shadow, _, transform| {
                *shadow = Some(BoxShadow {
                    offset_x: 0,
                    offset_y: 0,
                    blur: 3,
                    spread: 67_108_860,
                    color: RED,
                    inset: false,
                });
                *transform = Some(BoxTransform {
                    matrix: [1.0, 0.0, 0.0, 1.0, -67_108_832.0, 0.0],
                    origin_x: 0.0,
                    origin_y: 0.0,
                });
            },
        );
        group.opacity = 0.5;
        let expected = full_layer_reference(&group, 128, 64, &HashMap::new());
        assert_ne!(
            rgb(&expected, 35, 16),
            [255; 3],
            "real blurred shadow ink reaches beyond x=32"
        );
        let html = "<!doctype html><body style='margin:0;background:white'><div style='position:absolute;left:0;top:0;width:4px;height:4px;opacity:0.5;box-shadow:0 0 3px 67108860px red;transform:translateX(-67108832px);transform-origin:0 0'></div>";
        let png = crate::screenshot_png_with_opts(
            html,
            "",
            crate::Viewport {
                width: 128,
                height: 64,
            },
            &HashMap::new(),
            false,
            false,
        )
        .unwrap();
        let actual = Pixmap::decode_png(&png).unwrap();
        let first_difference = actual
            .pixels()
            .iter()
            .zip(expected.pixels())
            .position(|(a, b)| a != b);
        assert!(
            first_difference.is_none(),
            "public shadow edge differs from independent full-canvas paint at {first_difference:?}"
        );
    }

    #[test]
    fn opacity_groups_keep_signed_shadow_offsets_and_spreads() {
        for position in [-8.0, 0.0, 6.0] {
            for dx in [-6, 0, 6] {
                for dy in [-6, 0, 6] {
                    for spread in [0, 4, 10] {
                        let group = with_box(
                            plain(position, position, 5.0, 5.0, None),
                            |_, shadow, _, transform| {
                                *shadow = Some(BoxShadow {
                                    offset_x: dx,
                                    offset_y: dy,
                                    blur: 0,
                                    spread,
                                    color: RED,
                                    inset: false,
                                });
                                *transform = Some(BoxTransform {
                                    matrix: [1.0, 0.0, 0.0, 1.0, 32.0, 32.0],
                                    origin_x: 0.0,
                                    origin_y: 0.0,
                                });
                            },
                        );
                        check_opacity_ink(
                            group,
                            &HashMap::new(),
                            &format!("shadow {position}/{dx}/{dy}/{spread}"),
                        );
                    }
                }
            }
        }
    }

    fn ink_test_line(
        face: &FontFace,
        character: char,
        origin: (f32, f32),
        pen: (f32, f32),
        font_size: f32,
    ) -> Fragment {
        Fragment::new(
            NodeId(0),
            origin.0,
            origin.1,
            1.0,
            1.0,
            FragmentContent::TextLine {
                glyphs: vec![PositionedGlyph {
                    glyph_id: face.glyph_index(character).unwrap(),
                    x: pen.0,
                    y: pen.1,
                    text_start: 0,
                    text_end: 0,
                }],
                text: character.to_string(),
                face: face.clone(),
                font_size,
                color: RED,
                vertical: false,
                upright: false,
            },
        )
    }

    #[test]
    fn opacity_groups_keep_real_mark_overhangs_and_remote_glyph_pens() {
        let arabic = FontFace::from_bytes(
            include_bytes!(
                "../../turbo-dom-layout/assets/fonts/noto-sans-arabic/NotoSansArabic-Regular.ttf"
            )
            .to_vec(),
            "Noto Sans Arabic",
            400,
            false,
        )
        .unwrap();
        let italic = FontFace::from_bytes(
            include_bytes!("../../turbo-dom-layout/assets/fonts/inter/Inter-Italic.otf").to_vec(),
            "Inter",
            400,
            true,
        )
        .unwrap();
        let mark = '\u{0898}';
        assert_eq!(
            arabic.glyph_advance(arabic.glyph_index(mark).unwrap()),
            0,
            "a real combining mark has no advance width"
        );
        check_opacity_ink(
            ink_test_line(&arabic, mark, (80.0, 80.0), (0.0, 0.0), 64.0),
            &HashMap::new(),
            "high Arabic mark",
        );
        check_opacity_ink(
            ink_test_line(&italic, 'M', (0.0, 0.0), (80.0, 80.0), 16.0),
            &HashMap::new(),
            "remote glyph pen",
        );
        let mut translated = with_box(plain(-80.0, -80.0, 1.0, 1.0, None), |_, _, _, transform| {
            *transform = Some(BoxTransform {
                matrix: [1.0, 0.0, 0.0, 1.0, 100.0, 100.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        });
        translated.children.push(ink_test_line(
            &italic,
            'j',
            (-80.0, -80.0),
            (0.0, 0.0),
            32.0,
        ));
        check_opacity_ink(
            translated,
            &HashMap::new(),
            "negative pen with real italic overhang and descender",
        );
        let mut translated_mark =
            with_box(plain(80.0, -80.0, 1.0, 1.0, None), |_, _, _, transform| {
                *transform = Some(BoxTransform {
                    matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 100.0],
                    origin_x: 0.0,
                    origin_y: 0.0,
                });
            });
        translated_mark.children.push(ink_test_line(
            &arabic,
            mark,
            (80.0, -80.0),
            (0.0, 0.0),
            16.0,
        ));
        check_opacity_ink(translated_mark, &HashMap::new(), "negative mark baseline");
    }

    #[test]
    fn opacity_groups_keep_image_pixels_rounded_before_scaled_placement() {
        let mut images = HashMap::new();
        images.insert(
            "pic".to_string(),
            crate::image_paint::DecodedImage {
                width: 1,
                height: 1,
                rgba: vec![255, 0, 0, 255],
                intrinsic: turbo_dom_layout::image::Intrinsic::new(1, 1, false),
                placeholder: false,
            },
        );
        for (position, size) in [(7.0, 6.0), (7.25, 6.25), (7.75, 6.75)] {
            let mut group = with_box(
                plain(position, position, 1.0, 1.0, None),
                |_, _, _, transform| {
                    *transform = Some(BoxTransform {
                        matrix: [4.0, 0.0, 0.0, 4.0, 0.0, 0.0],
                        origin_x: 0.0,
                        origin_y: 0.0,
                    });
                },
            );
            let mut picture = image("pic", None);
            (picture.x, picture.y, picture.width, picture.height) =
                (position, position, size, size);
            group.children.push(picture);
            check_opacity_ink(group, &images, &format!("rounded image {position}/{size}"));
        }
    }

    #[test]
    fn an_opacity_layer_covers_only_its_subtree_ink() {
        // miaskiewicz/turbo-dom#206 follow-up: a canvas-sized layer per opacity box made
        // 2000 one-pixel `opacity:.5` divs take 14 s at 1280x800.
        let id = Transform::identity();
        let mut dot = plain(5.0, 6.0, 1.0, 1.0, Some(RED));
        dot.opacity = 0.5;
        assert_eq!(group_layer(&dot, id, 1280, 800), Some((5, 6, 1, 1)));
        // A child outside the parent box widens the layer; the canvas clamps it.
        dot.children.push(plain(1270.0, 2.0, 40.0, 3.0, Some(RED)));
        assert_eq!(group_layer(&dot, id, 1280, 800), Some((5, 2, 1275, 5)));
        // The device transform moves it; off-canvas ink has no layer.
        let shifted = Transform::from_translate(10.0, 20.0);
        let mut lone = plain(5.0, 6.0, 2.0, 2.0, Some(RED));
        lone.opacity = 0.5;
        assert_eq!(group_layer(&lone, shifted, 100, 100), Some((15, 26, 2, 2)));
        let off = Transform::from_translate(500.0, 0.0);
        assert_eq!(group_layer(&lone, off, 100, 100), None);
        // A transform with no finite bounds keeps the canvas-sized layer.
        let huge = with_box(plain(5.0, 6.0, 2.0, 2.0, Some(RED)), |_, _, _, t| {
            *t = Some(BoxTransform {
                matrix: [f32::MAX, 0.0, 0.0, f32::MAX, 0.0, 0.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        });
        assert_eq!(group_layer(&huge, id, 100, 100), Some((0, 0, 100, 100)));
        let mut parent = plain(0.0, 0.0, 2.0, 2.0, None);
        parent.children.push(huge);
        assert_eq!(group_layer(&parent, id, 100, 100), Some((0, 0, 100, 100)));
    }

    #[test]
    fn a_bounded_opacity_layer_paints_what_a_canvas_layer_does() {
        // The tight layer must still hold every pixel the group paints: a blurred,
        // offset shadow, a transformed child, and glyph ink outside its line box.
        let reg = turbo_dom_layout::FontRegistry::new();
        let face = reg.select(&["Inter"], 400, false).unwrap().clone();
        let glyphs: Vec<PositionedGlyph> = face
            .shape("Ag")
            .iter()
            .enumerate()
            .map(|(i, g)| PositionedGlyph {
                glyph_id: g.glyph_id,
                x: i as f32 * 20.0,
                y: 30.0,
                text_start: 0,
                text_end: 0,
            })
            .collect();
        let line = Fragment::new(
            NodeId(0),
            20.0,
            10.0,
            1.0,
            1.0,
            FragmentContent::TextLine {
                glyphs,
                text: String::new(),
                face,
                font_size: 30.0,
                color: RED,
                vertical: false,
                upright: false,
            },
        );
        let shadowed = with_box(plain(30.0, 30.0, 20.0, 20.0, Some(RED)), |_, s, _, _| {
            *s = Some(BoxShadow {
                offset_x: 12,
                offset_y: -9,
                blur: 6,
                spread: 3,
                color: RED,
                inset: false,
            });
        });
        let rotated = with_box(plain(60.0, 60.0, 20.0, 10.0, Some(RED)), |_, _, _, t| {
            *t = Some(BoxTransform {
                matrix: [0.0, 1.0, -1.0, 0.0, 0.0, 0.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        });
        // A decoded image at a fractional box paints at its rounded box (8..15 here).
        let mut pic = image("pic", None);
        (pic.x, pic.y, pic.width, pic.height) = (7.5, 7.5, 6.5, 6.5);
        let mut images = HashMap::new();
        images.insert(
            "pic".to_string(),
            crate::image_paint::DecodedImage {
                width: 1,
                height: 1,
                rgba: vec![255, 0, 0, 255],
                intrinsic: turbo_dom_layout::image::Intrinsic::new(1, 1, false),
                placeholder: false,
            },
        );
        // A vertical writing-mode line paints its glyphs turned a quarter clockwise
        // about the line box (miaskiewicz/turbo-dom#234): its ink is where they land.
        let mut vertical_line = line.clone();
        vertical_line.x = 60.0;
        if let FragmentContent::TextLine { vertical, .. } = &mut vertical_line.content {
            *vertical = true;
        }
        // Each group alone, so one feature's margin cannot hide another's.
        let groups = [shadowed, line, rotated, pic, vertical_line];
        for (i, mut group) in groups.into_iter().enumerate() {
            group.opacity = 0.5;
            let got = render(&group, 120, 120, &images);
            let want = full_layer_reference(&group, 120, 120, &images);
            assert!(!all_white(&got), "group {i} paints");
            assert_eq!(got.data(), want.data(), "group {i}");
        }
    }

    fn image(name: &str, tint: Option<Rgba>) -> Fragment {
        Fragment::new(
            NodeId(0),
            2.0,
            2.0,
            6.0,
            6.0,
            FragmentContent::Image(ImagePlacement {
                name: name.to_string(),
                intrinsic_w: 6,
                intrinsic_h: 6,
                has_alpha: true,
                tint,
                orientation: Orientation::IDENTITY,
            }),
        )
    }

    #[test]
    fn a_repeated_tile_shares_one_scaled_pixmap() {
        // A background layer draws one image many times: every hit of the memo is the
        // same pixmap, not a copy of it (round-1 review of 0.7.0, finding 6).
        let img = crate::image_paint::DecodedImage {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
            intrinsic: turbo_dom_layout::image::Intrinsic::new(1, 1, false),
            placeholder: false,
        };
        SCALED.with(|memo| memo.borrow_mut().take());
        let first = scaled(&img, Orientation::IDENTITY, (4, 3), (0, 0, 4, 3)).unwrap();
        let again = scaled(&img, Orientation::IDENTITY, (4, 3), (0, 0, 4, 3)).unwrap();
        assert!(std::rc::Rc::ptr_eq(&first, &again));
        assert_eq!((again.width(), again.height()), (4, 3));
        let other = scaled(&img, Orientation::IDENTITY, (2, 3), (0, 0, 2, 3)).unwrap();
        assert!(!std::rc::Rc::ptr_eq(&first, &other));
    }

    #[test]
    fn a_tile_texture_is_the_tiles_pixels_within_a_budget() {
        // miaskiewicz/turbo-dom#270: one tile's pixels at the scale rounded up to whole
        // pixels; an image at most at its own resolution, stencilled when it is a mask.
        let red = crate::image_paint::DecodedImage {
            width: 2,
            height: 2,
            rgba: [255, 0, 0, 255].repeat(4),
            intrinsic: turbo_dom_layout::image::Intrinsic::new(2, 2, false),
            placeholder: false,
        };
        let images: DecodedAssets = [("a.png".to_string(), red)].into_iter().collect();
        let pattern = |content: FragmentContent, size: f32| TilePattern {
            tile: Box::new(content),
            x: 0.0,
            y: 0.0,
            tile_width: size,
            tile_height: size,
            step_x: size,
            step_y: size,
        };
        let mask = pattern(image("a.png", Some(Rgba::new(0, 0, 255, 255))).content, 1.5);
        let texture = tile_texture(&mask, &images, 4.0, 100).unwrap();
        assert_eq!(
            (texture.width(), texture.height()),
            (2, 2),
            "the image's own 2px"
        );
        assert_eq!(rgb(&texture, 1, 1), [0, 0, 255], "the mask's tint");
        let plain_image = pattern(image("a.png", None).content, 1.5);
        let texture = tile_texture(&plain_image, &images, 1.0, 100).unwrap();
        assert_eq!(rgb(&texture, 0, 0), [255, 0, 0]);
        // Over the budget: none (painted tile by tile).
        assert!(tile_texture(&plain_image, &images, 1.0, 3).is_none());
        let gradient = pattern(plain(0.0, 0.0, 1.0, 1.0, Some(RED)).content, 2.5);
        let texture = tile_texture(&gradient, &images, 2.0, 25).unwrap();
        assert_eq!((texture.width(), texture.height()), (5, 5));
        assert!(tile_texture(&gradient, &images, 2.0, 24).is_none());
    }

    #[test]
    fn fractional_tiles_too_big_for_a_texture_paint_tile_by_tile() {
        if !super::exact_tests::rule_probe_child(
            "paint_png::tests::fractional_tiles_too_big_for_a_texture_paint_tile_by_tile",
        ) {
            return;
        }
        // 250.5px tiles under scale(0.5) on a 200px canvas: two tiles show, a 251px
        // texture would outgrow the canvas, so the tiles are walked.
        let grid = tiles(
            (0.0, 0.0, 600.0, 600.0),
            RED,
            (0.0, 0.0),
            (250.5, 250.5),
            (250.5, 250.5),
        );
        let mut pm = Pixmap::new(200, 200).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        paint_subtree(
            &mut pm,
            &grid,
            &HashMap::new(),
            Transform::from_scale(0.5, 0.5),
        );
        assert_eq!(rgb(&pm, 10, 10), [255, 0, 0]);
        assert_eq!(rgb(&pm, 199, 199), [255, 0, 0], "the second tile");
    }

    #[test]
    fn a_zero_or_oversized_canvas_is_an_error() {
        let root = plain(0.0, 0.0, 1.0, 1.0, None);
        let images = HashMap::new();
        let bg = Rgba::new(255, 255, 255, 255);
        assert_eq!(
            paint(&root, 0, 10, bg, &images),
            Err("bad canvas 0x10".into())
        );
        assert!(paint(&root, u32::MAX, u32::MAX, bg, &images).is_err());
    }

    #[test]
    fn a_marker_paints_nothing_but_its_children_do() {
        let mut marker = Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            10.0,
            10.0,
            FragmentContent::Marker(MarkerKind {
                id: 1,
                inline: false,
            }),
        );
        marker.children.push(plain(5.0, 0.0, 5.0, 10.0, Some(RED)));
        let pm = render(&marker, 10, 10, &HashMap::new());
        assert_eq!(rgb(&pm, 2, 5), [255, 255, 255], "the marker itself");
        assert_eq!(rgb(&pm, 7, 5), [255, 0, 0], "its child");
    }

    /// A tile pattern fragment over `(x, y, w, h)`: solid `colour` tiles of
    /// `tile` px every `step` px from `at`.
    fn tiles(
        (x, y, w, h): rrect::Rect,
        colour: Rgba,
        at: (f32, f32),
        tile: (f32, f32),
        step: (f32, f32),
    ) -> Fragment {
        let content = plain(0.0, 0.0, 1.0, 1.0, Some(colour)).content;
        Fragment::new(
            NodeId(0),
            x,
            y,
            w,
            h,
            FragmentContent::Tiles(TilePattern {
                tile: Box::new(content),
                x: at.0,
                y: at.1,
                tile_width: tile.0,
                tile_height: tile.1,
                step_x: step.0,
                step_y: step.1,
            }),
        )
    }

    /// Which pixels of a `w x h` canvas are red.
    fn red_mask(pm: &Pixmap) -> Vec<bool> {
        pm.pixels()
            .iter()
            .map(|p| p.red() == 255 && p.green() == 0)
            .collect()
    }

    #[test]
    fn translated_tile_patterns_paint_in_phase() {
        // miaskiewicz/turbo-dom#274: a 100x50 box of 20px tiles every 25px (the shader
        // path) or every 25.5px across (walked one by one), moved by `Fragment::translate`,
        // paints its 8 tiles where the same pattern laid out at the new spot does.
        for step in [(25.0, 25.0), (25.5, 25.0)] {
            let mut moved = tiles((0.0, 0.0, 100.0, 50.0), RED, (0.0, 0.0), (20.0, 20.0), step);
            moved.translate(10.0, 20.0);
            let placed = tiles(
                (10.0, 20.0, 100.0, 50.0),
                RED,
                (10.0, 20.0),
                (20.0, 20.0),
                step,
            );
            let (got, want) = (
                red_mask(&render(&moved, 120, 80, &HashMap::new())),
                red_mask(&render(&placed, 120, 80, &HashMap::new())),
            );
            assert_eq!(got.iter().filter(|&&r| r).count(), 8 * 20 * 20, "{step:?}");
            assert!(got == want, "{step:?}");
        }
    }

    #[test]
    fn tile_patterns_paint_every_tile_by_shader_or_one_by_one() {
        // 0.7.0 round-2 review, finding 1. A 1px grid every 2px over the whole canvas:
        // the shader path; the last tile too.
        let grid = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (0.0, 0.0),
            (1.0, 1.0),
            (2.0, 2.0),
        );
        let pm = render(&grid, 40, 30, &HashMap::new());
        assert_eq!(rgb(&pm, 38, 28), [255, 0, 0]);
        assert_eq!(rgb(&pm, 39, 28), [255, 255, 255]);
        assert_eq!(rgb(&pm, 38, 29), [255, 255, 255]);
        // The same tiles walked one by one paint the same pixels: a phase off the grid
        // and a 1.5px step (no period) against a 3px step drawn at half scale.
        let walked = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (0.25, 0.0),
            (1.0, 1.0),
            (1.5, 2.0),
        );
        let pm = render(&walked, 40, 30, &HashMap::new());
        let xs: Vec<u32> = (0..10).filter(|&x| rgb(&pm, x, 0) == [255, 0, 0]).collect();
        assert_eq!(xs, [0, 2, 3, 5, 6, 8, 9]);
        // A repeat on one axis only: the tile's own row.
        let row = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (0.0, 4.0),
            (2.0, 3.0),
            (4.0, 0.0),
        );
        let pm = render(&row, 40, 30, &HashMap::new());
        assert_eq!(rgb(&pm, 37, 5), [255, 0, 0]);
        assert_eq!(rgb(&pm, 37, 8), [255, 255, 255]);
        assert_eq!(rgb(&pm, 1, 2), [255, 255, 255]);
        // A period larger than the canvas, and a lone tile, walk their few tiles.
        let big = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (-30.0, 0.0),
            (35.0, 10.0),
            (60.0, 60.0),
        );
        let pm = render(&big, 40, 30, &HashMap::new());
        assert_eq!(
            (rgb(&pm, 4, 5), rgb(&pm, 6, 5)),
            ([255, 0, 0], [255, 255, 255])
        );
        let lone = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (10.0, 10.0),
            (3.0, 3.0),
            (0.0, 0.0),
        );
        let pm = render(&lone, 40, 30, &HashMap::new());
        assert_eq!(red_mask(&pm).iter().filter(|&&r| r).count(), 9);
        // Off the canvas, through a transform with no inverse, or with no tile meeting
        // the visible part: nothing.
        let off = tiles(
            (50.0, 0.0, 40.0, 30.0),
            RED,
            (50.0, 0.0),
            (1.0, 1.0),
            (2.0, 2.0),
        );
        assert!(all_white(&render(&off, 40, 30, &HashMap::new())));
        let mut flat = plain(0.0, 0.0, 40.0, 30.0, None);
        if let FragmentContent::Box { transform, .. } = &mut flat.content {
            *transform = Some(Transform2D {
                matrix: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        }
        flat.children.push(grid.clone());
        assert!(all_white(&render(&flat, 40, 30, &HashMap::new())));
        let missed = tiles(
            (0.0, 0.0, 40.0, 30.0),
            RED,
            (100.0, 0.0),
            (1.0, 1.0),
            (0.0, 2.0),
        );
        assert!(all_white(&render(&missed, 40, 30, &HashMap::new())));
        // Under a transform the shader follows the box: a 2x scale doubles the grid.
        let mut scaled = plain(0.0, 0.0, 20.0, 15.0, None);
        if let FragmentContent::Box { transform, .. } = &mut scaled.content {
            *transform = Some(Transform2D {
                matrix: [2.0, 0.0, 0.0, 2.0, 0.0, 0.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        }
        scaled.children.push(tiles(
            (0.0, 0.0, 20.0, 15.0),
            RED,
            (0.0, 0.0),
            (1.0, 1.0),
            (2.0, 2.0),
        ));
        let pm = render(&scaled, 40, 30, &HashMap::new());
        assert_eq!(
            (rgb(&pm, 1, 1), rgb(&pm, 2, 1), rgb(&pm, 37, 25)),
            ([255, 0, 0], [255, 255, 255], [255, 0, 0])
        );
    }

    #[test]
    fn only_the_part_of_an_image_the_canvas_shows_is_resampled() {
        let red = crate::image_paint::DecodedImage {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
            intrinsic: turbo_dom_layout::image::Intrinsic::new(1, 1, false),
            placeholder: false,
        };
        let place = Transform::from_translate(-5.0, 3.0);
        // A 20px box at (-5, 3) on a 10px canvas: its columns 5..15 and rows 0..7.
        assert_eq!(
            visible_window(place, (20, 20), (10, 10)),
            Some((5, 0, 10, 7))
        );
        // Wholly off the canvas, or through a transform with no inverse: nothing.
        let off = Transform::from_translate(30.0, 0.0);
        assert_eq!(visible_window(off, (20, 20), (10, 10)), None);
        let flat = Transform::from_scale(0.0, 1.0);
        assert_eq!(visible_window(flat, (20, 20), (10, 10)), None);
        let mut images = HashMap::new();
        images.insert("pic".to_string(), red);
        let mut pic = image("pic", None);
        pic.x = 40.0;
        assert!(all_white(&render(&pic, 10, 10, &images)), "off the canvas");
        let mut pic = image("pic", None);
        pic.x = 2.0;
        let pm = render(&pic, 10, 10, &images);
        assert_eq!(rgb(&pm, 5, 5), [255, 0, 0]);
    }

    #[test]
    fn a_missing_mask_paints_nothing_and_a_missing_image_the_placeholder() {
        let images = HashMap::new();
        let mask = render(&image("absent.svg", Some(RED)), 10, 10, &images);
        assert!(all_white(&mask), "a missing mask-image is not a box");
        let img = render(&image("absent.png", None), 10, 10, &images);
        assert_eq!(rgb(&img, 5, 5), [220, 220, 220], "placeholder");
        assert_eq!(rgb(&img, 0, 0), [255, 255, 255]);
    }

    #[test]
    fn an_undecodable_image_paints_nothing_and_one_without_a_decoder_the_placeholder() {
        // Size-only entries (turbo-dom#107): pixels that did not decode paint nothing, as in
        // Chrome; a format this build has no decoder for paints the placeholder.
        let mut images = HashMap::new();
        for (name, placeholder) in [("undecodable", false), ("no-decoder", true)] {
            images.insert(
                name.to_string(),
                crate::image_paint::DecodedImage {
                    width: 0,
                    height: 0,
                    rgba: Vec::new(),
                    intrinsic: turbo_dom_layout::image::Intrinsic::new(2, 1, true),
                    placeholder,
                },
            );
        }
        let none = render(&image("undecodable", None), 10, 10, &images);
        assert!(all_white(&none), "nothing painted");
        let mask = render(&image("undecodable", Some(RED)), 10, 10, &images);
        assert!(all_white(&mask), "an undecodable mask paints nothing");
        let grey = render(&image("no-decoder", None), 10, 10, &images);
        assert_eq!(rgb(&grey, 5, 5), [220, 220, 220], "placeholder");
    }

    #[test]
    fn clips_with_no_paintable_region_paint_nothing() {
        // A negative-width region (no rect) and one whose transformed bounds overflow f32.
        let mut negative = plain(0.0, 0.0, 10.0, 10.0, Some(RED));
        negative.clip = Some(ClipRect {
            x: 0.0,
            y: 0.0,
            width: -5.0,
            height: 10.0,
            radii: BorderRadii::ZERO,
        });
        assert!(all_white(&render(&negative, 10, 10, &HashMap::new())));
        let mut overflow = with_box(plain(0.0, 0.0, 10.0, 10.0, Some(RED)), |_, _, _, t| {
            *t = Some(Transform2D {
                matrix: [f32::MAX, 0.0, 0.0, f32::MAX, 0.0, 0.0],
                origin_x: 0.0,
                origin_y: 0.0,
            });
        });
        overflow.clip = Some(ClipRect {
            x: 1.0,
            y: 1.0,
            width: 5.0,
            height: 5.0,
            radii: BorderRadii::ZERO,
        });
        assert!(all_white(&render(&overflow, 10, 10, &HashMap::new())));
    }

    #[test]
    fn round_rects_skip_empty_and_transparent_boxes() {
        let mut pm = Pixmap::new(10, 10).unwrap();
        let id = Transform::identity();
        fill_round_rect(&mut pm, 0.0, 0.0, 0.0, 10.0, 3.0, RED, id);
        fill_round_rect(&mut pm, 0.0, 0.0, 10.0, -1.0, 3.0, RED, id);
        fill_round_rect(
            &mut pm,
            0.0,
            0.0,
            10.0,
            10.0,
            3.0,
            Rgba::new(255, 0, 0, 0),
            id,
        );
        assert!(pm.pixels().iter().all(|p| p.alpha() == 0));
        fill_round_rect(&mut pm, 0.0, 0.0, 10.0, 10.0, 5.0, RED, id);
        assert_eq!(pm.pixel(5, 5).unwrap().red(), 255);
        assert!(pm.pixel(0, 0).unwrap().alpha() < 32, "rounded corner");
    }

    #[test]
    fn a_rounded_box_with_a_non_finite_size_paints_nothing() {
        let blue = Rgba::new(0, 0, 255, 255);
        let side = BorderSide {
            width: 2,
            color: Some(blue),
        };
        let mut f = with_box(plain(1.0, 1.0, f32::NAN, 8.0, Some(RED)), |r, _, _, _| {
            *r = 3.0;
        });
        if let FragmentContent::Box { border, .. } = &mut f.content {
            *border = BorderEdges {
                top: side,
                right: side,
                bottom: side,
                left: side,
            };
        }
        assert!(all_white(&render(&f, 10, 10, &HashMap::new())));
    }

    #[test]
    fn a_box_wider_than_8192px_casts_its_blurred_shadow() {
        let shadowed = |w: f32| {
            with_box(plain(10.0, 10.0, w, 20.0, Some(RED)), |_, sh, _, _| {
                *sh = Some(BoxShadow {
                    offset_x: 0,
                    offset_y: 20,
                    blur: 4,
                    spread: 0,
                    color: Rgba::new(0, 0, 255, 255),
                    inset: false,
                });
            })
        };
        let small = render(&shadowed(30.0), 60, 60, &HashMap::new());
        let huge = render(&shadowed(9000.0), 60, 60, &HashMap::new());
        assert_eq!(rgb(&huge, 20, 20), [255, 0, 0], "the box");
        // Away from the narrow box's right end the two shadows are the same pixels.
        for y in 0..60 {
            for x in 0..30 {
                assert_eq!(rgb(&huge, x, y), rgb(&small, x, y), "({x},{y})");
            }
        }
        assert_eq!(rgb(&huge, 25, 40), [0, 0, 255], "shadow core");
        assert_eq!(
            rgb(&huge, 55, 40),
            [0, 0, 255],
            "the wide box's shadow continues"
        );
    }

    fn silhouette(x: f32, y: f32) -> Silhouette {
        Silhouette {
            x,
            y,
            width: 70.0,
            height: 40.0,
            radii: BorderRadii::uniform(6.0),
            color: Rgba::new(0, 0, 255, 200),
        }
    }

    /// `s` blurred by 10px through `xf` onto a white `w x h` canvas in `tile`-px tiles (one
    /// scratch up to `max_scratch` px a side under a rotation or skew).
    fn shadow_in(
        w: u32,
        h: u32,
        s: &Silhouette,
        xf: Transform,
        tile: i32,
        max_scratch: i32,
    ) -> Pixmap {
        let mut pm = Pixmap::new(w, h).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        blurred_shadow(&mut pm, s, 10.0, xf, tile, max_scratch);
        pm
    }

    /// [`shadow_in`] at the production tile sizes.
    fn shadow_on(w: u32, h: u32, s: &Silhouette, xf: Transform) -> Pixmap {
        shadow_in(w, h, s, xf, SHADOW_TILE, MAX_SHADOW_SCRATCH)
    }

    /// The pixels where `a` and `b` differ.
    fn differing(a: &Pixmap, b: &Pixmap) -> Vec<(u32, u32)> {
        (0..a.height())
            .flat_map(|y| (0..a.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| a.pixel(x, y) != b.pixel(x, y))
            .collect()
    }

    #[test]
    fn axis_aligned_shadow_tiles_paint_the_pixels_of_one_whole_scratch() {
        // Tiles of 7 or 16 px (each with the blur's reach of neighbours) give exactly the
        // pixels one whole scratch does: in place, at a fractional offset, scaled.
        for (s, xf) in [
            (silhouette(15.0, 20.0), Transform::identity()),
            (silhouette(15.3, 20.7), Transform::from_translate(0.4, 0.6)),
            (silhouette(10.0, 10.0), Transform::from_scale(1.5, 0.75)),
        ] {
            let whole = shadow_in(120, 100, &s, xf, 10_000, 10_000);
            assert!(whole
                .pixels()
                .iter()
                .any(|p| p.blue() == 255 && p.red() < 255));
            for tile in [7, 16] {
                let tiled = shadow_in(120, 100, &s, xf, tile, 10_000);
                assert_eq!(differing(&tiled, &whole), [], "tile {tile}, {xf:?}");
            }
        }
    }

    #[test]
    fn rotated_shadows_blur_in_one_scratch_until_it_would_exceed_its_bound() {
        let (s, xf) = (
            silhouette(15.3, 20.7),
            Transform::from_rotate_at(20.0, 60.0, 50.0),
        );
        let whole = shadow_in(120, 100, &s, xf, 10_000, 10_000);
        // Under the bound the tile size does not matter: one scratch.
        assert_eq!(
            differing(&shadow_in(120, 100, &s, xf, 7, 10_000), &whole),
            []
        );
        // Past it the visible part is tiled: only pixels along the rotated seams may be
        // covered or sampled by the neighbouring tile.
        let tiled = shadow_in(120, 100, &s, xf, 16, 16);
        let seams = differing(&tiled, &whole);
        assert!(!seams.is_empty() && seams.len() < 40, "{}", seams.len());
    }

    #[test]
    fn a_shadow_cropped_to_the_canvas_keeps_its_visible_pixels() {
        // Off the left/top edges and past the right/bottom ones: the canvas shows the same
        // pixels as the corresponding region of a canvas large enough for all of it.
        for (x, y, dx, dy) in [
            (-50.0, -30.0, 0, 0),
            (30.0, 25.0, 0, 0),
            (-50.0, 25.0, 0, 0),
        ] {
            let big = shadow_on(
                260,
                220,
                &silhouette(x + 100.0, y + 100.0),
                Transform::identity(),
            );
            let small = shadow_on(60, 50, &silhouette(x, y), Transform::identity());
            for py in 0..50 {
                for px in 0..60 {
                    assert_eq!(
                        small.pixel(px, py),
                        big.pixel(px + 100 + dx, py + 100 + dy),
                        "({x},{y}) at ({px},{py})"
                    );
                }
            }
        }
        // Entirely off the canvas, or under a singular transform: nothing.
        let off = shadow_on(60, 50, &silhouette(500.0, 0.0), Transform::identity());
        assert!(off
            .pixels()
            .iter()
            .all(|p| p.red() == 255 && p.blue() == 255));
        // A zero scale and a singular skew (rows 1 2 / 2 4) have no inverse: nothing.
        for flat in [
            Transform::from_scale(0.0, 1.0),
            Transform::from_row(1.0, 2.0, 2.0, 4.0, 0.0, 0.0),
        ] {
            assert!(
                flat.invert().is_none() || flat.is_scale_translate(),
                "{flat:?}"
            );
            let pm = shadow_on(60, 50, &silhouette(5.0, 5.0), flat);
            assert!(
                pm.pixels()
                    .iter()
                    .all(|p| p.red() == 255 && p.green() == 255),
                "{flat:?}"
            );
        }
        assert!(Transform::from_row(1.0, 2.0, 2.0, 4.0, 0.0, 0.0)
            .invert()
            .is_none());
    }

    #[test]
    fn shadow_radii_grow_by_the_spread_scaled_below_it() {
        let cases = [
            ((0.0, 8.0), 0.0),
            ((2.0, 8.0), 6.625),
            ((4.0, 8.0), 11.0),
            ((3.0, 3.0), 6.0),
            ((10.0, 6.0), 16.0),
            ((4.0, 3.0), 7.0),
            ((5.0, -2.0), 3.0),
            ((1.0, -3.0), 0.0),
            ((0.0, 0.0), 0.0),
            ((6.0, 0.0), 6.0),
        ];
        for ((r, spread), want) in cases {
            assert!(
                (crate::rrect::shadow_radius(r, spread) - want).abs() < 1e-5,
                "{r} + {spread}"
            );
        }
    }

    #[test]
    fn shadow_blur_caps_at_256px() {
        let blur = |b: u32| {
            shadow_blur(&BoxShadow {
                offset_x: 0,
                offset_y: 0,
                blur: b,
                spread: 0,
                color: RED,
                inset: false,
            })
        };
        assert_eq!(
            [
                blur(0),
                blur(10),
                blur(255),
                blur(256),
                blur(257),
                blur(u32::MAX)
            ],
            [0.0, 10.0, 255.0, 256.0, 256.0, 256.0]
        );
    }

    #[test]
    fn inset_and_transparent_shadows_paint_nothing_around_the_box() {
        // An inset shadow paints only inside the box (here its hole is offset wholly out of
        // it, so the box is the shadow colour), and a transparent one is nothing; an opaque
        // outer one shows below the box.
        let with_shadow = |inset: bool, a: u8| {
            with_box(plain(10.0, 10.0, 20.0, 10.0, Some(RED)), |_, sh, _, _| {
                *sh = Some(BoxShadow {
                    offset_x: 0,
                    offset_y: 10,
                    blur: 0,
                    spread: 0,
                    color: Rgba::new(0, 0, 255, a),
                    inset,
                });
            })
        };
        let images = HashMap::new();
        assert_eq!(
            rgb(&render(&with_shadow(false, 255), 40, 40, &images), 20, 25),
            [0, 0, 255]
        );
        for (inset, a, inside) in [
            (true, 255, [0, 0, 255]),
            (false, 0, [255, 0, 0]),
            (true, 0, [255, 0, 0]),
        ] {
            let pm = render(&with_shadow(inset, a), 40, 40, &images);
            assert_eq!(
                rgb(&pm, 20, 25),
                [255, 255, 255],
                "inset {inset}, alpha {a}"
            );
            assert_eq!(rgb(&pm, 20, 15), inside, "the box");
        }
        // Its layers cover the padding box's part of the canvas: a box partly off the
        // canvas paints its visible part, one wholly off it (or with no padding box)
        // paints nothing.
        let mut partly = with_shadow(true, 255);
        partly.x = -15.0;
        let pm = render(&partly, 40, 40, &images);
        assert_eq!(rgb(&pm, 2, 15), [0, 0, 255]);
        assert_eq!(rgb(&pm, 6, 15), [255, 255, 255]);
        for (x, w) in [(50.0, 20.0), (10.0, 0.0)] {
            let mut off = with_shadow(true, 255);
            (off.x, off.width) = (x, w);
            assert!(
                all_white(&render(&off, 40, 40, &images)),
                "x {x}, width {w}"
            );
        }
    }

    #[test]
    fn a_radius_of_half_a_pixel_or_less_paints_square_corners() {
        let corner = |radius: f32| {
            let f = with_box(plain(0.0, 0.0, 10.0, 10.0, Some(RED)), |r, _, _, _| {
                *r = radius;
            });
            render(&f, 10, 10, &HashMap::new()).pixel(0, 0).unwrap()
        };
        for square in [0.0, 0.5] {
            let p = corner(square);
            assert_eq!([p.red(), p.green(), p.blue()], [255, 0, 0], "{square}");
        }
        let round = corner(0.6);
        assert!(
            round.green() > 0 && round.green() < 255,
            "a rounded corner is anti-aliased"
        );
    }

    #[test]
    fn rects_two_pixels_wide_or_more_keep_exact_edge_coverage() {
        // From 2px wide (an interior pixel exists) the optimized AA rect scan runs: its edge
        // coverage is the exact area (0.7 and 0.3 of a pixel here), where the general path
        // scan would quantize it to quarters (192 / 64).
        let row = |x: f32, w: f32| {
            let mut pm = Pixmap::new(10, 3).unwrap();
            fill_rect(&mut pm, x, 0.0, w, 3.0, Rgba::BLACK, Transform::identity());
            (0..5)
                .map(|i| pm.pixel(i, 1).unwrap().alpha())
                .collect::<Vec<_>>()
        };
        assert_eq!(row(0.3, 2.0), [179, 255, 77, 0, 0]);
        assert_eq!(row(1.25, 3.5), [0, 192, 255, 255, 192]);
        assert_eq!(row(0.0, 2.0), [255, 255, 0, 0, 0]);
    }

    #[test]
    fn rounded_rects_follow_their_corner_circles() {
        // A 60 x 40 box at (5, 3) with 15px corners: on each corner's 45-degree diagonal a
        // point 2px inside the arc is filled and one 2px outside is not; the straight edges
        // are full and just outside them is empty.
        let mut pm = Pixmap::new(70, 50).unwrap();
        let id = Transform::identity();
        fill_round_rect(&mut pm, 5.0, 3.0, 60.0, 40.0, 15.0, RED, id);
        let alpha = |x: f32, y: f32| pm.pixel(x as u32, y as u32).unwrap().alpha();
        let d = std::f32::consts::FRAC_1_SQRT_2;
        for (cx, cy, sx, sy) in [
            (20.0, 18.0, -1.0, -1.0),
            (50.0, 18.0, 1.0, -1.0),
            (50.0, 28.0, 1.0, 1.0),
            (20.0, 28.0, -1.0, 1.0),
        ] {
            let at = |r: f32| (cx + sx * r * d, cy + sy * r * d);
            let (ix, iy) = at(13.0);
            let (ox, oy) = at(17.0);
            assert_eq!(alpha(ix, iy), 255, "inside the ({cx},{cy}) corner");
            assert_eq!(alpha(ox, oy), 0, "outside the ({cx},{cy}) corner");
        }
        for (x, y) in [
            (35.0, 3.0),
            (35.0, 42.0),
            (5.0, 23.0),
            (64.0, 23.0),
            (35.0, 23.0),
        ] {
            assert_eq!(alpha(x, y), 255, "edge ({x},{y})");
        }
        for (x, y) in [
            (35.0, 2.0),
            (35.0, 43.0),
            (4.0, 23.0),
            (65.0, 23.0),
            (6.0, 4.0),
        ] {
            assert_eq!(alpha(x, y), 0, "outside ({x},{y})");
        }
        // A radius past half the shorter side is clamped to it: a 20px-high pill.
        let mut pill = Pixmap::new(40, 20).unwrap();
        fill_round_rect(
            &mut pill,
            0.0,
            0.0,
            40.0,
            20.0,
            100.0,
            RED,
            Transform::identity(),
        );
        let a = |x: u32, y: u32| pill.pixel(x, y).unwrap().alpha();
        assert_eq!(
            (a(20, 0), a(20, 19), a(0, 10), a(39, 10)),
            (255, 255, 255, 255)
        );
        assert_eq!((a(0, 0), a(39, 0), a(0, 19), a(39, 19)), (0, 0, 0, 0));
        assert_eq!((a(1, 2), a(38, 17)), (0, 0), "outside the end caps");
    }

    #[test]
    fn gradients_without_a_paintable_line_paint_nothing() {
        let stops = |n: usize| {
            [RED, Rgba::new(0, 0, 255, 255)]
                .into_iter()
                .take(n)
                .enumerate()
                .map(|(i, color)| GradientStop {
                    color,
                    pos: i as f32,
                })
                .collect()
        };
        let boxed = |w: f32, angle_deg: f32, n: usize| {
            with_box(plain(0.0, 0.0, w, 10.0, None), |_, _, g, _| {
                *g = Some(LinearGradient {
                    angle_deg,
                    stops: stops(n),
                });
            })
        };
        let images = HashMap::new();
        assert!(
            all_white(&render(&boxed(0.4, 90.0, 2), 10, 10, &images)),
            "too narrow"
        );
        assert!(
            all_white(&render(&boxed(10.0, 90.0, 1), 10, 10, &images)),
            "one stop"
        );
        assert!(
            all_white(&render(&boxed(10.0, f32::NAN, 2), 10, 10, &images)),
            "no angle"
        );
        let painted = render(&boxed(10.0, 90.0, 2), 10, 10, &images);
        assert!(rgb(&painted, 0, 5)[0] > 200 && rgb(&painted, 9, 5)[2] > 200);
    }
}

/// Exact-output checks of the painters against independent references (P5-cov mutation
/// pass): each reference draws the expected result with tiny-skia primitives directly.
#[cfg(test)]
mod exact_tests {
    use super::*;
    use std::collections::HashMap;
    use turbo_dom_layout::{FontRegistry, GradientStop, NodeId};

    const BLUE: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    const RED: Rgba = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    fn white(w: u32, h: u32) -> Pixmap {
        let mut pm = Pixmap::new(w, h).unwrap();
        pm.fill(tiny_skia::Color::WHITE);
        pm
    }

    fn boxed(x: f32, y: f32, w: f32, h: f32, radius: f32, shadow: Option<BoxShadow>) -> Fragment {
        Fragment::new(
            NodeId(0),
            x,
            y,
            w,
            h,
            FragmentContent::Box {
                background: Some(RED),
                border: BorderEdges::default(),
                border_radius: radius.into(),
                shadow,
                gradient: None,
                transform: None,
            },
        )
    }

    fn drawn(f: &Fragment, w: u32, h: u32) -> Pixmap {
        let mut pm = white(w, h);
        paint_fragment(&mut pm, f, &HashMap::new(), Transform::identity());
        pm
    }

    fn hard(offset_x: i32, offset_y: i32, spread: i32) -> BoxShadow {
        BoxShadow {
            offset_x,
            offset_y,
            blur: 0,
            spread,
            color: BLUE,
            inset: false,
        }
    }

    #[test]
    fn unblurred_shadows_are_the_offset_spread_silhouette() {
        let id = Transform::identity();
        // Offset (5, 7), spread 3: the shadow is (10+5-3, 10+7-3) sized (20+6) x (10+6).
        let mut want = white(50, 40);
        fill_round_rect(&mut want, 12.0, 14.0, 26.0, 16.0, 0.0, BLUE, id);
        fill_rect(&mut want, 10.0, 10.0, 20.0, 10.0, RED, id);
        let got = drawn(
            &boxed(10.0, 10.0, 20.0, 10.0, 0.0, Some(hard(5, 7, 3))),
            50,
            40,
        );
        for y in 0..40 {
            for x in 0..50 {
                if got.pixel(x, y) != want.pixel(x, y) {
                    eprintln!("ZZ {x} {y} {:?} {:?}", got.pixel(x, y), want.pixel(x, y));
                }
            }
        }
        assert_eq!(got.data(), want.data());
        // A negative spread shrinks it; a rounded box's shadow radius grows by the spread.
        let mut want = white(50, 40);
        fill_round_rect(&mut want, 17.0, 19.0, 16.0, 6.0, 0.0, BLUE, id);
        fill_rect(&mut want, 10.0, 10.0, 20.0, 10.0, RED, id);
        assert_eq!(
            drawn(
                &boxed(10.0, 10.0, 20.0, 10.0, 0.0, Some(hard(5, 7, -2))),
                50,
                40
            )
            .data(),
            want.data()
        );
        let mut want = white(50, 40);
        fill_round_rect(&mut want, 7.0, 7.0, 26.0, 16.0, 7.0, BLUE, id);
        fill_round_rect(&mut want, 10.0, 10.0, 20.0, 10.0, 4.0, RED, id);
        assert_eq!(
            drawn(
                &boxed(10.0, 10.0, 20.0, 10.0, 4.0, Some(hard(0, 0, 3))),
                50,
                40
            )
            .data(),
            want.data()
        );
    }

    #[test]
    fn a_half_pixel_wide_or_high_shadow_still_paints() {
        let id = Transform::identity();
        for (w, h) in [(0.5, 10.0), (10.0, 0.5)] {
            let mut f = boxed(10.0, 10.0, w, h, 0.0, Some(hard(0, 15, 0)));
            if let FragmentContent::Box { background, .. } = &mut f.content {
                *background = None;
            }
            let mut want = white(40, 40);
            fill_round_rect(&mut want, 10.0, 25.0, w, h, 0.0, BLUE, id);
            let got = drawn(&f, 40, 40);
            assert_eq!(got.data(), want.data(), "{w} x {h}");
            assert!(got.pixels().iter().any(|p| p.red() < 255), "{w} x {h}");
        }
    }

    /// The shadow algorithm before tiling: one scratch padded by the blur, filled, blurred
    /// and drawn whole.
    fn reference_shadow(pm: &mut Pixmap, s: &Silhouette, blur: f32, xf: Transform) {
        let margin = blur.ceil() as i32;
        let tw = s.width.ceil() as i32 + 2 * margin;
        let th = s.height.ceil() as i32 + 2 * margin;
        let mut tmp = Pixmap::new(tw as u32, th as u32).unwrap();
        let m = margin as f32;
        fill_round_rect(
            &mut tmp,
            m,
            m,
            s.width,
            s.height,
            s.radii.top_left.h,
            s.color,
            Transform::identity(),
        );
        box_blur(&mut tmp, (blur / 2.0).round().max(1.0) as usize);
        let place = xf.pre_concat(Transform::from_translate(
            (s.x as i32 - margin) as f32,
            (s.y as i32 - margin) as f32,
        ));
        pm.draw_pixmap(0, 0, tmp.as_ref(), &PixmapPaint::default(), place, None);
    }

    #[test]
    fn blurred_shadows_equal_one_whole_scratch_on_and_off_the_canvas() {
        let shadow = |x: f32, y: f32, width: f32, height: f32| Silhouette {
            x,
            y,
            width,
            height,
            radii: BorderRadii::uniform(5.0),
            color: Rgba::new(0, 0, 255, 220),
        };
        for (s, xf, blur) in [
            (shadow(20.0, 15.0, 40.0, 25.0), Transform::identity(), 8.0),
            (shadow(20.6, 15.4, 40.0, 25.0), Transform::identity(), 9.0),
            (shadow(-30.0, -20.0, 60.0, 50.0), Transform::identity(), 8.0),
            (shadow(50.0, 40.0, 90.0, 70.0), Transform::identity(), 6.0),
            (shadow(-80.0, 30.0, 300.0, 20.0), Transform::identity(), 4.0),
            (
                shadow(10.0, 5.0, 30.0, 20.0),
                Transform::from_row(1.5, 0.0, 0.0, 0.75, 3.0, -2.0),
                8.0,
            ),
            (
                shadow(10.0, 5.0, 30.0, 20.0),
                Transform::from_rotate_at(15.0, 40.0, 30.0),
                8.0,
            ),
        ] {
            let mut want = white(100, 80);
            reference_shadow(&mut want, &s, blur, xf);
            for (tile, max_scratch) in [(SHADOW_TILE, MAX_SHADOW_SCRATCH), (9, 10_000)] {
                let mut got = white(100, 80);
                blurred_shadow(&mut got, &s, blur, xf, tile, max_scratch);
                let diff = (0..80u32)
                    .flat_map(|y| (0..100u32).map(move |x| (x, y)))
                    .filter(|&(x, y)| got.pixel(x, y) != want.pixel(x, y))
                    .count();
                assert_eq!(diff, 0, "({}, {}) {xf:?} blur {blur} tile {tile}", s.x, s.y);
            }
        }
    }

    fn gradient_box(x: f32, y: f32, w: f32, h: f32, angle_deg: f32, radius: f32) -> Fragment {
        let mut f = boxed(x, y, w, h, radius, None);
        if let FragmentContent::Box {
            background,
            gradient,
            ..
        } = &mut f.content
        {
            *background = None;
            *gradient = Some(CssGradient {
                angle_deg,
                stops: vec![
                    GradientStop {
                        color: Rgba::new(0, 0, 0, 255),
                        pos: 0.0,
                    },
                    GradientStop {
                        color: Rgba::new(255, 255, 255, 255),
                        pos: 1.0,
                    },
                ],
            });
        }
        f
    }

    #[test]
    fn gradients_follow_the_css_gradient_line() {
        // CSS Images 3 §3.1: for angle θ the line runs through the box centre along
        // (sin θ, -cos θ), half its length |w sin θ| + |h cos θ| over 2; a point's position is
        // its projection. Black -> white, so the grey level is the position.
        for (angle, x, y) in [
            (30.0_f32, 8.0_f32, 6.0_f32),
            (200.0, 3.0, 2.0),
            (90.0, 5.0, 9.0),
        ] {
            let (w, h) = (60.0_f32, 30.0_f32);
            let pm = drawn(&gradient_box(x, y, w, h, angle, 0.0), 80, 50);
            let t = angle.to_radians();
            let (dx, dy) = (t.sin(), -t.cos());
            let half = (w * t.sin().abs() + h * t.cos().abs()) / 2.0;
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            for (px, py) in [
                (x + 2.0, y + 2.0),
                (x + w - 3.0, y + h - 3.0),
                (x + 17.0, y + 11.0),
                (x + 45.0, y + 4.0),
            ] {
                let (sx, sy) = (px.floor() + 0.5, py.floor() + 0.5);
                let pos = (((sx - cx) * dx + (sy - cy) * dy) / (2.0 * half) + 0.5).clamp(0.0, 1.0);
                let want = (pos * 255.0).round();
                let got = f32::from(pm.pixel(px as u32, py as u32).unwrap().red());
                assert!(
                    (got - want).abs() <= 2.0,
                    "{angle} deg at ({px},{py}): {got} vs {want}"
                );
            }
        }
        // Half a pixel wide or high is still painted; rounded gradients clip to the radius.
        // (The line runs along the long side, so the half-covered pixels are mid-grey.)
        for (w, h, angle) in [(0.5, 10.0, 180.0), (10.0, 0.5, 90.0)] {
            let pm = drawn(&gradient_box(2.0, 2.0, w, h, angle, 0.0), 20, 20);
            assert!(pm.pixels().iter().any(|p| p.red() < 200), "{w} x {h}");
        }
        let round = drawn(&gradient_box(0.0, 0.0, 40.0, 40.0, 180.0, 20.0), 40, 40);
        assert_eq!(
            round.pixel(1, 1).unwrap().red(),
            255,
            "outside the circle: white"
        );
        assert!(round.pixel(20, 20).unwrap().red() < 200, "inside");
        let corner = |radius: f32| {
            drawn(&gradient_box(0.0, 0.0, 40.0, 40.0, 180.0, radius), 40, 40)
                .pixel(0, 0)
                .unwrap()
                .red()
        };
        assert!(corner(0.5) < 30, "a 0.5 radius is a square gradient");
        assert_eq!(corner(0.6), corner(0.6));
    }

    #[test]
    fn rounded_borders_paint_each_side_in_its_own_width_and_colour() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let mut f = boxed(10.0, 10.0, 40.0, 30.0, 8.0, None);
        if let FragmentContent::Box {
            border, background, ..
        } = &mut f.content
        {
            *background = Some(Rgba::new(0, 255, 0, 255));
            *border = BorderEdges {
                top: side(2, Some(RED)),
                right: side(4, Some(BLUE)),
                bottom: side(1, Some(RED)),
                left: side(0, Some(RED)),
            };
        }
        let pm = drawn(&f, 60, 50);
        let rgb = |x: u32, y: u32| {
            let p = pm.pixel(x, y).unwrap();
            [p.red(), p.green(), p.blue()]
        };
        // Each straight edge is its own side's band (miaskiewicz/turbo-dom#253); the
        // zero-width left side paints nothing over the fill.
        for (x, y) in [(30, 10), (30, 11), (30, 39)] {
            assert_eq!(rgb(x, y), [255, 0, 0], "red band ({x},{y})");
        }
        for (x, y) in [(49, 25), (46, 25)] {
            assert_eq!(rgb(x, y), [0, 0, 255], "blue band ({x},{y})");
        }
        for (x, y) in [(10, 25), (45, 25), (30, 12), (30, 38)] {
            assert_eq!(rgb(x, y), [0, 255, 0], "fill ({x},{y})");
        }
        for (x, y) in [(9, 25), (50, 25), (30, 9), (30, 40), (10, 10), (49, 39)] {
            assert_eq!(rgb(x, y), [255, 255, 255], "outside ({x},{y})");
        }
        // Around the top-right corner the inner edge follows the inner radii (8 less the
        // 4px right and 2px top sides): fill just inside it, the blue band on the
        // right half of the corner and red on the top half.
        assert_eq!(rgb(43, 13), [0, 255, 0]);
        assert_eq!(rgb(47, 15), [0, 0, 255]);
        assert_eq!(rgb(41, 11), [255, 0, 0]);
        // Zero-width sides draw no hairline.
        let mut none = boxed(10.0, 10.0, 40.0, 30.0, 8.0, None);
        if let FragmentContent::Box { border, .. } = &mut none.content {
            *border = BorderEdges {
                top: side(0, Some(BLUE)),
                right: side(0, Some(BLUE)),
                bottom: side(0, Some(BLUE)),
                left: side(0, Some(BLUE)),
            };
        }
        let mut want = white(60, 50);
        fill_round_rect(
            &mut want,
            10.0,
            10.0,
            40.0,
            30.0,
            8.0,
            RED,
            Transform::identity(),
        );
        assert_eq!(drawn(&none, 60, 50).data(), want.data());
    }

    /// The glyph outlines filled straight from ttf-parser into a tiny-skia path.
    struct Direct(PathBuilder, f32, f32, f32);

    impl ttf_parser::OutlineBuilder for Direct {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.move_to(self.1 + x * self.3, self.2 - y * self.3);
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.line_to(self.1 + x * self.3, self.2 - y * self.3);
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            let (o, b, s) = (self.1, self.2, self.3);
            self.0.quad_to(o + cx * s, b - cy * s, o + x * s, b - y * s);
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            let (o, b, s) = (self.1, self.2, self.3);
            self.0.cubic_to(
                o + ax * s,
                b - ay * s,
                o + bx * s,
                b - by * s,
                o + x * s,
                b - y * s,
            );
        }
        fn close(&mut self) {
            self.0.close();
        }
    }

    #[test]
    fn text_lines_fill_each_glyph_outline_at_its_pen_position() {
        let reg = FontRegistry::new();
        for family in ["Inter", "Liberation Serif"] {
            let face = reg.select(&[family], 400, false).unwrap().clone();
            let ttf = crate::glyph::outline_face(&face);
            let size = 30.0;
            let scale = size / f32::from(face.units_per_em());
            let mut x = 0.0;
            let glyphs: Vec<PositionedGlyph> = face
                .shape("Ag%")
                .iter()
                .map(|g| {
                    let pg = PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x,
                        y: 25.0,
                        text_start: 0,
                        text_end: 0,
                    };
                    x += g.x_advance as f32 * scale;
                    pg
                })
                .collect();
            let line = Fragment::new(
                NodeId(0),
                7.0,
                4.0,
                x,
                30.0,
                FragmentContent::TextLine {
                    glyphs: glyphs.clone(),
                    text: String::new(),
                    face: face.clone(),
                    font_size: size,
                    color: BLUE,
                    vertical: false,
                    upright: false,
                },
            );
            let got = drawn(&line, 90, 45);
            let mut want = white(90, 45);
            for g in &glyphs {
                let mut d = Direct(PathBuilder::new(), 7.0 + g.x, 4.0 + g.y, scale);
                ttf.outline_glyph(ttf_parser::GlyphId(g.glyph_id), &mut d);
                if let Some(path) = d.0.finish() {
                    want.fill_path(
                        &path,
                        &solid(BLUE),
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            assert!(
                got.pixels().iter().filter(|p| p.red() < 128).count() > 100,
                "{family}"
            );
            // Chrome's glyphs are the outline grown a fraction of a pixel (#266): every
            // pixel a full pixel inside the bare outline is solid ink, and every pixel a
            // full pixel clear of it stays white.
            let ink = |pm: &Pixmap, x: i32, y: i32| {
                pm.pixel(x.clamp(0, 89) as u32, y.clamp(0, 44) as u32)
                    .map_or(0, |p| 255 - p.red())
            };
            for y in 0..45 {
                for x in 0..90 {
                    let got_ink = ink(&got, x, y);
                    let around = || (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)));
                    if around().all(|(dx, dy)| ink(&want, x + dx, y + dy) == 255) {
                        assert!(got_ink >= 250, "{family} ({x},{y}) {got_ink}");
                    }
                    if around().all(|(dx, dy)| ink(&want, x + dx, y + dy) == 0) {
                        assert_eq!(got_ink, 0, "{family} ({x},{y})");
                    }
                }
            }
        }
    }

    /// A 20x20 box at (10, 10) clipping its children to `clip` (relative region).
    fn clipping(clip: turbo_dom_layout::OverflowClip, children: Vec<Fragment>) -> Fragment {
        let mut f = boxed(10.0, 10.0, 20.0, 20.0, 0.0, None);
        if let FragmentContent::Box { background, .. } = &mut f.content {
            *background = None;
        }
        f.overflow_clip = Some(clip);
        f.children = children;
        f
    }

    fn region(x: f32, y: f32, w: f32, h: f32, r: f32) -> turbo_dom_layout::OverflowClip {
        turbo_dom_layout::OverflowClip {
            x,
            y,
            width: w,
            height: h,
            radii: r.into(),
        }
    }

    #[test]
    fn overflow_clips_children_to_the_rounded_padding_box() {
        let id = Transform::identity();
        // A child overflowing a square region is cut at it (miaskiewicz/turbo-dom#253).
        let got = drawn(
            &clipping(
                region(0.0, 0.0, 20.0, 20.0, 0.0),
                vec![boxed(0.0, 0.0, 50.0, 50.0, 0.0, None)],
            ),
            50,
            50,
        );
        let mut want = white(50, 50);
        fill_rect(&mut want, 10.0, 10.0, 20.0, 20.0, RED, id);
        assert_eq!(got.data(), want.data());
        // A rounded region cuts the corners: the child paints the region's rounded shape.
        let got = drawn(
            &clipping(
                region(0.0, 0.0, 20.0, 20.0, 10.0),
                vec![boxed(0.0, 0.0, 50.0, 50.0, 0.0, None)],
            ),
            50,
            50,
        );
        assert_eq!(got.pixel(11, 11).unwrap().red(), 255, "corner cut");
        assert_eq!(got.pixel(11, 11).unwrap().green(), 255, "corner cut");
        assert_eq!(got.pixel(20, 20).unwrap().green(), 0, "inside");
        // An empty region paints no child.
        let got = drawn(
            &clipping(
                region(0.0, 0.0, 0.0, 20.0, 0.0),
                vec![boxed(0.0, 0.0, 50.0, 50.0, 0.0, None)],
            ),
            50,
            50,
        );
        assert_eq!(got.data(), white(50, 50).data());
        // A rounded region off the canvas paints nothing.
        let got = drawn(
            &clipping(
                region(0.0, 0.0, 20.0, 20.0, 5.0),
                vec![boxed(0.0, 0.0, 50.0, 50.0, 0.0, None)],
            ),
            8,
            8,
        );
        assert_eq!(got.data(), white(8, 8).data());
        // Children inside a square region paint directly, whatever their content (a
        // marker child too), exactly as unclipped.
        let inside = vec![
            boxed(12.0, 12.0, 5.0, 5.0, 0.0, None),
            Fragment::new(
                NodeId(1),
                12.0,
                20.0,
                5.0,
                5.0,
                FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                    id: 1,
                    inline: false,
                }),
            ),
        ];
        let mut unclipped = clipping(region(0.0, 0.0, 20.0, 20.0, 0.0), inside.clone());
        unclipped.overflow_clip = None;
        assert_eq!(
            drawn(&clipping(region(0.0, 0.0, 20.0, 20.0, 0.0), inside), 50, 50).data(),
            drawn(&unclipped, 50, 50).data()
        );
        // A shadowed or transformed child may paint past its box: it is clipped.
        let shadowed = boxed(12.0, 12.0, 5.0, 5.0, 0.0, Some(hard(30, 0, 0)));
        let got = drawn(
            &clipping(region(0.0, 0.0, 20.0, 20.0, 0.0), vec![shadowed]),
            50,
            50,
        );
        assert_eq!(
            got.pixel(44, 14).unwrap().blue(),
            255,
            "shadow clipped away"
        );
        assert_eq!(got.pixel(44, 14).unwrap().red(), 255, "shadow clipped away");
    }

    /// A `ColumnRule` fragment with `(style, color)` at `(x, y) size (w, h)`.
    fn rule(x: f32, y: f32, w: f32, h: f32, style: ColumnRuleStyle, color: Rgba) -> Fragment {
        Fragment::new(
            NodeId(0),
            x,
            y,
            w,
            h,
            FragmentContent::ColumnRule { style, color },
        )
    }

    /// Returns whether `pm` is a white rectangle (the canvas `white` fills).
    fn is_blank(pm: &Pixmap) -> bool {
        pm.pixels()
            .iter()
            .all(|p| [p.red(), p.green(), p.blue(), p.alpha()] == [255, 255, 255, 255])
    }

    // miaskiewicz/turbo-dom#398 PR-D: `paint_column_rule` must early-return on
    // a transparent colour or a zero-sized rect — without the guard the Dotted
    // and Dashed helpers would loop forever on a zero width. Each sub-case
    // paints over white and asserts the pixmap is unchanged.
    #[test]
    fn paint_column_rule_is_a_no_op_on_degenerate_fragments() {
        for style in [
            ColumnRuleStyle::Solid,
            ColumnRuleStyle::Dotted,
            ColumnRuleStyle::Dashed,
            ColumnRuleStyle::Double,
        ] {
            // Transparent colour: every opaque style skips painting.
            let transparent = Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 0,
            };
            assert!(
                is_blank(&drawn(
                    &rule(1.0, 1.0, 4.0, 8.0, style, transparent),
                    10,
                    10
                )),
                "alpha 0 must paint nothing ({style:?})"
            );
            // Zero width.
            assert!(
                is_blank(&drawn(&rule(1.0, 1.0, 0.0, 8.0, style, RED), 10, 10)),
                "zero width must paint nothing ({style:?})"
            );
            // Zero height.
            assert!(
                is_blank(&drawn(&rule(1.0, 1.0, 4.0, 0.0, style, RED), 10, 10)),
                "zero height must paint nothing ({style:?})"
            );
        }
    }

    // `Double` below 3 px wide collapses to a single solid band (CSS Backgrounds
    // 3 §4.2: a thin double still shows). A mutation that flipped `< 3.0` to
    // `>= 3.0` or dropped the `return` would paint the two-band version, which
    // leaves a hollow centre a `band = 1` thick (so the middle column at a 2 px
    // rule would be blank) — the test asserts the full rect is filled.
    #[test]
    fn paint_column_rule_thin_double_paints_a_single_solid_band() {
        let got = drawn(
            &rule(1.0, 1.0, 2.0, 6.0, ColumnRuleStyle::Double, RED),
            10,
            10,
        );
        for y in 1..7 {
            for x in 1..3 {
                let c = got.pixel(x, y).unwrap();
                assert_eq!(
                    [c.red(), c.green(), c.blue()],
                    [255, 0, 0],
                    "rule pixel ({x}, {y}) stays red"
                );
            }
        }
        // The band stays inside its box: outside it stays white.
        for (x, y) in [(0, 2), (3, 2), (2, 0), (2, 7)] {
            let c = got.pixel(x, y).unwrap();
            assert_eq!([c.red(), c.green(), c.blue()], [255, 255, 255]);
        }
    }

    // `circle_path` returns `None` for a non-positive radius; a mutation that
    // dropped the guard would produce a degenerate cubic path that tiny-skia
    // either rejects or renders as a single filled pixel at the centre — which
    // paint_dotted_rule would then stamp at every y step on the centerline.
    // The direct test keeps the contract pinned without touching the raster
    // walk.
    #[test]
    fn circle_path_is_none_for_a_non_positive_radius() {
        assert!(circle_path(5.0, 5.0, 0.0).is_none(), "r = 0");
        assert!(circle_path(5.0, 5.0, -0.5).is_none(), "r < 0");
        assert!(circle_path(5.0, 5.0, 0.5).is_some(), "r > 0");
    }

    // Invalid-size mutations must fail a bounded test, not leave the whole
    // mutation runner waiting on a painter loop in the test process.
    pub(super) fn rule_probe_child(name: &str) -> bool {
        const CHILD: &str = "TURBO_DOM_PNG_INVALID_RULE_CHILD";
        if std::env::var(CHILD).ok().as_deref() == Some(name) {
            return true;
        }
        let filter = if name.starts_with("paint_png::") {
            name.to_owned()
        } else {
            format!("paint_png::exact_tests::{name}")
        };
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &filter, "--nocapture"])
            .env(CHILD, name)
            .stdin(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "invalid-rule child failed: {name}: {status}"
                );
                return false;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                child.wait().unwrap();
                panic!("invalid-rule helper did not finish within three seconds: {name}");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn an_overflow_clip_with_children_limits_their_ink() {
        let f = clipping(
            region(3.0, 5.0, 7.0, 9.0, 0.0),
            vec![boxed(0.0, 0.0, 40.0, 40.0, 0.0, None)],
        );
        let got = drawn(&f, 40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let expected = if (13..20).contains(&x) && (15..24).contains(&y) {
                    [255, 0, 0, 255]
                } else {
                    [255; 4]
                };
                let p = got.pixel(x, y).unwrap();
                assert_eq!(
                    [p.red(), p.green(), p.blue(), p.alpha()],
                    expected,
                    "({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn clip_offsets_are_relative_to_the_fragment_origin() {
        let mut f = boxed(10.0, 11.0, 24.0, 24.0, 0.0, None);
        f.clip = Some(ClipRect {
            x: 3.0,
            y: 5.0,
            width: 7.0,
            height: 9.0,
            radii: BorderRadii::ZERO,
        });
        let got = drawn(&f, 40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let ink = (13..20).contains(&x) && (16..25).contains(&y);
                let expected = if ink { [255, 0, 0, 255] } else { [255; 4] };
                let p = got.pixel(x, y).unwrap();
                assert_eq!(
                    [p.red(), p.green(), p.blue(), p.alpha()],
                    expected,
                    "({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn dotted_rules_keep_circle_spacing_and_exclude_a_dot_starting_at_the_end() {
        for (width, height, centers) in [
            (1.0, 4.0, vec![1.5, 3.5]),
            (2.0, 8.0, vec![2.0, 6.0]),
            (3.0, 14.0, vec![2.5, 8.5, 14.5]),
        ] {
            let mut got = white(10, 20);
            let f = Fragment::new(
                NodeId(0),
                3.0,
                1.0,
                width,
                height,
                FragmentContent::ColumnRule {
                    style: ColumnRuleStyle::Dotted,
                    color: RED,
                },
            );
            paint_dotted_rule(&mut got, &f, RED, Transform::identity());
            let mut expected = white(10, 20);
            for center in centers {
                let path =
                    PathBuilder::from_circle(3.0 + width / 2.0, center, width / 2.0).unwrap();
                expected.fill_path(
                    &path,
                    &solid(RED),
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            assert_eq!(
                got.data(),
                expected.data(),
                "width={width}, height={height}"
            );
        }
    }

    #[test]
    fn containment_accounts_for_both_origins_and_both_far_edges() {
        let region = (5.0, 10.0, 10.0, 10.0);
        for (x, y, w, h, expected) in [
            (7.0, 13.0, 3.0, 4.0, true),
            (5.0, 10.0, 10.0, 10.0, true),
            (7.0, 13.0, 9.0, 4.0, false),
            (7.0, 13.0, 3.0, 8.0, false),
            (4.0, 13.0, 3.0, 4.0, false),
            (7.0, 9.0, 3.0, 4.0, false),
            (17.0, 13.0, 3.0, 4.0, false),
            (7.0, 23.0, 3.0, 4.0, false),
        ] {
            let f = boxed(x, y, w, h, 0.0, None);
            assert_eq!(within(&f, region), expected, "box {x},{y},{w},{h}");
        }
    }

    #[test]
    fn device_span_excludes_exact_canvas_edges() {
        for (x, y, w, h) in [(10.0, 2.0, 1.0, 3.0), (2.0, 10.0, 3.0, 1.0)] {
            assert_eq!(
                device_span(Rect::from_xywh(x, y, w, h).unwrap(), 10, 10),
                None
            );
        }
        assert_eq!(
            device_span(Rect::from_xywh(9.0, 9.0, 2.0, 2.0).unwrap(), 10, 10),
            Some((9, 9, 1, 1))
        );
    }

    #[test]
    fn dashed_rules_repeat_two_width_ink_and_one_width_gap() {
        let mut pm = white(8, 22);
        let f = Fragment::new(
            NodeId(0),
            2.0,
            1.0,
            2.0,
            20.0,
            FragmentContent::ColumnRule {
                style: ColumnRuleStyle::Dashed,
                color: RED,
            },
        );
        paint_dashed_rule(&mut pm, &f, RED, Transform::identity());
        for y in 0..22 {
            for x in 0..8 {
                let ink = (2..4).contains(&x) && (1..21).contains(&y) && (y - 1) % 6 < 4;
                let expected = if ink { [255, 0, 0, 255] } else { [255; 4] };
                let pixel = pm.pixel(x, y).unwrap();
                assert_eq!(
                    [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
                    expected,
                    "({x}, {y})"
                );
            }
        }
    }

    // The `paint_dotted_rule` and `paint_dashed_rule` helpers both guard
    // `period <= 0.0` so a zero-width rule can never infinite-loop
    // (`cy += period` of 0, `dash_top += period` of 0). The caller
    // `paint_column_rule` guards `f.width <= 0.0` first, so the guards sit on
    // the function contract rather than a reachable branch via `paint`.
    // Calling the helpers directly with a zero-width fragment keeps that
    // contract — and the loop-protection — pinned against a mutation that
    // flipped the guard's `<=` to `<`.
    #[test]
    fn dotted_and_dashed_helpers_return_on_zero_width_fragments() {
        if !rule_probe_child("dotted_and_dashed_helpers_return_on_zero_width_fragments") {
            return;
        }
        let mut pm = white(10, 10);
        let id = Transform::identity();
        let f = Fragment::new(
            NodeId(0),
            1.0,
            1.0,
            0.0,
            8.0,
            FragmentContent::ColumnRule {
                style: ColumnRuleStyle::Dotted,
                color: RED,
            },
        );
        paint_dotted_rule(&mut pm, &f, RED, id);
        paint_dashed_rule(&mut pm, &f, RED, id);
        assert!(is_blank(&pm), "zero-width helpers must not paint or loop");
    }

    // v0.8.2 R1 adversarial M1 + L1: a `NaN` or `Infinity` width / height
    // slips past the `f.width <= 0.0 || f.height <= 0.0` guard because every
    // comparison with `NaN` is false and `Infinity > 0.0`. Without the
    // non-finite guard the painter would compute `cx = NaN / 2.0 = NaN`,
    // feed NaN coordinates to tiny-skia (undefined output) and — for the
    // dotted / dashed loops — compute `period = 2 * NaN = NaN` so
    // `cy += NaN` leaves `cy < end` undefined, or `period = 2 * Infinity`
    // so the loop body can still run once with NaN/Inf coordinates. The
    // guard must reject both.
    #[test]
    fn paint_column_rule_is_a_no_op_on_non_finite_fragments() {
        if !rule_probe_child("paint_column_rule_is_a_no_op_on_non_finite_fragments") {
            return;
        }
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for style in [
                ColumnRuleStyle::Solid,
                ColumnRuleStyle::Dotted,
                ColumnRuleStyle::Dashed,
                ColumnRuleStyle::Double,
            ] {
                assert!(
                    is_blank(&drawn(&rule(1.0, 1.0, bad, 8.0, style, RED), 10, 10)),
                    "non-finite width must paint nothing ({style:?}, {bad})"
                );
                assert!(
                    is_blank(&drawn(&rule(1.0, 1.0, 4.0, bad, style, RED), 10, 10)),
                    "non-finite height must paint nothing ({style:?}, {bad})"
                );
            }
        }
    }

    // The rule-painter HELPERS (`paint_dotted_rule`, `paint_dashed_rule`,
    // `paint_double_rule`, `paint_dotted_rule_stroked`) each sit on a direct
    // contract — a mutation that weakens `paint_column_rule`'s guard must
    // not reach into the helper's own math. Call each helper directly with
    // a `NaN` / `Infinity` width to pin that contract.
    #[test]
    fn rule_helpers_are_a_no_op_on_non_finite_fragments() {
        if !rule_probe_child("rule_helpers_are_a_no_op_on_non_finite_fragments") {
            return;
        }
        let id = Transform::identity();
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut pm = white(10, 10);
            let f = Fragment::new(
                NodeId(0),
                1.0,
                1.0,
                bad,
                8.0,
                FragmentContent::ColumnRule {
                    style: ColumnRuleStyle::Dotted,
                    color: RED,
                },
            );
            paint_dotted_rule(&mut pm, &f, RED, id);
            paint_dashed_rule(&mut pm, &f, RED, id);
            paint_double_rule(&mut pm, &f, RED, id);
            paint_shaded_rule(&mut pm, &f, ColumnRuleStyle::Groove, RED, id);
            paint_dotted_rule_stroked(&mut pm, 5.0, 1.0, bad, 1.0, RED, id);
            paint_dotted_rule_stroked(&mut pm, 5.0, 1.0, 8.0, bad, RED, id);
            assert!(
                is_blank(&pm),
                "non-finite width/height helpers must not paint or loop ({bad})"
            );
        }
    }

    // v0.8.2 R1 adversarial L3: `paint_dotted_rule_stroked`'s leading cap
    // stamps a `diameter`-tall rect even when the fragment itself is
    // shorter than `diameter` (e.g. `f.height = 0.3`). The cap would then
    // paint pixels past `y_top + f.height`, leaking outside the fragment.
    // The clamped version stamps `(end - y_top).min(diameter)` so the
    // over-paint vanishes. Probe: a cap at integer `y=1` with
    // `height = 0.3` paints pixel (5,1) with ~30% alpha; leaving the pixel
    // mostly white rather than full red.
    #[test]
    fn paint_dotted_rule_stroked_leading_cap_does_not_overpaint_short_fragments() {
        if !rule_probe_child(
            "paint_dotted_rule_stroked_leading_cap_does_not_overpaint_short_fragments",
        ) {
            return;
        }
        let mut pm = white(10, 10);
        let id = Transform::identity();
        // cx=5.5, y_top=1.0, height=0.3, diameter=1.0.
        paint_dotted_rule_stroked(&mut pm, 5.5, 1.0, 0.3, 1.0, RED, id);
        let c = pm.pixel(5, 1).unwrap();
        assert!(
            c.green() > 100 && c.blue() > 100,
            "pixel (5,1) must stay largely white when f.height < diameter; got rgb=({},{},{})",
            c.red(),
            c.green(),
            c.blue()
        );
        // And no pixel below the fragment (y >= 2) gains any red.
        for y in 2..10 {
            let c = pm.pixel(5, y).unwrap();
            assert_eq!(
                [c.red(), c.green(), c.blue()],
                [255, 255, 255],
                "pixel (5,{y}) must stay white"
            );
        }
    }

    #[test]
    fn glyph_ink_past_its_line_box_is_clipped_by_overflow() {
        // `overflow:hidden; height:10px; line-height:10px; font-size:40px` around "gy": the
        // line box fits the region, its glyphs' ink does not.
        let face = FontRegistry::new()
            .select(&["Inter"], 400, false)
            .unwrap()
            .clone();
        let scale = 40.0 / f32::from(face.units_per_em());
        let mut x = 0.0;
        let glyphs: Vec<PositionedGlyph> = face
            .shape("gy")
            .iter()
            .map(|g| {
                let pg = PositionedGlyph {
                    glyph_id: g.glyph_id,
                    x,
                    y: 20.0,
                    text_start: 0,
                    text_end: 0,
                };
                x += g.x_advance as f32 * scale;
                pg
            })
            .collect();
        let line = Fragment::new(
            NodeId(1),
            10.0,
            10.0,
            x.min(20.0),
            10.0,
            FragmentContent::TextLine {
                glyphs,
                text: String::new(),
                face,
                font_size: 40.0,
                color: BLUE,
                vertical: false,
                upright: false,
            },
        );
        let got = drawn(
            &clipping(region(0.0, 0.0, 20.0, 10.0, 0.0), vec![line]),
            60,
            60,
        );
        for y in 0..60 {
            for x in 0..60 {
                let inside = (10..30).contains(&x) && (10..20).contains(&y);
                let p = got.pixel(x, y).unwrap();
                if !inside {
                    assert_eq!((p.red(), p.green()), (255, 255), "ink at ({x}, {y})");
                }
            }
        }
        assert!(
            (10..30).any(|x| (10..20).any(|y| got.pixel(x, y).unwrap().red() < 128)),
            "the glyphs paint inside the region"
        );
    }

    // v0.8.2 R2 M1: the shared `fragment_finite_positive` guard the non-rule
    // painters all call. A mutation that weakens any of its four clauses
    // (`is_finite` on width / height, `> 0.0` on width / height) would let a
    // non-finite or non-positive fragment through, so pin each clause here
    // independently of the painter tests below.
    #[test]
    fn fragment_finite_positive_rejects_non_finite_and_non_positive_sizes() {
        let ok = Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            4.0,
            8.0,
            FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                id: 1,
                inline: false,
            }),
        );
        assert!(fragment_finite_positive(&ok));
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut f = ok.clone();
            f.width = bad;
            assert!(!fragment_finite_positive(&f), "non-finite width ({bad})");
            let mut f = ok.clone();
            f.height = bad;
            assert!(!fragment_finite_positive(&f), "non-finite height ({bad})");
        }
        for &non_positive in &[0.0_f32, -1.0_f32, -0.0_f32] {
            let mut f = ok.clone();
            f.width = non_positive;
            assert!(
                !fragment_finite_positive(&f),
                "non-positive width ({non_positive})"
            );
            let mut f = ok.clone();
            f.height = non_positive;
            assert!(
                !fragment_finite_positive(&f),
                "non-positive height ({non_positive})"
            );
        }
    }

    // v0.8.2 R2 M1: every public painter that reads `f.width` / `f.height`
    // must no-op on a non-finite fragment. tiny-skia's own
    // `Rect::from_xywh` / `PathBuilder` silently reject NaN / Inf inputs in
    // most paths, but `paint_box_shadow` computes `sw = f.width + 2 * spread`
    // (`Inf + Inf = Inf`, `-Inf + Inf = NaN`) before any tiny-skia call, and
    // we don't want to lean on a 3rd-party contract. Call each painter
    // directly with NaN / Infinity width (and height) and assert nothing
    // painted.
    #[test]
    fn non_rule_painters_are_a_no_op_on_non_finite_fragments() {
        use turbo_dom_layout::image::Orientation;
        let id = Transform::identity();
        let mk = |w: f32, h: f32| {
            Fragment::new(
                NodeId(0),
                1.0,
                1.0,
                w,
                h,
                FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                    id: 1,
                    inline: false,
                }),
            )
        };
        let placement = turbo_dom_layout::ImagePlacement {
            name: "missing".to_string(),
            intrinsic_w: 4,
            intrinsic_h: 4,
            has_alpha: true,
            tint: None,
            orientation: Orientation::IDENTITY,
        };
        let border = BorderEdges {
            top: BorderSide {
                width: 2,
                color: Some(RED),
            },
            right: BorderSide {
                width: 2,
                color: Some(RED),
            },
            bottom: BorderSide {
                width: 2,
                color: Some(RED),
            },
            left: BorderSide {
                width: 2,
                color: Some(RED),
            },
        };
        let shadow_cfg = BoxShadow {
            offset_x: 0,
            offset_y: 0,
            blur: 0,
            spread: 0,
            color: BLUE,
            inset: false,
        };
        let inset_shadow = BoxShadow {
            inset: true,
            ..shadow_cfg
        };
        let gradient = CssGradient {
            angle_deg: 0.0,
            stops: vec![
                GradientStop {
                    color: RED,
                    pos: 0.0,
                },
                GradientStop {
                    color: BLUE,
                    pos: 1.0,
                },
            ],
        };
        let pattern = TilePattern {
            tile: Box::new(FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                id: 1,
                inline: false,
            })),
            x: 0.0,
            y: 0.0,
            tile_width: 1.0,
            tile_height: 1.0,
            step_x: 1.0,
            step_y: 1.0,
        };
        let radii = BorderRadii::uniform(2.0);
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for f in [mk(bad, 4.0), mk(4.0, bad)] {
                let mut pm = white(20, 20);
                let images = HashMap::new();
                paint_image(&mut pm, &f, &placement, &images, id);
                paint_border(&mut pm, &f, &border, id);
                paint_box_shadow(&mut pm, &f, &BorderRadii::ZERO, &shadow_cfg, id);
                paint_inset_shadow(
                    &mut pm,
                    &f,
                    &border,
                    &BorderRadii::ZERO,
                    Some(&inset_shadow),
                    id,
                );
                paint_gradient(&mut pm, &f, &BorderRadii::ZERO, &gradient, id);
                paint_round_border(&mut pm, &f, &border, &radii, id);
                paint_tiles(&mut pm, &f, &pattern, &images, id);
                assert!(
                    is_blank(&pm),
                    "non-finite ({bad}) size {}x{} must paint nothing",
                    f.width,
                    f.height
                );
            }
        }
    }

    // v0.8.2 R2 M1: `paint_gradient` additionally rejects a gradient whose
    // stops include a non-finite position, matching the painter's documented
    // contract (a `NaN` / `Infinity` `pos` would round-trip through `clamp`
    // or feed tiny-skia's `GradientStop::new` an undefined offset). The
    // outer finite-size guard is already covered above; this test pins the
    // stop-position clause specifically.
    #[test]
    fn paint_gradient_rejects_non_finite_stop_positions() {
        let id = Transform::identity();
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut pm = white(20, 20);
            let gradient = CssGradient {
                angle_deg: 0.0,
                stops: vec![
                    GradientStop {
                        color: RED,
                        pos: bad,
                    },
                    GradientStop {
                        color: BLUE,
                        pos: 1.0,
                    },
                ],
            };
            let f = Fragment::new(
                NodeId(0),
                1.0,
                1.0,
                10.0,
                10.0,
                FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                    id: 1,
                    inline: false,
                }),
            );
            paint_gradient(&mut pm, &f, &BorderRadii::ZERO, &gradient, id);
            assert!(
                is_blank(&pm),
                "non-finite ({bad}) stop position must paint nothing"
            );
        }
    }

    // v0.8.2 raster-cov: `paint_round_border` guards `ring.finish()` against
    // returning `None` — tiny-skia's `PathBuilder::finish()` returns `None`
    // when the collected points reject a bounding rect, which happens as
    // soon as any point is non-finite. `fragment_finite_positive` keeps
    // width / height finite, but `f.x` / `f.y` are not checked (every
    // painter but this one feeds them to tiny-skia primitives that silently
    // reject non-finite inputs; the round-border ring runs through
    // `PathBuilder` so the None branch IS reachable on a non-finite
    // position). Assert the painter no-ops: without the `else { return; }`
    // the `pm.fill_path(&ring, …)` below would be fed a path that was
    // never constructed, so this test pins the guard rather than the
    // branch alone.
    #[test]
    fn paint_round_border_is_a_no_op_on_a_non_finite_position() {
        let id = Transform::identity();
        let side = BorderSide {
            width: 2,
            color: Some(RED),
        };
        let border = BorderEdges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
        let radii = BorderRadii::ZERO;
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for f in [
                Fragment::new(
                    NodeId(0),
                    bad,
                    1.0,
                    10.0,
                    10.0,
                    FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                        id: 1,
                        inline: false,
                    }),
                ),
                Fragment::new(
                    NodeId(0),
                    1.0,
                    bad,
                    10.0,
                    10.0,
                    FragmentContent::Marker(turbo_dom_layout::MarkerKind {
                        id: 1,
                        inline: false,
                    }),
                ),
            ] {
                let mut pm = white(20, 20);
                paint_round_border(&mut pm, &f, &border, &radii, id);
                assert!(
                    is_blank(&pm),
                    "non-finite ({bad}) position {},{} must paint nothing",
                    f.x,
                    f.y
                );
            }
        }
    }

    #[test]
    fn invalid_rule_geometry_never_paints_or_stalls() {
        if !rule_probe_child("invalid_rule_geometry_never_paints_or_stalls") {
            return;
        }
        let styles = [
            ColumnRuleStyle::Solid,
            ColumnRuleStyle::Dotted,
            ColumnRuleStyle::Dashed,
            ColumnRuleStyle::Double,
            ColumnRuleStyle::Groove,
            ColumnRuleStyle::Ridge,
            ColumnRuleStyle::Inset,
            ColumnRuleStyle::Outset,
        ];
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            for style in styles {
                for (width, height) in [(bad, 8.0), (4.0, bad)] {
                    let f = rule(1.0, 1.0, width, height, style, RED);
                    assert!(is_blank(&drawn(&f, 10, 10)), "{style:?}: {width}x{height}");
                }
            }
            let mut pm = white(10, 10);
            let f = rule(1.0, 1.0, 1.0, bad, ColumnRuleStyle::Dotted, RED);
            paint_dotted_rule(&mut pm, &f, RED, Transform::identity());
            assert!(is_blank(&pm), "direct dotted height {bad}");
        }
    }

    fn assert_stroked_rule_blank(cx: f32, top: f32, height: f32, diameter: f32) {
        let mut pm = white(10, 10);
        paint_dotted_rule_stroked(
            &mut pm,
            cx,
            top,
            height,
            diameter,
            RED,
            Transform::identity(),
        );
        assert!(
            is_blank(&pm),
            "stroked input {cx},{top},{height},{diameter}"
        );
    }

    #[test]
    fn invalid_stroked_geometry_never_paints_or_stalls() {
        if !rule_probe_child("invalid_stroked_geometry_never_paints_or_stalls") {
            return;
        }
        // An invalid center must reject even a long off-canvas series without
        // walking millions of individually rejected rectangles.
        assert_stroked_rule_blank(f32::INFINITY, 1.0, 1.0e9, 1.0);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_stroked_rule_blank(bad, 1.0, 8.0, 1.0);
            assert_stroked_rule_blank(5.0, bad, 8.0, 1.0);
        }
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            assert_stroked_rule_blank(5.0, 1.0, bad, 1.0);
            assert_stroked_rule_blank(5.0, 1.0, 8.0, bad);
        }
    }

    #[test]
    fn short_subpixel_dots_survive_large_origin_localization() {
        let mut expected = white(10, 10);
        paint_dotted_rule_stroked(
            &mut expected,
            5.0,
            0.0,
            0.3,
            1.0,
            RED,
            Transform::identity(),
        );
        assert!(!is_blank(&expected), "the short cap must remain visible");
        let mut actual = white(10, 10);
        paint_dotted_rule_stroked(
            &mut actual,
            5.0,
            1.0e9,
            0.3,
            1.0,
            RED,
            Transform::from_translate(0.0, -1.0e9),
        );
        assert_eq!(actual.data(), expected.data());
    }
}

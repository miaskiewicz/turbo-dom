//! Raster painter: walk a [`Fragment`] galley into a tiny-skia pixmap and encode
//! PNG. Children paint back-to-front in CSS stacking order (§9.9); decoded images
//! are scaled into their box, others fall back to a placeholder. Content past the
//! pixmap edge is clipped by tiny-skia, giving a viewport-clipped screenshot.

use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Point,
    Rect, SpreadMode, Stroke, Transform,
};
use turbo_html2pdf_core::layout::value::{BorderEdges, BorderSide};
use turbo_html2pdf_core::{
    BoxShadow, Fragment, FragmentContent, LinearGradient as CssGradient, PositionedGlyph, Rgba,
    Transform2D as BoxTransform,
};

use crate::glyph::{self, Pen, Tracer};
use crate::image_paint::DecodedAssets;

/// Neutral fill for an `<img>` box whose pixels we don't have.
const IMAGE_PLACEHOLDER: Rgba = Rgba {
    r: 220,
    g: 220,
    b: 220,
    a: 255,
};

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
    let mut pm =
        Pixmap::new(width, height).ok_or_else(|| format!("bad canvas {width}x{height}"))?;
    pm.fill(tiny_skia::Color::from_rgba8(bg.r, bg.g, bg.b, bg.a));
    paint_fragment(&mut pm, galley, images, Transform::identity());
    pm.encode_png().map_err(|e| format!("png encode: {e}"))
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
    // A CSS `transform` on this box applies to it AND its whole subtree.
    let xf = match &f.content {
        FragmentContent::Box {
            transform: Some(t), ..
        } => compose_transform(xf, f, t),
        _ => xf,
    };
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
            // card sits on it; inset shadows are v1-unsupported.
            if let Some(sh) = shadow.filter(|s| !s.inset && s.color.a > 0) {
                paint_box_shadow(pm, f, *border_radius, &sh, xf);
            }
            if *border_radius > 0.5 {
                if let Some(bg) = background {
                    fill_round_rect(pm, f.x, f.y, f.width, f.height, *border_radius, *bg, xf);
                }
                if let Some(g) = gradient {
                    paint_gradient(pm, f, *border_radius, g, xf);
                }
                paint_round_border(pm, f, border, *border_radius, xf);
            } else {
                if let Some(bg) = background {
                    fill_rect(pm, f.x, f.y, f.width, f.height, *bg, xf);
                }
                if let Some(g) = gradient {
                    paint_gradient(pm, f, 0.0, g, xf);
                }
                paint_border(pm, f, border, xf);
            }
        }
        FragmentContent::TextLine {
            glyphs,
            face,
            font_size,
            color,
        } => paint_text(
            pm,
            f,
            glyphs,
            face.data(),
            face.index(),
            face.units_per_em(),
            *font_size,
            *color,
            xf,
        ),
        FragmentContent::Image(placement) => paint_image(pm, f, placement, images, xf),
        FragmentContent::Directive(_) => {}
    }
    // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
    // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
    for &i in &f.paint_order() {
        paint_fragment(pm, &f.children[i], images, xf);
    }
}

/// Draw an image into its layout box: scale the decoded pixels and blit them;
/// fall back to a neutral placeholder when the image is absent or won't scale.
/// A `mask-image` placement (`tint` set) is stencilled: the source's alpha selects
/// where the tint colour paints (Wikipedia's monochrome SVG UI glyphs).
fn paint_image(
    pm: &mut Pixmap,
    f: &Fragment,
    placement: &turbo_html2pdf_core::ImagePlacement,
    images: &DecodedAssets,
    xf: Transform,
) {
    let (w, h) = (f.width.round() as u32, f.height.round() as u32);
    let img = images
        .get(&placement.name)
        .and_then(|d| d.scaled_pixmap(w, h));
    // Fold the box's top-left into the transform so a transformed image (a rotated
    // thumbnail, a translated slide) maps correctly; draw the pixmap at (0,0).
    let place = xf.pre_concat(Transform::from_translate(f.x.round(), f.y.round()));
    match (img, placement.tint) {
        (Some(src), Some(tint)) => {
            let stencilled = tint_pixmap(&src, tint);
            pm.draw_pixmap(
                0,
                0,
                stencilled.as_ref(),
                &PixmapPaint::default(),
                place,
                None,
            );
        }
        (Some(src), None) => {
            pm.draw_pixmap(0, 0, src.as_ref(), &PixmapPaint::default(), place, None);
        }
        // A missing mask paints nothing (no placeholder box for a glyph); a missing
        // real image shows the neutral placeholder.
        (None, Some(_)) => {}
        (None, None) => fill_rect(pm, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER, xf),
    }
}

/// Stencil `tint` through a mask pixmap: each output pixel is `tint` with its alpha
/// scaled by the source pixel's alpha, premultiplied. Where the mask is transparent
/// nothing paints; where opaque the full tint shows.
fn tint_pixmap(src: &Pixmap, tint: Rgba) -> Pixmap {
    let mut out = Pixmap::new(src.width(), src.height()).expect("mask pixmap");
    let scale = |c: u8, a: u8| ((c as u32 * a as u32) / 255) as u8;
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

/// Paint each border side as a filled edge band (approximation: no corner
/// mitring, no line styles — width + color only).
fn paint_border(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, xf: Transform) {
    let side = |s: &BorderSide| s.color.filter(|_| s.width > 0).map(|c| (s.width as f32, c));
    if let Some((w, c)) = side(&b.top) {
        fill_rect(pm, f.x, f.y, f.width, w, c, xf);
    }
    if let Some((w, c)) = side(&b.bottom) {
        fill_rect(pm, f.x, f.y + f.height - w, f.width, w, c, xf);
    }
    if let Some((w, c)) = side(&b.left) {
        fill_rect(pm, f.x, f.y, w, f.height, c, xf);
    }
    if let Some((w, c)) = side(&b.right) {
        fill_rect(pm, f.x + f.width - w, f.y, w, f.height, c, xf);
    }
}

/// A rounded-rectangle path (four cubic corner arcs, `kappa` control points).
fn round_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    const K: f32 = 0.5522847; // circle-arc cubic-Bézier control ratio
    let c = r * K;
    let (r0, b0) = (x + w, y + h); // right, bottom edges
    let mut p = PathBuilder::new();
    p.move_to(x + r, y);
    p.line_to(r0 - r, y);
    p.cubic_to(r0 - r + c, y, r0, y + r - c, r0, y + r);
    p.line_to(r0, b0 - r);
    p.cubic_to(r0, b0 - r + c, r0 - r + c, b0, r0 - r, b0);
    p.line_to(x + r, b0);
    p.cubic_to(x + r - c, b0, x, b0 - r + c, x, b0 - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + r - c, x + r - c, y, x + r, y);
    p.close();
    p.finish()
}

/// Fill a rounded rectangle (e.g. a `border-radius:50%` radio/checkbox circle).
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
    if w <= 0.0 || h <= 0.0 || c.a == 0 {
        return;
    }
    if let Some(path) = round_rect_path(x, y, w, h, r) {
        pm.fill_path(&path, &solid(c), FillRule::Winding, xf, None);
    }
}

/// Paint an outer `box-shadow` behind the box: stamp its rounded-rect silhouette
/// (the border box, offset by `(offset_x, offset_y)` and grown by `spread`) in the
/// shadow colour onto a scratch pixmap, box-blur it by `blur`, and composite it
/// under the box. A `blur` of 0 fills the silhouette directly. This is what makes
/// an overlay card/modal read as raised chrome rather than a flat rectangle.
fn paint_box_shadow(
    pm: &mut Pixmap,
    f: &Fragment,
    border_radius: f32,
    sh: &BoxShadow,
    xf: Transform,
) {
    let spread = sh.spread as f32;
    let sx = f.x + sh.offset_x as f32 - spread;
    let sy = f.y + sh.offset_y as f32 - spread;
    let sw = f.width + 2.0 * spread;
    let sh_h = f.height + 2.0 * spread;
    if sw < 0.5 || sh_h < 0.5 {
        return;
    }
    let radius = (border_radius + spread).max(0.0);
    let blur = sh.blur as f32;
    if blur < 0.5 {
        fill_round_rect(pm, sx, sy, sw, sh_h, radius, sh.color, xf);
        return;
    }
    // Scratch pixmap padded by the blur radius so the feathered edge isn't clipped.
    let margin = blur.ceil() as i32;
    let tw = sw.ceil() as i32 + 2 * margin;
    let th = sh_h.ceil() as i32 + 2 * margin;
    if !(1..=8192).contains(&tw) || !(1..=8192).contains(&th) {
        return;
    }
    let Some(mut tmp) = Pixmap::new(tw as u32, th as u32) else {
        return;
    };
    fill_round_rect(
        &mut tmp,
        margin as f32,
        margin as f32,
        sw,
        sh_h,
        radius,
        sh.color,
        Transform::identity(),
    );
    // CSS blur radius ≈ 2σ; a 3-pass box blur of radius ≈ blur/2 approximates it.
    box_blur(&mut tmp, (blur / 2.0).round().max(1.0) as usize);
    let place = xf.pre_concat(Transform::from_translate(
        (sx as i32 - margin) as f32,
        (sy as i32 - margin) as f32,
    ));
    pm.draw_pixmap(0, 0, tmp.as_ref(), &PixmapPaint::default(), place, None);
}

/// Paint a `linear-gradient(...)` background into the box (clipped to its rounded
/// rect when `border_radius > 0`). The CSS gradient-line angle (0 = to top, 90 = to
/// right, clockwise) maps to start/end points across the box: the line runs through
/// the centre, its half-length the CSS `|w·sinθ| + |h·cosθ|` projection so the 0%
/// and 100% stops land at the box's extreme corners along the line.
fn paint_gradient(
    pm: &mut Pixmap,
    f: &Fragment,
    border_radius: f32,
    g: &CssGradient,
    xf: Transform,
) {
    if f.width < 0.5 || f.height < 0.5 || g.stops.len() < 2 {
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
    if border_radius > 0.5 {
        if let Some(path) = round_rect_path(f.x, f.y, w, h, border_radius) {
            pm.fill_path(&path, &paint, FillRule::Winding, xf, None);
        }
    } else if let Some(rect) = Rect::from_xywh(f.x, f.y, w, h) {
        pm.fill_rect(rect, &paint, xf, None);
    }
}

/// Approximate a Gaussian blur with three passes of a separable box blur over the
/// pixmap's premultiplied RGBA (edges clamp-extended). Only shadow scratch buffers
/// pass through here, so the O(passes·w·h) cost is bounded by the shadow's size.
fn box_blur(pm: &mut Pixmap, radius: usize) {
    if radius == 0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let data = pm.data_mut();
    for _ in 0..3 {
        box_blur_horizontal(data, w, h, radius);
        box_blur_vertical(data, w, h, radius);
    }
}

/// One horizontal box-blur pass with a running window sum (clamp-extended edges).
fn box_blur_horizontal(data: &mut [u8], w: usize, h: usize, r: usize) {
    if w == 0 {
        return;
    }
    let win = (2 * r + 1) as u32;
    let mut src = vec![0u8; w * 4];
    for y in 0..h {
        let row = &mut data[y * w * 4..(y + 1) * w * 4];
        src.copy_from_slice(row);
        let get = |x: isize, c: usize| src[x.clamp(0, w as isize - 1) as usize * 4 + c] as u32;
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
    if h == 0 {
        return;
    }
    let win = (2 * r + 1) as u32;
    let mut src = vec![0u8; h * 4];
    for x in 0..w {
        for y in 0..h {
            let s = (y * w + x) * 4;
            src[y * 4..y * 4 + 4].copy_from_slice(&data[s..s + 4]);
        }
        let get = |y: isize, c: usize| src[y.clamp(0, h as isize - 1) as usize * 4 + c] as u32;
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

/// Stroke a box's border as a single rounded outline (uniform width/color from the
/// widest side — enough for the common uniform-radius controls). Inset by half the
/// stroke so the outline stays inside the border box.
fn paint_round_border(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, r: f32, xf: Transform) {
    let widest = [&b.top, &b.right, &b.bottom, &b.left]
        .into_iter()
        .filter(|s| s.width > 0)
        .max_by_key(|s| s.width);
    let Some((w, color)) = widest.and_then(|s| s.color.map(|c| (f32::from(s.width), c))) else {
        return;
    };
    let hw = w / 2.0;
    let Some(path) = round_rect_path(
        f.x + hw,
        f.y + hw,
        f.width - w,
        f.height - w,
        (r - hw).max(0.0),
    ) else {
        return;
    };
    let stroke = Stroke {
        width: w,
        ..Default::default()
    };
    pm.stroke_path(&path, &solid(color), &stroke, xf, None);
}

/// A tiny-skia path sink for [`glyph::trace_glyph`].
struct PathSink(PathBuilder);

impl Tracer for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to(cx, cy, x, y);
    }
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.0.cubic_to(c1x, c1y, c2x, c2y, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

#[allow(clippy::too_many_arguments)] // glyph run + face metrics + colour + transform
fn paint_text(
    pm: &mut Pixmap,
    f: &Fragment,
    glyphs: &[PositionedGlyph],
    font_bytes: &[u8],
    font_index: u32,
    units_per_em: u16,
    font_size: f32,
    color: Rgba,
    xf: Transform,
) {
    if color.a == 0 || units_per_em == 0 {
        return;
    }
    let Some(face) = glyph::parse_face(font_bytes, font_index) else {
        return;
    };
    let scale = font_size / units_per_em as f32;
    let paint = solid(color);
    for g in glyphs {
        let pen = Pen {
            origin_x: f.x + g.x,
            baseline_y: f.y + g.y,
            scale,
        };
        let mut sink = PathSink(PathBuilder::new());
        glyph::trace_glyph(&face, g.glyph_id, pen, &mut sink);
        if let Some(path) = sink.0.finish() {
            pm.fill_path(&path, &paint, FillRule::Winding, xf, None);
        }
    }
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
}

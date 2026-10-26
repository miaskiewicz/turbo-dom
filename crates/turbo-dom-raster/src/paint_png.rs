//! Raster painter: walk a [`Fragment`] galley into a tiny-skia pixmap and encode
//! PNG. Children paint back-to-front in CSS stacking order (§9.9); decoded images
//! are scaled into their box, others fall back to a placeholder. Content past the
//! pixmap edge is clipped by tiny-skia, giving a viewport-clipped screenshot.

use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint,
    Point, Rect, SpreadMode, Stroke, Transform,
};
use turbo_dom_layout::layout::value::{BorderEdges, BorderSide};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{
    BoxShadow, ClipRect, Fragment, FragmentContent, LinearGradient as CssGradient, PositionedGlyph,
    Rgba, Transform2D as BoxTransform,
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
    let path = PathBuilder::from_rect(region);
    let Some(bounds) = path.clone().transform(xf).map(|p| p.bounds()) else {
        return;
    };
    let x0 = bounds.left().floor().max(0.0) as i32;
    let y0 = bounds.top().floor().max(0.0) as i32;
    let x1 = (bounds.right().ceil() as i32).min(pm.width() as i32);
    let y1 = (bounds.bottom().ceil() as i32).min(pm.height() as i32);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // The layer is a non-empty sub-rectangle of `pm` (`x0 < x1 <= pm.width()`, same for
    // y), and tiny-skia allocates any size an existing pixmap already has.
    let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let mut layer = Pixmap::new(w, h).expect("clip layer within the canvas");
    let mut mask = Mask::new(w, h).expect("clip mask within the canvas");
    let local = Transform::from_translate(-x0 as f32, -y0 as f32).pre_concat(xf);
    paint_subtree(&mut layer, f, images, local);
    mask.fill_path(&path, FillRule::Winding, true, local);
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
        } => paint_text(pm, f, glyphs, face, *font_size, *color, xf),
        FragmentContent::Image(placement) => paint_image(pm, f, placement, images, xf),
        FragmentContent::Marker(_) => {}
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
    placement: &turbo_dom_layout::ImagePlacement,
    images: &DecodedAssets,
    xf: Transform,
) {
    let (w, h) = (f.width.round() as u32, f.height.round() as u32);
    let img = images
        .get(&placement.name)
        .and_then(|d| d.oriented(placement.orientation).scaled_pixmap(w, h));
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

/// Paint each border side as a filled edge band (approximation: no corner
/// mitring, no line styles — width + color only).
fn paint_border(pm: &mut Pixmap, f: &Fragment, b: &BorderEdges, xf: Transform) {
    let side = |s: &BorderSide| {
        s.color
            .filter(|_| s.width > 0)
            .map(|c| (f32::from(s.width), c))
    };
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
    const K: f32 = 0.552_284_7; // circle-arc cubic-Bézier control ratio
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
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
    let blur = shadow_blur(sh);
    if blur < 0.5 {
        fill_round_rect(pm, sx, sy, sw, sh_h, radius, sh.color, xf);
        return;
    }
    let silhouette = Silhouette {
        x: sx,
        y: sy,
        width: sw,
        height: sh_h,
        radius,
        color: sh.color,
    };
    blurred_shadow(pm, &silhouette, blur, xf, SHADOW_TILE, MAX_SHADOW_SCRATCH);
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
    radius: f32,
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
            fill_round_rect(
                &mut scratch,
                (margin - gx0) as f32,
                (margin - gy0) as f32,
                s.width,
                s.height,
                s.radius,
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

/// Fill a line's glyph outlines in `color` (a transparent colour paints nothing). The
/// face's `units_per_em` is non-zero: `ttf-parser` rejects a `head` outside 16..=16384.
fn paint_text(
    pm: &mut Pixmap,
    f: &Fragment,
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    font_size: f32,
    color: Rgba,
    xf: Transform,
) {
    if color.a == 0 {
        return;
    }
    let scale = font_size / f32::from(face.units_per_em());
    let face = glyph::outline_face(face);
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
                border_radius: 0.0,
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
            edit(border_radius, shadow, gradient, transform);
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
    fn clips_with_no_paintable_region_paint_nothing() {
        // A negative-width region (no rect) and one whose transformed bounds overflow f32.
        let mut negative = plain(0.0, 0.0, 10.0, 10.0, Some(RED));
        negative.clip = Some(ClipRect {
            x: 0.0,
            y: 0.0,
            width: -5.0,
            height: 10.0,
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
            radius: 6.0,
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
        // Inset shadows are not painted (a known gap: Chrome paints them inside the box), and
        // a transparent one is nothing; an opaque outer one shows below the box.
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
        for (inset, a) in [(true, 255), (false, 0), (true, 0)] {
            let pm = render(&with_shadow(inset, a), 40, 40, &images);
            assert_eq!(
                rgb(&pm, 20, 25),
                [255, 255, 255],
                "inset {inset}, alpha {a}"
            );
            assert_eq!(rgb(&pm, 20, 15), [255, 0, 0], "the box");
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
        // A 60 x 40 box with 15px corners: on each corner's 45-degree diagonal a point 2px
        // inside the arc is filled and one 2px outside is not; the straight edges are full.
        let mut pm = Pixmap::new(60, 40).unwrap();
        fill_round_rect(
            &mut pm,
            0.0,
            0.0,
            60.0,
            40.0,
            15.0,
            RED,
            Transform::identity(),
        );
        let alpha = |x: f32, y: f32| pm.pixel(x as u32, y as u32).unwrap().alpha();
        let d = std::f32::consts::FRAC_1_SQRT_2;
        for (cx, cy, sx, sy) in [
            (15.0, 15.0, -1.0, -1.0),
            (45.0, 15.0, 1.0, -1.0),
            (45.0, 25.0, 1.0, 1.0),
            (15.0, 25.0, -1.0, 1.0),
        ] {
            let at = |r: f32| (cx + sx * r * d, cy + sy * r * d);
            let (ix, iy) = at(13.0);
            let (ox, oy) = at(17.0);
            assert_eq!(alpha(ix, iy), 255, "inside the ({cx},{cy}) corner");
            assert_eq!(alpha(ox, oy), 0, "outside the ({cx},{cy}) corner");
        }
        for (x, y) in [
            (30.0, 0.0),
            (30.0, 39.0),
            (0.0, 20.0),
            (59.0, 20.0),
            (30.0, 20.0),
        ] {
            assert_eq!(alpha(x, y), 255, "edge ({x},{y})");
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

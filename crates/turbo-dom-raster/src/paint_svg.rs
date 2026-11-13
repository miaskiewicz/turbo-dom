//! Vector painter: walk a [`Fragment`] galley into a standalone SVG document.
//! Same stacking-order walk as the raster painter, so the two agree. Glyphs are
//! emitted as filled `<path>` outlines (self-contained — no viewer font needed),
//! boxes/borders as `<rect>`, and images as base64 PNG `data:` URI `<image>`s
//! (any decoded source format; unknown formats fall back to a placeholder rect).

use std::fmt::Write;

use turbo_dom_layout::layout::value::{BorderEdges, BorderSide};
use turbo_dom_layout::text::FontFace;
use turbo_dom_layout::{
    border_style_colour, BorderRadii, BoxShadow, ClipRect, ColumnRuleStyle, Fragment,
    FragmentContent, LinearGradient as CssGradient, NodeId, PositionedGlyph, Rgba, TilePattern,
};

use crate::image_paint::{self, DecodedAssets};
use crate::paint_png::fragment_finite_positive;
use crate::rrect::{self, Seg};
use crate::tile_cell::TileCell;

const IMAGE_PLACEHOLDER: Rgba = Rgba {
    r: 220,
    g: 220,
    b: 220,
    a: 255,
};

/// Paint `galley` into a `width × height` SVG document string over a `bg` canvas
/// fill (the propagated root/body background).
pub fn paint(
    galley: &Fragment,
    width: u32,
    height: u32,
    bg: Rgba,
    images: &DecodedAssets,
) -> String {
    paint_at_size(galley, (width, height), (width, height), bg, images, 1.0)
}

pub(crate) fn paint_at_size(
    galley: &Fragment,
    css_size: (u32, u32),
    pixel_size: (u32, u32),
    bg: Rgba,
    images: &DecodedAssets,
    scale: f64,
) -> String {
    paint_at_size_with_masks(galley, css_size, pixel_size, bg, images, scale, &[])
}

pub(crate) fn paint_at_size_with_masks(
    galley: &Fragment,
    _css_size: (u32, u32),
    pixel_size: (u32, u32),
    bg: Rgba,
    images: &DecodedAssets,
    scale: f64,
    masks: &[crate::ScreenshotMask],
) -> String {
    let (pixel_width, pixel_height) = pixel_size;
    let view_width = f64::from(pixel_width) / scale;
    let view_height = f64::from(pixel_height) / scale;
    // Device-pixel rounding can enlarge the canvas beyond the CSS viewport.
    // Fill that entire canvas, as the PNG painter does, without changing layout.
    let mut svg = String::with_capacity(4096);
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixel_width}\" height=\"{pixel_height}\" \
         viewBox=\"0 0 {view_width} {view_height}\">\n<rect width=\"{view_width}\" height=\"{view_height}\" {}/>",
        fill_attrs(bg)
    );
    SHARED.with(|memo| memo.borrow_mut().take());
    paint_fragment(&mut svg, galley, images);
    glyph_filters(&mut svg);
    crate::masks::paint_svg(&mut svg, masks);
    svg.push_str("</svg>\n");
    svg
}

/// The id prefix of the glyph edge filters: `glyph-lum<class>`, one per text luminance
/// class ([`luminance_class`]).
const GLYPH_FILTER: &str = "glyph-lum";

/// Define each glyph edge filter the document uses: Skia's pre-blend table for that
/// luminance class ([`preblend_class`]) as an `feComponentTransfer` table on the run's
/// coverage (its alpha) (miaskiewicz/turbo-dom#294). The raster painter's own edge
/// transfer (`glyph_raster::edge_transfer`, #331) is linear for dark text from 28px up;
/// an SVG viewer's coarser anti-aliasing (resvg's quarter steps) lands nearer Chrome
/// with Skia's darker pre-blend, so the vector screenshot keeps it.
fn glyph_filters(svg: &mut String) {
    let marker = format!("url(#{GLYPH_FILTER}");
    let mut used = [false; 8];
    for (at, _) in svg.match_indices(&marker) {
        let class = svg.as_bytes()[at + marker.len()] - b'0';
        used[usize::from(class)] = true;
    }
    for class in (0..8u32).filter(|&c| used[c as usize]) {
        let lut = preblend_class(class);
        // One table value per 8-bit coverage: a viewer's linear interpolation between
        // them meets the raster painter's table at every 8-bit alpha.
        let table: Vec<String> = lut.iter().map(|v| format!("{v:.4}")).collect();
        let _ = writeln!(
            svg,
            "<filter id=\"{GLYPH_FILTER}{class}\" color-interpolation-filters=\"sRGB\">\
             <feComponentTransfer><feFuncA type=\"table\" tableValues=\"{}\"/>\
             </feComponentTransfer></filter>",
            table.join(" ")
        );
    }
}

/// The pre-blend's luminance gamma.
const GAMMA: f32 = 1.8;
/// The pre-blend's contrast boost (full for black text, tapering to none for white).
const CONTRAST: f32 = 1.0;

fn to_linear(v: f32) -> f32 {
    v.powf(GAMMA)
}

fn from_linear(v: f32) -> f32 {
    v.powf(GAMMA.recip())
}

/// The 3-bit luminance (0..=7) Skia's mask pre-blend keeps of text colour `(r, g, b)`
/// (in [`GAMMA`] space): two text colours of one class share a pre-blend table.
fn luminance_class(r: u8, g: u8, b: u8) -> u32 {
    let lum = 0.2126 * to_linear(f32::from(r) / 255.0)
        + 0.7152 * to_linear(f32::from(g) / 255.0)
        + 0.0722 * to_linear(f32::from(b) / 255.0);
    ((from_linear(lum) * 255.0).round() as u32) >> 5
}

/// Replicate a three-bit class (0..=7) across an eight-bit luminance value.
fn class_luminance(top: u32) -> u32 {
    // For the three-bit input, disjoint copies sum to 32*top + 4*top + top/2.
    top * 36 + top / 2
}

/// Skia's mask pre-blend table (`SkTMaskGamma_build_correcting_lut`) of luminance class
/// `top` ([`luminance_class`], replicated back to 8 bits): the coverage is boosted by the
/// contrast where the assumed backdrop (the opposite luminance) is light, and the table
/// undoes the sRGB blend so the result is a blend at [`GAMMA`].
fn preblend_class(top: u32) -> [f32; 256] {
    let src = class_luminance(top) as f32 / 255.0;
    let dst = 1.0 - src;
    let (lin_src, lin_dst) = (to_linear(src), to_linear(dst));
    let contrast = CONTRAST * lin_dst;
    let mut table = [0.0; 256];
    for (i, e) in table.iter_mut().enumerate() {
        let raw = i as f32 / 255.0;
        let a = (raw + (1.0 - raw) * contrast * raw).min(1.0);
        let out = from_linear(lin_src * a + (1.0 - a) * lin_dst);
        *e = ((out - dst) / (src - dst)).clamp(0.0, 1.0);
    }
    table
}

fn paint_fragment(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    // CSS `opacity` (miaskiewicz/turbo-dom#206) is a group opacity over the box and
    // its subtree; `opacity: 0` paints nothing.
    if f.opacity < 1.0 {
        if f.opacity > 0.0 {
            let _ = write!(svg, "<g opacity=\"{}\">", f.opacity);
            paint_opaque(svg, f, images);
            svg.push_str("</g>");
        }
        return;
    }
    paint_opaque(svg, f, images);
}

/// [`paint_fragment`] for a box painted at full opacity.
fn paint_opaque(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    // A CSS `transform` wraps this box + subtree in a `<g transform>` (applied about
    // the box's absolute transform-origin), so the SVG viewer transforms them as a unit.
    let transformed = matches!(
        &f.content,
        FragmentContent::Box {
            transform: Some(_),
            ..
        }
    );
    if let FragmentContent::Box {
        transform: Some(t), ..
    } = &f.content
    {
        let (ox, oy) = (f.x + t.origin_x, f.y + t.origin_y);
        let [a, b, c, d, e, g] = t.matrix;
        let _ = write!(
            svg,
            "<g transform=\"translate({ox:.2} {oy:.2}) matrix({a} {b} {c} {d} {e} {g}) translate({:.2} {:.2})\">",
            -ox, -oy
        );
    }
    match f.clip {
        Some(clip) => paint_clipped(svg, f, clip, images),
        None => paint_subtree(svg, f, images),
    }
    if transformed {
        svg.push_str("</g>");
    }
}

/// Paint a fragment whose box has a `clip: rect()` (miaskiewicz/turbo-dom#52) inside a
/// `<g clip-path>` of that region (in the box's own, possibly transformed, user space;
/// the `clipPath` id is the document offset, unique); an empty region paints nothing.
fn paint_clipped(svg: &mut String, f: &Fragment, clip: ClipRect, images: &DecodedAssets) {
    if clip.width <= 0.0 || clip.height <= 0.0 {
        return;
    }
    let id = svg.len();
    let _ = write!(svg, "<clipPath id=\"clip{id}\">");
    let region = (f.x + clip.x, f.y + clip.y, clip.width, clip.height);
    shape(svg, region, &clip.radii, "");
    let _ = write!(svg, "</clipPath><g clip-path=\"url(#clip{id})\">");
    paint_subtree(svg, f, images);
    svg.push_str("</g>");
}

/// Paint a fragment's own content, then its children in stacking order.
fn paint_subtree(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    match &f.content {
        FragmentContent::Box {
            background,
            border,
            border_radius,
            shadow,
            gradient,
            ..
        } => {
            if let Some(sh) = shadow.filter(|s| !s.inset && s.color.a > 0) {
                paint_box_shadow(svg, f, border_radius, &sh);
            }
            if let Some(bg) = background {
                rect_r(svg, (f.x, f.y, f.width, f.height), *bg, border_radius);
            }
            if let Some(g) = gradient {
                paint_gradient(svg, f, border_radius, g);
            }
            paint_inset_shadow(svg, f, border, border_radius, shadow.as_ref());
            if border_radius.is_rounded() {
                paint_round_border(svg, f, border, border_radius);
            } else {
                paint_border(svg, f, border);
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
        } => {
            paint_text(
                svg,
                (f, *vertical, *upright),
                glyphs,
                face,
                (*font_size, *color),
            );
        }
        FragmentContent::Image(placement) => paint_image(svg, f, placement, images),
        FragmentContent::Tiles(pattern) => paint_tiles(svg, f, pattern, images),
        FragmentContent::ColumnRule { style, color } => paint_column_rule(svg, f, *style, *color),
        FragmentContent::Marker(_) => {}
    }
    paint_children(svg, f, images);
}

/// Paint a multicol `column-rule` fragment (`FragmentContent::ColumnRule`;
/// miaskiewicz/turbo-dom#398 PR-D). The SVG geometry matches the raster
/// painter's — solid is the whole rect, dotted stamps circles of diameter
/// `width`, dashed stamps dashes `2 * width` long, double paints two edge
/// bands of `floor(width / 3)` thickness.
fn paint_column_rule(svg: &mut String, f: &Fragment, style: ColumnRuleStyle, color: Rgba) {
    // R1 M1: reject non-finite sizes — `f.width <= 0.0` is false for
    // `NaN` and `Infinity`, which would cascade into `cx = NaN / 2.0`
    // and SVG `cx="NaN"` / `width="NaN"` output.
    if color.a == 0
        || !f.width.is_finite()
        || !f.height.is_finite()
        || f.width <= 0.0
        || f.height <= 0.0
    {
        return;
    }
    if rule_needs_local_origin(f, style) {
        paint_rule_at_local_origin(svg, f, style, color);
        return;
    }
    match style {
        ColumnRuleStyle::Solid => rect(svg, f.x, f.y, f.width, f.height, color),
        ColumnRuleStyle::Dotted => paint_dotted_rule(svg, f, color),
        ColumnRuleStyle::Dashed => paint_dashed_rule(svg, f, color),
        ColumnRuleStyle::Double => paint_double_rule(svg, f, color),
        ColumnRuleStyle::Groove
        | ColumnRuleStyle::Ridge
        | ColumnRuleStyle::Inset
        | ColumnRuleStyle::Outset => paint_shaded_rule(svg, f, style, color),
    }
}

fn rule_needs_local_origin(f: &Fragment, style: ColumnRuleStyle) -> bool {
    let step = match style {
        ColumnRuleStyle::Dotted => f.width.max(1.0) / 2.0,
        ColumnRuleStyle::Dashed => f.width,
        _ => return false,
    };
    f.y.is_finite() && f.y + step <= f.y
}

fn paint_rule_at_local_origin(svg: &mut String, f: &Fragment, style: ColumnRuleStyle, color: Rgba) {
    // SVG can retain the large translation while the pattern steps stay local.
    // Dropping an offscreen rule would be wrong beneath an inverse transform.
    let _ = write!(svg, "<g transform=\"translate(0 {})\">", f.y);
    let local = Fragment::new(
        f.node_id,
        f.x,
        0.0,
        f.width,
        f.height,
        FragmentContent::ColumnRule { style, color },
    );
    paint_column_rule(svg, &local, style, color);
    svg.push_str("</g>");
}

/// SVG counterpart to [`crate::paint_png::paint_dotted_rule`]: diameter is
/// floored to 1 px (Chrome 156's `kDottedStroke` rounds the effective stroke
/// to >= 1 device pixel; a sub-pixel `column-rule-width` would vanish to
/// resvg's antialiased transparent otherwise), rule centred on
/// `f.x + f.width / 2` so gap allocation still uses the declared width.
/// For sub-pixel declared widths we emit pixel-aligned 1 px squares matching
/// the PNG painter's fallback — a disk of radius 0.5 would spread to
/// adjacent columns under resvg's anti-aliasing (#424).
fn paint_dotted_rule(svg: &mut String, f: &Fragment, color: Rgba) {
    // A zero- or negative-width fragment must emit nothing: the `max(1.0)`
    // floor below would otherwise coerce it into the sub-pixel rect
    // fallback (mirrors the PNG painter's guard, #424). R1 M1:
    // `is_finite()` also rejects `NaN` / `Infinity`, which comparisons
    // never catch — otherwise `cx = f.x + NaN / 2.0 = NaN` would render
    // `cx="NaN"` in the SVG output.
    if !f.width.is_finite() || !f.height.is_finite() || f.width <= 0.0 {
        return;
    }
    let diameter = f.width.max(1.0);
    let cx = f.x + f.width / 2.0;
    let period = 2.0 * diameter;
    let end = f.y + f.height;
    if f.width < 1.0 {
        paint_subpixel_dotted_rule(svg, f, color, (cx.floor(), end));
        return;
    }
    let radius = diameter / 2.0;
    let mut cy = f.y + radius;
    while cy - radius < end {
        let _ = writeln!(
            svg,
            "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{radius:.2}\" {}/>",
            fill_attrs(color)
        );
        let next = cy + period;
        if next <= cy {
            break;
        }
        cy = next;
    }
}

fn paint_subpixel_dotted_rule(
    svg: &mut String,
    f: &Fragment,
    color: Rgba,
    (px_left, end): (f32, f32),
) {
    // The sole caller selects a positive width below one pixel, floored to one.
    let diameter = 1.0;
    // rect_r also rejects empty/reversed caps and non-finite origins.
    let cap_h = (end - f.y).min(diameter);
    rect(svg, px_left, f.y, diameter, cap_h, color);
    let mut y = f.y + diameter;
    while y < end {
        let dash_h = (end - y).min(diameter);
        rect(svg, px_left, y, diameter, dash_h, color);
        let next = y + 2.0;
        if next <= y {
            break;
        }
        y = next;
    }
}

fn paint_dashed_rule(svg: &mut String, f: &Fragment, color: Rgba) {
    // R1 M1: reject non-finite sizes — `period <= 0.0` misses `NaN` and
    // `Infinity`.
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
    let end = f.y + f.height;
    let mut dash_top = f.y;
    while dash_top < end {
        let dash_h = dash.min(end - dash_top);
        rect(svg, f.x, dash_top, width, dash_h, color);
        let next = dash_top + period;
        if next <= dash_top {
            break;
        }
        dash_top = next;
    }
}

fn paint_double_rule(svg: &mut String, f: &Fragment, color: Rgba) {
    // Finite widths at least three give bands at least one pixel wide.
    // rect_r validates both emitted bands, including all non-finite bounds.
    if f.width < 3.0 {
        rect(svg, f.x, f.y, f.width, f.height, color);
        return;
    }
    let band = (f.width / 3.0).floor();
    rect(svg, f.x, f.y, band, f.height, color);
    rect(svg, f.x + f.width - band, f.y, band, f.height, color);
}

/// SVG counterpart to [`crate::paint_png::paint_shaded_rule`] — the two halves
/// of a 3-D column rule, shaded through [`border_style_colour`]
/// (miaskiewicz/turbo-dom#419). `groove` / `outset` paint the left half dark
/// and the right half light; `ridge` / `inset` swap them. The rule is
/// vertical, so "left" and "right" are the two vertical bands.
fn paint_shaded_rule(svg: &mut String, f: &Fragment, style: ColumnRuleStyle, color: Rgba) {
    // Both halves pass through rect_r, which rejects invalid or empty bounds.
    let left_dark = matches!(style, ColumnRuleStyle::Groove | ColumnRuleStyle::Outset);
    let left_color = border_style_colour(color, left_dark);
    let right_color = border_style_colour(color, !left_dark);
    let half = f.width / 2.0;
    rect(svg, f.x, f.y, half, f.height, left_color);
    rect(svg, f.x + half, f.y, f.width - half, f.height, right_color);
}

/// The most tiles without a whole-pixel period [`paint_tiles`] writes one by one.
const MAX_LISTED_TILES: usize = 4096;

/// A background layer's tiles ([`TilePattern`]; 0.7.0 round-2 review, finding 1): the
/// tiles of one [`TileCell`] (a whole-pixel super-period of the snapped tiles, the same
/// cell the PNG painter repeats), at their snapped places and sizes, in a `<pattern>`
/// repeating every cell, filling the box (only the tile's own row or column on an axis
/// that does not repeat). Tiles without a whole-pixel period (a `round` or `space` step
/// off the pixel grid) are written one by one when there are at most
/// [`MAX_LISTED_TILES`] of them. The pattern id is the document offset, unique.
fn paint_listed_tiles(
    svg: &mut String,
    node_id: NodeId,
    pattern: &TilePattern,
    area: (f32, f32, f32, f32),
    images: &DecodedAssets,
) {
    for (x, y, w, h) in pattern.tiles(area) {
        let tile = Fragment::new(node_id, x, y, w, h, (*pattern.tile).clone());
        paint_subtree(svg, &tile, images);
    }
}

fn paint_pattern_tiles(
    svg: &mut String,
    f: &Fragment,
    pattern: &TilePattern,
    first: (f32, f32, f32, f32),
    images: &DecodedAssets,
) {
    let cell = TileCell::new(pattern, first);
    let ((x, y), (pw, ph)) = (cell.origin, cell.size);
    let along = |repeats: bool, (from, len): (f32, f32), (at, size): (f32, f32)| {
        if repeats {
            (from, len)
        } else {
            (at, size)
        }
    };
    let (fx, fw) = along(cell.repeats.0, (f.x, f.width), (first.0, first.2));
    let (fy, fh) = along(cell.repeats.1, (f.y, f.height), (first.1, first.3));
    let id = svg.len();
    let _ = write!(
        svg,
        "<pattern id=\"tiles{id}\" patternUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" \
         width=\"{pw}\" height=\"{ph}\">"
    );
    SHARED.with(|memo| memo.borrow_mut().take());
    svg.push_str("<g shape-rendering=\"crispEdges\">");
    for (tx, ty, w, h) in cell.tiles(pattern) {
        let tile = Fragment::new(f.node_id, tx, ty, w, h, (*pattern.tile).clone());
        paint_subtree(svg, &tile, images);
    }
    SHARED.with(|memo| memo.borrow_mut().take());
    let _ = writeln!(
        svg,
        "</g></pattern><rect x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" fill=\"url(#tiles{id})\"/>"
    );
}

fn paint_tiles(svg: &mut String, f: &Fragment, pattern: &TilePattern, images: &DecodedAssets) {
    if !fragment_finite_positive(f) {
        return;
    }
    let area = (f.x, f.y, f.width, f.height);
    let Some(first) = pattern.tiles(area).next() else {
        return;
    };
    if pattern.period().is_none()
        && crate::paint_png::repeats_tiles(pattern)
        && pattern.tiles(area).nth(1).is_some()
        && paint_tile_texture(svg, f, pattern, (images, first)).is_some()
    {
        return;
    }
    if pattern.period().is_none() && pattern.tiles(area).nth(MAX_LISTED_TILES).is_none() {
        paint_listed_tiles(svg, f.node_id, pattern, area, images);
        return;
    }
    paint_pattern_tiles(svg, f, pattern, first, images);
}

/// The most pixels of one fractional tile's image [`paint_tile_texture`] embeds.
const MAX_TEXTURE_PIXELS: u64 = 1 << 22;

/// Fractional tiles repeated every tile size (miaskiewicz/turbo-dom#270): one whole-pixel
/// cell of them as Chrome and the PNG painter draw it
/// ([`crate::paint_png::texture_cell`]: the pixels a tile edge crosses blend), embedded as
/// one `<image>` in a `<pattern>` of the cell (a viewer rasterizes a fractional
/// `<pattern>` cell with gaps, and tiles drawn side by side with seams). `None` (written
/// another way) when the tile has no pixels to embed: a missing image, or a tile over
/// [`MAX_TEXTURE_PIXELS`].
fn paint_tile_texture(
    svg: &mut String,
    f: &Fragment,
    pattern: &TilePattern,
    (images, first): (&DecodedAssets, rrect::Rect),
) -> Option<()> {
    let (cell, (x, y)) =
        crate::paint_png::texture_cell(pattern, images, first, MAX_TEXTURE_PIXELS)?;
    let png = cell.encode_png().ok()?;
    let (w, h) = (cell.width(), cell.height());
    let along = |repeats: bool, (from, len): (f32, f32), (at, size): (f32, f32)| {
        if repeats {
            (from, len)
        } else {
            (at, size)
        }
    };
    let (fx, fw) = along(pattern.step_x > 0.0, (f.x, f.width), (first.0, first.2));
    let (fy, fh) = along(pattern.step_y > 0.0, (f.y, f.height), (first.1, first.3));
    let id = svg.len();
    let _ = writeln!(
        svg,
        "<pattern id=\"tiles{id}\" patternUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" \
         width=\"{w}\" height=\"{h}\"><image width=\"{w}\" height=\"{h}\" \
         preserveAspectRatio=\"none\" href=\"data:image/png;base64,{}\"/></pattern>\
         <rect x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" fill=\"url(#tiles{id})\"/>",
        image_paint::base64(&png)
    );
    Some(())
}

/// Paint `f`'s children in stacking order inside a `<g clip-path>` of its `overflow`
/// region (miaskiewicz/turbo-dom#253) when it has one and they may paint past it (an empty
/// region paints none of them).
fn paint_children(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
    let clip = f.overflow_clip.filter(|c| {
        !f.children.is_empty()
            && (c.radii.is_rounded()
                || !f
                    .children
                    .iter()
                    .all(|k| crate::paint_png::within(k, crate::paint_png::overflow_region(f, c))))
    });
    if let Some(c) = clip {
        if c.width <= 0.0 || c.height <= 0.0 {
            return;
        }
        let id = svg.len();
        let _ = write!(svg, "<clipPath id=\"overflow{id}\">");
        shape(svg, crate::paint_png::overflow_region(f, &c), &c.radii, "");
        let _ = write!(svg, "</clipPath><g clip-path=\"url(#overflow{id})\">");
    }
    // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
    // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
    for &i in &f.paint_order() {
        paint_fragment(svg, &f.children[i], images);
    }
    if clip.is_some() {
        svg.push_str("</g>");
    }
}

/// SVG path data for outline steps.
fn path_data(segs: &[Seg]) -> String {
    let mut d = String::new();
    for seg in segs {
        let _ = match *seg {
            Seg::Move(x, y) => write!(d, "M{x:.2} {y:.2} "),
            Seg::Line(x, y) => write!(d, "L{x:.2} {y:.2} "),
            Seg::Cubic(a, b, c, e, x, y) => {
                write!(d, "C{a:.2} {b:.2} {c:.2} {e:.2} {x:.2} {y:.2} ")
            }
            Seg::Close => write!(d, "Z "),
        };
    }
    d.truncate(d.trim_end().len());
    d
}

/// Emit `rect` with corner radii `radii` and the extra attributes `attrs`: a `<rect>`
/// (with `rx`/`ry` for one circular radius over half a pixel) or, for different or
/// elliptical corners, a `<path>` of its outline (miaskiewicz/turbo-dom#253).
fn shape(svg: &mut String, (x, y, w, h): rrect::Rect, radii: &BorderRadii, attrs: &str) {
    let sep = if attrs.is_empty() { "" } else { " " };
    let Some(r) = radii.as_uniform() else {
        let d = path_data(&rrect::outline((x, y, w, h), radii));
        let _ = write!(svg, "<path d=\"{d}\"{sep}{attrs}/>");
        return;
    };
    let rr = if r > 0.5 {
        format!(" rx=\"{r:.2}\" ry=\"{r:.2}\"")
    } else {
        String::new()
    };
    let _ = write!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr}{sep}{attrs}/>"
    );
}

/// Embed an image as a base64 `data:` URI `<image>` filling its layout box. An image
/// laid out at its size whose pixels did not decode paints nothing, as in Chrome
/// (miaskiewicz/turbo-dom#107); a neutral placeholder rect stands for an image the caller
/// did not supply or this build cannot decode.
/// `preserveAspectRatio="none"` because layout already sized the box to the
/// image's aspect ratio.
fn paint_image(
    svg: &mut String,
    f: &Fragment,
    placement: &turbo_dom_layout::ImagePlacement,
    images: &DecodedAssets,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let decoded = images.get(&placement.name);
    let key = decoded.map(|img| {
        let size = (f.width.to_bits(), f.height.to_bits());
        (
            std::ptr::from_ref(img) as usize,
            placement.orientation,
            size,
        )
    });
    // The next tile of the same image at the same size reuses the first one's `<image>`
    // through a `<use>` (miaskiewicz/turbo-dom#248: a background layer draws one image
    // many times); the id is given to that `<image>` only once it is reused.
    let reused = SHARED.with(|memo| {
        let mut memo = memo.borrow_mut();
        let shared = memo.as_mut().filter(|shared| Some(shared.key) == key)?;
        if !shared.named {
            svg.insert_str(
                shared.at + "<image".len(),
                &format!(" id=\"img{}\"", shared.at),
            );
            shared.named = true;
        }
        Some((shared.at, f.x - shared.x, f.y - shared.y))
    });
    if let Some((id, dx, dy)) = reused {
        let _ = writeln!(svg, "<use href=\"#img{id}\" x=\"{dx:.2}\" y=\"{dy:.2}\"/>");
        return;
    }
    let uri = decoded.and_then(|img| image_paint::data_uri(&img.oriented(placement.orientation)));
    match (uri, key) {
        (Some(uri), Some(key)) => {
            let at = svg.len();
            let _ = writeln!(
                svg,
                "<image x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" \
                 preserveAspectRatio=\"none\" href=\"{uri}\"/>",
                f.x, f.y, f.width, f.height
            );
            let shared = Shared {
                key,
                at,
                x: f.x,
                y: f.y,
                named: false,
            };
            SHARED.with(|memo| *memo.borrow_mut() = Some(shared));
        }
        _ if decoded.is_some_and(|d| !d.placeholder) => {}
        _ => rect(svg, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER),
    }
}

/// The last `<image>` painted: its key, offset in the document (its id once reused)
/// and position.
struct Shared {
    key: (usize, turbo_dom_layout::image::Orientation, (u32, u32)),
    at: usize,
    x: f32,
    y: f32,
    named: bool,
}
thread_local! {
    static SHARED: std::cell::RefCell<Option<Shared>> = const { std::cell::RefCell::new(None) };
}

/// `rgba(...)`-free fill: an opaque `#rrggbb` plus a separate `fill-opacity`
/// when the color carries alpha (keeps output ASCII + widely compatible).
fn fill_attrs(c: Rgba) -> String {
    if c.a == 255 {
        format!("fill=\"#{:02x}{:02x}{:02x}\"", c.r, c.g, c.b)
    } else {
        format!(
            "fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{:.3}\"",
            c.r,
            c.g,
            c.b,
            f32::from(c.a) / 255.0
        )
    }
}

fn rect(svg: &mut String, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
    rect_r(svg, (x, y, w, h), c, &BorderRadii::ZERO);
}

/// Paint an outer `box-shadow` as a blurred rounded rect behind the box, using an
/// inline SVG `feGaussianBlur` filter (native, so any viewer renders the soft
/// edge). `blur` 0 emits an unfiltered offset rect. The filter id is the document
/// offset, unique (round-1 review of 0.7.0, finding 8: it was the box origin).
fn paint_box_shadow(svg: &mut String, f: &Fragment, border_radius: &BorderRadii, sh: &BoxShadow) {
    if !fragment_finite_positive(f) {
        return;
    }
    let spread = sh.spread as f32;
    let x = f.x + sh.offset_x as f32 - spread;
    let y = f.y + sh.offset_y as f32 - spread;
    let w = f.width + 2.0 * spread;
    let h = f.height + 2.0 * spread;
    if w < 0.5 || h < 0.5 {
        return;
    }
    let radii = rrect::shadow_radii(border_radius, spread);
    if sh.blur < 1 {
        rect_r(svg, (x, y, w, h), sh.color, &radii);
        return;
    }
    // Pad the filter region by the blur so the feathered edge isn't clipped.
    // The document offset: unique, also for boxes at one origin.
    let id = format!("sh{}", svg.len());
    // Capped like Chrome's (miaskiewicz/turbo-dom#95).
    let std = crate::paint_png::shadow_blur(sh) / 2.0;
    let _ = writeln!(
        svg,
        "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\
         <feGaussianBlur stdDeviation=\"{std:.2}\"/></filter>",
    );
    let attrs = format!("{} filter=\"url(#{id})\"", fill_attrs(sh.color));
    shape(svg, (x, y, w, h), &radii, &attrs);
    svg.push('\n');
}

/// Paint an inset `box-shadow` (miaskiewicz/turbo-dom#192) as the padding box in the
/// shadow colour through a `<mask>` that cuts out the hole: the padding box offset, shrunk
/// by the spread (radius with it) and blurred (`feGaussianBlur` inside the mask). The mask
/// id is the document offset, unique.
fn paint_inset_shadow(
    svg: &mut String,
    f: &Fragment,
    border: &BorderEdges,
    border_radius: &BorderRadii,
    shadow: Option<&BoxShadow>,
) {
    if !fragment_finite_positive(f) {
        return;
    }
    let Some(sh) = shadow.filter(|s| s.inset && s.color.a > 0) else {
        return;
    };
    let ((px, py, pw, ph), pr) = rrect::padding_box(f, border, border_radius);
    if pw <= 0.0 || ph <= 0.0 {
        return;
    }
    let spread = sh.spread as f32;
    let (hx, hy) = (
        px + sh.offset_x as f32 + spread,
        py + sh.offset_y as f32 + spread,
    );
    let (hw, hh) = (pw - 2.0 * spread, ph - 2.0 * spread);
    let id = svg.len();
    let _ = write!(
        svg,
        "<mask id=\"inset{id}\" maskUnits=\"userSpaceOnUse\" x=\"{px:.2}\" y=\"{py:.2}\" width=\"{pw:.2}\" height=\"{ph:.2}\">\
         <rect x=\"{px:.2}\" y=\"{py:.2}\" width=\"{pw:.2}\" height=\"{ph:.2}\" fill=\"#ffffff\"/>"
    );
    if hw >= 0.5 && hh >= 0.5 {
        let blur = crate::paint_png::shadow_blur(sh);
        let filter = if sh.blur == 0 {
            String::new()
        } else {
            let _ = write!(
                svg,
                "<filter id=\"insetblur{id}\" filterUnits=\"userSpaceOnUse\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\">\
                 <feGaussianBlur stdDeviation=\"{:.2}\"/></filter>",
                hx - 2.0 * blur,
                hy - 2.0 * blur,
                hw + 4.0 * blur,
                hh + 4.0 * blur,
                blur / 2.0
            );
            format!(" filter=\"url(#insetblur{id})\"")
        };
        let hole = pr.inset(spread, spread, spread, spread).fit(hw, hh);
        shape(
            svg,
            (hx, hy, hw, hh),
            &hole,
            &format!("fill=\"#000000\"{filter}"),
        );
    }
    let _ = write!(svg, "</mask><g mask=\"url(#inset{id})\">");
    rect_r(svg, (px, py, pw, ph), sh.color, &pr);
    svg.push_str("</g>\n");
}

/// Paint a `linear-gradient(...)` background as a native SVG `<linearGradient>`
/// (userSpaceOnUse, start/end points across the box per the CSS angle) filling the
/// box rect (rounded by `border_radius`). The gradient id is the document offset,
/// unique.
fn paint_gradient(svg: &mut String, f: &Fragment, border_radius: &BorderRadii, g: &CssGradient) {
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
    let (dx, dy) = (theta.sin(), -theta.cos());
    let half = (w * theta.sin().abs() + h * theta.cos().abs()) / 2.0;
    let (cx, cy) = (f.x + w / 2.0, f.y + h / 2.0);
    let (x1, y1) = (cx - dx * half, cy - dy * half);
    let (x2, y2) = (cx + dx * half, cy + dy * half);
    let id = format!("g{}", svg.len());
    let _ = write!(
        svg,
        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
         x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\">"
    );
    for s in &g.stops {
        let _ = write!(
            svg,
            "<stop offset=\"{:.3}\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{:.3}\"/>",
            s.pos.clamp(0.0, 1.0),
            s.color.r,
            s.color.g,
            s.color.b,
            f32::from(s.color.a) / 255.0
        );
    }
    svg.push_str("</linearGradient>\n");
    let radii = if border_radius.is_rounded() {
        *border_radius
    } else {
        BorderRadii::ZERO
    };
    shape(
        svg,
        (f.x, f.y, w, h),
        &radii,
        &format!("fill=\"url(#{id})\""),
    );
    svg.push('\n');
}

/// A filled box `rect` with corner radii `radii` (square unless rounded).
fn rect_r(svg: &mut String, rect: rrect::Rect, c: Rgba, radii: &BorderRadii) {
    // R3 H1: `rect.2 <= 0.0 || rect.3 <= 0.0` is false for `NaN` (every
    // comparison with NaN is false) and for `Infinity` (strictly positive),
    // so a non-finite rect would reach `shape` and serialize `width="NaN"` /
    // `x="inf"` into the document. Reject non-finite coords up front.
    if !rect.0.is_finite()
        || !rect.1.is_finite()
        || !rect.2.is_finite()
        || !rect.3.is_finite()
        || rect.2 <= 0.0
        || rect.3 <= 0.0
        || c.a == 0
    {
        return;
    }
    let radii = if radii.is_rounded() {
        *radii
    } else {
        BorderRadii::ZERO
    };
    shape(svg, rect, &radii, &fill_attrs(c));
    svg.push('\n');
}

fn paint_border(svg: &mut String, f: &Fragment, b: &BorderEdges) {
    if !fragment_finite_positive(f) {
        return;
    }
    let side = |s: &BorderSide| {
        s.color
            .filter(|_| s.width > 0)
            .map(|c| (f32::from(s.width), c))
    };
    let mitred = crate::rrect::mixed_colours(b);
    if !mitred {
        if let Some((w, c)) = side(&b.top) {
            rect(svg, f.x, f.y, f.width, w, c);
        }
        if let Some((w, c)) = side(&b.bottom) {
            rect(svg, f.x, f.y + f.height - w, f.width, w, c);
        }
    }
    if let Some((w, c)) = side(&b.left) {
        rect(svg, f.x, f.y, w, f.height, c);
    }
    if let Some((w, c)) = side(&b.right) {
        rect(svg, f.x + f.width - w, f.y, w, f.height, c);
    }
    if !mitred {
        return;
    }
    // As the raster painter: sides that differ in colour paint the top and bottom bands
    // over the left and right ones, mitred on the corner diagonals.
    for (top, s) in [(true, &b.top), (false, &b.bottom)] {
        let Some((_, c)) = side(s) else { continue };
        let band =
            crate::rrect::mitred_band((f.x, f.y, f.width, f.height), crate::rrect::widths(b), top);
        let d = path_data(&crate::rrect::polygon(&band));
        let _ = writeln!(svg, "<path d=\"{d}\" {}/>", fill_attrs(c));
    }
}

/// Paint a rounded box's border as the raster painter does (miaskiewicz/turbo-dom#253):
/// the even-odd ring between the border-box and padding-box outlines, in one `<path>`
/// when every painted side has one colour, else each side's part of it through a
/// `<clipPath>` of its region ([`rrect::side_region`]). Clip ids are document offsets.
fn paint_round_border(svg: &mut String, f: &Fragment, b: &BorderEdges, radii: &BorderRadii) {
    if !fragment_finite_positive(f) {
        return;
    }
    let rect = (f.x, f.y, f.width, f.height);
    let (inner, inner_radii) = rrect::padding_box(f, b, radii);
    let mut d = path_data(&rrect::outline(rect, radii));
    if inner.2 > 0.0 && inner.3 > 0.0 {
        d.push(' ');
        d.push_str(&path_data(&rrect::outline(inner, &inner_radii)));
    }
    let sides = [&b.top, &b.right, &b.bottom, &b.left];
    let painted: Vec<(usize, Rgba)> = sides
        .iter()
        .enumerate()
        .filter(|(_, s)| s.width > 0)
        .filter_map(|(i, s)| s.color.filter(|c| c.a > 0).map(|c| (i, c)))
        .collect();
    let widths = rrect::widths(b);
    let all = widths.iter().filter(|w| **w > 0.0).count();
    if painted.len() == all && painted.windows(2).all(|p| p[0].1 == p[1].1) {
        if let Some(&(_, c)) = painted.first() {
            let _ = writeln!(
                svg,
                "<path d=\"{d}\" fill-rule=\"evenodd\" {}/>",
                fill_attrs(c)
            );
        }
        return;
    }
    for (side, c) in painted {
        let region = path_data(&rrect::polygon(&rrect::side_region(rect, widths, side)));
        let id = svg.len();
        let _ = writeln!(
            svg,
            "<clipPath id=\"side{id}\"><path d=\"{region}\"/></clipPath>\
             <path d=\"{d}\" fill-rule=\"evenodd\" clip-path=\"url(#side{id})\" {}/>",
            fill_attrs(c)
        );
    }
}

/// Emit a line's glyph run as one filled `<path>` of the outline the raster painter
/// fills ([`crate::glyph_raster::run_outline`]: snapped, lifted and emboldened like
/// Chrome's glyphs, in page coordinates; a transparent colour or a run with no outlined
/// glyph emits nothing), its coverage corrected by the colour's pre-blend filter
/// ([`glyph_filters`]). The colour's alpha is the path's `opacity`, which applies after
/// the filter: the raster painter's `table[coverage] * alpha`.
fn paint_text(
    svg: &mut String,
    (f, vertical, upright): (&Fragment, bool, bool),
    glyphs: &[PositionedGlyph],
    face: &FontFace,
    (font_size, color): (f32, Rgba),
) {
    if color.a == 0 {
        return;
    }
    let frame = crate::paint_png::glyph_frame(f, vertical);
    let run =
        crate::glyph_raster::run_outline((f, vertical && upright), glyphs, face, font_size, frame);
    let d = run.svg_path();
    if d.is_empty() {
        return;
    }
    let class = luminance_class(color.r, color.g, color.b);
    let opacity = if color.a == 255 {
        String::new()
    } else {
        format!(" opacity=\"{:.3}\"", f32::from(color.a) / 255.0)
    };
    let _ = writeln!(
        svg,
        "<path d=\"{d}\" fill=\"#{:02x}{:02x}{:02x}\"{opacity} filter=\"url(#{GLYPH_FILTER}{class})\"/>",
        color.r, color.g, color.b
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turbo_dom_layout::image::{Intrinsic, Orientation};
    use turbo_dom_layout::{
        FontRegistry, GradientStop, ImagePlacement, MarkerKind, NodeId, PositionedGlyph,
        Transform2D,
    };

    use crate::image_paint::DecodedImage;

    const RED: Rgba = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    const WHITE: Rgba = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

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

    /// The document body between the canvas background rect and `</svg>`.
    fn body(galley: &Fragment, images: &DecodedAssets) -> String {
        let svg = paint(galley, 10, 10, WHITE, images);
        let start = svg.find("/>\n").unwrap() + 3;
        svg[start..svg.len() - "</svg>\n".len()].to_string()
    }

    fn tiles(at: (f32, f32), tile: (f32, f32), step: (f32, f32)) -> Fragment {
        Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            10.0,
            10.0,
            FragmentContent::Tiles(turbo_dom_layout::TilePattern {
                tile: Box::new(plain(0.0, 0.0, 1.0, 1.0, Some(RED)).content),
                x: at.0,
                y: at.1,
                tile_width: tile.0,
                tile_height: tile.1,
                step_x: step.0,
                step_y: step.1,
            }),
        )
    }

    #[test]
    fn translated_tile_patterns_are_written_in_phase() {
        // miaskiewicz/turbo-dom#274: the 10x10 pattern box moved by `Fragment::translate`
        // writes the pattern (or the walked tiles) where the same pattern laid out at the
        // new spot does.
        for (tile, step) in [((2.0, 2.0), (4.0, 4.0)), ((2.0, 10.0), (4.5, 0.0))] {
            let mut moved = tiles((0.0, 0.0), tile, step);
            moved.translate(3.0, 5.0);
            let mut placed = tiles((3.0, 5.0), tile, step);
            (placed.x, placed.y) = (3.0, 5.0);
            let (got, want) = (
                body(&moved, &HashMap::new()),
                body(&placed, &HashMap::new()),
            );
            assert_eq!(got, want);
            if step.1 == 0.0 {
                // Walked: tiles at 3, 7.5 (8), 12 — the box's 3 tiles, not 4 out of phase.
                assert_eq!(got.matches("<rect").count(), 3, "{got}");
                assert!(got.contains("<rect x=\"8.00\""), "{got}");
            } else {
                assert!(
                    got.contains("x=\"3\" y=\"5\" width=\"4\" height=\"4\">"),
                    "{got}"
                );
            }
        }
    }

    #[test]
    fn tile_patterns_are_one_pattern_or_their_few_tiles() {
        // 0.7.0 round-2 review, finding 1: a whole-pixel period is one `<pattern>` of the
        // snapped tile, filling the box (a lone axis: the tile's own row).
        let grid = body(&tiles((0.5, 0.0), (1.0, 1.0), (2.0, 2.0)), &HashMap::new());
        let id = &grid["<pattern id=\"".len()..grid.find("\" patternUnits").unwrap()];
        assert!(grid.starts_with(&format!(
            "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" x=\"1\" y=\"0\" width=\"2\" height=\"2\">"
        )), "{grid}");
        assert!(
            grid.contains(&format!(
                "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"url(#{id})\"/>"
            )),
            "{grid}"
        );
        let row = body(&tiles((0.0, 4.0), (2.0, 3.0), (4.0, 0.0)), &HashMap::new());
        assert!(row.contains("width=\"4\" height=\"3\">"), "{row}");
        assert!(
            row.contains("<rect x=\"0\" y=\"4\" width=\"10\" height=\"3\""),
            "{row}"
        );
        // Without a period, spaced (a step that is not the tile's size): a few tiles are
        // written one by one at their snapped boxes, many as a pattern of one cell of
        // snapped tiles (0.7.0 round-3 review, finding 7: the PNG painter's cell, not the
        // exact unsnapped tile).
        let few = body(&tiles((0.0, 0.0), (1.0, 10.0), (1.5, 0.0)), &HashMap::new());
        assert_eq!(few.matches("<rect").count(), 7, "{few}");
        assert!(!few.contains("<pattern"), "{few}");
        let mut many = tiles((0.0, 0.0), (0.0, 0.0), (0.0, 0.0));
        many.width = 1000.0;
        many.height = 1000.0;
        if let FragmentContent::Tiles(p) = &mut many.content {
            (p.tile_width, p.tile_height, p.step_x, p.step_y) = (1.0, 1.0, 1.5, 1.5);
        }
        let many = body(&many, &HashMap::new());
        assert!(
            many.contains("x=\"0\" y=\"0\" width=\"3\" height=\"3\">"),
            "{many}"
        );
        // Two snapped 1px tiles per axis (at 0 and 2): four in the cell, then the fill.
        assert_eq!(many.matches("<rect").count(), 5, "{many}");
        assert!(
            many.contains("<rect x=\"2.00\" y=\"2.00\" width=\"1.00\" height=\"1.00\""),
            "{many}"
        );
        // Fractional tiles repeated every tile size (miaskiewicz/turbo-dom#270): one
        // whole-pixel cell of them (3px for 1.5px tiles, 22 of them: at least 64px, from
        // the whole px at or before the 0.25 origin) as one `<image>` in a pattern,
        // however few tiles there are.
        let blended = body(&tiles((0.25, 0.0), (1.5, 1.5), (1.5, 1.5)), &HashMap::new());
        let id = &blended["<pattern id=\"".len()..blended.find("\" patternUnits").unwrap()];
        assert!(
            blended.starts_with(&format!(
                "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" \
                 width=\"66\" height=\"66\"><image width=\"66\" height=\"66\" \
                 preserveAspectRatio=\"none\" href=\"data:image/png;base64,iVBORw0KGgo"
            )),
            "{blended}"
        );
        assert_eq!(blended.matches("<image").count(), 1, "{blended}");
        assert!(
            blended.contains(&format!(
                "</pattern><rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"url(#{id})\"/>"
            )),
            "{blended}"
        );
        // Along a lone axis: the tile's own (snapped) row, a cell of its height.
        let row = body(&tiles((0.0, 2.25), (1.5, 3.0), (1.5, 0.0)), &HashMap::new());
        assert!(row.contains("width=\"66\" height=\"3\"><image"), "{row}");
        assert!(
            row.contains("<rect x=\"0\" y=\"2\" width=\"10\" height=\"3\""),
            "{row}"
        );
        // A lone column must keep its snapped width instead of filling the box.
        let column = body(&tiles((2.25, 0.0), (3.0, 1.5), (0.0, 1.5)), &HashMap::new());
        assert!(
            column.contains("width=\"3\" height=\"66\"><image"),
            "{column}"
        );
        assert!(
            column.contains("<rect x=\"2\" y=\"0\" width=\"3\" height=\"10\""),
            "{column}"
        );
        // No tile meets the box: nothing.
        assert_eq!(
            body(&tiles((20.0, 0.0), (1.0, 1.0), (0.0, 2.0)), &HashMap::new()),
            ""
        );
    }

    #[test]
    fn the_document_is_sized_and_filled_with_the_canvas_background() {
        let svg = paint(
            &plain(0.0, 0.0, 1.0, 1.0, None),
            30,
            20,
            Rgba::new(0, 0, 255, 128),
            &HashMap::new(),
        );
        assert!(svg.starts_with(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="20" viewBox="0 0 30 20">
<rect width="30" height="20" fill="#0000ff" fill-opacity="0.502"/>"##
        ));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn a_marker_emits_nothing_but_its_children_do() {
        let mut marker = Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            10.0,
            10.0,
            FragmentContent::Marker(MarkerKind {
                id: 1,
                inline: true,
            }),
        );
        marker.children.push(plain(5.0, 0.0, 5.0, 10.0, Some(RED)));
        assert_eq!(
            body(&marker, &HashMap::new()),
            "<rect x=\"5.00\" y=\"0.00\" width=\"5.00\" height=\"10.00\" fill=\"#ff0000\"/>\n"
        );
    }

    fn image(name: &str) -> Fragment {
        Fragment::new(
            NodeId(0),
            1.0,
            2.0,
            4.0,
            3.0,
            FragmentContent::Image(ImagePlacement {
                name: name.to_string(),
                intrinsic_w: 2,
                intrinsic_h: 1,
                has_alpha: true,
                tint: None,
                orientation: Orientation::IDENTITY,
            }),
        )
    }

    #[test]
    fn images_embed_as_png_data_uris_else_nothing_or_a_placeholder_rect() {
        let mut images = HashMap::new();
        images.insert(
            "red.png".to_string(),
            DecodedImage {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 255, 0, 0, 255],
                intrinsic: Intrinsic::new(2, 1, true),
                placeholder: false,
            },
        );
        let embedded = body(&image("red.png"), &images);
        assert!(
            embedded.starts_with(
                r#"<image x="1.00" y="2.00" width="4.00" height="3.00" preserveAspectRatio="none" href="data:image/png;base64,iVBORw0KGgo"#
            ),
            "{embedded}"
        );
        // The next tile of the same image and size reuses it through a `<use>`, naming
        // it (miaskiewicz/turbo-dom#248); another size draws its own.
        let mut tiles = image("red.png");
        let mut second = image("red.png");
        second.x = 5.0;
        let mut third = image("red.png");
        third.y = 7.0;
        let mut wider = image("red.png");
        wider.width = 8.0;
        tiles.children = vec![second, third, wider];
        let painted = body(&tiles, &images);
        assert_eq!(painted.matches("<image").count(), 2, "{painted}");
        assert_eq!(painted.matches(" id=\"img").count(), 1, "{painted}");
        assert!(painted.contains(r#"" x="4.00" y="0.00"/>"#), "{painted}");
        assert!(painted.contains(r#"" x="0.00" y="5.00"/>"#), "{painted}");
        // A size-only entry whose pixels did not decode paints nothing, as in Chrome
        // (turbo-dom#107); absent bytes, or a size-only entry this build has no decoder
        // for, are the placeholder.
        for (name, placeholder) in [("undecodable", false), ("no-decoder", true)] {
            images.insert(
                name.to_string(),
                DecodedImage {
                    width: 0,
                    height: 0,
                    rgba: Vec::new(),
                    intrinsic: Intrinsic::new(2, 1, true),
                    placeholder,
                },
            );
        }
        let placeholder =
            "<rect x=\"1.00\" y=\"2.00\" width=\"4.00\" height=\"3.00\" fill=\"#dcdcdc\"/>\n";
        assert_eq!(body(&image("absent.png"), &images), placeholder);
        assert_eq!(body(&image("no-decoder"), &images), placeholder);
        assert_eq!(body(&image("undecodable"), &images), "");
    }

    #[test]
    fn empty_boxes_shadows_and_gradients_emit_nothing() {
        let mut f = plain(0.0, 0.0, 0.0, 10.0, Some(RED));
        if let FragmentContent::Box {
            shadow, gradient, ..
        } = &mut f.content
        {
            *shadow = Some(BoxShadow {
                offset_x: 2,
                offset_y: 2,
                blur: 0,
                spread: -1,
                color: RED,
                inset: false,
            });
            *gradient = Some(CssGradient {
                angle_deg: 90.0,
                stops: vec![
                    GradientStop {
                        color: RED,
                        pos: 0.0,
                    },
                    GradientStop {
                        color: WHITE,
                        pos: 1.0,
                    },
                ],
            });
        }
        assert_eq!(body(&f, &HashMap::new()), "");
        // One stop is no gradient line either.
        let mut one = plain(0.0, 0.0, 10.0, 10.0, None);
        if let FragmentContent::Box { gradient, .. } = &mut one.content {
            *gradient = Some(CssGradient {
                angle_deg: 0.0,
                stops: vec![GradientStop {
                    color: RED,
                    pos: 0.0,
                }],
            });
        }
        assert_eq!(body(&one, &HashMap::new()), "");
    }

    /// A rounded box with `border` sides on a 20x10 box at (1, 2) and the given radius.
    fn bordered(sides: [BorderSide; 4], radius: f32) -> Fragment {
        let mut f = plain(1.0, 2.0, 20.0, 10.0, None);
        if let FragmentContent::Box {
            border,
            border_radius,
            ..
        } = &mut f.content
        {
            let [top, right, bottom, left] = sides;
            *border = BorderEdges {
                top,
                right,
                bottom,
                left,
            };
            *border_radius = radius.into();
        }
        f
    }

    #[test]
    fn rounded_borders_fill_the_ring_between_the_outer_and_inner_outlines() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let blue = Some(Rgba::new(0, 0, 255, 255));
        let thin = side(1, Some(RED));
        // One colour on every side: one even-odd ring, the outer outline (radius 40 fitted
        // to the 20x10 box: 5) and the inner one (the padding box, radius 5 less 2: 3); a
        // translucent colour carries a fill opacity (miaskiewicz/turbo-dom#253).
        let half = Some(Rgba::new(255, 0, 0, 128));
        assert_eq!(
            body(&bordered([side(2, half); 4], 40.0), &HashMap::new()),
            "<path d=\"M6.00 2.00 L16.00 2.00 C18.76 2.00 21.00 4.24 21.00 7.00 L21.00 7.00 \
             C21.00 9.76 18.76 12.00 16.00 12.00 L6.00 12.00 C3.24 12.00 1.00 9.76 1.00 7.00 \
             L1.00 7.00 C1.00 4.24 3.24 2.00 6.00 2.00 Z M6.00 4.00 L16.00 4.00 C17.66 4.00 \
             19.00 5.34 19.00 7.00 L19.00 7.00 C19.00 8.66 17.66 10.00 16.00 10.00 L6.00 10.00 \
             C4.34 10.00 3.00 8.66 3.00 7.00 L3.00 7.00 C3.00 5.34 4.34 4.00 6.00 4.00 Z\" \
             fill-rule=\"evenodd\" fill=\"#ff0000\" fill-opacity=\"0.502\"/>\n"
        );
        // Different sides: the ring once per painted side, clipped to the side's region
        // (cut at the corner diagonals), in its own colour.
        let mixed = body(
            &bordered([thin, side(4, blue), thin, side(0, blue)], 6.0),
            &HashMap::new(),
        );
        assert_eq!(mixed.matches("<clipPath").count(), 3, "{mixed}");
        let top = mixed.find("M1.00 2.00 L21.00 2.00 L1.00 7.00 Z").unwrap();
        let right = mixed.find("M21.00 2.00 L21.00 12.00 L1.00 7.00 Z").unwrap();
        let bottom = mixed.find("M21.00 12.00 L1.00 12.00 L1.00 7.00 Z").unwrap();
        assert!(top < right && right < bottom, "{mixed}");
        assert_eq!(mixed.matches("fill=\"#ff0000\"").count(), 2);
        assert_eq!(mixed.matches("fill=\"#0000ff\"").count(), 1);
        // A border wider than the box leaves no inner outline: the whole outline fills.
        let wide = body(&bordered([side(30, blue); 4], 6.0), &HashMap::new());
        assert!(
            wide.starts_with("<path d=\"M6.00 2.00 L16.00 2.00"),
            "{wide}"
        );
        assert_eq!(wide.matches(" M").count(), 0, "{wide}");
        // No visible side or only transparent ones: nothing; a colourless side is left
        // out while the others paint.
        let none = side(0, blue);
        assert_eq!(body(&bordered([none; 4], 6.0), &HashMap::new()), "");
        let clear = side(3, Some(Rgba::new(0, 0, 255, 0)));
        assert_eq!(body(&bordered([clear; 4], 6.0), &HashMap::new()), "");
        let colourless = body(
            &bordered([side(3, None), thin, thin, thin], 6.0),
            &HashMap::new(),
        );
        assert_eq!(colourless.matches("<clipPath").count(), 3, "{colourless}");
        // At a radius of 0.5 or less the four square edge bands paint instead.
        assert_eq!(
            body(
                &bordered([side(2, blue), none, none, none], 0.5),
                &HashMap::new()
            ),
            "<rect x=\"1.00\" y=\"2.00\" width=\"20.00\" height=\"2.00\" fill=\"#0000ff\"/>\n"
        );
    }

    fn decorated(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        edit: impl FnOnce(&mut Option<BoxShadow>, &mut Option<CssGradient>, &mut Option<Transform2D>),
    ) -> Fragment {
        let mut f = plain(x, y, w, h, None);
        if let FragmentContent::Box {
            border_radius,
            shadow,
            gradient,
            transform,
            ..
        } = &mut f.content
        {
            *border_radius = radius.into();
            edit(shadow, gradient, transform);
        }
        f
    }

    fn shadow(
        offset_x: i32,
        offset_y: i32,
        blur: u32,
        spread: i32,
        color: Rgba,
        inset: bool,
    ) -> BoxShadow {
        BoxShadow {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
            inset,
        }
    }

    #[test]
    fn transforms_wrap_the_box_about_its_absolute_origin() {
        let f = decorated(10.0, 20.0, 40.0, 30.0, 0.0, |_, _, t| {
            *t = Some(Transform2D {
                matrix: [0.0, 1.0, -1.0, 0.5, 5.0, 6.0],
                origin_x: 20.0,
                origin_y: 15.0,
            });
        });
        assert_eq!(
            body(&f, &HashMap::new()),
            "<g transform=\"translate(30.00 35.00) matrix(0 1 -1 0.5 5 6) translate(-30.00 -35.00)\"></g>"
        );
    }

    #[test]
    fn shadows_are_the_offset_spread_silhouette_blurred_by_a_filter() {
        let blue = Rgba::new(0, 0, 255, 255);
        let svg = |radius: f32, sh: Option<BoxShadow>| {
            body(
                &decorated(10.0, 10.0, 20.0, 10.0, radius, |s, _, _| *s = sh),
                &HashMap::new(),
            )
        };
        assert_eq!(
            svg(0.0, Some(shadow(5, 7, 0, 3, blue, false))),
            "<rect x=\"12.00\" y=\"14.00\" width=\"26.00\" height=\"16.00\" fill=\"#0000ff\"/>\n"
        );
        assert_eq!(
            svg(4.0, Some(shadow(2, 3, 6, 0, blue, false))),
            "<filter id=\"sh130\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"3.00\"/></filter>\n\
             <rect x=\"12.00\" y=\"13.00\" width=\"20.00\" height=\"10.00\" rx=\"4.00\" ry=\"4.00\" fill=\"#0000ff\" filter=\"url(#sh130)\"/>\n"
        );
        // A 0.5px radius is square; the spread scales below the radius (CSS); a 400px blur
        // is capped at 256 (stdDeviation 128).
        assert!(!svg(0.5, Some(shadow(0, 0, 4, 0, blue, false))).contains("rx="));
        assert!(svg(2.0, Some(shadow(0, 0, 0, 8, blue, false))).contains("rx=\"6.62\""));
        assert!(
            svg(0.0, Some(shadow(0, 0, 400, 0, blue, false))).contains("stdDeviation=\"128.00\"")
        );
        // An inset shadow is a masked padding box (below); a transparent one emits nothing;
        // a half-pixel one still does.
        assert_eq!(svg(0.0, Some(shadow(0, 5, 0, 0, blue, true))), "<mask id=\"inset130\" maskUnits=\"userSpaceOnUse\" x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\">\
             <rect x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\" fill=\"#ffffff\"/>\
             <rect x=\"10.00\" y=\"15.00\" width=\"20.00\" height=\"10.00\" fill=\"#000000\"/></mask>\
             <g mask=\"url(#inset130)\"><rect x=\"10.00\" y=\"10.00\" width=\"20.00\" height=\"10.00\" fill=\"#0000ff\"/>\n</g>\n"
        );
        // A blurred inset hole is blurred inside the mask; its radius is the padding-box
        // radius less the spread, clamped to half the hole.
        let blurred = svg(9.0, Some(shadow(0, 0, 4, 2, blue, true)));
        assert!(
            blurred.contains(
                "<filter id=\"insetblur130\" filterUnits=\"userSpaceOnUse\" x=\"4.00\" y=\"4.00\" width=\"32.00\" height=\"22.00\"><feGaussianBlur stdDeviation=\"2.00\"/></filter>\
                 <rect x=\"12.00\" y=\"12.00\" width=\"16.00\" height=\"6.00\" rx=\"3.00\" ry=\"3.00\" fill=\"#000000\" filter=\"url(#insetblur130)\"/>"
            ),
            "{blurred}"
        );
        // A spread past half the box leaves no hole: the whole padding box is the colour.
        let full = svg(0.0, Some(shadow(0, 0, 0, 6, blue, true)));
        assert!(
            full.contains("fill=\"#ffffff\"/></mask>") && !full.contains("#000000"),
            "{full}"
        );
        // A box with no padding box (its borders fill it) paints no inset shadow.
        assert_eq!(
            body(
                &decorated(1.0, 2.0, 0.0, 0.0, 0.0, |s, _, _| {
                    *s = Some(shadow(0, 0, 0, 0, blue, true));
                }),
                &HashMap::new(),
            ),
            ""
        );
        assert_eq!(
            svg(0.0, Some(shadow(0, 5, 0, 0, Rgba::new(0, 0, 255, 0), true))),
            ""
        );
        assert_eq!(
            svg(
                0.0,
                Some(shadow(0, 5, 0, 0, Rgba::new(0, 0, 255, 0), false))
            ),
            ""
        );
        for (w, h) in [(0.5, 10.0), (10.0, 0.5)] {
            let half = body(
                &decorated(1.0, 2.0, w, h, 0.0, |s, _, _| {
                    *s = Some(shadow(0, 0, 0, 0, blue, false));
                }),
                &HashMap::new(),
            );
            assert_eq!(half, format!("<rect x=\"1.00\" y=\"2.00\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"#0000ff\"/>\n"));
        }
    }

    #[test]
    fn gradient_lines_reach_the_box_corners_along_the_css_angle() {
        let grad = |angle_deg: f32| {
            Some(CssGradient {
                angle_deg,
                stops: vec![
                    GradientStop {
                        color: RED,
                        pos: 0.0,
                    },
                    GradientStop {
                        color: WHITE,
                        pos: 1.0,
                    },
                ],
            })
        };
        for angle in [30.0_f32, 135.0, 200.0, 290.0] {
            let (x, y, w, h) = (10.0_f32, 20.0_f32, 60.0_f32, 30.0_f32);
            let out = body(
                &decorated(x, y, w, h, 0.0, |_, g, _| *g = grad(angle)),
                &HashMap::new(),
            );
            let num = |k: &str| -> f32 {
                let at = out.find(&format!(" {k}=\"")).unwrap() + k.len() + 3;
                out[at..at + out[at..].find('"').unwrap()].parse().unwrap()
            };
            let (x1, y1, x2, y2) = (num("x1"), num("y1"), num("x2"), num("y2"));
            // The line points along (sin, -cos) of the angle, through the box centre...
            let len = (x2 - x1).hypot(y2 - y1);
            let t = angle.to_radians();
            assert!(
                ((x2 - x1) / len - t.sin()).abs() < 1e-3
                    && ((y2 - y1) / len + t.cos()).abs() < 1e-3,
                "{angle}"
            );
            assert!(
                ((x1 + x2) / 2.0 - (x + w / 2.0)).abs() < 0.01
                    && ((y1 + y2) / 2.0 - (y + h / 2.0)).abs() < 0.01
            );
            // ...and spans the box exactly: its corners project onto 0 and 1.
            let proj =
                |px: f32, py: f32| ((px - x1) * (x2 - x1) + (py - y1) * (y2 - y1)) / (len * len);
            let corners = [
                proj(x, y),
                proj(x + w, y),
                proj(x, y + h),
                proj(x + w, y + h),
            ];
            let (lo, hi) = corners
                .iter()
                .fold((f32::MAX, f32::MIN), |(a, b), &c| (a.min(c), b.max(c)));
            assert!(
                lo.abs() < 1e-3 && (hi - 1.0).abs() < 1e-3,
                "{angle}: {lo} {hi}"
            );
            assert!(out.ends_with(
                "<rect x=\"10.00\" y=\"20.00\" width=\"60.00\" height=\"30.00\" fill=\"url(#g130)\"/>\n"
            ), "{out}");
        }
        // Half a pixel wide or high still emits the gradient; rounded boxes round it.
        for (w, h) in [(0.5, 10.0), (10.0, 0.5)] {
            assert!(body(
                &decorated(1.0, 1.0, w, h, 0.0, |_, g, _| *g = grad(90.0)),
                &HashMap::new()
            )
            .contains("<linearGradient"));
        }
        let round = body(
            &decorated(0.0, 0.0, 40.0, 40.0, 6.0, |_, g, _| *g = grad(90.0)),
            &HashMap::new(),
        );
        assert!(
            round.contains("rx=\"6.00\" ry=\"6.00\" fill=\"url(#g"),
            "{round}"
        );
        let square = body(
            &decorated(0.0, 0.0, 40.0, 40.0, 0.5, |_, g, _| *g = grad(90.0)),
            &HashMap::new(),
        );
        assert!(!square.contains("rx="), "{square}");
    }

    /// SVG path data of a glyph straight from ttf-parser, mapped by hand.
    struct Data(String, f32, f32, f32);

    impl ttf_parser::OutlineBuilder for Data {
        fn move_to(&mut self, x: f32, y: f32) {
            let (px, py) = (self.1 + x * self.3, self.2 - y * self.3);
            let _ = write!(self.0, "M{px:.2} {py:.2} ");
        }
        fn line_to(&mut self, x: f32, y: f32) {
            let (px, py) = (self.1 + x * self.3, self.2 - y * self.3);
            let _ = write!(self.0, "L{px:.2} {py:.2} ");
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (self.1 + x * self.3, self.2 - y * self.3);
            let ((a, b), (c, d)) = (m(cx, cy), m(x, y));
            let _ = write!(self.0, "Q{a:.2} {b:.2} {c:.2} {d:.2} ");
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (self.1 + x * self.3, self.2 - y * self.3);
            let ((a, b), (c, d), (e, f)) = (m(ax, ay), m(bx, by), m(x, y));
            let _ = write!(self.0, "C{a:.2} {b:.2} {c:.2} {d:.2} {e:.2} {f:.2} ");
        }
        fn close(&mut self) {
            self.0 += "Z ";
        }
    }

    #[test]
    fn text_paths_are_every_glyph_outline_at_its_pen_position() {
        let reg = FontRegistry::new();
        for family in ["Inter", "Liberation Serif"] {
            let face = reg.select(&[family], 400, false).unwrap().clone();
            let ttf = crate::glyph::outline_face(&face);
            let scale = 24.0 / f32::from(face.units_per_em());
            let mut x = 0.0;
            let glyphs: Vec<PositionedGlyph> = face
                .shape("Ag%")
                .iter()
                .map(|g| {
                    let p = PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x,
                        y: 20.0,
                        text_start: 0,
                        text_end: 0,
                    };
                    x += g.x_advance as f32 * scale;
                    p
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
                    font_size: 24.0,
                    color: RED,
                    vertical: false,
                    upright: false,
                },
            );
            let mut d = Data(String::new(), 0.0, 0.0, scale);
            for g in &glyphs {
                (d.1, d.2) = (7.0 + g.x, 4.0 + g.y);
                ttf.outline_glyph(ttf_parser::GlyphId(g.glyph_id), &mut d);
            }
            // The same commands, each point within a pixel of the bare outline's: the
            // path is the outline Chrome fills, grown and snapped a fraction of a pixel
            // (#266).
            let got = body(&line, &HashMap::new());
            // Red's luminance class is 3: the path takes that class's pre-blend filter,
            // defined after the content (#294).
            let (path, filter) = got.split_once('\n').expect("a path, then its filter");
            assert!(filter.starts_with("<filter id=\"glyph-lum3\""), "{filter}");
            let path = path
                .strip_prefix("<path d=\"")
                .and_then(|p| p.strip_suffix("\" fill=\"#ff0000\" filter=\"url(#glyph-lum3)\"/>"))
                .expect("one red path");
            let (got, want): (Vec<&str>, Vec<&str>) = (
                path.split_whitespace().collect(),
                d.0.split_whitespace().collect(),
            );
            assert_eq!(got.len(), want.len(), "{family}");
            for (g, w) in got.iter().zip(&want) {
                let num = |t: &str| t.trim_start_matches(char::is_alphabetic).parse::<f32>();
                let command = |t: &str| t.chars().take_while(|c| c.is_alphabetic()).count();
                assert_eq!(&g[..command(g)], &w[..command(w)], "{family}");
                let (Ok(g), Ok(w)) = (num(g), num(w)) else {
                    continue;
                };
                assert!((g - w).abs() < 1.0, "{family}: {g} vs {w}");
            }
        }
    }

    #[test]
    fn square_borders_are_four_edge_bands_of_their_own_widths() {
        let side = |width: u16, color: Option<Rgba>| BorderSide { width, color };
        let blue = Some(Rgba::new(0, 0, 255, 255));
        let mut f = plain(10.0, 20.0, 40.0, 30.0, None);
        if let FragmentContent::Box { border, .. } = &mut f.content {
            *border = BorderEdges {
                top: side(1, blue),
                right: side(2, blue),
                bottom: side(3, blue),
                left: side(4, blue),
            };
        }
        let r = |x: f32, y: f32, w: f32, h: f32| {
            format!("<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"#0000ff\"/>\n")
        };
        assert_eq!(
            body(&f, &HashMap::new()),
            [
                r(10.0, 20.0, 40.0, 1.0),
                r(10.0, 47.0, 40.0, 3.0),
                r(10.0, 20.0, 4.0, 30.0),
                r(48.0, 20.0, 2.0, 30.0)
            ]
            .concat()
        );
        // Zero-width or colourless sides are no band.
        if let FragmentContent::Box { border, .. } = &mut f.content {
            *border = BorderEdges {
                top: side(0, blue),
                right: side(3, None),
                bottom: side(0, blue),
                left: side(0, blue),
            };
        }
        assert_eq!(body(&f, &HashMap::new()), "");
    }

    #[test]
    fn backgrounds_need_a_size_a_colour_and_round_past_half_a_pixel() {
        let bg = |w: f32, h: f32, c: Rgba, radius: f32| {
            let mut f = plain(1.0, 2.0, w, h, Some(c));
            if let FragmentContent::Box { border_radius, .. } = &mut f.content {
                *border_radius = radius.into();
            }
            body(&f, &HashMap::new())
        };
        assert_eq!(bg(0.0, 5.0, RED, 0.0), "");
        assert_eq!(bg(5.0, 0.0, RED, 0.0), "");
        assert_eq!(bg(5.0, -1.0, RED, 0.0), "");
        assert_eq!(bg(5.0, 5.0, Rgba::new(255, 0, 0, 0), 0.0), "");
        assert_eq!(
            bg(5.0, 5.0, RED, 0.5),
            "<rect x=\"1.00\" y=\"2.00\" width=\"5.00\" height=\"5.00\" fill=\"#ff0000\"/>\n"
        );
        assert_eq!(
            bg(5.0, 5.0, RED, 0.75),
            "<rect x=\"1.00\" y=\"2.00\" width=\"5.00\" height=\"5.00\" rx=\"0.75\" ry=\"0.75\" fill=\"#ff0000\"/>\n"
        );
    }

    #[test]
    fn text_emits_one_outline_path_unless_nothing_has_an_outline() {
        let face = FontRegistry::new()
            .select(&["serif"], 400, false)
            .unwrap()
            .clone();
        let line = |text: &str, color: Rgba| {
            let glyphs = face
                .shape(text)
                .iter()
                .scan(0.0, |pen, g| {
                    let x = *pen;
                    *pen += g.x_advance as f32 * 20.0 / f32::from(face.units_per_em());
                    Some(PositionedGlyph {
                        glyph_id: g.glyph_id,
                        x,
                        y: 15.0,
                        text_start: 0,
                        text_end: 0,
                    })
                })
                .collect();
            Fragment::new(
                NodeId(0),
                0.0,
                0.0,
                10.0,
                20.0,
                FragmentContent::TextLine {
                    glyphs,
                    text: String::new(),
                    face: face.clone(),
                    font_size: 20.0,
                    color,
                    vertical: false,
                    upright: false,
                },
            )
        };
        let ink = body(&line("Ag", RED), &HashMap::new());
        assert!(ink.starts_with("<path d=\"M"), "{ink}");
        // A TrueType (quadratic) face traces `Q` segments.
        assert!(
            ink.contains(" Q") || ink.starts_with("<path d=\"Q"),
            "{ink}"
        );
        assert!(
            ink.contains(" fill=\"#ff0000\" filter=\"url(#glyph-lum3)\"/>\n<filter "),
            "{ink}"
        );
        assert_eq!(
            body(&line(" ", RED), &HashMap::new()),
            "",
            "a space has no outline"
        );
        assert_eq!(
            body(&line("Ag", Rgba::new(255, 0, 0, 0)), &HashMap::new()),
            ""
        );
    }

    #[test]
    fn preblend_darkens_dark_text_and_lightens_light_text_edges() {
        let black = preblend_class(luminance_class(0, 0, 0));
        let white = preblend_class(luminance_class(255, 255, 255));
        let blue = preblend_class(luminance_class(0, 0, 255));
        assert_eq!((black[0], black[255]), (0.0, 1.0));
        assert_eq!((white[0], white[255]), (0.0, 1.0));
        // Half coverage: black ink 1 - (1 - 0.75)^(1/1.8), white 0.5^(1/1.8).
        assert!((black[128] - 0.537).abs() < 0.01, "{}", black[128]);
        assert!((white[128] - 0.684).abs() < 0.01, "{}", white[128]);
        // Blue's low luminance blends almost like black.
        assert!((blue[128] - black[128]).abs() < 0.02, "{}", blue[128]);
        // A mid grey sits between.
        let grey = preblend_class(luminance_class(128, 128, 128));
        assert!(grey[128] > 0.0 && grey[128] < 1.0);
        // One table per 3-bit luminance class.
        assert_eq!(
            (luminance_class(0, 0, 0), luminance_class(0, 0, 255)),
            (0, 1)
        );
        assert_eq!(luminance_class(255, 255, 255), 7);
    }

    #[test]
    fn preblend_classes_keep_the_fixed_three_bit_luminances() {
        // abc expands to abcabcab; this fixed reference is independent of the
        // production packing expression and its derived pre-blend tables.
        let expected = [0, 36, 73, 109, 146, 182, 219, 255];
        for (class, luminance) in expected.into_iter().enumerate() {
            assert_eq!(class_luminance(class as u32), luminance);
        }
    }

    #[test]
    fn glyph_runs_share_one_pre_blend_filter_per_luminance_class() {
        // miaskiewicz/turbo-dom#294: each run's coverage goes through its text colour's
        // pre-blend table, one filter per luminance class the document uses.
        let face = FontRegistry::new()
            .select(&["serif"], 400, false)
            .unwrap()
            .clone();
        let glyph_id = face.shape("H")[0].glyph_id;
        let run = |x: f32, color: Rgba| {
            Fragment::new(
                NodeId(0),
                x,
                0.0,
                10.0,
                20.0,
                FragmentContent::TextLine {
                    glyphs: vec![PositionedGlyph {
                        glyph_id,
                        x: 0.0,
                        y: 15.0,
                        text_start: 0,
                        text_end: 0,
                    }],
                    text: String::new(),
                    face: face.clone(),
                    font_size: 20.0,
                    color,
                    vertical: false,
                    upright: false,
                },
            )
        };
        let mut page = plain(0.0, 0.0, 100.0, 20.0, None);
        page.children = vec![
            run(0.0, Rgba::new(0, 0, 0, 255)),
            // Black's class (0) again, at half alpha: the alpha is the path's opacity,
            // applied after the filter.
            run(20.0, Rgba::new(0, 0, 0, 128)),
            run(40.0, WHITE),
        ];
        let svg = body(&page, &HashMap::new());
        assert_eq!(
            svg.matches("filter=\"url(#glyph-lum0)\"").count(),
            2,
            "{svg}"
        );
        assert!(
            svg.contains("fill=\"#000000\" opacity=\"0.502\" filter="),
            "{svg}"
        );
        assert_eq!(
            svg.matches("filter=\"url(#glyph-lum7)\"").count(),
            1,
            "{svg}"
        );
        assert_eq!(svg.matches("<filter ").count(), 2, "{svg}");
        // Each filter's table is the raster painter's, one value per 8-bit coverage.
        for class in [0, 7] {
            let def = format!(
                "<filter id=\"glyph-lum{class}\" color-interpolation-filters=\"sRGB\">\
                 <feComponentTransfer><feFuncA type=\"table\" tableValues=\""
            );
            let at = svg.find(&def).expect("filter defined") + def.len();
            let values: Vec<f32> = svg[at..svg[at..].find('"').unwrap() + at]
                .split(' ')
                .map(|v| v.parse().unwrap())
                .collect();
            let lut = preblend_class(class);
            assert_eq!(values.len(), 256);
            for (v, want) in values.iter().zip(lut) {
                assert!((v - want).abs() < 1e-4, "{class}: {v} vs {want}");
            }
        }
        // No glyph, no filter.
        assert!(!body(&plain(0.0, 0.0, 5.0, 5.0, Some(RED)), &HashMap::new()).contains("<filter"));
    }

    #[test]
    fn opacity_wraps_the_box_and_its_subtree_in_one_group() {
        // miaskiewicz/turbo-dom#206: `opacity` is a group opacity over the whole subtree.
        let red = Rgba::new(255, 0, 0, 255);
        let mut half = plain(0.0, 0.0, 8.0, 8.0, Some(red));
        half.children.push(plain(0.0, 0.0, 4.0, 4.0, Some(red)));
        half.opacity = 0.5;
        let got = body(&half, &HashMap::new());
        assert!(got.starts_with("<g opacity=\"0.5\">"), "{got}");
        assert!(got.ends_with("</g>"), "{got}");
        assert_eq!(got.matches("<rect").count(), 2, "{got}");
        let mut gone = plain(0.0, 0.0, 8.0, 8.0, Some(red));
        gone.opacity = 0.0;
        assert_eq!(body(&gone, &HashMap::new()), "");
        assert!(!body(&plain(0.0, 0.0, 8.0, 8.0, Some(red)), &HashMap::new()).contains("opacity"));
    }

    #[test]
    fn overflow_clips_children_with_a_clip_path_of_the_padding_box() {
        let clipped = |w: f32, r: f32, child: Fragment| {
            let mut f = plain(1.0, 1.0, 8.0, 8.0, None);
            f.overflow_clip = Some(turbo_dom_layout::OverflowClip {
                x: 1.0,
                y: 1.0,
                width: w,
                height: 6.0,
                radii: r.into(),
            });
            f.children = vec![child];
            body(&f, &HashMap::new())
        };
        let big = plain(0.0, 0.0, 20.0, 20.0, Some(RED));
        // miaskiewicz/turbo-dom#253: square and rounded regions, an empty one (no child),
        // and children inside a square region (no clip path).
        assert_eq!(
            clipped(6.0, 0.0, big.clone()),
            format!(
                "<clipPath id=\"overflow130\"><rect x=\"2.00\" y=\"2.00\" width=\"6.00\" height=\"6.00\"/></clipPath><g clip-path=\"url(#overflow130)\">{}</g>",
                body(&big, &HashMap::new())
            )
        );
        assert!(clipped(6.0, 2.0, big.clone()).contains("rx=\"2.00\" ry=\"2.00\"/></clipPath>"));
        assert_eq!(clipped(0.0, 0.0, big), "");
        let small = plain(3.0, 3.0, 2.0, 2.0, Some(RED));
        assert_eq!(
            clipped(6.0, 0.0, small.clone()),
            body(&small, &HashMap::new())
        );
        // Elliptical corners clip with a path.
        let mut f = plain(1.0, 1.0, 8.0, 8.0, Some(RED));
        if let FragmentContent::Box { border_radius, .. } = &mut f.content {
            border_radius.top_left = turbo_dom_layout::CornerRadii { h: 4.0, v: 2.0 };
        }
        assert!(body(&f, &HashMap::new()).starts_with("<path d=\"M5.00 1.00 L9.00 1.00"));
    }

    /// Every `id="…"` the document defines.
    fn ids(svg: &str) -> Vec<String> {
        svg.split(" id=\"")
            .skip(1)
            .map(|rest| rest[..rest.find('"').unwrap()].to_string())
            .collect()
    }

    /// A `ColumnRule` fragment with `(style, color)` at `(x, y)` size `(w, h)`.
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

    #[test]
    fn clipped_svg_uses_both_relative_offsets_and_empty_axes() {
        let f = plain(5.0, 7.0, 20.0, 30.0, Some(RED));
        let clip = ClipRect {
            x: 2.0,
            y: 3.0,
            width: 11.0,
            height: 13.0,
            radii: BorderRadii::ZERO,
        };
        let mut svg = String::new();
        paint_clipped(&mut svg, &f, clip, &HashMap::new());
        assert!(svg.starts_with("<clipPath id=\"clip0\"><rect x=\"7.00\" y=\"10.00\" width=\"11.00\" height=\"13.00\""), "{svg}");
        assert!(svg.contains("<g clip-path=\"url(#clip0)\">"), "{svg}");
        assert!(
            svg.contains("x=\"5.00\" y=\"7.00\" width=\"20.00\" height=\"30.00\""),
            "{svg}"
        );
        assert!(svg.ends_with("</g>"), "{svg}");
        for (width, height) in [(0.0, 13.0), (11.0, 0.0)] {
            let mut svg = String::new();
            paint_clipped(
                &mut svg,
                &f,
                ClipRect {
                    width,
                    height,
                    ..clip
                },
                &HashMap::new(),
            );
            assert!(svg.is_empty(), "{width}x{height}: {svg}");
        }
    }

    #[test]
    fn transparent_outer_shadow_emits_no_unused_filter() {
        let f = decorated(2.0, 3.0, 10.0, 20.0, 0.0, |s, _, _| {
            *s = Some(shadow(1, 2, 4, 0, Rgba::new(255, 0, 0, 0), false));
        });
        assert_eq!(body(&f, &HashMap::new()), "");
    }

    #[test]
    fn rule_svg_geometry_keeps_independent_pattern_boundaries() {
        let render = |w, h, style| body(&rule(2.0, 3.0, w, h, style, RED), &HashMap::new());
        let short = render(2.0, 1.0, ColumnRuleStyle::Dotted);
        assert_eq!(short.matches("<circle ").count(), 1, "{short}");
        assert!(
            short.contains("cx=\"3.00\" cy=\"4.00\" r=\"1.00\""),
            "{short}"
        );
        let dots = render(2.0, 8.0, ColumnRuleStyle::Dotted);
        assert_eq!(dots.matches("<circle ").count(), 2, "{dots}");
        assert!(dots.contains("cy=\"4.00\""), "{dots}");
        assert!(dots.contains("cy=\"8.00\""), "{dots}");
        let subpixel = render(0.5, 3.5, ColumnRuleStyle::Dotted);
        for expected in [
            "x=\"2.00\" y=\"3.00\" width=\"1.00\" height=\"1.00\"",
            "x=\"2.00\" y=\"4.00\" width=\"1.00\" height=\"1.00\"",
            "x=\"2.00\" y=\"6.00\" width=\"1.00\" height=\"0.50\"",
        ] {
            assert!(subpixel.contains(expected), "{expected}: {subpixel}");
        }
        assert_eq!(subpixel.matches("<rect ").count(), 3, "{subpixel}");
        let dashes = render(2.0, 15.0, ColumnRuleStyle::Dashed);
        for expected in [
            "y=\"3.00\" width=\"2.00\" height=\"4.00\"",
            "y=\"9.00\" width=\"2.00\" height=\"4.00\"",
            "y=\"15.00\" width=\"2.00\" height=\"3.00\"",
        ] {
            assert!(dashes.contains(expected), "{expected}: {dashes}");
        }
        assert_eq!(dashes.matches("<rect ").count(), 3, "{dashes}");
        let narrow_dashes = render(1.0, 8.0, ColumnRuleStyle::Dashed);
        assert_eq!(
            narrow_dashes.matches("<rect ").count(),
            3,
            "{narrow_dashes}"
        );
        for y in ["3.00", "6.00", "9.00"] {
            assert!(
                narrow_dashes.contains(&format!("y=\"{y}\" width=\"1.00\" height=\"2.00\"")),
                "{narrow_dashes}"
            );
        }
        let double = render(3.0, 8.0, ColumnRuleStyle::Double);
        assert_eq!(double.matches("<rect ").count(), 2, "{double}");
        assert!(
            double.contains("x=\"2.00\" y=\"3.00\" width=\"1.00\""),
            "{double}"
        );
        assert!(
            double.contains("x=\"4.00\" y=\"3.00\" width=\"1.00\""),
            "{double}"
        );
    }

    #[test]
    fn rule_svg_helpers_reject_each_invalid_axis_independently() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let f = rule(2.0, 3.0, 2.0, bad, ColumnRuleStyle::Dotted, RED);
            let mut svg = String::new();
            paint_dotted_rule(&mut svg, &f, RED);
            paint_dashed_rule(&mut svg, &f, RED);
            paint_double_rule(&mut svg, &f, RED);
            paint_shaded_rule(&mut svg, &f, ColumnRuleStyle::Groove, RED);
            assert!(svg.is_empty(), "height {bad}: {svg}");
        }
        for (width, height) in [(0.0, 8.0), (2.0, 0.0), (-1.0, 8.0), (2.0, -1.0)] {
            let f = rule(2.0, 3.0, width, height, ColumnRuleStyle::Groove, RED);
            let mut svg = String::new();
            paint_shaded_rule(&mut svg, &f, ColumnRuleStyle::Groove, RED);
            assert!(svg.is_empty(), "{width}x{height}: {svg}");
        }
    }

    #[test]
    fn dotted_rule_origin_accounts_for_half_a_pixel_precision() {
        let f = rule(2.0, 8_388_608.0, 1.0, 8.0, ColumnRuleStyle::Dotted, RED);
        assert!(rule_needs_local_origin(&f, ColumnRuleStyle::Dotted));
        let svg = body(&f, &HashMap::new());
        assert!(
            svg.starts_with("<g transform=\"translate(0 8388608)\">"),
            "{svg}"
        );
        assert_eq!(svg.matches("<circle ").count(), 4, "{svg}");
        let invalid = rule(
            2.0,
            8_388_608.0,
            1.0,
            f32::NAN,
            ColumnRuleStyle::Dotted,
            RED,
        );
        assert_eq!(body(&invalid, &HashMap::new()), "");
    }

    /// miaskiewicz/turbo-dom#398 PR-D: the SVG painter's `paint_column_rule`
    /// must skip a transparent, zero-width or zero-height fragment — without
    /// the guard the Dotted / Dashed helpers would infinite-loop on a zero
    /// width.
    #[test]
    fn paint_column_rule_svg_is_a_no_op_on_degenerate_fragments() {
        for style in [
            ColumnRuleStyle::Solid,
            ColumnRuleStyle::Dotted,
            ColumnRuleStyle::Dashed,
            ColumnRuleStyle::Double,
        ] {
            let transparent = Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 0,
            };
            let body_t = body(
                &rule(1.0, 1.0, 4.0, 8.0, style, transparent),
                &HashMap::new(),
            );
            assert!(
                body_t.is_empty(),
                "alpha 0 must emit no rule markup ({style:?}): {body_t}"
            );
            let body_w = body(&rule(1.0, 1.0, 0.0, 8.0, style, RED), &HashMap::new());
            assert!(
                body_w.is_empty(),
                "zero width must emit no rule markup ({style:?}): {body_w}"
            );
            let body_h = body(&rule(1.0, 1.0, 4.0, 0.0, style, RED), &HashMap::new());
            assert!(
                body_h.is_empty(),
                "zero height must emit no rule markup ({style:?}): {body_h}"
            );
        }
    }

    /// miaskiewicz/turbo-dom#398 PR-D: a `Double` rule below 3 px wide falls
    /// back to a single solid rect so a thin double still shows (CSS
    /// Backgrounds 3 §4.2). The two-band path would emit two `<rect>`s each
    /// one band wide; this path emits exactly one at the full width.
    #[test]
    fn paint_column_rule_svg_thin_double_emits_one_solid_rect() {
        let svg = body(
            &rule(1.0, 1.0, 2.0, 6.0, ColumnRuleStyle::Double, RED),
            &HashMap::new(),
        );
        // A thick double (band >= 1 and width >= 3) produces two rects; this
        // fallback stays at exactly one.
        assert_eq!(svg.matches("<rect ").count(), 1, "got: {svg}");
        assert!(svg.contains("width=\"2.00\""), "full width: {svg}");
    }

    /// `paint_dotted_rule` and `paint_dashed_rule` both guard `period <= 0.0`
    /// so a zero-width fragment cannot infinite-loop (`cy += 0` / `dash_top +=
    /// 0`). `paint_column_rule` guards the caller first, so the per-helper
    /// guard is a contract pin, exercised here directly.
    #[test]
    fn dotted_and_dashed_svg_helpers_return_on_zero_width_fragments() {
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
        let mut buf = String::new();
        paint_dotted_rule(&mut buf, &f, RED);
        paint_dashed_rule(&mut buf, &f, RED);
        assert!(
            buf.is_empty(),
            "zero-width helpers must not emit markup or loop: {buf}"
        );
    }

    // v0.8.2 R1 adversarial M1 + L1: a `NaN` or `Infinity` width / height
    // slips past the `f.width <= 0.0 || f.height <= 0.0` guard because
    // every comparison with `NaN` is false (and `Infinity > 0.0`), and
    // would cascade into `cx = NaN / 2.0` / `period = 2 * NaN` — the SVG
    // painter would emit `cx="NaN"` or run an undefined loop. The guard
    // must reject both.
    #[test]
    fn paint_column_rule_svg_is_a_no_op_on_non_finite_fragments() {
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for style in [
                ColumnRuleStyle::Solid,
                ColumnRuleStyle::Dotted,
                ColumnRuleStyle::Dashed,
                ColumnRuleStyle::Double,
            ] {
                let body_w = body(&rule(1.0, 1.0, bad, 8.0, style, RED), &HashMap::new());
                assert!(
                    body_w.is_empty(),
                    "non-finite width must emit no rule markup ({style:?}, {bad}): {body_w}"
                );
                let body_h = body(&rule(1.0, 1.0, 4.0, bad, style, RED), &HashMap::new());
                assert!(
                    body_h.is_empty(),
                    "non-finite height must emit no rule markup ({style:?}, {bad}): {body_h}"
                );
            }
        }
    }

    // Helpers' direct contract: none of them may paint or loop on a
    // non-finite width.
    #[test]
    fn rule_svg_helpers_are_a_no_op_on_non_finite_fragments() {
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
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
            let mut buf = String::new();
            paint_dotted_rule(&mut buf, &f, RED);
            paint_dashed_rule(&mut buf, &f, RED);
            paint_double_rule(&mut buf, &f, RED);
            paint_shaded_rule(&mut buf, &f, ColumnRuleStyle::Groove, RED);
            assert!(
                buf.is_empty(),
                "non-finite-width SVG helpers must not emit markup or loop ({bad}): {buf}"
            );
        }
    }

    #[test]
    fn boxes_at_one_origin_get_their_own_gradient_and_shadow_ids() {
        let blue = Rgba::new(0, 0, 255, 255);
        let stops = |c: Rgba| {
            Some(CssGradient {
                angle_deg: 90.0,
                stops: vec![
                    GradientStop { color: c, pos: 0.0 },
                    GradientStop {
                        color: WHITE,
                        pos: 1.0,
                    },
                ],
            })
        };
        let mut root = plain(0.0, 0.0, 100.0, 100.0, None);
        root.children = vec![
            decorated(10.0, 10.0, 40.0, 20.0, 0.0, |s, g, _| {
                *s = Some(shadow(2, 2, 6, 0, RED, false));
                *g = stops(RED);
            }),
            decorated(10.0, 10.0, 30.0, 30.0, 0.0, |s, g, _| {
                *s = Some(shadow(4, 4, 8, 0, blue, false));
                *g = stops(blue);
            }),
        ];
        let svg = body(&root, &HashMap::new());
        let ids = ids(&svg);
        assert_eq!(ids.len(), 4, "{svg}");
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 4, "{ids:?}");
        // Each box's fill and filter reference its own definitions.
        for id in &ids {
            assert_eq!(
                svg.matches(&format!("url(#{id})")).count(),
                1,
                "{id}: {svg}"
            );
        }
    }

    // v0.8.2 R2 M1: every public SVG painter that reads `f.width` / `f.height`
    // must no-op on a non-finite fragment. Unlike the raster side (where
    // tiny-skia's own `Rect::from_xywh` rejects NaN / Inf), the SVG painter
    // formats coordinates directly into attributes — a non-finite size would
    // emit `width="NaN"` / `x="NaN"` / `cx="NaN"` into the document, which
    // is why this guard is load-bearing on the SVG path. Call each painter
    // with NaN / Infinity width (and height) and assert nothing emitted.
    #[test]
    fn non_rule_svg_painters_are_a_no_op_on_non_finite_fragments() {
        let mk = |w: f32, h: f32| {
            Fragment::new(
                NodeId(0),
                1.0,
                1.0,
                w,
                h,
                FragmentContent::Marker(MarkerKind {
                    id: 1,
                    inline: false,
                }),
            )
        };
        let placement = ImagePlacement {
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
            color: RED,
            inset: false,
        };
        let inset_shadow_cfg = BoxShadow {
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
                    color: WHITE,
                    pos: 1.0,
                },
            ],
        };
        let pattern = turbo_dom_layout::TilePattern {
            tile: Box::new(FragmentContent::Marker(MarkerKind {
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
                let mut buf = String::new();
                let images = HashMap::new();
                paint_image(&mut buf, &f, &placement, &images);
                paint_border(&mut buf, &f, &border);
                paint_box_shadow(&mut buf, &f, &BorderRadii::ZERO, &shadow_cfg);
                paint_inset_shadow(
                    &mut buf,
                    &f,
                    &border,
                    &BorderRadii::ZERO,
                    Some(&inset_shadow_cfg),
                );
                paint_gradient(&mut buf, &f, &BorderRadii::ZERO, &gradient);
                paint_round_border(&mut buf, &f, &border, &radii);
                paint_tiles(&mut buf, &f, &pattern, &images);
                assert!(
                    buf.is_empty(),
                    "non-finite ({bad}) size {}x{} must emit no markup, got: {buf}",
                    f.width,
                    f.height
                );
            }
        }
    }

    // v0.8.2 R2 M1: `paint_gradient` additionally rejects a gradient whose
    // stops include a non-finite position. Without it a `NaN` / `Infinity`
    // would reach the `<stop offset="…"/>` serializer and produce
    // `offset="NaN"` in the document.
    #[test]
    fn paint_gradient_svg_rejects_non_finite_stop_positions() {
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut buf = String::new();
            let gradient = CssGradient {
                angle_deg: 0.0,
                stops: vec![
                    GradientStop {
                        color: RED,
                        pos: bad,
                    },
                    GradientStop {
                        color: WHITE,
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
                FragmentContent::Marker(MarkerKind {
                    id: 1,
                    inline: false,
                }),
            );
            paint_gradient(&mut buf, &f, &BorderRadii::ZERO, &gradient);
            assert!(
                buf.is_empty(),
                "non-finite ({bad}) stop position must emit no markup, got: {buf}"
            );
        }
    }

    // v0.8.2 R3 H1: the `paint_subtree` Box branch fills the background through
    // `rect_r` BEFORE any `fragment_finite_positive` guard, so a non-finite
    // width / height / x / y on a `Box` fragment with a background colour
    // slipped past `rect_r`'s `w <= 0.0 || h <= 0.0` check (every comparison
    // with NaN is false; Infinity is strictly positive) and the `shape`
    // serializer wrote `width="NaN"` / `width="inf"` into the document.
    // Guard `rect_r` so non-finite coordinates emit nothing.
    // v0.8.2 raster-cov: `paint_inset_shadow` (SVG) guards the padding box
    // against a non-positive inline or block size BEFORE it writes a
    // `<mask>` into the document — without this guard a border wider than
    // the fragment (padding box = width - left - right <= 0) would emit
    // `<mask … width="-N"/>` and a `<rect … width="-N"/>` inside it,
    // dirtying the document and producing an invalid mask. Reachable from
    // honest layout: a `box-sizing: border-box` box whose borders exceed
    // its box dimensions, so this test pins the guard with a real
    // inset-shadow configuration.
    #[test]
    fn paint_inset_shadow_svg_skips_a_border_wider_than_the_fragment() {
        let side = BorderSide {
            width: 20,
            color: Some(RED),
        };
        let border = BorderEdges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        };
        let shadow = BoxShadow {
            offset_x: 0,
            offset_y: 0,
            blur: 0,
            spread: 0,
            color: RED,
            inset: true,
        };
        let f = Fragment::new(
            NodeId(0),
            1.0,
            1.0,
            10.0,
            10.0,
            FragmentContent::Marker(MarkerKind {
                id: 1,
                inline: false,
            }),
        );
        let mut buf = String::new();
        paint_inset_shadow(&mut buf, &f, &border, &BorderRadii::ZERO, Some(&shadow));
        assert!(
            buf.is_empty(),
            "a border wider than the fragment collapses the padding box; \
             the inset-shadow painter must emit no markup, got: {buf}"
        );
    }

    #[test]
    fn paint_subtree_box_rejects_non_finite_coords() {
        for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            // Vary which coordinate is non-finite: x, y, width, height.
            let cases = [
                plain(bad, 0.0, 4.0, 4.0, Some(RED)),
                plain(0.0, bad, 4.0, 4.0, Some(RED)),
                plain(0.0, 0.0, bad, 4.0, Some(RED)),
                plain(0.0, 0.0, 4.0, bad, Some(RED)),
            ];
            for f in cases {
                let mut buf = String::new();
                let images = HashMap::new();
                paint_subtree(&mut buf, &f, &images);
                assert!(
                    !buf.contains("NaN") && !buf.contains("inf"),
                    "non-finite ({bad}) box coord must not serialize NaN/inf, got: {buf}"
                );
            }
        }
    }

    fn exercise_stalled_rule_helpers() {
        let mut f = rule(
            1.0,
            1_000_000_000.0,
            1.0,
            128.0,
            ColumnRuleStyle::Dotted,
            RED,
        );
        let mut output = String::new();
        paint_dotted_rule(&mut output, &f, RED);
        assert_eq!(output.matches("<circle").count(), 1);
        output.clear();
        paint_dashed_rule(&mut output, &f, RED);
        assert_eq!(output.matches("<rect").count(), 1);
        output.clear();
        f.width = 0.5;
        paint_dotted_rule(&mut output, &f, RED);
        assert_eq!(output.matches("<rect").count(), 2);
    }

    #[test]
    fn rule_helpers_stop_when_an_advance_is_not_representable() {
        const CHILD: &str = "TURBO_DOM_STALLED_SVG_RULE_HELPERS";
        const MARKER: &str = "SVG_STALLED_ADVANCES_EXERCISED";
        if std::env::var_os(CHILD).is_some() {
            exercise_stalled_rule_helpers();
            println!("{MARKER}");
            return;
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "paint_svg::tests::rule_helpers_stop_when_an_advance_is_not_representable",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success(), "{output:?}");
                assert!(String::from_utf8(output.stdout).unwrap().contains(MARKER));
                return;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("stalled rule helpers did not stop within three seconds");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn subpixel_dotted_caps_reject_empty_reversed_and_non_finite_origins() {
        for height in [0.0, -1.0] {
            for y in [
                0.0,
                1_000_000_000.0,
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ] {
                let f = rule(1.0, y, 0.5, height, ColumnRuleStyle::Dotted, RED);
                let mut output = String::new();
                paint_dotted_rule(&mut output, &f, RED);
                assert!(output.is_empty(), "height={height}, y={y}: {output}");
            }
        }
    }

    #[test]
    fn inset_shadow_integer_blur_boundaries_keep_filter_semantics() {
        let mut capped = None;
        for blur in [0, 1, 256, u32::MAX] {
            let f = decorated(1.0, 2.0, 10.0, 10.0, 0.0, |slot, _, _| {
                *slot = Some(shadow(1, 2, blur, 0, RED, true));
            });
            let output = body(&f, &HashMap::new());
            assert_eq!(output.contains("<filter id=\"insetblur"), blur != 0);
            if blur == 1 {
                assert!(output.contains("stdDeviation=\"0.50\""));
            }
            if blur == 256 {
                capped = Some(output.clone());
            }
            if blur == u32::MAX {
                assert_eq!(output, capped.as_ref().unwrap().as_str());
            }
        }
    }
}

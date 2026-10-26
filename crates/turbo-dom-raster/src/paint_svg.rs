//! Vector painter: walk a [`Fragment`] galley into a standalone SVG document.
//! Same stacking-order walk as the raster painter, so the two agree. Glyphs are
//! emitted as filled `<path>` outlines (self-contained — no viewer font needed),
//! boxes/borders as `<rect>`, and images as base64 PNG `data:` URI `<image>`s
//! (any decoded source format; unknown formats fall back to a placeholder rect).

use std::fmt::Write;

use turbo_html2pdf_core::layout::value::{BorderEdges, BorderSide};
use turbo_html2pdf_core::{
    BoxShadow, Fragment, FragmentContent, LinearGradient as CssGradient, PositionedGlyph, Rgba,
};

use crate::glyph::{self, Pen, Tracer};
use crate::image_paint::{self, DecodedAssets};

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
    let mut svg = String::with_capacity(4096);
    let _ = writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\">\n<rect width=\"{width}\" height=\"{height}\" {}/>",
        fill_attrs(bg)
    );
    paint_fragment(&mut svg, galley, images);
    svg.push_str("</svg>\n");
    svg
}

fn paint_fragment(svg: &mut String, f: &Fragment, images: &DecodedAssets) {
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
                paint_box_shadow(svg, f, *border_radius, &sh);
            }
            if let Some(bg) = background {
                rect_r(svg, f.x, f.y, f.width, f.height, *bg, *border_radius);
            }
            if let Some(g) = gradient {
                paint_gradient(svg, f, *border_radius, g);
            }
            paint_border(svg, f, border);
        }
        FragmentContent::TextLine {
            glyphs,
            face,
            font_size,
            color,
        } => paint_text(
            svg,
            f,
            glyphs,
            face.data(),
            face.index(),
            face.units_per_em(),
            *font_size,
            *color,
        ),
        FragmentContent::Image(placement) => paint_image(svg, f, &placement.name, images),
        FragmentContent::Directive(_) => {}
    }
    // Paint children back-to-front in CSS stacking order (§9.9), not raw DOM
    // order, so `position`/`z-index` boxes (menus, modals) layer correctly.
    for &i in &f.paint_order() {
        paint_fragment(svg, &f.children[i], images);
    }
    if transformed {
        svg.push_str("</g>");
    }
}

/// Embed an image as a base64 `data:` URI `<image>` filling its layout box; fall
/// back to a neutral placeholder rect when the bytes are absent or unrecognized.
/// `preserveAspectRatio="none"` because layout already sized the box to the
/// image's aspect ratio.
fn paint_image(svg: &mut String, f: &Fragment, name: &str, images: &DecodedAssets) {
    match images.get(name).and_then(image_paint::data_uri) {
        Some(uri) => {
            let _ = writeln!(
                svg,
                "<image x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" \
                 preserveAspectRatio=\"none\" href=\"{uri}\"/>",
                f.x, f.y, f.width, f.height
            );
        }
        None => rect(svg, f.x, f.y, f.width, f.height, IMAGE_PLACEHOLDER),
    }
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
            c.a as f32 / 255.0
        )
    }
}

fn rect(svg: &mut String, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
    rect_r(svg, x, y, w, h, c, 0.0);
}

/// Paint an outer `box-shadow` as a blurred rounded rect behind the box, using an
/// inline SVG `feGaussianBlur` filter (native, so any viewer renders the soft
/// edge). `blur` 0 emits an unfiltered offset rect. The filter id is derived from
/// the box geometry to stay unique within the document.
fn paint_box_shadow(svg: &mut String, f: &Fragment, border_radius: f32, sh: &BoxShadow) {
    let spread = sh.spread as f32;
    let x = f.x + sh.offset_x as f32 - spread;
    let y = f.y + sh.offset_y as f32 - spread;
    let w = f.width + 2.0 * spread;
    let h = f.height + 2.0 * spread;
    if w < 0.5 || h < 0.5 {
        return;
    }
    let radius = (border_radius + spread).max(0.0);
    if sh.blur < 1 {
        rect_r(svg, x, y, w, h, sh.color, radius);
        return;
    }
    // Pad the filter region by the blur so the feathered edge isn't clipped.
    let id = format!(
        "sh{}_{}",
        (f.x as i64) + 1_000_000,
        (f.y as i64) + 1_000_000
    );
    let std = sh.blur as f32 / 2.0;
    let _ = writeln!(
        svg,
        "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">\
         <feGaussianBlur stdDeviation=\"{std:.2}\"/></filter>",
    );
    let rr = if radius > 0.5 {
        format!(" rx=\"{radius:.2}\" ry=\"{radius:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr} {} filter=\"url(#{id})\"/>",
        fill_attrs(sh.color)
    );
}

/// Paint a `linear-gradient(...)` background as a native SVG `<linearGradient>`
/// (userSpaceOnUse, start/end points across the box per the CSS angle) filling the
/// box rect (rounded by `border_radius`). The gradient id is derived from the box
/// geometry to stay unique within the document.
fn paint_gradient(svg: &mut String, f: &Fragment, border_radius: f32, g: &CssGradient) {
    if f.width < 0.5 || f.height < 0.5 || g.stops.len() < 2 {
        return;
    }
    let (w, h) = (f.width, f.height);
    let theta = g.angle_deg.to_radians();
    let (dx, dy) = (theta.sin(), -theta.cos());
    let half = (w * theta.sin().abs() + h * theta.cos().abs()) / 2.0;
    let (cx, cy) = (f.x + w / 2.0, f.y + h / 2.0);
    let (x1, y1) = (cx - dx * half, cy - dy * half);
    let (x2, y2) = (cx + dx * half, cy + dy * half);
    let id = format!("g{}_{}", (f.x as i64) + 1_000_000, (f.y as i64) + 1_000_000);
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
            s.color.a as f32 / 255.0
        );
    }
    let rr = if border_radius > 0.5 {
        format!(" rx=\"{border_radius:.2}\" ry=\"{border_radius:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "</linearGradient>\n<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{rr} \
         fill=\"url(#{id})\"/>",
        f.x, f.y
    );
}

/// A filled `<rect>`, rounded by `r` px (`rx`/`ry`) when `r > 0`.
fn rect_r(svg: &mut String, x: f32, y: f32, w: f32, h: f32, c: Rgba, r: f32) {
    if w <= 0.0 || h <= 0.0 || c.a == 0 {
        return;
    }
    let radius = if r > 0.5 {
        format!(" rx=\"{r:.2}\" ry=\"{r:.2}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{radius} {}/>",
        fill_attrs(c)
    );
}

fn paint_border(svg: &mut String, f: &Fragment, b: &BorderEdges) {
    let side = |s: &BorderSide| s.color.filter(|_| s.width > 0).map(|c| (s.width as f32, c));
    if let Some((w, c)) = side(&b.top) {
        rect(svg, f.x, f.y, f.width, w, c);
    }
    if let Some((w, c)) = side(&b.bottom) {
        rect(svg, f.x, f.y + f.height - w, f.width, w, c);
    }
    if let Some((w, c)) = side(&b.left) {
        rect(svg, f.x, f.y, w, f.height, c);
    }
    if let Some((w, c)) = side(&b.right) {
        rect(svg, f.x + f.width - w, f.y, w, f.height, c);
    }
}

/// An SVG path-data sink for [`glyph::trace_glyph`].
#[derive(Default)]
struct PathSink(String);

impl Tracer for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "M{x:.2} {y:.2} ");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "L{x:.2} {y:.2} ");
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let _ = write!(self.0, "Q{cx:.2} {cy:.2} {x:.2} {y:.2} ");
    }
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        let _ = write!(
            self.0,
            "C{c1x:.2} {c1y:.2} {c2x:.2} {c2y:.2} {x:.2} {y:.2} "
        );
    }
    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

#[allow(clippy::too_many_arguments)] // glyph run + face bytes/index/metrics + colour
fn paint_text(
    svg: &mut String,
    f: &Fragment,
    glyphs: &[PositionedGlyph],
    font_bytes: &[u8],
    font_index: u32,
    units_per_em: u16,
    font_size: f32,
    color: Rgba,
) {
    if color.a == 0 || units_per_em == 0 {
        return;
    }
    let Some(face) = glyph::parse_face(font_bytes, font_index) else {
        return;
    };
    let scale = font_size / units_per_em as f32;
    let mut sink = PathSink::default();
    for g in glyphs {
        let pen = Pen {
            origin_x: f.x + g.x,
            baseline_y: f.y + g.y,
            scale,
        };
        glyph::trace_glyph(&face, g.glyph_id, pen, &mut sink);
    }
    if !sink.0.is_empty() {
        let _ = writeln!(
            svg,
            "<path d=\"{}\" {}/>",
            sink.0.trim_end(),
            fill_attrs(color)
        );
    }
}

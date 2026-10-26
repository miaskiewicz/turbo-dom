//! Trace a glyph's outline out of a font face's raw bytes.
//!
//! Both painters need the same thing: given a font's bytes (from
//! `FontFace::data()`), a glyph id, a pixel scale, and a pen origin, walk the
//! glyph's contours. TrueType/OpenType outlines are y-up in font design units;
//! we scale by `font_size / units_per_em` and flip Y about the text baseline so
//! the contour lands in the image's y-down pixel space. The two [`Tracer`] impls
//! feed a tiny-skia path (raster) and an SVG `d` string (vector).

use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// A pen-space transform: multiply design units by `scale`, then place relative
/// to `(origin_x, baseline_y)` with Y flipped (font y-up → image y-down).
#[derive(Clone, Copy)]
pub struct Pen {
    pub origin_x: f32,
    pub baseline_y: f32,
    pub scale: f32,
}

impl Pen {
    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin_x + x * self.scale,
            self.baseline_y - y * self.scale,
        )
    }
}

/// A sink for the mapped outline segments of one glyph.
pub trait Tracer {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32);
    fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32);
    fn close(&mut self);
}

struct Adapter<'a, T: Tracer> {
    pen: Pen,
    sink: &'a mut T,
}

impl<T: Tracer> OutlineBuilder for Adapter<'_, T> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.pen.map(x, y);
        self.sink.move_to(px, py);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.pen.map(x, y);
        self.sink.line_to(px, py);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (pcx, pcy) = self.pen.map(cx, cy);
        let (px, py) = self.pen.map(x, y);
        self.sink.quad_to(pcx, pcy, px, py);
    }
    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        let (p1x, p1y) = self.pen.map(c1x, c1y);
        let (p2x, p2y) = self.pen.map(c2x, c2y);
        let (px, py) = self.pen.map(x, y);
        self.sink.cubic_to(p1x, p1y, p2x, p2y, px, py);
    }
    fn close(&mut self) {
        self.sink.close();
    }
}

/// Parse face `index` of a font program. Returns `None` if the bytes don't parse (a
/// malformed face paints nothing rather than panicking). `index` selects the face in
/// a `.ttc` collection — it MUST match the index the glyph ids were shaped against
/// (macOS Arial/Helvetica are collections; parsing index 0 for a non-0 face draws a
/// different sub-font's glyphs, shifting the whole run).
pub fn parse_face(data: &[u8], index: u32) -> Option<Face<'_>> {
    Face::parse(data, index).ok()
}

/// Trace one glyph into `sink`. No-op for a glyph the face has no outline for
/// (spaces, unresolved ids).
pub fn trace_glyph<T: Tracer>(face: &Face, glyph_id: u16, pen: Pen, sink: &mut T) {
    let mut adapter = Adapter { pen, sink };
    face.outline_glyph(GlyphId(glyph_id), &mut adapter);
}

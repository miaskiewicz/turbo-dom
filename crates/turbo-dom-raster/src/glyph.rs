//! Trace a glyph's outline out of a font face's raw bytes.
//!
//! Both painters need the same thing: given a font's bytes (from
//! `FontFace::data()`), a glyph id, a pixel scale, and a pen origin, walk the
//! glyph's contours. TrueType/OpenType outlines are y-up in font design units;
//! we scale by `font_size / units_per_em` and flip Y about the text baseline so
//! the contour lands in the image's y-down pixel space. The two [`Tracer`] impls
//! feed a tiny-skia path (raster) and an SVG `d` string (vector).

use ttf_parser::{Face, GlyphId, OutlineBuilder};
use turbo_dom_layout::text::FontFace;

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

/// The `ttf-parser` face of a laid-out [`FontFace`], parsed at the face's own index: it
/// MUST match the index the glyph ids were shaped against (macOS Arial/Helvetica are
/// collections; parsing index 0 for a non-0 face draws a different sub-font's glyphs,
/// shifting the whole run). Infallible: turbo-dom-layout only builds a `FontFace` from
/// bytes that `ttf_parser::Face::parse` accepted at that index (`FontFace::from_bytes*`),
/// and cargo unifies its `ttf-parser` 0.25 (through rustybuzz) with this crate's, so the
/// same parser re-reads the same bytes.
pub fn outline_face(face: &FontFace) -> Face<'_> {
    Face::parse(face.data(), face.index()).expect("a FontFace's bytes parse at its index")
}

/// Trace one glyph into `sink`. No-op for a glyph the face has no outline for
/// (spaces, unresolved ids).
pub fn trace_glyph<T: Tracer>(face: &Face, glyph_id: u16, pen: Pen, sink: &mut T) {
    let mut adapter = Adapter { pen, sink };
    face.outline_glyph(GlyphId(glyph_id), &mut adapter);
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_layout::text::FontRegistry;

    /// Every segment a trace emits, as text.
    #[derive(Default)]
    struct Record(Vec<String>);

    impl Tracer for Record {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("M {x} {y}"));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push(format!("L {x} {y}"));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.0.push(format!("Q {cx} {cy} {x} {y}"));
        }
        fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
            self.0.push(format!("C {c1x} {c1y} {c2x} {c2y} {x} {y}"));
        }
        fn close(&mut self) {
            self.0.push("Z".into());
        }
    }

    /// The same outline straight from ttf-parser, mapped by hand: x' = 10 + 0.5x,
    /// y' = 100 - 0.5y.
    struct Raw(Vec<String>);

    impl OutlineBuilder for Raw {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0
                .push(format!("M {} {}", 10.0 + x * 0.5, 100.0 - y * 0.5));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0
                .push(format!("L {} {}", 10.0 + x * 0.5, 100.0 - y * 0.5));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (10.0 + x * 0.5, 100.0 - y * 0.5);
            let ((a, b), (c, d)) = (m(cx, cy), m(x, y));
            self.0.push(format!("Q {a} {b} {c} {d}"));
        }
        fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
            let m = |x: f32, y: f32| (10.0 + x * 0.5, 100.0 - y * 0.5);
            let ((a, b), (c, d), (e, f)) = (m(c1x, c1y), m(c2x, c2y), m(x, y));
            self.0.push(format!("C {a} {b} {c} {d} {e} {f}"));
        }
        fn close(&mut self) {
            self.0.push("Z".into());
        }
    }

    #[test]
    fn traces_map_every_segment_through_the_pen_y_flipped() {
        let reg = FontRegistry::new();
        // Inter (CFF: cubic curves) and Liberation Serif (TrueType: quadratic curves).
        for (family, curve) in [("Inter", "C "), ("Liberation Serif", "Q ")] {
            let face = reg.select(&[family], 400, false).unwrap();
            let ttf = outline_face(face);
            for ch in ['o', 'A', '%'] {
                let id = ttf.glyph_index(ch).unwrap();
                let pen = Pen {
                    origin_x: 10.0,
                    baseline_y: 100.0,
                    scale: 0.5,
                };
                let mut got = Record::default();
                trace_glyph(&ttf, id.0, pen, &mut got);
                let mut want = Raw(Vec::new());
                ttf.outline_glyph(id, &mut want);
                assert_eq!(got.0, want.0, "{family} {ch}");
                let closes = got.0.iter().filter(|s| *s == "Z").count();
                let moves = got.0.iter().filter(|s| s.starts_with("M ")).count();
                assert!(
                    moves > 0 && closes == moves,
                    "{family} {ch}: every contour closes"
                );
                if ch == 'o' {
                    assert!(
                        got.0.iter().any(|s| s.starts_with(curve)),
                        "{family}: {curve}"
                    );
                }
            }
            // A glyph with no outline (space) traces nothing.
            let mut space = Record::default();
            trace_glyph(
                &ttf,
                ttf.glyph_index(' ').unwrap().0,
                Pen {
                    origin_x: 0.0,
                    baseline_y: 0.0,
                    scale: 1.0,
                },
                &mut space,
            );
            assert!(space.0.is_empty());
        }
    }
}

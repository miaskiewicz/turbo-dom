//! Default (tiny-skia) backend for the 2D-canvas op-log painter.
//!
//! Replays the shared [`crate::canvas_ops`] op stream into a tiny-skia pixmap and
//! encodes PNG. The pixmap starts fully transparent (the canvas initial state), so
//! an empty op-log yields a valid blank image; fills composite source-over, and
//! `clearRect` punches back to transparent. The opt-in wgpu→Metal backend
//! ([`crate::paint_canvas_gpu`]) paints the same op stream on the real GPU.

use tiny_skia::{BlendMode, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use turbo_html2pdf_core::Rgba;

use crate::canvas_ops::{self, CanvasBackend, Op, State, SubPath};

/// Rasterize the parsed op stream at `width × height` (both already clamped ≥ 1 by
/// the caller) into a PNG. `Err` only on pixmap allocation or PNG encoding.
pub(crate) fn replay(width: u32, height: u32, ops: &[Op]) -> Result<Vec<u8>, String> {
    let pm = Pixmap::new(width, height).ok_or_else(|| format!("bad canvas {width}x{height}"))?;
    let mut backend = SkiaCanvas { pm };
    canvas_ops::run(ops, &mut backend);
    backend
        .pm
        .encode_png()
        .map_err(|e| format!("png encode: {e}"))
}

/// Rasterize the op stream into RAW, straight-alpha RGBA8 (top-left origin, tightly packed) —
/// the byte layout `getImageData` returns. tiny-skia stores premultiplied alpha, so unpremultiply
/// back to straight alpha (else a browser reading `getImageData` after a translucent draw would
/// see premultiplied values a real canvas never returns).
pub(crate) fn replay_rgba(width: u32, height: u32, ops: &[Op]) -> Result<Vec<u8>, String> {
    let pm = Pixmap::new(width, height).ok_or_else(|| format!("bad canvas {width}x{height}"))?;
    let mut backend = SkiaCanvas { pm };
    canvas_ops::run(ops, &mut backend);
    let src = backend.pm.data(); // premultiplied RGBA8
    let mut out = Vec::with_capacity(src.len());
    let (pixels, _rem) = src.as_chunks::<4>();
    for px in pixels {
        let a = px[3];
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // straight = round(premul * 255 / a)
            let un = |c: u8| (((c as u32) * 255 + (a as u32) / 2) / a as u32).min(255) as u8;
            out.push(un(px[0]));
            out.push(un(px[1]));
            out.push(un(px[2]));
            out.push(a);
        }
    }
    Ok(out)
}

/// A tiny-skia pixmap wearing the [`CanvasBackend`] primitives.
struct SkiaCanvas {
    pm: Pixmap,
}

impl SkiaCanvas {
    /// Build a tiny-skia path from device-space subpaths (each `move_to` + `line_to`
    /// run, closed when the subpath is). `None` if nothing traceable remains.
    fn build_path(subs: &[SubPath]) -> Option<tiny_skia::Path> {
        let mut b = PathBuilder::new();
        for sub in subs {
            let mut pts = sub.pts.iter();
            if let Some(&(x, y)) = pts.next() {
                b.move_to(x, y);
                for &(x, y) in pts {
                    b.line_to(x, y);
                }
                if sub.closed {
                    b.close();
                }
            }
        }
        b.finish()
    }
}

/// A solid, anti-aliased tiny-skia paint.
fn solid(c: Rgba) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(c.r, c.g, c.b, c.a);
    paint.anti_alias = true;
    paint
}

impl CanvasBackend for SkiaCanvas {
    // Text: on macOS with `coretext`, rasterize glyphs through CoreText/CoreGraphics (Chrome's
    // glyph stack) and composite onto the pixmap — so getImageData over text matches a real Mac
    // Chrome. Falls back to the vector glyph trace (the trait default) off-macOS, without the
    // feature, or for a scaled/rotated CTM the CoreText path doesn't handle.
    fn fill_text(&mut self, st: &State, text: &str, x: f32, y: f32, color: Rgba) {
        #[cfg(all(target_os = "macos", feature = "coretext"))]
        {
            if crate::paint_canvas_coretext::render_text(&mut self.pm, st, text, x, y, color) {
                return;
            }
        }
        self.fill(&canvas_ops::glyph_subpaths(st, text, x, y), color);
    }

    fn fill(&mut self, subs: &[SubPath], color: Rgba) {
        if color.a == 0 {
            return;
        }
        if let Some(path) = Self::build_path(subs) {
            self.pm.fill_path(
                &path,
                &solid(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    fn stroke(&mut self, subs: &[SubPath], color: Rgba, width: f32) {
        if color.a == 0 || width <= 0.0 {
            return;
        }
        if let Some(path) = Self::build_path(subs) {
            let stroke = Stroke {
                width,
                ..Default::default()
            };
            self.pm
                .stroke_path(&path, &solid(color), &stroke, Transform::identity(), None);
        }
    }

    fn clear(&mut self, quad: &[(f32, f32); 4]) {
        // `Clear` blend writes transparent black regardless of source colour.
        let mut b = PathBuilder::new();
        b.move_to(quad[0].0, quad[0].1);
        for &(x, y) in &quad[1..] {
            b.line_to(x, y);
        }
        b.close();
        if let Some(path) = b.finish() {
            let paint = Paint {
                blend_mode: BlendMode::Clear,
                ..Default::default()
            };
            self.pm.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_dims(bytes: &[u8]) -> (u32, u32) {
        let img = image::load_from_memory(bytes).expect("decode png");
        (img.width(), img.height())
    }

    #[test]
    fn empty_ops_yield_blank_png_at_size() {
        let png = replay(40, 30, &[]).unwrap();
        assert_eq!(png_dims(&png), (40, 30));
    }

    // Edge cases in the op-log parser: malformed / non-array JSON errors; junk entries, unknown
    // ops, and short/typeless tuples are skipped without panicking (forward-compatible).
    #[test]
    fn parse_ops_edge_cases() {
        assert!(
            canvas_ops::parse_ops("not json").is_err(),
            "malformed JSON → Err"
        );
        assert!(
            canvas_ops::parse_ops(r#"{"op":"x"}"#).is_err(),
            "non-array top level → Err"
        );
        assert!(
            canvas_ops::parse_ops("[]").unwrap().is_empty(),
            "empty array → no ops"
        );
        // A junk tuple, an unknown op, a short fillRect (missing args), and a valid one: only the
        // valid op survives; nothing panics.
        let ops = canvas_ops::parse_ops(
            r##"[123, ["frobnicate",[1,2]], ["fillRect",[0,0]], ["fillRect",[0,0,4,4],"#f00",null,null,null,null,1,null]]"##,
        )
        .unwrap();
        // The two fillRects both parse (missing args default to 0 → a 0-size rect is harmless);
        // junk (123) and the unknown op are dropped. Either way replay must not panic.
        let png = replay(8, 8, &ops).unwrap();
        assert_eq!(png_dims(&png), (8, 8));
    }

    #[test]
    fn clamps_zero_size_to_valid_png() {
        // The public entry clamps 0-dims to 1×1 (replay itself requires a valid pixmap).
        let png = crate::canvas_ops_png(0, 0, "[]").unwrap();
        assert_eq!(
            png_dims(&png),
            (1, 1),
            "0×0 clamps to 1×1, still a valid PNG"
        );
    }

    // The op-log is the render tier's `ctx._ops` tuple format:
    // [name, argsArray, fillStyle, strokeStyle, font, textBaseline, textAlign, globalAlpha, comp].
    #[test]
    fn fill_rect_paints_its_colour() {
        let ops = canvas_ops::parse_ops(
            r##"[["fillRect",[0,0,16,16],"#ff0000","#000000","10px sans-serif","alphabetic","start",1,"source-over"]]"##,
        )
        .unwrap();
        let png = replay(16, 16, &ops).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        let px = img.get_pixel(8, 8);
        assert_eq!(px.0, [255, 0, 0, 255], "centre pixel is opaque red");
    }

    #[test]
    fn distinct_op_lists_differ() {
        let a = replay(
            20, 20,
            &canvas_ops::parse_ops(r##"[["fillRect",[0,0,20,20],"#000000","#000","10px x","alphabetic","start",1,"source-over"]]"##).unwrap(),
        ).unwrap();
        let b = replay(
            20, 20,
            &canvas_ops::parse_ops(r##"[["fillRect",[0,0,5,5],"#000000","#000","10px x","alphabetic","start",1,"source-over"]]"##).unwrap(),
        ).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn fill_text_renders_non_blank() {
        let ops = canvas_ops::parse_ops(
            r##"[["fillText",["Ag",2,22],"#000000","#000","20px sans-serif","alphabetic","start",1,"source-over"]]"##,
        )
        .unwrap();
        let png = replay(64, 32, &ops).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        let inked = img.pixels().filter(|p| p.0[3] > 0).count();
        assert!(inked > 0, "text painted some opaque pixels");
    }

    #[test]
    fn clear_rect_punches_transparent() {
        let ops = canvas_ops::parse_ops(
            r##"[["fillRect",[0,0,16,16],"#0000ff","#000","10px x","alphabetic","start",1,"source-over"],["clearRect",[4,4,8,8],"#0000ff","#000","10px x","alphabetic","start",1,"source-over"]]"##,
        )
        .unwrap();
        let png = replay(16, 16, &ops).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(8, 8).0[3], 0, "cleared centre is transparent");
        assert_eq!(img.get_pixel(1, 1).0, [0, 0, 255, 255], "corner still blue");
    }
}

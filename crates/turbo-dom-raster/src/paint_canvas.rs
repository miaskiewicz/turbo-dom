//! Default (tiny-skia) backend for the 2D-canvas op-log painter.
//!
//! Replays the shared [`crate::canvas_ops`] op stream into a tiny-skia pixmap and
//! encodes PNG. The pixmap starts fully transparent (the canvas initial state), so
//! an empty op-log yields a valid blank image; fills composite source-over, and
//! `clearRect` punches back to transparent. The opt-in wgpu→Metal backend
//! ([`crate::paint_canvas_gpu`]) paints the same op stream on the real GPU.

use tiny_skia::{BlendMode, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use turbo_dom_layout::Rgba;

#[cfg(all(target_os = "macos", feature = "coretext"))]
use crate::canvas_ops::State;
use crate::canvas_ops::{self, CanvasBackend, Op, SubPath};

/// Rasterize the parsed op stream at `width × height` (both already clamped ≥ 1 by
/// the caller) into a PNG. `Err` only on pixmap allocation or PNG encoding.
pub(crate) fn replay(width: u32, height: u32, ops: &[Op]) -> Result<Vec<u8>, String> {
    // The PNG carries the straight-alpha pixels `getImageData` reads (Chrome's `toDataURL`
    // encodes the same unpremultiplied values).
    let rgba = replay_rgba(width, height, ops)?;
    let img = image::RgbaImage::from_raw(width, height, rgba).expect("width * height * 4 bytes");
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(png.into_inner())
}

/// Rasterize the op stream into RAW, straight-alpha RGBA8 (top-left origin, tightly packed) —
/// the byte layout `getImageData` returns. tiny-skia stores premultiplied alpha, so unpremultiply
/// back to straight alpha (else a browser reading `getImageData` after a translucent draw would
/// see premultiplied values a real canvas never returns), rounding as Chrome does.
pub(crate) fn replay_rgba(width: u32, height: u32, ops: &[Op]) -> Result<Vec<u8>, String> {
    let pm = Pixmap::new(width, height).ok_or_else(|| format!("bad canvas {width}x{height}"))?;
    let mut backend = SkiaCanvas { pm };
    canvas_ops::run(ops, &mut backend);
    let (pixels, _rem) = backend.pm.data().as_chunks::<4>(); // premultiplied RGBA8
    Ok(pixels.iter().flat_map(|&px| unpremultiply(px)).collect())
}

/// One premultiplied RGBA8 pixel as straight alpha, the way Chrome's `getImageData` (Skia's
/// raster pipeline) computes it: in single precision, each channel scaled to 0..1, divided
/// by the alpha (multiplied by its reciprocal) and rounded to the nearest byte, ties to even.
/// A transparent pixel is transparent black. Checked over a 16 x 16 colour/alpha grid
/// against Chrome 156 (`unpremultiply` in `scripts/canvas-ops-chrome-control.mjs`).
fn unpremultiply([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    if a == 0 {
        return [0, 0, 0, 0];
    }
    let unit = 1.0_f32 / 255.0;
    let scale = 1.0 / (f32::from(a) * unit);
    let channel = |c: u8| {
        (f32::from(c) * unit * scale * 255.0)
            .round_ties_even()
            .min(255.0) as u8
    };
    [channel(r), channel(g), channel(b), a]
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
    // Chrome. Falls back to the vector glyph trace for a scaled/rotated CTM the CoreText path
    // doesn't handle; off-macOS or without the feature the trait default (that same trace)
    // fills text.
    #[cfg(all(target_os = "macos", feature = "coretext"))]
    fn fill_text(&mut self, st: &State, text: &str, x: f32, y: f32, color: Rgba) {
        if crate::paint_canvas_coretext::render_text(&mut self.pm, st, text, x, y, color) {
            return;
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

    /// The opaque-pixel count of a replayed op log.
    fn inked(w: u32, h: u32, log: &str) -> usize {
        let rgba = replay_rgba(w, h, &canvas_ops::parse_ops(log).unwrap()).unwrap();
        rgba.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count()
    }

    #[test]
    fn stroke_text_outlines_the_glyphs_fill_text_fills() {
        let text = |op: &str| {
            format!(
                r##"[["{op}",["O",4,40],"#000000","#000000","40px serif","alphabetic","start",1,"source-over"]]"##
            )
        };
        let (filled, stroked) = (
            inked(64, 48, &text("fillText")),
            inked(64, 48, &text("strokeText")),
        );
        assert!(filled > 0 && stroked > 0, "{filled} {stroked}");
        assert_ne!(filled, stroked, "an outline is not the filled glyph");
        // A transparent stroke style inks nothing.
        let clear = text("strokeText").replace(r##""#000000","40px"##, r#""transparent","40px"#);
        assert_eq!(inked(64, 48, &clear), 0);
    }

    #[test]
    fn text_under_a_singular_transform_inks_nothing() {
        let log = r##"[["scale",[0,1]],["fillText",["Ag",4,30],"#000000","#000000","30px sans-serif","alphabetic","start",1,"source-over"]]"##;
        assert_eq!(inked(64, 40, log), 0);
    }

    #[test]
    fn straight_alpha_readback_unpremultiplies_with_rounding() {
        // 40% green over transparent: tiny-skia stores the premultiplied value; the
        // readback rounds `premul * 255 / a` back to the straight channel.
        let log = r##"[["fillRect",[0,0,2,2],"rgba(0,255,0,0.4)","#000","10px x","alphabetic","start",1,"source-over"]]"##;
        let rgba = replay_rgba(2, 2, &canvas_ops::parse_ops(log).unwrap()).unwrap();
        assert_eq!(&rgba[..4], &[0, 255, 0, 102]);
        assert_eq!(rgba.len(), 16);
    }

    #[test]
    fn unpremultiply_divides_in_single_precision_rounding_ties_to_even() {
        assert_eq!(unpremultiply([0, 0, 0, 0]), [0, 0, 0, 0]);
        assert_eq!(
            unpremultiply([10, 20, 30, 0]),
            [0, 0, 0, 0],
            "no alpha, no colour"
        );
        for c in 0..=255 {
            assert_eq!(
                unpremultiply([c, c / 2, 255 - c, 255]),
                [c, c / 2, 255 - c, 255]
            );
        }
        // The exact-half quotients Chrome 156 resolves (single precision decides them):
        // 18/68 -> 67.5 -> 67, 51/102 -> 127.5 -> 128, 13/102 -> 32.5 -> 32, 5/34 -> 37.
        assert_eq!(unpremultiply([18, 51, 13, 68]), [67, 191, 49, 68]);
        assert_eq!(unpremultiply([51, 13, 41, 102]), [128, 32, 102, 102]);
        assert_eq!(unpremultiply([5, 2, 11, 34]), [37, 15, 82, 34]);
        // A premultiplied channel above its alpha (not producible by painting) saturates.
        assert_eq!(unpremultiply([200, 0, 0, 100]), [255, 0, 0, 100]);
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

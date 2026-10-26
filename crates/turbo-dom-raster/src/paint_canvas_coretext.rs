//! macOS-only canvas TEXT rasterizer via CoreText + CoreGraphics — the same system glyph stack
//! Chrome's Skia uses on macOS, so `getImageData` over rendered text reads back like a real Mac
//! Chrome instead of our ttf-parser + tiny-skia glyph trace. Gated behind `feature = "coretext"`
//! AND `target_os = "macos"` (the frameworks don't exist elsewhere). Shapes stay on tiny-skia; only
//! the text glyph raster is swapped, so shape pixels don't regress.
//!
//! Scope: handles an identity/translate CTM (the shape of essentially every canvas fingerprint).
//! For a scaled/rotated text matrix it returns `false` and the caller falls back to the vector
//! glyph trace (correct, just not CoreText-hinted) — keeping this module simple + robust.

use crate::canvas_ops::State;
use core_foundation::attributed_string::CFMutableAttributedString;
use core_foundation::base::{CFRange, TCFType};
use core_foundation::string::CFString;
use core_graphics::base::kCGImageAlphaPremultipliedLast;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_text::line::CTLine;
use tiny_skia::Pixmap;
use turbo_html2pdf_core::Rgba;

/// Rasterize `text` at canvas coords `(x, y)` (baseline) in `color` and source-over composite it
/// onto `pm`. Returns `false` (caller falls back to the vector glyph trace) when the CTM is not a
/// pure identity/translate. Never panics.
pub(crate) fn render_text(
    pm: &mut Pixmap,
    st: &State,
    text: &str,
    x: f32,
    y: f32,
    color: Rgba,
) -> bool {
    if text.is_empty() {
        return true;
    }
    // Only pure identity/translate: [a,b,c,d,e,f] with a==1,b==0,c==0,d==1.
    let m = st.ctm.0;
    let is_translate = (m[0] - 1.0).abs() < 1e-3
        && m[1].abs() < 1e-3
        && m[2].abs() < 1e-3
        && (m[3] - 1.0).abs() < 1e-3;
    if !is_translate {
        return false;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    if w == 0 || h == 0 {
        return false;
    }
    let dx = (x + m[4]) as f64;
    let dy = (y + m[5]) as f64;

    // Transparent bitmap context, premultiplied RGBA (matches tiny-skia's byte layout).
    let cs = CGColorSpace::create_device_rgb();
    let mut ctx = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CGContext::create_bitmap_context(None, w, h, 8, w * 4, &cs, kCGImageAlphaPremultipliedLast)
    })) {
        Ok(c) => c,
        Err(_) => return false,
    };
    ctx.set_should_antialias(true);
    ctx.set_should_smooth_fonts(true);

    let family = st
        .font_families
        .first()
        .map(String::as_str)
        .unwrap_or("sans-serif");
    let size = if st.font_px > 0.0 { st.font_px } else { 10.0 } as f64;
    let font = match core_text::font::new_from_name(family, size)
        .or_else(|_| core_text::font::new_from_name("Helvetica", size))
    {
        Ok(f) => f,
        Err(_) => return false,
    };

    // Attributed string: font + explicit foreground color (straight-alpha RGBA → 0..1).
    let cfs = CFString::new(text);
    let mut att = CFMutableAttributedString::new();
    att.replace_str(&cfs, CFRange::init(0, 0));
    // CFAttributedString ranges are in UTF-16 code units, not Unicode scalars — an
    // astral char (emoji, CJK-ext) is one `char` but two UTF-16 units, so counting
    // scalars would leave the trailing units unstyled. Count UTF-16 units.
    let n = text.encode_utf16().count() as isize;
    let full = CFRange::init(0, n);
    unsafe {
        att.set_attribute(
            full,
            core_text::string_attributes::kCTFontAttributeName,
            &font,
        );
        let cg = core_graphics::color::CGColor::rgb(
            color.r as f64 / 255.0,
            color.g as f64 / 255.0,
            color.b as f64 / 255.0,
            color.a as f64 / 255.0,
        );
        att.set_attribute(
            full,
            core_text::string_attributes::kCTForegroundColorAttributeName,
            &cg,
        );
    }
    let line = CTLine::new_with_attributed_string(att.as_concrete_TypeRef());
    // CoreGraphics origin is bottom-left; canvas baseline y grows downward → CG y = h - dy.
    ctx.set_text_position(dx, h as f64 - dy);
    line.draw(&ctx);

    // Source-over composite the (premultiplied) CG bitmap onto the (premultiplied) pixmap.
    let src = ctx.data();
    let dst = pm.data_mut();
    if src.len() != dst.len() {
        return false;
    }
    for i in (0..dst.len()).step_by(4) {
        let sa = src[i + 3] as u32;
        if sa == 0 {
            continue;
        }
        let inv = 255 - sa;
        for c in 0..4 {
            let s = src[i + c] as u32;
            let d = dst[i + c] as u32;
            dst[i + c] = (s + (d * inv + 127) / 255).min(255) as u8;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // CoreText renders text into the pixmap (a real glyph raster, not a no-op) and composites it —
    // non-transparent pixels appear within the expected text region, with near-opaque black core.
    #[test]
    fn coretext_rasterizes_text_into_pixmap() {
        let mut pm = Pixmap::new(240, 40).unwrap();
        let mut st = State::default();
        st.font_px = 20.0;
        st.font_families = vec!["Arial".to_string()];
        let ok = render_text(&mut pm, &st, "Hello", 2.0, 28.0, Rgba::new(0, 0, 0, 255));
        assert!(ok, "identity-CTM text must render via CoreText");
        let data = pm.data();
        let mut nz = 0usize;
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (999i32, 0i32, 999i32, 0i32);
        for y in 0..40i32 {
            for x in 0..240i32 {
                let a = data[((y * 240 + x) * 4 + 3) as usize];
                if a != 0 {
                    nz += 1;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        assert!(
            nz > 200,
            "text covers a browser-like pixel count (got {nz})"
        );
        // "Hello" at 20px on a baseline of 28 sits in the upper-left; not off-canvas or full-frame.
        assert!(
            min_x >= 0 && max_x < 80,
            "x extent within the left region: {min_x}..{max_x}"
        );
        assert!(
            min_y > 5 && max_y <= 28,
            "y extent above the baseline: {min_y}..{max_y}"
        );
    }

    // A scaled/rotated CTM is not handled here → returns false so the caller uses the vector trace.
    #[test]
    fn coretext_declines_non_translate_ctm() {
        let mut pm = Pixmap::new(64, 64).unwrap();
        let mut st = State::default();
        st.font_px = 16.0;
        st.font_families = vec!["Arial".to_string()];
        st.ctm = crate::canvas_ops::Matrix([2.0, 0.0, 0.0, 2.0, 0.0, 0.0]); // scale 2×
        assert!(!render_text(
            &mut pm,
            &st,
            "x",
            2.0,
            20.0,
            Rgba::new(0, 0, 0, 255)
        ));
    }
}

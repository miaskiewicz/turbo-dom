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
use core_text::font::CTFont;
use core_text::line::CTLine;
use tiny_skia::Pixmap;
use turbo_dom_layout::Rgba;

/// Rasterize `text` at canvas coords `(x, y)` (baseline) in `color` and source-over composite it
/// onto `pm`. Returns `false` (caller falls back to the vector glyph trace) when the CTM is not a
/// pure identity/translate. Never panics.
fn ctm_translate(m: &[f32; 6]) -> Option<(f32, f32)> {
    let is_translate = (m[0] - 1.0).abs() < 1e-3
        && m[1].abs() < 1e-3
        && m[2].abs() < 1e-3
        && (m[3] - 1.0).abs() < 1e-3;
    is_translate.then_some((m[4], m[5]))
}

fn draw_line(
    ctx: &mut CGContext,
    font: &CTFont,
    text: &str,
    color: Rgba,
    dx: f64,
    dy: f64,
    h: usize,
) {
    let cfs = CFString::new(text);
    let mut att = CFMutableAttributedString::new();
    att.replace_str(&cfs, CFRange::init(0, 0));
    let n = text.encode_utf16().count() as isize;
    let full = CFRange::init(0, n);
    unsafe {
        att.set_attribute(
            full,
            core_text::string_attributes::kCTFontAttributeName,
            font,
        );
        let cg = core_graphics::color::CGColor::rgb(
            f64::from(color.r) / 255.0,
            f64::from(color.g) / 255.0,
            f64::from(color.b) / 255.0,
            f64::from(color.a) / 255.0,
        );
        att.set_attribute(
            full,
            core_text::string_attributes::kCTForegroundColorAttributeName,
            &cg,
        );
    }
    let line = CTLine::new_with_attributed_string(att.as_concrete_TypeRef());
    ctx.set_text_position(dx, h as f64 - dy);
    line.draw(ctx);
}

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
    let Some((tx, ty)) = ctm_translate(&st.ctm.0) else {
        return false;
    };
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    if w == 0 || h == 0 {
        return false;
    }
    let cs = CGColorSpace::create_device_rgb();
    let Ok(mut ctx) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CGContext::create_bitmap_context(None, w, h, 8, w * 4, &cs, kCGImageAlphaPremultipliedLast)
    })) else {
        return false;
    };
    ctx.set_should_antialias(true);
    ctx.set_should_smooth_fonts(true);
    let Some(font) = ct_font(st) else {
        return false;
    };
    draw_line(
        &mut ctx,
        &font,
        text,
        color,
        f64::from(x + tx),
        f64::from(y + ty),
        h,
    );
    composite_over(ctx.data(), pm.data_mut())
}

fn composite_over(src: &[u8], dst: &mut [u8]) -> bool {
    if src.len() != dst.len() {
        return false;
    }
    for i in (0..dst.len()).step_by(4) {
        let sa = u32::from(src[i + 3]);
        if sa == 0 {
            continue;
        }
        let inv = 255 - sa;
        for c in 0..4 {
            let s = u32::from(src[i + c]);
            let d = u32::from(dst[i + c]);
            dst[i + c] = (s + (d * inv + 127) / 255).min(255) as u8;
        }
    }
    true
}

/// The CoreText face for the canvas font: the first family at the font size (Helvetica for
/// a CSS generic family), switched to its bold and/or italic face by the state's weight
/// (CSS bold threshold, 600 and up) and style. A family without the requested face keeps the
/// face it has, as Chrome does before synthesizing. `None` for a named family CoreText does
/// not have (a font only the bundled registry or a web font provides, like the fixture
/// Liberation Serif): CoreText would substitute another face, so the caller traces the
/// registry's glyphs instead (miaskiewicz/turbo-dom#342).
pub(crate) fn ct_font(st: &State) -> Option<core_text::font::CTFont> {
    use core_text::font_descriptor::{kCTFontBoldTrait, kCTFontItalicTrait};
    let family = st
        .font_families
        .first()
        .map_or("sans-serif", String::as_str);
    let size = f64::from(if st.font_px > 0.0 { st.font_px } else { 10.0 });
    let generic = [
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
    ]
    .contains(&family.to_ascii_lowercase().as_str());
    let font = if generic {
        core_text::font::new_from_name("Helvetica", size).ok()?
    } else {
        let font = core_text::font::new_from_name(family, size).ok()?;
        if !font.family_name().eq_ignore_ascii_case(family) {
            return None;
        }
        font
    };
    let mut traits = 0;
    if st.font_weight >= 600 {
        traits |= kCTFontBoldTrait;
    }
    if st.font_italic {
        traits |= kCTFontItalicTrait;
    }
    let mask = kCTFontBoldTrait | kCTFontItalicTrait;
    Some(
        font.clone_with_symbolic_traits(traits, mask)
            .unwrap_or(font),
    )
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

    // The canvas font's weight and style pick the CoreText face (turbo-dom#236), as Chrome's
    // font matching picks Arial Bold for `bold 20px Arial`: not the family's regular face.
    #[test]
    fn coretext_font_honours_weight_and_style() {
        let state = |weight, italic| {
            let mut st = State::default();
            st.font_px = 20.0;
            st.font_families = vec!["Arial".to_string()];
            st.font_weight = weight;
            st.font_italic = italic;
            st
        };
        let face = |weight, italic| ct_font(&state(weight, italic)).unwrap().postscript_name();
        assert_eq!(face(400, false), "ArialMT");
        assert_eq!(face(700, false), "Arial-BoldMT");
        assert_eq!(face(400, true), "Arial-ItalicMT");
        assert_eq!(face(700, true), "Arial-BoldItalicMT");
        // CSS bold threshold: 600 and up select the bold face, 500 stays regular.
        assert_eq!(face(600, false), "Arial-BoldMT");
        assert_eq!(face(500, false), "ArialMT");
        // The bold face draws more ink than the regular one.
        let ink = |weight| {
            let mut pm = Pixmap::new(240, 40).unwrap();
            let st = state(weight, false);
            assert!(render_text(
                &mut pm,
                &st,
                "Hello",
                2.0,
                28.0,
                Rgba::new(0, 0, 0, 255)
            ));
            pm.data().chunks(4).map(|p| u32::from(p[3])).sum::<u32>()
        };
        assert!(ink(700) > ink(400));
    }

    // A named family CoreText does not have declines (the caller traces the registry's
    // glyphs, turbo-dom#342); a generic family is Helvetica.
    #[test]
    fn coretext_declines_a_family_it_does_not_have() {
        let state = |family: &str| {
            let mut st = State::default();
            st.font_px = 20.0;
            st.font_families = vec![family.to_string()];
            st
        };
        assert!(ct_font(&state("No Such Family 342")).is_none());
        assert!(!render_text(
            &mut Pixmap::new(64, 32).unwrap(),
            &state("No Such Family 342"),
            "x",
            2.0,
            20.0,
            Rgba::new(0, 0, 0, 255)
        ));
        let helvetica = ct_font(&state("sans-serif")).unwrap().family_name();
        assert_eq!(helvetica, "Helvetica");
        assert_eq!(
            ct_font(&State::default()).unwrap().family_name(),
            "Helvetica"
        );
        assert_eq!(ct_font(&state("arial")).unwrap().family_name(), "Arial");
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

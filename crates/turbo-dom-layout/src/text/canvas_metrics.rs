//! Canvas `measureText` metrics as Chrome 156 computes them (miaskiewicz/turbo-dom#237):
//! the run's width ([`FontFace::measure_canvas`]), its ink box and the font's box and em
//! box, all relative to the alphabetic baseline at the start of the run. A canvas binding
//! shifts them for `textAlign` / `textBaseline`.
use super::FontFace;
use rustybuzz::ttf_parser::GlyphId;

/// Blink's `TextMetrics` before the `textAlign` / `textBaseline` offsets. Lengths are CSS
/// pixels in `f32` like Blink's (the width is the exact 16.16 sum); every box extent is
/// positive away from the baseline / start (`ink_left` is negative when the ink starts
/// right of the origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasTextMetrics {
    pub width: f64,
    pub ink_left: f32,
    pub ink_right: f32,
    pub ink_ascent: f32,
    pub ink_descent: f32,
    /// The font's rounded ascent / descent ([`FontFace::html_metrics`]).
    pub font_ascent: f32,
    pub font_descent: f32,
    /// The em box: the OS/2 typo ascent normalized so ascent + descent is the font size,
    /// each snapped to a 1/64 px `LayoutUnit` (Blink `NormalizedTypoAscentAndDescent`).
    pub em_ascent: f32,
    pub em_descent: f32,
}

/// `LayoutUnit::FromFloatRound`: the nearest 1/64 px.
fn layout_unit(value: f32) -> f32 {
    (value * 64.0).round() / 64.0
}

impl FontFace {
    /// Canvas `measureText(text)` at `font_size` px: Blink's word-by-word 16.16 advances
    /// for the width and the glyph positions, the union of the glyphs' outline boxes for
    /// the ink box (zero when no glyph has ink), and the font and em boxes.
    #[must_use]
    pub fn canvas_text_metrics(&self, text: &str, font_size: f32) -> CanvasTextMetrics {
        let scale = self.scale(font_size);
        let mut pen = 0.0f64;
        let mut ink: Option<[f32; 4]> = None;
        let glyphs = super::canvas_words(text).flat_map(|word| {
            let glyphs = self.shape(word);
            let advances = self.blink_advances(&glyphs, font_size, 1.0);
            glyphs.into_iter().zip(advances)
        });
        for (glyph, advance) in glyphs {
            if let Some(b) = self.ttf().glyph_bounding_box(GlyphId(glyph.glyph_id)) {
                let x = pen as f32 + glyph.x_offset as f32 * scale;
                let y = glyph.y_offset as f32 * scale;
                let rect = [
                    x + f32::from(b.x_min) * scale,
                    x + f32::from(b.x_max) * scale,
                    y + f32::from(b.y_max) * scale,
                    -(y + f32::from(b.y_min) * scale),
                ];
                ink = Some(ink.map_or(rect, |u| {
                    [
                        u[0].min(rect[0]),
                        u[1].max(rect[1]),
                        u[2].max(rect[2]),
                        u[3].max(rect[3]),
                    ]
                }));
            }
            pen += advance;
        }
        let [left, right, ascent, descent] = ink.unwrap_or_default();
        let font = self.html_metrics(font_size);
        let (em_ascent, em_descent) = self.em_box(font_size, font.ascent, font.descent);
        CanvasTextMetrics {
            width: pen,
            ink_left: -left,
            ink_right: right,
            ink_ascent: ascent,
            ink_descent: descent,
            font_ascent: font.ascent,
            font_descent: font.descent,
            em_ascent,
            em_descent,
        }
    }

    /// Blink `NormalizedTypoAscentAndDescent`: the OS/2 typo ascent / descent (else the
    /// font's rounded ascent / descent) scaled so they add up to the font size.
    fn em_box(&self, font_size: f32, ascent: f32, descent: f32) -> (f32, f32) {
        let ttf = self.ttf();
        let scale = self.scale(font_size);
        let (ascent, descent) = ttf
            .typographic_ascender()
            .zip(ttf.typographic_descender())
            .filter(|&(a, d)| i32::from(a) - i32::from(d) > 0)
            .map_or((ascent, descent), |(a, d)| {
                (f32::from(a) * scale, -f32::from(d) * scale)
            });
        let em_ascent = layout_unit(ascent * font_size / (ascent + descent));
        (em_ascent, layout_unit(font_size) - em_ascent)
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasTextMetrics, FontFace};

    fn inter() -> FontFace {
        let path = format!(
            "{}/assets/fonts/inter/Inter-Regular.otf",
            env!("CARGO_MANIFEST_DIR")
        );
        FontFace::from_bytes(std::fs::read(path).unwrap(), "Inter", 400, false).unwrap()
    }

    /// Chrome 156 `measureText` with `16px Inter` (turbo-dom-browser-env's
    /// `canvas_measure_text_chrome156.json`): ink extents within Skia's f32 noise.
    #[test]
    fn canvas_text_metrics_match_chrome156() {
        let m = inter().canvas_text_metrics("Typography", 16.0);
        assert_eq!(m.width, 90.546_875);
        let ink = [m.ink_left, m.ink_right, m.ink_ascent, m.ink_descent];
        let chrome = [-0.765_623_6, 90.124_985, 11.640_603, 3.453_118_3];
        for (got, want) in ink.iter().zip(chrome) {
            assert!((got - want).abs() < 1e-4, "{ink:?} vs {chrome:?}");
        }
        assert_eq!((m.font_ascent, m.font_descent), (16.0, 4.0));
        assert_eq!((m.em_ascent, m.em_descent), (12.8125, 3.1875));
    }

    /// A run without ink (empty, or only spaces) has an empty ink box but its advance.
    #[test]
    fn inkless_runs_have_a_zero_ink_box() {
        let m = inter().canvas_text_metrics(" ", 16.0);
        assert_eq!(
            m,
            CanvasTextMetrics {
                width: 4.5,
                ink_left: 0.0,
                ink_right: 0.0,
                ink_ascent: 0.0,
                ink_descent: 0.0,
                font_ascent: 16.0,
                font_descent: 4.0,
                em_ascent: 12.8125,
                em_descent: 3.1875,
            }
        );
        assert_eq!(inter().canvas_text_metrics("", 16.0).width, 0.0);
    }
}

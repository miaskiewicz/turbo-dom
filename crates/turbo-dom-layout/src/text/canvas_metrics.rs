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

/// How Blink's canvas font shapes a run (`fontKerning` / `textRendering`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CanvasShaping {
    /// `fontKerning: auto`: word by word (`CachingWordShaper`), kerning inside words only.
    #[default]
    WordByWord,
    /// `fontKerning: none`: word by word without the `kern` feature.
    NoKerning,
    /// `fontKerning: normal`, `textRendering: optimizeLegibility | geometricPrecision`:
    /// the whole run is shaped at once, so kerning pairs cross spaces.
    WholeRun,
}

/// The canvas text attributes that change `measureText`: `letterSpacing` and
/// `wordSpacing` in px (added after every character / every space, as Blink does) and the
/// shaping mode.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CanvasTextOptions {
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub shaping: CanvasShaping,
}

/// The union of two ink rectangles `[left, right, ascent, descent]`.
fn union(a: Option<[f32; 4]>, b: [f32; 4]) -> [f32; 4] {
    a.map_or(b, |u| {
        [
            u[0].min(b[0]),
            u[1].max(b[1]),
            u[2].max(b[2]),
            u[3].max(b[3]),
        ]
    })
}

/// The spacing after glyph `index` of `glyphs` (shaped from `segment`): nothing inside a
/// cluster; at a cluster's end, `letter_spacing` per character and `word_spacing` per
/// space of the cluster's text.
fn spacing(
    segment: &str,
    glyphs: &[super::ShapedGlyph],
    index: usize,
    options: CanvasTextOptions,
) -> f64 {
    let start = glyphs[index].cluster as usize;
    let end = match glyphs.get(index + 1) {
        Some(next) if next.cluster as usize == start => return 0.0,
        Some(next) => next.cluster as usize,
        None => segment.len(),
    };
    let text = segment.get(start.min(end)..start.max(end)).unwrap_or("");
    let chars = text.chars().count() as f32;
    let spaces = text.matches(' ').count() as f32;
    f64::from(options.letter_spacing * chars + options.word_spacing * spaces)
}

impl FontFace {
    /// Canvas `measureText(text)` at `font_size` px with default text attributes
    /// ([`Self::canvas_text_metrics_with`]).
    #[must_use]
    pub fn canvas_text_metrics(&self, text: &str, font_size: f32) -> CanvasTextMetrics {
        self.canvas_text_metrics_with(text, font_size, CanvasTextOptions::default())
    }

    /// Canvas `measureText(text)` at `font_size` px under `options`: Blink's 16.16 glyph
    /// advances plus the spacing for the width and the glyph positions, the union of the
    /// glyphs' outline boxes for the ink box (zero when no glyph has ink), and the font and
    /// em boxes.
    #[must_use]
    pub fn canvas_text_metrics_with(
        &self,
        text: &str,
        font_size: f32,
        options: CanvasTextOptions,
    ) -> CanvasTextMetrics {
        let mut pen = 0.0f64;
        let mut ink: Option<[f32; 4]> = None;
        let segments: Vec<&str> = match options.shaping {
            CanvasShaping::WholeRun => vec![text],
            _ => super::canvas_words(text).collect(),
        };
        for segment in segments {
            let glyphs = self.canvas_shape(segment, options.shaping);
            let advances = self.blink_advances(&glyphs, font_size, 1.0);
            for (index, glyph) in glyphs.iter().enumerate() {
                if let Some(rect) = self.glyph_ink(glyph, pen as f32, font_size) {
                    ink = Some(union(ink, rect));
                }
                pen += advances[index];
                pen += spacing(segment, &glyphs, index, options);
            }
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

    /// The face's x-height in px (OS/2 `sxHeight`, else half the font size): the CSS `ex`.
    #[must_use]
    pub fn x_height_px(&self, font_size: f32) -> f32 {
        self.ttf()
            .x_height()
            .map_or(font_size / 2.0, |h| f32::from(h) * self.scale(font_size))
    }

    /// The face's cap height in px (OS/2 `sCapHeight`, else the ascent): the CSS `cap`.
    #[must_use]
    pub fn cap_height_px(&self, font_size: f32) -> f32 {
        self.ttf()
            .capital_height()
            .map_or(self.ascent_px(font_size), |h| {
                f32::from(h) * self.scale(font_size)
            })
    }

    /// The advance of U+6C34 (水) in px when the face has the glyph: the CSS `ic` (which
    /// is 1em otherwise).
    #[must_use]
    pub fn ideographic_advance_px(&self, font_size: f32) -> Option<f32> {
        let glyph = self.glyph_index('\u{6c34}')?;
        Some(f32::from(self.glyph_advance(glyph)) * self.scale(font_size))
    }

    fn canvas_shape(&self, text: &str, shaping: CanvasShaping) -> Vec<super::ShapedGlyph> {
        if shaping != CanvasShaping::NoKerning {
            return self.shape(text);
        }
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let no_kern =
            rustybuzz::Feature::new(rustybuzz::ttf_parser::Tag::from_bytes(b"kern"), 0, ..);
        let shaped = rustybuzz::shape(&self.faces.borrow_dependent().rb, &[no_kern], buffer);
        super::collect_glyphs(&shaped)
    }

    /// The glyph's outline box at pen position `pen`, or `None` for a glyph without ink.
    fn glyph_ink(&self, glyph: &super::ShapedGlyph, pen: f32, font_size: f32) -> Option<[f32; 4]> {
        let scale = self.scale(font_size);
        let b = self.ttf().glyph_bounding_box(GlyphId(glyph.glyph_id))?;
        let x = pen + glyph.x_offset as f32 * scale;
        let y = glyph.y_offset as f32 * scale;
        Some([
            x + f32::from(b.x_min) * scale,
            x + f32::from(b.x_max) * scale,
            y + f32::from(b.y_max) * scale,
            -(y + f32::from(b.y_min) * scale),
        ])
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

    fn serif() -> FontFace {
        let path = format!(
            "{}/assets/fonts/liberation-serif/LiberationSerif-Regular.ttf",
            env!("CARGO_MANIFEST_DIR")
        );
        FontFace::from_bytes(std::fs::read(path).unwrap(), "Liberation Serif", 400, false).unwrap()
    }

    /// Chrome 156 widths under the canvas text attributes (turbo-dom-browser-env's
    /// `canvas_text_attributes_chrome156.json`, 16px).
    #[test]
    fn text_attributes_change_widths_like_chrome156() {
        use super::{CanvasShaping, CanvasTextOptions};
        let width = |face: &FontFace, text, options| {
            face.canvas_text_metrics_with(text, 16.0, options).width
        };
        let shaped = |shaping| CanvasTextOptions {
            shaping,
            ..CanvasTextOptions::default()
        };
        let serif = serif();
        assert_eq!(
            width(&serif, "AVATAR Wave", shaped(CanvasShaping::WordByWord)),
            99.507_812_5
        );
        assert_eq!(
            width(&serif, "AVATAR Wave", shaped(CanvasShaping::WholeRun)),
            99.218_75
        );
        assert_eq!(
            width(&serif, "AVATAR Wave", shaped(CanvasShaping::NoKerning)),
            107.968_75
        );
        let spaced = CanvasTextOptions {
            letter_spacing: 2.0,
            ..CanvasTextOptions::default()
        };
        let m = serif.canvas_text_metrics_with("AVATAR Wave", 16.0, spaced);
        assert_eq!((m.width, m.ink_right), (121.507_812_5, 118.953_125));
        let words = CanvasTextOptions {
            letter_spacing: 1.0,
            word_spacing: 3.0,
            ..CanvasTextOptions::default()
        };
        assert_eq!(width(&inter(), "a b  c", words), 56.421_875);
    }

    #[test]
    fn x_height_reads_the_os2_table() {
        let x = inter().x_height_px(16.0);
        assert!(x > 7.0 && x < 9.5, "{x}");
    }

    /// Chrome 156's canvas `1cap` with `10px Inter` is 7.275390625 (Inter's sCapHeight
    /// 1490 / 2048 em); Inter has no U+6C34, so `ic` falls back to 1em.
    #[test]
    fn cap_height_and_ideographic_advance() {
        assert_eq!(inter().cap_height_px(10.0), 7.275_390_6);
        assert_eq!(inter().ideographic_advance_px(10.0), None);
    }
}

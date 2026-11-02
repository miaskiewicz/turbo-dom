//! Per-glyph advances of a shaped run as Blink positions them (turbo-dom#191, #236).
//!
//! Blink's `HarfBuzz` font is scaled: glyph advances come from Skia as 16.16 positions
//! truncated toward zero (`SkiaScalarToHarfBuzzPosition`), at the size the font cache
//! created the font at ([`blink_font_size`]). `rustybuzz` shapes unscaled, in design units,
//! and that is exact for GPOS, whose adjustments are added whole. The legacy `kern` table
//! is different: `HarfBuzz`'s kern machine (`hb-kern.hh`, rustybuzz `hb/kerning.rs`) splits
//! each pair value in half, `kern >> 1` onto the first glyph's advance and the rest onto the
//! second's, in the font's units — 16.16 scaled positions in Blink, whole design units in
//! `rustybuzz`. So for a face kerned by its `kern` table (no GPOS, no `kerx`: the macOS
//! system Times, Helvetica, ...) the advances are rebuilt from `hmtx` plus each pair's
//! scaled half, as Blink sees them.

use super::font::{blink_font_size, FontFace, ShapedGlyph};
use rustybuzz::ttf_parser::{kern, GlyphId};

impl FontFace {
    /// Each glyph's advance in CSS px, as Blink lays out HTML text at `device_scale` (a
    /// canvas font: scale 1): [`blink_device_glyph_advance`](Self::blink_device_glyph_advance)
    /// per glyph, except that a `kern`-table pair adjustment is split in 16.16 device units
    /// the way Blink's scaled `HarfBuzz` font splits it (module docs).
    #[must_use]
    pub fn blink_advances(
        &self,
        glyphs: &[ShapedGlyph],
        font_size: f32,
        device_scale: f32,
    ) -> Vec<f64> {
        let pairs = self.kern_table_pairs(glyphs).unwrap_or_default();
        self.split_kern_advances(glyphs, &pairs, font_size, device_scale)
    }

    fn split_kern_advances(
        &self,
        glyphs: &[ShapedGlyph],
        pairs: &[Vec<i32>],
        font_size: f32,
        device_scale: f32,
    ) -> Vec<f64> {
        // `hb_font_set_scale(SkiaScalarToHarfBuzzPosition(size))`, then `em_mult`.
        let x_scale = i64::from((blink_font_size(font_size * device_scale) * 65536.0) as i32);
        let mult = (x_scale << 16) / i64::from(self.units_per_em());
        let em = |v: i32| (i64::from(v) * mult + 32768) >> 16;
        let none = Vec::new();
        (0..glyphs.len())
            .map(|i| {
                let base = self.blink_device_glyph_advance(
                    font_size,
                    device_scale,
                    i32::from(self.glyph_advance(glyphs[i].glyph_id)),
                );
                let after = pairs.get(i).unwrap_or(&none);
                let before = i.checked_sub(1).and_then(|p| pairs.get(p)).unwrap_or(&none);
                let split: i32 = after.iter().map(|&k| k >> 1).sum::<i32>()
                    + before.iter().map(|&k| k - (k >> 1)).sum::<i32>();
                // Whatever else the shaper added (GPOS) is one scaled adjustment.
                let rest =
                    glyphs[i].x_advance - i32::from(self.glyph_advance(glyphs[i].glyph_id)) - split;
                let units: i64 = after.iter().map(|&k| em(k) >> 1).sum::<i64>()
                    + before.iter().map(|&k| em(k) - (em(k) >> 1)).sum::<i64>()
                    + em(rest);
                base + units as f64 / 65536.0 / f64::from(device_scale)
            })
            .collect()
    }

    /// For a face `rustybuzz` kerns by its `kern` table, every adjacent glyph pair's
    /// value in each applied subtable (design units), when they account for every shaped
    /// advance exactly (`hmtx` plus the kern machine's integer halves); `None` for any
    /// other face or run (GPOS, `kerx`, marks skipped by the kern machine, a state-machine
    /// or cross-stream subtable, an advance that is not explained), which keeps the plain
    /// per-glyph conversion.
    fn kern_table_pairs(&self, glyphs: &[ShapedGlyph]) -> Option<Vec<Vec<i32>>> {
        let tables = self.ttf().tables();
        if tables.gpos.is_some() || tables.kerx.is_some() {
            return None;
        }
        let subtables: Vec<kern::Subtable> = tables
            .kern?
            .subtables
            .into_iter()
            .filter(|s| s.horizontal && !s.variable)
            .collect();
        if subtables
            .iter()
            .any(|s| s.has_state_machine || s.has_cross_stream)
        {
            return None;
        }
        let pairs: Vec<Vec<i32>> = glyphs
            .windows(2)
            .map(|w| {
                let (left, right) = (GlyphId(w[0].glyph_id), GlyphId(w[1].glyph_id));
                subtables
                    .iter()
                    .map(|s| i32::from(s.glyphs_kerning(left, right).unwrap_or(0)))
                    .collect()
            })
            .collect();
        self.pairs_explain(glyphs, &pairs).then_some(pairs)
    }

    fn pairs_explain(&self, glyphs: &[ShapedGlyph], pairs: &[Vec<i32>]) -> bool {
        glyphs.iter().enumerate().all(|(i, g)| {
            let after: i32 = pairs.get(i).map_or(0, |p| p.iter().map(|k| k >> 1).sum());
            let before: i32 = i
                .checked_sub(1)
                .map_or(0, |p| pairs[p].iter().map(|k| k - (k >> 1)).sum());
            g.x_advance == i32::from(self.glyph_advance(g.glyph_id)) + after + before
        })
    }
}

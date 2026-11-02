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
        let kern = tables
            .kern
            .filter(|_| tables.gpos.is_none() && tables.kerx.is_none())?;
        let subtables: Vec<kern::Subtable> = kern
            .subtables
            .into_iter()
            .filter(|s| s.horizontal && !s.variable)
            .collect();
        let simple = subtables
            .iter()
            .all(|s| !s.has_state_machine && !s.has_cross_stream);
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
        (simple && self.pairs_explain(glyphs, &pairs)).then_some(pairs)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn face(file: &str) -> FontFace {
        let path = format!("{}/assets/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
        FontFace::from_bytes(std::fs::read(path).unwrap(), "test", 400, false).unwrap()
    }

    // Warsaw Gothic kerns by a `kern` table and has no GPOS: rustybuzz splits each pair
    // `k >> 1` / `k - (k >> 1)` in design units, Blink's scaled font splits the scaled pair.
    // (The macOS-only system Times control pins this against Chrome; Chrome applies no
    // kerning to this web font, so it has no portable control.)
    #[test]
    fn kern_table_pairs_are_split_in_scaled_units() {
        let warsaw = face("WarsawGothic-BnBV.otf");
        // The first Latin letter pair the table kerns by an odd value (where the design-unit
        // and scaled halves differ).
        let (glyphs, k) = ('A'..='Z')
            .flat_map(|l| ('a'..='z').map(move |r| format!("{l}{r}")))
            .find_map(|text| {
                let glyphs = warsaw.shape(&text);
                let k = warsaw.kern_table_pairs(&glyphs)?[0][0];
                (k % 2 != 0).then_some((glyphs, k))
            })
            .expect("an odd kern-table pair");
        let [a, v] = [0, 1].map(|i| i32::from(warsaw.glyph_advance(glyphs[i].glyph_id)));
        assert_eq!(glyphs[0].x_advance, a + (k >> 1));
        assert_eq!(glyphs[1].x_advance, v + k - (k >> 1));
        let got = warsaw.blink_advances(&glyphs, 16.0, 1.0);
        let unit = |x: i64| x as f64 / 65536.0;
        let mult = (1_i64 << 36) / i64::from(warsaw.units_per_em());
        let em = (i64::from(k) * mult + 32768) >> 16;
        assert_eq!(got[0], warsaw.blink_glyph_advance(16.0, a) + unit(em >> 1));
        assert_eq!(
            got[1],
            warsaw.blink_glyph_advance(16.0, v) + unit(em - (em >> 1))
        );
        // A GPOS face is not rebuilt from pairs.
        let serif = face("liberation-serif/LiberationSerif-Regular.ttf");
        assert!(serif.kern_table_pairs(&serif.shape("AV")).is_none());
    }
}

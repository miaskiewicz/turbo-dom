//! Reverse a font's Unicode `cmap` (glyph id -> lowest codepoint). Split out of
//! turbo-html2pdf-core's `emit/tounicode.rs`: the text layer needs it for per-glyph
//! source text, the PDF emitter for `/ToUnicode` CMaps.

use std::collections::BTreeMap;

use rustybuzz::ttf_parser::Face;

/// Reverse a font's Unicode `cmap` into a glyph-id → codepoint map by walking
/// every Unicode subtable. Keeps the lowest codepoint per glyph (insert-if-absent
/// in the ascending `codepoints` walk), so the `/ToUnicode` mapping a tagged PDF
/// needs is deterministic. A font with no `cmap` yields an empty map.
pub fn reverse_cmap(face: &Face) -> BTreeMap<u16, u32> {
    let mut out: BTreeMap<u16, u32> = BTreeMap::new();
    let Some(cmap) = face.tables().cmap else {
        return out;
    };
    for sub in cmap.subtables {
        if sub.is_unicode() {
            sub.codepoints(|cp| record_codepoint(&sub, cp, &mut out));
        }
    }
    out
}

/// Record `cp → glyph` reversed into `out`, keeping the first (lowest) codepoint
/// seen for each glyph so the `/ToUnicode` mapping is deterministic.
fn record_codepoint(
    sub: &rustybuzz::ttf_parser::cmap::Subtable,
    cp: u32,
    out: &mut BTreeMap<u16, u32>,
) {
    if let Some(gid) = sub.glyph_index(cp) {
        out.entry(gid.0).or_insert(cp);
    }
}

//! Reverse a font's Unicode `cmap` (glyph id -> lowest codepoint). Split out of
//! turbo-html2pdf-core's `emit/tounicode.rs`: the text layer needs it for per-glyph
//! source text, the PDF emitter for `/ToUnicode` `CMaps`.

use std::collections::BTreeMap;

use rustybuzz::ttf_parser::Face;

/// Reverse a font's Unicode `cmap` into a glyph-id → codepoint map by walking
/// every Unicode subtable. Keeps the lowest codepoint per glyph (insert-if-absent
/// in the ascending `codepoints` walk), so the `/ToUnicode` mapping a tagged PDF
/// needs is deterministic. A font with no `cmap` yields an empty map.
#[must_use]
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

#[cfg(test)]
mod tests {
    use super::reverse_cmap;
    use rustybuzz::ttf_parser::Face;

    fn font(name: &str) -> Vec<u8> {
        crate::test_support::font_bytes(name)
    }

    /// The bytes of `name` with its `cmap` table record renamed to `cmaq` (the
    /// directory stays sorted), so the face parses without a `cmap`.
    fn without_cmap(name: &str) -> Vec<u8> {
        let mut bytes = font(name);
        let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        let record = (0..tables)
            .map(|i| 12 + 16 * i)
            .find(|&at| &bytes[at..at + 4] == b"cmap")
            .expect("a cmap record");
        bytes[record + 3] = b'q';
        bytes
    }

    #[test]
    fn reverses_unicode_subtables_to_the_lowest_codepoint() {
        // Go Regular's format-4 cmap (independently decoded): A->36, a->68, e-acute->171,
        // space->3, no-break space->98.
        let bytes = font("Go-Regular.ttf");
        let map = reverse_cmap(&Face::parse(&bytes, 0).unwrap());
        for (gid, cp) in [(36, 0x41), (68, 0x61), (171, 0xE9), (3, 0x20), (98, 0xA0)] {
            assert_eq!(map.get(&gid), Some(&cp), "glyph {gid}");
        }
        assert_eq!(map.get(&0), None, ".notdef has no codepoint");
        // PT Serif maps U+00B5 and U+03BC (and U+0394/U+2206) to one glyph: the lowest
        // codepoint wins whatever the walk order.
        let bytes = font("pt-serif/PTSerif-Regular.ttf");
        let map = reverse_cmap(&Face::parse(&bytes, 0).unwrap());
        assert_eq!(map.get(&119), Some(&0xB5));
        assert_eq!(map.get(&312), Some(&0x394));
        assert_eq!(map.get(&313), Some(&0x3A9));
    }

    #[test]
    fn a_face_without_cmap_reverses_to_nothing() {
        let bytes = without_cmap("Go-Regular.ttf");
        let face = Face::parse(&bytes, 0).expect("parses without cmap");
        assert!(face.tables().cmap.is_none());
        assert!(reverse_cmap(&face).is_empty());
    }
}

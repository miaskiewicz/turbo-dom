// The single-element array expectations `ranges(...) == [a..=b]` are what the test
// is measuring: that *exactly one* range comes back. Rewriting them as a longer
// `Vec<RangeInclusive<_>>` to satisfy `single_range_in_vec_init` would hide the
// assertion. The one-element form is intentional in this file only.
#![allow(clippy::single_range_in_vec_init)]
//! `@font-face` `unicode-range` support (miaskiewicz/turbo-dom#539, CSS Fonts 4 §4.5).
//!
//! A page that registers Next.js-style split webfonts under one family — the latin-basic
//! WOFF2 and a latin-extended WOFF2 both as `Inter` — must pick the face whose declared
//! `unicode-range` covers each codepoint, not the last-registered one. Before the fix
//! `FontFace` had no `unicode_range` field at all, so a Latin-Extended subset ended up
//! serving `A-Z` as NO GLYPH.
use std::ops::RangeInclusive;
use turbo_dom_layout::text::{parse_unicode_range, FontFace, FontRegistry};

/// A fixture `.ttf` loaded as a web face so `is_web_font()` is true (the registry
/// prefers `@font-face` faces over installed ones of the same family).
fn web_face(bytes: &[u8], family: &str, weight: u16) -> FontFace {
    FontFace::from_web_font_bytes(bytes.to_vec(), family, weight, false)
        .expect("fixture font parses")
}

const LIBERATION_SERIF: &[u8] =
    include_bytes!("../assets/fonts/liberation-serif/LiberationSerif-Regular.ttf");
const INTER: &[u8] = include_bytes!("../assets/fonts/inter/Inter-Regular.otf");

fn ranges(css: &str) -> Vec<RangeInclusive<u32>> {
    parse_unicode_range(css).unwrap_or_else(|| panic!("parse_unicode_range({css:?}) returned None"))
}

#[test]
fn parse_unicode_range_single_hex_codepoint() {
    assert_eq!(ranges("U+26"), [0x26..=0x26]);
    assert_eq!(ranges("U+0041"), [0x41..=0x41]);
}

#[test]
fn parse_unicode_range_explicit_range() {
    assert_eq!(ranges("U+0-7F"), [0x00..=0x7F]);
    assert_eq!(ranges("U+0100-02BA"), [0x0100..=0x02BA]);
}

#[test]
fn parse_unicode_range_wildcard_shorthand() {
    // `U+??` is CSS Fonts 4's shorthand for U+00-FF, `U+4??` for U+400-4FF; a full
    // `U+????` covers the whole BMP (CSS Fonts 4 §4.5 "Character range").
    assert_eq!(ranges("U+??"), [0x00..=0xFF]);
    assert_eq!(ranges("U+4??"), [0x400..=0x4FF]);
    assert_eq!(ranges("U+????"), [0x0000..=0xFFFF]);
}

#[test]
fn parse_unicode_range_comma_separated_list() {
    // The real-world Next.js / Google Fonts split: one `@font-face` carries several
    // disjoint ranges in one descriptor.
    let parsed = parse_unicode_range("U+00-7F, U+0100-02BA, U+2C60-2C7F").unwrap();
    assert_eq!(
        parsed.as_slice(),
        [0x00..=0x7F, 0x0100..=0x02BA, 0x2C60..=0x2C7F]
    );
}

#[test]
fn parse_unicode_range_is_ascii_case_insensitive() {
    // Browsers accept both `U+` and `u+`, both `A-F` and `a-f` in the hex digits.
    assert_eq!(ranges("u+0-7f"), [0x00..=0x7F]);
}

#[test]
fn parse_unicode_range_rejects_malformed_input() {
    assert!(parse_unicode_range("").is_none());
    assert!(parse_unicode_range("0-7F").is_none(), "missing U+ prefix");
    assert!(parse_unicode_range("U+").is_none(), "no digits after U+");
    assert!(parse_unicode_range("U+GH").is_none(), "non-hex digits");
    assert!(
        parse_unicode_range("U+110000").is_none(),
        "above Unicode max"
    );
    assert!(parse_unicode_range("U+10-5").is_none(), "start > end");
    assert!(parse_unicode_range("U+1-2-3").is_none(), "two dashes");
}

#[test]
fn wildcard_blocks_reject_overlong_interleaved_and_nonhex_prefixes() {
    for (css, reason) in [
        ("U+000000?", "wildcard block exceeds six hex digits"),
        ("U+4?0", "a hex digit follows the first wildcard"),
        ("U+G?", "wildcard prefix is not hexadecimal"),
    ] {
        assert!(parse_unicode_range(css).is_none(), "{css}: {reason}");
    }
}

#[test]
fn face_without_a_declared_range_covers_every_codepoint_its_cmap_has() {
    // No declared range = inherit "U+0-10FFFF" default (CSS Fonts 4), the current
    // behaviour; changing it would regress every face registered before #539.
    let face = web_face(LIBERATION_SERIF, "Serif", 400);
    assert!(face.has_glyph('A'));
    assert!(face.has_glyph('é'));
}

#[test]
fn a_declared_range_hides_glyphs_outside_the_range_from_face_selection() {
    // The bug's repro, reduced: Liberation Serif's cmap covers 'A' AND 'é', but when
    // a page restricts the face to Latin-Extended with `unicode-range: U+0100-02BA`
    // the face must NOT be selected for 'A' (U+0041, outside the range). The face
    // must still be selected for 'é' (U+00E9) when the range covers it.
    let latin_ext = web_face(LIBERATION_SERIF, "Serif", 400)
        .with_unicode_range(parse_unicode_range("U+0100-02BA").unwrap());
    assert!(!latin_ext.has_glyph('A'), "A is outside the declared range");
    let covering = web_face(LIBERATION_SERIF, "Serif", 400)
        .with_unicode_range(parse_unicode_range("U+00-7F, U+00E9").unwrap());
    assert!(covering.has_glyph('é'));
    assert!(covering.has_glyph('A'));
}

#[test]
fn resolve_glyph_picks_the_face_whose_range_covers_the_codepoint() {
    // The end-to-end bug (`#539`): two faces under one family with disjoint ranges.
    // `A` (U+0041) must resolve to the latin-basic face even though BOTH cmaps have
    // the glyph. Before the fix the latin-extended face wins by its registration
    // position and emits glyph 0 for `A`.
    let basic =
        web_face(INTER, "Inter", 400).with_unicode_range(parse_unicode_range("U+00-7F").unwrap());
    let extended = web_face(LIBERATION_SERIF, "Inter", 400)
        .with_unicode_range(parse_unicode_range("U+0100-02BA").unwrap());
    let mut registry = FontRegistry::new();
    // Register the extended face FIRST so a last-registered-wins policy would
    // catch the bug.
    registry.add(extended);
    registry.add(basic);
    let picked = registry
        .resolve_glyph(&["Inter"], 400, false, 'A')
        .expect("some face covers A");
    assert_eq!(picked.family(), "Inter");
    // The right face uses Inter's bytes (300 KB OTF); the wrong one would use
    // Liberation Serif's TTF. A quick byte-length discriminator is enough.
    assert_eq!(picked.data().len(), INTER.len(), "latin-basic face chosen");
}

#[test]
fn resolve_glyph_falls_through_to_an_installed_face_when_no_range_covers() {
    // A codepoint outside every registered `unicode-range` must not match the family
    // at all; `resolve_glyph` then walks installed families / bundled faces.
    let a =
        web_face(INTER, "Inter", 400).with_unicode_range(parse_unicode_range("U+00-7F").unwrap());
    let mut registry = FontRegistry::new();
    registry.add(a);
    // '一' (U+4E00) is outside the declared ASCII range: the Inter face must not
    // be returned even though it is the only face in the registry's family.
    let picked = registry.resolve_glyph(&["Inter"], 400, false, '一');
    // Either None (no face), or a bundled fallback — but never the ASCII-ranged face.
    if let Some(face) = picked {
        assert_ne!(
            face.data().len(),
            INTER.len(),
            "ASCII-ranged face must not render '一'"
        );
    }
}

#[test]
fn with_unicode_range_is_cheap_to_clone_and_preserves_metrics() {
    // Attaching a range must not reparse the font program or lose any existing
    // metrics / family / weight state.
    let face = web_face(INTER, "Inter", 400);
    let metrics = face.html_metrics(16.0);
    let ranged = face.clone().with_unicode_range(vec![0x00..=0x7F]);
    assert_eq!(ranged.family(), "Inter");
    assert_eq!(ranged.html_metrics(16.0), metrics);
    assert!(ranged.is_web_font());
}

/// Behaviour assertion for the registry's whole-face selection. Before the fix,
/// `best_in_family` returned the first latin-extended face even for a page
/// whose dominant text was A-Z; now, when a range is declared, the face must
/// advertise as "unusable for A" so the family-level picker falls back.
#[test]
fn select_falls_through_a_ranged_face_that_does_not_cover_a_sample_codepoint() {
    let narrow = web_face(INTER, "Inter", 400)
        .with_unicode_range(parse_unicode_range("U+2000-206F").unwrap());
    let wide = web_face(LIBERATION_SERIF, "Inter", 400);
    let mut registry = FontRegistry::new();
    registry.add(narrow);
    registry.add(wide);
    // A page that runs on `A` must not have `resolve_glyph` hand it the narrow face.
    let picked = registry.resolve_glyph(&["Inter"], 400, false, 'A').unwrap();
    assert_eq!(picked.data().len(), LIBERATION_SERIF.len());
}

#[test]
fn registry_add_allows_two_web_faces_of_the_same_family_with_different_ranges() {
    // Regression gate for the Next.js / Google Fonts split: every face is retained
    // (nothing deduplicates by `(family, weight, italic)`) and each is addressable
    // by range. Before the fix the registry accepted them too — but once we route
    // per range, the retained count must still be two.
    let a =
        web_face(INTER, "Inter", 400).with_unicode_range(parse_unicode_range("U+00-FF").unwrap());
    let b = web_face(LIBERATION_SERIF, "Inter", 400)
        .with_unicode_range(parse_unicode_range("U+0100-02BA").unwrap());
    let mut registry = FontRegistry::new();
    registry.add(a);
    registry.add(b);
    // 'A' (U+0041) lives in a's basic range, 'ā' (U+0101) lives in b's extended range.
    let picked_a = registry.resolve_glyph(&["Inter"], 400, false, 'A').unwrap();
    let picked_b = registry.resolve_glyph(&["Inter"], 400, false, 'ā').unwrap();
    assert_eq!(picked_a.data().len(), INTER.len());
    assert_eq!(picked_b.data().len(), LIBERATION_SERIF.len());
}

// A direct check that the declared `Vec<RangeInclusive<u32>>` lives on the face
// and is reflected in a public getter. This freezes the API shape surf and other
// embedders must call.
#[test]
fn unicode_range_is_a_public_getter_on_face() {
    let ranges: Vec<RangeInclusive<u32>> = vec![0x00..=0x7F, 0x100..=0x2BA];
    let face = web_face(INTER, "Inter", 400).with_unicode_range(ranges.clone());
    assert_eq!(
        face.unicode_range().collect::<Vec<_>>(),
        [&ranges[0], &ranges[1]]
    );
    let unranged = web_face(INTER, "Inter", 400);
    assert_eq!(unranged.unicode_range().count(), 0);
}

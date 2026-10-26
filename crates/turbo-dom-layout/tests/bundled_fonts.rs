//! `bundled-fonts` feature (§4.4): the core embeds OFL faces so a document
//! renders with ZERO caller-supplied fonts. These tests drive the full pipeline
//! with a fresh `FontRegistry::new()` (no `.add`) and assert real glyphs are laid
//! out and the bundled face is subset + embedded in the PDF.
//!
//! The whole file is gated on the feature: with `--no-default-features` there are
//! no bundled faces, so a registry with no caller fonts can't render — exactly
//! the no-bundled behaviour the other suites exercise.
//!
//! Split at the S1 relocation: the cases that need TKind directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/bundled_fonts.rs`.
#![cfg(feature = "bundled-fonts")]

use turbo_dom_layout::{FontFace, FontRegistry};

/// Every embedded bundled face parses and is tagged with a non-zero up-em.
#[test]
fn bundled_set_loads_all_faces() {
    let reg = FontRegistry::new();
    // 4 (Inter) + 4 (Roboto) + 4 (Liberation Serif) + 4 (PT Serif)
    // + 2 (Fira Code) + 4 (IBM Plex Mono) = 22 faces.
    assert!(!reg.is_empty());
    // Each generic family resolves to a real face at regular and bold.
    for fam in ["sans-serif", "serif", "monospace"] {
        let regular = reg.select(&[fam], 400, false);
        assert!(regular.is_some(), "{fam} regular missing");
        let bold = reg.select(&[fam], 700, false);
        assert!(bold.is_some(), "{fam} bold missing");
        assert!(regular.unwrap().units_per_em() > 0);
    }
}

/// `font-family: sans-serif` (the UA body default) selects Inter; bold/italic
/// pick the matching Inter face; the bundled secondary (Roboto) is reachable.
#[test]
fn generic_families_map_to_bundled_primaries() {
    let reg = FontRegistry::new();
    assert_eq!(
        reg.select(&["sans-serif"], 400, false).unwrap().family(),
        "Inter"
    );
    assert_eq!(
        reg.select(&["sans-serif"], 700, true).unwrap().family(),
        "Inter"
    );
    assert!(reg.select(&["sans-serif"], 700, true).unwrap().is_italic());
    assert_eq!(
        reg.select(&["serif"], 400, false).unwrap().family(),
        "Liberation Serif"
    );
    assert_eq!(
        reg.select(&["monospace"], 400, false).unwrap().family(),
        "Fira Code"
    );
    // A name that matches no bundled family still falls back to *some* face.
    assert!(reg.select(&["Nonexistent Font"], 400, false).is_some());
}

/// A caller-supplied face takes precedence over the bundled set for the same
/// family request (bundled faces are fallbacks, not overrides).
#[test]
fn caller_face_overrides_bundled_for_same_family() {
    let go_bytes = std::fs::read(format!(
        "{}/assets/fonts/Go-Regular.ttf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("Go fixture");
    let mut reg = FontRegistry::new();
    reg.add(FontFace::from_bytes(go_bytes, "Go", 400, false).expect("parse Go"));
    // The caller face is selected for its own family.
    assert_eq!(reg.select(&["Go"], 400, false).unwrap().family(), "Go");
    // It is also first in the fallback order, so an unknown family falls back to
    // the caller's face, not a bundled one.
    assert_eq!(
        reg.select(&["Totally Unknown"], 400, false)
            .unwrap()
            .family(),
        "Go"
    );
    // Generic keywords still resolve to bundled faces (the caller didn't supply
    // a sans-serif), so bundled faces remain available as fallbacks.
    assert_eq!(
        reg.select(&["sans-serif"], 400, false).unwrap().family(),
        "Inter"
    );
}

//! Phase 15 `print-color` feature tests (AC-7.x print colour). Only compiled with
//! `--features print-color`. Asserts the emitter writes DeviceCMYK fills (`k`
//! operator) instead of DeviceRGB (`rg`), and that `cmyk(...)` colours parse.
//!
//! Split at the S1 relocation: the cases that need TKind directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/print_color.rs`.

#![cfg(feature = "print-color")]

use turbo_dom_layout::layout::value::{parse_color, Rgba};

#[test]
fn cmyk_function_parses_percentages_and_fractions() {
    // Pure cyan ink: cmyk(100%,0,0,0) -> rgb(0,255,255).
    assert_eq!(
        parse_color("cmyk(100%, 0, 0, 0)"),
        Some(Rgba::new(0, 255, 255, 255))
    );
    // Fractional form, key ink only.
    assert_eq!(
        parse_color("cmyk(0, 0, 0, 1)"),
        Some(Rgba::new(0, 0, 0, 255))
    );
    // Slash-separated, magenta.
    assert_eq!(
        parse_color("cmyk(0 / 1 / 0 / 0)"),
        Some(Rgba::new(255, 0, 255, 255))
    );
    // No ink at all -> white.
    assert_eq!(
        parse_color("cmyk(0,0,0,0)"),
        Some(Rgba::new(255, 255, 255, 255))
    );
}

#[test]
fn cmyk_function_rejects_malformed_input() {
    assert_eq!(parse_color("cmyk(1, 2, 3)"), None); // too few components
    assert_eq!(parse_color("cmyk(1, 2, 3, x)"), None); // non-numeric
    assert_eq!(parse_color("cmyk 1,0,0,0"), None); // no parens
}

#[test]
fn cmyk_components_clamp_out_of_range() {
    // Over-range components clamp to [0,1]: 200% -> 1.0 (full ink).
    assert_eq!(
        parse_color("cmyk(200%, 0, 0, 0)"),
        Some(Rgba::new(0, 255, 255, 255))
    );
    // Negative key clamps to 0 (no key ink).
    assert_eq!(
        parse_color("cmyk(0, 0, 0, -1)"),
        Some(Rgba::new(255, 255, 255, 255))
    );
}

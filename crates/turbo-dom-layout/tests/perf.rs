//! Hot-path instrumentation tests (Phase 13). Compiled only with the `perf`
//! feature; run with `cargo test -p turbo-dom-layout --features perf`. They assert
//! the `FontFace` caches actually collapse the hot paths (one parse per distinct
//! char, one shape per distinct run) rather than per occurrence.
//!
//! Split at the S1 relocation: the cases that need TKind directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/perf.rs`.
#![cfg(feature = "perf")]

mod common;

use turbo_dom_layout::perf;

#[test]
fn a_face_parses_the_font_once() {
    perf::reset();
    let f = common::evolventa();
    // The font is parsed once at construction; lookups reuse the cached face.
    f.glyph_index('A');
    f.glyph_index('B');
    assert_eq!(perf::count("font.face.build"), 1);
}

#[test]
fn shaping_runs_once_per_distinct_run() {
    let f = common::evolventa();
    perf::reset();
    f.shape("Hello");
    f.measure("Hello", 16.0, 0.0); // re-uses the cached shaping
    f.shape("World");
    assert_eq!(perf::count("font.shape"), 2); // not 3
}

#[test]
fn snapshot_exposes_named_counters() {
    let f = common::evolventa();
    perf::reset();
    f.shape("x");
    let snap = perf::snapshot();
    assert_eq!(snap.get("font.shape").copied(), Some(1));
}

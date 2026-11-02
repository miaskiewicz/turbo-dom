//! Hot-path instrumentation tests (Phase 13). Compiled only with the `perf`
//! feature; run with `cargo test -p turbo-dom-layout --features perf`. They assert
//! the `FontFace` caches actually collapse the hot paths (one parse per distinct
//! char, one shape per distinct run) rather than per occurrence.
//!
//! Split at the S1 relocation: the cases that need `TKind` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/perf.rs`.
#![cfg(feature = "perf")]

mod common;

use turbo_dom_layout::perf;

#[test]
fn a_face_parses_the_font_once() {
    perf::reset();
    let f = common::evolventa();
    // The font is parsed once at construction; lookups reuse the cached face.
    let _ = f.glyph_index('A');
    let _ = f.glyph_index('B');
    assert_eq!(perf::count("font.face.build"), 1);
}

#[test]
fn shaping_runs_once_per_distinct_run() {
    let f = common::evolventa();
    perf::reset();
    let _ = f.shape("Hello");
    let _ = f.measure("Hello", 16.0, 0.0); // re-uses the cached shaping
    let _ = f.shape("World");
    assert_eq!(perf::count("font.shape"), 2); // not 3
}

#[test]
fn snapshot_exposes_named_counters() {
    let f = common::evolventa();
    perf::reset();
    let _ = f.shape("x");
    let snap = perf::snapshot();
    assert_eq!(snap.get("font.shape").copied(), Some(1));
}

/// `<div>` nested `depth` deep around one word.
fn nested_divs(depth: usize) -> String {
    format!(
        "<!doctype html><body style='margin:0'>{}a{}</body>",
        "<div>".repeat(depth),
        "</div>".repeat(depth)
    )
}

/// Counts on a thread with a large stack: layout recursion is as deep as the document.
fn block_resolves(depth: usize) -> u64 {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || block_resolves_on_thread(depth))
        .unwrap()
        .join()
        .unwrap()
}

fn block_resolves_on_thread(depth: usize) -> u64 {
    let tree = turbo_dom_layout::parse_html_tree(&nested_divs(depth));
    perf::reset();
    let _ = turbo_dom_layout::layout_boxes_tree(
        &tree,
        "",
        400.0,
        300.0,
        &turbo_dom_layout::text::FontRegistry::new(),
        &turbo_dom_layout::HtmlOptions::default(),
        &mut turbo_dom_layout::Diagnostics::default(),
    );
    perf::count("layout.block.resolve")
}

#[test]
fn nested_block_style_resolves_grow_linearly() {
    // miaskiewicz/turbo-dom#202: margin collapsing re-walked the first/last-child chain
    // at every level, so doubling the depth quadrupled the resolves. Linear is ~2x.
    let (small, large) = (block_resolves(200), block_resolves(400));
    assert!(small > 0);
    assert!(
        large * 10 < small * 25,
        "resolves {small} at d=200 vs {large} at d=400"
    );
}

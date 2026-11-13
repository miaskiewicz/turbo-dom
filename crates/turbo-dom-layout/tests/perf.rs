//! Hot-path instrumentation tests (Phase 13). Compiled only with the `perf`
//! feature; run with `cargo test -p turbo-dom-layout --features perf`. They assert
//! the `FontFace` caches actually collapse the hot paths (one parse per distinct
//! char, one shape per distinct run) rather than per occurrence.
//!
//! Split at the S1 relocation: the cases that need `TKind` directives, templating or the
//! PDF emitter stay in turbo-html2pdf-core's `tests/perf.rs`.
#![cfg(feature = "perf")]

mod common;

use std::fmt::Write as _;

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

/// `depth` flex containers nested inside each other, alternating `writing-mode`
/// (horizontal-tb / vertical-rl), around one word.
fn alternating_writing_modes(depth: usize) -> String {
    let mut html = String::from("<!doctype html><body style='margin:0'>");
    for level in 0..depth {
        let mode = if level % 2 == 0 {
            "vertical-rl"
        } else {
            "horizontal-tb"
        };
        write!(html, "<div style='display:flex;writing-mode:{mode}'>").unwrap();
    }
    html.push_str("word");
    html.push_str(&"</div>".repeat(depth));
    html
}

/// The writing-mode root layouts and orthogonal extents laying out
/// [`alternating_writing_modes`] (on a large stack: layout recursion is document deep).
fn writing_root_work(depth: usize) -> (u64, u64) {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || {
            let tree = turbo_dom_layout::parse_html_tree(&alternating_writing_modes(depth));
            perf::reset();
            let _ = turbo_dom_layout::layout_boxes_tree(
                &tree,
                "",
                800.0,
                600.0,
                &turbo_dom_layout::test_support::fixture_fonts(),
                &turbo_dom_layout::HtmlOptions::default(),
                &mut turbo_dom_layout::Diagnostics::default(),
            );
            (
                perf::count("layout.writing.root"),
                perf::count("layout.writing.extent"),
            )
        })
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn nested_alternating_writing_modes_grow_polynomially() {
    // R2 review #4: each orthogonal flex level measured its item on a relabelled copy
    // with every cache dropped and laid it out again, so the work doubled per level and
    // about 25 levels hung. Now an orthogonal extent is measured a constant number of
    // times per level (linear: doubling the depth at most triples them) and a writing
    // root is laid out once per enclosing layout (quadratic: 24 levels stay under
    // 5 * 24^2 root layouts, doubling the depth from 12 grows them less than 6x; the
    // exponential growth was about 2^12 = 4096x).
    let (small_roots, small_extents) = writing_root_work(12);
    let (large_roots, large_extents) = writing_root_work(24);
    assert!(small_roots > 0 && small_extents > 0);
    assert!(
        large_roots <= small_roots * 6
            && large_roots <= 5 * 24 * 24
            && large_extents <= small_extents * 3,
        "roots {small_roots} -> {large_roots}, extents {small_extents} -> {large_extents}"
    );
}

/// How many words the `box-decoration-break: clone` bookkeeping scans laying out one
/// `words`-word line (the box 1e6px wide) ended by a forced break, inside a cloned
/// `<span>` or not. The bookkeeping runs where a line ends (a wrap or a `<br>`).
fn clone_scans(words: usize, cloned: bool) -> u64 {
    let text = format!("{}<br>", "ab ".repeat(words));
    let body = if cloned {
        format!("<span style='box-decoration-break:clone;padding:0 3px'>{text}</span>")
    } else {
        text
    };
    let html = format!("<!doctype html><body style='margin:0;width:1000000px'>{body}</body>");
    let tree = turbo_dom_layout::parse_html_tree(&html);
    perf::reset();
    let _ = turbo_dom_layout::layout_boxes_tree(
        &tree,
        "",
        1_000_000.0,
        300.0,
        &turbo_dom_layout::test_support::fixture_fonts(),
        &turbo_dom_layout::HtmlOptions::default(),
        &mut turbo_dom_layout::Diagnostics::default(),
    );
    perf::count("inline.clone_scan")
}

#[test]
fn clone_bookkeeping_grows_linearly_with_a_line() {
    // R2 review #6: every word's wrap check re-scanned the whole line for open cloned
    // boxes, so a line's words cost O(words²) even with no `clone` box. Linear is ~2x.
    for cloned in [false, true] {
        let (small, large) = (clone_scans(1000, cloned), clone_scans(2000, cloned));
        assert!(
            large * 10 <= small * 25 + 10,
            "cloned {cloned}: scans {small} at 1000 words vs {large} at 2000"
        );
        if cloned {
            assert!(small >= 1000, "a cloned line scans its words: {small}");
        } else {
            assert_eq!(small, 0, "no clone box, nothing to scan");
        }
    }
}

#[test]
fn frequently_reused_labels_are_shaped_once_during_unique_row_churn() {
    let face = common::evolventa();
    perf::reset();
    for index in 0..512 {
        std::hint::black_box(face.shape("repeated invoice label"));
        std::hint::black_box(face.shape(&format!("unique row {index}")));
    }
    assert_eq!(perf::count("font.shape"), 513);
}

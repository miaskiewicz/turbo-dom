//! Missing actual run data must not turn a line box into fabricated inline geometry.
use crate::cssom_view::inline;
use crate::live::testing;

#[test]
fn glyphless_partial_run_does_not_fabricate_inline_placement() {
    let (tree, mut snapshot, styles) = testing::layout_with_fonts(
        "<!doctype html><body style='font-family:monospace;font-size:16px'><span id=target>abc</span></body>",
        "", (400.0, 200.0), Some(1.0), |_| {}, &crate::test_support::fixture_fonts(),
    );
    let id = testing::id(&tree, "#target");
    let complete = inline::geometry(&snapshot, &styles, id).unwrap();
    assert_eq!(complete.rects.len(), 1);
    assert!(complete.bounds.width > 0.0);
    let indices: Vec<_> = snapshot
        .text_runs
        .iter()
        .enumerate()
        .filter(|(_, run)| run.source_ancestors.contains(&id))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        indices.len(),
        1,
        "fixture must have one real native font/run"
    );
    assert!(!snapshot.text_runs[indices[0]].glyphs.is_empty());
    // Only a test-local imported run loses its actual captured glyphs. Its line
    // fragment and source identity remain, but cannot supply an inline content box.
    snapshot.text_runs[indices[0]].glyphs.clear();
    assert_eq!(
        inline::geometry(&snapshot, &styles, id).unwrap_err(),
        "native empty inline placement is pending"
    );
}

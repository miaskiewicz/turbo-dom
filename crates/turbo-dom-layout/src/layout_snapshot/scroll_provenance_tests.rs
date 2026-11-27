//! Diagnostic contracts for imported or incomplete geometry, not new DOM bugs.
use super::{GeometryKind, LayoutSnapshot};
use crate::cssom_view::{metrics, scroll};
use crate::live::testing;

#[test]
fn scroll_rejects_an_element_source_without_border_provenance() {
    let (tree, snapshot, styles) = testing::layout_with_fonts(
        "<!doctype html><div id=complete style='width:30px;height:12px'></div><span id=empty></span><span id=text>abc</span>",
        "", (200.0, 100.0), None, |_| {}, &crate::test_support::fixture_fonts(),
    );
    let complete =
        scroll::scroll_metrics(&snapshot, &styles, testing::id(&tree, "#complete")).unwrap();
    assert_eq!((complete.width, complete.height), (30, 12));
    for selector in ["#empty", "#text"] {
        let id = testing::id(&tree, selector);
        let source = snapshot.source(id).unwrap();
        assert!(!source.excluded_from_layout);
        assert!(source
            .fragment_indices
            .iter()
            .all(|&index| snapshot.fragments()[index].kind != GeometryKind::BorderBox));
        assert_eq!(
            scroll::scroll_metrics(&snapshot, &styles, id).unwrap_err(),
            "native scroll requires inline border provenance"
        );
    }
}

fn border_index(snapshot: &LayoutSnapshot, id: super::SourceId) -> usize {
    snapshot
        .source(id)
        .unwrap()
        .fragment_indices
        .iter()
        .copied()
        .find(|&index| snapshot.fragments()[index].kind == GeometryKind::BorderBox)
        .unwrap()
}

#[test]
fn partial_used_box_provenance_is_not_reconstructed_from_authored_style() {
    let (tree, mut snapshot, styles) = testing::layout(
        "<!doctype html><div id=scroller style='width:20px;height:10px;overflow:auto'><div style='width:40px;height:30px'></div></div>",
        "", (200.0, 100.0), Some(1.0),
    );
    let id = testing::id(&tree, "#scroller");
    let complete = scroll::scroll_metrics(&snapshot, &styles, id).unwrap();
    assert_eq!((complete.width, complete.height), (40, 30));
    let index = border_index(&snapshot, id);
    assert!(snapshot.fragments[index].used_box.take().is_some());
    // Model an imported partial geometry record. A border and authored width are
    // insufficient substitutes for the actual used metrics; no production data is changed.
    assert_eq!(styles[&id].style.get("width"), Some("20px"));
    assert_eq!(
        scroll::scroll_metrics(&snapshot, &styles, id).unwrap_err(),
        "native scroll used box missing"
    );
    assert_eq!(
        metrics::client_metrics(&snapshot, id).unwrap_err(),
        "native live layout used box metrics are missing"
    );
}

#[test]
fn generic_imported_geometry_refuses_nonfinite_scroll_projection() {
    // The generic snapshot API deliberately does not promise native-text finite
    // provenance. Finite CSS coefficients can overflow its determinant; scroll
    // must diagnose this rather than manufacture zero or invert a nonfinite value.
    let (tree, snapshot, styles) = testing::layout(
        "<!doctype html><div id=scroller style='width:20px;height:10px;overflow:auto;transform:scale(1e200);transform-origin:0 0'></div>",
        "", (200.0, 100.0), None,
    );
    let id = testing::id(&tree, "#scroller");
    let matrix = snapshot
        .native_world_transform(border_index(&snapshot, id))
        .unwrap();
    assert!(matrix.iter().all(|value| value.is_finite()));
    assert!(!(matrix[0] * matrix[3] - matrix[1] * matrix[2]).is_finite());
    assert_eq!(
        scroll::scroll_metrics(&snapshot, &styles, id).unwrap_err(),
        "native scroll degenerate-container relative transform provenance pending"
    );
}

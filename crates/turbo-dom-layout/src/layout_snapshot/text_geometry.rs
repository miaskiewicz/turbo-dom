//! Advance/font boxes from real glyph clusters, not line-container rectangles.
use super::{geometry, LayoutSnapshot, Point, Rect, SnapshotTextRun, SourceId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One actual shaper cluster's native advance/font box and composed transform.
/// It may span several UTF-16 units; this is not an interpolated caret position.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceClusterBox {
    pub fragment_index: usize,
    pub source_id: Option<SourceId>,
    pub source_ancestors: Arc<[SourceId]>,
    pub right_to_left: bool,
    /// Actual CSS-collapse provenance; a rendered first space can still advance.
    pub collapsed_whitespace: bool,
    pub utf16_start: usize,
    pub utf16_end: usize,
    /// The advance/font box in the line's own frame: physical, unless `vertical`.
    pub rect: Rect,
    /// The run lies in a vertical line: `rect` is in its frame, which [`turn`]s into
    /// the fragment; `quad` and `transformed_bounds` are physical.
    pub vertical: bool,
    pub quad: [Point; 4],
    pub transformed_bounds: Rect,
}

/// A rect in a vertical line's own frame (anchored at its fragment's top-left `frame`,
/// `x` along the line, `y` down from its line-over edge) in physical coordinates: the
/// frame turns a quarter clockwise into the fragment, like its glyphs paint
/// (miaskiewicz/turbo-dom#239).
pub(crate) fn turn(rect: Rect, frame: Rect) -> Rect {
    Rect {
        x: frame.x + frame.width - (rect.y - frame.y) - rect.height,
        y: frame.y + (rect.x - frame.x),
        width: rect.height,
        height: rect.width,
    }
}

/// The inverse of [`turn`] for a point.
pub(crate) fn unturn(point: Point, frame: Rect) -> Point {
    Point {
        x: frame.x + (point.y - frame.y),
        y: frame.y + frame.width - (point.x - frame.x),
    }
}

/// Whether fragment `index` holds a vertical text run.
pub(crate) fn vertical_fragment(snapshot: &LayoutSnapshot, index: usize) -> bool {
    snapshot
        .text_runs()
        .iter()
        .any(|run| run.vertical && run.fragment_index == index)
}

pub(crate) fn cluster_boxes(snapshot: &LayoutSnapshot) -> Vec<SourceClusterBox> {
    collect_boxes(snapshot, snapshot.text_runs().iter())
}

pub(crate) fn source_cluster_boxes(
    snapshot: &LayoutSnapshot,
    source: SourceId,
) -> Vec<SourceClusterBox> {
    collect_boxes(
        snapshot,
        snapshot
            .text_runs()
            .iter()
            .filter(|run| run.source_id == Some(source)),
    )
}

fn collect_boxes<'a>(
    snapshot: &LayoutSnapshot,
    runs: impl Iterator<Item = &'a SnapshotTextRun>,
) -> Vec<SourceClusterBox> {
    let mut boxes = Vec::new();
    for run in runs {
        let fragment = &snapshot.fragments()[run.fragment_index];
        let ancestry: Arc<[SourceId]> = run.source_ancestors.clone().into();
        let mut groups = BTreeMap::<(usize, usize), (f32, f32, bool, bool)>::new();
        for glyph in &run.glyphs {
            let source = glyph.source;
            let begin = source.pen_x.min(source.pen_x + source.advance);
            let end = source.pen_x.max(source.pen_x + source.advance);
            let edges = groups
                .entry((source.utf16_start, source.utf16_end))
                .or_insert((
                    begin,
                    end,
                    source.right_to_left,
                    source.collapsed_whitespace,
                ));
            edges.0 = edges.0.min(begin);
            edges.1 = edges.1.max(end);
            edges.3 &= source.collapsed_whitespace;
        }
        for ((start, end), (left, right, right_to_left, collapsed_whitespace)) in groups {
            let font = run.font_box;
            let rect = Rect {
                x: fragment.rect.x + left,
                y: fragment.rect.y + font.baseline - font.ascent,
                width: right - left,
                height: font.ascent + font.descent,
            };
            let physical = if run.vertical {
                turn(rect, fragment.rect)
            } else {
                rect
            };
            let quad = geometry::rect_quad(physical).map(|point| {
                geometry::transform(point, snapshot.native_transforms[fragment.index])
            });
            boxes.push(SourceClusterBox {
                right_to_left,
                collapsed_whitespace,
                fragment_index: fragment.index,
                source_id: run.source_id,
                source_ancestors: Arc::clone(&ancestry),
                utf16_start: start,
                utf16_end: end,
                rect,
                vertical: run.vertical,
                quad,
                transformed_bounds: geometry::bounds(quad),
            });
        }
    }
    boxes
}

//! Caret hits select real cluster edges; they never subdivide glyph advances.
use super::{LayoutSnapshot, Point, Rect, SourceClusterBox, SourceId, TextGeometryError};

pub(crate) fn offset(
    snapshot: &LayoutSnapshot,
    id: SourceId,
    point: Point,
) -> Result<usize, TextGeometryError> {
    validate_point(point)?;
    if !snapshot
        .text_sources()
        .iter()
        .any(|source| source.source_id == id)
    {
        return Err(TextGeometryError::MissingSource(id));
    }
    let cells = snapshot.text_cluster_boxes_for_source(id);
    let mut best = None;
    for (index, cell) in cells.iter().enumerate() {
        let fragment = &snapshot.fragments()[cell.fragment_index];
        let matrix = snapshot.native_hit_transforms[fragment.index];
        let local = local_point(point, matrix, snapshot.html_layout_scale)?;
        let distance = vertical_distance(local.y, cell.rect) * matrix[2].hypot(matrix[3]) as f32;
        let mut edge = nearest_edge(local.x, cell);
        edge.0 *= matrix[0].hypot(matrix[1]) as f32;
        consider(
            &mut best,
            distance,
            fragment.rect.y,
            cell.fragment_index,
            edge,
            index,
        );
    }
    best.map(|(_, _, _, (_, offset), index)| {
        bidi_adjusted(&cells, index, offset).unwrap_or_else(|| {
            super::text_caret_edges::canonicalize(snapshot, &cells, index, offset)
        })
    })
    .ok_or(TextGeometryError::UnresolvedSource)
}

/// Blink's bidi hit-test adjustment (Chrome 156, miaskiewicz/turbo-dom#238) for a caret
/// at an edge of a cell above its paragraph's level: next to a deeper cell it takes
/// that cell's offset at the edge; a cell one level above the paragraph next to a
/// paragraph-level cell (or nothing on this line) moves to the other logical end of its
/// run (the visually contiguous cells at its level or deeper). `None` leaves the caret
/// as hit.
fn bidi_adjusted(cells: &[SourceClusterBox], index: usize, offset: usize) -> Option<usize> {
    let cell = &cells[index];
    if cell.bidi_level <= cell.paragraph_level {
        return None;
    }
    let line: Vec<&SourceClusterBox> = cells
        .iter()
        .filter(|other| other.rect.y == cell.rect.y && other.rect.width > 0.0)
        .collect();
    let at_left = offset == edge_offset(cell, true);
    let touching = |from: &SourceClusterBox, left: bool| {
        let edge = if left {
            from.rect.x
        } else {
            from.rect.x + from.rect.width
        };
        line.iter().copied().find(|other| {
            let near = if left {
                other.rect.x + other.rect.width
            } else {
                other.rect.x
            };
            (near - edge).abs() < 0.01 && !std::ptr::eq(*other, from)
        })
    };
    let neighbour = touching(cell, at_left);
    if let Some(deeper) = neighbour.filter(|n| n.bidi_level > cell.bidi_level) {
        return Some(edge_offset(deeper, !at_left));
    }
    let shallow = neighbour.is_none_or(|n| n.bidi_level <= cell.paragraph_level);
    if !shallow || cell.bidi_level != cell.paragraph_level + 1 {
        return None;
    }
    let (mut start, mut end) = (cell.utf16_start, cell.utf16_end);
    for left in [true, false] {
        let mut far = cell;
        while let Some(next) = touching(far, left).filter(|n| n.bidi_level >= cell.bidi_level) {
            start = start.min(next.utf16_start);
            end = end.max(next.utf16_end);
            far = next;
        }
    }
    Some(if offset == start { end } else { start })
}

/// The caret offset at a cell's left (`left`) or right edge.
fn edge_offset(cell: &SourceClusterBox, left: bool) -> usize {
    if left == cell.right_to_left {
        cell.utf16_end
    } else {
        cell.utf16_start
    }
}

fn validate_point(point: Point) -> Result<(), TextGeometryError> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(TextGeometryError::InvalidPoint)
    }
}

fn local_point(
    point: Point,
    matrix: [f64; 6],
    scale: Option<f32>,
) -> Result<Point, TextGeometryError> {
    let [a, b, c, d, e, f] = matrix;
    let determinant = a * d - b * c;
    if determinant == 0.0 || !determinant.is_finite() {
        return Err(TextGeometryError::UnsupportedCaretTransform);
    }
    let (x, y) = (f64::from(point.x) - e, f64::from(point.y) - f);
    let local = Point {
        x: ((d * x - c * y) / determinant) as f32,
        y: ((a * y - b * x) / determinant) as f32,
    };
    validate_point(local)?;
    let local = physical_hit_point(local, scale);
    validate_point(local)?;
    Ok(local)
}

// A transformed HitTestLocation preserves the original PointF through inverse
// projection, then floors the resulting local PhysicalOffset.
fn physical_hit_point(point: Point, scale: Option<f32>) -> Point {
    match scale {
        Some(scale) => {
            let units = 64.0 * scale;
            Point {
                x: (point.x * units).floor() / units,
                y: (point.y * units).floor() / units,
            }
        }
        None => point,
    }
}

fn vertical_distance(y: f32, rect: Rect) -> f32 {
    (rect.y - y).max(y - rect.y - rect.height).max(0.0)
}

fn nearest_edge(x: f32, cell: &SourceClusterBox) -> (f32, usize) {
    let left = (x - cell.rect.x).abs();
    let right = (x - cell.rect.x - cell.rect.width).abs();
    let (start, end) = if cell.right_to_left {
        (cell.utf16_end, cell.utf16_start)
    } else {
        (cell.utf16_start, cell.utf16_end)
    };
    // At a shared edge the cell holding the point wins (a bidi boundary's two cells
    // name different offsets; miaskiewicz/turbo-dom#238).
    // A collapsed (zero-width) cell never beats a real cell at the same edge: Chrome
    // hits the glyph there (a space collapsed after an empty inline box, Chrome 156).
    let outside = if cell.rect.width == 0.0 {
        2e-4
    } else if (cell.rect.x..=cell.rect.x + cell.rect.width).contains(&x) {
        0.0
    } else {
        1e-4
    };
    if left <= right {
        (left + outside, start)
    } else {
        (right + outside, end)
    }
}

type Candidate = (f32, f32, usize, (f32, usize), usize);

fn consider(
    best: &mut Option<Candidate>,
    vertical: f32,
    line_top: f32,
    fragment: usize,
    edge: (f32, usize),
    index: usize,
) {
    let candidate = (vertical, line_top, fragment, edge, index);
    if best.is_none_or(|old| preference(candidate) < preference(old)) {
        *best = Some(candidate);
    }
}

fn preference(candidate: Candidate) -> (f32, f32, f32, usize) {
    // Overlapping font boxes hit the first actual line; equal gaps between font
    // boxes select the following line. Runs on that line compete by real edges.
    let line = if candidate.0 == 0.0 {
        candidate.1
    } else {
        -candidate.1
    };
    (candidate.0, line, candidate.3 .0, candidate.2)
}

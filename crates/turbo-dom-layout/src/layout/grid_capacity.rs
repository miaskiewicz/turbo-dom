//! Native grid budget, independent of Chromium's much larger implementation
//! limit. CSS Grid §5.4 permits clipping areas to a bounded implicit grid.
use taffy::{GridAutoFlow, GridPlacement, Line, Position, Style};

pub(super) const MAX_TRACKS: usize = 1000;

pub(super) fn decimal(text: &str, maximum: usize) -> Option<usize> {
    let text = text.strip_prefix('+').unwrap_or(text);
    if text.is_empty() {
        return None;
    }
    let mut result = 0_usize;
    for byte in text.bytes() {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        result = result
            .saturating_mul(10)
            .saturating_add(usize::from(digit))
            .min(maximum);
    }
    Some(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    start: i32,
    end: i32,
}

#[derive(Clone, Copy)]
struct Window {
    low: i32,
    high: i32,
    explicit: i32,
}
impl Window {
    fn new(explicit: usize, minimum: i32) -> Self {
        let explicit = explicit.min(MAX_TRACKS) as i32;
        let low = minimum.min(0).max(explicit - MAX_TRACKS as i32);
        Self {
            low,
            high: low + MAX_TRACKS as i32,
            explicit,
        }
    }
    fn clip(self, range: Range) -> Range {
        if range.end <= self.low {
            return Range {
                start: self.low,
                end: self.low + 1,
            };
        }
        if range.start >= self.high {
            return Range {
                start: self.high - 1,
                end: self.high,
            };
        }
        Range {
            start: range.start.max(self.low),
            end: range.end.min(self.high),
        }
    }
    fn line(self, coordinate: i32) -> GridPlacement {
        let css = if coordinate >= 0 {
            coordinate + 1
        } else {
            coordinate - self.explicit - 1
        };
        taffy::style_helpers::line(css as i16)
    }
    fn placement(self, range: Range) -> Line<GridPlacement> {
        Line {
            start: self.line(range.start),
            end: self.line(range.end),
        }
    }
}
#[derive(Clone, Copy)]
struct Axis {
    fixed: Option<Range>,
    span: i32,
}
#[derive(Clone, Copy)]
struct Request {
    index: usize,
    col: Axis,
    row: Axis,
}

fn coordinate(placement: GridPlacement, explicit: i32) -> Option<i32> {
    match placement {
        GridPlacement::Line(line) if line.as_i16() > 0 => Some(i32::from(line.as_i16()) - 1),
        GridPlacement::Line(line) if line.as_i16() < 0 => {
            Some(explicit + i32::from(line.as_i16()) + 1)
        }
        _ => None,
    }
}
fn span(placement: GridPlacement) -> i32 {
    match placement {
        GridPlacement::Span(n) => i32::from(n).max(1),
        _ => 1,
    }
}
fn raw_range(placement: Line<GridPlacement>, explicit: i32) -> Option<Range> {
    let first = coordinate(placement.start, explicit);
    let last = coordinate(placement.end, explicit);
    match (first, last) {
        (Some(a), Some(b)) if a == b => Some(Range {
            start: a,
            end: a + 1,
        }),
        (Some(a), Some(b)) => Some(Range {
            start: a.min(b),
            end: a.max(b),
        }),
        (Some(a), None) => Some(Range {
            start: a,
            end: a + span(placement.end),
        }),
        (None, Some(b)) => Some(Range {
            start: b - span(placement.start),
            end: b,
        }),
        _ => None,
    }
}
fn axis(placement: Line<GridPlacement>, window: Window) -> Axis {
    Axis {
        fixed: raw_range(placement, window.explicit).map(|r| window.clip(r)),
        span: span(placement.start)
            .max(span(placement.end))
            .min(MAX_TRACKS as i32),
    }
}
fn window<'a>(styles: impl Iterator<Item = &'a Style>, explicit: usize, column: bool) -> Window {
    let explicit = explicit.min(MAX_TRACKS);
    let minimum = styles
        .filter(|s| in_flow(s))
        .filter_map(|s| {
            raw_range(
                if column { s.grid_column } else { s.grid_row },
                explicit as i32,
            )
        })
        .map(|r| r.start)
        .min()
        .unwrap_or(0);
    Window::new(explicit, minimum)
}
fn overflow<'a>(styles: impl Iterator<Item = &'a Style>, window: Window, column: bool) -> bool {
    let mut end = window.explicit;
    let mut automatic = 0_i64;
    for style in styles.filter(|s| in_flow(s)) {
        let placement = if column {
            style.grid_column
        } else {
            style.grid_row
        };
        if let Some(raw) = raw_range(placement, window.explicit) {
            let bounded = window.clip(raw);
            if raw != bounded {
                return true;
            }
            end = end.max(bounded.end);
        } else {
            automatic =
                automatic.saturating_add(i64::from(span(placement.start).max(span(placement.end))));
        }
    }
    i64::from(end) + automatic > i64::from(window.high)
}

fn in_flow(style: &Style) -> bool {
    style.position != Position::Absolute && style.display != taffy::Display::None
}
fn extent(axis: Axis) -> i32 {
    axis.fixed.map_or(axis.span, |range| range.end.max(0))
}
pub(super) fn needed<'a>(
    styles: impl Iterator<Item = &'a Style> + Clone,
    columns: usize,
    rows: usize,
) -> bool {
    let col = window(styles.clone(), columns, true);
    let row = window(styles.clone(), rows, false);
    overflow(styles.clone(), col, true) || overflow(styles, row, false)
}

/// Ordinary placements are left untouched. Only author areas clipped by the UA
/// window, or a conservative overflow estimate, enter the bounded prepass.
pub(super) fn prepare(styles: &mut [Style], columns: usize, rows: usize, flow: GridAutoFlow) {
    if !needed(styles.iter(), columns, rows) {
        return;
    }
    let column_flow = matches!(flow, GridAutoFlow::Column | GridAutoFlow::ColumnDense);
    if column_flow {
        transpose(styles);
        prepare_rows(styles, rows, columns, flow.is_dense());
        transpose(styles);
    } else {
        prepare_rows(styles, columns, rows, flow.is_dense());
    }
}

fn transpose(styles: &mut [Style]) {
    for style in styles {
        std::mem::swap(&mut style.grid_column, &mut style.grid_row);
    }
}

fn prepare_rows(styles: &mut [Style], columns: usize, rows: usize, dense: bool) {
    let col_window = window(styles.iter(), columns, true);
    let row_window = window(styles.iter(), rows, false);
    if !overflow(styles.iter(), col_window, true) && !overflow(styles.iter(), row_window, false) {
        return;
    }
    let requests: Vec<_> = styles
        .iter_mut()
        .enumerate()
        .filter_map(|(index, style)| {
            if !in_flow(style) {
                return None;
            }
            let col = axis(style.grid_column, col_window);
            let row = axis(style.grid_row, row_window);
            clip_author(&mut style.grid_column, col_window);
            clip_author(&mut style.grid_row, row_window);
            Some(Request { index, col, row })
        })
        .collect();
    let mut placer = Placer::new(&requests, col_window, row_window);
    placer.place(&requests, dense);
    for (request, area) in requests.iter().zip(placer.areas) {
        let (col, row) = area.expect("every in-flow grid child receives a bounded area");
        styles[request.index].grid_column = col_window.placement(col);
        styles[request.index].grid_row = row_window.placement(row);
    }
}
fn clip_author(placement: &mut Line<GridPlacement>, window: Window) {
    let a = coordinate(placement.start, window.explicit);
    let b = coordinate(placement.end, window.explicit);
    let raw = match (a, b) {
        (Some(a), Some(b)) => Some(Range {
            start: a.min(b),
            end: a.max(b).max(a.min(b) + 1),
        }),
        (Some(a), None) => Some(Range {
            start: a,
            end: a + span(placement.end),
        }),
        (None, Some(b)) => Some(Range {
            start: b - span(placement.start),
            end: b,
        }),
        _ => None,
    };
    if let Some(raw) = raw {
        if window.clip(raw) != raw {
            *placement = window.placement(window.clip(raw));
        }
    }
}

/// Overflow-only occupancy: 1000 rows of sixteen 64-bit words (128 kB / 125 KiB).
/// Rectangle checks stop at the first occupied word; no global rebuild per item.
struct Occupancy {
    cells: Vec<u64>,
    left: i32,
    top: i32,
}
impl Occupancy {
    const PITCH: usize = MAX_TRACKS.div_ceil(64);
    fn new(left: i32, top: i32) -> Self {
        Self {
            cells: vec![0; MAX_TRACKS * Self::PITCH],
            left,
            top,
        }
    }
    fn mask(col: Range, word: usize) -> u64 {
        // Callers enumerate start/64 through (end-1)/64 for nonempty ranges:
        // the first bit is <64 and the last bit is in 1..=64, so neither shift
        // can reach 64, including exact word boundaries.
        let first = (col.start as usize).saturating_sub(word * 64).min(64);
        let last = (col.end as usize).saturating_sub(word * 64).min(64);
        (u64::MAX << first) & (u64::MAX >> (64 - last))
    }
    fn relative(&self, col: Range) -> Range {
        Range {
            start: col.start - self.left,
            end: col.end - self.left,
        }
    }
    fn record(&mut self, col: Range, row: Range) {
        let col = self.relative(col);
        for y in row.start - self.top..row.end - self.top {
            for word in col.start as usize / 64..=(col.end as usize - 1) / 64 {
                self.cells[y as usize * Self::PITCH + word] |= Self::mask(col, word);
            }
        }
    }
    fn collision(&self, col: Range, row: Range) -> Option<(i32, i32)> {
        let col = self.relative(col);
        for y in row.start - self.top..row.end - self.top {
            for word in col.start as usize / 64..=(col.end as usize - 1) / 64 {
                let bits = self.cells[y as usize * Self::PITCH + word] & Self::mask(col, word);
                if bits != 0 {
                    return Some((
                        (word * 64 + bits.trailing_zeros() as usize) as i32 + self.left,
                        y + self.top,
                    ));
                }
            }
        }
        None
    }
    fn empty(&self, col: Range, row: Range) -> bool {
        self.collision(col, row).is_none()
    }
    fn full(&self, col: Range, row: Range) -> bool {
        let col = self.relative(col);
        for y in row.start - self.top..row.end - self.top {
            for word in col.start as usize / 64..=(col.end as usize - 1) / 64 {
                let mask = Self::mask(col, word);
                if self.cells[y as usize * Self::PITCH + word] & mask != mask {
                    return false;
                }
            }
        }
        true
    }
}
struct Placer {
    occupancy: Occupancy,
    areas: Vec<Option<(Range, Range)>>,
    col: Window,
    row: Window,
    left: i32,
    top: i32,
    end: i32,
    row_cursors: Vec<i32>,
}
impl Placer {
    fn new(requests: &[Request], col: Window, row: Window) -> Self {
        let left = requests
            .iter()
            .filter_map(|r| r.col.fixed)
            .map(|r| r.start)
            .min()
            .unwrap_or(0)
            .min(0);
        let top = requests
            .iter()
            .filter_map(|r| r.row.fixed)
            .map(|r| r.start)
            .min()
            .unwrap_or(0)
            .min(0);
        let end = requests
            .iter()
            .map(|r| extent(r.col))
            .max()
            .unwrap_or(1)
            .max(col.explicit)
            .max(1)
            .min(col.high);
        Self {
            occupancy: Occupancy::new(col.low, row.low),
            areas: vec![None; requests.len()],
            col,
            row,
            left,
            top,
            end,
            row_cursors: vec![left; MAX_TRACKS],
        }
    }
    fn save(&mut self, index: usize, col: Range, row: Range) {
        self.occupancy.record(col, row);
        self.areas[index] = Some((col, row));
        self.end = self.end.max(col.end);
    }
    fn place_fixed(&mut self, requests: &[Request]) {
        for (i, r) in requests.iter().enumerate() {
            if let (Some(col), Some(row)) = (r.col.fixed, r.row.fixed) {
                self.save(i, col, row);
            }
        }
    }
    fn place_fixed_rows(&mut self, requests: &[Request], dense: bool) {
        for (i, r) in requests.iter().enumerate() {
            if r.col.fixed.is_none() {
                if let Some(row) = r.row.fixed {
                    let start = if dense {
                        self.left
                    } else {
                        self.row_cursors[(row.start - self.row.low) as usize]
                    };
                    let col = self.fixed_row(*r, row, start);
                    self.save(i, col, row);
                    for y in row.start..row.end {
                        self.row_cursors[(y - self.row.low) as usize] =
                            self.row_cursors[(y - self.row.low) as usize].max(col.end);
                    }
                }
            }
        }
    }
    fn place(&mut self, requests: &[Request], dense: bool) {
        self.place_fixed(requests);
        self.place_fixed_rows(requests, dense);
        let mut cursor = (self.left, self.top);
        for (i, r) in requests.iter().enumerate() {
            if r.row.fixed.is_some() {
                continue;
            }
            if dense {
                cursor = (self.left, self.top);
            }
            let (col, row) = self.remaining(*r, cursor);
            self.save(i, col, row);
            cursor = (col.end, row.start);
        }
    }
    fn fixed_row(&mut self, r: Request, row: Range, start: i32) -> Range {
        if start >= self.col.high {
            return Range {
                start: self.col.high - 1,
                end: self.col.high,
            };
        }
        let available = Range {
            start: self.left,
            end: self.col.high,
        };
        if !self.occupancy.full(available, row) {
            let mut start = start;
            while start < self.col.high {
                let col = Range {
                    start,
                    end: (start + r.col.span).min(self.col.high),
                };
                match self.occupancy.collision(col, row) {
                    None => return col,
                    Some((x, _)) => start = x + 1,
                }
            }
        }
        Range {
            start: self.col.high - 1,
            end: self.col.high,
        }
    }
    fn remaining(&mut self, r: Request, cursor: (i32, i32)) -> (Range, Range) {
        let mut top = cursor.1;
        if r.col.fixed.is_some_and(|col| col.start < cursor.0) {
            top += 1;
        }
        let available = Range {
            start: self.left,
            end: self.end,
        };
        let rows = Range {
            start: top.min(self.row.high - 1),
            end: self.row.high,
        };
        if top < self.row.high && !self.occupancy.full(available, rows) {
            for y in top..self.row.high {
                let row = Range {
                    start: y,
                    end: (y + r.row.span).min(self.row.high),
                };
                if let Some(col) = r.col.fixed {
                    if self.occupancy.empty(col, row) {
                        return (col, row);
                    }
                } else if let Some(col) =
                    self.auto_column(r, row, if y == top { cursor.0 } else { self.left })
                {
                    return (col, row);
                }
            }
        }
        let col = r.col.fixed.unwrap_or(Range {
            start: self.left,
            end: (self.left + r.col.span).min(self.end),
        });
        (
            col,
            Range {
                start: self.row.high - 1,
                end: self.row.high,
            },
        )
    }
    fn auto_column(&mut self, r: Request, row: Range, start: i32) -> Option<Range> {
        let width = r.col.span.min(self.end - self.left);
        let mut x = start;
        while x <= self.end - width {
            let col = Range {
                start: x,
                end: x + width,
            };
            match self.occupancy.collision(col, row) {
                None => return Some(col),
                Some((hit, _)) => x = hit + 1,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::style_helpers::{line, span as grid_span};
    fn placement(start: GridPlacement, end: GridPlacement) -> Line<GridPlacement> {
        Line { start, end }
    }
    #[test]
    fn numeric_parsing_saturates_without_accepting_invalid_digits() {
        assert_eq!(
            decimal("1844674407370955161599999999", MAX_TRACKS),
            Some(MAX_TRACKS)
        );
        assert_eq!(decimal("+12", MAX_TRACKS), Some(12));
        for invalid in ["", "-1", "12x", " "] {
            assert_eq!(decimal(invalid, MAX_TRACKS), None);
        }
    }
    #[test]
    fn windows_use_only_requested_negative_space() {
        let normal = Window::new(2, 0);
        assert_eq!((normal.low, normal.high), (0, 1000));
        let both = Window::new(2, -400);
        assert_eq!((both.low, both.high), (-400, 600));
        let explicit = Window::new(MAX_TRACKS, -400);
        assert_eq!((explicit.low, explicit.high), (0, 1000));
        assert_eq!(
            explicit.clip(Range {
                start: -30,
                end: -10
            }),
            Range { start: 0, end: 1 }
        );
        assert_eq!(
            explicit.clip(Range {
                start: 990,
                end: 2000
            }),
            Range {
                start: 990,
                end: 1000
            }
        );
        assert_eq!(
            explicit.clip(Range {
                start: 2000,
                end: 3000
            }),
            Range {
                start: 999,
                end: 1000
            }
        );
    }
    #[test]
    fn healthy_auto_rows_leave_every_declaration_unchanged() {
        let mut styles = vec![Style::default(); 501];
        let before = styles.clone();
        assert!(!needed(styles.iter(), 1, 0));
        prepare(&mut styles, 1, 0, GridAutoFlow::Row);
        assert_eq!(styles, before);
        let both = [Style {
            grid_column: placement(line(-403), line(401)),
            ..Style::default()
        }];
        assert!(!needed(both.iter(), 2, 0));
    }
    #[test]
    fn bitset_words_have_exact_exclusive_boundaries() {
        for (start, end) in [(0, 64), (64, 128), (63, 65), (999, 1000)] {
            let mut occupied = Occupancy::new(-400, -500);
            let col = Range {
                start: start - 400,
                end: end - 400,
            };
            let row = Range {
                start: -500,
                end: -499,
            };
            assert!(occupied.empty(col, row));
            occupied.record(col, row);
            assert!(occupied.full(col, row));
            assert!(occupied.empty(
                col,
                Range {
                    start: -499,
                    end: -498
                }
            ));
            assert_eq!(occupied.collision(col, row), Some((start - 400, -500)));
            assert_eq!(occupied.cells.len() * std::mem::size_of::<u64>(), 128_000);
        }
        let window = Window::new(MAX_TRACKS, 0);
        let tracks = (window.high - window.low) as usize;
        assert!((2 * tracks + 1).pow(2) <= 4_004_001);
    }
    #[test]
    fn sparse_and_dense_cursors_preserve_the_first_available_hole() {
        let col = Window {
            low: 0,
            high: 3,
            explicit: 3,
        };
        let row = Window {
            low: 0,
            high: 3,
            explicit: 0,
        };
        let make = |index, columns| Request {
            index,
            col: Axis {
                fixed: None,
                span: columns,
            },
            row: Axis {
                fixed: None,
                span: 1,
            },
        };
        let requests = [make(0, 2), make(1, 2), make(2, 1)];
        let mut sparse = Placer::new(&requests, col, row);
        sparse.place(&requests, false);
        let mut dense = Placer::new(&requests, col, row);
        dense.place(&requests, true);
        assert_eq!(
            sparse.areas[2],
            Some((Range { start: 2, end: 3 }, Range { start: 1, end: 2 }))
        );
        assert_eq!(
            dense.areas[2],
            Some((Range { start: 2, end: 3 }, Range { start: 0, end: 1 }))
        );
    }
    #[test]
    fn signed_and_equal_lines_resolve_before_clipping() {
        let window = Window::new(3, -5);
        let same = axis(placement(line(2), line(2)), window);
        assert_eq!(same.fixed, Some(Range { start: 1, end: 2 }));
        assert_eq!(
            axis(placement(line(-1), grid_span(2)), window).fixed,
            Some(Range { start: 3, end: 5 })
        );
        assert_eq!(
            axis(placement(grid_span(2), line(1)), window).fixed,
            Some(Range { start: -2, end: 0 })
        );
        assert_eq!(super::span(GridPlacement::Auto), 1);
    }
}

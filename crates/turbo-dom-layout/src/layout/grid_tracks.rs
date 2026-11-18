//! Taffy 0.8 resolves percentage preferred widths against the whole grid when
//! computing an item's minimum contribution. CSS treats that cyclic percentage
//! as indefinite until tracks have been sized. Resolve tracks without it, then
//! retain those tracks while restoring area-relative sizes for final placement.
use super::{
    line, solve, Ctx, Dimension, Display, Fill, FromLength, GridTrackRepetition, JustifyContent,
    LayoutBox, LengthPercentage, Line, Position, Style, TaffyId, TaffyTree, TrackSizingFunction,
};
use taffy::DetailedGridInfo;
use taffy::DetailedLayoutInfo;

pub(super) fn solve_grid(
    tree: &mut TaffyTree<usize>,
    root: TaffyId,
    leaves: &[TaffyId],
    items: &[LayoutBox],
    dimensions: (f32, f32),
    fill: Fill,
    ctx: &Ctx,
) {
    if !leaves
        .iter()
        .any(|id| needs_intrinsic_style(tree.style(*id).unwrap()))
    {
        solve(tree, root, items, dimensions, fill, ctx);
        return;
    }
    let originals: Vec<_> = leaves
        .iter()
        .map(|id| tree.style(*id).unwrap().clone())
        .collect();
    // The unchanged styles passed the same needs_intrinsic_style predicate,
    // so at least one declaration necessarily becomes deferred in this pass.
    defer_intrinsic_styles(tree, leaves, &originals);
    solve(tree, root, items, dimensions, fill, ctx);
    let DetailedLayoutInfo::Grid(info) = tree.detailed_layout_info(root) else {
        unreachable!("a solved grid records its tracks");
    };
    let info = info.clone();
    let mut root_style = tree.style(root).unwrap().clone();
    freeze_columns(&mut root_style, &info, dimensions.1);
    tree.set_style(root, root_style).unwrap();
    restore_items(tree, leaves, originals, &info);
    solve(tree, root, items, dimensions, fill, ctx);
}

fn defer_intrinsic_styles(tree: &mut TaffyTree<usize>, leaves: &[TaffyId], styles: &[Style]) {
    for (id, original) in leaves.iter().zip(styles) {
        if let Some(intrinsic) = intrinsic_style(original) {
            tree.set_style(*id, intrinsic).unwrap();
        }
    }
}

fn intrinsic_style(original: &Style) -> Option<Style> {
    // Absolutely positioned children do not create tracks. Taffy includes their
    // lines in its initial implicit-grid estimate, so defer those declarations
    // until restore_items can resolve them against the actual in-flow grid.
    if has_deferred_placement(original) {
        let mut intrinsic = original.clone();
        intrinsic.grid_column = Line::default();
        intrinsic.grid_row = Line::default();
        return Some(intrinsic);
    }
    if !has_cyclic_width(original) {
        return None;
    }
    let mut intrinsic = original.clone();
    for (value, indefinite) in [
        (&mut intrinsic.size.width, Dimension::auto()),
        (&mut intrinsic.min_size.width, Dimension::length(0.0)),
        (&mut intrinsic.max_size.width, Dimension::auto()),
    ] {
        if cyclic_percentage(*value) {
            *value = indefinite;
        }
    }
    Some(intrinsic)
}

fn needs_intrinsic_style(style: &Style) -> bool {
    has_cyclic_width(style) || has_deferred_placement(style)
}

fn has_deferred_placement(style: &Style) -> bool {
    style.position == Position::Absolute
        && (style.grid_column != Line::default() || style.grid_row != Line::default())
}

fn has_cyclic_width(style: &Style) -> bool {
    style.position != Position::Absolute
        && style.display != Display::None
        && [style.size.width, style.min_size.width, style.max_size.width]
            .into_iter()
            .any(cyclic_percentage)
}

fn cyclic_percentage(value: Dimension) -> bool {
    value.tag() == taffy::CompactLength::PERCENT_TAG || value.into_raw().is_calc()
}

/// Represent each already-used gutter as a fixed spacer track. This preserves
/// collapsed auto-fit gutters, implicit tracks and distributed content alignment.
fn freeze_columns(style: &mut Style, info: &DetailedGridInfo, width: f32) {
    let sizes = &info.columns.sizes;
    let gutters = &info.columns.gutters;
    let active = active_columns(style, info);
    let count = active.iter().filter(|v| **v).count();
    let free = width - sizes.iter().sum::<f32>() - gutters.iter().sum::<f32>();
    let (leading, between) = alignment_spacing(style.justify_content, free, count);
    let mut seen = false;
    let mut tracks = Vec::with_capacity(sizes.len() * 2 + 1);
    for (i, size) in sizes.iter().enumerate() {
        let alignment = if active[i] {
            let offset = if seen { between } else { leading };
            seen = true;
            offset
        } else {
            0.0
        };
        tracks.push(TrackSizingFunction::from_length(gutters[i] + alignment));
        tracks.push(TrackSizingFunction::from_length(*size));
    }
    tracks.push(TrackSizingFunction::from_length(*gutters.last().unwrap()));
    style.grid_template_columns = tracks;
    style.gap.width = LengthPercentage::length(0.0);
    style.justify_content = Some(JustifyContent::Start);
}

fn active_columns(style: &Style, info: &DetailedGridInfo) -> Vec<bool> {
    let mut active = vec![true; info.columns.sizes.len()];
    let total = usize::from(info.columns.explicit_tracks);
    // parse_repeat expands every counted repetition into Single tracks. Only
    // auto-fit/auto-fill repetitions reach grid_container_style and this adapter.
    let fixed = style
        .grid_template_columns
        .iter()
        .filter(|track| matches!(track, TrackSizingFunction::Single(_)))
        .count();
    let mut start = usize::from(info.columns.negative_implicit_tracks);
    for track in &style.grid_template_columns {
        match track {
            TrackSizingFunction::Repeat(GridTrackRepetition::AutoFit, _) => {
                for (i, value) in active
                    .iter_mut()
                    .enumerate()
                    .skip(start)
                    .take(total - fixed)
                {
                    *value = info.columns.sizes[i] != 0.0
                        || info.items.iter().any(|item| {
                            usize::from(item.column_start) <= i + 1
                                && i + 1 < usize::from(item.column_end)
                        });
                }
                start += total - fixed;
            }
            TrackSizingFunction::Repeat(_, _) => start += total - fixed,
            TrackSizingFunction::Single(_) => start += 1,
        }
    }
    active
}

fn alignment_spacing(alignment: Option<JustifyContent>, free: f32, count: usize) -> (f32, f32) {
    let n = count.max(1) as f32;
    match alignment {
        Some(JustifyContent::End | JustifyContent::FlexEnd) => (free, 0.0),
        Some(JustifyContent::SpaceBetween) if count > 1 && free > 0.0 => (0.0, free / (n - 1.0)),
        Some(JustifyContent::SpaceAround) if free > 0.0 => (free / (2.0 * n), free / n),
        Some(JustifyContent::SpaceEvenly) if free > 0.0 => (free / (n + 1.0), free / (n + 1.0)),
        Some(
            JustifyContent::Center | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly,
        ) => (free / 2.0, 0.0),
        _ => (0.0, 0.0),
    }
}

fn restore_items(
    tree: &mut TaffyTree<usize>,
    leaves: &[TaffyId],
    originals: Vec<Style>,
    info: &DetailedGridInfo,
) {
    let mut placements = info.items.iter();
    for (id, mut style) in leaves.iter().zip(originals) {
        if style.position != Position::Absolute && style.display != Display::None {
            let placement = placements
                .next()
                .expect("every in-flow grid item has a placement");
            style.grid_column = Line {
                start: frozen_line(u32::from(placement.column_start.saturating_sub(1)), true)
                    .unwrap_or(super::GridPlacement::Auto),
                end: frozen_line(u32::from(placement.column_end.saturating_sub(1)), false)
                    .unwrap_or(super::GridPlacement::Auto),
            };
        } else if style.position == Position::Absolute {
            style.grid_column = absolute_placement(style.grid_column, &info.columns, true);
            style.grid_row = absolute_placement(style.grid_row, &info.rows, false);
        }
        tree.set_style(*id, style).unwrap();
    }
}

/// Resolve spans before converting grid coordinates. Doing this in i32 prevents
/// Taffy's i16 origin-line arithmetic from overflowing on valid large CSS spans.
fn absolute_placement(
    placement: Line<super::GridPlacement>,
    tracks: &taffy::DetailedGridTracksInfo,
    expanded: bool,
) -> Line<super::GridPlacement> {
    let start = explicit_line(placement.start, tracks.explicit_tracks);
    let end = explicit_line(placement.end, tracks.explicit_tracks);
    let (start, end) = match (start, end) {
        (Some(a), Some(b)) if a == b => (Some(a), Some(a + 1)),
        (Some(a), Some(b)) => (Some(a.min(b)), Some(a.max(b))),
        (Some(a), None) => (Some(a), span_line(placement.end, a, 1)),
        (None, Some(b)) => (span_line(placement.start, b, -1), Some(b)),
        (None, None) => (None, None),
    };
    let map = |value: Option<i32>, start: bool| {
        value
            .and_then(|value| mapped_absolute_line(value, tracks, expanded, start))
            .unwrap_or(super::GridPlacement::Auto)
    };
    Line {
        start: map(start, true),
        end: map(end, false),
    }
}

fn explicit_line(placement: super::GridPlacement, explicit: u16) -> Option<i32> {
    let super::GridPlacement::Line(index) = placement else {
        return None;
    };
    let index = i32::from(index.as_i16());
    match index {
        0 => None,
        1.. => Some(index - 1),
        _ => Some(i32::from(explicit) + index + 1),
    }
}

fn span_line(placement: super::GridPlacement, base: i32, sign: i32) -> Option<i32> {
    match placement {
        super::GridPlacement::Span(n) if n > 0 => Some(base + sign * i32::from(n)),
        _ => None,
    }
}

fn mapped_absolute_line(
    original: i32,
    tracks: &taffy::DetailedGridTracksInfo,
    expanded: bool,
    start: bool,
) -> Option<super::GridPlacement> {
    let index = original + i32::from(tracks.negative_implicit_tracks);
    if index < 0 || index as usize > tracks.sizes.len() {
        return None;
    }
    if expanded {
        return frozen_line(index as u32, start);
    }
    // Rows retain their original explicit grid, including negative implicit rows.
    let css_line = if original >= 0 {
        i64::from(original) + 1
    } else {
        i64::from(original) - i64::from(tracks.explicit_tracks) - 1
    };
    i16::try_from(css_line).ok().map(line)
}

fn frozen_line(original: u32, start: bool) -> Option<super::GridPlacement> {
    let mapped = 2 * (i64::from(original) + 1) - i64::from(!start);
    i16::try_from(mapped).ok().map(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_repetitions_are_expanded_before_grid_track_adapter() {
        let parsed = super::super::grid_tracks(Some("20px repeat(2, 1fr 50px) 10px"));
        assert_eq!(
            parsed,
            vec![
                TrackSizingFunction::from_length(20.0_f32),
                super::super::fr(1.0_f32),
                TrackSizingFunction::from_length(50.0_f32),
                super::super::fr(1.0_f32),
                TrackSizingFunction::from_length(50.0_f32),
                TrackSizingFunction::from_length(10.0_f32),
            ]
        );
    }

    #[test]
    fn spacer_grid_lines_use_checked_representation_limits() {
        assert_eq!(frozen_line(16_382, true), Some(line(32_766)));
        assert_eq!(frozen_line(16_383, false), Some(line(i16::MAX)));
        assert_eq!(frozen_line(16_383, true), None);
        assert_eq!(frozen_line(u32::MAX, true), None);
        assert_eq!(frozen_line(u32::MAX, false), None);
        assert_eq!(explicit_line(line(0), 3), None);
        assert_eq!(explicit_line(line(1), 3), Some(0));
        assert_eq!(explicit_line(line(-1), 3), Some(3));
        assert_eq!(span_line(super::super::span(0), 1, 1), None);
    }
}

//! Taffy 0.8 resolves percentage preferred widths against the whole grid when
//! computing an item's minimum contribution. CSS treats that cyclic percentage
//! as indefinite until tracks have been sized. Resolve tracks without it, then
//! retain those tracks while restoring area-relative sizes for final placement.
use super::{
    line, solve, Ctx, Dimension, Display, Fill, FromLength, GridTrackRepetition, JustifyContent,
    LayoutBox, LengthPercentage, LengthPercentageAuto, Line, Position, Style, TaffyId, TaffyTree,
    TrackSizingFunction,
};
use taffy::DetailedGridInfo;
use taffy::DetailedLayoutInfo;

#[path = "grid_frames.rs"]
mod frames;
#[path = "grid_repeat.rs"]
mod repeat;

pub(super) fn solve_grid(
    tree: &mut TaffyTree<usize>,
    root: TaffyId,
    leaves: &[TaffyId],
    items: &[LayoutBox],
    dimensions: (f32, f32),
    fill: Fill,
    ctx: &Ctx,
) {
    let auto_repeat = repeat::has_auto_repeat(tree.style(root).unwrap());
    if !auto_repeat
        && !leaves
            .iter()
            .any(|id| needs_intrinsic_style(tree.style(*id).unwrap()))
    {
        solve(tree, root, items, dimensions, fill, ctx);
        return;
    }
    let mut root_style = tree.style(root).unwrap().clone();
    let originals: Vec<_> = leaves
        .iter()
        .map(|id| tree.style(*id).unwrap().clone())
        .collect();
    defer_intrinsic_styles(tree, leaves, &originals);
    if auto_repeat {
        tree.set_style(root, repeat::probe_style(root_style.clone(), dimensions.1))
            .unwrap();
    }
    solve(tree, root, items, dimensions, fill, ctx);
    let DetailedLayoutInfo::Grid(info) = tree.detailed_layout_info(root) else {
        unreachable!("a solved grid records its tracks");
    };
    let mut info = info.clone();
    let mut offsets = info.columns.gutters.clone();
    if let Some(plan) = repeat::collapse_plan(&root_style, &info) {
        let deferred = leaves
            .iter()
            .map(|id| tree.style(*id).unwrap().clone())
            .collect();
        tree.set_style(root, plan.style).unwrap();
        restore_items(tree, leaves, deferred, &info, None, None);
        solve(tree, root, items, dimensions, fill, ctx);
        let DetailedLayoutInfo::Grid(sized) = tree.detailed_layout_info(root) else {
            unreachable!("the expanded grid records its tracks");
        };
        repeat::project(&mut info, sized, plan.gutters);
        offsets = plan.offsets;
    }
    let (gutters, trailing) = freeze_columns(&mut root_style, &info, &offsets, dimensions.1);
    let areas = frames::Areas::new(&info.columns.sizes, &gutters, dimensions.1);
    tree.set_style(root, root_style).unwrap();
    restore_items(tree, leaves, originals, &info, Some(&areas), trailing);
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
        (&mut intrinsic.max_size.width, Dimension::auto()),
    ] {
        if cyclic_percentage(*value) {
            *value = indefinite;
        }
    }
    if cyclic_percentage(intrinsic.min_size.width) {
        let raw = intrinsic.min_size.width.into_raw();
        intrinsic.min_size.width = if raw.is_calc() {
            Dimension::calc(fixed_calc(raw.calc_value(), true))
        } else {
            Dimension::length(0.0)
        };
    }
    defer_edges(&mut intrinsic);
    Some(intrinsic)
}

fn fixed_calc(token: *const (), nonnegative: bool) -> *const () {
    let px = super::native_tree::calc_fixed(token);
    super::native_tree::calc(0.0, if nonnegative { px.max(0.0) } else { px })
}

// CSS Sizing §5.2.1 resolves cyclic frame percentages against zero, retaining
// fixed calc terms and the same LayoutUnit quantization used in final placement.
fn defer_edges(style: &mut Style) {
    for value in [
        &mut style.padding.left,
        &mut style.padding.right,
        &mut style.padding.top,
        &mut style.padding.bottom,
    ] {
        let raw = value.into_raw();
        if raw.is_calc() {
            *value = LengthPercentage::calc(fixed_calc(raw.calc_value(), true));
        } else if raw.tag() == taffy::CompactLength::PERCENT_TAG {
            *value = LengthPercentage::length(0.0);
        }
    }
    for value in [
        &mut style.margin.left,
        &mut style.margin.right,
        &mut style.margin.top,
        &mut style.margin.bottom,
    ] {
        let raw = value.into_raw();
        if raw.is_calc() {
            *value = LengthPercentageAuto::calc(fixed_calc(raw.calc_value(), false));
        } else if raw.tag() == taffy::CompactLength::PERCENT_TAG {
            *value = LengthPercentageAuto::length(0.0);
        }
    }
}

fn needs_intrinsic_style(style: &Style) -> bool {
    has_cyclic_width(style) || has_deferred_placement(style)
}

fn has_deferred_placement(style: &Style) -> bool {
    style.position == Position::Absolute
        && (style.grid_column != Line::default() || style.grid_row != Line::default())
}

fn has_cyclic_width(style: &Style) -> bool {
    if style.position == Position::Absolute || style.display == Display::None {
        return false;
    }
    [style.size.width, style.min_size.width, style.max_size.width]
        .into_iter()
        .any(cyclic_percentage)
        || cyclic_edges(style)
}

fn cyclic_edges(style: &Style) -> bool {
    [
        style.padding.left.into_raw(),
        style.padding.right.into_raw(),
        style.padding.top.into_raw(),
        style.padding.bottom.into_raw(),
        style.margin.left.into_raw(),
        style.margin.right.into_raw(),
        style.margin.top.into_raw(),
        style.margin.bottom.into_raw(),
    ]
    .into_iter()
    .any(|value| value.is_calc() || value.tag() == taffy::CompactLength::PERCENT_TAG)
}

fn cyclic_percentage(value: Dimension) -> bool {
    value.tag() == taffy::CompactLength::PERCENT_TAG || value.into_raw().is_calc()
}

/// Represent each already-used gutter as a fixed spacer track. This preserves
/// collapsed auto-fit gutters, implicit tracks and distributed content alignment.
fn freeze_columns(
    style: &mut Style,
    info: &DetailedGridInfo,
    offsets: &[f32],
    width: f32,
) -> (Vec<f32>, Option<super::GridPlacement>) {
    let sizes = &info.columns.sizes;
    let gutters = &info.columns.gutters;
    let active = active_columns(style, info);
    let count = active.iter().filter(|v| **v).count();
    let free = width - sizes.iter().sum::<f32>() - gutters.iter().sum::<f32>();
    let (leading, between) = alignment_spacing(style.justify_content, free, count);
    let gutters = aligned_gutters(offsets, &active, leading, between);
    let tracks = repeat::interleave(
        sizes
            .iter()
            .map(|size| TrackSizingFunction::from_length(*size)),
        &gutters,
    );

    style.grid_template_columns = tracks;
    style.gap.width = LengthPercentage::length(0.0);
    style.justify_content = Some(JustifyContent::Start);
    let trailing = active.iter().rposition(|present| *present).map_or_else(
        || frozen_line(0, true),
        |last| frozen_line(last as u32 + 1, false),
    );
    (gutters, trailing)
}

fn aligned_gutters(base: &[f32], active: &[bool], leading: f32, between: f32) -> Vec<f32> {
    let mut gutters = base.to_vec();
    gutters[0] += leading;
    let mut previous = None;
    for (i, present) in active.iter().enumerate() {
        if *present {
            if let Some(previous) = previous {
                gutters[previous + 1] += between;
            }
            previous = Some(i);
        }
    }
    // Chromium's collapsed trailing line anchors continue the last overlapped
    // gutter, although that gutter does not consume track-sizing free space.
    if let Some(previous) = previous {
        if previous + 1 < active.len() {
            gutters[previous + 1] += between;
        }
    }
    gutters
}

fn active_columns(style: &Style, info: &DetailedGridInfo) -> Vec<bool> {
    let mut active = vec![true; info.columns.sizes.len()];
    let total = usize::from(info.columns.explicit_tracks);
    if total == 0 {
        return active;
    }
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
                    *value = info.items.iter().any(|item| {
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
    let n = count as f32;
    match alignment {
        Some(JustifyContent::End | JustifyContent::FlexEnd) => (free, 0.0),
        Some(JustifyContent::SpaceBetween) if count > 1 && free > 0.0 => (0.0, free / (n - 1.0)),
        Some(JustifyContent::SpaceAround) if free > 0.0 && count > 0 => {
            (free / (2.0 * n), free / n)
        }
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
    areas: Option<&frames::Areas>,
    trailing: Option<super::GridPlacement>,
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
            let row = |boundary: u16, start| {
                let original =
                    i32::from(boundary) - 1 - i32::from(info.rows.negative_implicit_tracks);
                mapped_absolute_line(original, &info.rows, false, start)
                    .unwrap_or(super::GridPlacement::Auto)
            };
            style.grid_row = Line {
                start: row(placement.row_start, true),
                end: row(placement.row_end, false),
            };
        } else if style.position == Position::Absolute {
            style.grid_column =
                absolute_placement(style.grid_column, &info.columns, true, trailing);
            style.grid_row = absolute_placement(style.grid_row, &info.rows, false, None);
        }
        if let Some(areas) = areas {
            let width = areas.width(style.grid_column);
            frames::resolve(&mut style, width);
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
    trailing: Option<super::GridPlacement>,
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
            .and_then(|value| {
                if expanded
                    && value + i32::from(tracks.negative_implicit_tracks)
                        == tracks.sizes.len() as i32
                {
                    return trailing
                        .or_else(|| mapped_absolute_line(value, tracks, expanded, start));
                }
                mapped_absolute_line(value, tracks, expanded, start)
            })
            .unwrap_or(super::GridPlacement::Auto)
    };
    let start = map(start, true);
    let mut end = map(end, false);
    // Logical indices were normalized above. A collapsed trailing internal line
    // may lie beyond the outer end line; its area remains zero at that start,
    // rather than letting Taffy sort the transformed physical indices again.
    if let (super::GridPlacement::Line(a), super::GridPlacement::Line(b)) = (start, end) {
        if a.as_i16() > b.as_i16() {
            end = start;
        }
    }
    Line { start, end }
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
        let start = if index == 0 {
            true
        } else if index as usize == tracks.sizes.len() {
            false
        } else {
            start
        };
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
    fn cyclic_edges_keep_fixed_calc_terms_and_restore_original_declarations() {
        let _scope = super::super::native_tree::CalcScope::new(None);
        let token = super::super::native_tree::calc(10.0, 4.1);
        let negative = super::super::native_tree::calc(25.0, -3.0);
        let mut original = Style::default();
        original.padding.left = LengthPercentage::calc(token);
        original.padding.right = LengthPercentage::percent(0.1);
        original.padding.top = LengthPercentage::length(5.0);
        original.margin.left = LengthPercentageAuto::calc(negative);
        original.margin.right = LengthPercentageAuto::percent(0.05);
        original.margin.top = LengthPercentageAuto::auto();
        original.min_size.width = Dimension::calc(token);
        let before = original.clone();
        let intrinsic =
            intrinsic_style(&original).expect("percentage edges participate in intrinsic sizing");
        let resolve = |raw: taffy::CompactLength| {
            super::super::native_tree::resolve_for_test(raw.calc_value(), 1000.0)
        };
        assert_eq!(resolve(intrinsic.padding.left.into_raw()), 4.09375);
        assert_eq!(intrinsic.padding.right, LengthPercentage::length(0.0));
        assert_eq!(intrinsic.padding.top, original.padding.top);
        assert_eq!(resolve(intrinsic.margin.left.into_raw()), -3.0);
        assert_eq!(intrinsic.margin.right, LengthPercentageAuto::length(0.0));
        assert_eq!(intrinsic.margin.top, LengthPercentageAuto::auto());
        assert_eq!(resolve(intrinsic.min_size.width.into_raw()), 4.09375);
        assert_eq!(
            original, before,
            "final placement retains the complete declared style"
        );
        let only_edge = Style {
            padding: taffy::Rect {
                bottom: LengthPercentage::percent(0.1),
                ..Style::default().padding
            },
            ..Style::default()
        };
        assert!(needs_intrinsic_style(&only_edge));
        assert!(intrinsic_style(&only_edge).is_some());
    }

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

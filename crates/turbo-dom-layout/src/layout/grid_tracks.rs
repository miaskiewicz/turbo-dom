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
        .any(|id| has_cyclic_width(tree.style(*id).unwrap()))
    {
        solve(tree, root, items, dimensions, fill, ctx);
        return;
    }
    let originals: Vec<_> = leaves
        .iter()
        .map(|id| tree.style(*id).unwrap().clone())
        .collect();
    let changed = indefinite_percentages(tree, leaves, &originals);
    solve(tree, root, items, dimensions, fill, ctx);
    if !changed {
        return;
    }
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

fn indefinite_percentages(
    tree: &mut TaffyTree<usize>,
    leaves: &[TaffyId],
    styles: &[Style],
) -> bool {
    let mut changed = false;
    for (id, original) in leaves.iter().zip(styles) {
        if let Some(intrinsic) = intrinsic_style(original) {
            tree.set_style(*id, intrinsic).unwrap();
            changed = true;
        }
    }
    changed
}

fn intrinsic_style(original: &Style) -> Option<Style> {
    if !has_cyclic_width(original) {
        return None;
    }
    let mut intrinsic = original.clone();
    let mut changed = false;
    for (value, indefinite) in [
        (&mut intrinsic.size.width, Dimension::auto()),
        (&mut intrinsic.min_size.width, Dimension::length(0.0)),
        (&mut intrinsic.max_size.width, Dimension::auto()),
    ] {
        if cyclic_percentage(*value) {
            *value = indefinite;
            changed = true;
        }
    }
    changed.then_some(intrinsic)
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
    let fixed: usize = style
        .grid_template_columns
        .iter()
        .map(|track| match track {
            TrackSizingFunction::Single(_) => 1,
            TrackSizingFunction::Repeat(GridTrackRepetition::Count(n), tracks) => {
                usize::from(*n) * tracks.len()
            }
            TrackSizingFunction::Repeat(_, _) => 0,
        })
        .sum();
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
            TrackSizingFunction::Repeat(GridTrackRepetition::AutoFill, _) => start += total - fixed,
            TrackSizingFunction::Repeat(GridTrackRepetition::Count(n), tracks) => {
                start += usize::from(*n) * tracks.len();
            }
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
                start: line((2 * placement.column_start) as i16),
                end: line((2 * placement.column_end - 1) as i16),
            };
        } else if style.position == Position::Absolute {
            style.grid_column = Line {
                start: absolute_boundary(style.grid_column.start, true, info),
                end: absolute_boundary(style.grid_column.end, false, info),
            };
        }
        tree.set_style(*id, style).unwrap();
    }
}

fn absolute_boundary(
    placement: super::GridPlacement,
    start: bool,
    info: &DetailedGridInfo,
) -> super::GridPlacement {
    match placement {
        super::GridPlacement::Auto => super::GridPlacement::Auto,
        super::GridPlacement::Span(n) => super::span(n * 2 - 1),
        super::GridPlacement::Line(index) => {
            let index = index.as_i16();
            if index == 0 {
                return super::GridPlacement::Auto;
            }
            let explicit = if index > 0 {
                index - 1
            } else {
                info.columns.explicit_tracks as i16 + index + 1
            };
            let original = explicit + info.columns.negative_implicit_tracks as i16;
            if original < 0 || original as usize > info.columns.sizes.len() {
                return super::GridPlacement::Auto;
            }
            line(2 * (original + 1) - i16::from(!start))
        }
    }
}

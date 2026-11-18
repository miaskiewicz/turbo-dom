//! Auto-fit track collapse with the original sizing functions. Gutters are fixed
//! tracks during sizing (CSS Grid §10.1); a collapsed run overlaps its internal
//! gutters once and has no gutter at an outer grid edge.
use super::{
    active_columns, GridTrackRepetition, JustifyContent, LengthPercentage, Style,
    TrackSizingFunction,
};
use taffy::prelude::{FromLength, TaffyAuto};
use taffy::util::ResolveOrZero;
use taffy::{DetailedGridInfo, NonRepeatedTrackSizingFunction};

pub(super) fn has_auto_repeat(style: &Style) -> bool {
    style.grid_template_columns.iter().any(|track| {
        matches!(
            track,
            TrackSizingFunction::Repeat(
                GridTrackRepetition::AutoFit | GridTrackRepetition::AutoFill,
                _
            )
        )
    })
}

/// Auto-fill and auto-fit have identical counts and placement. Probe without
/// collapse so Taffy's retained trailing gutter cannot affect the sizing input.
pub(super) fn probe_style(mut style: Style, width: f32) -> Style {
    for track in &mut style.grid_template_columns {
        match track {
            TrackSizingFunction::Single(function) => {
                *function = counting_function(*function, width, false);
            }
            TrackSizingFunction::Repeat(kind, members) => {
                if *kind == GridTrackRepetition::AutoFit {
                    *kind = GridTrackRepetition::AutoFill;
                }
                for function in members {
                    *function = counting_function(*function, width, true);
                }
            }
        }
    }
    style.grid_template_columns = counted_tracks(
        &style.grid_template_columns,
        Some(width),
        style
            .gap
            .width
            .resolve_or_zero(Some(width), super::super::native_tree::resolve_calc),
    );
    style
}

/// Compute auto-repeat counts before Taffy sees them: its u16 arithmetic must
/// never receive an infinite quotient or an unbounded repeated list.
pub(super) fn counted_tracks(
    template: &[TrackSizingFunction],
    width: Option<f32>,
    gap: f32,
) -> Vec<TrackSizingFunction> {
    counted_tracks_mode(template, width, gap, false)
}

pub(super) fn counted_tracks_mode(
    template: &[TrackSizingFunction],
    width: Option<f32>,
    gap: f32,
    minimum: bool,
) -> Vec<TrackSizingFunction> {
    let limit = super::super::grid_capacity::MAX_TRACKS;
    let fixed = template
        .iter()
        .filter(|t| matches!(t, TrackSizingFunction::Single(_)))
        .count();
    let repeated: Vec<_> = template
        .iter()
        .filter_map(|t| match t {
            TrackSizingFunction::Repeat(_, members) => Some(members),
            TrackSizingFunction::Single(_) => None,
        })
        .collect();
    if repeated.is_empty() {
        return template.iter().take(limit).cloned().collect();
    }
    if repeated.len() != 1 || repeated[0].is_empty() {
        return Vec::new();
    }
    let basis = width.unwrap_or(0.0);
    let values: Option<Vec<f64>> = template
        .iter()
        .map(|track| match track {
            TrackSizingFunction::Single(f) => counting_value(*f, basis, false),
            TrackSizingFunction::Repeat(_, members) => members
                .iter()
                .map(|f| counting_value(*f, basis, true))
                .sum(),
        })
        .collect();
    let Some(values) = values else {
        return Vec::new();
    };
    let fixed_space = template
        .iter()
        .zip(&values)
        .filter(|(t, _)| matches!(t, TrackSizingFunction::Single(_)))
        .map(|(_, v)| *v)
        .sum();
    let member_space = template
        .iter()
        .zip(&values)
        .find_map(|(t, v)| matches!(t, TrackSizingFunction::Repeat(_, _)).then_some(*v))
        .unwrap();
    let count = repeat_members(
        (width, minimum),
        fixed,
        fixed_space,
        repeated[0].len(),
        member_space,
        gap,
        limit.saturating_sub(fixed),
    );
    let mut output = Vec::with_capacity((fixed + count).min(limit));
    for track in template {
        match track {
            TrackSizingFunction::Single(_) => {
                if output.len() < limit {
                    output.push(track.clone());
                }
            }
            TrackSizingFunction::Repeat(_, members) => output.extend(
                members
                    .iter()
                    .copied()
                    .cycle()
                    .take(count.min(limit - output.len()))
                    .map(TrackSizingFunction::Single),
            ),
        }
    }
    output
}
fn counting_value(
    function: NonRepeatedTrackSizingFunction,
    width: f32,
    repeated: bool,
) -> Option<f64> {
    let min = function
        .min
        .definite_value(Some(width), super::super::native_tree::resolve_calc);
    let max = function
        .max
        .definite_value(Some(width), super::super::native_tree::resolve_calc);
    let value = match (min, max) {
        (Some(a), Some(b)) => a.max(b),
        (Some(a), None) | (None, Some(a)) => a,
        _ => return None,
    };
    Some(f64::from(if repeated {
        value.max(1.0)
    } else {
        value.max(0.0)
    }))
}
fn repeat_members(
    counting: (Option<f32>, bool),
    fixed: usize,
    fixed_space: f64,
    members: usize,
    space: f64,
    gap: f32,
    remaining: usize,
) -> usize {
    if remaining == 0 {
        return 0;
    }
    let first =
        fixed_space + space + (fixed + members).saturating_sub(1) as f64 * f64::from(gap.max(0.0));
    let step = space + members as f64 * f64::from(gap.max(0.0));
    let repetitions = counting.0.map_or(1.0, |w| {
        let quotient = (f64::from(w) - first) / step;
        let rounded = if counting.1 {
            quotient.ceil()
        } else {
            quotient.floor()
        };
        rounded.max(0.0) + 1.0
    });
    (repetitions.min(remaining as f64) as usize)
        .saturating_mul(members)
        .min(remaining)
}

fn counting_function(
    function: NonRepeatedTrackSizingFunction,
    width: f32,
    repeated: bool,
) -> NonRepeatedTrackSizingFunction {
    counting_value(function, width, repeated).map_or(function, |value| {
        NonRepeatedTrackSizingFunction::from_length(value as f32)
    })
}

pub(super) struct CollapsePlan {
    pub style: Style,
    pub gutters: Vec<f32>,
    pub offsets: Vec<f32>,
}

pub(super) fn collapse_plan(style: &Style, info: &DetailedGridInfo) -> Option<CollapsePlan> {
    if !has_auto_repeat(style) || info.columns.explicit_tracks == 0 {
        return None;
    }
    let active = active_columns(style, info);
    let (gutters, offsets) = collapsed_gutters(&info.columns.gutters, &active);
    let functions = expanded_columns(style, info);
    let functions = functions
        .into_iter()
        .zip(&active)
        .map(|(function, present)| {
            if *present {
                function
            } else {
                TrackSizingFunction::from_length(0.0_f32)
            }
        });
    let mut sizing = style.clone();
    sizing.grid_template_columns = interleave(functions, &gutters);
    sizing.gap.width = LengthPercentage::length(0.0);
    // Preserve normal/stretch's auto-track growth. Distributed alignment belongs
    // to the original tracks, not these implementation spacer columns.
    if !matches!(sizing.justify_content, None | Some(JustifyContent::Stretch)) {
        sizing.justify_content = Some(JustifyContent::Start);
    }
    Some(CollapsePlan {
        style: sizing,
        gutters,
        offsets,
    })
}

/// Reuse the same track solver in the block axis; logical track indices and
/// content alignment exchange axes, while item dimensions remain untouched.
pub(super) fn transpose_style(style: &mut Style) {
    std::mem::swap(
        &mut style.grid_template_columns,
        &mut style.grid_template_rows,
    );
    std::mem::swap(&mut style.grid_auto_columns, &mut style.grid_auto_rows);
    std::mem::swap(&mut style.gap.width, &mut style.gap.height);
    std::mem::swap(&mut style.justify_content, &mut style.align_content);
}
pub(super) fn transpose_info(info: &mut DetailedGridInfo) {
    std::mem::swap(&mut info.columns, &mut info.rows);
    for item in &mut info.items {
        std::mem::swap(&mut item.column_start, &mut item.row_start);
        std::mem::swap(&mut item.column_end, &mut item.row_end);
    }
}
pub(super) fn row_plan(style: &Style, info: &DetailedGridInfo) -> Option<CollapsePlan> {
    let mut transposed_style = style.clone();
    let mut transposed_info = info.clone();
    transpose_style(&mut transposed_style);
    transpose_info(&mut transposed_info);
    let mut plan = collapse_plan(&transposed_style, &transposed_info)?;
    transpose_style(&mut plan.style);
    Some(plan)
}
pub(super) fn project_rows(
    info: &mut DetailedGridInfo,
    sized: &DetailedGridInfo,
    gutters: Vec<f32>,
) {
    info.rows.sizes = sized
        .rows
        .sizes
        .iter()
        .skip(1)
        .step_by(2)
        .copied()
        .collect();
    info.rows.gutters = gutters;
}

fn collapsed_gutters(base: &[f32], active: &[bool]) -> (Vec<f32>, Vec<f32>) {
    let mut gutters = vec![0.0; base.len()];
    let mut previous = None;
    for (i, present) in active.iter().enumerate() {
        if *present {
            if let Some(previous) = previous {
                gutters[previous + 1] = base[previous + 1];
            }
            previous = Some(i);
        }
    }
    let mut offsets = gutters.clone();
    if let Some(previous) = previous {
        if previous + 1 < active.len() {
            offsets[previous + 1] = base[previous + 1];
        }
    }
    (gutters, offsets)
}

pub(super) fn interleave(
    functions: impl Iterator<Item = TrackSizingFunction>,
    gutters: &[f32],
) -> Vec<TrackSizingFunction> {
    let mut tracks = Vec::with_capacity(gutters.len() * 2 - 1);
    for (i, function) in functions.enumerate() {
        tracks.push(TrackSizingFunction::from_length(gutters[i]));
        tracks.push(function);
    }
    tracks.push(TrackSizingFunction::from_length(*gutters.last().unwrap()));
    tracks
}

fn expanded_columns(style: &Style, info: &DetailedGridInfo) -> Vec<TrackSizingFunction> {
    let counts = &info.columns;
    let mut columns = Vec::with_capacity(counts.sizes.len());
    implicit_columns(
        &mut columns,
        &style.grid_auto_columns,
        usize::from(counts.negative_implicit_tracks),
        true,
    );
    if counts.explicit_tracks > 0 {
        let fixed = style
            .grid_template_columns
            .iter()
            .filter(|track| matches!(track, TrackSizingFunction::Single(_)))
            .count();
        let repeated = usize::from(counts.explicit_tracks).saturating_sub(fixed);
        for track in &style.grid_template_columns {
            match track {
                TrackSizingFunction::Single(_) => columns.push(track.clone()),
                // Counted repeats were already expanded by the CSS parser.
                TrackSizingFunction::Repeat(_, members) => columns.extend(
                    members
                        .iter()
                        .copied()
                        .cycle()
                        .take(repeated)
                        .map(TrackSizingFunction::Single),
                ),
            }
        }
    }
    implicit_columns(
        &mut columns,
        &style.grid_auto_columns,
        usize::from(counts.positive_implicit_tracks),
        false,
    );
    columns
}

fn implicit_columns(
    columns: &mut Vec<TrackSizingFunction>,
    definitions: &[NonRepeatedTrackSizingFunction],
    count: usize,
    negative: bool,
) {
    if definitions.is_empty() {
        columns.extend(std::iter::repeat_n(TrackSizingFunction::AUTO, count));
        return;
    }
    let offset = if negative {
        definitions.len() - count % definitions.len()
    } else {
        0
    };
    columns.extend(
        definitions
            .iter()
            .copied()
            .cycle()
            .skip(offset)
            .take(count)
            .map(TrackSizingFunction::Single),
    );
}

pub(super) fn project(info: &mut DetailedGridInfo, sized: &DetailedGridInfo, gutters: Vec<f32>) {
    info.columns.sizes = sized
        .columns
        .sizes
        .iter()
        .skip(1)
        .step_by(2)
        .copied()
        .collect();
    info.columns.gutters = gutters;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_patterns_repeat_from_each_explicit_grid_edge() {
        let definitions = [
            NonRepeatedTrackSizingFunction::from_length(40.0_f32),
            NonRepeatedTrackSizingFunction::from_length(60.0_f32),
        ];
        let mut leading = Vec::new();
        implicit_columns(&mut leading, &definitions, 3, true);
        assert_eq!(
            leading,
            [60.0_f32, 40.0, 60.0].map(TrackSizingFunction::from_length)
        );
        let mut trailing = Vec::new();
        implicit_columns(&mut trailing, &definitions, 3, false);
        assert_eq!(
            trailing,
            [40.0_f32, 60.0, 40.0].map(TrackSizingFunction::from_length)
        );
    }
    #[test]
    fn counting_is_bounded_before_expanding_extreme_values() {
        let zero = NonRepeatedTrackSizingFunction::from_length(0.0_f32);
        let repeated = TrackSizingFunction::Repeat(GridTrackRepetition::AutoFill, vec![zero]);
        assert_eq!(
            counted_tracks(std::slice::from_ref(&repeated), Some(f32::INFINITY), 0.0).len(),
            super::super::super::grid_capacity::MAX_TRACKS
        );
        assert_eq!(
            counted_tracks(std::slice::from_ref(&repeated), Some(120.0), 0.0).len(),
            120
        );
        assert_eq!(
            counted_tracks(std::slice::from_ref(&repeated), None, 0.0).len(),
            1
        );
        assert_eq!(
            counted_tracks(
                &[TrackSizingFunction::from_length(0.0_f32), repeated.clone()],
                Some(120.0),
                0.0
            )
            .len(),
            121
        );
        assert!(counted_tracks(&[repeated.clone(), repeated], Some(120.0), 0.0).is_empty());
        let empty = TrackSizingFunction::Repeat(GridTrackRepetition::AutoFit, Vec::new());
        assert!(counted_tracks(&[empty], Some(120.0), 0.0).is_empty());
        assert_eq!(
            repeat_members((Some(400.0), false), 1000, 1000.0, 1, 1.0, 0.0, 0),
            0
        );
    }
}

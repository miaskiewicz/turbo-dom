//! Auto-fit track collapse with the original sizing functions. Gutters are fixed
//! tracks during sizing (CSS Grid §10.1); a collapsed run overlaps its internal
//! gutters once and has no gutter at an outer grid edge.
use super::{
    active_columns, GridTrackRepetition, JustifyContent, LengthPercentage, Style,
    TrackSizingFunction,
};
use taffy::prelude::{FromLength, TaffyAuto};
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
    style
}

fn counting_function(
    function: NonRepeatedTrackSizingFunction,
    width: f32,
    repeated: bool,
) -> NonRepeatedTrackSizingFunction {
    let min = function
        .min
        .definite_value(Some(width), super::super::native_tree::resolve_calc);
    let max = function
        .max
        .definite_value(Some(width), super::super::native_tree::resolve_calc);
    let value = match (min, max) {
        (Some(min), Some(max)) => max.max(min),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => return function,
    };
    // CSS floors auto-repeated zero tracks for counting, not fixed zero prefixes.
    NonRepeatedTrackSizingFunction::from_length(if repeated { value.max(1.0) } else { value })
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
}

//! Signed outer contributions for an indefinite column main size. Taffy's
//! intrinsic algorithm floors an already outer contribution by an inner basis,
//! discarding negative margins. Measure intrinsic content without those margins,
//! then add them in outer space before the final flex solve.
use super::{resolve_calc, LayoutInput, LayoutOutput, NodeId, RunMode, Size, Style, View};
use taffy::{Dimension, FlexDirection, MaybeResolve, Position, ResolveOrZero};

fn active(style: &Style) -> bool {
    style.display != taffy::Display::None && style.position != Position::Absolute
}

fn eligible(style: &Style, inputs: LayoutInput) -> bool {
    inputs.known_dimensions.height.is_none()
        && style
            .size
            .height
            .maybe_resolve(inputs.parent_size.height, resolve_calc)
            .is_none()
        && matches!(
            style.flex_direction,
            FlexDirection::Column | FlexDirection::ColumnReverse
        )
        && style.aspect_ratio.is_none()
}

fn unresolved(value: Dimension) -> bool {
    value.tag() == taffy::CompactLength::PERCENT_TAG || value.into_raw().is_calc()
}

pub(super) fn indefinite_style(mut style: Style) -> Style {
    if unresolved(style.size.height) {
        style.size.height = Dimension::auto();
    }
    if unresolved(style.flex_basis) {
        style.flex_basis = Dimension::auto();
    }
    if unresolved(style.min_size.height) {
        style.min_size.height = Dimension::length(0.0);
    }
    if unresolved(style.max_size.height) {
        style.max_size.height = Dimension::auto();
    }
    style
}

fn clear<C>(view: &mut View<'_, C>) {
    for node in &mut view.tree.nodes {
        node.cache.clear();
    }
}

fn margin(style: &Style, width: Option<f32>) -> f32 {
    style.margin.top.resolve_or_zero(width, resolve_calc)
        + style.margin.bottom.resolve_or_zero(width, resolve_calc)
}

fn constrained_height(style: &Style, inputs: LayoutInput, intrinsic: f32) -> f32 {
    let min = style
        .min_size
        .height
        .maybe_resolve(inputs.parent_size.height, resolve_calc)
        .unwrap_or(0.0);
    let max = style
        .max_size
        .height
        .maybe_resolve(inputs.parent_size.height, resolve_calc)
        .unwrap_or(f32::INFINITY);
    intrinsic.max(0.0).min(max).max(min)
}

pub(super) fn compute<C>(
    view: &mut View<'_, C>,
    id: NodeId,
    inputs: LayoutInput,
) -> Option<LayoutOutput> {
    let index = usize::from(id);
    let original = view.tree.nodes[index].style.clone();
    if !eligible(&original, inputs) {
        return None;
    }
    let width = inputs.known_dimensions.width.or(inputs.parent_size.width);
    let children = view.tree.nodes[index].children.clone();
    let saved: Vec<_> = children
        .iter()
        .map(|child| (*child, view.tree.nodes[usize::from(*child)].style.clone()))
        .collect();
    if !saved
        .iter()
        .any(|(_, style)| active(style) && margin(style, width) < 0.0)
    {
        return None;
    }
    let total_margin: f32 = saved
        .iter()
        .filter(|(_, style)| active(style))
        .map(|(_, style)| margin(style, width))
        .sum();
    for (child, style) in &saved {
        let mut style = indefinite_style(style.clone());
        style.margin.top = taffy::LengthPercentageAuto::length(0.0);
        style.margin.bottom = taffy::LengthPercentageAuto::length(0.0);
        view.tree.nodes[usize::from(*child)].style = style;
    }
    let root = &mut view.tree.nodes[index].style;
    root.size.height = Dimension::auto();
    root.min_size.height = Dimension::auto();
    root.max_size.height = Dimension::auto();
    clear(view);
    let probe = taffy::compute_flexbox_layout(
        view,
        id,
        LayoutInput {
            run_mode: RunMode::ComputeSize,
            ..inputs
        },
    );
    let height = constrained_height(&original, inputs, probe.size.height + total_margin);
    view.tree.nodes[index].style = original.clone();
    // The final solve receives an already derived automatic main size, not a
    // CSS height declaration. Resolve in-flow percentages against the original
    // indefinite basis before supplying that internal used size to Taffy.
    for (child, style) in &saved {
        view.tree.nodes[usize::from(*child)].style = if active(style) {
            indefinite_style(style.clone())
        } else {
            style.clone()
        };
    }
    clear(view);
    let output = taffy::compute_flexbox_layout(
        view,
        id,
        LayoutInput {
            known_dimensions: Size {
                height: Some(height),
                ..inputs.known_dimensions
            },
            ..inputs
        },
    );
    view.tree.nodes[index].style = original;
    for (child, style) in saved {
        view.tree.nodes[usize::from(child)].style = style;
    }
    clear(view);
    Some(output)
}

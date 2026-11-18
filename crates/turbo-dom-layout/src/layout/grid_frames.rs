//! Resolve frames only after the grid area's used columns and gutters are known.
//! Taffy's intrinsic row contributions otherwise resolve percentages against the
//! entire grid rather than the item's own (possibly spanning) area.
use super::{LengthPercentage, LengthPercentageAuto, Line, Style};
use taffy::util::ResolveOrZero;

pub(super) struct Areas {
    points: Vec<f32>,
    width: f32,
}

impl Areas {
    pub fn new(sizes: &[f32], gutters: &[f32], width: f32) -> Self {
        let mut points = Vec::with_capacity(sizes.len() * 2 + 2);
        points.push(0.0);
        let mut position = 0.0;
        for (i, size) in sizes.iter().enumerate() {
            position += gutters[i];
            points.push(position);
            position += size;
            points.push(position);
        }
        points.push(position + gutters.last().unwrap());
        Self { points, width }
    }

    pub fn width(&self, placement: Line<super::super::GridPlacement>) -> f32 {
        (self.boundary(placement.end, self.width) - self.boundary(placement.start, 0.0)).max(0.0)
    }

    fn boundary(&self, placement: super::super::GridPlacement, fallback: f32) -> f32 {
        let super::super::GridPlacement::Line(index) = placement else {
            return fallback;
        };
        usize::try_from(i32::from(index.as_i16()) - 1)
            .ok()
            .and_then(|index| self.points.get(index))
            .copied()
            .unwrap_or(fallback)
    }
}

pub(super) fn resolve(style: &mut Style, width: f32) {
    for value in [
        &mut style.padding.left,
        &mut style.padding.right,
        &mut style.padding.top,
        &mut style.padding.bottom,
    ] {
        *value = LengthPercentage::length(
            value
                .resolve_or_zero(Some(width), super::super::native_tree::resolve_calc)
                .max(0.0),
        );
    }
    for value in [
        &mut style.margin.left,
        &mut style.margin.right,
        &mut style.margin.top,
        &mut style.margin.bottom,
    ] {
        if let Some(px) = value.resolve_to_option(width, super::super::native_tree::resolve_calc) {
            *value = LengthPercentageAuto::length(px);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::style_helpers::line;

    #[test]
    fn spanning_frames_use_area_width_and_preserve_auto_and_negative_margins() {
        let areas = Areas::new(&[100.0, 200.0], &[10.0, 20.0, 0.0], 400.0);
        let placement = Line {
            start: line(2),
            end: line(5),
        };
        assert_eq!(areas.width(placement), 320.0);
        let mut style = Style::default();
        style.padding.top = LengthPercentage::percent(0.05);
        style.margin.top = LengthPercentageAuto::percent(-0.05);
        style.margin.left = LengthPercentageAuto::auto();
        resolve(&mut style, areas.width(placement));
        assert_eq!(style.padding.top, LengthPercentage::length(16.0));
        assert_eq!(style.margin.top, LengthPercentageAuto::length(-16.0));
        assert_eq!(style.margin.left, LengthPercentageAuto::auto());
        assert_eq!(
            areas.width(Line {
                start: super::super::super::GridPlacement::Auto,
                end: line(2)
            }),
            10.0
        );
        assert_eq!(
            areas.width(Line {
                start: line(5),
                end: super::super::super::GridPlacement::Auto
            }),
            70.0
        );
        assert_eq!(
            areas.width(Line {
                start: line(i16::MIN),
                end: line(i16::MAX)
            }),
            400.0
        );
    }
}

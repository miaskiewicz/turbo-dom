//! The repeating cell of a background layer's tiles ([`TilePattern`]), shared by the PNG
//! and SVG painters so both paint the same snapped tiles (0.7.0 round-3 review, findings
//! 4 and 7).
//!
//! Each tile snaps to whole pixels, so tiles a fractional step apart alternate sizes
//! (2.5px tiles: 3px, 2px, 3px, ...). The snapped tiles still repeat, every whole number
//! of steps that spans whole pixels (two 2.5px steps: 5px): that super-period is the
//! cell, drawn once and repeated, whatever the number of tiles.

use turbo_dom_layout::TilePattern;

use crate::rrect;

/// The most steps one cell spans along an axis.
const MAX_CELL_STEPS: u16 = 64;

/// How far from a whole pixel `k` steps may land and still repeat the snapped tiles
/// exactly (f32 noise: three steps of 1/3 px).
const WHOLE: f32 = 1e-3;

/// One cell of a tile pattern, in the layer's space: `size` whole px from `origin` (a
/// tile's snapped corner), repeated along each axis whose tiles repeat; on an axis that
/// does not repeat it is the one tile's snapped extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TileCell {
    pub origin: (f32, f32),
    pub size: (f32, f32),
    /// Whether the tiles repeat along each axis.
    pub repeats: (bool, bool),
}

/// The cell length along an axis of tiles every `step` px: the whole px of the fewest
/// steps (up to [`MAX_CELL_STEPS`]) that land on a whole pixel. A step no such count
/// lands on (an irrational-looking `space` gap) takes the fewest steps that drift least
/// per step, rounded: its tiles shift by under half a pixel per cell.
pub(crate) fn cell_length(step: f32) -> f32 {
    let off = |k: u16| {
        let span = step * f32::from(k);
        (span - span.round()).abs()
    };
    let drift = |k: u16| off(k) / f32::from(k);
    let least = (1..=MAX_CELL_STEPS)
        .map(drift)
        .fold(f32::INFINITY, f32::min);
    let steps = (1..=MAX_CELL_STEPS)
        .find(|&k| off(k) <= WHOLE)
        .or_else(|| (1..=MAX_CELL_STEPS).find(|&k| drift(k) <= least + 1e-6))
        .unwrap_or(1);
    (step * f32::from(steps)).round().max(1.0)
}

impl TileCell {
    /// The cell of `pattern` whose corner is the `first` (snapped) tile's.
    pub(crate) fn new(pattern: &TilePattern, (x, y, w, h): rrect::Rect) -> TileCell {
        let along = |step: f32, size: f32| {
            if step > 0.0 {
                cell_length(step)
            } else {
                size
            }
        };
        TileCell {
            origin: (x, y),
            size: (along(pattern.step_x, w), along(pattern.step_y, h)),
            repeats: (pattern.step_x > 0.0, pattern.step_y > 0.0),
        }
    }

    /// The snapped tiles that paint inside the cell, relative to its corner, in paint
    /// order (a tile reaching in from the cell before it included).
    pub(crate) fn tiles<'a>(
        &self,
        pattern: &'a TilePattern,
    ) -> impl Iterator<Item = rrect::Rect> + 'a {
        let ((ox, oy), (w, h)) = (self.origin, self.size);
        pattern
            .tiles((ox - 1.0, oy - 1.0, w + 2.0, h + 2.0))
            .filter(move |&(x, y, tw, th)| x < ox + w && x + tw > ox && y < oy + h && y + th > oy)
            .map(move |(x, y, tw, th)| (x - ox, y - oy, tw, th))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_layout::{FragmentContent, LinearGradient};

    fn pattern(tile: (f32, f32), step: (f32, f32)) -> TilePattern {
        TilePattern {
            tile: Box::new(FragmentContent::Box {
                background: None,
                border: turbo_dom_layout::layout::value::BorderEdges::default(),
                border_radius: turbo_dom_layout::BorderRadii::default(),
                shadow: None,
                gradient: Some(LinearGradient {
                    angle_deg: 180.0,
                    stops: Vec::new(),
                }),
                transform: None,
            }),
            x: 0.0,
            y: 0.0,
            tile_width: tile.0,
            tile_height: tile.1,
            step_x: step.0,
            step_y: step.1,
        }
    }

    #[test]
    fn a_cell_spans_the_fewest_steps_that_land_on_a_whole_pixel() {
        assert_eq!(cell_length(4.0), 4.0);
        assert_eq!(cell_length(2.5), 5.0);
        assert_eq!(cell_length(1.0 / 3.0), 1.0);
        assert_eq!(cell_length(1.25), 5.0);
        // No count up to 64 lands 1.001px steps on a pixel, and every count drifts
        // 0.001px per step: the fewest, one, is the cell.
        assert_eq!(cell_length(1.001), 1.0);
        // pi px: 64 steps are 201.06px (0.062px off, 0.00097 per step), the least drift
        // (7 steps: 21.99px, 0.0013 per step).
        assert_eq!(cell_length(std::f32::consts::PI), 201.0);
    }

    #[test]
    fn a_cell_holds_its_snapped_tiles() {
        let p = pattern((2.5, 4.0), (2.5, 0.0));
        let cell = TileCell::new(&p, (0.0, 0.0, 3.0, 4.0));
        assert_eq!(
            cell,
            TileCell {
                origin: (0.0, 0.0),
                size: (5.0, 4.0),
                repeats: (true, false)
            }
        );
        let tiles: Vec<_> = cell.tiles(&p).collect();
        assert_eq!(tiles, vec![(0.0, 0.0, 3.0, 4.0), (3.0, 0.0, 2.0, 4.0)]);
        // A cell starting elsewhere holds the same tiles, relative to its corner.
        let later = TileCell::new(&p, (5.0, 0.0, 3.0, 4.0));
        assert_eq!(later.tiles(&p).collect::<Vec<_>>(), tiles);
    }

    #[test]
    fn a_tile_reaching_into_the_cell_paints_in_it() {
        // 3px tiles every 2px (overlapping): the cell from x=2 also shows the tail of
        // the tile at x=0, under the one at x=2.
        let p = pattern((3.0, 3.0), (2.0, 2.0));
        let cell = TileCell::new(&p, (2.0, 2.0, 3.0, 3.0));
        let tiles: Vec<_> = cell.tiles(&p).collect();
        assert_eq!(tiles.first(), Some(&(-2.0, -2.0, 3.0, 3.0)));
        assert_eq!(tiles.last(), Some(&(0.0, 0.0, 3.0, 3.0)));
        assert_eq!(tiles.len(), 4);
    }
}

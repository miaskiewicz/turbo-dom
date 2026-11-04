//! `position: sticky` (CSS Position 3 §3.4): the shift of a sticky box towards its
//! insets' thresholds inside its nearest scrollport, clamped to its containing block
//! (miaskiewicz/turbo-dom#216, #218, #295).
//!
//! Layout shifts every sticky box for its unscrolled scrollport ([`StickyState`] tracks
//! the nearest one) and records its [`StickyConstraint`]; a CSSOM-View read under scroll
//! offsets recomputes the shift with the scrolled scrollport
//! (`cssom_view::scroll::sticky_offset`). Both go through [`axis_shift`].
use std::collections::HashMap;

use super::fragment::NodeId;

/// One axis of a sticky box's shift (Blink's `StickyPositionScrollingConstraints`):
/// towards the `start` inset's threshold (the scrollport's start edge plus it) when the
/// box's start edge is before it, else towards the `end` one (the end edge less it) when
/// its end edge is past it, never moving its margin box out of the containing block's
/// content box. `start` wins when both are set.
#[must_use]
pub(crate) fn axis_shift(
    (start, end): (Option<f32>, Option<f32>),
    (lo, hi): (f32, f32),
    (margin_lo, margin_hi): (f32, f32),
    (cb_lo, cb_hi): (f32, f32),
    (view_lo, view_hi): (f32, f32),
) -> f32 {
    if let Some(start) = start {
        let room = (cb_hi - margin_hi - hi).max(0.0);
        return (view_lo + start - lo).clamp(0.0, room);
    }
    let back = (lo - margin_lo - cb_lo).max(0.0);
    end.map_or(0.0, |end| (view_hi - end - hi).clamp(-back, 0.0))
}

/// A sticky box's constraints as layout resolved them, for a scrolled re-read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyConstraint {
    /// The `top`, `right`, `bottom`, `left` insets in px (`None` = `auto`).
    pub insets: [Option<f32>; 4],
    /// The `top`, `right`, `bottom`, `left` margins (`auto` as 0; 0 for an inline).
    pub margins: [f32; 4],
    /// The box whose content box is the containing block.
    pub containing_block: NodeId,
    /// The nearest scroll container (`None` = the viewport).
    pub scroller: Option<NodeId>,
    /// The shift layout applied (`[dx, dy]`), for the unscrolled scrollport.
    pub applied: [f32; 2],
}

impl StickyConstraint {
    /// The `[dx, dy]` shift of a box whose unshifted border box spans `rect`
    /// (`[x0, y0, x1, y1]`) in a containing block content box `cb` and a scrollport
    /// `view` (both `[x0, y0, x1, y1]`).
    #[must_use]
    pub(crate) fn shift(&self, rect: [f32; 4], cb: [f32; 4], view: [f32; 4]) -> [f32; 2] {
        let [top, right, bottom, left] = self.insets;
        let [mt, mr, mb, ml] = self.margins;
        [
            axis_shift(
                (left, right),
                (rect[0], rect[2]),
                (ml, mr),
                (cb[0], cb[2]),
                (view[0], view[2]),
            ),
            axis_shift(
                (top, bottom),
                (rect[1], rect[3]),
                (mt, mb),
                (cb[1], cb[3]),
                (view[1], view[3]),
            ),
        ]
    }
}

/// The scrollport sticky boxes laid out now stick to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StickyScope {
    /// The scroll container's content box `[x0, y0, x1, y1]` (an auto height reaches
    /// `f32::INFINITY`: the box grows to its content, so nothing passes its bottom).
    pub(crate) view: [f32; 4],
    pub(crate) scroller: NodeId,
}

/// Layout's sticky bookkeeping: the nearest scrollport and, on the live path, every
/// sticky box's constraints.
#[derive(Debug, Default)]
pub(crate) struct StickyState {
    /// `None` = the viewport.
    pub(crate) scope: Option<StickyScope>,
    /// How many boxes deep layout is (saturating at 3): the galley root is 0, the root
    /// element 1 and the body 2, whose `overflow` may belong to the viewport instead.
    pub(crate) depth: u8,
    /// Whether the root element's `overflow` is not `visible` (it then belongs to the
    /// viewport, and the body's stays its own).
    pub(crate) root_overflows: bool,
    pub(crate) records: Option<HashMap<NodeId, StickyConstraint>>,
}

impl StickyState {
    pub(crate) fn recording(record: bool) -> StickyState {
        StickyState {
            records: record.then(HashMap::new),
            ..StickyState::default()
        }
    }

    /// Enter a box at the next depth whose `overflow` axes `scrolls` says make it a
    /// scroll container, with content box `view`; returns what [`Self::leave`] restores.
    pub(crate) fn enter(
        &mut self,
        node: NodeId,
        scrolls: bool,
        view: [f32; 4],
    ) -> (Option<StickyScope>, u8) {
        let saved = (self.scope, self.depth);
        // The root element's overflow is the viewport's; so is the body's unless the
        // root's is not `visible` (CSS Overflow 3 §3.3).
        let own = match self.depth {
            0 => false,
            1 => {
                self.root_overflows = scrolls;
                false
            }
            2 => self.root_overflows,
            _ => true,
        };
        if scrolls && own {
            self.scope = Some(StickyScope {
                view,
                scroller: node,
            });
        }
        self.depth = self.depth.saturating_add(1).min(3);
        saved
    }

    pub(crate) fn leave(&mut self, (scope, depth): (Option<StickyScope>, u8)) {
        self.scope = scope;
        self.depth = depth;
    }

    /// The current scrollport: the scope's, else the viewport `(width, height)`.
    pub(crate) fn view(&self, viewport: (f32, f32)) -> ([f32; 4], Option<NodeId>) {
        self.scope
            .map_or(([0.0, 0.0, viewport.0, viewport.1], None), |scope| {
                (scope.view, Some(scope.scroller))
            })
    }

    pub(crate) fn record(&mut self, node: NodeId, constraint: StickyConstraint) {
        if let Some(records) = &mut self.records {
            records.insert(node, constraint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_inset_wins_and_shifts_stay_inside_the_containing_block() {
        // Before the start threshold: moves to it, up to the room left in the block.
        assert_eq!(
            axis_shift(
                (Some(5.0), Some(1.0)),
                (0.0, 10.0),
                (0.0, 2.0),
                (0.0, 30.0),
                (0.0, 50.0)
            ),
            5.0
        );
        assert_eq!(
            axis_shift(
                (Some(40.0), None),
                (0.0, 10.0),
                (0.0, 2.0),
                (0.0, 30.0),
                (0.0, 50.0)
            ),
            18.0
        );
        // Past the end threshold: moves back, no further than the block's start.
        assert_eq!(
            axis_shift(
                (None, Some(5.0)),
                (40.0, 60.0),
                (3.0, 0.0),
                (0.0, 60.0),
                (0.0, 50.0)
            ),
            -15.0
        );
        assert_eq!(
            axis_shift(
                (None, Some(45.0)),
                (40.0, 60.0),
                (3.0, 0.0),
                (0.0, 60.0),
                (0.0, 50.0)
            ),
            -37.0
        );
        assert_eq!(
            axis_shift(
                (None, None),
                (40.0, 60.0),
                (0.0, 0.0),
                (0.0, 60.0),
                (0.0, 50.0)
            ),
            0.0
        );
    }

    #[test]
    fn the_root_and_a_propagated_body_overflow_belong_to_the_viewport() {
        let view = [1.0, 2.0, 3.0, 4.0];
        let mut state = StickyState::recording(false);
        let galley = state.enter(NodeId(0), true, view);
        let root = state.enter(NodeId(1), false, view);
        let body = state.enter(NodeId(2), true, view);
        assert_eq!(state.view((9.0, 8.0)), ([0.0, 0.0, 9.0, 8.0], None));
        let deep = state.enter(NodeId(3), true, view);
        assert_eq!(state.view((9.0, 8.0)), (view, Some(NodeId(3))));
        state.enter(NodeId(4), false, [0.0; 4]);
        assert_eq!(state.depth, 3);
        state.leave(deep);
        state.leave(body);
        state.leave(root);
        // A scrolling root keeps the body's overflow the body's own.
        state.enter(NodeId(1), true, view);
        assert_eq!(state.view((9.0, 8.0)).1, None);
        state.enter(NodeId(2), true, view);
        assert_eq!(state.view((9.0, 8.0)).1, Some(NodeId(2)));
        state.leave(galley);
        assert_eq!(state.depth, 0);
        state.record(
            NodeId(5),
            StickyConstraint {
                insets: [None; 4],
                margins: [0.0; 4],
                containing_block: NodeId(0),
                scroller: None,
                applied: [0.0; 2],
            },
        );
        assert!(state.records.is_none());
    }
}

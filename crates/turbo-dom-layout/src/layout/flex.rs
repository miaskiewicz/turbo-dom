//! Flex layout (Â§5.3, AC-5.6). `taffy` owns the flexbox math (direction, wrap,
//! grow/shrink/basis, justify/align, gap); we map CSS to `taffy::Style`, feed it
//! each item's measured size, read back rects, then re-lay each item's content at
//! its assigned width (Â§5.3 decision: taffy owns flex, the engine owns the rest).
//!
//! Content sizing: an item's main size comes from its `flex-basis`/`width` when
//! set, otherwise from a max-content measurement of its content (`natural_width`);
//! the cross size comes from laying the content out at the proposed width. The
//! item's typed padding, borders and margins remain with taffy until their containing
//! block is known. The measurement bridge converts native border boxes to taffy
//! content measurements without applying the frame twice.

use std::collections::HashMap;
#[path = "flex_tree.rs"]
mod native_tree;
use native_tree::Tree as TaffyTree;

#[path = "grid_tracks.rs"]
mod grid_tracks;

#[path = "flex_frame.rs"]
mod frames;

use taffy::prelude::{FromLength, FromPercent, TaffyAuto, TaffyMaxContent, TaffyMinContent};
use taffy::style_helpers::{fr, line, minmax, percent, span};
use taffy::{
    AlignItems, AvailableSpace, Dimension, Display, FlexDirection, FlexWrap, GridPlacement,
    GridTrackRepetition, JustifyContent, Layout, LengthPercentage, LengthPercentageAuto, Line,
    MaxTrackSizingFunction, MinTrackSizingFunction, NodeId as TaffyId,
    NonRepeatedTrackSizingFunction, Position, Rect, Size, Style, TrackSizingFunction,
};

use crate::error::Diagnostics;
use crate::style::ComputedStyle;
use crate::text::FontRegistry;

use super::block::{self, Ctx};
use super::boxgen::{BoxKind, InlineItem, LayoutBox};
use super::fragment::Fragment;
use super::inline;
use super::value::{
    component_tokens as basis_tokens, parse_px, BoxSizing, BoxStyle, LengthPct, ResolveCtx,
    DEFAULT_FONT_SIZE,
};
use super::ImageCtx;

// --------------------------------------------------------------------------
// CSS -> taffy style mapping
// --------------------------------------------------------------------------

fn flex_direction(s: &ComputedStyle) -> FlexDirection {
    match s.get("flex-direction").unwrap_or("row").trim() {
        "row-reverse" => FlexDirection::RowReverse,
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Row,
    }
}

fn flex_wrap(s: &ComputedStyle) -> FlexWrap {
    match s.get("flex-wrap").unwrap_or("nowrap").trim() {
        "wrap" => FlexWrap::Wrap,
        "wrap-reverse" => FlexWrap::WrapReverse,
        _ => FlexWrap::NoWrap,
    }
}

fn justify_content_value(v: &str) -> JustifyContent {
    match v.trim() {
        "flex-end" | "end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::FlexStart,
    }
}

#[allow(clippy::unnecessary_wraps)] // keeps the Option/Result shape its call sites (and sibling helpers) share
fn justify_content(s: &ComputedStyle) -> Option<JustifyContent> {
    Some(justify_content_value(
        s.get("justify-content").unwrap_or("flex-start"),
    ))
}

/// A grid's `justify-content`, `None` when unset. Unlike flex (whose initial is
/// `flex-start`), CSS grid's initial `normal` acts as `stretch`: a single auto
/// column then fills the container so `justify-items` can center within it. Leaving
/// it `None` hands taffy that stretch default — forcing `FlexStart` instead pinned
/// google's home logo track to its content width at the left padding edge.
fn justify_content_grid(s: &ComputedStyle) -> Option<JustifyContent> {
    s.get("justify-content").map(justify_content_value)
}

#[allow(clippy::unnecessary_wraps)] // keeps the Option/Result shape its call sites (and sibling helpers) share
fn align_items(s: &ComputedStyle) -> Option<AlignItems> {
    Some(match s.get("align-items").unwrap_or("stretch").trim() {
        "flex-start" | "start" => AlignItems::FlexStart,
        "flex-end" | "end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        _ => AlignItems::Stretch,
    })
}

/// CSS `justify-items` (a grid's inline-axis item alignment) → taffy. A grid item
/// defaults to `stretch`; `center`/`start`/`end` instead shrink it to its content
/// and place it in the track. Flex has no `justify-items`, so this is grid-only
/// (google's home page centers its full-width-cell logo with `justify-items:center`
/// — without mapping it the 272px logo pinned to the cell's left/padding edge).
#[allow(clippy::unnecessary_wraps)] // keeps the Option/Result shape its call sites (and sibling helpers) share
fn justify_items(s: &ComputedStyle) -> Option<AlignItems> {
    Some(match s.get("justify-items").unwrap_or("stretch").trim() {
        "start" | "flex-start" | "left" => AlignItems::Start,
        "end" | "flex-end" | "right" => AlignItems::End,
        "center" => AlignItems::Center,
        _ => AlignItems::Stretch,
    })
}

/// CSS `justify-self` (a grid item's own inline-axis alignment, overriding the
/// container's `justify-items`). `None`/`auto` defers to the container. A non-stretch
/// value shrinks the item to its own (measured/explicit) inline size and places it in
/// the track — without mapping it a `width:80px; justify-self:center` cell inherited
/// the default `stretch` and filled its whole track instead of centering (google's
/// home logo cell).
fn justify_self(s: &ComputedStyle) -> Option<AlignItems> {
    match s.get("justify-self").map(str::trim) {
        None | Some("auto") => None,
        Some("start" | "flex-start" | "left") => Some(AlignItems::Start),
        Some("end" | "flex-end" | "right") => Some(AlignItems::End),
        Some("center") => Some(AlignItems::Center),
        Some(_) => Some(AlignItems::Stretch),
    }
}

/// A flex item's `align-self` (its own cross-axis alignment, overriding the
/// container's `align-items`), `None`/`auto` deferring to the container. Google's
/// search-bar "AI Mode" pill, the "Sign in" button and the two search buttons all
/// carry `align-self:center` with a fixed height — without it they inherited the
/// default `stretch`/top and rode the top edge of their taller row (the Sign-in pill
/// stretched to the header height and rendered as a circle).
fn align_self(s: &ComputedStyle) -> Option<AlignItems> {
    match s.get("align-self").map(str::trim) {
        None | Some("auto") => None,
        Some("flex-start" | "start") => Some(AlignItems::FlexStart),
        Some("flex-end" | "end") => Some(AlignItems::FlexEnd),
        Some("center") => Some(AlignItems::Center),
        Some("baseline") => Some(AlignItems::Baseline),
        Some(_) => Some(AlignItems::Stretch),
    }
}

/// One gap axis of the `gap` shorthand (`gap: <row> <column>`, CSS Box Alignment 3
/// §8.3): `column` picks the second value, defaulting to the first.
fn gap_shorthand(s: &ComputedStyle, column: bool) -> Option<f32> {
    let v = s.get("gap")?;
    let mut parts = v.split_whitespace();
    let row = parts.next()?;
    let tok = if column {
        parts.next().unwrap_or(row)
    } else {
        row
    };
    parse_px(tok, DEFAULT_FONT_SIZE)
}

fn container_style(container: &LayoutBox, cw: f32, fs: f32, known_height: Option<f32>) -> Style {
    let s = &container.style;
    let bs = container.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: cw,
    });
    let inset = match bs.box_sizing {
        BoxSizing::BorderBox => bs.padding.vertical() + bs.border.widths().vertical(),
        BoxSizing::ContentBox => 0.0,
    };
    // This Taffy root has no padding/border: native block layout already owns
    // that frame. Prefer its actual definite content height (including flexed or
    // percentage-resolved sizes), and convert declared constraints to this space.
    let container_h =
        known_height.map_or_else(|| dim(content_height(bs.height, inset)), Dimension::length);
    Style {
        display: Display::Flex,
        flex_direction: flex_direction(s),
        flex_wrap: flex_wrap(s),
        justify_content: justify_content(s),
        align_items: align_items(s),
        gap: Size {
            width: gap_axis(s, "column-gap"),
            height: gap_axis(s, "row-gap"),
        },
        // Carry the container's own `height` and `min/max-height` to taffy: a flex
        // box with only a `min-height` (a search bar's `min-height:50px`, a hero
        // banner) must not collapse to its content height. Width stays the given
        // content width.
        size: Size {
            width: Dimension::length(cw),
            height: container_h,
        },
        min_size: Size {
            width: Dimension::auto(),
            height: dim(content_height(bs.min_height, inset)),
        },
        max_size: Size {
            width: Dimension::auto(),
            height: dim(content_height(bs.max_height, inset)),
        },
        ..Default::default()
    }
}

fn num(s: &ComputedStyle, prop: &str, default: f32) -> f32 {
    s.get(prop)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

/// The `(flex-grow, flex-shrink)` implied by the `flex` shorthand, so a `flex:1`
/// item grows to fill without an explicit `flex-grow` longhand (google's search box
/// input `flex:1` collapsed, cramming its trailing icons to the left). Defaults to
/// the CSS initial `(0, 1)`; the explicit longhands still override in `item_style`.
fn flex_grow_shrink(s: &ComputedStyle) -> (f32, f32) {
    let Some(v) = s.get("flex") else {
        return (0.0, 1.0);
    };
    match v.trim() {
        "none" => (0.0, 0.0),
        "auto" => (1.0, 1.0),
        "initial" => (0.0, 1.0),
        // `<grow> [<shrink>] [<basis>]`: the leading unitless numbers are grow/shrink
        // (a `<basis>` length/% carries a unit and is skipped by the number parse).
        rest => {
            let nums: Vec<f32> = basis_tokens(rest)
                .into_iter()
                .filter_map(|t| t.parse().ok())
                .collect();
            (
                nums.first().copied().unwrap_or(1.0),
                nums.get(1).copied().unwrap_or(1.0),
            )
        }
    }
}

/// A flex item's main-size basis. `flex-basis:auto` (the initial value) defers to
/// the item's main size `main` — its `width` in a row, its `height` in a column
/// (miaskiewicz/turbo-dom#86) — without that fallback a `width:100%` flex item (e.g.
/// Wikipedia's `.mw-header`, a `width:100%` grid that is itself a flex child)
/// shrink-wrapped to its content instead of filling the row.
fn item_basis(s: &ComputedStyle, main: LengthPct, fs: f32) -> Dimension {
    match s.get("flex-basis").map(str::trim) {
        // No `flex-basis` longhand: the basis may still live in the `flex`
        // shorthand's third component (`flex: 0 0 60px`), which the grow/shrink
        // parse skips. Fall back to it before defaulting to the main size.
        None | Some("auto") => flex_shorthand_basis(s, fs).unwrap_or_else(|| dim(main)),
        Some("content") => Dimension::auto(),
        Some(b) => parse_basis(b, fs).unwrap_or_else(|| dim(main)),
    }
}

/// The `<basis>` component of the `flex` shorthand (`flex: <grow> <shrink>?
/// <basis>?`), or `None` when the shorthand is absent or carries no basis:
/// `none`→`auto`, a length/`%` token→that size, `content`→content, a bare
/// number-only shorthand (`flex: 1`)→`0` (the CSS `flex: 1` == `1 1 0%`), and
/// an explicit `auto` token→`None` so the caller falls back to `width`.
fn flex_shorthand_basis(s: &ComputedStyle, fs: f32) -> Option<Dimension> {
    let v = s.get("flex").map(str::trim)?;
    if matches!(v, "none" | "initial") {
        return None;
    }
    let mut saw_token = false;
    for tok in basis_tokens(v) {
        saw_token = true;
        match token_basis(tok, fs) {
            TokenBasis::DeferToWidth => return None,
            TokenBasis::Skip => {}
            TokenBasis::Basis(d) => return Some(d),
        }
    }
    // A numbers-only shorthand (`flex: 1`, `flex: 2 0`) has an implied `0` basis.
    saw_token.then(|| Dimension::percent(0.0))
}

/// One `flex`-shorthand token's role in basis resolution.
enum TokenBasis {
    /// A bare number — grow/shrink, not the basis.
    Skip,
    /// An explicit `auto` — basis defers to `width`.
    DeferToWidth,
    /// A length/`%`/`content` basis.
    Basis(Dimension),
}

fn token_basis(tok: &str, fs: f32) -> TokenBasis {
    if tok == "auto" {
        return TokenBasis::DeferToWidth;
    }
    if tok.parse::<f32>().is_ok() {
        return TokenBasis::Skip; // a bare number is grow/shrink
    }
    match parse_basis(tok, fs) {
        Some(d) => TokenBasis::Basis(d),
        None => TokenBasis::Skip,
    }
}

/// Parse a single `flex-basis`/basis token (`60px`, `50%`, `content`) into a taffy
/// [`Dimension`]; `None` for `auto`/unparsable (caller falls back to `width`).
fn parse_basis(b: &str, fs: f32) -> Option<Dimension> {
    if b == "content" {
        return Some(Dimension::auto());
    }
    if let Some(px) = parse_px(b, fs) {
        return Some(Dimension::length(px));
    }
    super::value::parse_length_pct(b, fs).map(dim)
}

/// A flex (or grid: `column` false) item's taffy style; `column`: the container's main
/// axis is vertical.
fn item_style(item: &LayoutBox, fs: f32, column: bool) -> Style {
    let s = &item.style;
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: 0.0,
    });
    // A `position:absolute`/`fixed` child is out of flow: taffy must NOT treat it as
    // a flex item (else `align-items:stretch` in a column flex fills it to the
    // container width, ignoring its own width â the Codex radio's absolute icon blew
    // up from 18px to the whole row). Mark it absolute so taffy sizes it from its
    // width/height + insets and excludes it from the flex line.
    if bs.position.is_out_of_flow() {
        return Style {
            position: Position::Absolute,
            inset: abs_item_inset(&bs),
            size: item_size(&bs),
            ..Default::default()
        };
    }
    let (grow, shrink) = flex_grow_shrink(s);
    let main = if column { bs.height } else { bs.width };
    Style {
        flex_grow: num(s, "flex-grow", grow),
        flex_shrink: num(s, "flex-shrink", shrink),
        flex_basis: item_basis(s, main, fs),
        align_self: align_self(s),
        // A flex item's own `min/max` sizes and explicit cross-axis size (`height` in a
        // row, `width` in a column) must reach taffy — else an item whose height is only
        // a `min-height` collapses, and a column item stretches over its `width`.
        // Preferred width also caps the automatic minimum; flex-basis still drives flexing.
        size: Size {
            width: dim(bs.width),
            height: if column {
                Dimension::auto()
            } else {
                dim(bs.height)
            },
        },
        min_size: Size {
            width: dim(bs.min_width),
            height: dim(bs.min_height),
        },
        max_size: Size {
            width: dim(bs.max_width),
            height: dim(bs.max_height),
        },
        ..Default::default()
    }
}

/// A `LengthPct` â taffy `Dimension` (`auto` when not a fixed/percentage length).
fn dim(lp: LengthPct) -> Dimension {
    match lp {
        LengthPct::Px(v) => Dimension::length(v),
        LengthPct::Pct(p) => Dimension::percent(p / 100.0),
        LengthPct::Calc { pct, px } => Dimension::calc(native_tree::calc(pct, px)),
        LengthPct::Auto => Dimension::auto(),
    }
}

/// The width/height of an out-of-flow flex child as a taffy `Size`.
fn item_size(bs: &BoxStyle) -> Size<Dimension> {
    Size {
        width: dim(bs.width),
        height: dim(bs.height),
    }
}

/// The `inset` (top/right/bottom/left) of an out-of-flow flex child; `auto` edges
/// let taffy keep the item at its static position on that axis.
fn item_inset(bs: &BoxStyle) -> Rect<LengthPercentageAuto> {
    let edge = |lp: LengthPct| match lp {
        LengthPct::Px(v) => LengthPercentageAuto::length(v),
        LengthPct::Pct(p) => LengthPercentageAuto::percent(p / 100.0),
        LengthPct::Calc { pct, px } => LengthPercentageAuto::calc(native_tree::calc(pct, px)),
        LengthPct::Auto => LengthPercentageAuto::auto(),
    };
    Rect {
        left: edge(bs.inset_left),
        right: edge(bs.inset_right),
        top: edge(bs.inset_top),
        bottom: edge(bs.inset_bottom),
    }
}

/// The inset for an absolutely-positioned flex child, but with a fully-`auto`
/// horizontal axis anchored to the start (`left: 0`) — its STATIC in-flow origin —
/// instead of letting taffy CENTER it via the container's `justify-content`. Google's
/// "AI Mode" pill absolutely-positions its sparkle icon with no insets; centered on
/// the main axis it printed over the label. The (cross) vertical axis is left `auto`
/// so `align-items` still centers it; an explicit inset on either edge is kept.
fn abs_item_inset(bs: &BoxStyle) -> Rect<LengthPercentageAuto> {
    let mut inset = item_inset(bs);
    if bs.inset_left == LengthPct::Auto && bs.inset_right == LengthPct::Auto {
        inset.left = LengthPercentageAuto::length(0.0);
    }
    inset
}

// --------------------------------------------------------------------------
// content measurement
// --------------------------------------------------------------------------

/// The max-content width contributed by an inline run's atomic items
/// (`inline-block`/replaced boxes). `build_runs` keeps only text pieces, so a line
/// whose content is an atomic measured 0 — google's footer wraps its "Settings"
/// link in an `inline-block` button, and that whole footer cell collapsed and its
/// text overflowed past the viewport. Atomics sit inline (side by side), so their
/// natural widths sum, like the text runs they share the line with.
fn atomics_natural(items: &[InlineItem], fonts: &FontRegistry) -> f32 {
    items
        .iter()
        .filter_map(|it| match it {
            InlineItem::Atomic(b) if !b.out_of_flow() => {
                Some(margin_box(b, natural_width(b, fonts)))
            }
            _ => None,
        })
        .sum()
}

/// The inline boxes' own horizontal margins, borders and padding in a run laid on one
/// line (miaskiewicz/turbo-dom#244): each start boundary carries the edges its fragment
/// keeps ([`crate::ComputedStyle::inline_fragment_style`] zeroes a split side).
fn inline_edges_natural(items: &[InlineItem], fs: f32) -> f32 {
    items
        .iter()
        .filter_map(|it| match it {
            InlineItem::Boundary {
                style, start: true, ..
            } => {
                let used = super::value::resolve_box_style(
                    style,
                    ResolveCtx {
                        parent_font_size: fs,
                        cb_width: 0.0,
                    },
                );
                Some(
                    used.margin.left
                        + used.margin.right
                        + used.padding.horizontal()
                        + used.border.widths().horizontal(),
                )
            }
            _ => None,
        })
        .sum()
}

fn lines_natural(
    items: &[InlineItem],
    fs: f32,
    fonts: &FontRegistry,
    original: Option<&crate::layout_snapshot::text_units::NativeUnitMap>,
    policy: (bool, Option<f32>),
) -> f32 {
    // A forced break (`<br>`) ends a line: the max-content width is the widest line's
    // (Chrome 156: `sticky-inline-inline-block-*`, miaskiewicz/turbo-dom#295).
    items
        .split(|item| matches!(item, InlineItem::LineBreak | InlineItem::SourceBreak { .. }))
        .map(|line| line_natural(line, fs, fonts, original, policy))
        .fold(0.0, f32::max)
}

/// The max-content width of one forced line of inline items.
fn line_natural(
    items: &[InlineItem],
    fs: f32,
    fonts: &FontRegistry,
    original: Option<&crate::layout_snapshot::text_units::NativeUnitMap>,
    (html_metrics, scale): (bool, Option<f32>),
) -> f32 {
    let runs = block::build_runs(items, fs, 0.0, fonts);
    let mut scratch = Diagnostics::default();
    // The legacy PDF path rounds max-content width UP: a shrink-wrapped fractional width
    // and then re-laid at that width can't fit the fractional last glyph, so the run
    // wraps a word onto a second line even though it "fit" the measurement (nike's
    // "Find a Store" utility links, google's footer). A whole-pixel max-content leaves
    // room for the rounding.
    let text = inline::measure_runs_with_units(
        &runs,
        fonts,
        f32::MAX,
        &mut scratch,
        original,
        (html_metrics, scale),
    );
    let text = if html_metrics { text } else { text.ceil() };
    text + atomics_natural(items, fonts) + inline_edges_natural(items, fs)
}

/// The widest in-flow child's max-content contribution: its margin box
/// (miaskiewicz/turbo-dom#79); out-of-flow children contribute nothing (#84).
fn kids_natural(kids: &[LayoutBox], fonts: &FontRegistry) -> f32 {
    in_flow(kids)
        .map(|k| margin_box(k, natural_width(k, fonts)))
        .fold(0.0_f32, f32::max)
}

/// A block container's max-content width (miaskiewicz/turbo-dom#85): consecutive floats
/// sit side by side, so their margin boxes add up (per side, a `clear` closing the side's
/// run); an in-flow block ends the run, and one that avoids floats (a formatting-context
/// root) sits beside it (its contribution adds the run). Otherwise the widest margin box.
fn block_natural(kids: &[LayoutBox], fonts: &FontRegistry) -> f32 {
    let mut run = FloatRun::default();
    for k in in_flow(kids) {
        let w = margin_box(k, natural_width(k, fonts));
        run.clear(clear_sides(k));
        if k.floated() {
            run.float(super::value::float_of(&k.style), w);
        } else {
            run.block(w, block::avoids_floats(k));
        }
    }
    run.end()
}

/// The `clear` sides of a box: (left, right).
fn clear_sides(k: &LayoutBox) -> (bool, bool) {
    match k.style.get("clear").map(str::trim) {
        Some("both") => (true, true),
        Some("left") => (true, false),
        Some("right") => (false, true),
        _ => (false, false),
    }
}

/// [`block_natural`]'s running state: the widest line so far and the current run of
/// floats on each side.
#[derive(Default)]
struct FloatRun {
    widest: f32,
    left: f32,
    right: f32,
}

impl FloatRun {
    /// A `clear` ends the cleared sides' runs.
    fn clear(&mut self, (left, right): (bool, bool)) {
        if left || right {
            self.widest = self.widest.max(self.left + self.right);
        }
        if left {
            self.left = 0.0;
        }
        if right {
            self.right = 0.0;
        }
    }

    /// A float of margin-box width `w` joins its side's run.
    fn float(&mut self, side: super::value::Float, w: f32) {
        match side {
            super::value::Float::Right => self.right += w,
            _ => self.left += w,
        }
    }

    /// An in-flow block of margin-box width `w` ends the run; one that avoids floats
    /// (`beside`) sits beside it.
    fn block(&mut self, w: f32, beside: bool) {
        let floats = self.left + self.right;
        let line = if beside { w + floats } else { w };
        self.widest = self.widest.max(floats).max(line);
        (self.left, self.right) = (0.0, 0.0);
    }

    fn end(&self) -> f32 {
        self.widest.max(self.left + self.right)
    }
}

/// The in-flow children of a box: absolutely positioned and fixed ones are out of flow
/// and add nothing to its intrinsic widths (miaskiewicz/turbo-dom#84).
fn in_flow(kids: &[LayoutBox]) -> impl Iterator<Item = &LayoutBox> {
    kids.iter().filter(|k| !k.out_of_flow())
}

/// A child's intrinsic size contribution: its border-box width `w` plus its horizontal
/// margins — signed (a negative margin shrinks it), `%` and `auto` margins counting 0
/// (resolved against no width), as Chrome 156 sizes floats, `width: auto` positioned and
/// inline-block boxes and table cells (miaskiewicz/turbo-dom#79). The caller's fold
/// floors the result at 0.
fn margin_box(k: &LayoutBox, w: f32) -> f32 {
    let bs = k.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    w + bs.margin.left + bs.margin.right
}

/// Max-content width of a flex container. A **row** flex lays its items side by
/// side, so its max-content is the SUM of the items' widths plus the column gaps
/// (not the widest item, as for block/column) â otherwise a shrink-to-fit row flex
/// collapses to one item's width and its children wrap/stack (Wikipedia's centered
/// header did exactly this). A **column** flex stacks, so max = widest item.
fn flex_natural(kids: &[LayoutBox], s: &ComputedStyle, fonts: &FontRegistry) -> f32 {
    if matches!(
        flex_direction(s),
        FlexDirection::Row | FlexDirection::RowReverse
    ) {
        let gap = s
            .get("column-gap")
            .or_else(|| s.get("gap"))
            .and_then(|v| parse_px(v, DEFAULT_FONT_SIZE))
            .unwrap_or(0.0);
        // Each item contributes its border-box natural width PLUS its horizontal
        // margins â the items are laid side by side including those margins, so
        // omitting them undersizes the row and its children overflow (Wikipedia's
        // page-action tabs are margin-spaced; the row measured short and "View
        // history" spilled past the toolbar into the next column).
        let sum: f32 = in_flow(kids)
            .map(|k| natural_width(k, fonts) + item_hmargin(k))
            .sum();
        sum + gap * in_flow(kids).count().saturating_sub(1) as f32
    } else {
        kids_natural(kids, fonts)
    }
}

/// A grid's max-content width: its explicit `grid-template-columns` when every track
/// is a `px` length (their sum plus the column gaps; miaskiewicz/turbo-dom#53, an
/// `inline-grid` shrinks to it), else its widest item (one implicit column).
fn grid_natural(kids: &[LayoutBox], s: &ComputedStyle, fonts: &FontRegistry) -> f32 {
    let tracks: Option<Vec<f32>> = s.get("grid-template-columns").and_then(|columns| {
        columns
            .split_whitespace()
            .map(|track| parse_px(track, DEFAULT_FONT_SIZE))
            .collect()
    });
    let Some(tracks) = tracks.filter(|tracks| !tracks.is_empty()) else {
        return kids_natural(kids, fonts);
    };
    let gap = s
        .get("column-gap")
        .or_else(|| s.get("gap").and_then(|gap| gap.split_whitespace().last()))
        .and_then(|v| parse_px(v, DEFAULT_FONT_SIZE))
        .unwrap_or(0.0);
    tracks.iter().sum::<f32>() + gap * (tracks.len() - 1) as f32
}

/// The item's positive horizontal margins (px), which sit between it and its flex
/// siblings. Negative/auto margins contribute nothing to the intrinsic width.
fn item_hmargin(k: &LayoutBox) -> f32 {
    let bs = k.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    bs.margin.left.max(0.0) + bs.margin.right.max(0.0)
}

/// A box's border-box width `w`, widened for a table wrapper box to its widest caption's
/// min-content (miaskiewicz/turbo-dom#51).
fn caption_floored(lb: &LayoutBox, w: f32, fonts: &FontRegistry) -> f32 {
    match &lb.kind {
        BoxKind::Table(items) => super::table::caption_min(items, fonts).map_or(w, |c| w.max(c)),
        _ => w,
    }
}

/// Seed only a text leaf's existing numeric caches. Wrappers retain their own
/// ordinary sizing arithmetic; no prepared data survives a query or policy change.
pub(crate) fn prepare_text_intrinsics(lb: &LayoutBox, fonts: &FontRegistry) {
    if !lb.intrinsic_widths_uncached() || lb.out_of_flow() {
        return;
    }
    let bs = lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    if matches!(bs.width, LengthPct::Px(_)) || lb.containment().width {
        return;
    }
    if lb.is_replaced() || (lb.orthogonal_root && lb.writing_root) {
        return;
    }
    match &lb.kind {
        BoxKind::Block(children) if children.len() == 1 => {
            prepare_text_intrinsics(&children[0], fonts);
        }
        BoxKind::Lines(items) if items.iter().all(|i| matches!(i, InlineItem::Text { .. })) => {
            prepare_lines_intrinsics(
                lb,
                items,
                bs.font_size,
                fonts,
                bs.padding.horizontal() + bs.border.widths().horizontal(),
            );
        }
        _ => {}
    }
}

fn prepare_lines_intrinsics(
    lb: &LayoutBox,
    items: &[InlineItem],
    font_size: f32,
    fonts: &FontRegistry,
    frame: f32,
) {
    let runs = block::build_runs(items, font_size, 0.0, fonts);
    let html = lb.uses_html_font_metrics();
    let (max, min) = inline::measure_intrinsic_pair(
        &runs,
        fonts,
        lb.original_units().as_deref(),
        (html, lb.native_scale()),
    );
    let max = if html { max } else { max.ceil() };
    lb.seed_intrinsic_widths(max.max(0.0) + frame, min.max(0.0) + frame);
}

pub(crate) fn natural_width(lb: &LayoutBox, fonts: &FontRegistry) -> f32 {
    crate::hot!("layout.natural_width");
    lb.natural_cached(|| {
        let bs = lb.resolved(ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        });
        let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
        if let LengthPct::Px(w) = bs.width {
            return caption_floored(lb, block::to_border(w, bs.box_sizing, frame), fonts);
        }
        intrinsic_natural_width(lb, &bs, fonts)
    })
}

fn intrinsic_natural_width(lb: &LayoutBox, bs: &BoxStyle, fonts: &FontRegistry) -> f32 {
    let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
    if lb.containment().width {
        // Size containment (miaskiewicz/turbo-dom#265): sized as if empty.
        return frame;
    }
    if lb.orthogonal_root && lb.writing_root {
        return super::writing::orthogonal_extent(lb, fonts);
    }
    // A replaced `<img>`: its intrinsic width (stamped from the resolver before
    // layout) â so a `<div>` wrapping a logo measures the image, not 0.
    if let Some(iw) = lb.intrinsic_w() {
        return replaced_natural(lb, bs, iw) + frame;
    }
    let inner = match &lb.kind {
        BoxKind::Lines(items) => lines_natural(
            items,
            bs.font_size,
            fonts,
            lb.original_units().as_deref(),
            (lb.uses_html_font_metrics(), lb.native_scale()),
        ),
        BoxKind::Flex(k) => flex_natural(k, &lb.style, fonts),
        // A table's max-content: its columns side by side plus border-spacing
        // (miaskiewicz/turbo-dom#28 item 4), not its widest row group.
        BoxKind::Table(k) => super::table::max_content_width(k, &lb.style, fonts),
        BoxKind::Grid(k) => grid_natural(k, &lb.style, fonts),
        BoxKind::Block(k) => block_natural(k, fonts),
        BoxKind::Marker(_) => 0.0,
    };
    inner + frame
}

/// A replaced box's max-content (content-box) width from its intrinsic width `iw`: a
/// fixed `height` transfers through the intrinsic aspect ratio when the box has one
/// (an image, a canvas, a loaded poster; not an iframe or a media default box). In a vertical space
/// this is how a fixed physical `width` sizes the line (miaskiewicz/turbo-dom#252).
fn replaced_natural(lb: &LayoutBox, bs: &super::value::BoxStyle, iw: f32) -> f32 {
    let ratio = lb.image.as_ref().is_some_and(|source| {
        source.media != super::boxgen::Media::Viewport || !source.unloaded.get()
    }) && !lb.native_iframe();
    match (bs.height, lb.intrinsic_h()) {
        (LengthPct::Px(h), Some(ih)) if ratio && ih > 0.0 => {
            let h = match bs.box_sizing {
                super::value::BoxSizing::BorderBox => {
                    (h - bs.padding.vertical() - bs.border.widths().vertical()).max(0.0)
                }
                super::value::BoxSizing::ContentBox => h,
            };
            h * iw / ih
        }
        _ => iw,
    }
}

/// The widest single unbreakable piece (max word / longest line at zero available
/// width) of an inline run â the text's min-content width.
fn lines_min(
    items: &[InlineItem],
    fs: f32,
    fonts: &FontRegistry,
    original: Option<&crate::layout_snapshot::text_units::NativeUnitMap>,
    (html_metrics, scale): (bool, Option<f32>),
) -> f32 {
    let runs = block::build_runs(items, fs, 0.0, fonts);
    let mut scratch = Diagnostics::default();
    let text = inline::measure_runs_with_units(
        &runs,
        fonts,
        0.0,
        &mut scratch,
        original,
        (html_metrics, scale),
    );
    // An atomic doesn't break, so it contributes its own min-content; the line can
    // wrap between pieces, so its min-content is the widest single piece.
    let atom = items
        .iter()
        .filter_map(|it| match it {
            InlineItem::Atomic(b) if !b.out_of_flow() => {
                Some(margin_box(b, min_content_width(b, fonts)))
            }
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    text.max(atom)
}

/// The **min-content** width of a box: the least width it can take without its
/// content overflowing. Text wraps to its widest word; a fixed/`%`-width box and a
/// replaced image keep their declared size; a container is the widest child's
/// min-content. Used so a table never shrinks a column below its content.
pub(crate) fn min_content_width(lb: &LayoutBox, fonts: &FontRegistry) -> f32 {
    lb.min_content_cached(|| {
        let bs = lb.resolved(ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        });
        let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
        if let LengthPct::Px(w) = bs.width {
            return caption_floored(lb, block::to_border(w, bs.box_sizing, frame), fonts);
        }
        if lb.containment().width {
            // Size containment (miaskiewicz/turbo-dom#265): sized as if empty.
            return frame;
        }
        if lb.is_replaced() || (lb.orthogonal_root && lb.writing_root) {
            // A replaced image: its intrinsic / declared size; an orthogonal root: its
            // laid-out block size.
            return natural_width(lb, fonts);
        }
        let inner = match &lb.kind {
            BoxKind::Lines(items) => lines_min(
                items,
                bs.font_size,
                fonts,
                lb.original_units().as_deref(),
                (lb.uses_html_font_metrics(), lb.native_scale()),
            ),
            BoxKind::Table(k) => super::table::min_content_width(k, &lb.style, fonts),
            BoxKind::Flex(k) | BoxKind::Block(k) | BoxKind::Grid(k) => in_flow(k)
                .map(|c| margin_box(c, min_content_width(c, fonts)))
                .fold(0.0_f32, f32::max),
            BoxKind::Marker(_) => 0.0,
        };
        inner + frame
    })
}

fn measure_width(
    known: Option<f32>,
    avail: AvailableSpace,
    item: &LayoutBox,
    fonts: &FontRegistry,
) -> f32 {
    match (known, avail) {
        (Some(w), _) => w,
        // Cross-axis width sizing: max-content capped by the offer, NOT the full offer
        // (returning the offer made an align-items:center item fill and never center
        // — a logo column left-aligned; taffy stretches for align:stretch itself).
        (None, AvailableSpace::Definite(w)) => natural_width(item, fonts).min(w),
        (None, AvailableSpace::MinContent) => min_content_width(item, fonts),
        (None, _) => natural_width(item, fonts),
    }
}

/// How a replaced `<img>` item's measurement width is chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Probe {
    /// Its explicit `width` or intrinsic pixel width, plus the box frame.
    Fixed(f32),
    /// An image with a natural ratio and no natural size (an SVG with only a
    /// `viewBox`) and an auto `width`: it fills the width it is offered
    /// (miaskiewicz/turbo-dom#326); `ratio` is its natural width / height.
    RatioOnly { ratio: f32 },
}

/// The max-content width of a replaced `<img>` flex item: its explicit `width`, else
/// the intrinsic pixel width from the resolver (plus the box frame). `None` for a
/// non-replaced box (the caller falls back to the content measurement). Without this
/// an `<img>` with `width:auto` measured 0 in a flex row â its `max-width:100%` clamps
/// against a 0-width containing block in the scratch measurement â and the item (a
/// logo / hero image) collapsed to nothing.
fn replaced_probe(item: &LayoutBox, images: &ImageCtx, fs: f32) -> Option<Probe> {
    if let Some(viewport) = &item.svg_viewport {
        return Some(Probe::Fixed(svg_probe_width(item, viewport, fs)));
    }
    let src = item.image.as_ref().filter(|s| s.replaced)?;
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: 0.0,
    });
    let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
    if let LengthPct::Px(w) = bs.width {
        return Some(Probe::Fixed(block::to_border(w, bs.box_sizing, frame)));
    }
    let (intrinsic, _) = super::imgsize::replaced_intrinsic(images.resolver, src, &item.style)?;
    Some(match intrinsic.natural_ratio {
        Some(ratio) => Probe::RatioOnly { ratio },
        None => Probe::Fixed(intrinsic.width as f32 + frame),
    })
}

/// Whether `item` is a replaced image with a natural ratio and no natural size.
fn is_ratio_only(item: &LayoutBox, images: &ImageCtx) -> bool {
    item.image
        .as_ref()
        .filter(|s| s.replaced && item.svg_viewport.is_none())
        .and_then(|src| super::imgsize::replaced_intrinsic(images.resolver, src, &item.style))
        .is_some_and(|(intrinsic, _)| intrinsic.natural_ratio.is_some())
}

/// The container an item is measured in: its inner (content-box) `width`, and whether
/// it is a row flex container (a column flex or a grid container: `false`).
#[derive(Clone, Copy, Debug)]
struct Fill {
    width: f32,
    row: bool,
    column_height: Option<bool>,
}

/// The border-box width a ratio-only image item takes as a row flex item's base size
/// (miaskiewicz/turbo-dom#326), like Chrome 156: a definite `known_height` (the item
/// stretched in a container of definite height) gives the width through the `ratio`;
/// with no definite or min/max height it is the container's whole inner width less its
/// margins — exactly, where block layout derives the width back from the layout-unit
/// height (400 x 266.65625 against a block's 399.984375); taffy then applies its
/// `min-width` / `max-width`. `None` when a height constraint sizes it instead, which the
/// scratch layout already resolves.
fn ratio_only_row_width(
    item: &LayoutBox,
    (fs, cb_width): (f32, f32),
    known_height: Option<f32>,
    ratio: f32,
) -> Option<f32> {
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width,
    });
    if let Some(height) = known_height {
        let frame_v = bs.padding.vertical() + bs.border.widths().vertical();
        let frame_h = bs.padding.horizontal() + bs.border.widths().horizontal();
        return Some((height - frame_v).max(0.0) * ratio + frame_h);
    }
    let free = matches!(bs.height, LengthPct::Auto)
        && matches!(bs.max_height, LengthPct::Auto)
        && matches!(bs.min_height, LengthPct::Auto | LengthPct::Px(..=0.0));
    free.then(|| (cb_width - bs.margin.horizontal()).max(0.0))
}

fn svg_probe_width(
    item: &LayoutBox,
    viewport: &super::svg_viewport::SvgViewport,
    font_size: f32,
) -> f32 {
    let style = item.resolved(ResolveCtx {
        parent_font_size: font_size,
        cb_width: 0.0,
    });
    let frame = style.padding.horizontal() + style.border.widths().horizontal();
    match style.width {
        LengthPct::Px(width) => width + frame,
        _ => viewport.natural_width(style.font_size) + frame,
    }
}

/// Natural-size replaced items keep their aspect ratio when flex constraints
/// supply a used width. Resolve percentages against the container, not the
/// item's own scratch width, and avoid mixing auto/known sizes in its cache.
fn measure_natural_image(
    item: &LayoutBox,
    images: &ImageCtx,
    known: Size<Option<f32>>,
    (fs, cb_width): (f32, f32),
) -> Option<Size<f32>> {
    if item.svg_viewport.is_some() {
        return None;
    }
    let src = item.image.as_ref().filter(|src| src.replaced)?;
    let (intrinsic, _) = super::imgsize::replaced_intrinsic(images.resolver, src, &item.style)?;
    if intrinsic.natural_ratio.is_some() {
        return None;
    }
    let mut bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width,
    });
    let frame = (
        bs.padding.horizontal() + bs.border.widths().horizontal(),
        bs.padding.vertical() + bs.border.widths().vertical(),
    );
    let inset = if bs.box_sizing == BoxSizing::BorderBox {
        (0.0, 0.0)
    } else {
        frame
    };
    if let Some(width) = known.width {
        bs.width = LengthPct::Px((width - inset.0).max(0.0));
    }
    if let Some(height) = known.height {
        bs.height = LengthPct::Px((height - inset.1).max(0.0));
    }
    let sizing = super::imgsize::SizeCtx {
        style: &bs,
        cb_width,
        cb_height: None,
        body_height: images.body_height,
    };
    let sized = super::imgsize::size_replaced_live(String::new(), intrinsic, &sizing, 1.0);
    Some(Size {
        width: known.width.unwrap_or(sized.width + frame.0),
        height: known.height.unwrap_or(sized.height + frame.1),
    })
}

fn explicit_content_basis(style: &ComputedStyle) -> bool {
    style
        .get("flex-basis")
        .is_some_and(|value| value.trim() == "content")
        || style
            .get("flex")
            .is_some_and(|value| basis_tokens(value).contains(&"content"))
}

fn content_column_basis(item: &LayoutBox, fs: f32, fill: Fill) -> bool {
    let Some(definite) = fill.column_height else {
        return false;
    };
    let explicit = explicit_content_basis(&item.style);
    let style = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: fill.width,
    });
    let basis = item_basis(&item.style, style.height, fs);
    explicit
        || (!definite
            && (basis.tag() == taffy::CompactLength::PERCENT_TAG || basis.into_raw().is_calc()))
}

fn specified_content_minimum(
    style: Option<&BoxStyle>,
    avail: AvailableSpace,
    content: bool,
) -> f32 {
    if !content || avail != AvailableSpace::MinContent {
        return f32::INFINITY;
    }
    let Some(style) = style else {
        return f32::INFINITY;
    };
    match style.height {
        LengthPct::Px(height) => block::to_border(
            height,
            style.box_sizing,
            style.padding.vertical() + style.border.widths().vertical(),
        ),
        _ => f32::INFINITY,
    }
}

fn measure_item(
    known: Size<Option<f32>>,
    avail: Size<AvailableSpace>,
    item: &LayoutBox,
    fs: f32,
    fonts: &FontRegistry,
    images: &ImageCtx,
    policy: (
        bool,
        bool,
        Option<f32>,
        Option<&crate::layout_snapshot::text_units::NativeUnitMap>,
        Fill,
        Option<&BoxStyle>,
    ),
) -> Size<f32> {
    let (live_geometry, html_font_metrics, html_font_scale, original_units, fill, measured_style) =
        policy;
    let content_basis = content_column_basis(item, fs, fill);
    if fill.row {
        if let Some(size) = measure_natural_image(item, images, known, (fs, fill.width)) {
            return size;
        }
    }
    // A replaced `<img>` sizes from its intrinsic (or explicit) width, a ratio-only one
    // from the width it is offered; other items from a max-content measurement of
    // their content.
    let mut exact_width = None;
    let w = match replaced_probe(item, images, fs) {
        Some(Probe::Fixed(w)) => w,
        Some(Probe::RatioOnly { ratio }) => {
            // The width it is given, else the space it is offered (a grid area), else
            // the container's (a flex base size is probed under a content constraint).
            let w = known
                .width
                .or(avail.width.into_option())
                .unwrap_or(fill.width);
            if fill.row && known.width.is_none() {
                exact_width = ratio_only_row_width(item, (fs, fill.width), known.height, ratio);
            }
            w
        }
        None => measure_width(known.width, avail.width, item, fonts),
    };
    // Memoize the full sub-layout by proposed width: taffy probes each item several
    // times per solve, and each probe recurses a full layout, so nested flex is
    // exponential without this. `fs` (the flex container's font size) is stable per
    // item. Percentage frames also depend on the current containing-block basis.
    let probe = || {
        let padding = measured_style.map_or_else(
            || {
                item.resolved(ResolveCtx {
                    parent_font_size: fs,
                    cb_width: w,
                })
                .padding
            },
            |style| style.padding,
        );
        let key = [
            w.to_bits(),
            padding.top.to_bits(),
            padding.right.to_bits(),
            padding.bottom.to_bits(),
            padding.left.to_bits(),
            u32::from(content_basis),
        ];
        item.measure_cached(key, || {
            let mut bs = measured_style.cloned().unwrap_or_else(|| {
                item.resolved(ResolveCtx {
                    parent_font_size: fs,
                    cb_width: w,
                })
            });
            if content_basis {
                bs.height = LengthPct::Auto;
            }
            let mut sd = Diagnostics::default();
            let mut mctx = Ctx {
                fonts,
                // The real resolver, so a replaced `<img>` reaches its intrinsic size in
                // the scratch layout (an empty resolver measured every image to 0).
                images,
                diags: &mut sd,
                // Scratch measurement: the item is its own containing block at origin.
                abs_cb_x: 0.0,
                abs_cb_y: 0.0,
                abs_cb_w: w,
                abs_cb_h: 0.0,
                abs_cb_h_known: false,
                cb_h: 0.0,
                cb_h_known: false,
                quirks_parent: None,
                root_w: w,
                root_h: crate::style::viewport_px().1,
                icb_h: 0.0,
                abs_cb_icb: false,
                live_geometry,
                used_metrics: None,
                text_capture: None,
                html_font_metrics,
                html_font_scale,
                original_units,
                native_transforms: None,
                floats: Vec::new(),
                table_rows: None,
                dropped_end_margins: std::collections::HashSet::new(),
                sticky_pending: Vec::new(),
                sticky: crate::layout::sticky::StickyState::default(),
                writing: crate::layout::writing::WritingMode::default(),
            };
            let frag = block::layout_box_sized_isolated(item, &bs, 0.0, 0.0, w, &mut mctx);
            (frag.width, frag.height)
        })
    };
    // A relabelled copy's own cache starts empty: share the measurement with the copies
    // made the same way (R2 review #4), keyed by everything the scratch layout reads.
    let (cw, ch) = if item.relabel_memo.is_copy() {
        let mut inputs = super::boxgen::measure_inputs(item, fonts);
        inputs.extend([
            u64::from(w.to_bits()),
            u64::from(fs.to_bits()),
            u64::from(content_basis),
            measured_style.map_or(0, |style| u64::from(style.padding.top.to_bits())),
            measured_style.map_or(0, |style| u64::from(style.padding.right.to_bits())),
            measured_style.map_or(0, |style| u64::from(style.padding.bottom.to_bits())),
            measured_style.map_or(0, |style| u64::from(style.padding.left.to_bits())),
            std::ptr::from_ref(images.resolver).cast::<()>() as usize as u64,
            u64::from(images.body_height.unwrap_or(-1.0).to_bits()),
            u64::from(live_geometry),
            u64::from(html_font_metrics),
            u64::from(html_font_scale.unwrap_or(0.0).to_bits()),
            original_units.map_or(0, |units| std::ptr::from_ref(units) as usize as u64),
        ]);
        let kind = super::boxgen::MemoKind::FlexMeasure;
        item.relabel_memo.get_or(kind, inputs, probe)
    } else {
        probe()
    };
    Size {
        width: known.width.or(exact_width).unwrap_or(cw),
        height: known.height.unwrap_or(ch.min(specified_content_minimum(
            measured_style,
            avail.height,
            content_basis,
        ))),
    }
}

// --------------------------------------------------------------------------
// solve + placement
// --------------------------------------------------------------------------

fn build_leaves(
    tree: &mut TaffyTree<usize>,
    items: &[LayoutBox],
    (fs, column, width): (f32, bool, f32),
    images: &ImageCtx,
    fonts: &FontRegistry,
) -> Vec<TaffyId> {
    items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let mut original = item_style(it, fs, column);
            row_content_basis(&mut original, it, (fs, column, width), fonts, images);
            let mut style = frames::style(original, it, fs);
            auto_min_main(&mut style, it, column, images);
            tree.new_leaf_with_context(style, i)
        })
        .collect()
}

fn row_content_basis(
    style: &mut Style,
    item: &LayoutBox,
    (fs, column, width): (f32, bool, f32),
    fonts: &FontRegistry,
    images: &ImageCtx,
) {
    if column {
        return;
    }
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: width,
    });
    let content = explicit_content_basis(&item.style);
    let transferred = match (bs.aspect_ratio, bs.height) {
        (Some(ratio), LengthPct::Px(height)) => Some(height * ratio),
        _ => None,
    };
    if !(content
        || (style.flex_basis.is_auto() && bs.width == LengthPct::Auto && transferred.is_some()))
    {
        return;
    }
    let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
    let border = transferred.map_or_else(
        || row_intrinsic_width(item, &bs, fonts, images, width),
        |width| match bs.box_sizing {
            BoxSizing::BorderBox => width,
            BoxSizing::ContentBox => width + frame,
        },
    );
    style.flex_basis = Dimension::length(match bs.box_sizing {
        BoxSizing::BorderBox => border,
        BoxSizing::ContentBox => (border - frame).max(0.0),
    });
}

fn row_intrinsic_width(
    item: &LayoutBox,
    bs: &BoxStyle,
    fonts: &FontRegistry,
    images: &ImageCtx,
    width: f32,
) -> f32 {
    if let Some(viewport) = &item.svg_viewport {
        let mut style = bs.clone();
        style.width = LengthPct::Auto;
        let (width, _) = viewport.size(&super::imgsize::SizeCtx {
            style: &style,
            cb_width: width,
            cb_height: None,
            body_height: images.body_height,
        });
        return width + bs.padding.horizontal() + bs.border.widths().horizontal();
    }
    let intrinsic = item
        .image
        .as_ref()
        .filter(|src| src.replaced)
        .and_then(|src| super::imgsize::replaced_intrinsic(images.resolver, src, &item.style));
    let Some((intrinsic, _)) = intrinsic else {
        return intrinsic_natural_width(item, bs, fonts);
    };
    let mut style = bs.clone();
    style.width = LengthPct::Auto;
    let sized = super::imgsize::size_replaced_live(
        String::new(),
        intrinsic,
        &super::imgsize::SizeCtx {
            style: &style,
            cb_width: 0.0,
            cb_height: None,
            body_height: images.body_height,
        },
        1.0,
    );
    sized.width + bs.padding.horizontal() + bs.border.widths().horizontal()
}

/// Scrollable overflow uses a zero automatic minimum; visible/clip retain content.
fn scrollable_overflow(style: &ComputedStyle) -> bool {
    let mut parts = style
        .get("overflow")
        .unwrap_or("visible")
        .split_whitespace();
    let x = parts.next().unwrap_or("visible");
    let y = parts.next().unwrap_or(x);
    [("overflow-x", x), ("overflow-y", y)]
        .into_iter()
        .any(|(axis, fallback)| {
            matches!(
                style.get(axis).unwrap_or(fallback).trim(),
                "hidden" | "scroll" | "auto"
            )
        })
}

fn auto_min_main(style: &mut Style, item: &LayoutBox, column: bool, _images: &ImageCtx) {
    let prop = if column { "min-height" } else { "min-width" };
    let auto = item.style.get(prop).is_none_or(|v| v.trim() == "auto");
    if auto && !scrollable_overflow(&item.style) {
        if column {
            style.min_size.height = Dimension::auto();
        } else {
            style.min_size.width = Dimension::auto();
        }
    }
}

fn solve(
    tree: &mut TaffyTree<usize>,
    root: TaffyId,
    items: &[LayoutBox],
    (fs, cw): (f32, f32),
    fill: Fill,
    ctx: &Ctx,
) {
    // Browser CSS geometry preserves fractional advances; whole-pixel rounding
    // belongs only to the legacy PDF layout policy.
    if ctx.html_font_metrics || ctx.live_geometry || super::geometry_policy::fractional() {
        tree.disable_rounding();
    }
    let avail = Size {
        width: AvailableSpace::Definite(cw),
        height: AvailableSpace::MaxContent,
    };
    tree.compute_layout_with_measure(root, avail, |known, av, _node, ctx_idx, style, parent| {
        let idx = *ctx_idx.expect("leaf context");
        frames::measure(known, av, parent, style, (&items[idx], fs), ctx, fill)
    });
}

/// Lay out one placed item; `axis`: `Some(column)` for a flex item, `None` for a grid
/// item.
fn place_one(
    item: &LayoutBox,
    layout: &Layout,
    parent_width: f32,
    (cx, cy, fs): (f32, f32, f32),
    axis: Option<(bool, AlignItems, bool)>,
    ctx: &mut Ctx,
) -> Fragment {
    let column = axis.is_some_and(|(column, _, _)| column);
    let stretch = axis.is_none_or(|(_, parent, _)| {
        align_self(&item.style).unwrap_or(parent) == AlignItems::Stretch
    });
    let mut bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: parent_width,
    });
    bs.padding = frames::edges(layout.padding);
    let mut height_basis = block::ContentHeightBasis::Declared;
    // taffy has resolved this item's main-axis size (column `flex-basis`/`grow`)
    // and cross-axis size (row `align-items:stretch`). Force that border-box height
    // onto the item's own layout as a definite content-box height, so it drives the
    // fragment height AND is exposed to the item's own content (nested flex,
    // `%` heights) — otherwise the item collapses back to its content height and a
    // `flex:1` / stretched item, and anything it centres, is placed wrong. An item
    // whose height taffy left at auto keeps its content height (basis == content). In a
    // column the height is the main size, so taffy's always wins over a `px` `height`
    // (a `flex-basis`, grow or shrink; miaskiewicz/turbo-dom#86).
    if column || stretch || matches!(bs.height, LengthPct::Px(_)) {
        let inset_v = bs.padding.vertical() + bs.border.widths().vertical();
        bs.height = LengthPct::Px((layout.size.height - inset_v).max(0.0));
        height_basis = item_height_basis(
            axis,
            item,
            bs.font_size,
            (layout.size.height - inset_v).max(0.0),
        );
        if bs.box_sizing == BoxSizing::BorderBox {
            // The item's own border-box `min-height` / `max-height` become content-box
            // lengths with it, so they don't add the frame twice (0.7.0 review R3 #2).
            bs.min_height = content_height(bs.min_height, inset_v);
            bs.max_height = content_height(bs.max_height, inset_v);
        }
        bs.box_sizing = BoxSizing::ContentBox;
    }
    // An auto-width ratio-only grid image derives its width back from the
    // quantized height; flex items instead retain their allocated width.
    let ratio_grid =
        axis.is_none() && bs.width == LengthPct::Auto && is_ratio_only(item, ctx.images);
    if !ratio_grid {
        used_width(&mut bs, layout.size.width);
    }
    let width = layout
        .size
        .width
        .max(bs.padding.horizontal() + bs.border.widths().horizontal());
    let mut frag = block::layout_box_sized_isolated_with_height_basis(
        item,
        &bs,
        (0.0, 0.0),
        width,
        height_basis,
        ctx,
    );
    frag.translate(cx + layout.location.x, cy + layout.location.y);
    frag
}

fn item_height_basis(
    axis: Option<(bool, AlignItems, bool)>,
    item: &LayoutBox,
    fs: f32,
    height: f32,
) -> block::ContentHeightBasis {
    match axis {
        Some((true, _, false)) if !definite_item_basis(item, fs) => {
            block::ContentHeightBasis::Indefinite
        }
        _ => block::ContentHeightBasis::Definite(height),
    }
}

fn definite_item_basis(item: &LayoutBox, fs: f32) -> bool {
    let bs = item.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: 0.0,
    });
    bs.aspect_ratio.is_some()
        || item_basis(&item.style, bs.height, fs).tag() == taffy::CompactLength::LENGTH_TAG
}

fn used_width(bs: &mut BoxStyle, width: f32) {
    // The solver already applied preferred and min/max widths against the
    // containing block. Reapplying percentages against the allocated item width
    // would constrain it a second time and give its descendants the wrong width.
    let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
    bs.width = match bs.box_sizing {
        BoxSizing::BorderBox => LengthPct::Px(width),
        BoxSizing::ContentBox => LengthPct::Px((width - frame).max(0.0)),
    };
    bs.min_width = LengthPct::Auto;
    bs.max_width = LengthPct::Auto;
}

/// A border-box `px` length as a content-box one (`inset`: padding and border on that
/// axis); percentages and `auto` / `none` are left as they are.
fn content_height(lp: LengthPct, inset: f32) -> LengthPct {
    match lp {
        LengthPct::Px(v) => LengthPct::Px((v - inset).max(0.0)),
        other => other,
    }
}

fn place_items(
    tree: &TaffyTree<usize>,
    leaves: &[TaffyId],
    items: &[LayoutBox],
    origin: (f32, f32, f32),
    axis: Option<(bool, AlignItems, bool)>,
    ctx: &mut Ctx,
) -> Vec<Fragment> {
    let mut frags = Vec::new();
    for (i, leaf) in leaves.iter().enumerate() {
        let layout = tree.layout(*leaf).expect("item layout");
        frags.push(place_one(
            &items[i],
            layout,
            tree.parent_size(*leaf).width.unwrap_or(0.0),
            origin,
            axis,
            ctx,
        ));
    }
    frags
}

// --------------------------------------------------------------------------
// grid (taffy owns the grid algorithm; we map CSS templates + gaps)
// --------------------------------------------------------------------------

/// One explicit-gap axis: `column-gap`/`row-gap`, falling back to `gap`.
fn gap_axis(s: &ComputedStyle, axis: &str) -> LengthPercentage {
    let px = match s.get(axis) {
        Some(v) => parse_px(v, DEFAULT_FONT_SIZE),
        None => gap_shorthand(s, axis == "column-gap"),
    }
    .unwrap_or(0.0);
    LengthPercentage::length(px)
}

/// Parse a `minmax(min, max)` track, if `t` is one (both sides required).
fn minmax_track(t: &str) -> Option<NonRepeatedTrackSizingFunction> {
    let inner = t
        .strip_prefix("minmax(")
        .and_then(|x| x.strip_suffix(')'))?;
    let (a, b) = inner.split_once(',')?;
    Some(minmax(min_track(a), max_track(b)))
}

/// One grid track: `1fr`, `50%`, `200px`/`15.5rem`, `minmax(min, max)`, or
/// `auto`/`min-content`/`max-content` (â taffy `AUTO`). Unparsable â `AUTO`.
fn track_of(tok: &str) -> TrackSizingFunction {
    single_track(tok).into()
}

/// [`track_of`] as the non-repeated track an auto-`repeat()` lists.
fn single_track(tok: &str) -> NonRepeatedTrackSizingFunction {
    let t = tok.trim();
    if let Some(mm) = minmax_track(t) {
        return mm;
    }
    if let Some(kw) = keyword_track(t) {
        return kw;
    }
    scalar_track(t).unwrap_or(NonRepeatedTrackSizingFunction::AUTO)
}

/// `min-content`/`max-content` size the track to the items' content (a
/// `grid-template-rows:min-content` row hugs its tallest cell's min-content height);
/// mapping them to `AUTO` would let the row stretch to the container instead. Any
/// other token is not a keyword track (`None`).
fn keyword_track(t: &str) -> Option<NonRepeatedTrackSizingFunction> {
    match t {
        "min-content" => Some(minmax(
            MinTrackSizingFunction::MIN_CONTENT,
            MaxTrackSizingFunction::MIN_CONTENT,
        )),
        "max-content" => Some(minmax(
            MinTrackSizingFunction::MAX_CONTENT,
            MaxTrackSizingFunction::MAX_CONTENT,
        )),
        _ => None,
    }
}

/// A single scalar track: `1fr`, `50%`, or a length (`200px`/`15.5rem`). `None` if
/// the token matches none of those (the caller falls back to `AUTO`).
fn scalar_track(t: &str) -> Option<NonRepeatedTrackSizingFunction> {
    if let Some(f) = t
        .strip_suffix("fr")
        .and_then(|x| x.trim().parse::<f32>().ok())
    {
        return Some(fr(f));
    }
    if let Some(p) = t
        .strip_suffix('%')
        .and_then(|x| x.trim().parse::<f32>().ok())
    {
        return Some(percent(p / 100.0));
    }
    parse_px(t, DEFAULT_FONT_SIZE).map(NonRepeatedTrackSizingFunction::from_length)
}

/// The min side of a `minmax()` (no `fr` allowed): length/`%`, else `auto`.
fn min_track(t: &str) -> MinTrackSizingFunction {
    let t = t.trim();
    if let Some(p) = t
        .strip_suffix('%')
        .and_then(|x| x.trim().parse::<f32>().ok())
    {
        return MinTrackSizingFunction::from_percent(p / 100.0);
    }
    if !t.ends_with("fr") {
        if let Some(px) = parse_px(t, DEFAULT_FONT_SIZE) {
            return MinTrackSizingFunction::from_length(px);
        }
    }
    MinTrackSizingFunction::AUTO
}

/// The max side of a `minmax()`: `fr`/length/`%`, else `auto`.
fn max_track(t: &str) -> MaxTrackSizingFunction {
    let t = t.trim();
    if let Some(f) = t
        .strip_suffix("fr")
        .and_then(|x| x.trim().parse::<f32>().ok())
    {
        return fr(f);
    }
    if let Some(p) = t
        .strip_suffix('%')
        .and_then(|x| x.trim().parse::<f32>().ok())
    {
        return MaxTrackSizingFunction::from_percent(p / 100.0);
    }
    if let Some(px) = parse_px(t, DEFAULT_FONT_SIZE) {
        return MaxTrackSizingFunction::from_length(px);
    }
    MaxTrackSizingFunction::AUTO
}

/// A `grid-template-areas` map: area name â the grid-line rectangle it covers
/// `(row_start, row_end, col_start, col_end)`, 0-based cell indices (converted to
/// 1-based taffy lines at use).
type AreaMap = HashMap<String, (i16, i16, i16, i16)>;

/// Parse `grid-template-areas: "a b" "a c"` (each quoted string is a row of
/// space-separated cell names; `.` is an empty cell) into an [`AreaMap`].
fn grid_areas(value: &str) -> AreaMap {
    let mut map: AreaMap = HashMap::new();
    for (r, row) in quoted_rows(value).into_iter().enumerate() {
        for (c, name) in row.split_whitespace().enumerate() {
            if name == "." {
                continue;
            }
            let (r, c) = (r as i16, c as i16);
            let cell = map.entry(name.to_string()).or_insert((r, r, c, c));
            cell.0 = cell.0.min(r);
            cell.1 = cell.1.max(r);
            cell.2 = cell.2.min(c);
            cell.3 = cell.3.max(c);
        }
    }
    map
}

/// The quoted row strings of a `grid-template-areas` value (`'â¦'` or `"â¦"`).
fn quoted_rows(value: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut rest = value;
    while let Some(open) = rest.find(['"', '\'']) {
        let quote = rest.as_bytes()[open] as char;
        rest = &rest[open + 1..];
        match rest.find(quote) {
            Some(close) => {
                rows.push(rest[..close].to_string());
                rest = &rest[close + 1..];
            }
            None => break,
        }
    }
    rows
}

/// Column count implied by a `grid-template-areas` map (max column line used).
fn area_cols(areas: &AreaMap) -> usize {
    areas
        .values()
        .map(|&(_, _, _, c1)| c1 as usize + 1)
        .max()
        .unwrap_or(0)
}

/// Row count implied by a `grid-template-areas` map (max row line used).
fn area_rows(areas: &AreaMap) -> usize {
    areas
        .values()
        .map(|&(_, r1, _, _)| r1 as usize + 1)
        .max()
        .unwrap_or(0)
}

/// Keep explicit `tracks` if present; otherwise synthesize `n` AUTO tracks so a
/// grid declared only via `grid-template-areas` still has tracks to place into.
fn fill_tracks(tracks: Vec<TrackSizingFunction>, n: usize) -> Vec<TrackSizingFunction> {
    if tracks.is_empty() {
        vec![TrackSizingFunction::AUTO; n]
    } else {
        tracks
    }
}

/// A grid item's row/column line spans from `grid-area: <name>` resolved against
/// the container's `grid-template-areas`. `None` when the item names no area (taffy
/// then auto-places it). Grid lines are 1-based with an exclusive end line.
fn area_placement(
    item: &LayoutBox,
    areas: &AreaMap,
) -> Option<(Line<GridPlacement>, Line<GridPlacement>)> {
    let name = item.style.get("grid-area")?.trim();
    let &(r0, r1, c0, c1) = areas.get(name)?;
    let row = Line {
        start: line(r0 + 1),
        end: line(r1 + 2),
    };
    let col = Line {
        start: line(c0 + 1),
        end: line(c1 + 2),
    };
    Some((row, col))
}

/// `repeat(N, <tracks>)` expanded into N copies of its track list; `repeat(auto-fill
/// | auto-fit, <tracks>)` kept as taffy's auto-repetition, which fills the container
/// with as many tracks as fit (`auto-fit` then collapses the empty ones;
/// miaskiewicz/turbo-dom#201).
fn parse_repeat(tok: &str) -> Option<Vec<TrackSizingFunction>> {
    let inner = tok.trim().strip_prefix("repeat(")?.strip_suffix(')')?;
    let (count, tracks) = inner.split_once(',')?;
    let tokens = super::value::css_value_tokens(tracks);
    let auto = match count.trim() {
        "auto-fill" => GridTrackRepetition::AutoFill,
        "auto-fit" => GridTrackRepetition::AutoFit,
        n => {
            let count: usize = n.parse().ok()?;
            let one: Vec<TrackSizingFunction> = tokens.into_iter().map(track_of).collect();
            return Some(
                one.iter()
                    .cloned()
                    .cycle()
                    .take(count * one.len())
                    .collect(),
            );
        }
    };
    let singles = tokens.into_iter().map(single_track).collect();
    Some(vec![TrackSizingFunction::Repeat(auto, singles)])
}

/// Parse a `grid-template-columns`/`-rows` value into taffy tracks. Empty /
/// `none` â no explicit tracks (taffy's implicit-grid auto-placement applies).
fn grid_tracks(spec: Option<&str>) -> Vec<TrackSizingFunction> {
    let Some(spec) = spec
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "none")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for tok in super::value::css_value_tokens(spec) {
        match parse_repeat(tok) {
            Some(rep) => out.extend(rep),
            None => out.push(track_of(tok)),
        }
    }
    out
}

/// The track list for one axis: the longhand `grid-template-columns`/`-rows`, or,
/// if absent, the corresponding side of the `grid-template` / `grid` shorthand
/// (`<rows> / <cols>`). Parsing the shorthand matters for real pages (e.g.
/// Wikipedia's Vector `grid-template: â¦ / 12.25rem minmax(0,1fr)`): without it the
/// axis falls back to AUTO tracks, which with named areas makes taffy content-size
/// a huge subtree per track â pathologically slow â instead of using fixed tracks.
fn axis_tracks(s: &ComputedStyle, longhand: &str, want_cols: bool) -> Vec<TrackSizingFunction> {
    let explicit = grid_tracks(s.get(longhand));
    if !explicit.is_empty() {
        return explicit;
    }
    grid_tracks(shorthand_axis(s, want_cols).as_deref())
}

/// The rows (`before /`) or columns (`after /`) side of a `grid-template` / `grid`
/// shorthand, with any quoted `grid-template-areas` row strings stripped out.
fn shorthand_axis(s: &ComputedStyle, want_cols: bool) -> Option<String> {
    let value = s.get("grid-template").or_else(|| s.get("grid"))?;
    let (rows, cols) = value.split_once('/')?;
    let part = strip_quoted(if want_cols { cols } else { rows });
    let part = part.trim();
    (!part.is_empty()).then(|| part.to_string())
}

/// Advance `chars` past the next occurrence of `quote` (its closing delimiter).
fn skip_quoted(chars: &mut impl Iterator<Item = char>, quote: char) {
    for c in chars {
        if c == quote {
            break;
        }
    }
}

/// Remove single/double-quoted segments (area-template rows) from a value.
fn strip_quoted(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            skip_quoted(&mut chars, ch);
        } else {
            out.push(ch);
        }
    }
    out
}

fn grid_container_style(container: &LayoutBox, cw: f32, areas: &AreaMap) -> Style {
    let s = &container.style;
    // Explicit tracks (longhand or the `grid-template` shorthand), else â when only
    // `grid-template-areas` is given â one AUTO track per area column/row so named
    // placement still has a grid to land in.
    let cols = axis_tracks(s, "grid-template-columns", true);
    let rows = axis_tracks(s, "grid-template-rows", false);
    let cols = fill_tracks(cols, area_cols(areas));
    let rows = fill_tracks(rows, area_rows(areas));
    Style {
        display: Display::Grid,
        grid_template_columns: cols.into_iter().collect(),
        grid_template_rows: rows.into_iter().collect(),
        gap: Size {
            width: gap_axis(s, "column-gap"),
            height: gap_axis(s, "row-gap"),
        },
        justify_content: justify_content_grid(s),
        justify_items: justify_items(s),
        align_items: align_items(s),
        size: Size {
            width: Dimension::length(cw),
            height: Dimension::auto(),
        },
        ..Default::default()
    }
}

/// Grid leaves, each carrying its `grid-area` line placement (against `areas`)
/// on top of the shared item style; unnamed items keep taffy auto-placement.
fn build_grid_leaves(
    tree: &mut TaffyTree<usize>,
    items: &[LayoutBox],
    fs: f32,
    areas: &AreaMap,
    container: &ComputedStyle,
) -> Vec<TaffyId> {
    items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let mut style = item_style(it, fs, false);
            // Grid has no flex-basis: an explicit width must reach its own size
            // constraint instead of becoming the track's stretch width — under every
            // layout policy (the HTML path too, miaskiewicz/turbo-dom#267).
            style.size = item_size(&it.resolved(ResolveCtx {
                parent_font_size: fs,
                cb_width: 0.0,
            }));
            // Taffy owns the typed frame and the content-/border-box conversion.
            let mut style = frames::style(style, it, fs);
            if it
                .style
                .get("min-width")
                .is_none_or(|value| value.trim() == "auto")
            {
                style.min_size.width = Dimension::auto();
            }
            // taffy's track sizing reads the preferred size unclamped: a `px` size
            // reaches it already within its `px` min / max, like Chrome's used size.
            style.size = Size {
                width: clamp_len(style.size.width, style.min_size.width, style.max_size.width),
                height: clamp_len(
                    style.size.height,
                    style.min_size.height,
                    style.max_size.height,
                ),
            };
            // Grid items carry their own `justify-self` (flex has none). Its default
            // is `stretch` via the container's `justify-items`; a `center`/`start`/
            // `end` value here instead sizes the item to its own width and aligns it
            // in the track.
            style.justify_self = justify_self(&it.style);
            frames::grid_alignment(&mut style, it, container);
            if let Some((row, col)) = area_placement(it, areas) {
                style.grid_row = row;
                style.grid_column = col;
            } else {
                // Explicit line placement (`grid-column`/`grid-row`) when the grid uses
                // no named areas — `span N` in particular: nike's header items are
                // `grid-column:span 6` in a 12-col grid, so each should fill half the
                // bar; unparsed, taffy auto-placed them one column wide and the utility
                // nav collapsed to ~99px.
                if let Some(col) = grid_line(it.style.get("grid-column")) {
                    style.grid_column = col;
                }
                if let Some(row) = grid_line(it.style.get("grid-row")) {
                    style.grid_row = row;
                }
            }
            tree.new_leaf_with_context(style, i)
        })
        .collect()
}

/// A `px` taffy length clamped by `px` `min` / `max` lengths (`min` wins, as in CSS);
/// any other length or bound is left to taffy.
fn clamp_len(d: Dimension, min: Dimension, max: Dimension) -> Dimension {
    let px = |d: Dimension| (d.tag() == taffy::CompactLength::LENGTH_TAG).then(|| d.value());
    let Some(mut v) = px(d) else {
        return d;
    };
    if let Some(hi) = px(max) {
        v = v.min(hi);
    }
    if let Some(lo) = px(min) {
        v = v.max(lo);
    }
    Dimension::length(v)
}

/// A CSS `grid-column`/`grid-row` value → taffy line placement. Handles `span N`,
/// a single line index, and the `start / end` two-value form. `None` for `auto` or
/// an unparseable value (taffy keeps its auto-placement default).
fn grid_line(value: Option<&str>) -> Option<Line<GridPlacement>> {
    let v = value?.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("auto") {
        return None;
    }
    let (start, end) = v
        .split_once('/')
        .map_or((v, ""), |(a, b)| (a.trim(), b.trim()));
    Some(Line {
        start: grid_placement(start),
        end: grid_placement(end),
    })
}

fn grid_placement(tok: &str) -> GridPlacement {
    if let Some(n) = tok
        .strip_prefix("span")
        .and_then(|r| r.trim().parse::<u16>().ok())
    {
        return span(n);
    }
    tok.parse::<i16>().map_or(GridPlacement::Auto, line)
}

/// Lay out a grid container's items into the content box at `(cx, cy)` of width
/// `cw`. Items are placed by `grid-area` (against the container's
/// `grid-template-areas`) or auto-placed into the tracks. Returns galley-absolute
/// fragments and the content height.
pub(crate) fn layout_grid(
    container: &LayoutBox,
    items: &[LayoutBox],
    cx: f32,
    cy: f32,
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    if items.is_empty() {
        return (Vec::new(), 0.0);
    }
    let areas = container
        .style
        .get("grid-template-areas")
        .map(grid_areas)
        .unwrap_or_default();
    let _calc_scope = native_tree::CalcScope::new(ctx.html_font_scale);
    let mut tree: TaffyTree<usize> = TaffyTree::new();
    let leaves = build_grid_leaves(&mut tree, items, fs, &areas, &container.style);
    let root = tree.new_with_children(grid_container_style(container, cw, &areas), &leaves);
    let fill = Fill {
        width: cw,
        row: false,
        column_height: None,
    };
    grid_tracks::solve_grid(&mut tree, root, &leaves, items, (fs, cw), fill, ctx);
    let frags = place_items(&tree, &leaves, items, (cx, cy, fs), None, ctx);
    let height = tree.layout(root).expect("root layout").size.height;
    (frags, height)
}

/// Lay out a flex container's items into the content box at `(cx, cy)` of width
/// `cw`. Returns the item fragments (galley-absolute) and the content height.
pub(crate) fn layout_flex(
    container: &LayoutBox,
    items: &[LayoutBox],
    cx: f32,
    cy: f32,
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    if items.is_empty() {
        return (Vec::new(), 0.0);
    }
    let _calc_scope = native_tree::CalcScope::new(ctx.html_font_scale);
    let mut tree: TaffyTree<usize> = TaffyTree::new();
    let column = matches!(
        flex_direction(&container.style),
        FlexDirection::Column | FlexDirection::ColumnReverse
    );
    let leaves = build_leaves(&mut tree, items, (fs, column, cw), ctx.images, ctx.fonts);
    let root = tree.new_with_children(
        container_style(container, cw, fs, ctx.cb_h_known.then_some(ctx.cb_h)),
        &leaves,
    );
    if column && !ctx.cb_h_known {
        tree.resolve_indefinite_column_percentages(root);
    }
    let fill = Fill {
        width: cw,
        row: !column,
        column_height: column.then_some(ctx.cb_h_known),
    };
    solve(&mut tree, root, items, (fs, cw), fill, ctx);
    let frags = place_items(
        &tree,
        &leaves,
        items,
        (cx, cy, fs),
        Some((
            column,
            align_items(&container.style).unwrap_or(AlignItems::Stretch),
            ctx.cb_h_known,
        )),
        ctx,
    );
    let height = tree.layout(root).expect("root layout").size.height;
    (frags, height)
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use crate::layout::boxgen::build_box_tree;
    use crate::tag::{Attr, MarkerKind, Tag};
    use crate::text::FontRegistry;
    use crate::{StyledElement, StyledNode};

    fn cs(pairs: &[(&str, &str)]) -> ComputedStyle {
        ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
    }

    #[test]
    fn abs_item_inset_anchors_fully_auto_horizontal_axis_to_start() {
        use crate::layout::value::{resolve_box_style, ResolveCtx};
        let ctx = ResolveCtx {
            parent_font_size: 16.0,
            cb_width: 200.0,
        };
        // No horizontal inset → `left` anchors to 0 (static start), not centered.
        let none = resolve_box_style(&cs(&[("position", "absolute")]), ctx);
        assert_eq!(
            abs_item_inset(&none).left,
            LengthPercentageAuto::length(0.0)
        );
        // Vertical stays `auto` so `align-items` can still center it.
        assert_eq!(abs_item_inset(&none).top, LengthPercentageAuto::auto());
        // An explicit horizontal inset is preserved (no start-anchor).
        let right = resolve_box_style(&cs(&[("position", "absolute"), ("right", "4px")]), ctx);
        assert_eq!(abs_item_inset(&right).left, LengthPercentageAuto::auto());
        assert_eq!(
            abs_item_inset(&right).right,
            LengthPercentageAuto::length(4.0)
        );
    }

    #[test]
    fn mixed_calc_keeps_its_pixel_offset() {
        use taffy::ResolveOrZero;
        let _scope = native_tree::CalcScope::new(None);
        let d = dim(LengthPct::Calc {
            pct: 50.0,
            px: -20.0,
        });
        assert_eq!(
            d.resolve_or_zero(Some(400.0), native_tree::resolve_for_test),
            180.0
        );
    }

    #[test]
    fn grid_line_parses_span_index_and_pair() {
        use taffy::style_helpers::{line, span};
        // unset / `auto` -> None (taffy keeps auto-placement).
        assert_eq!(grid_line(None), None);
        assert_eq!(grid_line(Some("auto")), None);
        assert_eq!(grid_line(Some("  ")), None);
        // `span N` -> a span end, auto start (nike's `grid-column:span 6`).
        assert_eq!(
            grid_line(Some("span 6")),
            Some(Line {
                start: span(6),
                end: GridPlacement::Auto
            })
        );
        // a bare line index.
        assert_eq!(
            grid_line(Some("2")),
            Some(Line {
                start: line(2),
                end: GridPlacement::Auto
            })
        );
        // the `start / end` two-value form (`3 / span 2`).
        assert_eq!(
            grid_line(Some("3 / span 2")),
            Some(Line {
                start: line(3),
                end: span(2)
            })
        );
        // an unparseable token -> Auto.
        assert_eq!(grid_placement("wat"), GridPlacement::Auto);
    }

    #[test]
    fn align_self_maps_every_keyword() {
        // unset / `auto` -> None (defer to the container's align-items).
        assert_eq!(align_self(&cs(&[])), None);
        assert_eq!(align_self(&cs(&[("align-self", "auto")])), None);
        assert_eq!(
            align_self(&cs(&[("align-self", "center")])),
            Some(AlignItems::Center)
        );
        assert_eq!(
            align_self(&cs(&[("align-self", "flex-start")])),
            Some(AlignItems::FlexStart)
        );
        assert_eq!(
            align_self(&cs(&[("align-self", "end")])),
            Some(AlignItems::FlexEnd)
        );
        assert_eq!(
            align_self(&cs(&[("align-self", "baseline")])),
            Some(AlignItems::Baseline)
        );
        assert_eq!(
            align_self(&cs(&[("align-self", "stretch")])),
            Some(AlignItems::Stretch)
        );
    }

    #[test]
    fn justify_content_grid_is_none_when_unset() {
        // Grid leaves `justify-content` unset -> None (taffy's normal = stretch).
        assert_eq!(justify_content_grid(&cs(&[])), None);
        // An explicit value still maps through.
        assert_eq!(
            justify_content_grid(&cs(&[("justify-content", "center")])),
            Some(JustifyContent::Center)
        );
    }

    #[test]
    fn justify_content_value_maps_every_keyword() {
        assert_eq!(justify_content_value("flex-end"), JustifyContent::FlexEnd);
        assert_eq!(justify_content_value("end"), JustifyContent::FlexEnd);
        assert_eq!(justify_content_value("center"), JustifyContent::Center);
        assert_eq!(
            justify_content_value("space-between"),
            JustifyContent::SpaceBetween
        );
        assert_eq!(
            justify_content_value("space-around"),
            JustifyContent::SpaceAround
        );
        assert_eq!(
            justify_content_value("space-evenly"),
            JustifyContent::SpaceEvenly
        );
        assert_eq!(
            justify_content_value("flex-start"),
            JustifyContent::FlexStart
        );
        // the flex default flows through `justify_content`.
        assert_eq!(justify_content(&cs(&[])), Some(JustifyContent::FlexStart));
    }

    #[test]
    fn justify_items_maps_grid_inline_alignment() {
        // default (unset) and explicit `stretch` -> Stretch (grid item fills the cell).
        assert_eq!(justify_items(&cs(&[])), Some(AlignItems::Stretch));
        assert_eq!(
            justify_items(&cs(&[("justify-items", "stretch")])),
            Some(AlignItems::Stretch)
        );
        // `center` shrinks the item to content and centers it (google home logo).
        assert_eq!(
            justify_items(&cs(&[("justify-items", "center")])),
            Some(AlignItems::Center)
        );
        assert_eq!(
            justify_items(&cs(&[("justify-items", "start")])),
            Some(AlignItems::Start)
        );
        assert_eq!(
            justify_items(&cs(&[("justify-items", "end")])),
            Some(AlignItems::End)
        );
    }

    #[test]
    fn flex_shorthand_grow_shrink() {
        assert_eq!(flex_grow_shrink(&cs(&[])), (0.0, 1.0)); // no `flex` -> CSS initial
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "none")])), (0.0, 0.0));
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "auto")])), (1.0, 1.0));
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "initial")])), (0.0, 1.0));
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "1")])), (1.0, 1.0)); // grow 1, shrink default
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "2 3")])), (2.0, 3.0));
        assert_eq!(flex_grow_shrink(&cs(&[("flex", "1 1 0%")])), (1.0, 1.0)); // basis skipped
    }

    #[test]
    fn justify_self_maps_every_keyword() {
        // unset / `auto` -> None (defer to the container's justify-items).
        assert_eq!(justify_self(&cs(&[])), None);
        assert_eq!(justify_self(&cs(&[("justify-self", "auto")])), None);
        // the start family (`start`/`flex-start`/`left`) -> Start.
        for v in ["start", "flex-start", "left"] {
            assert_eq!(
                justify_self(&cs(&[("justify-self", v)])),
                Some(AlignItems::Start)
            );
        }
        // the end family (`end`/`flex-end`/`right`) -> End.
        for v in ["end", "flex-end", "right"] {
            assert_eq!(
                justify_self(&cs(&[("justify-self", v)])),
                Some(AlignItems::End)
            );
        }
        assert_eq!(
            justify_self(&cs(&[("justify-self", "center")])),
            Some(AlignItems::Center)
        );
        // anything else -> Stretch (the item fills its track).
        assert_eq!(
            justify_self(&cs(&[("justify-self", "stretch")])),
            Some(AlignItems::Stretch)
        );
    }

    #[test]
    fn flex_shorthand_basis_resolves_every_form() {
        // `none`/`initial` use `auto`, which defers to the declared main size.
        assert_eq!(flex_shorthand_basis(&cs(&[("flex", "none")]), 16.0), None);
        // a length `<basis>` token wins (`1 1 60px` -> 60px).
        assert_eq!(
            flex_shorthand_basis(&cs(&[("flex", "1 1 60px")]), 16.0),
            Some(Dimension::length(60.0))
        );
        // a `content` `<basis>` token -> content-sized.
        assert_eq!(
            flex_shorthand_basis(&cs(&[("flex", "1 1 content")]), 16.0),
            Some(Dimension::auto())
        );
        // an explicit `auto` token defers to `width` (`None`).
        assert_eq!(
            flex_shorthand_basis(&cs(&[("flex", "1 1 auto")]), 16.0),
            None
        );
        // a numbers-only shorthand carries an implied `0` basis (CSS `flex:1` == 1 1 0%).
        assert_eq!(
            flex_shorthand_basis(&cs(&[("flex", "2 0")]), 16.0),
            Some(Dimension::percent(0.0))
        );
        // an unparsable trailing token is skipped, leaving the implied `0` basis.
        assert_eq!(
            flex_shorthand_basis(&cs(&[("flex", "1 garbage")]), 16.0),
            Some(Dimension::percent(0.0))
        );
        // no `flex` at all -> None (caller falls back to `width`).
        assert_eq!(flex_shorthand_basis(&cs(&[]), 16.0), None);
    }

    fn bs_of(pairs: &[(&str, &str)]) -> BoxStyle {
        super::super::value::resolve_box_style(
            &cs(pairs),
            ResolveCtx {
                parent_font_size: 16.0,
                cb_width: 200.0,
            },
        )
    }

    fn el(tag: &str, pairs: &[(&str, &str)], children: Vec<StyledNode>) -> StyledNode {
        let mut style = cs(pairs);
        // Chrome's `html.css` `div { display: block }` (miaskiewicz/turbo-dom#375):
        // test-built styled trees bypass the UA cascade, so a `div` without an explicit
        // display sets it here.
        if tag == "div" && style.get("display").is_none() {
            style.set("display", "block".into());
        }
        StyledNode::Element(StyledElement {
            tag: Tag::Html(tag.to_string()),
            attrs: vec![],
            style,
            children,
            source: None,
        })
    }

    fn text_item(t: &str) -> StyledNode {
        el("div", &[], vec![StyledNode::Text(t.into())])
    }

    // --- item_basis: flex-basis %, and the non-length fallback to `width` ---
    #[test]
    fn item_basis_percent_and_width_fallback() {
        // `flex-basis: 50%` -> a percentage dimension (the `strip_suffix('%')` arm).
        let s = cs(&[("flex-basis", "50%")]);
        let bs = bs_of(&[("flex-basis", "50%")]);
        assert_eq!(item_basis(&s, bs.width, 16.0), Dimension::percent(0.5_f32));

        // A basis that is neither a length nor a percentage (`min-content`) falls back
        // to the item's own `width`.
        let s = cs(&[("flex-basis", "min-content"), ("width", "77px")]);
        let bs = bs_of(&[("flex-basis", "min-content"), ("width", "77px")]);
        assert_eq!(item_basis(&s, bs.width, 16.0), Dimension::length(77.0_f32));
    }

    // --- item_inset: a percentage inset edge on an out-of-flow flex child ---
    #[test]
    fn item_inset_percentage_edge() {
        let bs = bs_of(&[("top", "25%"), ("left", "10%")]);
        let inset = item_inset(&bs);
        assert_eq!(inset.top, LengthPercentageAuto::percent(0.25_f32));
        assert_eq!(inset.left, LengthPercentageAuto::percent(0.10_f32));
    }

    // --- grid track keywords: min-content / max-content size to content ---
    #[test]
    fn track_of_maps_content_keywords() {
        // A `min-content`/`max-content` track must size to content, not stretch like
        // `auto` — else a `grid-template-rows:min-content` row fills the container.
        assert_eq!(
            track_of("min-content"),
            minmax(
                MinTrackSizingFunction::MIN_CONTENT,
                MaxTrackSizingFunction::MIN_CONTENT
            )
        );
        assert_eq!(
            track_of("max-content"),
            minmax(
                MinTrackSizingFunction::MAX_CONTENT,
                MaxTrackSizingFunction::MAX_CONTENT
            )
        );
        assert_eq!(track_of("auto"), TrackSizingFunction::AUTO);
        assert_eq!(
            track_of("120px"),
            TrackSizingFunction::from_length(120.0_f32)
        );
    }

    // --- grid natural width: px column tracks + gaps, else the widest item ---
    #[test]
    fn grid_natural_sums_px_tracks_and_gaps() {
        let fonts = FontRegistry::new();
        let item = |w: &'static str| el("div", &[("width", w), ("height", "1px")], vec![]);
        let grid = |pairs: &[(&str, &str)]| {
            let mut style = vec![("display", "grid")];
            style.extend_from_slice(pairs);
            let tree = build_box_tree(&[el("div", &style, vec![item("7px"), item("9px")])]);
            let BoxKind::Block(kids) = &tree.kind else {
                unreachable!()
            };
            natural_width(&kids[0], &fonts)
        };
        assert_eq!(grid(&[("grid-template-columns", "15px 25px")]), 40.0);
        assert_eq!(
            grid(&[
                ("grid-template-columns", "15px 25px"),
                ("column-gap", "3px")
            ]),
            43.0
        );
        assert_eq!(
            grid(&[("grid-template-columns", "15px 25px"), ("gap", "1px 4px")]),
            44.0
        );
        // Not all px tracks (or none): the widest item.
        assert_eq!(grid(&[("grid-template-columns", "15px 1fr")]), 9.0);
        assert_eq!(grid(&[]), 9.0);
    }

    // --- flex_natural: row sums items, column takes the widest ---
    #[test]
    fn flex_natural_row_and_column() {
        let fonts = FontRegistry::new();
        let row = build_box_tree(&[el(
            "div",
            &[("display", "flex")],
            vec![text_item("aa"), text_item("bbbb")],
        )]);
        let col = build_box_tree(&[el(
            "div",
            &[("display", "flex"), ("flex-direction", "column")],
            vec![text_item("aa"), text_item("bbbb")],
        )]);
        let rw = natural_width(&row, &fonts);
        let cw = natural_width(&col, &fonts);
        // Row lays items side by side (sum), column stacks them (widest) -> row >= col.
        assert!(rw >= cw);
        assert!(rw >= 0.0 && cw >= 0.0);
    }

    // --- a relabelled copy's measure memo is keyed by the exact probe width ---
    #[test]
    fn relabelled_measure_memo_keys_the_exact_width() {
        use crate::layout::writing::{Relabel, WritingMode};
        // Two 10px-tall inline-blocks, 50px and 50.4px wide: one line at a 100.4px
        // probe, two at 99.6px. Both round to 100, so a rounded key handed the second
        // probe the first one's height (0.7.0 review R3 #6).
        let block = |w: &str| {
            el(
                "span",
                &[
                    ("display", "inline-block"),
                    ("width", w),
                    ("height", "10px"),
                ],
                vec![],
            )
        };
        let root = build_box_tree(&[el(
            "div",
            &[("font-size", "0"), ("line-height", "0")],
            vec![block("50px"), block("50.4px")],
        )]);
        let copy = root
            .relabelled(Relabel::between(
                WritingMode::HorizontalTb,
                WritingMode::VerticalRl,
            ))
            .relabelled(Relabel::between(
                WritingMode::VerticalRl,
                WritingMode::HorizontalTb,
            ));
        assert!(copy.relabel_memo.is_copy());
        let fonts = FontRegistry::new();
        let images = crate::ImageCtx {
            resolver: &crate::NoImages,
            body_height: None,
        };
        let probe = |w: f32| {
            measure_item(
                Size {
                    width: Some(w),
                    height: None,
                },
                Size {
                    width: AvailableSpace::Definite(w),
                    height: AvailableSpace::MaxContent,
                },
                &copy,
                16.0,
                &fonts,
                &images,
                (
                    false,
                    false,
                    None,
                    None,
                    Fill {
                        width: 0.0,
                        row: false,
                        column_height: None,
                    },
                    None,
                ),
            )
            .height
        };
        assert_eq!(probe(100.4), 10.0, "one line at 100.4px");
        assert_eq!(probe(99.6), 20.0, "two lines at 99.6px");
    }

    // --- inline-block atomic contributes to a line's natural/min width ---
    #[test]
    fn inline_block_atomic_measured_in_line() {
        let fonts = FontRegistry::new();
        // `<span display:inline>` wrapping a `<span display:inline-block>text</span>`
        // -> a Lines box whose only piece is an atomic. `build_runs` drops atomics, so
        // without folding their width in, the line (and the flex cell around it)
        // measured 0 and its text overflowed (google's footer "Settings" link).
        let tree = build_box_tree(&[el(
            "span",
            &[("display", "inline")],
            vec![el(
                "span",
                &[("display", "inline-block")],
                vec![StyledNode::Text("Settings".into())],
            )],
        )]);
        let nat = natural_width(&tree, &fonts);
        let min = min_content_width(&tree, &fonts);
        assert!(
            nat > 20.0,
            "atomic line natural width should be its content, got {nat}"
        );
        assert!(
            min > 20.0,
            "atomic line min-content should be its content, got {min}"
        );
    }

    struct LogoImages;

    impl crate::image::ImageResolver for LogoImages {
        fn resolve(&self, _name: &str) -> Option<&[u8]> {
            None
        }

        fn intrinsic(&self, name: &str) -> Option<crate::image::Intrinsic> {
            let (width, height) = match name {
                "wide" => (253, 80),
                "tall" => (560, 400),
                _ => return None,
            };
            Some(crate::image::Intrinsic::new(width, height, true))
        }
    }

    #[test]
    fn row_image_measurement_keeps_constraints_ratio_and_known_width_separate() {
        let fonts = FontRegistry::new();
        let images = ImageCtx {
            resolver: &LogoImages,
            body_height: None,
        };
        for (name, expected) in [
            ("wide", (108.0, 108.0 * 80.0 / 253.0)),
            ("tall", (86.8, 62.0)),
        ] {
            let img = StyledNode::Element(StyledElement {
                tag: Tag::Html("img".into()),
                attrs: vec![Attr {
                    name: "src".into(),
                    value: name.into(),
                }],
                style: cs(&[("max-width", "80%"), ("max-height", "62px")]),
                children: vec![],
                source: None,
            });
            let tree = build_box_tree(&[el("div", &[("display", "flex")], vec![img])]);
            let BoxKind::Block(rows) = &tree.kind else {
                unreachable!()
            };
            let BoxKind::Flex(items) = &rows[0].kind else {
                unreachable!()
            };
            let measure = |width| {
                measure_item(
                    Size {
                        width,
                        height: None,
                    },
                    Size {
                        width: AvailableSpace::MaxContent,
                        height: AvailableSpace::Definite(62.0),
                    },
                    &items[0],
                    16.0,
                    &fonts,
                    &images,
                    (
                        true,
                        true,
                        None,
                        None,
                        Fill {
                            width: 135.0,
                            row: true,
                            column_height: None,
                        },
                        None,
                    ),
                )
            };
            let auto = measure(None);
            assert!((auto.width - expected.0).abs() < 0.001, "{name}: {auto:?}");
            assert!((auto.height - expected.1).abs() < 0.001, "{name}: {auto:?}");
            // The same width can be auto or known; a width-only scratch cache
            // must not substitute its unconstrained intrinsic-height result.
            assert_eq!(measure(Some(auto.width)), auto);
            let shrunk = measure(Some(54.0));
            assert_eq!(shrunk.width, 54.0);
            assert!(shrunk.height < auto.height);
            assert_eq!(measure(None), auto);
        }
    }

    #[test]
    fn image_frames_preserve_border_box_width_and_explicit_known_dimensions() {
        let images = ImageCtx {
            resolver: &LogoImages,
            body_height: None,
        };
        for (sizing, expected_width) in [("border-box", 100.0), ("content-box", 120.0)] {
            let img = StyledNode::Element(StyledElement {
                tag: Tag::Html("img".into()),
                attrs: vec![Attr {
                    name: "src".into(),
                    value: "wide".into(),
                }],
                style: cs(&[
                    ("width", "100px"),
                    ("padding", "10px"),
                    ("box-sizing", sizing),
                ]),
                children: vec![],
                source: None,
            });
            let tree = build_box_tree(&[el(
                "div",
                &[("display", "flex"), ("flex-direction", "column")],
                vec![img],
            )]);
            let BoxKind::Block(rows) = &tree.kind else {
                unreachable!()
            };
            let BoxKind::Flex(items) = &rows[0].kind else {
                unreachable!()
            };
            assert_eq!(
                replaced_probe(&items[0], &images, 16.0),
                Some(Probe::Fixed(expected_width))
            );
            let known = Size {
                width: Some(100.0),
                height: Some(80.0),
            };
            assert_eq!(
                measure_natural_image(&items[0], &images, known, (16.0, 200.0)),
                Some(Size {
                    width: 100.0,
                    height: 80.0
                })
            );
        }
    }

    // --- min_content_width: replaced image path + marker (zero) path ---
    #[test]
    fn min_content_replaced_image_and_directive() {
        let fonts = FontRegistry::new();

        // A replaced `<img src>` (no explicit width) takes the intrinsic/declared
        // branch of `min_content_width`. Wrapped as a flex item so it stays a box.
        let img = StyledNode::Element(StyledElement {
            tag: Tag::Html("img".to_string()),
            attrs: vec![Attr {
                name: "src".to_string(),
                value: "x.png".to_string(),
            }],
            style: cs(&[]),
            children: vec![],
            source: None,
        });
        let root = build_box_tree(&[el("div", &[("display", "flex")], vec![img])]);
        assert!(min_content_width(&root, &fonts) >= 0.0);

        // A marker box has zero min-content width.
        let directive = StyledNode::Element(StyledElement {
            tag: Tag::Marker(MarkerKind {
                id: 0,
                inline: true,
            }),
            attrs: vec![],
            style: cs(&[]),
            children: vec![],
            source: None,
        });
        let root = build_box_tree(&[el("div", &[("display", "flex")], vec![directive])]);
        assert_eq!(min_content_width(&root, &fonts), 0.0);
    }

    // --- grid track parsers: percentage tracks and the AUTO fallbacks ---
    #[test]
    fn track_parsers_percent_and_auto_branches() {
        // An explicit percentage track.
        assert_eq!(track_of("50%"), percent(0.5_f32));
        // minmax() min side: percentage, and `fr` (not a valid min) -> AUTO.
        assert_eq!(
            min_track("50%"),
            MinTrackSizingFunction::from_percent(0.5_f32)
        );
        assert_eq!(min_track("1fr"), MinTrackSizingFunction::AUTO);
        // minmax() max side: percentage, and a non-length (`auto`) -> AUTO.
        assert_eq!(
            max_track("50%"),
            MaxTrackSizingFunction::from_percent(0.5_f32)
        );
        assert_eq!(max_track("auto"), MaxTrackSizingFunction::AUTO);
        // A full minmax() track round-trips both sides.
        assert_eq!(
            track_of("minmax(50%, 1fr)"),
            minmax(MinTrackSizingFunction::from_percent(0.5_f32), fr(1.0_f32))
        );
    }

    // --- grid-template-areas quoting: `.` cells, unbalanced quotes, strip_quoted ---
    #[test]
    fn grid_area_quoting_helpers() {
        // `.` marks an empty cell and is skipped; named cells become rectangles.
        let map = grid_areas("'a .' 'a b'");
        assert!(map.contains_key("a"));
        assert!(map.contains_key("b"));
        assert!(!map.contains_key("."));

        // An unterminated quote stops row scanning cleanly (no infinite loop).
        assert_eq!(quoted_rows("'unterminated").len(), 0);

        // `strip_quoted` drops the quoted area-row segment, keeping the track list.
        assert_eq!(strip_quoted("10px 'a b' 1fr").trim(), "10px  1fr".trim());
    }
}

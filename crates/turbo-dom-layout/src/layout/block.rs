//! Block layout (§5.3, AC-5.5): turns the box tree into positioned fragments in
//! the galley's continuous top-down coordinate space. Widths resolve top-down
//! (honoring `box-sizing`, `auto` fill, and min/max clamps); heights/positions
//! accumulate as children stack, with **margin collapsing** between siblings,
//! collapse-through of empty blocks, and parent/child collapse of a first child's
//! top margin through a borderless/paddingless parent (a running-margin model;
//! negative margins collapse per CSS 2.1). `Lines` boxes defer to the inline
//! builder (which places atomic `inline-block`s within the line, honouring their
//! `vertical-align` + margins); `Flex`/`Grid`/`Table` dispatch to their own modules.
//!
//! Deferred in v1 (documented): some `%`/auto explicit-height edge cases and full
//! `border-collapse` border merging (see `table.rs`).

use crate::error::Diagnostics;
use crate::layout_snapshot::UsedBoxMetrics;
use crate::tag::MarkerKind;
use crate::text::{Align, FontRegistry};
use std::collections::{HashMap, HashSet};

use super::boxgen::{BoxKind, ImageSource, InlineItem, LayoutBox};
use super::fragment::{BreakMeta, Fragment, FragmentContent, ImagePlacement, NodeId, Transform2D};
use super::imgsize::{size_replaced, SizeCtx, SizedImage};
use super::inline::{self, InlineRun};
use super::value::{
    resolve_box_style, BoxSizing, BoxStyle, Float, LengthPct, Position, ResolveCtx, VAlign,
    DEFAULT_FONT_SIZE,
};
use super::ImageCtx;

/// Shared layout inputs threaded through the recursion. Shared with `flex.rs`.
#[derive(Default)]
pub(crate) struct TextLayoutPolicy {
    pub(crate) start: Option<(u32, bool, Option<f32>)>,
    pub(crate) original_units:
        Option<std::sync::Arc<crate::layout_snapshot::text_units::NativeUnitMap>>,
}

pub(crate) struct Ctx<'a> {
    pub(crate) fonts: &'a FontRegistry,
    pub(crate) images: &'a ImageCtx<'a>,
    pub(crate) diags: &'a mut Diagnostics,
    /// Containing block for `position:absolute` descendants — the content-box
    /// origin (`x`,`y`) + width of the nearest positioned ancestor (the page at
    /// the root). `position:fixed` uses the page origin (`0,0`,[`Ctx::root_w`]).
    pub(crate) abs_cb_x: f32,
    pub(crate) abs_cb_y: f32,
    pub(crate) abs_cb_w: f32,
    /// The nearest positioned ancestor's **definite** content-box height, if known
    /// (0 = unknown) — the basis a `%` height / `inset:0` bottom resolves against.
    /// Set only when the ancestor's height is explicit (not derived from content),
    /// so an `<img height:100%>` in a sized hero card gets a real height.
    pub(crate) abs_cb_h: f32,
    /// Native CSS distinguishes a definite zero from an unresolved auto height.
    pub(crate) abs_cb_h_known: bool,
    /// The nearest ancestor's **definite** content-box height for *in-flow* `%`
    /// height resolution (0 = unknown). Unlike [`Ctx::abs_cb_h`] (positioned CBs)
    /// this is set for ANY box with a definite height, so a normal-flow
    /// `<img height:100%>` / `max-height:100%` inside a fixed-height parent gets a
    /// real height instead of falling back to its intrinsic size.
    pub(crate) cb_h: f32,
    /// Paired with `cb_h` so native percentage sizing retains definite zero.
    pub(crate) cb_h_known: bool,
    /// The initial containing block width (page content width) — the CB for
    /// `position:fixed` boxes.
    pub(crate) root_w: f32,
    pub(crate) root_h: f32,
    /// The initial containing block's height on the static path (the viewport height
    /// of [`crate::layout_in_viewport`]; 0 = unknown): what `position: fixed` boxes and
    /// absolutely positioned boxes whose containing block is the initial one resolve
    /// `bottom` / `%` insets and `%` heights against (miaskiewicz/turbo-dom#81). The live
    /// path keeps its own viewport bases (`root_h`, `abs_cb_h`).
    pub(crate) icb_h: f32,
    /// Whether the containing block of `absolute` descendants is the initial containing
    /// block (no positioned ancestor yet); saved/restored with `abs_cb_*`.
    pub(crate) abs_cb_icb: bool,
    /// Live snapshots use viewport geometry rather than pagination image caps.
    pub(crate) live_geometry: bool,
    pub(crate) used_metrics: Option<HashMap<NodeId, UsedBoxMetrics>>,
    pub(crate) text_capture: Option<crate::layout_snapshot::text::TextCapture>,
    pub(crate) html_font_metrics: bool,
    pub(crate) html_font_scale: Option<f32>,
    pub(crate) original_units: Option<&'a crate::layout_snapshot::text_units::NativeUnitMap>,
    pub(crate) native_transforms: Option<HashMap<NodeId, super::value::NativeTransform>>,
    /// Source-recording mode: the boxes laid out as table rows (see
    /// [`crate::layout::NodeSources::table_rows`]). `None` = not recording.
    pub(crate) table_rows: Option<std::collections::HashSet<NodeId>>,
    /// Floats active in the current **block formatting context**, in absolute
    /// galley coordinates. In-flow content within the same BFC flows in the
    /// inline region these leave free (a `float:right` infobox narrows the text
    /// column to its left), across block *and* sibling-section boundaries — a
    /// float belongs to its BFC, not to the nearest block. Saved/cleared when a
    /// box establishes a new BFC (`float`, out-of-flow, `overflow`≠visible,
    /// flex/grid/table), and that box grows to contain the floats placed inside.
    pub(crate) floats: Vec<FloatRect>,
    /// The boxes whose last child's bottom margin would collapse through their bottom
    /// edge but whose used height is not their content height (`min-height` /
    /// `max-height` / `aspect-ratio` changed it): Chrome drops that margin — it neither
    /// collapses through nor counts inside (miaskiewicz/turbo-dom#75). Kept for the
    /// latest layout of each box, read by [`trailing_margin`] right after it.
    pub(crate) dropped_end_margins: HashSet<NodeId>,
}

/// A placed float's margin box in absolute galley coordinates, plus which edge
/// it packs against. Consulted to narrow the inline region of in-flow content
/// whose vertical span overlaps `[top, bottom)`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FloatRect {
    x0: f32,
    x1: f32,
    top: f32,
    bottom: f32,
    side: Float,
}

/// The containing-block origin (x, y) + width a positioned box resolves its
/// insets against: the page for `fixed`, else the nearest positioned ancestor.
fn containing_block(bs: &BoxStyle, ctx: &Ctx) -> (f32, f32, f32) {
    if bs.position == Position::Fixed {
        (0.0, 0.0, ctx.root_w)
    } else {
        (ctx.abs_cb_x, ctx.abs_cb_y, ctx.abs_cb_w)
    }
}

/// The border-box top-left of an out-of-flow box. `static_pos` is where the box
/// would sit in normal flow — the top-left **margin** edge of its hypothetical box
/// (CSS 2.1 §10.3.7 / §10.6.4): per CSS an `auto` inset resolves to that **static
/// position**, NOT the containing-block origin — so a `position:absolute` decoration
/// with no `top`/`left` stays where it is in the document (e.g. a navbox's rotated
/// label at the bottom) instead of jumping to the top-left of the CB and piling up
/// with every other no-offset absolute. Each axis adds the box's own start margin
/// exactly once; a `right`-anchored box sits its `margin-right` inside the inset
/// (miaskiewicz/turbo-dom#28 item 2).
fn out_of_flow_origin(
    lb: &LayoutBox,
    bs: &BoxStyle,
    cb_width: f32,
    ctx: &Ctx,
    static_pos: (f32, f32),
) -> (f32, f32) {
    let (cbx, cby, cbw) = containing_block(bs, ctx);
    let bbw = auto_positioned_width(lb, bs, static_pos.0 + bs.margin.left, ctx)
        .unwrap_or_else(|| border_box_width(bs, cbw.max(cb_width)));
    let x = match (bs.inset_left.resolve(cbw), bs.inset_right.resolve(cbw)) {
        (Some(l), Some(r)) if auto_margins(lb, "margin-left", "margin-right") => {
            let free = cbw - l - r - bbw;
            match bs.width.resolve(cbw) {
                Some(_) => cbx + l + (free / 2.0).max(0.0),
                None => cbx + l + bs.margin.left,
            }
        }
        (Some(l), _) => cbx + l + bs.margin.left,
        (None, Some(r)) => cbx + cbw - bbw - r - bs.margin.right,
        (None, None) => static_pos.0 + bs.margin.left,
    };
    // `top`/`bottom` resolve against the CB *height* (`abs_cb_h`), not its width —
    // a `top:calc(66% - 48px)` overlay on a card must land at that fraction of the
    // card's height. When the CB height is still indefinite the box is deferred
    // (`place_deferred_abs`), so `abs_cb_h` here is definite.
    let cbh = out_of_flow_height(bs, ctx);
    let y = match (bs.inset_top.resolve(cbh), bs.inset_bottom.resolve(cbh)) {
        (Some(t), Some(b)) if auto_margins(lb, "margin-top", "margin-bottom") => {
            let extra = bs.padding.vertical() + bs.border.widths().vertical();
            match bs.height.resolve(cbh) {
                Some(h) if cbh > 0.0 => {
                    let free = cbh - t - b - to_border(h, bs.box_sizing, extra);
                    return (x, cby + t + (free / 2.0).max(0.0));
                }
                _ => cby + t,
            }
        }
        (Some(t), _) => cby + t,
        // A `bottom`-only box needs its own (laid) height to anchor; the caller's
        // `anchor_bottom` corrects it once known. Until then keep its static y.
        (None, Some(_) | None) => static_pos.1,
    };
    (x, y + bs.margin.top)
}

/// Whether both of an absolutely positioned box's margins on one axis are `auto`
/// (CSS 2.2 §10.3.7 / §10.6.4: they then split the free space between the insets).
fn auto_margins(lb: &LayoutBox, start: &str, end: &str) -> bool {
    let is_auto = |p| lb.style.get(p).map(str::trim) == Some("auto");
    is_auto(start) && is_auto(end)
}

/// The static position of an out-of-flow box between blocks: the flow's left content
/// edge and the cursor after the margins collapsed so far. The box's own margins do not
/// join that run; [`out_of_flow_origin`] adds them (Chrome 156).
fn block_static_position(cx: f32, flow: &FlowRun) -> (f32, f32) {
    (cx, flow.cursor + flow.pending)
}

/// The `(dx, dy)` a `position:relative` box is visually shifted by (it still
/// occupies its normal-flow space). `left`/`top` win over `right`/`bottom`.
fn relative_offset(bs: &BoxStyle, cb_width: f32) -> (f32, f32) {
    let dx = bs
        .inset_left
        .resolve(cb_width)
        .or_else(|| bs.inset_right.resolve(cb_width).map(|r| -r))
        .unwrap_or(0.0);
    let dy = bs
        .inset_top
        .resolve(cb_width)
        .or_else(|| bs.inset_bottom.resolve(cb_width).map(|b| -b))
        .unwrap_or(0.0);
    (dx, dy)
}

fn resolve(lb: &LayoutBox, cb_width: f32, parent_fs: f32) -> BoxStyle {
    lb.resolved(ResolveCtx {
        parent_font_size: parent_fs,
        cb_width,
    })
}

// --------------------------------------------------------------------------
// width resolution
// --------------------------------------------------------------------------

/// A `width`-like length `value` as a border-box width, given the box's horizontal
/// padding + border `extra`: under `box-sizing: border-box` the length is the border box,
/// never narrower than its padding and border (miaskiewicz/turbo-dom#65).
pub(crate) fn to_border(value: f32, sizing: BoxSizing, extra: f32) -> f32 {
    match sizing {
        BoxSizing::BorderBox => value.max(extra),
        BoxSizing::ContentBox => value + extra,
    }
}

fn clamp_width(mut bb: f32, bs: &BoxStyle, cb_width: f32, extra: f32) -> f32 {
    if let Some(min) = bs.min_width.resolve(cb_width) {
        bb = bb.max(to_border(min, bs.box_sizing, extra));
    }
    if let Some(max) = bs.max_width.resolve(cb_width) {
        bb = bb.min(to_border(max, bs.box_sizing, extra));
    }
    bb
}

pub(crate) fn border_box_width(bs: &BoxStyle, cb_width: f32) -> f32 {
    let avail = (cb_width - bs.margin.horizontal()).max(0.0);
    let extra = bs.padding.horizontal() + bs.border.widths().horizontal();
    let bb = match bs.width.resolve(cb_width) {
        None => avail,
        Some(w) => to_border(w, bs.box_sizing, extra),
    };
    clamp_width(bb, bs, cb_width, extra)
}

/// A box's **definite** content-box height, or 0 when it isn't known without
/// laying out content: an explicit `px` height, or a `%` height resolved against a
/// (definite) outer CB height. `auto`/an unresolvable `%` → 0. Used to expose a
/// positioned box's height to its `%`-height descendants.
fn definite_content_height(bs: &BoxStyle, outer_cb_h: f32) -> f32 {
    let h = match bs.height {
        LengthPct::Px(px) => px,
        LengthPct::Auto => return 0.0,
        // `%` / `calc(% ± px)` need a definite outer height to resolve against.
        pct_or_calc if outer_cb_h > 0.0 => pct_or_calc.resolve(outer_cb_h).unwrap_or(0.0),
        _ => return 0.0,
    };
    // A content-box height excludes padding/border; a border-box height includes
    // them, so subtract to recover the content height the CB exposes.
    let inset = match bs.box_sizing {
        BoxSizing::BorderBox => bs.padding.vertical() + bs.border.widths().vertical(),
        BoxSizing::ContentBox => 0.0,
    };
    (h - inset).max(0.0)
}

/// The containing-block height a box resolves its `%` `height` / `min-height` /
/// `max-height` against on the static path: a positioned box's `abs_cb_h` (so a
/// `bottom:0; height:33%` overlay fills its band on a card sized by its content), an
/// in-flow box's parent's definite content height `cb_h` (miaskiewicz/turbo-dom#78), a fixed
/// box's or an initial-containing-block absolute box's known initial containing block
/// height `icb_h` (#81). 0 = indefinite: the percentages then behave as `auto` / 0 / `none`.
fn percentage_height_basis(bs: &BoxStyle, ctx: &Ctx) -> f32 {
    if bs.position == Position::Static {
        ctx.cb_h
    } else if in_known_icb(bs, ctx) {
        ctx.icb_h
    } else {
        ctx.abs_cb_h
    }
}

fn live_child_height(lb: &LayoutBox, bs: &BoxStyle, outer_height: f32, live: bool) -> f32 {
    // The anonymous galley root represents the initial containing block, not
    // an auto-height DOM element whose children lose the viewport basis.
    if live && lb.node_id == NodeId(0) {
        return outer_height;
    }
    definite_content_height(bs, outer_height)
}

fn resolved_content_height(bs: &BoxStyle, auto_height: f32, ctx: &Ctx) -> f32 {
    if !ctx.live_geometry {
        return content_box_height(bs, auto_height, percentage_height_basis(bs, ctx));
    }
    let basis = native_height_basis(bs, ctx);
    let frame = bs.padding.vertical() + bs.border.widths().vertical();
    let inset = if bs.box_sizing == BoxSizing::BorderBox {
        frame
    } else {
        0.0
    };
    let content = resolved_live_height(bs.height, basis, inset).unwrap_or(auto_height);
    let max = resolved_live_height(bs.max_height, basis, inset).unwrap_or(f32::INFINITY);
    let min = resolved_live_height(bs.min_height, basis, inset).unwrap_or(0.0);
    content.min(max).max(min)
}

// Absolute positioning uses the padding box; normal-flow children use content.
fn positioned_child_block(
    bs: &BoxStyle,
    frame: (f32, f32, f32),
    ctx: &Ctx,
) -> (f32, f32, f32, f32) {
    let border = bs.border.widths();
    if !ctx.live_geometry {
        // The padding box (miaskiewicz/turbo-dom#100); 0 = an indefinite height. A
        // definite zero content height (`height: 0; padding-bottom: 56%`) still has its
        // padding.
        let basis = percentage_height_basis(bs, ctx);
        let definite = match bs.height {
            LengthPct::Px(_) => true,
            LengthPct::Auto => false,
            _ => basis > 0.0,
        };
        let height = if definite {
            definite_content_height(bs, basis) + bs.padding.vertical()
        } else {
            0.0
        };
        return (
            frame.0 + border.left,
            frame.1 + border.top,
            frame.2 - border.horizontal(),
            height,
        );
    }
    let inset = match bs.box_sizing {
        BoxSizing::BorderBox => border.vertical() + bs.padding.vertical(),
        BoxSizing::ContentBox => 0.0,
    };
    let height = resolved_live_height(bs.height, native_height_basis(bs, ctx), inset)
        .map_or(0.0, |h| h + bs.padding.vertical());
    (
        frame.0 + border.left,
        frame.1 + border.top,
        frame.2 - border.horizontal(),
        height,
    )
}

/// A positioned box's padding-box height from its laid-out `content` height, for its
/// deferred absolute children (miaskiewicz/turbo-dom#100: on every path).
fn deferred_cb_height(content: f32, bs: &BoxStyle) -> f32 {
    content + bs.padding.vertical()
}

fn layout_positioned(
    lb: &LayoutBox,
    origin: (f32, f32),
    width: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let width = out_of_flow_width(&resolve(lb, width, fs), ctx);
    if lb.is_replaced() {
        return layout_box(lb, origin.0, origin.1, width, fs, ctx);
    }
    let mut bs = resolve(lb, width, fs);
    if !ctx.live_geometry {
        return layout_positioned_static(lb, bs, origin, (width, fs), ctx);
    }
    let border = bs.border.widths();
    let frame = match bs.box_sizing {
        BoxSizing::ContentBox => (
            bs.padding.horizontal() + border.horizontal(),
            bs.padding.vertical() + border.vertical(),
        ),
        BoxSizing::BorderBox => (0.0, 0.0),
    };
    bs.width = stretch_insets(
        bs.width,
        (bs.inset_left, bs.inset_right),
        width,
        bs.margin.horizontal() + frame.0,
    );
    bs.height = stretch_insets(
        bs.height,
        (bs.inset_top, bs.inset_bottom),
        out_of_flow_height(&bs, ctx),
        bs.margin.vertical() + frame.1,
    );
    let border_width = auto_positioned_width(lb, &bs, origin.0, ctx)
        .unwrap_or_else(|| border_box_width(&bs, width));
    layout_box_sized(lb, &bs, origin.0, origin.1, border_width, ctx)
}

/// [`layout_positioned`] on the static path: shrink-to-fit `width: auto`
/// ([`auto_positioned_width`]) and the `top` + `bottom` stretched auto height
/// ([`static_stretched_height`], miaskiewicz/turbo-dom#89).
fn layout_positioned_static(
    lb: &LayoutBox,
    mut bs: BoxStyle,
    origin: (f32, f32),
    (width, fs): (f32, f32),
    ctx: &mut Ctx,
) -> Fragment {
    let stretched = static_stretched_height(lb, &bs, ctx);
    if let Some(height) = stretched {
        bs.height = height;
    }
    match auto_positioned_width(lb, &bs, origin.0, ctx) {
        Some(border_width) => layout_box_sized(lb, &bs, origin.0, origin.1, border_width, ctx),
        None if stretched.is_some() => {
            let border_width = border_box_width(&bs, width);
            layout_box_sized(lb, &bs, origin.0, origin.1, border_width, ctx)
        }
        None => layout_box(lb, origin.0, origin.1, width, fs, ctx),
    }
}

/// The used border-box width of an absolutely positioned non-replaced box with
/// `width: auto` (CSS 2.1 §10.3.7; miaskiewicz/turbo-dom#48): stretched between
/// `left` and `right` when both are set, else shrink-to-fit —
/// `min(max(min-content, available), max-content)`, the available width running from
/// the set inset (or the static position, `border_x` being the border-box x there)
/// to the far edge of the containing block — then `min-width` / `max-width`. `None`
/// for any other box.
fn auto_positioned_width(lb: &LayoutBox, bs: &BoxStyle, border_x: f32, ctx: &Ctx) -> Option<f32> {
    if bs.width != LengthPct::Auto || lb.is_replaced() {
        return None;
    }
    let (cbx, _, cbw) = containing_block(bs, ctx);
    let margins = bs.margin.horizontal();
    let insets = (bs.inset_left.resolve(cbw), bs.inset_right.resolve(cbw));
    let width = if let (Some(left), Some(right)) = insets {
        cbw - left - right - margins
    } else {
        let start = insets
            .0
            .or(insets.1)
            .unwrap_or(border_x - bs.margin.left - cbx);
        let available = cbw - start - margins;
        let fonts = ctx.fonts;
        let min = super::flex::min_content_width(lb, fonts);
        super::flex::natural_width(lb, fonts).min(available.max(min))
    };
    let extra = bs.padding.horizontal() + bs.border.widths().horizontal();
    Some(clamp_width(width.max(0.0), bs, cbw, extra))
}

/// The static-path used `height` of an absolutely positioned (or fixed) non-table box
/// with `height: auto` and both `top` and `bottom` set: the containing block's height
/// less the insets, its vertical margins and (content-box) its padding and border (CSS
/// 2.2 §10.6.4; miaskiewicz/turbo-dom#89), as the live path stretches it. `None` when it
/// does not apply or the containing block's height is unknown (0).
fn static_stretched_height(lb: &LayoutBox, bs: &BoxStyle, ctx: &Ctx) -> Option<LengthPct> {
    let cbh = out_of_flow_height(bs, ctx);
    if bs.height != LengthPct::Auto || cbh <= 0.0 || matches!(lb.kind, BoxKind::Table(_)) {
        return None;
    }
    let frame = match bs.box_sizing {
        BoxSizing::ContentBox => bs.padding.vertical() + bs.border.widths().vertical(),
        BoxSizing::BorderBox => 0.0,
    };
    let insets = (bs.inset_top, bs.inset_bottom);
    let height = stretch_insets(bs.height, insets, cbh, bs.margin.vertical() + frame);
    (height != LengthPct::Auto).then_some(height)
}

fn stretch_insets(
    value: LengthPct,
    insets: (LengthPct, LengthPct),
    basis: f32,
    frame: f32,
) -> LengthPct {
    if value != LengthPct::Auto {
        return value;
    }
    let available = insets
        .0
        .resolve(basis)
        .zip(insets.1.resolve(basis))
        .map(|(start, end)| (basis - start - end - frame).max(0.0));
    available.map_or(value, LengthPct::Px)
}

/// An out-of-flow box's style with its percentages (horizontal margins and padding too)
/// resolved against its containing block's width ([`out_of_flow_width`]) where its
/// origin is computed; `cw` is its parent's content width (miaskiewicz/turbo-dom#100).
fn positioned_style(kid: &LayoutBox, cw: f32, fs: f32, ctx: &Ctx) -> BoxStyle {
    resolve(kid, out_of_flow_width(&resolve(kid, cw, fs), ctx), fs)
}

/// The width an out-of-flow box resolves its percentages (`width`, `min-` / `max-width`,
/// horizontal padding and margins) against: its containing block's, on every path — the
/// nearest positioned ancestor's, the viewport's for `fixed` (CSS 2.2 §10.2;
/// miaskiewicz/turbo-dom#100; the static path used the parent's width).
fn out_of_flow_width(bs: &BoxStyle, ctx: &Ctx) -> f32 {
    containing_block(bs, ctx).2
}

fn out_of_flow_height(bs: &BoxStyle, ctx: &Ctx) -> f32 {
    if ctx.live_geometry {
        live_height_basis(bs, ctx)
    } else if in_known_icb(bs, ctx) {
        ctx.icb_h
    } else {
        ctx.abs_cb_h
    }
}

/// Whether a static-path box is positioned against the initial containing block of a
/// known height (miaskiewicz/turbo-dom#81): a `position: fixed` box, or an absolutely
/// positioned one with no positioned ancestor. With the height unknown (`icb_h == 0`,
/// e.g. a paged layout) such a box keeps the containing-block basis it had before.
fn in_known_icb(bs: &BoxStyle, ctx: &Ctx) -> bool {
    ctx.icb_h > 0.0
        && match bs.position {
            Position::Fixed => true,
            Position::Absolute => ctx.abs_cb_icb,
            _ => false,
        }
}

fn live_height_basis(bs: &BoxStyle, ctx: &Ctx) -> f32 {
    match bs.position {
        Position::Absolute => ctx.abs_cb_h,
        Position::Fixed => ctx.root_h,
        _ => ctx.cb_h,
    }
}

fn native_height_basis(bs: &BoxStyle, ctx: &Ctx) -> Option<f32> {
    if !ctx.html_font_metrics {
        let height = live_height_basis(bs, ctx);
        return (height > 0.0).then_some(height);
    }
    let (height, known) = match bs.position {
        Position::Absolute => (ctx.abs_cb_h, ctx.abs_cb_h_known),
        Position::Fixed => (ctx.root_h, true),
        _ => (ctx.cb_h, ctx.cb_h_known),
    };
    known.then_some(height)
}

fn resolved_live_height(value: LengthPct, basis: Option<f32>, inset: f32) -> Option<f32> {
    match value {
        LengthPct::Px(px) => Some((px - inset).max(0.0)),
        LengthPct::Auto => None,
        _ => basis.map(|basis| (value.resolve(basis).unwrap_or(0.0) - inset).max(0.0)),
    }
}

fn content_box_height(bs: &BoxStyle, content_h: f32, pct_basis: f32) -> f32 {
    // A `px` `height`/`min-height`/`max-height` is a BORDER-box length under
    // `box-sizing:border-box`, so recover the content height by subtracting the
    // padding+border. Google's header "Sign in" pill is `min-height:40px;padding:10px
    // 12px;border:1px` under the reset's border-box: 40 is the whole pill, so its
    // content is 18 and the box is 40 tall — treating 40 as content made a 62px oval.
    let inset = match bs.box_sizing {
        BoxSizing::BorderBox => bs.padding.vertical() + bs.border.widths().vertical(),
        BoxSizing::ContentBox => 0.0,
    };
    // A `%` / `calc(% ± px)` length resolves against a definite basis
    // ([`percentage_height_basis`]); against an indefinite one it does not apply.
    let to_content = |lp: LengthPct| match lp {
        LengthPct::Px(v) => Some((v - inset).max(0.0)),
        LengthPct::Pct(_) | LengthPct::Calc { .. } if pct_basis > 0.0 => {
            lp.resolve(pct_basis).map(|v| (v - inset).max(0.0))
        }
        _ => None,
    };
    let mut h = to_content(bs.height).unwrap_or(content_h);
    // CSS 2.2 §10.7: `max-height` clamps first, then `min-height` wins over both it and
    // `height` (miaskiewicz/turbo-dom#77).
    if let Some(max) = to_content(bs.max_height) {
        h = h.min(max);
    }
    if let Some(min) = to_content(bs.min_height) {
        h = h.max(min);
    }
    h
}

// --------------------------------------------------------------------------
// inline (Lines) layout
// --------------------------------------------------------------------------

fn text_run(item: &InlineItem, parent_fs: f32, cw: f32, fonts: &FontRegistry) -> Option<InlineRun> {
    let InlineItem::Text {
        node_id,
        style,
        text,
    } = item
    else {
        return None;
    };
    let bs = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: parent_fs,
            cb_width: cw,
        },
    );
    let families: Vec<&str> = bs.font_families.iter().map(String::as_str).collect();
    let face = fonts.select(&families, bs.font_weight, bs.italic)?.clone();
    Some(InlineRun {
        node_id: *node_id,
        text: text.clone(),
        face,
        families: bs.font_families.clone(),
        weight: bs.font_weight,
        italic: bs.italic,
        font_size: bs.font_size,
        line_height: bs.line_height,
        letter_spacing: bs.letter_spacing,
        color: bs.color,
        valign: bs.vertical_align,
        nowrap: matches!(
            style.get("white-space").map(str::trim),
            Some("nowrap" | "pre")
        ),
        preserve_spaces: matches!(
            style.get("white-space").map(str::trim),
            Some("pre" | "pre-wrap" | "break-spaces")
        ),
        forced_newlines: matches!(
            style.get("white-space").map(str::trim),
            Some("pre" | "pre-wrap" | "break-spaces" | "pre-line")
        ),
    })
}

/// The block's line-box strut height: the minimum height of every line box it
/// establishes — its explicit `line-height`, else its own font's natural line
/// height. Empty lines (from consecutive `<br>`) are exactly this tall, matching a
/// browser's strut.
fn line_strut(bs: &BoxStyle, fonts: &FontRegistry) -> f32 {
    if let Some(lh) = bs.line_height {
        return lh;
    }
    let families: Vec<&str> = bs.font_families.iter().map(String::as_str).collect();
    match fonts.select(&families, bs.font_weight, bs.italic) {
        Some(face) => face.line_height_px(bs.font_size),
        None => bs.font_size * 1.2,
    }
}

pub(crate) fn build_runs(
    items: &[InlineItem],
    parent_fs: f32,
    cw: f32,
    fonts: &FontRegistry,
) -> Vec<InlineRun> {
    items
        .iter()
        .filter_map(|it| text_run(it, parent_fs, cw, fonts))
        .collect()
}

fn lines_to_fragments(
    para: &inline::ParagraphSourceLayout,
    cx: f32,
    cy: f32,
    out: &mut Vec<Fragment>,
    capture: &mut Option<crate::layout_snapshot::text::TextCapture>,
    offsets: &HashMap<NodeId, (f32, f32)>,
) {
    for (line_index, line) in para.layout.lines.iter().enumerate() {
        for (run_index, gr) in line.runs.iter().enumerate() {
            let (dx, dy) = offsets.get(&gr.node_id).copied().unwrap_or_default();
            let node_id = capture.as_mut().map_or(gr.node_id, |capture| {
                capture.record(
                    gr.node_id,
                    &para.clusters[line_index][run_index],
                    para.font_boxes[line_index][run_index],
                )
            });
            let content = FragmentContent::TextLine {
                glyphs: gr.glyphs.clone(),
                text: gr.text.clone(),
                face: gr.face.clone(),
                font_size: gr.font_size,
                color: gr.color,
            };
            out.push(Fragment::new(
                node_id,
                cx + line.indent + dx,
                cy + line.top + dy,
                line.width,
                line.height,
                content,
            ));
        }
    }
}

fn marker_frag(node_id: NodeId, kind: MarkerKind, x: f32, y: f32) -> Fragment {
    Fragment::new(node_id, x, y, 0.0, 0.0, FragmentContent::Marker(kind))
}

/// Lay out an atomic inline (`inline-block` / replaced `<img>`) at the origin so
/// its size is known; the caller translates it to its placed line position. An
/// auto-width, non-replaced `inline-block` shrinks to its content (else it would
/// fill the line); replaced/explicit-width boxes size themselves normally. An
/// `inline-block` is a block formatting context root, so its first / last child's
/// vertical margins stay inside it (miaskiewicz/turbo-dom#80).
fn lay_atomic(b: &LayoutBox, cw: f32, fs: f32, ctx: &mut Ctx) -> Fragment {
    let bs = resolve(b, cw, fs);
    let replaced = b.is_replaced();
    let root = inline_block(b);
    if !replaced && bs.width.resolve(cw).is_none() {
        // Shrink-to-fit, then honor `min-width`/`max-width` — google's header "Sign in"
        // pill has `min-width:85px`; unclamped it shrank to its ~68px text and, nearly
        // as tall as wide under `border-radius:100px`, rendered as a circle not a pill.
        let extra = bs.padding.horizontal() + bs.border.widths().horizontal();
        // CSS shrink-to-fit = min(max-content, max(available, min-content)). Flooring
        // at min-content (not a bare `.min(cw)`) lets a box whose content can't fit —
        // a `white-space:nowrap` tab wider than its parent — overflow at its full
        // width instead of being clamped to the parent and dropping text.
        let nat = super::flex::natural_width(b, ctx.fonts);
        let mc = super::flex::min_content_width(b, ctx.fonts);
        let w = clamp_width(nat.min(cw.max(mc)), &bs, cw, extra);
        let w = super::value::html_used_value(w, ctx.html_font_scale);
        layout_box_sized_impl(b, &bs, 0.0, 0.0, w, ctx, root)
    } else if root {
        let bbw = border_box_width(&bs, cw);
        layout_box_sized_isolated(b, &bs, 0.0, 0.0, bbw, ctx)
    } else {
        layout_box(b, 0.0, 0.0, cw, fs, ctx)
    }
}

/// Whether an atomic inline is an `inline-block` (as specified: it lays out as a block
/// container that establishes a block formatting context). A nested block the generic
/// policy keeps atomic inside an inline is not.
fn inline_block(b: &LayoutBox) -> bool {
    b.style.get("display").map(str::trim) == Some("inline-block")
}

/// Lay out an inline formatting context: text runs and atomic inline boxes flow
/// together into line boxes (atoms are unbreakable units placed on the baseline),
/// so an `inline-block`/`<img>` sits *within* the line next to text rather than
/// stacking below it. Inline directives become zero-size markers at the origin.
fn layout_lines(
    lb: &LayoutBox,
    items: &[InlineItem],
    bs: &BoxStyle,
    cx: f32,
    cy: f32,
    cw: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    let placement = super::inline_boxes::positions(items, bs.font_size, cw, ctx);
    if let Some(empty) =
        super::inline_boxes::empty_fragments(items, bs.font_size, (cx, cy), cw, &placement, ctx)
    {
        return (empty, 0.0);
    }
    layout_nonempty_lines(lb, items, bs, (cx, cy), cw, ctx, placement)
}
fn layout_nonempty_lines(
    lb: &LayoutBox,
    items: &[InlineItem],
    bs: &BoxStyle,
    origin: (f32, f32),
    cw: f32,
    ctx: &mut Ctx,
    placement: super::inline_boxes::SourcePlacement,
) -> (Vec<Fragment>, f32) {
    let (cx, cy) = origin;
    let (mut pieces, atom_frags, directives) = build_inline_pieces(items, bs, cx, cy, cw, ctx);
    super::inline_boxes::native_atoms(
        &mut pieces,
        &atom_frags,
        &placement.baselines,
        ctx.html_font_metrics,
    );
    // Per-line float wrapping: each line's `(indent, width)` is the free inline
    // region the BFC's floats leave at that line's own y (absolute `cy + top`), so
    // text flows beside a `float:right` infobox and widens once below its bottom.
    let text_indent = lb
        .style
        .get("text-indent")
        .and_then(|value| super::value::parse_length_pct(value, bs.font_size))
        .and_then(|value| value.resolve(cw))
        .unwrap_or(0.0);
    let inputs = ParagraphInputs {
        origin: (cx, cy),
        cw,
        text_indent: super::value::html_used_value(text_indent, ctx.html_font_scale),
        strut: line_strut(bs, ctx.fonts),
        strut_metrics: super::inline_boxes::strut(bs, ctx),
        align: bs.text_align,
        font_size: bs.font_size,
        baselines: &placement.baselines,
    };
    // A float met inside the line content (miaskiewicz/turbo-dom#28 item 4) is placed
    // at the top of the line it lands on and narrows that line and the ones beside
    // it: re-lay the lines until the floats' places settle (scratch diagnostics),
    // then lay them out for real.
    let base = ctx.floats.clone();
    let line = LineContent {
        pieces: &pieces,
        items,
        atoms: &atom_frags,
    };
    let line_floats = settle_line_floats(&line, &base, &inputs, ctx);
    let floats = [base.as_slice(), &rects(&line_floats)].concat();
    let mut diags = std::mem::take(ctx.diags);
    let para = paragraph(&pieces, &floats, &inputs, ctx, &mut diags);
    *ctx.diags = diags;
    let line_floats = if line_floats.is_empty() {
        line_floats
    } else {
        place_line_floats(&para.layout, items, &atom_frags, &base, &inputs, ctx)
    };
    ctx.floats.extend(rects(&line_floats));
    let offsets = placement.offsets;
    let mut frags = Vec::new();
    frags.extend(super::inline_boxes::fragments(
        &para.layout,
        items,
        bs.font_size,
        (cx, cy),
        cw,
        ctx,
        &offsets,
    ));
    lines_to_fragments(&para, cx, cy, &mut frags, &mut ctx.text_capture, &offsets);
    // Translate each pre-laid atom to where it landed within its line.
    let atoms = AtomPlacement {
        items,
        atoms: &atom_frags,
        offsets: &offsets,
        origin: (cx, cy),
        cw,
        font_size: bs.font_size,
        floats: &line_floats,
    };
    for line in &para.layout.lines {
        for placed in &line.atoms {
            frags.push(place_atom(line, placed, &atoms, ctx));
        }
    }
    frags.extend(directives);
    (frags, para.layout.height)
}

/// What placing a paragraph's atoms reads: its items, the pre-laid atoms, the inline
/// source offsets and the content box.
struct AtomPlacement<'a> {
    items: &'a [InlineItem],
    atoms: &'a [Fragment],
    offsets: &'a HashMap<NodeId, (f32, f32)>,
    origin: (f32, f32),
    cw: f32,
    font_size: f32,
    /// Where each in-line float was placed.
    floats: &'a [LineFloat],
}

/// How many times a paragraph with floats inside its line content is re-laid at most
/// while their places settle (each pass places them at the tops of the lines they
/// landed on in the previous one).
const IN_LINE_FLOAT_PASSES: usize = 4;

/// The paragraph geometry every line layout pass shares.
struct ParagraphInputs<'a> {
    origin: (f32, f32),
    cw: f32,
    text_indent: f32,
    strut: f32,
    strut_metrics: Option<(f32, f32)>,
    align: Align,
    font_size: f32,
    baselines: &'a HashMap<NodeId, f32>,
}

/// A float met in line content, placed: its id, border-box origin and margin box.
#[derive(Clone, Copy)]
struct LineFloat {
    node_id: NodeId,
    origin: (f32, f32),
    rect: FloatRect,
}

fn rects(floats: &[LineFloat]) -> Vec<FloatRect> {
    floats.iter().map(|float| float.rect).collect()
}

/// The floated box of an inline item, if it is one (an absolutely positioned box
/// ignores `float`).
fn floated_kid(item: &InlineItem) -> Option<&LayoutBox> {
    match item {
        InlineItem::Atomic(b) if !b.out_of_flow() && b.floated() => Some(b),
        _ => None,
    }
}

/// A paragraph's line content: its pieces, items and pre-laid atoms.
struct LineContent<'a> {
    pieces: &'a [inline::Piece],
    items: &'a [InlineItem],
    atoms: &'a [Fragment],
}

/// The places of the floats met in the paragraph's line content, settled: lay the lines
/// out (scratch diagnostics), place the floats at the tops of the lines they land on,
/// and repeat with those floats until their places stop moving (at most
/// [`IN_LINE_FLOAT_PASSES`] times). Empty when the content has no float.
fn settle_line_floats(
    line: &LineContent,
    base: &[FloatRect],
    inputs: &ParagraphInputs,
    ctx: &Ctx,
) -> Vec<LineFloat> {
    let mut line_floats = Vec::new();
    if !line.items.iter().any(|item| floated_kid(item).is_some()) {
        return line_floats;
    }
    let mut scratch = Diagnostics::default();
    for _ in 0..IN_LINE_FLOAT_PASSES {
        let floats = [base, &rects(&line_floats)].concat();
        let para = paragraph(line.pieces, &floats, inputs, ctx, &mut scratch);
        let next = place_line_floats(&para.layout, line.items, line.atoms, base, inputs, ctx);
        let settled = rects(&next) == rects(&line_floats);
        line_floats = next;
        if settled {
            break;
        }
    }
    line_floats
}

/// Lay out the paragraph's lines in the inline region `floats` leave free.
fn paragraph(
    pieces: &[inline::Piece],
    floats: &[FloatRect],
    inputs: &ParagraphInputs,
    ctx: &Ctx,
    diags: &mut Diagnostics,
) -> inline::ParagraphSourceLayout {
    let ((cx, cy), cw, text_indent) = (inputs.origin, inputs.cw, inputs.text_indent);
    let region = |top: f32| {
        let (rx, rw) = inline_region_from(floats, cx, cw, cy + top, 1.0);
        let indent = if top == 0.0 { text_indent } else { 0.0 };
        (rx - cx + indent, rw - indent)
    };
    let native_region = |top: f32| {
        let (rx, rw) = inline_region_from(floats, cx, cw, cy + top, 0.0);
        (rx - cx, rw)
    };
    let clearance = |top, needed| inline_clearance(floats, (cx, cy, cw), top, needed);
    if ctx.html_font_metrics {
        inline::layout_html_paragraph_in_with_sources_at_scale(
            pieces,
            ctx.fonts,
            inputs.align,
            diags,
            &native_region,
            inputs.strut,
            inline::NativeLineOptions {
                scale: ctx.html_font_scale,
                first_indent: text_indent,
                clearance: &clearance,
                original_units: ctx.original_units,
                baselines: inputs.baselines,
                strut_metrics: inputs.strut_metrics,
            },
        )
    } else if ctx.text_capture.is_some() {
        inline::layout_paragraph_in_with_sources(
            pieces,
            ctx.fonts,
            inputs.align,
            diags,
            &region,
            inputs.strut,
        )
    } else {
        inline::ParagraphSourceLayout {
            layout: inline::layout_paragraph_in(
                pieces,
                ctx.fonts,
                inputs.align,
                diags,
                &region,
                inputs.strut,
            ),
            clusters: Vec::new(),
            font_boxes: Vec::new(),
        }
    }
}

/// Place the in-line floats of `para` in line order: each at the top of the line
/// its placeholder landed on (dropped below earlier floats it does not fit beside),
/// packed against its edge of the region left free there (CSS 2.1 §9.5.1), exactly
/// as [`place_float`] packs one between blocks.
fn place_line_floats(
    para: &inline::ParagraphLayout,
    items: &[InlineItem],
    atoms: &[Fragment],
    base: &[FloatRect],
    inputs: &ParagraphInputs,
    ctx: &Ctx,
) -> Vec<LineFloat> {
    let ((cx, cy), cw) = (inputs.origin, inputs.cw);
    let minimum = if ctx.html_font_metrics { 0.0 } else { 1.0 };
    let mut floats = base.to_vec();
    let mut placed = Vec::new();
    for line in &para.lines {
        for atom in &line.atoms {
            let frag = &atoms[atom.id];
            let Some(kid) = items
                .iter()
                .filter_map(floated_kid)
                .find(|kid| kid.node_id == frag.node_id)
            else {
                continue;
            };
            let kbs = resolve(kid, cw, inputs.font_size);
            let w = frag.width;
            let y = float_drop_y_in(&floats, (cx, cw, minimum), w, cy + line.top) + kbs.margin.top;
            let (rx, rw) = inline_region_from(&floats, cx, cw, y, minimum);
            let bx = match kbs.float {
                Float::Right => rx + rw - w - kbs.margin.right,
                _ => rx + kbs.margin.left,
            };
            let rect = FloatRect {
                x0: bx - kbs.margin.left,
                x1: bx + w + kbs.margin.right,
                top: y - kbs.margin.top,
                bottom: y + frag.height + kbs.margin.bottom,
                side: kbs.float,
            };
            floats.push(rect);
            placed.push(LineFloat {
                node_id: kid.node_id,
                origin: (bx, y),
                rect,
            });
        }
    }
    placed
}

/// One atom where it landed in `line`; an absolutely positioned box is laid out
/// from that static position instead (its zero-size place is the line top there),
/// and an in-line float goes where [`place_line_floats`] put it.
fn place_atom(
    line: &inline::InlineLine,
    placed: &inline::PlacedAtom,
    atoms: &AtomPlacement,
    ctx: &mut Ctx,
) -> Fragment {
    let atom = &atoms.atoms[placed.id];
    let (dx, dy) = atoms
        .offsets
        .get(&atom.node_id)
        .copied()
        .unwrap_or_default();
    let (cx, cy) = atoms.origin;
    if let Some(float) = atoms.floats.iter().find(|f| f.node_id == atom.node_id) {
        let mut f = atom.clone();
        f.translate(float.origin.0 + dx, float.origin.1 + dy);
        return f;
    }
    let x = cx + line.indent + placed.x + dx;
    if let Some(kid) = out_of_flow_kid(atoms.items, atom.node_id) {
        return place_inline_out_of_flow(
            kid,
            (x, cy + line.top + dy),
            atoms.cw,
            atoms.font_size,
            ctx,
        );
    }
    let mut f = atom.clone();
    f.translate(x, cy + line.top + placed.y + dy);
    f
}

/// The absolutely positioned atomic box of `items` with id `node_id`, if it is one.
fn out_of_flow_kid(items: &[InlineItem], node_id: NodeId) -> Option<&LayoutBox> {
    items.iter().find_map(|item| match item {
        InlineItem::Atomic(b) if b.node_id == node_id && b.out_of_flow() => Some(b),
        _ => None,
    })
}

/// The zero-size line piece an absolutely positioned box leaves at its static
/// position (or a float at its place in the line): no advance, no height, no margins,
/// and no in-flow content for the line box.
fn out_of_flow_atom(id: usize) -> inline::InlineAtom {
    inline::InlineAtom {
        id,
        width: 0.0,
        height: 0.0,
        valign: VAlign::Baseline,
        font_size: 0.0,
        margin_top: 0.0,
        margin_bottom: 0.0,
        margin_left: 0.0,
        margin_right: 0.0,
        placeholder: true,
        baseline: None,
    }
}

/// An atomic inline's own baseline below its border-box top (CSS 2.1 §10.8.1,
/// miaskiewicz/turbo-dom#142): an `inline-block` with `overflow: visible` sits on
/// its last in-flow line box's baseline; a replaced box, a box without line boxes,
/// or one establishing a scroll container has none (its bottom margin edge
/// aligns instead).
fn atom_baseline(b: &LayoutBox, f: &Fragment) -> Option<f32> {
    if b.is_replaced() || !inline_block(b) {
        return None;
    }
    let scrolls = ["overflow", "overflow-x", "overflow-y"].iter().any(|p| {
        matches!(
            b.style.get(p).map(str::trim),
            Some("hidden" | "auto" | "scroll")
        )
    });
    if scrolls {
        return None;
    }
    let mut floats = std::collections::HashSet::new();
    float_ids(b, &mut floats);
    let mut last: Option<(f32, f32)> = None;
    last_line(f, &floats, &mut last);
    last.map(|(_, baseline)| baseline - f.y)
}

/// The node ids of the floats in `b`'s subtree (their lines are out of flow).
fn float_ids(b: &LayoutBox, out: &mut std::collections::HashSet<NodeId>) {
    let kids: Vec<&LayoutBox> = match &b.kind {
        BoxKind::Block(kids) | BoxKind::Flex(kids) | BoxKind::Grid(kids) | BoxKind::Table(kids) => {
            kids.iter().collect()
        }
        BoxKind::Lines(items) => items
            .iter()
            .filter_map(|item| match item {
                InlineItem::Atomic(kid) => Some(kid),
                _ => None,
            })
            .collect(),
        BoxKind::Marker(_) => Vec::new(),
    };
    for kid in kids {
        if super::value::float_of(&kid.style) == Float::None {
            float_ids(kid, out);
        } else {
            out.insert(kid.node_id);
        }
    }
}

/// The `(top, baseline)` of the lowest in-flow text line in `f`'s subtree.
fn last_line(
    f: &Fragment,
    floats: &std::collections::HashSet<NodeId>,
    last: &mut Option<(f32, f32)>,
) {
    let in_flow = |child: &&Fragment| !child.is_positioned && !floats.contains(&child.node_id);
    for child in f.children.iter().filter(in_flow) {
        if let FragmentContent::TextLine { glyphs, .. } = &child.content {
            if let Some(glyph) = glyphs.first() {
                if last.is_none_or(|(top, _)| child.y >= top) {
                    *last = Some((child.y, child.y + glyph.y));
                }
            }
        }
        last_line(child, floats, last);
    }
}

/// Lay out an absolutely positioned box from an inline run at its static position
/// `static_pos` (the line top at its place in the line), as [`place_special_kid`]
/// does for one between blocks.
fn place_inline_out_of_flow(
    kid: &LayoutBox,
    static_pos: (f32, f32),
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let kbs = positioned_style(kid, cw, fs, ctx);
    let (bx, by) = out_of_flow_origin(kid, &kbs, cw, ctx, static_pos);
    let mut frag = layout_positioned(kid, (bx, by), cw, fs, ctx);
    let height = out_of_flow_height(&kbs, ctx);
    if height > 0.0 {
        anchor_bottom(&mut frag, &kbs, height, ctx);
    }
    frag
}

/// Pre-lay the inline sequence in document order, returning `(pieces, atoms,
/// markers)`. Text becomes measured runs; each atomic inline is laid out
/// recursively and returned *separately* (keyed by its index in `atoms`, via the
/// piece's id) so it can be re-placed once its line position is known; directives
/// collapse to zero-size markers at the container origin.
fn append_source_break(
    node_id: NodeId,
    style: &crate::ComputedStyle,
    font: f32,
    width: f32,
    ctx: &Ctx,
    pieces: &mut Vec<inline::Piece>,
) {
    if let Some(mut edge) = super::inline_boxes::edge(node_id, style, true, font, width, ctx) {
        edge.advance = 0.0;
        edge.contributes_metrics = false;
        pieces.push(inline::Piece::SourceBreak(edge));
    }
}
fn build_inline_pieces(
    items: &[InlineItem],
    bs: &BoxStyle,
    cx: f32,
    cy: f32,
    cw: f32,
    ctx: &mut Ctx,
) -> (Vec<inline::Piece>, Vec<Fragment>, Vec<Fragment>) {
    let mut atom_frags: Vec<Fragment> = Vec::new();
    let mut directives: Vec<Fragment> = Vec::new();
    let mut pieces: Vec<inline::Piece> = Vec::new();
    for item in items {
        match item {
            InlineItem::Text { .. } => {
                if let Some(run) = text_run(item, bs.font_size, cw, ctx.fonts) {
                    pieces.push(inline::Piece::Run(run));
                }
            }
            InlineItem::Boundary {
                node_id,
                style,
                start,
            } => {
                if let Some(edge) =
                    super::inline_boxes::edge(*node_id, style, *start, bs.font_size, cw, ctx)
                {
                    pieces.push(inline::Piece::Edge(edge));
                }
            }
            InlineItem::SourceBreak { node_id, style } => {
                append_source_break(*node_id, style, bs.font_size, cw, ctx, &mut pieces);
            }
            InlineItem::LineBreak => pieces.push(inline::Piece::Break),
            // An absolutely positioned box only takes a zero-size place in the line
            // (its static position); it is laid out once that place is known.
            // A float takes a zero-size place too; it is laid out (shrink-to-fit) now
            // and moved to the line top it lands on (`place_line_floats`).
            InlineItem::Atomic(b) if floated_kid(item).is_some() => {
                pieces.push(inline::Piece::Atom(out_of_flow_atom(atom_frags.len())));
                atom_frags.push(lay_float(b, cw, bs.font_size, ctx));
            }
            InlineItem::Atomic(b) if b.out_of_flow() => {
                pieces.push(inline::Piece::Atom(out_of_flow_atom(atom_frags.len())));
                atom_frags.push(Fragment::new(
                    b.node_id,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    content_kind(b, &resolve(b, cw, bs.font_size), 0.0, 0.0),
                ));
            }
            InlineItem::Atomic(b) => {
                let f = lay_atomic(b, cw, bs.font_size, ctx);
                // Cheap re-resolve (style cache memoizes) to read the box's own
                // `vertical-align` + margins for line-box placement.
                let abs = resolve(b, cw, bs.font_size);
                pieces.push(inline::Piece::Atom(inline::InlineAtom {
                    id: atom_frags.len(),
                    width: f.width,
                    height: f.height,
                    valign: abs.vertical_align,
                    font_size: abs.font_size,
                    margin_top: abs.margin.top,
                    margin_bottom: abs.margin.bottom,
                    margin_left: abs.margin.left,
                    margin_right: abs.margin.right,
                    placeholder: false,
                    baseline: atom_baseline(b, &f),
                }));
                atom_frags.push(f);
            }
            InlineItem::Marker { node_id, kind } => {
                directives.push(marker_frag(*node_id, *kind, cx, cy));
            }
        }
    }
    (pieces, atom_frags, directives)
}

// --------------------------------------------------------------------------
// block flow + margin collapsing
// --------------------------------------------------------------------------

/// Mutable running state of a block formatting context's vertical stacking: the
/// y `cursor` where the next in-flow box goes, the `pending` collapsed top-margin
/// carried from the previous box, and the container's block-level `align`.
struct FlowRun {
    cursor: f32,
    pending: f32,
    align: Align,
}

/// How a block container aligns its block-level children: by its `text-align` only
/// when that is a legacy `-webkit-*` value (Chrome 156; plain `center` / `right` align
/// inline content only; miaskiewicz/turbo-dom#69).
fn block_align(bs: &BoxStyle) -> Align {
    if bs.legacy_align {
        bs.text_align
    } else {
        Align::Left
    }
}

/// Collapse two adjacent vertical margins (CSS 2.1 §8.3.1): the used margin is the
/// largest positive margin plus the most-negative negative margin, so an all-
/// positive pair takes the max (the common case) while a negative margin pulls the
/// following box back up/over its neighbour.
fn collapse_margins(a: f32, b: f32) -> f32 {
    a.max(0.0).max(b.max(0.0)) + a.min(0.0).min(b.min(0.0))
}

/// Whether a box's first in-flow child's top margin collapses *through* the box's
/// top edge (CSS 2.1 §8.3.1): the box is a block container with no top border and
/// no top padding that does not establish a new block formatting context (so not a
/// float, out-of-flow, flex/grid/table, or scroll container). When it does, that
/// child margin appears above the box rather than inside it.
fn collapsing_block_container(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    lb.node_id.0 != 0 && matches!(lb.kind, BoxKind::Block(_)) && !establishes_bfc(lb, bs)
}

fn top_margins_collapse_through(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    // The galley root (the synthetic box `build_box_tree` stamps id 0, the initial
    // containing block — a styled document's `<html>` sits inside it) does NOT collapse
    // its children's margins out its top — root-element margins never collapse (CSS
    // 2.1 §8.3.1), so a first child's `margin-top` manifests at the page top as a
    // browser renders it, rather than vanishing to y=0.
    collapsing_block_container(lb, bs) && bs.border.widths().top == 0.0 && bs.padding.top == 0.0
}

fn collapsible_inline_box(kid: &LayoutBox, cw: f32, fs: f32) -> bool {
    kid.uses_html_font_metrics()
        && kid.anonymous_inline_flow()
        && matches!(&kid.kind, BoxKind::Lines(items)
        if super::inline_boxes::collapsible_run(items, fs, cw))
}

/// The in-flow (non-float, non-out-of-flow) children of a block container, with their
/// resolved styles (none for any other box).
fn inflow_children(
    lb: &LayoutBox,
    cw: f32,
    fs: f32,
) -> impl Iterator<Item = (&LayoutBox, BoxStyle)> {
    let kids: &[LayoutBox] = match &lb.kind {
        BoxKind::Block(kids) => kids,
        _ => &[],
    };
    kids.iter().filter_map(move |k| {
        let kbs = resolve(k, cw, fs);
        let inflow = kbs.float == Float::None
            && !kbs.position.is_out_of_flow()
            && !collapsible_inline_box(k, cw, fs);
        inflow.then_some((k, kbs))
    })
}

/// Whether a box's own top and bottom margins adjoin (CSS 2.2 §8.3.1): an empty box or a
/// collapsing block container (no own formatting context, not a replaced image) with a zero or
/// `auto` height, a zero `min-height`, no vertical border or padding, and only such
/// in-flow children (an empty `<div>`, nested empty `<div>`s).
fn self_collapsing(lb: &LayoutBox, bs: &BoxStyle, cw: f32) -> bool {
    empty_flow_container(lb, bs)
        && collapsible_extent(bs)
        && inflow_children(lb, cw, bs.font_size).all(|(kid, kbs)| self_collapsing(kid, &kbs, cw))
}

/// A box whose content can collapse away: a collapsing block container, or an element
/// with no content at all (an empty `Lines` box) that does not establish a formatting
/// context — not a replaced image.
fn empty_flow_container(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    let container = match &lb.kind {
        BoxKind::Lines(items) => items.is_empty() && lb.node_id.0 != 0 && !establishes_bfc(lb, bs),
        _ => collapsing_block_container(lb, bs),
    };
    container && !lb.image.as_ref().is_some_and(|image| image.replaced)
}

/// No vertical border or padding, a zero / `auto` `height` and `min-height`, no
/// `aspect-ratio`: nothing gives the box a height of its own.
fn collapsible_extent(bs: &BoxStyle) -> bool {
    // Widths and paddings are never negative, so a zero sum means no frame at all.
    let frame = bs.border.widths().vertical() + bs.padding.vertical();
    frame == 0.0
        && zero_or_auto(bs.height)
        && zero_or_auto(bs.min_height)
        && bs.aspect_ratio.is_none()
}

/// Whether a `height`-like length is `auto` or zero.
fn zero_or_auto(value: LengthPct) -> bool {
    match value {
        LengthPct::Auto => true,
        LengthPct::Px(v) | LengthPct::Pct(v) => v == 0.0,
        LengthPct::Calc { pct, px } => pct == 0.0 && px == 0.0,
    }
}

/// The collapsed top margin that shows *above* `lb`'s content: its own `margin-top`
/// collapsed, whenever its children's margins collapse through its top edge, with its
/// first in-flow child's leading margin, recursively — and while that child is empty
/// (self-collapsing, [`self_collapsing`]) with its bottom margin and the next in-flow
/// child's leading margin too (miaskiewicz/turbo-dom#76). A box's caller uses this
/// (rather than the bare `margin-top`) so a child's margin escaping a
/// borderless/paddingless parent stacks the parent against its sibling correctly.
fn leading_margin(lb: &LayoutBox, bs: &BoxStyle, cw: f32, fs: f32) -> f32 {
    let mut m = bs.margin.top;
    if top_margins_collapse_through(lb, bs) {
        for (child, cbs) in inflow_children(lb, cw, fs) {
            m = collapse_margins(m, leading_margin(child, &cbs, cw, fs));
            if !self_collapsing(child, &cbs, cw) {
                break;
            }
            m = collapse_margins(m, cbs.margin.bottom);
        }
    }
    m
}

/// Whether a block's last in-flow child's bottom margin can collapse through its bottom
/// edge (CSS 2.2 §8.3.1): a collapsing block container (no own formatting context)
/// with no bottom border or padding and an automatic `height` — on every layout policy
/// (miaskiewicz/turbo-dom#41; the generic / PDF one used to drop the margin). Like
/// Chrome, a `min-height` / `max-height` does not keep the margin inside: the box lays
/// its content out without it, and the margin collapses through only when the used
/// height is the content height ([`Ctx::dropped_end_margins`], miaskiewicz/turbo-dom#75).
fn bottom_margins_collapse_through(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    let open_edge = bs.border.widths().bottom == 0.0 && bs.padding.bottom == 0.0;
    collapsing_block_container(lb, bs) && open_edge && bs.height == LengthPct::Auto
}

/// The bottom margin `lb` passes to its following sibling: its own `margin-bottom`
/// collapsed with its last in-flow child's trailing margin when that collapses through
/// it (and was not dropped by `min-height` / `max-height`: `dropped`), recursively.
fn trailing_margin(lb: &LayoutBox, bs: &BoxStyle, cw: f32, dropped: &HashSet<NodeId>) -> f32 {
    let kids = match &lb.kind {
        BoxKind::Block(kids)
            if bottom_margins_collapse_through(lb, bs) && !dropped.contains(&lb.node_id) =>
        {
            kids
        }
        _ => return bs.margin.bottom,
    };
    let cw = (border_box_width(bs, cw) - bs.padding.horizontal() - bs.border.widths().horizontal())
        .max(0.0);
    let last = kids.iter().rev().find(|kid| {
        let used = resolve(kid, cw, bs.font_size);
        used.float == Float::None
            && !used.position.is_out_of_flow()
            && !collapsible_inline_box(kid, cw, bs.font_size)
    });
    last.map_or(bs.margin.bottom, |kid| {
        let used = resolve(kid, cw, bs.font_size);
        collapse_margins(bs.margin.bottom, trailing_margin(kid, &used, cw, dropped))
    })
}

/// The leading top margin to feed a flow child: `0` while this container hoists its
/// leading children's margins out its own top (parent/child collapse: the first in-flow
/// child, past any empty ones, see [`leading_margin`]), else the child's
/// [`leading_margin`].
fn child_leading(kid: &LayoutBox, kbs: &BoxStyle, cw: f32, fs: f32, hoisted: bool) -> f32 {
    if hoisted {
        0.0
    } else {
        leading_margin(kid, kbs, cw, fs)
    }
}

fn root_trailing_margin(margin: f32, root: bool) -> f32 {
    if root {
        margin
    } else {
        0.0
    }
}

fn flush_empty_inline(
    empty: &mut Vec<&LayoutBox>,
    origin: (f32, f32),
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
    fragments: &mut Vec<Fragment>,
) {
    for empty in empty.drain(..) {
        fragments.push(layout_box(empty, origin.0, origin.1, cw, fs, ctx));
    }
}

fn layout_block_flow(
    kids: &[LayoutBox],
    cx: f32,
    cy: f32,
    cw: f32,
    bs: &BoxStyle,
    ctx: &mut Ctx,
    boundaries: (bool, bool),
) -> (Vec<Fragment>, f32) {
    let fs = bs.font_size;
    let mut frags = Vec::new();
    let mut flow = FlowRun {
        cursor: cy,
        pending: 0.0,
        align: block_align(bs),
    };
    let mut deferred: Vec<(&LayoutBox, (f32, f32))> = Vec::new();
    // Whether the margins of the next in-flow child were hoisted out this box's top by
    // its caller: the first in-flow child's, and past it each following child's while
    // the earlier ones are empty (self-collapsing; miaskiewicz/turbo-dom#76).
    let mut hoisting = boundaries.0;
    let mut empty_inline = Vec::new();
    for kid in kids {
        let kbs = resolve(kid, cw, fs);
        if collapsible_inline_box(kid, cw, fs) {
            empty_inline.push(kid);
            continue;
        }
        // When this box is the containing block for an `absolute` child but its own
        // height is still indefinite (auto — driven by in-flow content), the child's
        // `%`-height / `bottom` inset has no basis yet. Defer it to a second pass and
        // resolve it against the measured content height, so a `bottom:0; height:33%`
        // overlay lands inside its card instead of collapsing and dropping below it.
        if defer_this_kid(bs, &kbs, ctx) {
            deferred.push((kid, block_static_position(cx, &flow)));
            continue;
        }
        // `absolute`/`fixed`/`float`: taken out of normal flow, placed on the side
        // without advancing the cursor or margin run (handled by `place_special_kid`).
        if let Some(f) = place_special_kid(kid, &kbs, cx, cw, fs, &flow, ctx) {
            frags.push(f);
            continue;
        }
        // The box's leading (collapsed) top margin — suppressed while this container
        // hoists it out its own top edge (parent/child collapse), else the child's
        // `margin-top` collapsed with any margins that escape ITS borderless top.
        let fragment = layout_flow_child(kid, &kbs, (cx, cw, fs), &mut hoisting, &mut flow, ctx);
        flush_empty_inline(&mut empty_inline, (cx, fragment.y), cw, fs, ctx, &mut frags);
        frags.push(fragment);
    }
    flush_empty_inline(
        &mut empty_inline,
        (cx, flow.cursor + flow.pending),
        cw,
        fs,
        ctx,
        &mut frags,
    );
    // Height is the in-flow content only. Floats are contained by their BFC (see
    // `layout_box_sized`), not by every block they pass through — a non-BFC block
    // does not grow to enclose a float, so following content wraps beside it.
    let content_h = flow.cursor - cy + root_trailing_margin(flow.pending, boundaries.1);
    place_deferred_abs(
        &deferred,
        cw,
        fs,
        deferred_cb_height(content_h, bs),
        &mut frags,
        ctx,
    );
    (frags, content_h)
}

/// Lay out one in-flow child of [`layout_block_flow`] (`(cx, cw, fs)`: the content box's
/// x, width and font size). While `hoisting`, the child's leading margin was applied by
/// this box's caller; an empty (self-collapsing) child's own margins were too, so it
/// leaves nothing pending and the hoisting goes on past it (miaskiewicz/turbo-dom#76).
fn layout_flow_child(
    kid: &LayoutBox,
    kbs: &BoxStyle,
    (cx, cw, fs): (f32, f32, f32),
    hoisting: &mut bool,
    flow: &mut FlowRun,
    ctx: &mut Ctx,
) -> Fragment {
    let leading = child_leading(kid, kbs, cw, fs, *hoisting);
    let empty = *hoisting && self_collapsing(kid, kbs, cw);
    let fragment = layout_in_flow_kid(kid, leading, cx, cw, fs, flow, ctx);
    if empty {
        flow.pending = 0.0;
    } else {
        *hoisting = false;
    }
    fragment
}

/// Whether an `absolute` child must wait for its CB's content height: the parent
/// IS its containing block (positioned) but that block's height is still
/// indefinite (`abs_cb_h == 0`), so a `%`-height / `bottom` inset can't resolve.
fn defer_this_kid(parent: &BoxStyle, kbs: &BoxStyle, ctx: &Ctx) -> bool {
    kbs.position == Position::Absolute
        && parent.position != Position::Static
        && ctx.abs_cb_h == 0.0
        && !(ctx.html_font_metrics && ctx.abs_cb_h_known)
}

/// Second pass for a positioned CB's own `absolute` children, now that its in-flow
/// content height (`content_h`) — the basis for their `%`-height and `bottom`
/// inset — is known. Exposes `content_h` as the CB height for the duration; the
/// enclosing box's frame restores `abs_cb_h` afterwards.
fn place_deferred_abs(
    deferred: &[(&LayoutBox, (f32, f32))],
    cw: f32,
    fs: f32,
    content_h: f32,
    frags: &mut Vec<Fragment>,
    ctx: &mut Ctx,
) {
    if deferred.is_empty() {
        return;
    }
    ctx.abs_cb_h = content_h;
    ctx.abs_cb_h_known = true;
    for (kid, static_pos) in deferred {
        let kbs = positioned_style(kid, cw, fs, ctx);
        let (bx, by) = out_of_flow_origin(kid, &kbs, cw, ctx, *static_pos);
        let mut frag = layout_positioned(kid, (bx, by), cw, fs, ctx);
        anchor_bottom(&mut frag, &kbs, content_h, ctx);
        frags.push(frag);
    }
}

/// Pull a `bottom`-anchored absolute box (auto `top`) to the CB's bottom edge now
/// that the CB height is known — `out_of_flow_origin` could only leave it at its
/// static y while the height was indefinite.
fn anchor_bottom(frag: &mut Fragment, kbs: &BoxStyle, content_h: f32, ctx: &Ctx) {
    if kbs.inset_top != LengthPct::Auto {
        return; // `top` wins; the box is already anchored from the top.
    }
    let Some(bottom) = kbs.inset_bottom.resolve(content_h) else {
        return;
    };
    let origin_y = if ctx.live_geometry || in_known_icb(kbs, ctx) {
        containing_block(kbs, ctx).1
    } else {
        ctx.abs_cb_y
    };
    let target_y = origin_y + content_h - bottom - frag.height - kbs.margin.bottom;
    frag.translate(0.0, target_y - frag.y);
}

/// Place a child that sits outside normal block flow, returning its fragment if
/// it was handled (contributing nothing to the cursor/margin run) or `None` for a
/// normal-flow box. `absolute`/`fixed` land at their insets against the containing
/// block (or their static in-flow position for `auto` insets); a `float:left/right`
/// packs to its edge and registers in the BFC's float set so later in-flow content
/// — even in sibling blocks — wraps beside it.
fn place_special_kid(
    kid: &LayoutBox,
    kbs: &BoxStyle,
    cx: f32,
    cw: f32,
    fs: f32,
    flow: &FlowRun,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    if kbs.position.is_out_of_flow() {
        let static_pos = block_static_position(cx, flow);
        let kbs = &positioned_style(kid, cw, fs, ctx);
        let (bx, by) = out_of_flow_origin(kid, kbs, cw, ctx, static_pos);
        let mut frag = layout_positioned(kid, (bx, by), cw, fs, ctx);
        // A `bottom`-anchored (auto `top`) box left at its static y by
        // `out_of_flow_origin` — because its height wasn't known there — is pulled
        // to the CB bottom now, when the CB height is DEFINITE. (An indefinite-height
        // CB defers its absolute children instead, via `place_deferred_abs`, so the
        // `> 0` guard keeps this off that path.)
        let height = out_of_flow_height(kbs, ctx);
        if height > 0.0 {
            anchor_bottom(&mut frag, kbs, height, ctx);
        }
        return Some(frag);
    }
    if kbs.float != Float::None {
        return Some(place_float(
            kid,
            kbs,
            cx,
            cw,
            fs,
            flow.cursor + flow.pending,
            ctx,
        ));
    }
    None
}

/// Lay out one normal-flow block child, advancing `flow`'s cursor/margin run, and
/// return its fragment. Applies `clear`, the `relative`/`sticky` paint shift, float
/// avoidance for BFC/replaced boxes, and horizontal alignment.
fn layout_in_flow_kid(
    kid: &LayoutBox,
    leading: f32,
    cx: f32,
    cw: f32,
    fs: f32,
    flow: &mut FlowRun,
    ctx: &mut Ctx,
) -> Fragment {
    let kbs = resolve(kid, cw, fs);
    let kbs = &kbs;
    // `clear` (CSS 2.2 §9.5.2): clearance is introduced only when the box's hypothetical
    // border edge (its collapsed top margin included) is above a cleared float's bottom;
    // the border edge then sits exactly at that bottom (the margin is absorbed).
    let hypothetical = flow.cursor + collapse_margins(flow.pending, leading);
    let cleared = clear_below(ctx, kid, hypothetical);
    let leading = if cleared > hypothetical {
        flow.cursor = cleared;
        flow.pending = 0.0;
        0.0
    } else {
        leading
    };
    let (dx, dy) = flow_relative_offset(kbs, cw);
    // `leading` is the box's `margin-top` already collapsed with any margin escaping
    // its borderless top (parent/child collapse); collapse it into the running margin.
    flow.pending = collapse_margins(flow.pending, leading);
    let flow_y = flow.cursor + flow.pending;
    let (region_x, region_w) = flow_region(kid, kbs, ctx, cx, cw, flow_y);
    let mut frag = layout_box(
        kid,
        region_x + kbs.margin.left + dx,
        flow_y + dy,
        region_w,
        fs,
        ctx,
    );
    // Horizontally align a width-constrained block within its inline region:
    // `margin: … auto` centers it, and a legacy `text-align: -webkit-center` /
    // `-webkit-right` container (`<center>`, `align=center`) centers / right-aligns
    // block children (`block_align`).
    let hx = block_h_offset(flow.align, kbs, kid, frag.width, region_w);
    if hx != 0.0 {
        frag.translate(hx, 0.0);
    }
    // Advance the cursor by the box's height at its *unshifted* flow position. A
    // zero-height box lets the margins run through it — unless it establishes a
    // formatting context, which is never self-collapsing (miaskiewicz/turbo-dom#88).
    if frag.height == 0.0 && !establishes_bfc(kid, kbs) {
        flow.pending = collapse_margins(
            flow.pending,
            trailing_margin(kid, kbs, cw, &ctx.dropped_end_margins),
        );
    } else {
        flow.cursor += flow.pending + frag.height;
        flow.pending = trailing_margin(kid, kbs, cw, &ctx.dropped_end_margins);
    }
    frag
}

/// The paint-time shift for a `relative`/`sticky` box: it flows normally (its
/// space is reserved via the cursor) but is painted shifted by its insets. Any
/// other positioning contributes no shift.
pub(super) fn flow_relative_offset(kbs: &BoxStyle, cw: f32) -> (f32, f32) {
    if matches!(kbs.position, Position::Relative | Position::Sticky) {
        relative_offset(kbs, cw)
    } else {
        (0.0, 0.0)
    }
}

/// The inline region `(x, width)` a normal-flow block child occupies at `flow_y`.
/// A plain block spans the full content width — a float only shortens the *line
/// boxes* of its inline content (handled per-line in `layout_lines`), not the
/// block itself. Only a box that avoids floats as a unit — one establishing a BFC
/// (table/flex/grid/`overflow`) or a replaced `<img>` — is narrowed and shifted
/// into the free region beside the float.
fn flow_region(
    kid: &LayoutBox,
    kbs: &BoxStyle,
    ctx: &Ctx,
    cx: f32,
    cw: f32,
    flow_y: f32,
) -> (f32, f32) {
    let avoids_floats = establishes_bfc(kid, kbs) || kid.is_replaced();
    if avoids_floats {
        inline_region(ctx, cx, cw, flow_y)
    } else {
        (cx, cw)
    }
}

/// The horizontal shift to align a width-constrained in-flow block within its
/// container (0 for a full-width/auto block, which fills the line). `margin:auto`
/// centers; otherwise the container's block alignment (`block_align`: a legacy
/// `-webkit-*` `text-align`, as `<center>` and `align="center"` give) centers /
/// right-aligns block children.
fn block_h_offset(align: Align, kbs: &BoxStyle, kid: &LayoutBox, frag_w: f32, cw: f32) -> f32 {
    let avail = (cw - kbs.margin.horizontal()).max(0.0);
    if frag_w >= avail - 0.5 {
        return 0.0;
    }
    if auto_x_margins(&kid.style) || align == Align::Center {
        return (cw - frag_w) / 2.0 - kbs.margin.left;
    }
    if align == Align::Right {
        return cw - frag_w - kbs.margin.right - kbs.margin.left;
    }
    0.0
}

/// Whether a box centers itself via `margin: … auto` (horizontal auto margins).
fn auto_x_margins(s: &crate::style::ComputedStyle) -> bool {
    let is_auto = |p| s.get(p).map(str::trim) == Some("auto");
    is_auto("margin-left") && is_auto("margin-right")
        || s.get("margin")
            .is_some_and(|m| m.split_whitespace().any(|t| t == "auto"))
}

/// Whether a box establishes a new block formatting context — floats it contains
/// are isolated from the outer context (they don't wrap outer floats, and they
/// grow this box). Floats, out-of-flow boxes, non-`visible` `overflow`, `display:
/// flow-root` (miaskiewicz/turbo-dom#88) and the independent formatting roots
/// (flex/grid/table) all qualify.
fn intrinsic_formatting_root(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    lb.document_root() || bs.float != Float::None || bs.position.is_out_of_flow() || flow_root(lb)
}

/// `display: flow-root` (also the two-value `block flow-root`): a block box that
/// establishes a block formatting context (CSS Display 3; miaskiewicz/turbo-dom#88).
fn flow_root(lb: &LayoutBox) -> bool {
    lb.style
        .get("display")
        .is_some_and(|display| display.split_whitespace().any(|token| token == "flow-root"))
}

/// Whether an in-flow block-level box avoids floats (a formatting-context root or a
/// replaced box): it sits beside them rather than under them — also in a max-content
/// width (miaskiewicz/turbo-dom#85).
pub(crate) fn avoids_floats(lb: &LayoutBox) -> bool {
    let bs = lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    establishes_bfc(lb, &bs) || lb.is_replaced()
}

fn establishes_bfc(lb: &LayoutBox, bs: &BoxStyle) -> bool {
    if intrinsic_formatting_root(lb, bs) {
        return true;
    }
    if matches!(
        lb.kind,
        BoxKind::Flex(_) | BoxKind::Grid(_) | BoxKind::Table(_)
    ) {
        return true;
    }
    ["overflow", "overflow-x", "overflow-y"].iter().any(|p| {
        matches!(
            lb.style.get(p).map(str::trim),
            Some("hidden" | "auto" | "scroll" | "clip")
        )
    })
}

/// The free inline region `(x, width)` at vertical position `y` within the
/// content box `[cx, cx+cw]`, narrowed by every float in the current BFC whose
/// vertical span contains `y` (left floats push the start right, right floats
/// pull the end left). Native CSS keeps valid zero widths; PDF retains its
/// positive floor for a fully-covered row.
fn inline_region(ctx: &Ctx, cx: f32, cw: f32, y: f32) -> (f32, f32) {
    inline_region_from(
        &ctx.floats,
        cx,
        cw,
        y,
        if ctx.html_font_metrics { 0.0 } else { 1.0 },
    )
}

fn inline_region_from(floats: &[FloatRect], cx: f32, cw: f32, y: f32, minimum: f32) -> (f32, f32) {
    let mut left = cx;
    let mut right = cx + cw;
    for f in floats {
        if y + 0.5 < f.top || y > f.bottom - 0.5 {
            continue;
        }
        narrow_region(f, &mut left, &mut right);
    }
    (left, (right - left).max(minimum))
}

fn inline_clearance(floats: &[FloatRect], region: (f32, f32, f32), top: f32, needed: f32) -> f32 {
    let (cx, cy, cw) = region;
    let mut y = cy + top;
    for _ in floats {
        if inline_region_from(floats, cx, cw, y, 0.0).1 >= needed {
            break;
        }
        let Some(bottom) = next_float_bottom(floats, y) else {
            break;
        };
        y = bottom;
    }
    y - cy
}

fn next_float_bottom(floats: &[FloatRect], y: f32) -> Option<f32> {
    floats
        .iter()
        .filter(|float| y + 0.5 >= float.top && y <= float.bottom - 0.5)
        .map(|float| float.bottom)
        .min_by(f32::total_cmp)
}

/// Narrow the inline region for one float whose span contains the row: a `float:left`
/// pushes the start right, a `float:right` pulls the end left. A `FloatRect` is only
/// ever built for a real left/right float (`None` never enters the list), so there is
/// no dead third arm.
fn narrow_region(f: &FloatRect, left: &mut f32, right: &mut f32) {
    if matches!(f.side, Float::Left) {
        *left = left.max(f.x1);
    } else if matches!(f.side, Float::Right) {
        *right = right.min(f.x0);
    }
}

/// The y a box with `clear` must drop to: the lowest bottom edge of any active
/// float on the cleared side(s) (`clear:left/right/both`), or `y` if none.
fn clear_below(ctx: &Ctx, kid: &LayoutBox, y: f32) -> f32 {
    let (cl, cr) = match kid.style.get("clear").map(str::trim) {
        Some("both") => (true, true),
        Some("left") => (true, false),
        Some("right") => (false, true),
        _ => return y,
    };
    // Drop to the lowest bottom of any active float on a cleared side, or stay at
    // `y` if none (the fold seed) — equivalent to the running `max` over hits.
    ctx.floats
        .iter()
        .filter(|f| float_on_cleared_side(f, cl, cr))
        .map(|f| f.bottom)
        .fold(y, f32::max)
}

/// Whether float `f` lies on a side the current box is clearing.
fn float_on_cleared_side(f: &FloatRect, clear_left: bool, clear_right: bool) -> bool {
    matches!(f.side, Float::Left) && clear_left || matches!(f.side, Float::Right) && clear_right
}

/// Place a floated box: size it (shrink-to-fit for auto width), drop it to the
/// first y at or below `y0` where it fits on its side beside earlier floats
/// (else below them), pack it to that edge, and register it in the BFC's float
/// set so following in-flow content wraps beside it.
fn place_float(
    kid: &LayoutBox,
    kbs: &BoxStyle,
    cx: f32,
    cw: f32,
    fs: f32,
    y0: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let replaced = kid.is_replaced();
    let (w, shrink) = float_width(kid, kbs, cw, ctx.fonts, replaced);
    // A float is offset from its packing edge by its own margins (CSS 9.5): its
    // border box drops below preceding floats plus `margin-top`, and packs inside
    // the inline region by `margin-left`/`margin-right`.
    let y = float_drop_y(ctx, cx, cw, w, y0) + kbs.margin.top;
    let (rx, rw) = inline_region(ctx, cx, cw, y);
    let bx = match kbs.float {
        Float::Right => rx + rw - w - kbs.margin.right,
        _ => rx + kbs.margin.left,
    };
    let mut f = if shrink {
        layout_box_sized(kid, kbs, bx, y, w, ctx)
    } else {
        layout_box(kid, bx, y, cw, fs, ctx)
    };
    reanchor_right_float(&mut f, kbs, rx, rw, w);
    // Register the float's MARGIN box (not just its border box), so later in-flow
    // content clears the float's margins too — matching the `FloatRect` contract.
    ctx.floats.push(FloatRect {
        x0: f.x - kbs.margin.left,
        x1: f.x + f.width + kbs.margin.right,
        top: y - kbs.margin.top,
        bottom: y + f.height + kbs.margin.bottom,
        side: kbs.float,
    });
    f
}

/// A float met in line content, laid out at the origin (border box at `0,0`) at its
/// used width; [`place_atom`] moves it to its place.
fn lay_float(kid: &LayoutBox, cw: f32, fs: f32, ctx: &mut Ctx) -> Fragment {
    let kbs = resolve(kid, cw, fs);
    let (w, shrink) = float_width(kid, &kbs, cw, ctx.fonts, kid.is_replaced());
    if shrink {
        layout_box_sized(kid, &kbs, 0.0, 0.0, w, ctx)
    } else {
        layout_box(kid, 0.0, 0.0, cw, fs, ctx)
    }
}

/// The border-box width of a float and whether it is shrink-to-fit sized. An
/// `auto`-width non-replaced float is shrink-to-fit (CSS 2.2 §10.3.5;
/// miaskiewicz/turbo-dom#83): `min(max(min-content, available), max-content)`, the
/// available width being the container's less the float's own horizontal margins, then
/// `min-width` / `max-width` (`shrink = true`); otherwise it takes its resolved
/// border-box width.
fn float_width(
    kid: &LayoutBox,
    kbs: &BoxStyle,
    cw: f32,
    fonts: &FontRegistry,
    replaced: bool,
) -> (f32, bool) {
    let shrink = !replaced && kbs.width.resolve(cw).is_none();
    let w = if shrink {
        let available = cw - kbs.margin.horizontal();
        let min = super::flex::min_content_width(kid, fonts);
        let fit = super::flex::natural_width(kid, fonts).min(available.max(min));
        let extra = kbs.padding.horizontal() + kbs.border.widths().horizontal();
        clamp_width(fit, kbs, cw, extra)
    } else {
        border_box_width(kbs, cw)
    };
    (w, shrink)
}

/// The y at/below `y0` where a `w`-wide float fits beside existing same-BFC
/// floats; if a row is too full, drop past the nearest float bottom and retry.
fn float_drop_y(ctx: &Ctx, cx: f32, cw: f32, w: f32, y0: f32) -> f32 {
    let minimum = if ctx.html_font_metrics { 0.0 } else { 1.0 };
    float_drop_y_in(&ctx.floats, (cx, cw, minimum), w, y0)
}

/// [`float_drop_y`] against `floats` (`region`: the content box's x, width and the
/// minimum region width).
fn float_drop_y_in(floats: &[FloatRect], region: (f32, f32, f32), w: f32, y0: f32) -> f32 {
    let (cx, cw, minimum) = region;
    let mut y = y0;
    loop {
        let (_, avail) = inline_region_from(floats, cx, cw, y, minimum);
        if avail >= w - 0.5 {
            break;
        }
        let next = floats
            .iter()
            .filter(|f| f.bottom > y + 0.5)
            .map(|f| f.bottom)
            .fold(f32::INFINITY, f32::min);
        if !next.is_finite() {
            break;
        }
        y = next;
    }
    y
}

/// Re-pin a right float to the true right edge when its laid width differs from
/// the reserved width `w` (e.g. min/max-width clamped the box during layout).
fn reanchor_right_float(f: &mut Fragment, kbs: &BoxStyle, rx: f32, rw: f32, w: f32) {
    if kbs.float == Float::Right && (f.width - w).abs() > 0.5 {
        f.translate(rx + rw - f.width - f.x, 0.0);
    }
}

fn layout_content(
    lb: &LayoutBox,
    bs: &BoxStyle,
    (cx, cy, cw): (f32, f32, f32),
    bfc: bool,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    match &lb.kind {
        BoxKind::Block(kids) => {
            // When this box lets its first child's top margin collapse through, that
            // leading margin is applied by THIS box's own caller (via `leading_margin`)
            // — so suppress it inside to avoid double-counting. A box that establishes a
            // formatting context (`bfc`, also a flex item or table cell) keeps its
            // children's margins inside, as does one whose last child's bottom margin
            // cannot collapse through it (miaskiewicz/turbo-dom#41).
            let suppress_first = !bfc && top_margins_collapse_through(lb, bs);
            let keep_last = lb.document_root() || bfc || !bottom_margins_collapse_through(lb, bs);
            layout_block_flow(kids, cx, cy, cw, bs, ctx, (suppress_first, keep_last))
        }
        BoxKind::Flex(kids) => super::flex::layout_flex(lb, kids, cx, cy, cw, bs.font_size, ctx),
        BoxKind::Grid(kids) => super::flex::layout_grid(lb, kids, cx, cy, cw, bs.font_size, ctx),
        BoxKind::Table(kids) if super::table::is_wrapper(lb) => {
            super::table::layout_wrapper(kids, cx, cy, cw, bs.font_size, ctx)
        }
        BoxKind::Table(kids) => {
            // The rows share the table's definite height (miaskiewicz/turbo-dom#64).
            let fill = resolved_content_height(bs, 0.0, ctx);
            super::table::layout_table(lb, kids, (cx, cy, cw, fill), bs.font_size, ctx)
        }
        BoxKind::Lines(items) => layout_lines(lb, items, bs, cx, cy, cw, ctx),
        BoxKind::Marker(_) => (Vec::new(), 0.0),
    }
}

fn break_meta_of(bs: &BoxStyle) -> BreakMeta {
    BreakMeta {
        break_before: bs.break_before,
        break_after: bs.break_after,
        break_inside_avoid: bs.break_inside_avoid,
        orphans: bs.orphans,
        widows: bs.widows,
        repeatable: None,
        footnotes: Vec::new(),
    }
}

fn content_kind(lb: &LayoutBox, bs: &BoxStyle, bbw: f32, bbh: f32) -> FragmentContent {
    match &lb.kind {
        BoxKind::Marker(k) => FragmentContent::Marker(*k),
        _ => FragmentContent::Box {
            // A masked box paints its fill THROUGH the mask (a separate tinted image
            // fragment), so it must not also paint a solid rectangle behind it. A
            // gradient background likewise covers the solid colour.
            background: bs
                .background
                .filter(|_| lb.mask.is_none() && bs.background_gradient.is_none()),
            border: bs.border,
            border_radius: resolve_radius(bs.border_radius, bbw, bbh),
            shadow: bs.box_shadow,
            gradient: bs.background_gradient.clone(),
            // Resolve a `%` translate against the box size and the `transform-origin`
            // (its `%`/keyword parts) against the box's own width/height. The raster
            // composes this about the box's absolute position.
            transform: bs.transform.map(|t| {
                let e = t.tx.resolve(bbw).unwrap_or(0.0);
                let f = t.ty.resolve(bbh).unwrap_or(0.0);
                Transform2D {
                    matrix: [t.linear[0], t.linear[1], t.linear[2], t.linear[3], e, f],
                    origin_x: bs.transform_origin.0.resolve(bbw).unwrap_or(bbw / 2.0),
                    origin_y: bs.transform_origin.1.resolve(bbh).unwrap_or(bbh / 2.0),
                }
            }),
        },
    }
}

/// The colour a `mask-image` box paints through the mask: its `background-color`,
/// falling back to `color` (`currentColor`, the common idiom) when the background
/// is transparent.
fn mask_tint(bs: &BoxStyle) -> super::value::Rgba {
    bs.background.unwrap_or(bs.color)
}

/// If the box carries a resolvable `mask-image`, insert a tinted `Image` fragment
/// filling its content box (in front of the background, behind its children) that
/// the raster paints as `mask_tint` stencilled by the mask's alpha.
fn prepend_mask_image(
    lb: &LayoutBox,
    bs: &BoxStyle,
    rect: (f32, f32, f32, f32),
    ctx: &Ctx,
    children: &mut Vec<Fragment>,
) {
    let Some(name) = lb.mask.as_ref() else {
        return;
    };
    let src = ImageSource {
        name: name.clone(),
        replaced: false,
    };
    let Some(intrinsic) = probe_source(lb, &src, ctx) else {
        return;
    };
    let (cx, cy, cw, ch) = rect;
    let mut placement = super::imgsize::placement_of(name.clone(), intrinsic);
    placement.tint = Some(mask_tint(bs));
    let frag = Fragment::new(
        lb.node_id,
        cx,
        cy,
        cw,
        ch,
        FragmentContent::Image(placement),
    );
    children.insert(0, frag);
}

/// Record whether `lb`'s last child's bottom margin is dropped: it would collapse through
/// `lb` (laid out without it, see `layout_content`) but `lb`'s used height is not its
/// content height (`resized`), so it neither escapes nor counts (miaskiewicz/turbo-dom#75).
fn record_dropped_end_margin(
    lb: &LayoutBox,
    bs: &BoxStyle,
    bfc: bool,
    resized: bool,
    ctx: &mut Ctx,
) {
    if resized && !bfc && !lb.document_root() && bottom_margins_collapse_through(lb, bs) {
        ctx.dropped_end_margins.insert(lb.node_id);
    } else {
        ctx.dropped_end_margins.remove(&lb.node_id);
    }
}

/// Resolve `border-radius` to px against the border box, clamped to half the
/// shorter side (so `50%` on a square is a circle, and no corner over-rounds).
fn resolve_radius(r: LengthPct, bbw: f32, bbh: f32) -> f32 {
    let px = r.resolve(bbw.min(bbh)).unwrap_or(0.0);
    px.clamp(0.0, (bbw.min(bbh) / 2.0).max(0.0))
}

/// Lay out a box whose border-box width is already decided (`bbw`), at border-box
/// top-left `(bx, by)`. Used by `flex.rs`, which sizes items itself.
pub(crate) fn layout_box_sized(
    lb: &LayoutBox,
    bs: &BoxStyle,
    bx: f32,
    by: f32,
    bbw: f32,
    ctx: &mut Ctx,
) -> Fragment {
    layout_box_sized_impl(lb, bs, bx, by, bbw, ctx, false)
}

/// [`layout_box_sized`] for a box that ALWAYS establishes a block formatting
/// context — a flex/grid item or a table cell (§CSS: each is an independent
/// formatting context). Without this, an item that floats its content (a clearfix
/// bar like Wikipedia's page-title row) doesn't contain the float, so its
/// min-content height measures 0 and the grid row collapses under the overflow.
pub(crate) fn layout_box_sized_isolated(
    lb: &LayoutBox,
    bs: &BoxStyle,
    bx: f32,
    by: f32,
    bbw: f32,
    ctx: &mut Ctx,
) -> Fragment {
    layout_box_sized_impl(lb, bs, bx, by, bbw, ctx, true)
}

/// Whether a box is the containing block of its `absolute` descendants: a positioned
/// box (an anonymous inline-flow box never is: it does not inherit `position`).
fn contains_positioned(bs: &BoxStyle) -> bool {
    bs.position != Position::Static
}

fn child_height_known(lb: &LayoutBox, height: Option<f32>, inherited: bool) -> bool {
    height.is_some() || (lb.node_id == NodeId(0) && inherited)
}

fn layout_box_sized_impl(
    lb: &LayoutBox,
    bs: &BoxStyle,
    bx: f32,
    by: f32,
    bbw: f32,
    ctx: &mut Ctx,
    force_bfc: bool,
) -> Fragment {
    // A replaced `<img>` sizes from its intrinsic dimensions + caps rather than
    // flowing content — the same as the normal-flow `layout_box` path. Flex/grid
    // items and table cells reach layout through here, so without this an `<img>`
    // flex item (a logo/hero) laid out as an empty container: 0 height, no image.
    if let Some(frag) = replaced_image_box(lb, bs, bx, by, bbw, ctx) {
        return frag;
    }
    // A table wrapper box (a captioned table): its frame, background and heights are
    // its grid box's (miaskiewicz/turbo-dom#51).
    let bs: &BoxStyle = &super::table::own_style(lb, bs);
    let bw = bs.border.widths();
    let content_x = bx + bw.left + bs.padding.left;
    let content_y = by + bw.top + bs.padding.top;
    let content_w = (bbw - bs.padding.horizontal() - bw.horizontal()).max(0.0);
    // A positioned box establishes the containing block for its `absolute`
    // descendants (its content box). Save/restore around the child layout.
    let saved_cb = (ctx.abs_cb_x, ctx.abs_cb_y, ctx.abs_cb_w, ctx.abs_cb_h);
    let saved_abs_known = (ctx.abs_cb_h_known, ctx.abs_cb_icb);
    let own_height = resolved_live_height(bs.height, native_height_basis(bs, ctx), 0.0);
    if contains_positioned(bs) {
        (ctx.abs_cb_x, ctx.abs_cb_y, ctx.abs_cb_w, ctx.abs_cb_h) =
            positioned_child_block(bs, (bx, by, bbw), ctx);
        ctx.abs_cb_h_known = own_height.is_some();
        ctx.abs_cb_icb = false;
    }
    // In-flow `%`-height CB: expose this box's own definite content height to its
    // children (any box, not just positioned) so a `height:100%`/`max-height:100%`
    // child resolves against it. 0 (auto) leaves children on their intrinsic size.
    let saved_cb_h = ctx.cb_h;
    let saved_cb_known = ctx.cb_h_known;
    ctx.cb_h_known = child_height_known(lb, own_height, saved_cb_known);
    ctx.cb_h = live_child_height(lb, bs, saved_cb_h, ctx.live_geometry);
    // A box that establishes a block formatting context isolates floats: its
    // content does not wrap around outer floats, and the floats placed inside it
    // are contained by (grow) this box rather than escaping to sibling blocks.
    let bfc = force_bfc || establishes_bfc(lb, bs);
    let saved_floats = bfc.then(|| std::mem::take(&mut ctx.floats));
    let (mut children, mut content_h) =
        layout_content(lb, bs, (content_x, content_y, content_w), bfc, ctx);
    if let Some(outer) = saved_floats {
        let inner_bottom = ctx
            .floats
            .iter()
            .map(|f| f.bottom)
            .fold(content_y, f32::max);
        content_h = content_h.max(inner_bottom - content_y);
        ctx.floats = outer;
    }
    (ctx.abs_cb_x, ctx.abs_cb_y, ctx.abs_cb_w, ctx.abs_cb_h) = saved_cb;
    ctx.cb_h = saved_cb_h;
    ctx.cb_h_known = saved_cb_known;
    (ctx.abs_cb_h_known, ctx.abs_cb_icb) = saved_abs_known;
    // The content-box height honoring an explicit/min/max `height` — a `background`
    // or `mask` image fills the box even when it has no flow content (an empty icon
    // span sized only by `height`, whose measured `content_h` is 0).
    // `abs_cb_h` is restored above to the CONTAINING block's height — the basis a
    // `%` height on this (positioned) box resolves against.
    // An `aspect-ratio` box with an auto height derives it from the content width
    // (`width / ratio`); content can still expand it. Used as the auto-height basis
    // so a square media tile is sized by ratio, not by a `min-height` fallback.
    let auto_h = aspect_auto_height(bs, content_w).map_or(content_h, |a| a.max(content_h));
    let fill_h = resolved_content_height(bs, auto_h, ctx);
    record_dropped_end_margin(lb, bs, bfc, fill_h != content_h, ctx);
    let bbh = fill_h + bs.padding.vertical() + bw.vertical();
    prepend_mask_image(
        lb,
        bs,
        (content_x, content_y, content_w, fill_h),
        &*ctx,
        &mut children,
    );
    prepend_background_image(
        lb,
        content_x,
        content_y,
        content_w,
        fill_h,
        &*ctx,
        &mut children,
    );
    record_metrics(lb, bs, content_w, fill_h, ctx);
    Fragment {
        node_id: lb.node_id,
        x: bx,
        y: by,
        width: bbw,
        height: bbh,
        content: content_kind(lb, bs, bbw, bbh),
        break_meta: break_meta_of(bs),
        children,
        z_index: bs.z_index,
        is_positioned: bs.position != Position::Static,
        clip: super::value::clip_region(&lb.style, bbw, bbh, 1.0),
        opacity: bs.opacity,
    }
}

/// If the box carries a resolvable `background-image`, insert an `Image`
/// fragment filling its content box *behind* the box's children (index 0, so it
/// paints first). A `background-image` does not resize the box.
fn prepend_background_image(
    lb: &LayoutBox,
    cx: f32,
    cy: f32,
    cw: f32,
    ch: f32,
    ctx: &Ctx,
    children: &mut Vec<Fragment>,
) {
    let Some(placement) = background_placement(lb, ctx) else {
        return;
    };
    let frag = Fragment::new(
        lb.node_id,
        cx,
        cy,
        cw,
        ch,
        FragmentContent::Image(placement),
    );
    children.insert(0, frag);
}

/// The placement for a box's `background-image`, if it has one the resolver can
/// supply and decode (probe succeeds).
fn background_placement(lb: &LayoutBox, ctx: &Ctx) -> Option<ImagePlacement> {
    let src = lb.image.as_ref().filter(|s| !s.replaced)?;
    let intrinsic = probe_source(lb, src, ctx)?;
    Some(super::imgsize::placement_of(src.name.clone(), intrinsic))
}

/// Lay out one block-level box with its border-box top-left at `(bx, by)`,
/// resolving its width against the containing block. A replaced `<img>` is sized
/// from its intrinsic dimensions and the overflow caps instead of flowing.
fn layout_box(
    lb: &LayoutBox,
    bx: f32,
    by: f32,
    cb_width: f32,
    parent_fs: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let bs = resolve(lb, cb_width, parent_fs);
    if let Some(frag) = replaced_image_box(lb, &bs, bx, by, cb_width, ctx) {
        return frag;
    }
    let mut bbw = border_box_width(&bs, cb_width);
    // A table never shrinks below its content: grow the border box to the columns'
    // min-content when a declared/`max-width` is narrower (CSS auto table layout).
    // Wikipedia's infobox is `width:200px` but its cat-image row is 267px wide — so
    // the box must be 267, not clip the images and squeeze the label column.
    if let BoxKind::Table(items) = &lb.kind {
        let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
        bbw = shrink_auto_table(items, lb, &bs, (bbw, cb_width), ctx);
        bbw = bbw.max(super::table::min_content_width(items, &lb.style, ctx.fonts) + frame);
    }
    layout_box_sized(lb, &bs, bx, by, bbw, ctx)
}

/// A `width: auto` table is shrink-to-fit: no wider than its columns' max-content
/// (+ spacing and frame), as Chrome lays it out — live (miaskiewicz/turbo-surf#36 item
/// 3) and on the PDF / screenshot path (miaskiewicz/turbo-dom#28 item 7); the caller
/// still floors it at min-content.
fn shrink_auto_table(
    items: &[LayoutBox],
    lb: &LayoutBox,
    bs: &BoxStyle,
    (bbw, cb_width): (f32, f32),
    ctx: &Ctx,
) -> f32 {
    if bs.width != LengthPct::Auto {
        return bbw;
    }
    let extra = bs.padding.horizontal() + bs.border.widths().horizontal();
    let (max_content, unbounded) = super::table::max_content(items, &lb.style, ctx.fonts);
    let fit = if unbounded { bbw } else { max_content + extra };
    clamp_width(bbw.min(fit), bs, cb_width, extra)
}

/// The height an `aspect-ratio` box derives from its content width when its own
/// height is `auto` (`content_w / ratio`). `None` for no ratio or an explicit
/// height — a set height wins and the ratio would instead constrain the width.
fn aspect_auto_height(bs: &BoxStyle, content_w: f32) -> Option<f32> {
    if bs.height != LengthPct::Auto {
        return None;
    }
    bs.aspect_ratio.map(|r| content_w / r)
}

/// Stamp every replaced `<img>` box's intrinsic pixel width from the resolver,
/// depth-first, before layout. The intrinsic size needs the decoded bytes (only this
/// entry has the resolver), but the resolver-less `natural_width`/`min_content_width`
/// sizers run deep in the flex/table passes — so without a stamped width a `<div>`
/// wrapping an `<img>` (a logo) measured 0 and its flex item collapsed / mis-aligned.
pub(crate) fn stamp_intrinsic_widths(lb: &LayoutBox, images: &ImageCtx) {
    stamp_replaced_intrinsic(lb, images);
    match &lb.kind {
        BoxKind::Block(k) | BoxKind::Flex(k) | BoxKind::Grid(k) | BoxKind::Table(k) => {
            for c in k {
                stamp_intrinsic_widths(c, images);
            }
        }
        BoxKind::Lines(items) => items
            .iter()
            .filter_map(inline_atomic)
            .for_each(|c| stamp_intrinsic_widths(c, images)),
        BoxKind::Marker(_) => {}
    }
}

/// Stamp one box's `intrinsic_w` if it is a resolvable replaced image.
fn stamp_replaced_intrinsic(lb: &LayoutBox, images: &ImageCtx) {
    if lb.native_iframe() {
        lb.intrinsic_w.set(Some(300.0));
        return;
    }
    if let Some(viewport) = &lb.svg_viewport {
        let style = lb.resolved(ResolveCtx {
            parent_font_size: super::value::DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        });
        lb.intrinsic_w
            .set(Some(viewport.natural_width(style.font_size)));
        return;
    }
    let Some(src) = lb.image.as_ref().filter(|s| s.replaced) else {
        return;
    };
    if let Some(i) = super::imgsize::box_intrinsic(images.resolver, &src.name, &lb.style) {
        lb.intrinsic_w.set(Some(i.width as f32));
    }
}

/// The box inside an atomic inline item (an inline-block / inline `<img>`), if any.
fn inline_atomic(it: &InlineItem) -> Option<&LayoutBox> {
    match it {
        InlineItem::Atomic(b) => Some(b),
        _ => None,
    }
}

/// Build the fragment for a replaced `<img>` box, or `None` when the box is not
/// a (resolvable) replaced image. The box size is the capped image size; the
/// image is the fragment's own content (no children flow inside an `<img>`).
fn replaced_image_box(
    lb: &LayoutBox,
    bs: &BoxStyle,
    bx: f32,
    by: f32,
    cb_width: f32,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    if let Some(fragment) = replaced_viewport_box(lb, bs, bx, by, cb_width, ctx) {
        return Some(fragment);
    }
    let src = lb.image.as_ref().filter(|s| s.replaced)?;
    let intrinsic = probe_source(lb, src, ctx)?;
    let sizing = size_ctx(bs, cb_width, ctx);
    if ctx.live_geometry {
        let sized = super::imgsize::size_replaced_live(src.name.clone(), intrinsic, &sizing);
        record_metrics(lb, bs, sized.width, sized.height, ctx);
        return Some(live_image_fragment(lb, bx, by, &sized, bs));
    }
    let sized = size_replaced(src.name.clone(), intrinsic, &sizing);
    Some(image_fragment(lb, bx, by, &sized, bs))
}

fn replaced_viewport_box(
    lb: &LayoutBox,
    style: &BoxStyle,
    x: f32,
    y: f32,
    width: f32,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    if lb.native_iframe() {
        return Some(embedded_viewport_fragment(lb, style, x, y, width, ctx));
    }
    let viewport = lb.svg_viewport.as_ref()?;
    Some(svg_viewport_fragment(lb, viewport, style, x, y, width, ctx))
}

fn embedded_viewport_fragment(
    lb: &LayoutBox,
    style: &BoxStyle,
    x: f32,
    y: f32,
    width: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let dimensions = super::imgsize::size_viewport_live(
        Some(300.0),
        None,
        style.aspect_ratio,
        ctx.html_font_scale.unwrap_or(1.0),
        &size_ctx(style, width, ctx),
    );
    viewport_fragment(lb, style, x, y, dimensions, ctx)
}

fn svg_viewport_fragment(
    lb: &LayoutBox,
    viewport: &super::svg_viewport::SvgViewport,
    style: &BoxStyle,
    x: f32,
    y: f32,
    width: f32,
    ctx: &mut Ctx,
) -> Fragment {
    let dimensions = viewport.size(&size_ctx(style, width, ctx));
    viewport_fragment(lb, style, x, y, dimensions, ctx)
}

fn viewport_fragment(
    lb: &LayoutBox,
    style: &BoxStyle,
    x: f32,
    y: f32,
    (content_width, content_height): (f32, f32),
    ctx: &mut Ctx,
) -> Fragment {
    record_metrics(lb, style, content_width, content_height, ctx);
    let width = content_width + style.padding.horizontal() + style.border.widths().horizontal();
    let height = content_height + style.padding.vertical() + style.border.widths().vertical();
    let mut fragment = Fragment::new(
        lb.node_id,
        x,
        y,
        width,
        height,
        content_kind(lb, style, width, height),
    );
    fragment.break_meta = break_meta_of(style);
    fragment.z_index = style.z_index;
    fragment.is_positioned = style.position != Position::Static;
    fragment.opacity = style.opacity;
    fragment
}

/// Assemble the [`SizeCtx`] for the image sizer from a box's style and the
/// layout image context (containing-block width + optional page body height).
fn size_ctx<'a>(bs: &'a BoxStyle, cb_width: f32, ctx: &Ctx) -> SizeCtx<'a> {
    // A positioned image resolves `%` height against its nearest positioned ancestor
    // (`abs_cb_h`); an in-flow one against its parent's definite height (`cb_h`).
    let cbh = if bs.position == Position::Static {
        ctx.cb_h
    } else {
        ctx.abs_cb_h
    };
    SizeCtx {
        style: bs,
        cb_width,
        cb_height: if ctx.html_font_metrics {
            native_height_basis(bs, ctx)
        } else {
            (cbh > 0.0).then_some(cbh)
        },
        body_height: ctx.images.body_height,
    }
}

/// Build a positioned `Image` fragment from a sized image, carrying the box's
/// break metadata so a tall image still obeys break hints.
fn image_fragment(lb: &LayoutBox, bx: f32, by: f32, sized: &SizedImage, bs: &BoxStyle) -> Fragment {
    Fragment {
        node_id: lb.node_id,
        x: bx,
        y: by,
        width: sized.width,
        height: sized.height,
        content: FragmentContent::Image(sized.placement.clone()),
        break_meta: break_meta_of(bs),
        children: Vec::new(),
        z_index: bs.z_index,
        is_positioned: bs.position != Position::Static,
        clip: super::value::clip_region(&lb.style, sized.width, sized.height, 1.0),
        opacity: bs.opacity,
    }
}

fn record_metrics(lb: &LayoutBox, bs: &BoxStyle, width: f32, height: f32, ctx: &mut Ctx) {
    if let Some(transforms) = &mut ctx.native_transforms {
        let border = bs.border.widths();
        if let Some(transform) = super::value::native_transform(
            &lb.style,
            bs,
            width + bs.padding.horizontal() + border.horizontal(),
            height + bs.padding.vertical() + border.vertical(),
        ) {
            transforms.insert(lb.node_id, transform);
        }
    }
    if let Some(metrics) = &mut ctx.used_metrics {
        metrics.insert(
            lb.node_id,
            UsedBoxMetrics {
                inline_fragment: false,
                border: bs.border.widths(),
                padding: bs.padding,
                margin: bs.margin,
                content_width: width,
                content_height: height,
                position: bs.position,
                display: lb.display,
            },
        );
    }
}

/// Keep the replaced border box distinct from its content image fragment.
fn live_image_fragment(
    lb: &LayoutBox,
    bx: f32,
    by: f32,
    sized: &SizedImage,
    bs: &BoxStyle,
) -> Fragment {
    let borders = bs.border.widths();
    let width = sized.width + bs.padding.horizontal() + borders.horizontal();
    let height = sized.height + bs.padding.vertical() + borders.vertical();
    let mut outer = image_fragment(lb, bx, by, sized, bs);
    outer.width = width;
    outer.height = height;
    outer.content = content_kind(lb, bs, width, height);
    let image = image_fragment(
        lb,
        bx + bs.padding.left + borders.left,
        by + bs.padding.top + borders.top,
        sized,
        bs,
    );
    outer.children.push(image);
    outer
}

/// Probe an image source for its intrinsic size + alpha flag via the resolver
/// ([`ImageResolver::intrinsic`](crate::image::ImageResolver::intrinsic)), or `None`
/// if it does not resolve or decode.
fn probe_source(lb: &LayoutBox, src: &ImageSource, ctx: &Ctx) -> Option<crate::image::Intrinsic> {
    super::imgsize::box_intrinsic(ctx.images.resolver, &src.name, &lb.style)
}

/// Lay out a box tree (root from `boxgen::build_box_tree`) into a galley fragment
/// rooted at `(0, 0)` with the given containing-block width. Embeds no images;
/// use [`layout_tree_with_images`] to size `<img>`/`background-image` boxes.
pub fn layout_tree(
    root: &LayoutBox,
    cb_width: f32,
    fonts: &FontRegistry,
    diags: &mut Diagnostics,
) -> Fragment {
    layout_tree_with_images(root, cb_width, fonts, &ImageCtx::none(), diags)
}

/// Lay out a box tree, sizing raster images against `images` (its resolver +
/// page-height basis). See [`layout_tree`] for the image-free entry.
pub fn layout_tree_with_images(
    root: &LayoutBox,
    cb_width: f32,
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
) -> Fragment {
    layout_tree_in_viewport(root, (cb_width, 0.0), fonts, images, diags)
}

/// [`layout_tree_with_images`] in a `(width, height)` viewport: the initial containing
/// block's height is `height` (0 = unknown; see [`crate::layout_in_viewport`]).
pub(crate) fn layout_tree_in_viewport(
    root: &LayoutBox,
    (cb_width, icb_height): (f32, f32),
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
) -> Fragment {
    layout_tree_policy(
        root,
        cb_width,
        (0.0, icb_height),
        (false, TextLayoutPolicy::default(), false),
        fonts,
        images,
        diags,
    )
    .frag
}

/// [`layout_tree_with_images`] that also returns the boxes laid out as table rows
/// (source-recording mode; the galley is identical).
pub(crate) fn layout_tree_recorded(
    root: &LayoutBox,
    (cb_width, icb_height): (f32, f32),
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
) -> (Fragment, std::collections::HashSet<NodeId>) {
    let laid = layout_tree_policy(
        root,
        cb_width,
        (0.0, icb_height),
        (false, TextLayoutPolicy::default(), true),
        fonts,
        images,
        diags,
    );
    (laid.frag, laid.table_rows.unwrap_or_default())
}

pub(crate) fn layout_tree_live(
    root: &LayoutBox,
    viewport: (f32, f32),
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
    text_start: TextLayoutPolicy,
) -> (
    Fragment,
    HashMap<NodeId, UsedBoxMetrics>,
    Option<crate::layout_snapshot::text::TextCapture>,
    HashMap<NodeId, super::value::NativeTransform>,
) {
    let laid = layout_tree_policy(
        root,
        viewport.0,
        (viewport.1, 0.0),
        (true, text_start, false),
        fonts,
        images,
        diags,
    );
    (laid.frag, laid.metrics, laid.text_capture, laid.transforms)
}

/// Everything one [`layout_tree_policy`] run produces.
struct PolicyLayout {
    frag: Fragment,
    metrics: HashMap<NodeId, UsedBoxMetrics>,
    text_capture: Option<crate::layout_snapshot::text::TextCapture>,
    transforms: HashMap<NodeId, super::value::NativeTransform>,
    table_rows: Option<std::collections::HashSet<NodeId>>,
}

fn layout_tree_policy(
    root: &LayoutBox,
    cb_width: f32,
    (viewport_height, icb_h): (f32, f32),
    policy: (bool, TextLayoutPolicy, bool),
    fonts: &FontRegistry,
    images: &ImageCtx,
    diags: &mut Diagnostics,
) -> PolicyLayout {
    let (
        live_geometry,
        TextLayoutPolicy {
            start: text_start,
            original_units,
        },
        record_rows,
    ) = policy;
    let mut ctx = Ctx {
        fonts,
        images,
        diags,
        // The initial containing block is the page: origin (0,0), page width.
        abs_cb_x: 0.0,
        abs_cb_y: 0.0,
        abs_cb_w: cb_width,
        abs_cb_h: viewport_height,
        abs_cb_h_known: live_geometry,
        cb_h: viewport_height,
        cb_h_known: live_geometry,
        root_w: cb_width,
        root_h: viewport_height,
        icb_h,
        abs_cb_icb: true,
        live_geometry,
        used_metrics: live_geometry.then(HashMap::new),
        html_font_metrics: text_start.is_some_and(|value| value.1),
        html_font_scale: text_start.and_then(|value| value.2),
        original_units: original_units.as_deref(),
        native_transforms: live_geometry.then(HashMap::new),
        text_capture: text_start
            .map(|value| crate::layout_snapshot::text::TextCapture::new(value.0)),
        floats: Vec::new(),
        table_rows: record_rows.then(std::collections::HashSet::new),
        dropped_end_margins: HashSet::new(),
    };
    root.set_html_layout_policy((ctx.html_font_metrics, ctx.html_font_scale));
    root.set_original_units(original_units.as_ref());
    stamp_intrinsic_widths(root, images);
    let mut frag = layout_box(root, 0.0, 0.0, cb_width, DEFAULT_FONT_SIZE, &mut ctx);
    // The galley (initial containing block) contains every float — grow it to the
    // lowest float edge if a float outran the in-flow content (a tall right-floated
    // infobox on a short page). Floats inside a nested BFC are already contained.
    let float_bottom = ctx.floats.iter().map(|f| f.bottom).fold(0.0, f32::max);
    if float_bottom > frag.height {
        frag.height = float_bottom;
    }
    PolicyLayout {
        frag,
        metrics: ctx.used_metrics.unwrap_or_default(),
        text_capture: ctx.text_capture,
        transforms: ctx.native_transforms.unwrap_or_default(),
        table_rows: ctx.table_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{inflow_children, zero_or_auto};
    use crate::layout::boxgen::{build_box_tree, BoxKind};
    use crate::layout::value::LengthPct;
    use crate::tag::Tag;
    use crate::{ComputedStyle, StyledElement, StyledNode};

    fn flex_div() -> StyledNode {
        StyledNode::Element(StyledElement {
            tag: Tag::Html("div".to_string()),
            attrs: vec![],
            style: ComputedStyle::from_pairs([("display".to_string(), "flex".to_string())]),
            children: vec![],
            source: None,
        })
    }

    #[test]
    fn line_strut_prefers_line_height_then_face_then_fallback() {
        use super::line_strut;
        use crate::layout::value::{resolve_box_style, ResolveCtx};
        use crate::text::FontRegistry;
        let ctx = ResolveCtx {
            parent_font_size: 16.0,
            cb_width: 100.0,
        };
        let fonts = FontRegistry::new();
        // An explicit `line-height` is the strut directly.
        let explicit = resolve_box_style(
            &ComputedStyle::from_pairs([("line-height".to_string(), "40px".to_string())]),
            ctx,
        );
        assert_eq!(line_strut(&explicit, &fonts), 40.0);
        // No `line-height`: the block's own font natural line height (positive).
        let normal = resolve_box_style(
            &ComputedStyle::from_pairs([("font-size".to_string(), "20px".to_string())]),
            ctx,
        );
        let natural = line_strut(&normal, &fonts);
        assert!(natural > 0.0 && natural < 60.0);
        // No selectable face (empty registry): the `font-size * 1.2` fallback.
        let empty = FontRegistry::default();
        assert_eq!(line_strut(&normal, &empty), normal.font_size * 1.2);
    }

    #[test]
    fn first_inflow_child_is_none_for_a_non_block_box() {
        // The box-flow "first in-flow child" only applies to a `BoxKind::Block`; a
        // flex box hits the let-else guard and yields `None`.
        let root = build_box_tree(&[flex_div()]);
        let BoxKind::Block(kids) = &root.kind else {
            panic!("synthetic root is a block");
        };
        assert!(matches!(kids[0].kind, BoxKind::Flex(_)));
        assert!(inflow_children(&kids[0], 100.0, 16.0).next().is_none());
    }

    #[test]
    fn bottom_margin_collapse_requires_a_provably_zero_minimum_height() {
        // A self-collapsing box (its margins collapse through it) needs a zero or `auto`
        // `height` / `min-height` (miaskiewicz/turbo-dom#76).
        for value in [LengthPct::Auto, LengthPct::Px(0.0), LengthPct::Pct(0.0)] {
            assert!(zero_or_auto(value));
        }
        for value in [
            LengthPct::Px(1.0),
            LengthPct::Pct(10.0),
            LengthPct::Calc { pct: 10.0, px: 0.0 },
            LengthPct::Calc { pct: 0.0, px: 2.0 },
        ] {
            assert!(!zero_or_auto(value));
        }
        assert!(zero_or_auto(LengthPct::Calc { pct: 0.0, px: 0.0 }));
    }
}

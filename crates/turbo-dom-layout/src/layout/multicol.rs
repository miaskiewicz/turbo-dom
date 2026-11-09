//! CSS multi-column layout (§CSS Multicol 1; miaskiewicz/turbo-dom#398).
//!
//! A container is a multicol formatting context when its computed
//! `column-count` or `column-width` is not `auto`; its in-flow children flow
//! into equal-width column strips of the container's content box, laid out
//! sequentially top-to-bottom within each strip and left-to-right across
//! strips (CSS Multicol 1 §3 "The multi-column model", `column-fill: balance`).
//!
//! PR-C1 was the three cleanest Chrome 156 cases (`basic-2col-equal`,
//! `basic-3col-with-gap`, `column-width-driven`): equal-width tracks, no
//! fragmentation, column height = container's definite height.
//!
//! PR-C2 layered `column-fill: balance`, fragmenting fill across the column
//! strips, overflow into extra strips beyond `used_count` (an item that
//! cannot fit in the last tracked column opens a new strip at `col_index *
//! (used_width + gap)` with no upper bound), and a union-geometry pass for
//! fragmented children — Chrome's `getBoundingClientRect()` returns the union
//! of every column fragment a box produces, and the oracle harness asserts
//! against it.
//!
//! PR-D (in `turbo-dom-raster`) painted `column-rule-*` between adjacent
//! columns.
//!
//! PR-E layers `column-span: all` and the `break-{before,after,inside}`
//! flow-honouring on top. A child with `column-span: all` breaks the
//! multicol flow: pre-span children balance over their own sub-flow,
//! the span-all child places at the container's full content inline size
//! (`cw`), and post-span children flow into a fresh column set below
//! (CSS Multicol 1 §6 "Spanning columns"). `break-before: column` /
//! `break-after: column` force a column break at a child's start or after
//! its end; `break-inside: avoid` / `avoid-column` push a child that would
//! fragment into a fresh column instead (when it fits in one; CSS
//! Fragmentation 3 §5.2 "If the fragment does not fit, the user agent may
//! ignore the value"). `break-before: page` / `page` / `avoid-page` are
//! pagination's concern (that's turbo-html2pdf) and are no-ops here but
//! keep their computed value through the cascade for pagination to read.
//! The balancer becomes a binary search over `col_h` for each non-span
//! sub-flow: the smallest height at which the flow (respecting the break
//! rules) fits in `used_count` columns, capped by the container's definite
//! height when present.
//!
//! Chrome 156's multicol widths live on its 1/64 px `LayoutUnit` grid
//! (`crates/turbo-dom-layout/src/layout/Blink-layout-unit.LICENSE`): the
//! `basic-3col-with-gap` case reports each column at `189.328125` px rather
//! than `568÷3 = 189.333…` px because Chrome truncates a `column-width` in
//! `LayoutUnit`s before placing columns, and its last column starts 1/64 px
//! further right than three equal-width tracks would otherwise predict. The
//! exported fragments are compared within a 0.5 px tolerance, so a float
//! formula matches each column's reported `x` / `width` exactly within that
//! bound, and Chrome's balanced column height `content_h / used_count` rounds
//! within the same tolerance.

use super::block::{self, Ctx};
use super::boxgen::LayoutBox;
use super::fragment::{BorderRadii, ColumnRuleStyle, Fragment, FragmentContent};
use super::value::BorderEdges;
use super::value::{
    break_inside_column_avoid_of, break_rule_of, float_of, parse_color, parse_px, position_of,
    BoxSizing, BoxStyle, BreakRule, Float, LengthPct, ResolveCtx, Rgba,
};
use crate::style::ComputedStyle;

/// CSS Multicol 1 §5.1: `column-gap: normal` is `1em`. Chrome 156 uses the font
/// size of the multicol container as the basis; the fixture captures explicit
/// `0` / `16px` gaps, so this path is only exercised by a default-gap future
/// case.
fn normal_gap(fs: f32) -> f32 {
    fs
}

/// Whether `s` establishes a multicol formatting context: a non-`auto`
/// `column-count` or a non-`auto` `column-width`.
pub(crate) fn is_multicol_container(s: &ComputedStyle) -> bool {
    non_auto(s, "column-count") || non_auto(s, "column-width")
}

fn non_auto(s: &ComputedStyle, prop: &str) -> bool {
    match s.get(prop) {
        Some(v) => {
            let t = v.trim();
            !t.is_empty() && t != "auto"
        }
        None => false,
    }
}

/// Parse `column-count: <integer> | auto`. `None` for `auto` / empty /
/// unparsable.
fn parse_column_count(s: &ComputedStyle) -> Option<u32> {
    let raw = s.get("column-count")?.trim();
    if raw == "auto" {
        return None;
    }
    raw.parse::<u32>().ok().filter(|n| *n >= 1)
}

/// Parse `column-width: <length> | auto` in px. `None` for `auto` / empty /
/// unparsable / non-positive (`parse_px` rejects an empty string).
fn parse_column_width(s: &ComputedStyle, fs: f32) -> Option<f32> {
    let raw = s.get("column-width")?.trim();
    if raw == "auto" {
        return None;
    }
    parse_px(raw, fs).filter(|w| *w > 0.0)
}

/// Resolve `column-gap`: a `<length>` in px, or `normal` ≈ `1em`.
fn parse_column_gap(s: &ComputedStyle, fs: f32) -> f32 {
    let raw = s.get("column-gap").map_or("normal", str::trim);
    if raw == "normal" {
        return normal_gap(fs);
    }
    parse_px(raw, fs).unwrap_or_else(|| normal_gap(fs))
}

/// Pick the used column count and width for a container whose content box has
/// inline size `cw`. Follows Chrome's multicol-widen rule: when
/// `column-count` is set, honour it; otherwise `floor((cw + gap) / (width +
/// gap))` columns, each as wide as `(cw - (count - 1) * gap) / count` — i.e.
/// `column-width` is a MINIMUM and the leftover width is distributed back
/// across the resulting columns so they fill the content box.
pub(crate) fn resolve_tracks(s: &ComputedStyle, cw: f32, fs: f32) -> (u32, f32, f32) {
    let gap = parse_column_gap(s, fs);
    let count_hint = parse_column_count(s);
    let width_hint = parse_column_width(s, fs);
    let used_count = match (count_hint, width_hint) {
        (Some(n), _) => n,
        (None, Some(w)) => widen_count(cw, w, gap),
        (None, None) => 1,
    }
    .max(1);
    let used_width = used_column_width(cw, gap, used_count);
    (used_count, used_width, gap)
}

/// `floor((cw + gap) / (width + gap))`, with the content-box edge included so
/// a container exactly as wide as one column admits it (CSS Multicol 1 §7.1
/// "Pseudo-algorithm").
fn widen_count(cw: f32, width: f32, gap: f32) -> u32 {
    if width <= 0.0 {
        return 1;
    }
    let raw = ((cw + gap) / (width + gap)).floor();
    if raw.is_finite() && raw >= 1.0 {
        raw as u32
    } else {
        1
    }
}

/// `(cw - (count - 1) * gap) / count`, clamped at zero so a container
/// narrower than its gaps still measures.
fn used_column_width(cw: f32, gap: f32, count: u32) -> f32 {
    debug_assert!(count >= 1);
    let gaps_total = gap * (count.saturating_sub(1) as f32);
    ((cw - gaps_total) / count as f32).max(0.0)
}

/// The used `column-rule-width` in px (CSS Multicol 1 §5.1, matching Chrome 156
/// `getComputedStyle`): a CSS length, else the `thin` / `medium` / `thick`
/// keywords at 1 / 3 / 5 px. An unparsable value is treated as `medium`.
fn parse_rule_width(s: &ComputedStyle, fs: f32) -> f32 {
    let raw = s.get("column-rule-width").map_or("medium", str::trim);
    match raw {
        "thin" => 1.0,
        "medium" => 3.0,
        "thick" => 5.0,
        "none" => 0.0,
        other => parse_px(other, fs).unwrap_or(3.0),
    }
    .max(0.0)
}

/// The used `column-rule-style` as one of the four [`ColumnRuleStyle`] variants
/// this PR paints, or `None` for `none` / `hidden` / an unsupported keyword
/// (`groove` / `ridge` / `inset` / `outset`: deferred, miaskiewicz/turbo-dom#419).
fn parse_rule_style(s: &ComputedStyle) -> Option<ColumnRuleStyle> {
    match s.get("column-rule-style").map_or("none", str::trim) {
        "solid" => Some(ColumnRuleStyle::Solid),
        "dotted" => Some(ColumnRuleStyle::Dotted),
        "dashed" => Some(ColumnRuleStyle::Dashed),
        "double" => Some(ColumnRuleStyle::Double),
        _ => None,
    }
}

/// The used `column-rule-color`: `currentcolor` resolves to the container's own
/// `color` (initial black). Any parse failure falls back to that same current
/// colour (CSS Multicol 1 §5.3).
fn parse_rule_color(s: &ComputedStyle) -> Rgba {
    let current = s.get("color").and_then(parse_color).unwrap_or(Rgba::BLACK);
    let raw = s.get("column-rule-color").map_or("currentcolor", str::trim);
    if raw.eq_ignore_ascii_case("currentcolor") {
        return current;
    }
    parse_color(raw).unwrap_or(current)
}

/// Emit a rule fragment for each pair of adjacent columns when the container's
/// `column-rule-style` is a paintable keyword and `column-rule-width` is positive
/// (CSS Multicol 1 §5): rule `k` is centered in the gap after column `k`, runs
/// the full column content height, and carries the rule's resolved
/// [`ColumnRuleStyle`] and colour. A rule wider than the gap is clamped to the
/// gap (Blink's `MultiColumnSet::PaintObject` — the rule never paints outside
/// the gap); the `multicol-rule-wider-than-gap` Chrome 156 fixture probes at
/// x = 215 / 225 return the column background, not the rule colour.
///
/// Review R3 M4: rule centres come from the shared [`col_x`] helper — the
/// gap between columns `k` and `k+1` has its centre at the midpoint of
/// `col_x(k) + used_width` (column `k`'s right edge) and `col_x(k+1)`
/// (column `k+1`'s left edge). This is the SAME helper `place_measured`
/// uses to place in-flow kids, so the last in-flow column's end-aligned
/// origin (R2 M3) and the rule's gap-centre share one formula. On a pure
/// float path the shared formula agrees with the earlier stride-based
/// expression; on a rounded `used_width` path the two would otherwise
/// drift at the final gap.
fn column_rule_fragments(
    container: &LayoutBox,
    (cx, cy): (f32, f32),
    (used_count, used_width, gap): (u32, f32, f32),
    cw: f32,
    fs: f32,
    col_h: f32,
) -> Vec<Fragment> {
    if used_count <= 1 || col_h <= 0.0 || gap <= 0.0 {
        return Vec::new();
    }
    let Some(style) = parse_rule_style(&container.style) else {
        return Vec::new();
    };
    // Chrome 156 clamps `column-rule-width` to the gap (Blink's
    // `MultiColumnSet::PaintObject`: the rule is drawn inside the gap; a
    // declared width that exceeds it is truncated), matched by the
    // `multicol-rule-wider-than-gap` fixture.
    let declared = parse_rule_width(&container.style, fs);
    let rule_w = declared.min(gap);
    if rule_w <= 0.0 {
        return Vec::new();
    }
    let color = parse_rule_color(&container.style);
    if color.a == 0 {
        return Vec::new();
    }
    (0..(used_count - 1))
        .map(|k| {
            let right_edge_of_k = col_x(k, (used_count, used_width, gap), cx, cw) + used_width;
            let left_edge_of_k_plus_1 = col_x(k + 1, (used_count, used_width, gap), cx, cw);
            let gap_center = (right_edge_of_k + left_edge_of_k_plus_1) / 2.0;
            Fragment::new(
                container.node_id,
                gap_center - rule_w / 2.0,
                cy,
                rule_w,
                col_h,
                FragmentContent::ColumnRule { style, color },
            )
        })
        .collect()
}

/// Review R2 M3 + R3 M4: the x-origin of column `col_index` within a sub-
/// flow whose content-box spans `[cx, cx+cw]`. For the LAST in-flow column
/// (index `used_count - 1`) the origin is end-aligned at `cx + cw -
/// used_width` (Chrome's Blink multicol); for every other in-flow column
/// and for overflow strips past `used_count`, the origin is the stride
/// formula `cx + col_index * (used_width + gap)`. On a pure-float path the
/// two agree; on a rounded `used_width` path they would otherwise drift by
/// up to 1/64 px at the last column, and `column_rule_fragments` (R3 M4)
/// would lose lockstep with `place_measured` on the final gap-centre.
/// Hoisted out of `place_measured` so both call sites share one formula.
fn col_x(col_index: u32, (used_count, used_width, gap): (u32, f32, f32), cx: f32, cw: f32) -> f32 {
    let stride = used_width + gap;
    if used_count >= 1 && col_index == used_count - 1 {
        cx + cw - used_width
    } else {
        cx + (col_index as f32) * stride
    }
}

/// Per-child flags read from the computed style that steer the multicol flow
/// (CSS Multicol 1 §6, CSS Fragmentation 3 §5): `column-span: all` opens a
/// fresh column set, `break-before: column` / `break-after: column` force a
/// column break around the child, `break-inside: avoid` / `avoid-column`
/// pushes a would-be-fragmented child to a fresh column when it fits in one.
#[derive(Copy, Clone, Default)]
struct KidFlags {
    column_span_all: bool,
    break_before_column: bool,
    break_after_column: bool,
    break_inside_avoid: bool,
}

/// Read the flags of one kid's computed style. Keywords are ASCII
/// case-insensitive (CSS 2.2 §4.1.1). `column-span: all` is the only non-
/// `none` value; `break-{before,after}: column` is the forced column break
/// read through the shared [`break_rule_of`] parser so the paginator and the
/// multicol flow see the same rule set (`page` / `left` / `right` / `recto` /
/// `verso` are pagination's concern, left as a no-op here; turbo-html2pdf#69).
/// `break-inside: avoid | avoid-column` reads through
/// [`break_inside_column_avoid_of`] — multicol's sibling to
/// [`super::value::break_inside_avoid_of`] — so the "which keywords
/// forbid a column break here" set lives in one place next to the
/// page-break parser (R3 M7). We do NOT route `break-inside` through
/// [`break_rule_of`], because that parser collapses `avoid` and
/// `avoid-page` into one [`BreakRule::Avoid`], and `avoid-page` is
/// explicitly pagination-only — it must not forbid a column break.
fn kid_flags(style: &ComputedStyle) -> KidFlags {
    // Review R2 layout M1 + R4 H3: CSS Multicol 1 §6.1 "A spanning box": an
    // out-of-flow (`position: absolute` / `fixed`) or floated element
    // "cannot be column-span-all". Chrome 156 obeys. R4 H3 extends the gate
    // to the three break-related flags: an abspos / floated kid is NOT in
    // the column flow (positioned relative to the containing block) and
    // contributes zero to column-break analysis — `fits_in_count` must
    // not count a `break-before: column` on an abspos kid as a forced
    // column break, nor an `avoid` on a floated kid as a reason to shift
    // the surrounding in-flow content.
    let in_flow = !position_of(style).is_out_of_flow() && float_of(style) == Float::None;
    // R2 M1: span-all is ignored for floats / out-of-flow kids (CSS Multicol §6.1).
    let column_span_all = in_flow && keyword_eq(style, "column-span", "all");
    KidFlags {
        column_span_all,
        // R4 H3: break flags count only for in-flow kids.
        // BreakRule fix: route break-{before,after}: column through the shared parser so
        // `column` is distinguished from `page`.
        break_before_column: in_flow
            && matches!(break_rule_of(style, "break-before"), BreakRule::Column),
        break_after_column: in_flow
            && matches!(break_rule_of(style, "break-after"), BreakRule::Column),
        // miaskiewicz/turbo-dom#440: a span-all segment does not fragment,
        // so `break-inside: avoid | avoid-column` on it has no multicol
        // semantics. Clear it here so the two flags never coexist.
        break_inside_avoid: in_flow && !column_span_all && break_inside_column_avoid_of(style),
    }
}

/// Trim-and-ASCII-lowercase compare one keyword value: the way the cascade's
/// shorthands expand a longhand with surrounding whitespace and the way
/// `column-span` is specified (`all`).
fn keyword_eq(style: &ComputedStyle, prop: &str, want: &str) -> bool {
    style
        .get(prop)
        .is_some_and(|v| v.trim().eq_ignore_ascii_case(want))
}

/// The container's content-box height when its CSS `height` is definite, else
/// `None` for an auto height the balancer resolves from the content.
fn definite_container_height(bs: &BoxStyle) -> Option<f32> {
    match bs.height {
        LengthPct::Px(px) => {
            let inset = match bs.box_sizing {
                BoxSizing::BorderBox => bs.padding.vertical() + bs.border.widths().vertical(),
                BoxSizing::ContentBox => 0.0,
            };
            Some((px - inset).max(0.0))
        }
        _ => None,
    }
}

/// Chrome's getBoundingClientRect-of-fragments tolerance. A column fragment
/// whose used height differs by at most this much from a whole column is
/// treated as a WHOLE column for the fragmentation loop (so a 1/64 px rounding
/// slop does not spawn a zero-height tail piece).
const FRAG_EPS: f32 = 0.5;

/// Lay out a multicol container's kids into equal-width column strips.
/// Returns the item fragments (galley-absolute) and the content height of the
/// container.
///
/// Children with `column-span: all` break the flow: each run of non-span
/// kids becomes a sub-flow balanced over `used_count` columns, each span-all
/// kid lays out at the container's full content inline size `cw`, and
/// successive sub-flows stack below. Within a sub-flow, `break-before:
/// column` / `break-after: column` force column breaks and `break-inside:
/// avoid` / `avoid-column` pushes a would-be-fragmented kid to a fresh
/// column (when it fits in one; a kid taller than `col_h` fragments like
/// any other block, per CSS Fragmentation 3 §5.2).
pub(crate) fn layout_multicol(
    container: &LayoutBox,
    kids: &[LayoutBox],
    cx: f32,
    cy: f32,
    cw: f32,
    bs: &BoxStyle,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    if kids.is_empty() {
        return (Vec::new(), 0.0);
    }
    let fs = bs.font_size;
    let tracks = resolve_tracks(&container.style, cw, fs);
    let container_h = definite_container_height(bs);
    // miaskiewicz/turbo-dom#444: partition kids. Out-of-flow (position:
    // absolute/fixed) and floated kids are NOT in the column flow (CSS
    // Multicol 1 §6.1); they do not consume column space and lay out
    // against their own containing block. Floats inside a column still
    // participate in that column's inline layout — but a float declared
    // DIRECTLY on a multicol kid is treated like an abspos here (R2 L5).
    let (in_flow_kids, oof_kids): (Vec<&LayoutBox>, Vec<&LayoutBox>) =
        kids.iter().partition(|k| !is_out_of_flow_kid(&k.style));
    let in_flow_owned: Vec<LayoutBox> = in_flow_kids.into_iter().cloned().collect();
    let flags: Vec<KidFlags> = in_flow_owned.iter().map(|k| kid_flags(&k.style)).collect();
    let segments = partition_segments(&flags);
    // The container's definite height caps the balanced `col_h` only when
    // the container has a single sub-flow (no span-all child). A sub-flow
    // on either side of a span-all balances to its own content — Chrome
    // leaves the content short in the container rather than stretching
    // sub-flows to share the container height (visible in
    // `span-all-simple`'s 120 px of content inside a 400 px container).
    let single_flow_cap = (segments.len() == 1).then_some(container_h).flatten();
    let mut placed: Vec<Fragment> = Vec::with_capacity(kids.len());
    let mut y_cursor: f32 = 0.0;
    for seg in &segments {
        let seg_out = lay_out_segment(
            seg,
            &in_flow_owned,
            &flags,
            container,
            tracks,
            cw,
            cx,
            cy + y_cursor,
            fs,
            single_flow_cap,
            ctx,
        );
        placed.extend(seg_out.fragments);
        y_cursor += seg_out.advance_y;
    }
    // A definite container height wins; otherwise the box grows to hold all
    // the sub-flows and span-all slabs.
    let content_h = container_h.unwrap_or(y_cursor);
    // miaskiewicz/turbo-dom#444 / #461: lay out out-of-flow kids at the
    // multicol content-box origin as their static-position fallback.
    // `block::layout_multicol_out_of_flow` resolves their used `width`
    // and `height` from their own CSS declarations (not `used_width`)
    // so an abspos kid with an 80x50 box inside a 2-column 400px
    // multicol places at its CSS geometry, never the column strip.
    for kid in &oof_kids {
        let frag = block::layout_multicol_out_of_flow(kid, (cx, cy), cw, fs, ctx);
        placed.push(frag);
    }
    (placed, content_h)
}

/// Whether a multicol kid is laid out out-of-flow for column purposes:
/// `position: absolute` / `fixed`, or a direct float. Mirrors the
/// `in_flow` gate in `kid_flags` so partitioning and flag-clearing stay
/// in lockstep (R2 L5).
fn is_out_of_flow_kid(style: &ComputedStyle) -> bool {
    position_of(style).is_out_of_flow() || float_of(style) != Float::None
}

/// A contiguous slice of children the layout handles as one unit: either a
/// multicol sub-flow (a run of non-span kids balanced over `used_count`
/// columns) or a single span-all kid that occupies the full content inline
/// size on its own row.
enum Segment {
    /// Indices `[start, end)` in `kids` — a non-span sub-flow.
    SubFlow { start: usize, end: usize },
    /// One span-all kid at `index`.
    Span { index: usize },
}

/// Split `kids` into alternating sub-flow and span-all segments. A span-all
/// kid at the very start or end still spawns a zero-width sub-flow around
/// it — [`lay_out_segment`] short-circuits on an empty range.
fn partition_segments(flags: &[KidFlags]) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut start = 0usize;
    for (i, f) in flags.iter().enumerate() {
        if !f.column_span_all {
            continue;
        }
        if i > start {
            out.push(Segment::SubFlow { start, end: i });
        }
        out.push(Segment::Span { index: i });
        start = i + 1;
    }
    if start < flags.len() {
        out.push(Segment::SubFlow {
            start,
            end: flags.len(),
        });
    }
    out
}

/// One segment's output: the fragments it emits (in galley-absolute space)
/// and the vertical advance it contributes to `content_h`.
struct SegmentOut {
    fragments: Vec<Fragment>,
    advance_y: f32,
}

/// Lay out one [`Segment`]: a sub-flow balances its measured kids, flows
/// them into columns respecting break rules and emits column rules; a
/// span-all kid lays out at the container's full content inline size and
/// places at the current y cursor.
#[allow(clippy::too_many_arguments)]
fn lay_out_segment(
    seg: &Segment,
    kids: &[LayoutBox],
    flags: &[KidFlags],
    container: &LayoutBox,
    (used_count, used_width, gap): (u32, f32, f32),
    cw: f32,
    cx: f32,
    cy: f32,
    fs: f32,
    col_h_cap: Option<f32>,
    ctx: &mut Ctx,
) -> SegmentOut {
    match *seg {
        Segment::SubFlow { start, end } => lay_out_subflow(
            &kids[start..end],
            &flags[start..end],
            container,
            (used_count, used_width, gap),
            cx,
            cy,
            fs,
            col_h_cap,
            ctx,
        ),
        Segment::Span { index } => lay_out_span_all(&kids[index], cw, cx, cy, fs, ctx),
    }
}

/// Measure one kid at `cb_width` as an isolated BFC. The returned fragment
/// lives at `(0, 0)` so the caller translates it into place.
fn measure_kid(kid: &LayoutBox, cb_width: f32, fs: f32, ctx: &mut Ctx) -> Fragment {
    let kbs = kid.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width,
    });
    let kw = block::border_box_width(&kbs, cb_width);
    block::layout_box_sized_isolated(kid, &kbs, 0.0, 0.0, kw, ctx)
}

/// Lay out a span-all kid at the container's full content inline size `cw`,
/// place it at `(cx, cy)` and advance by its measured height.
fn lay_out_span_all(
    kid: &LayoutBox,
    cw: f32,
    cx: f32,
    cy: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> SegmentOut {
    let mut m = measure_kid(kid, cw, fs, ctx);
    if cx != 0.0 || cy != 0.0 {
        m.translate(cx, cy);
    }
    let advance = m.height;
    SegmentOut {
        fragments: vec![m],
        advance_y: advance,
    }
}

/// Lay out one sub-flow: measure each kid at `used_width`, balance `col_h`
/// so the break-aware flow fits in `used_count` columns, flow, and append
/// `column-rule-*` fragments for the sub-flow's own column set. `col_h_cap`
/// caps the balanced height when the container has a definite height and
/// no span-all; a capped `col_h` smaller than the break-aware minimum
/// causes overflow into extra strips past `used_count`.
#[allow(clippy::too_many_arguments)]
fn lay_out_subflow(
    kids: &[LayoutBox],
    flags: &[KidFlags],
    container: &LayoutBox,
    (used_count, used_width, gap): (u32, f32, f32),
    cx: f32,
    cy: f32,
    fs: f32,
    col_h_cap: Option<f32>,
    ctx: &mut Ctx,
) -> SegmentOut {
    if kids.is_empty() {
        return SegmentOut {
            fragments: Vec::new(),
            advance_y: 0.0,
        };
    }
    let measured: Vec<Fragment> = kids
        .iter()
        .map(|k| measure_kid(k, used_width, fs, ctx))
        .collect();
    let col_h = capped(section_col_h(&measured, flags, used_count), col_h_cap);
    // Review R2 layout M3: Chrome places the last column at `container_end -
    // used_width` (end-aligned), not at `(used_count - 1) * stride`. With
    // integer-grid rounding they differ by up to 1/64 px. The engine does
    // not truncate `used_width` to the LayoutUnit grid, but if a future
    // change ever rounds `used_width` the stride-based formula would
    // accumulate drift across columns. Thread the container's cw through so
    // `place_measured` can end-align the last column exactly.
    let cw = (used_count.saturating_sub(1) as f32) * gap + (used_count as f32) * used_width;
    let mut out = flow_section(
        measured,
        flags,
        col_h,
        (used_count, used_width, gap),
        cw,
        cx,
        cy,
    );
    let rules = column_rule_fragments(
        container,
        (cx, cy),
        (used_count, used_width, gap),
        cw,
        fs,
        col_h,
    );
    out.extend(rules);
    SegmentOut {
        fragments: out,
        advance_y: col_h,
    }
}

/// `min(col_h, cap)` when `cap` is some and positive, else `col_h`. A
/// capped-below-zero `cap` collapses to zero and overflow absorbs the
/// content (`fragment_pieces` has the zero-col_h short circuit).
fn capped(col_h: f32, cap: Option<f32>) -> f32 {
    cap.map_or(col_h, |c| col_h.min(c.max(0.0)))
}

/// Smallest `col_h` such that the break-aware flow fits in `used_count`
/// columns. `column-fill: balance` (the default) picks the minimum — CSS
/// Multicol 1 §7.3 "Pseudo-algorithm". Without break constraints the
/// minimum is the plain `total / used_count`; `break-before: column` /
/// `break-inside: avoid` can raise it (an unbreakable block has to fit in
/// a single column, and a forced column break splits content into chunks
/// that each occupy whole columns). A binary search finds it: 30 bisections
/// give 1 part in a billion, plenty under the harness's 0.1 px tolerance.
///
/// Unsatisfiable break constraints — e.g. `used_count + 1` kids each with
/// `break-before: column`, where every kid opens a new column so
/// `fits_in_count` returns false regardless of `col_h` — fall back to the
/// plain `total / used_count`. CSS Fragmentation 3 §5.2 lets the UA ignore
/// break values that cannot be satisfied; returning the plain balance keeps
/// overflow strips at the regular column stride instead of a phantom
/// `total + max_single` height that the rule painter would draw through.
fn section_col_h(measured: &[Fragment], flags: &[KidFlags], used_count: u32) -> f32 {
    let heights: Vec<f32> = measured.iter().map(|f| f.height).collect();
    let total: f32 = heights.iter().sum();
    let max_single = heights.iter().copied().fold(0.0_f32, f32::max);
    let n = used_count.max(1) as f32;
    let lo_init = (total / n).max(0.0);
    // `hi` must be large enough to always fit WHEN the break constraints
    // are satisfiable: a `col_h` of `total + max_single` trivially fits
    // every kid in column 0 (one column), which honours every `break-*`
    // flag by definition. If `used_count == 1` the search is unnecessary —
    // the single-column fit is the smallest `col_h`.
    let hi_init = (total + max_single).max(lo_init);
    let (mut lo, mut hi) = (lo_init, hi_init);
    if fits_in_count(&heights, flags, lo, used_count) {
        return lo;
    }
    if !fits_in_count(&heights, flags, hi, used_count) {
        // Unsatisfiable: no `col_h` lets the break-aware flow fit in
        // `used_count` columns. The spec lets the UA ignore the values;
        // fall back to the plain balance so overflow lines up with the
        // regular column stride.
        return lo_init;
    }
    for _ in 0..30 {
        let mid = 0.5 * (lo + hi);
        if fits_in_count(&heights, flags, mid, used_count) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// Whether a break-aware flow at `col_h` fits in `used_count` columns.
/// Walks the kids once, honouring forced column breaks and avoid-column:
/// break-inside:avoid pushes a would-be-fragmented kid to a fresh column
/// (when it fits in one); a kid taller than `col_h` fragments like any
/// other block — "If the fragment does not fit, the user agent may ignore
/// the value" (CSS Fragmentation 3 §5.2).
fn fits_in_count(heights: &[f32], flags: &[KidFlags], col_h: f32, used_count: u32) -> bool {
    if col_h <= 0.0 {
        return false;
    }
    let mut col: u32 = 0;
    let mut cursor: f32 = 0.0;
    let last = heights.len().saturating_sub(1);
    for (i, h) in heights.iter().enumerate() {
        let f = flags[i];
        if f.break_before_column && cursor > 0.0 {
            col += 1;
            cursor = 0.0;
        }
        if f.break_inside_avoid
            && *h + FRAG_EPS <= col_h
            && cursor + *h > col_h + FRAG_EPS
            && cursor > 0.0
        {
            col += 1;
            cursor = 0.0;
        }
        let (c_after, cur_after) = place_in_flow(col, cursor, *h, col_h);
        col = c_after;
        cursor = cur_after;
        // `break-after: column` on the last kid is a no-op (nothing
        // follows to break), so skip it rather than counting a trailing
        // empty strip as an overflow and growing `col_h` unnecessarily.
        if f.break_after_column && i < last {
            col += 1;
            cursor = 0.0;
        }
        if col >= used_count {
            return false;
        }
    }
    true
}

/// Return the `(col, cursor)` after placing a `h`-tall block at a given
/// `(col, cursor)` into columns of `col_h`. A block that fits in the
/// current column's remaining space advances the cursor; one that overruns
/// is cut at the boundary and the tail flows into the next column
/// (fragmentation). A tail that exactly fills a column advances the
/// cursor to the next one. Shared by [`fits_in_count`] and
/// [`flow_section`]'s flow (through [`fragment_pieces`]).
fn place_in_flow(col: u32, cursor: f32, h: f32, col_h: f32) -> (u32, f32) {
    let mut c = col;
    let mut cur = cursor;
    let mut rem = h;
    if col_h <= 0.0 {
        return (c, cur + rem);
    }
    while rem > (col_h - cur) + FRAG_EPS {
        let avail = (col_h - cur).max(0.0);
        rem -= avail;
        c += 1;
        cur = 0.0;
    }
    (c, cur + rem)
}

/// One contiguous fragment of a source box inside its column. Measured in the
/// container's content-box coordinates: `col_index` names the column strip
/// (`0..used_count` for the in-flow strips, `>= used_count` for overflow
/// strips beyond the content box), and `(y, height)` is the fragment's span
/// along the block axis of THAT column.
#[derive(Copy, Clone)]
struct ColPiece {
    col_index: u32,
    y: f32,
    height: f32,
}

/// Flow the measured kids into column strips, honouring the per-kid break
/// rules. A kid that fits in the current column's remaining space is placed
/// whole and `col_cursor` advances; a kid that overruns is cut at the column
/// boundary and the rest flows into the next one (spawning an overflow strip
/// past `used_count` when needed). `break-before: column` forces a new
/// column at the kid's start, `break-inside: avoid` / `avoid-column` pushes
/// a kid that would fragment to a fresh column (when it fits in one), and
/// `break-after: column` forces a new column after the kid.
#[allow(clippy::too_many_arguments)]
fn flow_section(
    measured: Vec<Fragment>,
    flags: &[KidFlags],
    col_h: f32,
    (used_count, used_width, gap): (u32, f32, f32),
    cw: f32,
    cx: f32,
    cy: f32,
) -> Vec<Fragment> {
    let mut out: Vec<Fragment> = Vec::with_capacity(measured.len());
    let mut col_index: u32 = 0;
    let mut col_cursor: f32 = 0.0;
    for (i, frag) in measured.into_iter().enumerate() {
        let f = flags[i];
        apply_pre_break(f, col_h, frag.height, &mut col_index, &mut col_cursor);
        let pieces = fragment_pieces(frag.height, col_h, &mut col_index, &mut col_cursor);
        out.push(place_measured(
            frag,
            &pieces,
            (used_count, used_width, gap),
            cw,
            cx,
            cy,
        ));
        if f.break_after_column {
            col_index += 1;
            col_cursor = 0.0;
        }
    }
    out
}

/// Apply the forced column break before a kid and the `break-inside: avoid`
/// push to a fresh column. Shared between the flow and the fitness check so
/// the two agree on where a column break would occur. `col_h <= 0` disables
/// both: `fits_in_count` early-returns on it and `fragment_pieces` collapses
/// to a single piece at the current cursor, so advancing `col_index` here
/// would stride a kid chain across phantom overflow strips the flow never
/// paints.
fn apply_pre_break(f: KidFlags, col_h: f32, h: f32, col_index: &mut u32, col_cursor: &mut f32) {
    if col_h <= 0.0 {
        return;
    }
    if f.break_before_column && *col_cursor > 0.0 {
        *col_index += 1;
        *col_cursor = 0.0;
    }
    if f.break_inside_avoid
        && h + FRAG_EPS <= col_h
        && *col_cursor + h > col_h + FRAG_EPS
        && *col_cursor > 0.0
    {
        *col_index += 1;
        *col_cursor = 0.0;
    }
}

/// Cut a block of `item_h` into one or more [`ColPiece`]s. The function
/// advances `col_index` / `col_cursor` as the block consumes columns: a block
/// that fits at `col_cursor` consumes exactly `item_h`; one that would overrun
/// is cut at `col_h` and the tail falls to the next column (`col_index += 1`,
/// `col_cursor = 0`). A block too tall for one column leaves a chain of
/// full-column pieces plus a short tail.
fn fragment_pieces(
    item_h: f32,
    col_h: f32,
    col_index: &mut u32,
    col_cursor: &mut f32,
) -> Vec<ColPiece> {
    let mut pieces: Vec<ColPiece> = Vec::new();
    // A column height at or below `FRAG_EPS` can't admit a real piece — the
    // "fits" branch needs `remaining <= available + FRAG_EPS` (so any item
    // wider than ~eps fails it) and the "advance to next column" branch only
    // fires once the cursor has moved off 0, so the loop would keep slicing
    // one ~col_h chunk into every column until `remaining` ran out. Collapse
    // the whole fragmentation to a single piece at the current cursor:
    // caller owns the placement, the flow does not walk forever trying to
    // spill into fresh strips, and pathological styling can't force
    // thousands of pieces (turbo-dom#426). The existing `col_h <= 0.0`
    // policy is a special case of this bound.
    if col_h < FRAG_EPS {
        pieces.push(ColPiece {
            col_index: *col_index,
            y: *col_cursor,
            height: item_h,
        });
        *col_cursor += item_h;
        return pieces;
    }
    let mut remaining = item_h;
    loop {
        let available = (col_h - *col_cursor).max(0.0);
        if available <= FRAG_EPS && *col_cursor > 0.0 {
            // No room (within rounding slop) — open the next column.
            *col_index += 1;
            *col_cursor = 0.0;
            continue;
        }
        if remaining <= available + FRAG_EPS {
            // The block (or its last piece) fits — place and stop.
            pieces.push(ColPiece {
                col_index: *col_index,
                y: *col_cursor,
                height: remaining,
            });
            *col_cursor += remaining;
            return pieces;
        }
        // Partial — fill the column, carry the tail to the next one.
        pieces.push(ColPiece {
            col_index: *col_index,
            y: *col_cursor,
            height: available,
        });
        remaining -= available;
        *col_index += 1;
        *col_cursor = 0.0;
    }
}

/// Translate the measured fragment into its final column strip. For a block
/// that fits in one column (the common case, including the whole of PR-C1),
/// the measured fragment moves whole; a fragmented block collapses into one
/// Fragment whose rectangle is the UNION of its column fragments (Chrome's
/// `getBoundingClientRect()` on a block that split across the column gap).
/// The fragmenter loses the per-column paint for a fragmented block — the
/// union Fragment paints its background once across the gap. A per-column
/// paint of a fragmented block is a future followup; no case in the Chrome
/// 156 oracle paints a fragmented block's background, so PR-C2 / PR-E both
/// pass with the union rect.
#[allow(clippy::too_many_arguments)]
fn place_measured(
    mut measured: Fragment,
    pieces: &[ColPiece],
    (used_count, used_width, gap): (u32, f32, f32),
    cw: f32,
    cx: f32,
    cy: f32,
) -> Fragment {
    // `measure_kids` lays each kid out at `(0, 0)`, so the translation to the
    // column strip is the strip's origin directly — one `translate` rather
    // than a `cx + ... - measured.x` difference whose subtracted zero is
    // indistinguishable from an addition on a mutant.
    debug_assert_eq!(measured.x, 0.0);
    debug_assert_eq!(measured.y, 0.0);
    // Review R2 M3 + R3 M4: column origin helper is now the module-level
    // `col_x`, shared with `column_rule_fragments` so the last-column
    // end-align rule and the gap-centre stay lockstep on any future
    // rounded-`used_width` path.
    let strip_origin = |col_index: u32| col_x(col_index, (used_count, used_width, gap), cx, cw);
    if pieces.len() == 1 {
        let p = pieces[0];
        let dx = strip_origin(p.col_index);
        let dy = cy + p.y;
        if dx != 0.0 || dy != 0.0 {
            measured.translate(dx, dy);
        }
        return measured;
    }
    let (left_col, right_col, min_y, max_bottom) = piece_bounds(pieces);
    let left = strip_origin(left_col);
    let right = strip_origin(right_col) + used_width;
    // miaskiewicz/turbo-dom#427: emit ONE invisible union shell whose
    // `(x, y, width, height)` is the union of the per-column pieces
    // (so `collect_boxes` reports Chrome's `getBoundingClientRect`
    // contract), with one Box child per piece carrying the original
    // paint at that column strip's rect. The painter walks the
    // children and paints each column fragment separately — a block's
    // background and border no longer bleed across the gap.
    //
    // The outer shell carries NO paint: its Box content is zeroed to
    // `background: None` / default borders / no radii / no shadow / no
    // gradient, but the transform is preserved so a CSS `transform`
    // still affects the whole fragmented block (per-column paint is
    // inside the transform).
    //
    // The measured Fragment's original inline children can't be
    // flowed per column at this layer (CSS Fragmentation's per-column
    // text re-layout is a bigger follow-up); they are dropped, as they
    // were before #427. No Chrome 156 oracle case has text content
    // inside a fragmenting block yet.
    let (per_column_content, transform) = match measured.content.clone() {
        FragmentContent::Box {
            background,
            border,
            border_radius,
            shadow,
            gradient,
            transform,
        } => (
            FragmentContent::Box {
                background,
                border,
                border_radius,
                shadow,
                gradient,
                // Each per-column piece paints its own slice, not a
                // double transform of the whole: transform is applied
                // on the outer shell.
                transform: None,
            },
            transform,
        ),
        // Non-Box content (e.g. an image or a text line) isn't a
        // paintable block anyway; fall back to the pre-#427 union rect.
        other => {
            measured.x = left;
            measured.y = cy + min_y;
            measured.width = right - left;
            measured.height = max_bottom - min_y;
            measured.content = other;
            measured.children.clear();
            return measured;
        }
    };
    measured.x = left;
    measured.y = cy + min_y;
    measured.width = right - left;
    measured.height = max_bottom - min_y;
    measured.content = FragmentContent::Box {
        background: None,
        border: BorderEdges::default(),
        border_radius: BorderRadii::ZERO,
        shadow: None,
        gradient: None,
        transform,
    };
    measured.children.clear();
    measured.children.reserve(pieces.len());
    for p in pieces {
        let ox = strip_origin(p.col_index);
        let oy = cy + p.y;
        measured.children.push(Fragment::new(
            measured.node_id,
            ox,
            oy,
            used_width,
            p.height,
            per_column_content.clone(),
        ));
    }
    measured
}

/// `(leftmost column index, rightmost column index, smallest piece y in its
/// column, largest piece bottom in its column)` — the half-planes the union
/// rect of a fragmented block takes its corners from.
fn piece_bounds(pieces: &[ColPiece]) -> (u32, u32, f32, f32) {
    let mut left = u32::MAX;
    let mut right = 0u32;
    let mut min_y = f32::INFINITY;
    let mut max_bottom = f32::NEG_INFINITY;
    for p in pieces {
        left = left.min(p.col_index);
        right = right.max(p.col_index);
        min_y = min_y.min(p.y);
        max_bottom = max_bottom.max(p.y + p.height);
    }
    (left, right, min_y, max_bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::ComputedStyle;

    fn cs(pairs: &[(&str, &str)]) -> ComputedStyle {
        ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
    }

    #[test]
    fn is_multicol_container_reads_both_longhands() {
        assert!(!is_multicol_container(&cs(&[])));
        assert!(!is_multicol_container(&cs(&[
            ("column-count", "auto"),
            ("column-width", "auto"),
        ])));
        assert!(is_multicol_container(&cs(&[("column-count", "2")])));
        assert!(is_multicol_container(&cs(&[("column-width", "180px")])));
        // Blank / whitespace is `auto`-equivalent.
        assert!(!is_multicol_container(&cs(&[("column-count", "   ")])));
    }

    #[test]
    fn resolve_tracks_honours_column_count_first() {
        let s = cs(&[("column-count", "2"), ("column-gap", "0")]);
        let (count, width, gap) = resolve_tracks(&s, 400.0, 16.0);
        assert_eq!(count, 2);
        assert!((width - 200.0).abs() < 1e-5);
        assert!((gap - 0.0).abs() < 1e-5);
    }

    #[test]
    fn resolve_tracks_distributes_leftover_when_width_is_hint() {
        // used_count = floor((500 + 20) / (180 + 20)) = floor(2.6) = 2.
        // used_width = (500 - 20) / 2 = 240.
        let s = cs(&[("column-width", "180px"), ("column-gap", "20px")]);
        let (count, width, gap) = resolve_tracks(&s, 500.0, 16.0);
        assert_eq!(count, 2);
        assert!((width - 240.0).abs() < 1e-5);
        assert!((gap - 20.0).abs() < 1e-5);
    }

    #[test]
    fn resolve_tracks_reports_three_tracks_for_basic_3col() {
        // used_count = 3, used_width = (600 - 32) / 3 = 189.333…
        let s = cs(&[("column-count", "3"), ("column-gap", "16px")]);
        let (count, width, gap) = resolve_tracks(&s, 600.0, 16.0);
        assert_eq!(count, 3);
        assert!((width - 189.333_33).abs() < 0.01);
        assert!((gap - 16.0).abs() < 1e-5);
    }

    #[test]
    fn resolve_tracks_falls_back_to_one_column_when_both_auto() {
        let s = cs(&[("column-count", "auto"), ("column-width", "auto")]);
        let (count, width, _) = resolve_tracks(&s, 400.0, 16.0);
        assert_eq!(count, 1);
        assert!((width - 400.0).abs() < 1e-5);
    }

    #[test]
    fn widen_count_clamps_minimum_one_column() {
        assert_eq!(widen_count(100.0, 0.0, 0.0), 1);
        assert_eq!(widen_count(100.0, 200.0, 0.0), 1);
        assert_eq!(widen_count(500.0, 180.0, 20.0), 2);
        assert_eq!(widen_count(600.0, 180.0, 20.0), 3);
    }

    #[test]
    fn used_column_width_rejects_narrow_container() {
        assert!((used_column_width(500.0, 20.0, 2) - 240.0).abs() < 1e-5);
        assert_eq!(used_column_width(10.0, 20.0, 2), 0.0);
    }

    #[test]
    fn parse_column_gap_resolves_normal_to_font_size() {
        let s = cs(&[("column-gap", "normal")]);
        assert!((parse_column_gap(&s, 20.0) - 20.0).abs() < 1e-5);
        let s = cs(&[]);
        assert!((parse_column_gap(&s, 16.0) - 16.0).abs() < 1e-5);
        let s = cs(&[("column-gap", "16px")]);
        assert!((parse_column_gap(&s, 10.0) - 16.0).abs() < 1e-5);
        // Unparsable falls back to normal.
        let s = cs(&[("column-gap", "wat")]);
        assert!((parse_column_gap(&s, 10.0) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn parse_column_count_accepts_integer_rejects_auto() {
        assert_eq!(parse_column_count(&cs(&[("column-count", "3")])), Some(3));
        assert_eq!(parse_column_count(&cs(&[("column-count", "auto")])), None);
        // Trimming applies before the keyword check (Chrome 156 getComputedStyle
        // emits untrimmed `column-count`/`column-width` values for a shorthand
        // whose expansion keeps surrounding whitespace).
        assert_eq!(
            parse_column_count(&cs(&[("column-count", "  auto  ")])),
            None
        );
        assert_eq!(parse_column_count(&cs(&[("column-count", " 3 ")])), Some(3));
        assert_eq!(parse_column_count(&cs(&[("column-count", "  ")])), None);
        assert_eq!(parse_column_count(&cs(&[])), None);
        // A negative or zero column count is rejected; the fallback is 1.
        assert_eq!(parse_column_count(&cs(&[("column-count", "0")])), None);
    }

    #[test]
    fn parse_column_width_accepts_px_rejects_auto() {
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "180px")]), 16.0),
            Some(180.0)
        );
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "auto")]), 16.0),
            None
        );
        // Same trimming contract as `parse_column_count`.
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "  180px  ")]), 16.0),
            Some(180.0)
        );
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "  auto  ")]), 16.0),
            None
        );
        assert_eq!(parse_column_width(&cs(&[]), 16.0), None);
        // em resolves against the font size.
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "10em")]), 20.0),
            Some(200.0)
        );
        // A non-positive width is rejected.
        assert_eq!(
            parse_column_width(&cs(&[("column-width", "0px")]), 16.0),
            None
        );
    }

    fn frag(h: f32) -> Fragment {
        use super::super::fragment::{BorderRadii, FragmentContent, NodeId};
        use super::super::value::BorderEdges;
        Fragment::new(
            NodeId(0),
            0.0,
            0.0,
            100.0,
            h,
            FragmentContent::Box {
                background: None,
                border: BorderEdges::default(),
                border_radius: BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            },
        )
    }

    fn fs_default() -> Vec<KidFlags> {
        Vec::new()
    }

    #[test]
    fn section_col_h_without_breaks_matches_plain_balance() {
        // 4 blocks of 100 across 2 columns: min col_h = 200.
        let m: Vec<Fragment> = [100.0, 100.0, 100.0, 100.0].into_iter().map(frag).collect();
        let flags = vec![KidFlags::default(); 4];
        let col_h = section_col_h(&m, &flags, 2);
        assert!((col_h - 200.0).abs() < 1e-4);
    }

    #[test]
    fn section_col_h_grows_for_break_before_column() {
        // heights=[80,80,80]; i1 has break-before:column; used=2 → col_h≈160
        // (chrome's reported value, within the fragmenter's `FRAG_EPS`
        // tolerance since the balancer bisects `fits_in_count`).
        let m: Vec<Fragment> = [80.0, 80.0, 80.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 3];
        flags[1].break_before_column = true;
        let col_h = section_col_h(&m, &flags, 2);
        assert!(
            (col_h - 160.0).abs() < FRAG_EPS + 1e-4,
            "col_h = {col_h}, expected within FRAG_EPS of 160"
        );
        // The plain balance (240/2 = 120) is strictly below: the forced
        // break grew col_h.
        assert!(col_h > 120.0 + FRAG_EPS);
    }

    #[test]
    fn section_col_h_grows_for_break_after_column() {
        // Mirror of break-before: i0 has break-after:column; used=2 → ≈160.
        let m: Vec<Fragment> = [80.0, 80.0, 80.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 3];
        flags[0].break_after_column = true;
        let col_h = section_col_h(&m, &flags, 2);
        assert!((col_h - 160.0).abs() < FRAG_EPS + 1e-4, "col_h = {col_h}");
        assert!(col_h > 120.0 + FRAG_EPS);
    }

    #[test]
    fn section_col_h_grows_for_break_inside_avoid() {
        // heights=[60,60,60]; i1 avoid; used=2 → col_h≈120 (i0+i1 col1).
        let m: Vec<Fragment> = [60.0, 60.0, 60.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 3];
        flags[1].break_inside_avoid = true;
        let col_h = section_col_h(&m, &flags, 2);
        assert!((col_h - 120.0).abs() < FRAG_EPS + 1e-4, "col_h = {col_h}");
        // The plain balance (180/2 = 90) is strictly below: the avoid grew col_h.
        assert!(col_h > 90.0 + FRAG_EPS);
    }

    #[test]
    fn section_col_h_ignores_break_after_on_the_last_kid() {
        // Two kids summing to 160, last one with `break-after: column`:
        // the trailing break is a no-op (nothing follows), so the balance
        // stays at 80 per column — a bug that counted it as overflow
        // would instead grow `col_h` to 160.
        let m: Vec<Fragment> = [80.0, 80.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 2];
        flags[1].break_after_column = true;
        let col_h = section_col_h(&m, &flags, 2);
        assert!(
            (col_h - 80.0).abs() < FRAG_EPS + 1e-4,
            "col_h = {col_h}, expected 80 (trailing break-after is a no-op)"
        );
    }

    #[test]
    fn section_col_h_ignores_avoid_when_block_is_taller_than_a_column() {
        // A 300-high avoid block across 2 columns of 100 each (plain
        // balance = 150). Chrome fragments it anyway (CSS Fragmentation
        // 3 §5.2 "may ignore the value"), so the balance stays at 150.
        let m: Vec<Fragment> = [300.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 1];
        flags[0].break_inside_avoid = true;
        let col_h = section_col_h(&m, &flags, 2);
        assert!((col_h - 150.0).abs() < 0.2);
    }

    #[test]
    fn section_col_h_is_zero_for_zero_content() {
        let m: Vec<Fragment> = Vec::new();
        let col_h = section_col_h(&m, &fs_default(), 2);
        assert!(col_h.abs() < 1e-5);
    }

    #[test]
    fn section_col_h_unsatisfiable_breaks_fall_back_to_plain_balance() {
        // Review R1 M8: four blocks of 80 each with `break-before:column`,
        // used=2. Every kid opens a new column, so `fits_in_count` returns
        // false for ANY `col_h`; the previous `hi = total + max_single =
        // 400` would silently fall out of the binary search. The fallback
        // now reports the plain `total / used_count = 160` so the caller
        // produces overflow at the regular stride instead of a phantom
        // 400-px-tall column that the rule paint would have drawn through.
        let m: Vec<Fragment> = [80.0, 80.0, 80.0, 80.0].into_iter().map(frag).collect();
        let mut flags = vec![KidFlags::default(); 4];
        for f in &mut flags {
            f.break_before_column = true;
        }
        let col_h = section_col_h(&m, &flags, 2);
        assert!(
            (col_h - 160.0).abs() < 1e-4,
            "col_h = {col_h}; expected 160 (plain balance) on an \
             unsatisfiable break constraint, not the previous 400"
        );
    }

    #[test]
    fn apply_pre_break_does_not_advance_on_zero_col_h() {
        // Review R1 M2: `fits_in_count` early-returns on `col_h <= 0` and
        // `fragment_pieces` collapses to a single piece at the current
        // cursor, but `apply_pre_break`'s `break_before_column` branch used
        // to advance the column index regardless of `col_h`. That would
        // stride a kid chain with repeated `break-before:column` across
        // phantom column indices in a `height:0` multicol. Both branches
        // must stay put when `col_h <= 0`.
        let f = KidFlags {
            break_before_column: true,
            ..KidFlags::default()
        };
        let mut col_index = 0u32;
        let mut col_cursor = 40.0f32;
        apply_pre_break(f, 0.0, 10.0, &mut col_index, &mut col_cursor);
        assert_eq!(
            col_index, 0,
            "break-before:column must be a no-op when col_h <= 0"
        );
        assert!(
            (col_cursor - 40.0).abs() < 1e-5,
            "col_cursor must not reset when the break is suppressed"
        );
    }

    #[test]
    fn capped_min_with_cap_else_passthrough() {
        assert!((capped(200.0, Some(150.0)) - 150.0).abs() < 1e-5);
        assert!((capped(100.0, Some(150.0)) - 100.0).abs() < 1e-5);
        assert!((capped(200.0, None) - 200.0).abs() < 1e-5);
        // A negative cap clamps to zero.
        assert_eq!(capped(100.0, Some(-10.0)), 0.0);
    }

    #[test]
    fn place_in_flow_advances_within_a_column() {
        let (c, cur) = place_in_flow(0, 60.0, 40.0, 165.0);
        assert_eq!(c, 0);
        assert!((cur - 100.0).abs() < 1e-5);
    }

    #[test]
    fn place_in_flow_splits_at_column_boundary() {
        let (c, cur) = place_in_flow(0, 140.0, 40.0, 165.0);
        // 25 fills col1, 15 lands in col2.
        assert_eq!(c, 1);
        assert!((cur - 15.0).abs() < 1e-5);
    }

    #[test]
    fn place_in_flow_chains_across_several_columns() {
        let (c, cur) = place_in_flow(0, 0.0, 120.0, 50.0);
        // 50 + 50 + 20: ends in col 2 at cursor 20.
        assert_eq!(c, 2);
        assert!((cur - 20.0).abs() < 1e-5);
    }

    #[test]
    fn place_in_flow_short_circuits_on_non_positive_col_height() {
        let (c, cur) = place_in_flow(3, 10.0, 40.0, 0.0);
        assert_eq!(c, 3);
        assert!((cur - 50.0).abs() < 1e-5);
    }

    #[test]
    fn partition_segments_handles_leading_and_trailing_and_adjacent_spans() {
        // Leading span, interior run, two adjacent spans, trailing run.
        let flags = vec![
            KidFlags {
                column_span_all: true,
                ..KidFlags::default()
            },
            KidFlags::default(),
            KidFlags::default(),
            KidFlags {
                column_span_all: true,
                ..KidFlags::default()
            },
            KidFlags {
                column_span_all: true,
                ..KidFlags::default()
            },
            KidFlags::default(),
        ];
        let segs = partition_segments(&flags);
        // Expect: Span(0), SubFlow(1..3), Span(3), Span(4), SubFlow(5..6).
        assert_eq!(segs.len(), 5);
        assert!(matches!(segs[0], Segment::Span { index: 0 }));
        assert!(matches!(segs[1], Segment::SubFlow { start: 1, end: 3 }));
        assert!(matches!(segs[2], Segment::Span { index: 3 }));
        assert!(matches!(segs[3], Segment::Span { index: 4 }));
        assert!(matches!(segs[4], Segment::SubFlow { start: 5, end: 6 }));
    }

    #[test]
    fn kid_flags_reads_all_four_properties_case_insensitively() {
        // `break-inside: avoid-column` kept separate from `column-span: all`
        // (#440: the two normalize to span-all-only; a dedicated test covers
        // that normalization). This case just exercises case-insensitive
        // parsing of each property on an in-flow kid.
        let f = kid_flags(&cs(&[
            ("column-span", "  ALL  "),
            ("break-before", "Column"),
            ("break-after", "  column "),
        ]));
        assert!(f.column_span_all);
        assert!(f.break_before_column);
        assert!(f.break_after_column);
        let f = kid_flags(&cs(&[("break-inside", "AVOID-COLUMN")]));
        assert!(f.break_inside_avoid);
    }

    /// Review R4 H3: R2 M1's in-flow gate applied ONLY to `column_span_all`.
    /// `break_before_column` / `break_after_column` / `break_inside_avoid`
    /// remained unconditional, so a kid with `position: absolute` and
    /// `break-before: column` still fired a forced column break inside
    /// `fits_in_count`. Chrome's abspos / floated kids are NOT in the
    /// column flow (positioned relative to the containing block) and
    /// contribute zero to break analysis; gate all four flags on the
    /// same in-flow predicate.
    #[test]
    fn kid_flags_rejects_break_flags_on_out_of_flow_or_floated_boxes() {
        // Baseline: an in-flow kid with the break keywords carries them.
        let f = kid_flags(&cs(&[
            ("break-before", "column"),
            ("break-after", "column"),
            ("break-inside", "avoid"),
        ]));
        assert!(f.break_before_column);
        assert!(f.break_after_column);
        assert!(f.break_inside_avoid);
        // Out-of-flow kid: every break flag ignored (not in the column flow).
        for position in ["absolute", "fixed"] {
            let f = kid_flags(&cs(&[
                ("break-before", "column"),
                ("break-after", "column"),
                ("break-inside", "avoid"),
                ("position", position),
            ]));
            assert!(
                !f.break_before_column,
                "position:{position}: break-before:column must be ignored \
                 (out-of-flow kid is not in the column flow)",
            );
            assert!(
                !f.break_after_column,
                "position:{position}: break-after:column must be ignored \
                 (out-of-flow kid is not in the column flow)",
            );
            assert!(
                !f.break_inside_avoid,
                "position:{position}: break-inside:avoid must be ignored \
                 (out-of-flow kid is not in the column flow)",
            );
        }
        // Floated kid: ignored (float is out-of-flow for multicol break analysis).
        for float_value in ["left", "right"] {
            let f = kid_flags(&cs(&[
                ("break-before", "column"),
                ("break-after", "column"),
                ("break-inside", "avoid"),
                ("float", float_value),
            ]));
            assert!(
                !f.break_before_column,
                "float:{float_value}: break-before:column must be ignored",
            );
            assert!(
                !f.break_after_column,
                "float:{float_value}: break-after:column must be ignored",
            );
            assert!(
                !f.break_inside_avoid,
                "float:{float_value}: break-inside:avoid must be ignored",
            );
        }
        // `position: relative` / `sticky` / `static` are in-flow — flags stay.
        for position in ["relative", "sticky", "static"] {
            let f = kid_flags(&cs(&[
                ("break-before", "column"),
                ("break-after", "column"),
                ("break-inside", "avoid"),
                ("position", position),
            ]));
            assert!(
                f.break_before_column,
                "position:{position}: break-before:column must stay (in-flow)",
            );
            assert!(
                f.break_after_column,
                "position:{position}: break-after:column must stay (in-flow)",
            );
            assert!(
                f.break_inside_avoid,
                "position:{position}: break-inside:avoid must stay (in-flow)",
            );
        }
    }

    /// Review R2 layout M1: CSS Multicol 1 §6.1 "A spanning box" says an
    /// out-of-flow or floated kid cannot be column-span-all; Chrome 156
    /// honours it. Gate the flag on the box being in-flow — a kid with
    /// `position: absolute` / `position: fixed` / `float: left|right` and
    /// `column-span: all` must NOT be treated as span-all.
    #[test]
    fn kid_flags_rejects_column_span_all_on_out_of_flow_or_floated_boxes() {
        // Baseline: an in-flow kid with `column-span: all` is span-all.
        let f = kid_flags(&cs(&[("column-span", "all")]));
        assert!(f.column_span_all);
        // Out-of-flow kid: the flag is ignored.
        for position in ["absolute", "fixed"] {
            let f = kid_flags(&cs(&[("column-span", "all"), ("position", position)]));
            assert!(
                !f.column_span_all,
                "position:{position}: column-span-all must be ignored",
            );
        }
        // Floated kid: ignored.
        for float_value in ["left", "right"] {
            let f = kid_flags(&cs(&[("column-span", "all"), ("float", float_value)]));
            assert!(
                !f.column_span_all,
                "float:{float_value}: column-span-all must be ignored",
            );
        }
        // `position: relative` / `sticky` are in-flow — still span-all.
        for position in ["relative", "sticky", "static"] {
            let f = kid_flags(&cs(&[("column-span", "all"), ("position", position)]));
            assert!(
                f.column_span_all,
                "position:{position}: column-span-all must stay",
            );
        }
    }

    /// miaskiewicz/turbo-dom#440: a `column-span: all` box is its own
    /// full-width segment (CSS Multicol 1 §6.1 "A spanning box") and does
    /// not fragment — `break-inside: avoid` on it has no multicol
    /// semantics. The engine already acts on `column_span_all` only, but
    /// `KidFlags` must normalize the combination so a future reader never
    /// sees the two flags set at once.
    #[test]
    fn kid_flags_clears_break_inside_avoid_when_column_span_all() {
        let f = kid_flags(&cs(&[("column-span", "all"), ("break-inside", "avoid")]));
        assert!(f.column_span_all);
        assert!(
            !f.break_inside_avoid,
            "break-inside:avoid is meaningless on a column-span:all segment",
        );
        let f = kid_flags(&cs(&[
            ("column-span", "all"),
            ("break-inside", "avoid-column"),
        ]));
        assert!(f.column_span_all);
        assert!(
            !f.break_inside_avoid,
            "break-inside:avoid-column is meaningless on a column-span:all segment",
        );
        // Guard against the inverse mutation (clearing column_span_all
        // instead of break_inside_avoid): with no span-all the avoid flag
        // must still carry through.
        let f = kid_flags(&cs(&[("break-inside", "avoid")]));
        assert!(!f.column_span_all);
        assert!(f.break_inside_avoid);
    }

    #[test]
    fn kid_flags_rejects_unrelated_keywords() {
        // `break-before: page` / `avoid-page` / `avoid` are not a column
        // break (pagination's concern) — PR-E ignores them so they stay
        // no-ops here.
        let f = kid_flags(&cs(&[
            ("column-span", "none"),
            ("break-before", "page"),
            ("break-after", "avoid-page"),
            ("break-inside", "avoid-page"),
        ]));
        assert!(!f.column_span_all);
        assert!(!f.break_before_column);
        assert!(!f.break_after_column);
        assert!(!f.break_inside_avoid);
        // `break-inside: avoid` is also a column-break-avoid (same for
        // multicol as `avoid-column`).
        let f = kid_flags(&cs(&[("break-inside", "avoid")]));
        assert!(f.break_inside_avoid);
    }

    #[test]
    fn kid_flags_distinguishes_column_from_page_breaks() {
        // turbo-html2pdf#69. Before the fix `break_rule_of` collapsed
        // `break-before: column` into `BreakRule::Page`, so the paginator
        // injected an unwanted page break at every column boundary. The
        // multicol flow now reads the SAME parser the paginator reads, so a
        // column break here must stay a column break (it opens a new column,
        // not a new page) and a page break must NOT be mistaken for one (it
        // leaves multicol alone — pagination handles it).
        let col = kid_flags(&cs(&[("break-before", "column")]));
        assert!(
            col.break_before_column,
            "break-before:column → column break"
        );
        let page = kid_flags(&cs(&[("break-before", "page")]));
        assert!(
            !page.break_before_column,
            "break-before:page must NOT be a column break (it is a page break)"
        );
        let col = kid_flags(&cs(&[("break-after", "column")]));
        assert!(col.break_after_column, "break-after:column → column break");
        let page = kid_flags(&cs(&[("break-after", "page")]));
        assert!(
            !page.break_after_column,
            "break-after:page must NOT be a column break (it is a page break)"
        );
        // `break-inside: avoid-column` is a column-break-avoid in multicol;
        // `avoid-page` is pagination's concern and stays off here.
        let avc = kid_flags(&cs(&[("break-inside", "avoid-column")]));
        assert!(avc.break_inside_avoid, "break-inside:avoid-column → avoid");
        let avp = kid_flags(&cs(&[("break-inside", "avoid-page")]));
        assert!(
            !avp.break_inside_avoid,
            "break-inside:avoid-page must NOT be a column-break-avoid"
        );
    }

    #[test]
    fn fragment_pieces_places_a_single_piece_when_the_block_fits() {
        let mut ci: u32 = 0;
        let mut cc: f32 = 60.0;
        let pieces = fragment_pieces(40.0, 165.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].col_index, 0);
        assert!((pieces[0].y - 60.0).abs() < 1e-5);
        assert!((pieces[0].height - 40.0).abs() < 1e-5);
        assert_eq!(ci, 0);
        assert!((cc - 100.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_splits_a_block_at_the_column_boundary() {
        let mut ci: u32 = 0;
        let mut cc: f32 = 140.0;
        // col_h = 165, block = 40 → piece A h=25 (fills col1 from 140), piece B h=15 (0..15 in col2).
        let pieces = fragment_pieces(40.0, 165.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].col_index, 0);
        assert!((pieces[0].y - 140.0).abs() < 1e-5);
        assert!((pieces[0].height - 25.0).abs() < 1e-5);
        assert_eq!(pieces[1].col_index, 1);
        assert_eq!(pieces[1].y, 0.0);
        assert!((pieces[1].height - 15.0).abs() < 1e-5);
        assert_eq!(ci, 1);
        assert!((cc - 15.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_chains_a_block_taller_than_a_column_across_strips() {
        let mut ci: u32 = 0;
        let mut cc: f32 = 0.0;
        // col_h=50, block=120 → piece A 50 in col0, piece B 50 in col1, piece C 20 in col2.
        let pieces = fragment_pieces(120.0, 50.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].col_index, 0);
        assert!((pieces[0].height - 50.0).abs() < 1e-5);
        assert_eq!(pieces[1].col_index, 1);
        assert!((pieces[1].height - 50.0).abs() < 1e-5);
        assert_eq!(pieces[2].col_index, 2);
        assert!((pieces[2].height - 20.0).abs() < 1e-5);
        assert_eq!(ci, 2);
        assert!((cc - 20.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_opens_overflow_strip_when_col_is_full_at_entry() {
        let mut ci: u32 = 0;
        let mut cc: f32 = 150.0; // at col_h, no room.
        let pieces = fragment_pieces(100.0, 150.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].col_index, 1);
        assert_eq!(pieces[0].y, 0.0);
        assert!((pieces[0].height - 100.0).abs() < 1e-5);
        assert_eq!(ci, 1);
        assert!((cc - 100.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_does_not_advance_from_column_top_when_col_h_barely_positive() {
        // col_h = 0.4 (below FRAG_EPS=0.5), item = 10. At the top of a column
        // (cursor=0) the "no room" branch's `col_cursor > 0.0` guard must stay
        // strict: a `>=` mutation would spin the column index past every
        // output without ever placing a piece.
        let mut ci: u32 = 0;
        let mut cc: f32 = 0.0;
        let pieces = fragment_pieces(10.0, 0.4, &mut ci, &mut cc);
        // At least one piece should land in col 0 at y=0 (progress made).
        assert!(!pieces.is_empty());
        assert_eq!(pieces[0].col_index, 0);
        assert_eq!(pieces[0].y, 0.0);
        assert!(pieces[0].height > 0.0);
    }

    #[test]
    fn fragment_pieces_caps_output_for_subeps_col_height() {
        // turbo-dom#426. `col_h` below `FRAG_EPS` can't admit a real piece
        // (the "fits" branch needs `remaining <= available + FRAG_EPS` and
        // the "advance" branch only fires once the cursor has moved off 0),
        // so without a cap the loop fills column after column with the same
        // ~col_h slice — ceil(item_h / col_h) pieces. Treating a sub-eps
        // column like a non-positive one (one piece, caller owns the paint)
        // closes the DoS window and matches the existing `col_h <= 0.0`
        // policy. 100 / 0.3 would otherwise emit > 300 pieces.
        let mut ci: u32 = 0;
        let mut cc: f32 = 0.0;
        let pieces = fragment_pieces(100.0, 0.3, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].col_index, 0);
        assert_eq!(pieces[0].y, 0.0);
        assert!((pieces[0].height - 100.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_trailing_zero_height_after_full_column_advances() {
        // turbo-dom#443. Prior kid filled col 0 to `col_h` exactly; a
        // margin-collapsed trailing zero-height kid must land in col 1, not
        // at (col_index=0, y=col_h). The current `available <= FRAG_EPS &&
        // col_cursor > 0.0` advance branch already handles this (cursor at
        // col_h is > 0, so the branch fires before the "place and stop"
        // branch sees `remaining <= available + FRAG_EPS` as a 0-at-col_h
        // placement). Lock the behaviour so a later refactor can't drop the
        // `> 0.0` guard — which would spin the column index at the top of
        // the FIRST column — or flip the order of the "fits" and "advance"
        // branches.
        let mut ci: u32 = 0;
        let mut cc: f32 = 50.0; // == col_h
        let pieces = fragment_pieces(0.0, 50.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].col_index, 1);
        assert_eq!(pieces[0].y, 0.0);
        assert!((pieces[0].height - 0.0).abs() < 1e-5);
        assert_eq!(ci, 1);
        assert!((cc - 0.0).abs() < 1e-5);
    }

    #[test]
    fn fragment_pieces_single_piece_for_non_positive_col_height() {
        let mut ci: u32 = 0;
        let mut cc: f32 = 0.0;
        let pieces = fragment_pieces(40.0, 0.0, &mut ci, &mut cc);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].col_index, 0);
        assert!((pieces[0].height - 40.0).abs() < 1e-5);
        // The cursor is advanced past the single piece — a later call to the
        // same column strip picks up where this one left off (not the +=
        // branch collapsing to -= or *=, both of which this assertion kills).
        assert!((cc - 40.0).abs() < 1e-5);
        assert_eq!(ci, 0);
    }

    #[test]
    fn parse_rule_width_resolves_keywords_and_lengths() {
        assert!((parse_rule_width(&cs(&[("column-rule-width", "thin")]), 16.0) - 1.0).abs() < 1e-5);
        assert!(
            (parse_rule_width(&cs(&[("column-rule-width", "medium")]), 16.0) - 3.0).abs() < 1e-5
        );
        assert!(
            (parse_rule_width(&cs(&[("column-rule-width", "thick")]), 16.0) - 5.0).abs() < 1e-5
        );
        assert_eq!(
            parse_rule_width(&cs(&[("column-rule-width", "none")]), 16.0),
            0.0
        );
        assert!(
            (parse_rule_width(&cs(&[("column-rule-width", "10px")]), 16.0) - 10.0).abs() < 1e-5
        );
        // Default (unset) is `medium` = 3px.
        assert!((parse_rule_width(&cs(&[]), 16.0) - 3.0).abs() < 1e-5);
        // An unparsable length falls back to `medium`.
        assert!((parse_rule_width(&cs(&[("column-rule-width", "wat")]), 16.0) - 3.0).abs() < 1e-5);
    }

    #[test]
    fn parse_rule_style_accepts_paintable_keywords() {
        assert_eq!(
            parse_rule_style(&cs(&[("column-rule-style", "solid")])),
            Some(ColumnRuleStyle::Solid)
        );
        assert_eq!(
            parse_rule_style(&cs(&[("column-rule-style", "dotted")])),
            Some(ColumnRuleStyle::Dotted)
        );
        assert_eq!(
            parse_rule_style(&cs(&[("column-rule-style", "dashed")])),
            Some(ColumnRuleStyle::Dashed)
        );
        assert_eq!(
            parse_rule_style(&cs(&[("column-rule-style", "double")])),
            Some(ColumnRuleStyle::Double)
        );
        // Default (unset) is `none` and returns None (no rule painted).
        assert_eq!(parse_rule_style(&cs(&[])), None);
        // Deferred keywords (`groove` / `ridge` / `inset` / `outset`) and
        // `hidden` all come back as `None` (miaskiewicz/turbo-dom#419).
        for k in ["hidden", "groove", "ridge", "inset", "outset", "wat"] {
            assert_eq!(parse_rule_style(&cs(&[("column-rule-style", k)])), None);
        }
    }

    fn container(pairs: &[(&str, &str)]) -> LayoutBox {
        super::super::boxgen::anonymous_box(
            super::super::fragment::NodeId(0),
            cs(pairs),
            super::super::boxgen::BoxKind::Block(Vec::new()),
        )
    }

    #[test]
    fn column_rule_fragments_emits_one_per_gap_with_declared_width() {
        // Three columns, non-zero gap, a solid 2px rule centered in each gap.
        let c = container(&[
            ("column-rule-style", "solid"),
            ("column-rule-width", "2px"),
            ("column-rule-color", "#ff0000"),
        ]);
        // `cw = (used_count - 1) * gap + used_count * used_width`:
        // 2 * 20 + 3 * 200 = 640.
        let fr = column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 20.0), 640.0, 16.0, 300.0);
        assert_eq!(fr.len(), 2);
        // Rule 0 centered at cx + 1*width + 0.5*gap = 0 + 200 + 10 = 210 ± 1.
        assert!((fr[0].x - 209.0).abs() < 1e-5); // 210 - 2/2
        assert!((fr[0].y - 0.0).abs() < 1e-5);
        assert!((fr[0].width - 2.0).abs() < 1e-5);
        assert!((fr[0].height - 300.0).abs() < 1e-5);
        // Rule 1 centered at cx + 2*width + 1.5*gap = 0 + 400 + 30 = 430 ± 1.
        assert!((fr[1].x - 429.0).abs() < 1e-5);
    }

    #[test]
    fn column_rule_fragments_declines_single_column_or_zero_gap_or_height() {
        let c = container(&[("column-rule-style", "solid")]);
        // Single column -> no rules. cw = used_width = 200.
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (1, 200.0, 20.0), 200.0, 16.0, 300.0).is_empty()
        );
        // Zero gap -> no rules. cw = 3 * 200 = 600.
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 0.0), 600.0, 16.0, 300.0).is_empty()
        );
        // Non-positive col height -> no rules. cw = 640.
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 20.0), 640.0, 16.0, 0.0).is_empty()
        );
    }

    #[test]
    fn column_rule_fragments_declines_unpaintable_style_or_zero_width_or_transparent_color() {
        // style unset (`none`) -> no rules.
        let c = container(&[]);
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 20.0), 640.0, 16.0, 300.0).is_empty()
        );
        // declared width 0 -> no rules.
        let c = container(&[("column-rule-style", "solid"), ("column-rule-width", "0px")]);
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 20.0), 640.0, 16.0, 300.0).is_empty()
        );
        // Fully transparent colour -> no rules.
        let c = container(&[
            ("column-rule-style", "solid"),
            ("column-rule-color", "rgba(0,0,0,0)"),
        ]);
        assert!(
            column_rule_fragments(&c, (0.0, 0.0), (3, 200.0, 20.0), 640.0, 16.0, 300.0).is_empty()
        );
    }

    #[test]
    fn column_rule_fragments_clamps_width_to_the_gap() {
        // Declared 10px rule into a 4px gap: rule stays centered, truncated to 4px.
        let c = container(&[
            ("column-rule-style", "solid"),
            ("column-rule-width", "10px"),
        ]);
        // cw = 1 * 4 + 2 * 100 = 204.
        let fr = column_rule_fragments(&c, (0.0, 0.0), (2, 100.0, 4.0), 204.0, 16.0, 50.0);
        assert_eq!(fr.len(), 1);
        assert!((fr[0].width - 4.0).abs() < 1e-5);
        // Center of the gap at x=100 + 0.5*4 = 102, left edge at 102 - 2 = 100.
        assert!((fr[0].x - 100.0).abs() < 1e-5);
    }

    /// R3 M4: the shared `col_x` helper must agree with the stride
    /// formula for every column EXCEPT the last in-flow one. For a 1/64
    /// px integer-rounded `used_width` path (future-proofing), the last
    /// in-flow column end-aligns at `cx + cw - used_width`, so the final
    /// gap-centre from `column_rule_fragments` must match the midpoint
    /// between `col_x(used_count-2) + used_width` and
    /// `col_x(used_count-1)` — not the stride-only expression.
    #[test]
    fn column_rule_fragments_use_shared_col_x_for_last_gap() {
        let c = container(&[("column-rule-style", "solid"), ("column-rule-width", "2px")]);
        // Three columns at used_width = 189.328125 (Chrome's 1/64-rounded
        // value), gap = 20, cw = 2 * 20 + 3 * 189.328125 = 607.984375.
        // The LAST column end-aligns at cx + cw - used_width =
        // 0 + 607.984375 - 189.328125 = 418.65625, whereas stride-based
        // would place it at 2 * (189.328125 + 20) = 418.65625 as well
        // (equal for a FLOOR-rounded 1/64 value; the drift appears only
        // on a different rounding direction). Keep the assertion
        // self-consistent: last-gap-centre via col_x equals midpoint of
        // (col_x(1) + used_width) and col_x(2).
        let used_width = 189.328_13_f32;
        let gap = 20.0_f32;
        let cw = 2.0 * gap + 3.0 * used_width;
        let fr = column_rule_fragments(&c, (0.0, 0.0), (3, used_width, gap), cw, 16.0, 100.0);
        assert_eq!(fr.len(), 2);
        let right_edge_of_1 = col_x(1, (3, used_width, gap), 0.0, cw) + used_width;
        let left_edge_of_2 = col_x(2, (3, used_width, gap), 0.0, cw);
        let expected_center = (right_edge_of_1 + left_edge_of_2) / 2.0;
        // fr[1].x is the LEFT edge of the rule; its centre is fr[1].x + 2/2.
        let actual_center = fr[1].x + fr[1].width / 2.0;
        assert!(
            (actual_center - expected_center).abs() < 1e-5,
            "last-gap centre: expected {expected_center}, got {actual_center} \
             — column_rule_fragments must share col_x with place_measured"
        );
    }

    #[test]
    fn parse_rule_color_resolves_currentcolor_and_explicit() {
        let c = parse_rule_color(&cs(&[("color", "#f00")]));
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
        let c = parse_rule_color(&cs(&[
            ("color", "#f00"),
            ("column-rule-color", "currentcolor"),
        ]));
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
        let c = parse_rule_color(&cs(&[("color", "#f00"), ("column-rule-color", "#0f0")]));
        assert_eq!((c.r, c.g, c.b), (0, 255, 0));
        // Unparsable colour falls back to currentcolor.
        let c = parse_rule_color(&cs(&[("color", "#f00"), ("column-rule-color", "wat")]));
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
        // Missing `color` defaults to black.
        let c = parse_rule_color(&cs(&[]));
        assert_eq!((c.r, c.g, c.b), (0, 0, 0));
    }

    #[test]
    fn place_measured_single_piece_translates_to_its_column_strip() {
        use super::super::fragment::{BorderRadii, NodeId};
        use super::super::value::BorderEdges;
        let bx = |w, h| {
            super::super::fragment::Fragment::new(
                NodeId(42),
                0.0,
                0.0,
                w,
                h,
                super::super::fragment::FragmentContent::Box {
                    background: None,
                    border: BorderEdges::default(),
                    border_radius: BorderRadii::ZERO,
                    shadow: None,
                    gradient: None,
                    transform: None,
                },
            )
        };
        let p = [ColPiece {
            col_index: 2,
            y: 45.0,
            height: 60.0,
        }];
        // cw = 3*200 + 2*20 = 640 (gap-consistent). Last column (2) is
        // end-aligned at cw - used_width = 440. Pre-R2 M3 the formula was
        // `col * stride = 2 * 220 = 440`; same value because used_width is
        // not integer-truncated here. Under the end-align formula the
        // answer stays 440.
        let got = place_measured(bx(200.0, 60.0), &p, (3, 200.0, 20.0), 640.0, 0.0, 30.0);
        assert!((got.x - 440.0).abs() < 1e-5, "x = {}", got.x);
        assert!((got.y - 75.0).abs() < 1e-5);
        assert!((got.width - 200.0).abs() < 1e-5);
        assert!((got.height - 60.0).abs() < 1e-5);
    }

    /// Review R2 layout M3: the last column is end-aligned at
    /// `cw - used_width` (Chrome's Blink multicol). On a pure-float path
    /// the formula agrees with `col * stride`, but if a future change
    /// rounds `used_width` to Chrome's 1/64 px `LayoutUnit` grid, the
    /// stride formula would drift by up to 1/64 px. Pin the end-align
    /// behaviour so a `used_width` rounded down still lands the last
    /// column flush with the content-box right edge.
    #[test]
    fn place_measured_end_aligns_the_last_column_on_a_rounded_used_width() {
        use super::super::fragment::{BorderRadii, NodeId};
        use super::super::value::BorderEdges;
        let bx = |w, h| {
            super::super::fragment::Fragment::new(
                NodeId(42),
                0.0,
                0.0,
                w,
                h,
                super::super::fragment::FragmentContent::Box {
                    background: None,
                    border: BorderEdges::default(),
                    border_radius: BorderRadii::ZERO,
                    shadow: None,
                    gradient: None,
                    transform: None,
                },
            )
        };
        // Simulate Chrome's LayoutUnit truncation: cw = 600, gap = 16,
        // used_count = 3, used_width rounded down to 189.328125 (12117/64).
        // Stride formula for col 2 would give `2 * (189.328125 + 16) =
        // 410.65625`. End-align formula gives `600 - 189.328125 =
        // 410.671875` — Chrome's actual column-2 x. The fix must land on
        // 410.671875, not 410.65625.
        // used_width = 12117 / 64 (Chrome 1/64 px grid, exactly representable
        // in f32). Division keeps clippy's excessive-precision lint happy.
        let used_width: f32 = 12117.0 / 64.0;
        let p = [ColPiece {
            col_index: 2,
            y: 0.0,
            height: 50.0,
        }];
        let got = place_measured(
            bx(used_width, 50.0),
            &p,
            (3, used_width, 16.0),
            600.0,
            0.0,
            0.0,
        );
        // Chrome's actual column-2 x: 600 - 12117/64 = 410.671875.
        let want_last = 600.0 - used_width;
        assert!(
            (got.x - want_last).abs() < 1e-5,
            "last column must end-align to cw - used_width (got {})",
            got.x
        );
        // Other columns still use stride placement.
        let p = [ColPiece {
            col_index: 1,
            y: 0.0,
            height: 50.0,
        }];
        let got = place_measured(
            bx(used_width, 50.0),
            &p,
            (3, used_width, 16.0),
            600.0,
            0.0,
            0.0,
        );
        // Column 1: 0 + 1 * (used_width + 16) = used_width + 16.
        let want_col1 = used_width + 16.0;
        assert!(
            (got.x - want_col1).abs() < 1e-5,
            "non-last column uses stride (got {})",
            got.x
        );
    }

    #[test]
    fn place_measured_multi_piece_builds_union_corners() {
        use super::super::fragment::{BorderRadii, NodeId};
        use super::super::value::BorderEdges;
        let bx = |w, h| {
            super::super::fragment::Fragment::new(
                NodeId(42),
                0.0,
                0.0,
                w,
                h,
                super::super::fragment::FragmentContent::Box {
                    background: None,
                    border: BorderEdges::default(),
                    border_radius: BorderRadii::ZERO,
                    shadow: None,
                    gradient: None,
                    transform: None,
                },
            )
        };
        // Hand-crafted multi-piece spread across three columns with non-zero
        // `min_y` so the `cy + min_y` arithmetic cannot collapse to `cy`
        // (an invariant the real flow upholds but a mutant on `+` would exploit).
        let p = [
            ColPiece {
                col_index: 1,
                y: 10.0,
                height: 50.0,
            },
            ColPiece {
                col_index: 2,
                y: 5.0,
                height: 30.0,
            },
            ColPiece {
                col_index: 3,
                y: 20.0,
                height: 40.0,
            },
        ];
        // used_count = 4 so the right_col (3) is the last — end-aligned at
        // `cx + cw - used_width = 100 + 800 - 200 = 700`; adding used_width
        // gives the same right edge the old stride formula produced
        // (`cx + 3*stride + used_width = 900`). left_col=1 is non-last.
        let got = place_measured(bx(200.0, 120.0), &p, (4, 200.0, 0.0), 800.0, 100.0, 50.0);
        // left_col=1 at stride placement: 100 + 1 * 200 = 300.
        // right_col=3 end-aligned: 100 + 800 - 200 = 700; + used_width = 900.
        // width = 900 - 300 = 600. y = cy + min_y = 50 + 5 = 55.
        assert!((got.x - 300.0).abs() < 1e-5);
        assert!((got.y - 55.0).abs() < 1e-5);
        assert!((got.width - 600.0).abs() < 1e-5);
        assert!((got.height - 55.0).abs() < 1e-5);
    }

    #[test]
    fn piece_bounds_covers_columns_and_block_axis() {
        let pieces = [
            ColPiece {
                col_index: 0,
                y: 140.0,
                height: 25.0,
            },
            ColPiece {
                col_index: 1,
                y: 0.0,
                height: 15.0,
            },
        ];
        let (l, r, min_y, max_bottom) = piece_bounds(&pieces);
        assert_eq!(l, 0);
        assert_eq!(r, 1);
        assert!((min_y - 0.0).abs() < 1e-5);
        assert!((max_bottom - 165.0).abs() < 1e-5);
    }

    #[test]
    fn definite_container_height_reports_content_and_border_box() {
        use crate::layout::value::{resolve_box_style, ResolveCtx};
        let ctx = ResolveCtx {
            parent_font_size: 16.0,
            cb_width: 400.0,
        };
        // Content-box height passes through.
        let bs = resolve_box_style(
            &cs(&[("height", "300px"), ("box-sizing", "content-box")]),
            ctx,
        );
        assert_eq!(definite_container_height(&bs), Some(300.0));
        // Border-box height removes padding + border.
        let bs = resolve_box_style(
            &cs(&[
                ("height", "300px"),
                ("padding", "10px"),
                ("border", "5px solid red"),
                ("box-sizing", "border-box"),
            ]),
            ctx,
        );
        assert_eq!(definite_container_height(&bs), Some(270.0));
        // Auto height = indefinite, so the balancer resolves from content.
        let bs = resolve_box_style(&cs(&[("height", "auto")]), ctx);
        assert_eq!(definite_container_height(&bs), None);
    }
}

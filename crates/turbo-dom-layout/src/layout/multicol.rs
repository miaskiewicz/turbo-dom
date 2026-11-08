//! CSS multi-column layout (§CSS Multicol 1; miaskiewicz/turbo-dom#398 PR-C1 /
//! PR-C2 of a multi-PR stack).
//!
//! A container is a multicol formatting context when its computed
//! `column-count` or `column-width` is not `auto`; its in-flow children flow
//! into equal-width column strips of the container's content box, laid out
//! sequentially top-to-bottom within each strip and left-to-right across
//! strips (CSS Multicol 1 §3 "The multi-column model", `column-fill: balance`).
//!
//! PR-C1 scope was the three cleanest Chrome 156 cases (`basic-2col-equal`,
//! `basic-3col-with-gap`, `column-width-driven`): equal-width tracks, no
//! fragmentation, column height = container's definite height.
//!
//! PR-C2 layers `column-fill: balance`, fragmenting fill across the column
//! strips, overflow into extra strips beyond `used_count` (an item that
//! cannot fit in the last tracked column opens a new strip at `col_index *
//! (used_width + gap)` with no upper bound), and a union-geometry pass for
//! fragmented children — Chrome's `getBoundingClientRect()` returns the union
//! of every column fragment a box produces, and the oracle harness asserts
//! against it.
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
//!
//! Scope deferred to PR-E: `column-span: all`, the `break-{before,after,inside}`
//! honouring (`break-inside: avoid` turning a would-be fragmentation into an
//! anticipated column break, `break-after: column`, etc.), and column-rule
//! painting (that's PR-D, in `turbo-dom-raster`). PR-C2's fragmenter splits a
//! box blindly once it would overrun the balanced column height and the union
//! Fragment paints its background once across the gap — the Chrome oracle's
//! `columns-shorthand` i2 / i5 captures assert against the union rect only,
//! so the geometry matches, but a block with its own background painting
//! per column is PR-E / PR-D's job.

use super::block::{self, Ctx};
use super::boxgen::LayoutBox;
use super::fragment::{ColumnRuleStyle, Fragment, FragmentContent};
use super::value::{parse_color, parse_px, BoxSizing, BoxStyle, LengthPct, ResolveCtx, Rgba};
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
/// [`ColumnRuleStyle`] and colour. A rule wider than the gap stays centered and
/// overlaps the adjacent columns, matching Chrome 156's `multicol-rule-wider-
/// than-gap` fixture.
fn column_rule_fragments(
    container: &LayoutBox,
    (cx, cy): (f32, f32),
    (used_count, used_width, gap): (u32, f32, f32),
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
            let k_f = k as f32;
            let gap_center = cx + (k_f + 1.0) * used_width + (k_f + 0.5) * gap;
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
/// PR-C2 adds fragmenting `column-fill: balance` over PR-C1's equal-width
/// sequential-fill scaffold: a block that would overrun the balanced column
/// height is split across the column boundary, and a block that cannot fit
/// inside `used_count` columns opens a new strip at `col_index * (used_width +
/// gap)` with no upper bound (Chrome's "overflow columns" spill, visible past
/// the container's own box when `overflow: visible`).
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
    let (used_count, used_width, gap) = resolve_tracks(&container.style, cw, fs);
    // Pass 1: measure each kid at `used_width` so pass 2 can balance.
    let measured = measure_kids(kids, used_width, fs, ctx);
    // Pass 2: balance the column height. `column-fill: balance` (the default)
    // picks the smallest column height that fits the content in `used_count`
    // columns — `content_h / used_count` — capped at the container's definite
    // height, which allows overflow into extra strips beyond `used_count`
    // (CSS Multicol 1 §7.3 "Pseudo-algorithm").
    let total: f32 = measured.iter().map(|f| f.height).sum();
    let container_h = definite_container_height(bs);
    let col_h = balanced_col_height(total, used_count, container_h);
    // Pass 3: flow measured items into columns in source order, fragmenting a
    // block whose height would overrun the remaining space in the current
    // column and opening overflow strips past `used_count` when needed.
    let mut placed = flow_into_columns(measured, col_h, used_width, gap, cx, cy);
    // A definite height wins; otherwise the box grows to hold the balanced
    // column content.
    let content_h = container_h.unwrap_or(col_h);
    // `column-rule-*` paints between adjacent columns after the content
    // (CSS Multicol 1 §5; miaskiewicz/turbo-dom#398 PR-D). Rules are
    // appended after the content so they paint over any column content
    // they overlap (`multicol-rule-wider-than-gap`). The rule runs the
    // balanced column content height, which may be smaller than the
    // container's definite height — Chrome stops painting a rule at
    // the end of the column it sits between.
    let rules = column_rule_fragments(
        container,
        (cx, cy),
        (used_count, used_width, gap),
        fs,
        col_h,
    );
    placed.extend(rules);
    (placed, content_h)
}

/// Lay out each kid at `(0, 0)` at `used_width` so pass 3 can translate the
/// resulting fragment into its final column strip. An isolated BFC so floats
/// inside the kid are contained and compose into the column's inline flow
/// rather than escaping into the outer multicol container.
fn measure_kids(kids: &[LayoutBox], used_width: f32, fs: f32, ctx: &mut Ctx) -> Vec<Fragment> {
    let mut measured: Vec<Fragment> = Vec::with_capacity(kids.len());
    for kid in kids {
        let kbs = kid.resolved(ResolveCtx {
            parent_font_size: fs,
            cb_width: used_width,
        });
        let kw = block::border_box_width(&kbs, used_width);
        measured.push(block::layout_box_sized_isolated(
            kid, &kbs, 0.0, 0.0, kw, ctx,
        ));
    }
    measured
}

/// `min(total_content / used_count, definite_height)` with a positive floor so
/// an auto-height empty container does not drive the flow loop to zero.
/// `None` for `definite` = auto height, which uses the balanced height straight.
fn balanced_col_height(total: f32, used_count: u32, definite: Option<f32>) -> f32 {
    let balance = (total / used_count.max(1) as f32).max(0.0);
    // `f32::min` over a cap-clamped definite height, so an equivalent `<` /
    // `<=` branch is not open to a mutant on the boundary where balance meets
    // the cap (both branches then return the same value).
    definite.map_or(balance, |def| balance.min(def.max(0.0)))
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

/// Flow the measured kids into column strips. A kid that fits in the current
/// column's remaining space is placed whole and `col_cursor` advances; a kid
/// that overruns is cut at the column boundary and the rest flows into the
/// next one (spawning an overflow strip past `used_count` when needed).
fn flow_into_columns(
    measured: Vec<Fragment>,
    col_h: f32,
    used_width: f32,
    gap: f32,
    cx: f32,
    cy: f32,
) -> Vec<Fragment> {
    let mut out: Vec<Fragment> = Vec::with_capacity(measured.len());
    let mut col_index: u32 = 0;
    let mut col_cursor: f32 = 0.0;
    for frag in measured {
        let pieces = fragment_pieces(frag.height, col_h, &mut col_index, &mut col_cursor);
        out.push(place_measured(frag, &pieces, used_width, gap, cx, cy));
    }
    out
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
    // A non-positive column height collapses fragmentation to a single piece
    // at the current cursor: the caller owns the placement, the flow does not
    // walk forever trying to spill into fresh strips.
    if col_h <= 0.0 {
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
/// PR-C2's fragmenter loses the per-column paint for a fragmented block (the
/// union Fragment paints its background once across the gap) — PR-E owns the
/// per-column paint, and no case in the Chrome 156 oracle paints a
/// fragmented block's background.
fn place_measured(
    mut measured: Fragment,
    pieces: &[ColPiece],
    used_width: f32,
    gap: f32,
    cx: f32,
    cy: f32,
) -> Fragment {
    // `measure_kids` lays each kid out at `(0, 0)`, so the translation to the
    // column strip is the strip's origin directly — one `translate` rather
    // than a `cx + ... - measured.x` difference whose subtracted zero is
    // indistinguishable from an addition on a mutant.
    debug_assert_eq!(measured.x, 0.0);
    debug_assert_eq!(measured.y, 0.0);
    let stride = used_width + gap;
    if pieces.len() == 1 {
        let p = pieces[0];
        let dx = cx + p.col_index as f32 * stride;
        let dy = cy + p.y;
        if dx != 0.0 || dy != 0.0 {
            measured.translate(dx, dy);
        }
        return measured;
    }
    let (left_col, right_col, min_y, max_bottom) = piece_bounds(pieces);
    let left = cx + left_col as f32 * stride;
    let right = cx + right_col as f32 * stride + used_width;
    measured.x = left;
    measured.y = cy + min_y;
    measured.width = right - left;
    measured.height = max_bottom - min_y;
    // The measured Fragment's own children live in its per-column paint; once
    // the block is fragmented across strips the union box cannot host them
    // sensibly (they would paint off their column). Clearing them also keeps
    // `collect_boxes` from re-emitting a now-invalid per-column cid under the
    // union rect. PR-E restores the per-column paint.
    measured.children.clear();
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

    #[test]
    fn balanced_col_height_picks_min_of_balance_and_definite() {
        // definite caps balance (overflow case).
        assert!((balanced_col_height(400.0, 2, Some(150.0)) - 150.0).abs() < 1e-5);
        // balance < definite: use balance (true `column-fill: balance`).
        assert!((balanced_col_height(330.0, 2, Some(200.0)) - 165.0).abs() < 1e-5);
        // no definite: balance wins.
        assert!((balanced_col_height(350.0, 2, None) - 175.0).abs() < 1e-5);
        // auto height, zero content → zero col height.
        assert_eq!(balanced_col_height(0.0, 2, None), 0.0);
        // used_count == 0 is clamped to 1 so a stray style does not div-by-zero.
        assert!((balanced_col_height(100.0, 0, None) - 100.0).abs() < 1e-5);
        // Negative definite is clamped at zero.
        assert_eq!(balanced_col_height(0.0, 2, Some(-10.0)), 0.0);
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

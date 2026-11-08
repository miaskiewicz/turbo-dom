//! CSS multi-column layout (§CSS Multicol 1; miaskiewicz/turbo-dom#398 PR-C1 of
//! a multi-PR stack).
//!
//! A container is a multicol formatting context when its computed
//! `column-count` or `column-width` is not `auto`; its in-flow children flow
//! into equal-width column strips of the container's content box, laid out
//! sequentially top-to-bottom within each strip and left-to-right across
//! strips (CSS Multicol 1 §3 "The multi-column model", `column-fill: balance`).
//!
//! PR-C1 scope is the three cleanest Chrome 156 cases:
//!
//! * `basic-2col-equal` — fixed-height container, two equal-width columns,
//!   `column-gap: 0`.
//! * `basic-3col-with-gap` — fixed-height container, three equal-width
//!   columns, non-zero `column-gap`.
//! * `column-width-driven` — auto `column-count`, non-auto `column-width`, the
//!   container's own height derived from the balanced column content height.
//!
//! The remaining five cases from the oracle
//! (`crates/turbo-dom-layout/tests/fixtures/multicol_chrome156.json`) — the
//! `column-span` capture anomaly in `columns-shorthand` (i2's `offsetWidth:
//! 400, offsetHeight: 133`), the mixed-height sequential-fill, the overflow
//! into an N+1 strip beyond the content box, nested multicol containers and
//! floats inside a column — stay `#[ignore]`d in the harness and land in
//! PR-C2.
//!
//! Chrome 156's multicol widths live on its 1/64 px `LayoutUnit` grid
//! (`crates/turbo-dom-layout/src/layout/Blink-layout-unit.LICENSE`): the
//! `basic-3col-with-gap` case reports each column at `189.328125` px rather
//! than `568÷3 = 189.333…` px because Chrome truncates a `column-width` in
//! `LayoutUnit`s before placing columns, and its last column starts 1/64 px
//! further right than three equal-width tracks would otherwise predict. The
//! exported fragments are compared within a 0.5 px tolerance, so a float
//! formula matches each column's reported `x` / `width` exactly within that
//! bound.

use super::block::{self, Ctx};
use super::boxgen::LayoutBox;
use super::fragment::Fragment;
use super::value::{parse_px, BoxSizing, BoxStyle, LengthPct, ResolveCtx};
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

/// Lay out a multicol container's kids into equal-width column strips.
/// Returns the item fragments (galley-absolute) and the content height of the
/// container.
///
/// PR-C1 scope: equal-width columns, sequential top-to-bottom fill within
/// each column, kids flowed strictly in source order. A child whose height
/// would exceed the remaining space in the current column advances to the
/// next one. The N+1 strip overflow (a child that cannot fit inside
/// `used_count` columns spills into a strip beyond the content box) is
/// PR-C2 — PR-C1 lets the overflowing child flow into the LAST column
/// instead of crashing, matching no Chrome case here but also not regressing
/// any.
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
    // Pass 1: lay each kid out at (0,0) at width `used_width` so we know its
    // measured height; the final placement in pass 3 translates it.
    let mut measured: Vec<Fragment> = Vec::with_capacity(kids.len());
    for kid in kids {
        let kbs = kid.resolved(ResolveCtx {
            parent_font_size: fs,
            cb_width: used_width,
        });
        let kw = block::border_box_width(&kbs, used_width);
        let frag = block::layout_box_sized_isolated(kid, &kbs, 0.0, 0.0, kw, ctx);
        measured.push(frag);
    }
    // Pass 2: pick a column height. A definite container height is the
    // column height; an auto one takes the balanced fill — total measured
    // height divided across `used_count` columns (CSS Multicol 1 §7.3
    // "Pseudo-algorithm", `column-fill: balance`).
    let total: f32 = measured.iter().map(|f| f.height).sum();
    let balanced_h = (total / used_count as f32).max(0.0);
    let container_h = definite_container_height(bs);
    let col_h = container_h.unwrap_or(balanced_h);
    // Pass 3: flow measured items into columns in source order. A child whose
    // height would overrun the remaining column space advances to the next
    // column; staying within column 0 is also fine for an empty container.
    let mut placed: Vec<Fragment> = Vec::with_capacity(measured.len());
    let mut col_index: u32 = 0;
    let mut col_cursor: f32 = 0.0;
    let mut max_col_content: f32 = 0.0;
    for mut frag in measured {
        let kid_h = frag.height;
        if col_cursor > 0.0 && col_cursor + kid_h > col_h + 0.5 && col_index + 1 < used_count {
            col_index += 1;
            col_cursor = 0.0;
        }
        let col_x = cx + col_index as f32 * (used_width + gap);
        let col_y = cy + col_cursor;
        let dx = col_x - frag.x;
        let dy = col_y - frag.y;
        if dx != 0.0 || dy != 0.0 {
            frag.translate(dx, dy);
        }
        col_cursor += kid_h;
        max_col_content = max_col_content.max(col_cursor);
        placed.push(frag);
    }
    // A definite height wins; otherwise the box grows to hold the tallest
    // column's content.
    let content_h = container_h.unwrap_or(max_col_content);
    (placed, content_h)
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

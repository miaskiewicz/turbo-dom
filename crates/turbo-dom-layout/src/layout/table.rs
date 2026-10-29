//! Table layout (§5.4, AC-5.7–5.10). Builds a cell grid (with `colspan`/`rowspan`
//! occupancy), resolves column widths by the `fixed` or `auto` algorithm, lays
//! each cell out at its column-span width, sizes rows to their tallest cell, and
//! applies per-cell `vertical-align`. `<thead>`/`<tfoot>` rows are marked
//! repeatable (`BreakMeta.repeatable`) so the fragmenter can re-emit them on each
//! page a table spans (§6.3, AC-5.8).
//!
//! A `border-collapse: collapse` table (fixed or automatic layout) merges shared cell
//! edges onto a collapsed grid (each shared border counted once, half on each side,
//! CSS 2.1 §17.6.2; the table's own border is the outer grid line) so table/row/cell
//! boxes size correctly across a `colspan`; see the collapsed-border section below.
//! A table with captions is a table wrapper box (CSS Tables 3 §3.1, as Chrome 156 lays it
//! out; miaskiewicz/turbo-dom#51): box generation gives it the captions and one anonymous
//! grid box (see [`layout_wrapper`]).
//! Deferred (documented): a cell taller than its row span expands the last spanned row.

use crate::style::ComputedStyle;
use crate::tag::Attr;
use crate::text::FontRegistry;

use super::block::{self, Ctx};
use super::boxgen::{BoxKind, LayoutBox};
use super::flex::natural_width;
use super::fragment::{Fragment, FragmentContent, NodeId, RepeatKind};
use super::value::{
    break_inside_avoid_of, break_rule_of, parse_px, position_of, resolve_box_style, BorderEdges,
    BoxSizing, BoxStyle, BreakRule, Display, Edges, LengthPct, Position, ResolveCtx, VAlign,
    DEFAULT_FONT_SIZE,
};
use crate::layout_snapshot::UsedBoxMetrics;

// --------------------------------------------------------------------------
// row collection
// --------------------------------------------------------------------------

struct RowRef<'a> {
    node_id: NodeId,
    /// The row's own `position` (its used box metrics report it).
    position: Position,
    cells: Vec<&'a LayoutBox>,
    /// The row's explicit `height` in px (0 if none) — a floor on the row height,
    /// so an empty spacer row (`<tr style="height:5px">`, common in table layouts
    /// like Hacker News) reserves its space instead of collapsing to zero.
    min_height: f32,
    repeat: Option<RepeatKind>,
    /// The row's `break-inside: avoid`, carried to its fragment's break hints
    /// (miaskiewicz/turbo-dom#158).
    avoid_break_inside: bool,
    /// The row's forced/avoid breaks, with its row group's `break-before` on its
    /// first row and `break-after` on its last (Chrome 156 honours both on rows
    /// and `<tbody>`s; a row group has no fragment of its own).
    break_before: BreakRule,
    break_after: BreakRule,
}

fn cells_of(row: &LayoutBox) -> Vec<&LayoutBox> {
    match &row.kind {
        BoxKind::Block(cells) => cells.iter().collect(),
        _ => Vec::new(),
    }
}

fn rows_of(group: &LayoutBox) -> Vec<&LayoutBox> {
    match &group.kind {
        BoxKind::Block(rows) => rows.iter().collect(),
        _ => Vec::new(),
    }
}

fn group_repeat(d: Display) -> Option<RepeatKind> {
    match d {
        Display::TableHeaderGroup => Some(RepeatKind::Header),
        Display::TableFooterGroup => Some(RepeatKind::Footer),
        _ => None,
    }
}

fn row_ref(row: &LayoutBox, repeat: Option<RepeatKind>) -> RowRef<'_> {
    RowRef {
        node_id: row.node_id,
        position: position_of(&row.style),
        cells: cells_of(row),
        min_height: row_min_height(row),
        repeat,
        avoid_break_inside: break_inside_avoid_of(&row.style),
        break_before: break_rule_of(&row.style, "break-before"),
        break_after: break_rule_of(&row.style, "break-after"),
    }
}

/// A table row's explicit `height` in px (0 if unset/relative) — used as a floor
/// on the computed row height.
fn row_min_height(row: &LayoutBox) -> f32 {
    row.style
        .get("height")
        .and_then(|v| parse_px(v, DEFAULT_FONT_SIZE))
        .unwrap_or(0.0_f32)
        .max(0.0)
}

/// A row group (`<tbody>` / `<thead>` / `<tfoot>`) and the run of collected rows it
/// holds (`rows[start..start + count]`).
struct GroupRef {
    node_id: NodeId,
    position: Position,
    display: Display,
    start: usize,
    count: usize,
}

/// The row groups among a table's children (every child that is not itself a row,
/// as [`collect_rows`] flattens them), empty ones included, in order, indexed into
/// [`collect_rows`]' rows.
fn row_groups(items: &[LayoutBox]) -> Vec<GroupRef> {
    let mut out = Vec::new();
    let mut start = 0;
    for child in items {
        let row = matches!(child.display, Display::TableRow);
        let count = if row { 1 } else { rows_of(child).len() };
        if !row {
            out.push(GroupRef {
                node_id: child.node_id,
                position: position_of(&child.style),
                display: child.display,
                start,
                count,
            });
        }
        start += count;
    }
    out
}

/// A run of a table's rows that forms one section: a row group's rows, or consecutive
/// rows directly in the table (an anonymous body): `rows[start..start + count]`, and
/// whether it is a body (not a `<thead>` / `<tfoot>`).
struct Section {
    start: usize,
    count: usize,
    body: bool,
}

/// The sections of a table's children, indexed into [`collect_rows`]' rows.
fn sections(items: &[LayoutBox]) -> Vec<Section> {
    let mut out = Vec::new();
    let mut start = 0;
    for child in items {
        let row = child.display == Display::TableRow;
        let count = if row { 1 } else { rows_of(child).len() };
        match out.last_mut() {
            Some(Section {
                start: s,
                count: n,
                body: true,
            }) if row && *s + *n == start => {
                *n += 1;
            }
            _ => out.push(Section {
                start,
                count,
                body: group_repeat(child.display).is_none(),
            }),
        }
        start += count;
    }
    out
}

/// Give a table's `extra` height (beyond its rows and spacing) to its rows `h`, as Chrome
/// 156 does: to the body sections (else every section) in proportion to their `weights`,
/// and inside a section to its rows without an explicit height (else all of them), in
/// proportion — equally when they weigh nothing (miaskiewicz/turbo-dom#64).
fn distribute_extra(
    h: &mut [f32],
    weights: &[f32],
    (rows, sections): (&[RowRef], &[Section]),
    extra: f32,
) {
    if extra <= 0.0 {
        return;
    }
    let chosen = receiving_sections(sections);
    let section_weights: Vec<f32> = chosen
        .iter()
        .map(|s| weights[s.start..s.start + s.count].iter().sum())
        .collect();
    for (section, share) in chosen.iter().zip(shares(extra, &section_weights)) {
        let range = section.start..section.start + section.count;
        let auto: Vec<usize> = range
            .clone()
            .filter(|&r| rows[r].min_height <= 0.0)
            .collect();
        let idx: Vec<usize> = if auto.is_empty() {
            range.collect()
        } else {
            auto
        };
        let row_weights: Vec<f32> = idx.iter().map(|&r| weights[r]).collect();
        for (&r, add) in idx.iter().zip(shares(share, &row_weights)) {
            h[r] += add;
        }
    }
}

/// The sections that receive a table's extra height: its non-empty bodies, else every
/// non-empty section.
fn receiving_sections(sections: &[Section]) -> Vec<&Section> {
    let filled: Vec<&Section> = sections.iter().filter(|s| s.count > 0).collect();
    let bodies: Vec<&Section> = filled.iter().copied().filter(|s| s.body).collect();
    if bodies.is_empty() {
        filled
    } else {
        bodies
    }
}

/// `total` split in proportion to `weights` (equally when they sum to 0), each share
/// floored to Chrome's layout unit (1/64 px) and the last taking the remainder.
fn shares(total: f32, weights: &[f32]) -> Vec<f32> {
    let sum: f32 = weights.iter().sum();
    let n = weights.len() as f32;
    let mut left = total;
    let mut out: Vec<f32> = weights
        .iter()
        .map(|w| {
            let share = if sum > 0.0 {
                total * w / sum
            } else {
                total / n
            };
            let share = (share * 64.0).floor() / 64.0;
            left -= share;
            share
        })
        .collect();
    if let Some(last) = out.last_mut() {
        *last += left;
    }
    out
}

/// The group's break value where it is set, else the row's own (a forced `Page`
/// on either side wins over `Avoid`, CSS Fragmentation 3 §3.1).
fn stronger(group: BreakRule, row: BreakRule) -> BreakRule {
    match (group, row) {
        (BreakRule::Page, _) | (_, BreakRule::Page) => BreakRule::Page,
        (BreakRule::Avoid, _) | (_, BreakRule::Avoid) => BreakRule::Avoid,
        _ => BreakRule::Auto,
    }
}

fn collect_rows(items: &[LayoutBox]) -> Vec<RowRef<'_>> {
    let mut out = Vec::new();
    for child in items {
        if matches!(child.display, Display::TableRow) {
            out.push(row_ref(child, None));
        } else {
            let repeat = group_repeat(child.display);
            let mut rows: Vec<_> = rows_of(child)
                .into_iter()
                .map(|r| row_ref(r, repeat))
                .collect();
            if let Some(first) = rows.first_mut() {
                first.break_before = stronger(
                    break_rule_of(&child.style, "break-before"),
                    first.break_before,
                );
            }
            if let Some(last) = rows.last_mut() {
                last.break_after =
                    stronger(break_rule_of(&child.style, "break-after"), last.break_after);
            }
            out.extend(rows);
        }
    }
    out
}

// --------------------------------------------------------------------------
// grid placement (colspan / rowspan occupancy)
// --------------------------------------------------------------------------

struct Placed<'a> {
    lb: &'a LayoutBox,
    row: usize,
    col: usize,
    colspan: usize,
    rowspan: usize,
}

fn attr_usize(attrs: &[Attr], name: &str) -> usize {
    attrs
        .iter()
        .find(|a| a.name == name)
        .and_then(|a| a.value.trim().parse().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(1)
}

fn ensure(occ: &mut Vec<Vec<bool>>, rows: usize, cols: usize) {
    while occ.len() < rows {
        occ.push(Vec::new());
    }
    for row in occ.iter_mut() {
        while row.len() < cols {
            row.push(false);
        }
    }
}

fn next_free(row: &[bool], start: usize) -> usize {
    let mut c = start;
    while c < row.len() && row[c] {
        c += 1;
    }
    c
}

fn mark(occ: &mut Vec<Vec<bool>>, r: usize, col: usize, cs: usize, rs: usize) {
    ensure(occ, r + rs, col + cs);
    for row in occ.iter_mut().take(r + rs).skip(r) {
        row[col..col + cs].fill(true);
    }
}

fn place_row<'a>(
    occ: &mut Vec<Vec<bool>>,
    row: &RowRef<'a>,
    r: usize,
    placed: &mut Vec<Placed<'a>>,
) -> usize {
    ensure(occ, r + 1, 0);
    let mut col = 0;
    let mut max_col = 0;
    for cell in &row.cells {
        col = next_free(&occ[r], col);
        let colspan = attr_usize(&cell.attrs, "colspan");
        let rowspan = attr_usize(&cell.attrs, "rowspan");
        mark(occ, r, col, colspan, rowspan);
        placed.push(Placed {
            lb: cell,
            row: r,
            col,
            colspan,
            rowspan,
        });
        col += colspan;
        max_col = max_col.max(col);
    }
    max_col
}

fn build_grid<'a>(rows: &[RowRef<'a>]) -> (Vec<Placed<'a>>, usize) {
    let mut occ: Vec<Vec<bool>> = Vec::new();
    let mut placed = Vec::new();
    let mut ncols = 0;
    for (r, row) in rows.iter().enumerate() {
        ncols = ncols.max(place_row(&mut occ, row, r, &mut placed));
    }
    (placed, ncols)
}

// --------------------------------------------------------------------------
// column widths
// --------------------------------------------------------------------------

fn is_fixed(style: &ComputedStyle) -> bool {
    style.get("table-layout").map(str::trim) == Some("fixed")
}

/// Whether the fixed table layout governs: `table-layout: fixed` with a definite
/// (non-`auto`) width; a `width: auto` fixed table uses the automatic layout (Chrome
/// 156).
fn fixed_layout(style: &ComputedStyle) -> bool {
    is_fixed(style)
        && style
            .get("width")
            .is_some_and(|width| !width.trim().eq_ignore_ascii_case("auto"))
}

/// The table's own left + right border widths. In the collapsed model they are the
/// outer grid lines, part of the grid (miaskiewicz/turbo-dom#49).
fn table_border(style: &ComputedStyle) -> Edges {
    resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        },
    )
    .border
    .widths()
}

fn collapsed(style: &ComputedStyle) -> bool {
    style.get("border-collapse").map(str::trim) == Some("collapse")
}

fn explicit_width(lb: &LayoutBox) -> Option<f32> {
    let bs = lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    let frame = bs.padding.horizontal() + bs.border.widths().horizontal();
    match bs.width {
        LengthPct::Px(w) => Some(block::to_border(w, bs.box_sizing, frame)),
        _ => None,
    }
}

fn cell_span_width(cols: &[f32], col: usize, span: usize) -> f32 {
    cols[col..col + span].iter().sum()
}

/// Each column's max-content for a separated table with horizontal border-spacing
/// `hs`.
fn auto_columns(placed: &[Placed], ncols: usize, hs: f32, fonts: &FontRegistry) -> Vec<f32> {
    let mut w = vec![0.0_f32; ncols];
    for p in placed {
        if p.colspan == 1 {
            w[p.col] = w[p.col].max(natural_width(p.lb, fonts));
        }
    }
    for p in placed {
        fit_span(&mut w, p, hs, fonts);
    }
    w
}

/// Widen the columns a spanning cell covers to its max-content: the spacing between
/// them counts towards the span and the excess spreads in proportion to the columns'
/// widths, evenly when they are empty (Chrome 156; miaskiewicz/turbo-dom#56).
fn fit_span(w: &mut [f32], p: &Placed, hs: f32, fonts: &FontRegistry) {
    if p.colspan > 1 {
        let have = cell_span_width(w, p.col, p.colspan) + hs * (p.colspan - 1) as f32;
        let need = natural_width(p.lb, fonts) - have;
        if need > 0.0 {
            spread(&mut w[p.col..p.col + p.colspan], need);
        }
    }
}

fn first_row_widths(placed: &[Placed], ncols: usize) -> Vec<Option<f32>> {
    let mut w = vec![None; ncols];
    for p in placed {
        if p.row == 0 && p.colspan == 1 && w[p.col].is_none() {
            w[p.col] = explicit_width(p.lb);
        }
    }
    w
}

fn share(remaining: f32, count: usize) -> f32 {
    if count == 0 {
        0.0
    } else {
        (remaining / count as f32).max(0.0)
    }
}

fn fixed_columns(placed: &[Placed], ncols: usize, table_width: f32) -> Vec<f32> {
    let w = first_row_widths(placed, ncols);
    let specified: f32 = w.iter().flatten().sum();
    let unspec = w.iter().filter(|x| x.is_none()).count();
    let each = share(table_width - specified, unspec);
    w.iter().map(|x| x.unwrap_or(each)).collect()
}

/// Each column's min-content width (widest single-column cell that can't shrink).
fn min_columns(placed: &[Placed], ncols: usize, fonts: &FontRegistry) -> Vec<f32> {
    let mut w = vec![0.0_f32; ncols];
    for p in placed {
        if p.colspan == 1 {
            w[p.col] = w[p.col].max(super::flex::min_content_width(p.lb, fonts));
        }
    }
    w
}

/// Each column's percentage width: the largest `width: N%` of its single-column cells
/// (miaskiewicz/turbo-dom#50).
fn column_percents(placed: &[Placed], ncols: usize) -> Vec<Option<f32>> {
    let mut pct = vec![None; ncols];
    for p in placed.iter().filter(|p| p.colspan == 1) {
        if let Some(v) = cell_percent(p.lb) {
            pct[p.col] = Some(pct[p.col].map_or(v, |w: f32| w.max(v)));
        }
    }
    pct
}

/// A cell's positive `width` percentage.
fn cell_percent(lb: &LayoutBox) -> Option<f32> {
    let bs = lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    match bs.width {
        LengthPct::Pct(p) if p > 0.0 => Some(p),
        _ => None,
    }
}

/// The columns' max-content total honouring percentage columns, as Chrome 156 sizes an
/// auto table: the largest of the plain sum, each percentage column's max-content
/// over its share, and the other columns' over the share left to them — and whether
/// it is unbounded (percentages total 100% or more beside other content: the table
/// then fills the line).
fn percent_aware_total(cols: &[f32], pcts: &[Option<f32>]) -> (f32, bool) {
    let sum: f32 = cols.iter().sum();
    let pct_total = pcts.iter().flatten().sum::<f32>().min(100.0);
    if pct_total <= 0.0 {
        return (sum, false);
    }
    let paired = || cols.iter().zip(pcts);
    let from_pct = paired()
        .filter_map(|(c, p)| p.map(|p| c * 100.0 / p))
        .fold(0.0_f32, f32::max);
    let rest: f32 = paired().filter(|(_, p)| p.is_none()).map(|(c, _)| c).sum();
    let from_rest = if pct_total < 100.0 {
        rest * 100.0 / (100.0 - pct_total)
    } else {
        0.0
    };
    (
        sum.max(from_pct).max(from_rest),
        pct_total >= 100.0 && rest > 0.0,
    )
}

/// Fit the auto `cols` to the table's `target` width: a percentage column takes its
/// share of `target` (never below its min-content; shares over 100% scale down), the
/// other columns share the rest as [`scale_to`] does, taking back from the percentage
/// columns what their minima need (miaskiewicz/turbo-dom#50). Without percentages this
/// is [`scale_to`].
fn fit_columns(cols: &mut [f32], limits: &Limits, pcts: &[Option<f32>], target: f32) {
    if pcts.iter().all(Option::is_none) {
        return scale_to(cols, target, limits);
    }
    let mins = &limits.mins[..];
    let total: f32 = pcts.iter().flatten().sum();
    let scale = 100.0_f32.min(total) / total / 100.0;
    let (pct_cols, auto_cols): (Vec<usize>, Vec<usize>) =
        (0..cols.len()).partition(|&i| pcts[i].is_some());
    for &i in &pct_cols {
        cols[i] = mins[i].max(target * pcts[i].unwrap_or(0.0) * scale);
    }
    let auto_min: f32 = auto_cols.iter().map(|&i| mins[i]).sum();
    let taken: f32 = pct_cols.iter().map(|&i| cols[i]).sum();
    if taken > target - auto_min {
        shrink_subset(cols, mins, &pct_cols, taken - (target - auto_min).max(0.0));
    }
    // Only percentage columns: they fill the table between them.
    let (idx, target) = if auto_cols.is_empty() {
        (&pct_cols, target)
    } else {
        let rest = target - pct_cols.iter().map(|&i| cols[i]).sum::<f32>();
        (&auto_cols, rest.max(auto_min))
    };
    let mut subset: Vec<f32> = idx.iter().map(|&i| cols[i]).collect();
    scale_to(&mut subset, target, &limits.subset(idx));
    scatter(cols, idx, &subset);
}

/// What column fitting honours per column: its min-content width, and whether a cell
/// gives it a `px` width (such a column grows only when no auto column can;
/// miaskiewicz/turbo-dom#55).
struct Limits {
    mins: Vec<f32>,
    fixed: Vec<bool>,
}

impl Limits {
    /// The limits of the columns `idx`.
    fn subset(&self, idx: &[usize]) -> Limits {
        Limits {
            mins: idx.iter().map(|&i| self.mins[i]).collect(),
            fixed: idx.iter().map(|&i| self.fixed[i]).collect(),
        }
    }
}

/// Which of `ncols` columns a single-column cell gives a `px` width (`width_of`).
fn px_columns(
    placed: &[Placed],
    ncols: usize,
    width_of: fn(&LayoutBox) -> Option<f32>,
) -> Vec<bool> {
    let mut fixed = vec![false; ncols];
    for p in placed.iter().filter(|p| p.colspan == 1) {
        fixed[p.col] |= width_of(p.lb).is_some();
    }
    fixed
}

/// [`shrink_columns`] over the columns `idx` of `cols`.
fn shrink_subset(cols: &mut [f32], mins: &[f32], idx: &[usize], reduce: f32) {
    let mut subset: Vec<f32> = idx.iter().map(|&i| cols[i]).collect();
    let subset_mins: Vec<f32> = idx.iter().map(|&i| mins[i]).collect();
    shrink_columns(&mut subset, &subset_mins, reduce);
    scatter(cols, idx, &subset);
}

fn scatter(cols: &mut [f32], idx: &[usize], values: &[f32]) {
    for (&i, &v) in idx.iter().zip(values) {
        cols[i] = v;
    }
}

/// Fit `cols` to `target`. Growing goes to the auto columns ([`grow_columns`]);
/// shrinking takes only from each column's slack above its min-content, so a column
/// never clips its content (the infobox label column kept clipping "Infraclass:" when
/// the whole row was scaled down uniformly). If even the minima don't fit, columns sit
/// at their min-content and the table overflows.
fn scale_to(cols: &mut [f32], target: f32, limits: &Limits) {
    let sum: f32 = cols.iter().sum();
    if cols.is_empty() || (sum - target).abs() <= f32::EPSILON {
        return;
    }
    if target >= sum {
        grow_columns(cols, &limits.fixed, target, sum);
    } else {
        shrink_columns(cols, &limits.mins, sum - target);
    }
}

/// Grow `cols` (summing `sum`) to `target`, as Chrome 156 distributes a table's extra
/// width: over the auto columns (no `px` width) in proportion to their widths, equally
/// when they are all empty; only without auto columns over the `px` ones
/// (miaskiewicz/turbo-dom#55). Growing every column in proportion scales them by
/// `target / sum`.
fn grow_columns(cols: &mut [f32], fixed: &[bool], target: f32, sum: f32) {
    let idx = growing_columns(fixed);
    if idx.len() == cols.len() && sum > 0.0 {
        let k = target / sum;
        for c in cols.iter_mut() {
            *c *= k;
        }
        return;
    }
    let mut subset: Vec<f32> = idx.iter().map(|&i| cols[i]).collect();
    spread(&mut subset, target - sum);
    scatter(cols, &idx, &subset);
}

/// The columns a table's extra width goes to: the auto ones, else all.
fn growing_columns(fixed: &[bool]) -> Vec<usize> {
    let auto: Vec<usize> = (0..fixed.len()).filter(|&i| !fixed[i]).collect();
    if auto.is_empty() {
        (0..fixed.len()).collect()
    } else {
        auto
    }
}

/// Remove `reduce` px total, taking only from each column's slack above its
/// min-content so no column clips its content; a column never drops below `mins`.
fn shrink_columns(cols: &mut [f32], mins: &[f32], reduce: f32) {
    let slack: f32 = cols.iter().zip(mins).map(|(c, m)| (c - m).max(0.0)).sum();
    for (c, m) in cols.iter_mut().zip(mins) {
        if slack > 0.0 {
            *c -= reduce * (*c - *m).max(0.0) / slack;
        }
        *c = c.max(*m);
    }
}

fn explicit_table_width(style: &ComputedStyle, cw: f32) -> Option<f32> {
    let bs = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: cw,
        },
    );
    bs.width.resolve(cw)
}

fn column_widths(
    style: &ComputedStyle,
    placed: &[Placed],
    ncols: usize,
    cw: f32,
    fonts: &FontRegistry,
) -> Vec<f32> {
    // `cw` is the table's already-resolved content width (its box was sized from
    // the `width` declaration upstream), so a table with an explicit width fills
    // `cw`. Re-resolving the `width` value here would apply a percentage twice
    // (e.g. an 85%-wide table's columns collapsing to 85% of 85%).
    // `table-layout: fixed` applies to a table with a definite width; a `width: auto`
    // one uses the automatic algorithm (Chrome 156; miaskiewicz/turbo-dom#28 item 7).
    if is_fixed(style) && explicit_table_width(style, cw).is_some() {
        return fixed_columns(placed, ncols, cw.max(1.0));
    }
    // The columns fill the table box: an auto table is already shrink-to-fit (its
    // columns' max-content, wider under `min-width` or when stretched), an explicit
    // width is `cw` itself.
    let mut cols = auto_columns(placed, ncols, border_spacing(style).0, fonts);
    let limits = Limits {
        mins: min_columns(placed, ncols, fonts),
        fixed: px_columns(placed, ncols, explicit_width),
    };
    fit_columns(&mut cols, &limits, &column_percents(placed, ncols), cw);
    cols
}

// --------------------------------------------------------------------------
// collapsed-border grid (`border-collapse: collapse`)
// --------------------------------------------------------------------------
//
// In the collapse model a shared edge between two cells is a SINGLE border, drawn
// at the max width of the abutting borders and centred on the grid line, with half
// on each side (CSS 2.1 §17.6.2). So a cell's border box runs from the centre of
// its left grid line to the centre of its right one, and the table's width is the
// column content+padding tracks plus one border per grid line — NOT each cell's
// full borders summed (which double-counts every internal edge and overshoots,
// especially under a `colspan`). We model it explicitly: per grid-line border
// widths, content+padding column tracks, and grid-line centre positions.

/// A cell's resolved border widths (px). Cheap wrapper so callers read
/// `.left`/`.right`/`.top`/`.bottom` without re-plumbing a `ResolveCtx`.
fn cell_border(lb: &LayoutBox) -> Edges {
    lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    })
    .border
    .widths()
}

/// A cell's specified content+padding width (px) when it has a `width: <px>` — the
/// column's preferred track width in the collapse model (borders live on the grid
/// lines, not inside the track). `None` for an auto width.
fn explicit_cp(lb: &LayoutBox) -> Option<f32> {
    let bs = lb.resolved(ResolveCtx {
        parent_font_size: DEFAULT_FONT_SIZE,
        cb_width: 0.0,
    });
    // A border-box width includes the cell's collapsed half-borders (Chrome 156).
    let half = bs.border.widths().horizontal() / 2.0;
    match bs.width {
        LengthPct::Px(w) if bs.box_sizing == BoxSizing::BorderBox => {
            Some((w - half).max(bs.padding.horizontal()))
        }
        LengthPct::Px(w) => Some(w + bs.padding.horizontal()),
        _ => None,
    }
}

/// Collapsed border widths at the `ncols + 1` vertical grid lines. Line `j` is the
/// max of the table's left/right border (outer lines only) and every cell edging it
/// (its left border if it starts at `j`, its right if it ends at `j`). A `colspan`
/// cell contributes only at its two outer edges, so the line under it still takes
/// its width from cells in other rows that really border there.
fn vborders(placed: &[Placed], ncols: usize, style: &ComputedStyle) -> Vec<f32> {
    let table = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        },
    )
    .border
    .widths();
    let mut v = vec![0.0_f32; ncols + 1];
    v[0] = table.left;
    v[ncols] = v[ncols].max(table.right);
    for p in placed {
        let b = cell_border(p.lb);
        v[p.col] = v[p.col].max(b.left);
        v[p.col + p.colspan] = v[p.col + p.colspan].max(b.right);
    }
    v
}

/// Collapsed border widths at the `nrows + 1` horizontal grid lines (the row-axis
/// analogue of [`vborders`], keyed on `rowspan` and top/bottom borders).
fn hborders(placed: &[Placed], nrows: usize, style: &ComputedStyle) -> Vec<f32> {
    let table = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: DEFAULT_FONT_SIZE,
            cb_width: 0.0,
        },
    )
    .border
    .widths();
    let mut h = vec![0.0_f32; nrows + 1];
    h[0] = table.top;
    h[nrows] = h[nrows].max(table.bottom);
    for p in placed {
        let b = cell_border(p.lb);
        h[p.row] = h[p.row].max(b.top);
        h[p.row + p.rowspan] = h[p.row + p.rowspan].max(b.bottom);
    }
    h
}

/// Centre positions of the `n + 1` grid lines bounding `n` content+padding tracks,
/// where line `i` has collapsed border width `borders[i]`. Track `i` sits between
/// the full widths of lines `i` and `i+1`; a cell's border box runs centre-to-centre
/// so it owns half of each bounding border.
fn line_centers(tracks: &[f32], borders: &[f32]) -> Vec<f32> {
    let n = tracks.len();
    let mut out = Vec::with_capacity(n + 1);
    let mut pos = 0.0_f32;
    for i in 0..n {
        out.push(pos + borders[i] / 2.0);
        pos += borders[i] + tracks[i];
    }
    out.push(pos + borders[n] / 2.0);
    out
}

/// Per-column specified content+padding width from the FIRST row (fixed layout's
/// column authority), a `colspan` cell's width divided over the columns it covers.
/// `None` = auto (shares the leftover). Later rows never widen a column here — they
/// just flow into it (fixed layout, CSS 2.1 §17.5.2.1).
fn first_row_cp(placed: &[Placed], ncols: usize) -> Vec<Option<f32>> {
    let mut w = vec![None; ncols];
    for p in placed.iter().filter(|p| p.row == 0) {
        if let Some(total) = explicit_cp(p.lb) {
            let per = total / p.colspan as f32;
            for slot in w.iter_mut().skip(p.col).take(p.colspan) {
                slot.get_or_insert(per);
            }
        }
    }
    w
}

/// Content+padding widths per column for a fixed-layout collapsed table, given the
/// available content width (`table width − all grid-line borders`). Specified
/// columns keep their width and auto columns share the rest; when every column is
/// specified, the whole set scales proportionally to fill (or fit) `avail` — CSS
/// distributes the table's surplus/deficit over the columns.
fn fixed_collapse_columns(placed: &[Placed], ncols: usize, avail: f32) -> Vec<f32> {
    let spec = first_row_cp(placed, ncols);
    let specified: f32 = spec.iter().flatten().sum();
    let n_auto = spec.iter().filter(|x| x.is_none()).count();
    if n_auto > 0 {
        let each = share(avail - specified, n_auto);
        return spec.iter().map(|x| x.unwrap_or(each)).collect();
    }
    let k = if specified > 0.0 {
        avail / specified
    } else {
        0.0
    };
    spec.iter().map(|x| x.unwrap_or(0.0) * k).collect()
}

/// The fixed-collapse table's minimum content+padding per column: each first-row
/// cell's specified width, else its text min-content (colspan-distributed). Summed
/// with the grid-line borders this is the table's used-width floor — so a declared
/// width narrower than the columns grows (07), while one wider than them is honoured
/// (29's `colspan` first row asks for less than its `width:200px`).
fn fixed_min_cp(placed: &[Placed], ncols: usize, fonts: &FontRegistry) -> Vec<f32> {
    let mut w = vec![0.0_f32; ncols];
    let mut fixed = vec![false; ncols];
    for p in placed.iter().filter(|p| p.row == 0) {
        let explicit = explicit_cp(p.lb);
        let val = explicit.unwrap_or_else(|| {
            (super::flex::min_content_width(p.lb, fonts) - cell_border(p.lb).horizontal()).max(0.0)
        });
        let per = val / p.colspan as f32;
        for j in p.col..p.col + p.colspan {
            assign_fixed_col(&mut w, &mut fixed, j, per, explicit.is_some());
        }
    }
    w
}

/// Fold one first-row cell's per-column width `per` into column `j`: an explicit
/// width pins the column (first wins); an implicit width only raises a still-open
/// column's floor.
fn assign_fixed_col(w: &mut [f32], fixed: &mut [bool], j: usize, per: f32, explicit: bool) {
    if explicit && !fixed[j] {
        w[j] = per;
        fixed[j] = true;
    } else if !fixed[j] {
        w[j] = w[j].max(per);
    }
}

/// Content+padding widths per column of an auto-layout collapsed table, each cell
/// measured by `measure` (its natural or min-content border-box width) less its own
/// borders, which live on the grid lines; a spanning cell wider than its columns
/// (and the lines between them) widens them in proportion (evenly when empty).
fn collapse_columns(
    placed: &[Placed],
    vert: &[f32],
    fonts: &FontRegistry,
    measure: fn(&LayoutBox, &FontRegistry) -> f32,
) -> Vec<f32> {
    // A `px` width is the cell's content+padding (`explicit_cp`: a border-box one less
    // its collapsed half-borders, miaskiewicz/turbo-dom#65).
    let cp = |p: &Placed| {
        explicit_cp(p.lb)
            .unwrap_or_else(|| (measure(p.lb, fonts) - cell_border(p.lb).horizontal()).max(0.0))
    };
    let mut w = vec![0.0_f32; vert.len() - 1];
    for p in placed.iter().filter(|p| p.colspan == 1) {
        w[p.col] = w[p.col].max(cp(p));
    }
    for p in placed.iter().filter(|p| p.colspan > 1) {
        let span = p.col..p.col + p.colspan;
        let lines: f32 = vert[p.col + 1..p.col + p.colspan].iter().sum();
        let need = cp(p) - lines - w[span.clone()].iter().sum::<f32>();
        if need > 0.0 {
            spread(&mut w[span], need);
        }
    }
    w
}

/// Add `extra` to `cols` in proportion to their widths (evenly when all are zero).
fn spread(cols: &mut [f32], extra: f32) {
    let sum: f32 = cols.iter().sum();
    let n = cols.len() as f32;
    for c in cols.iter_mut() {
        *c += if sum > 0.0 {
            extra * *c / sum
        } else {
            extra / n
        };
    }
}

/// A collapsed table's content width from its column tracks and grid lines: their sum
/// less the table's own left/right border (the box frame adds it back).
fn collapsed_width(tracks: &[f32], vert: &[f32], style: &ComputedStyle) -> f32 {
    tracks.iter().sum::<f32>() + vert.iter().sum::<f32>() - table_border(style).horizontal()
}

/// Lay out the cells of a collapsed-border table into the grid defined by the
/// vertical grid-line centres `line_x` and their border widths `vert`. Each cell's
/// content is laid at the width its content region truly gets — the collapsed
/// border box minus the half-borders it shares, plus the cell's own full border
/// (which the block layout subtracts back out) — then its reported box width is
/// overwritten to the collapsed border box (centre-to-centre).
fn layout_cells_collapsed<'a>(
    placed: &'a [Placed],
    line_x: &[f32],
    vert: &[f32],
    fs: f32,
    ctx: &mut Ctx,
) -> Vec<LaidCell<'a>> {
    placed
        .iter()
        .map(|p| {
            let extent = line_x[p.col + p.colspan] - line_x[p.col];
            let half = vert[p.col] / 2.0 + vert[p.col + p.colspan] / 2.0;
            let bs = p.lb.resolved(ResolveCtx {
                parent_font_size: fs,
                cb_width: extent,
            });
            let own = bs.border.widths().horizontal();
            let layout_w = (extent - half + own).max(0.0);
            let mut frag = block::layout_box_sized_isolated(p.lb, &bs, 0.0, 0.0, layout_w, ctx);
            let content_h = frag.height;
            frag.width = extent;
            LaidCell {
                p,
                frag,
                valign: bs.vertical_align,
                content_h,
            }
        })
        .collect()
}

/// Content+padding height per row for a collapsed table: the tallest cell's own
/// content+padding (its laid border box minus its vertical borders), floored by the
/// row's explicit `height`. A `rowspan` cell that overflows its rows grows the last.
fn collapse_row_heights(laid: &[LaidCell], rows: &[RowRef]) -> Vec<f32> {
    let cp = |c: &LaidCell| (c.frag.height - cell_border(c.p.lb).vertical()).max(0.0);
    let mut h: Vec<f32> = rows.iter().map(|r| r.min_height).collect();
    for c in laid.iter().filter(|c| c.p.rowspan == 1) {
        h[c.p.row] = h[c.p.row].max(cp(c));
    }
    for c in laid.iter().filter(|c| c.p.rowspan > 1) {
        let (start, span) = (c.p.row, c.p.rowspan);
        let have: f32 = h[start..start + span].iter().sum();
        let need = cp(c);
        if need > have {
            h[start + span - 1] += need - have;
        }
    }
    h
}

/// A placed table grid: the collected rows and their cells positioned on the
/// `ncols`-wide occupancy grid (colspan/rowspan resolved). Bundled so the collapse
/// layout doesn't thread three coupled values through its signature.
struct Grid<'a> {
    rows: &'a [RowRef<'a>],
    placed: &'a [Placed<'a>],
    ncols: usize,
    groups: &'a [GroupRef],
    /// The row sections ([`sections`]) and the table's definite content height the rows
    /// share ([`distribute_extra`]).
    sections: &'a [Section],
    fill: f32,
}

/// Lay out a fixed-layout `border-collapse: collapse` table (§17.6.2): merge shared
/// cell edges so the table/row/cell boxes are sized against the collapsed grid, not
/// each cell's full borders. Returns the row fragments and the table content height.
fn layout_table_collapsed(
    table: &LayoutBox,
    grid: &Grid,
    cx: f32,
    cy: f32,
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    let &Grid {
        rows,
        placed,
        ncols,
        ..
    } = grid;
    let nrows = rows.len();
    // The grid spans the table's border box: its own borders are the outer lines.
    let own = table_border(&table.style);
    let (cx, cy) = (cx - own.left, cy - own.top);
    let vert = vborders(placed, ncols, &table.style);
    let avail = cw + own.horizontal() - vert.iter().sum::<f32>();
    let col_cp = if fixed_layout(&table.style) {
        fixed_collapse_columns(placed, ncols, avail.max(1.0))
    } else {
        let mut cols = collapse_columns(placed, &vert, ctx.fonts, natural_width);
        let limits = Limits {
            mins: collapse_columns(placed, &vert, ctx.fonts, super::flex::min_content_width),
            fixed: px_columns(placed, ncols, explicit_cp),
        };
        fit_columns(
            &mut cols,
            &limits,
            &column_percents(placed, ncols),
            avail.max(0.0),
        );
        cols
    };
    let line_x = line_centers(&col_cp, &vert);
    let laid = layout_cells_collapsed(placed, &line_x, &vert, fs, ctx);
    let horiz = hborders(placed, nrows, &table.style);
    let mut row_cp = collapse_row_heights(&laid, rows);
    // Each row's extent between its grid-line centres weighs its share of the extra.
    let weights: Vec<f32> = (0..nrows)
        .map(|r| row_cp[r] + (horiz[r] + horiz[r + 1]) / 2.0)
        .collect();
    let extra =
        grid.fill - (row_cp.iter().sum::<f32>() + horiz.iter().sum::<f32>() - own.vertical());
    distribute_extra(&mut row_cp, &weights, (rows, grid.sections), extra);
    let line_y = line_centers(&row_cp, &horiz);
    let geom = Geom {
        col_x: line_x[..ncols].to_vec(),
        row_y: line_y[..nrows].to_vec(),
        row_h: line_y.windows(2).map(|w| w[1] - w[0]).collect(),
        row_x: line_x[0],
        row_w: line_x[ncols] - line_x[0],
    };
    let height = row_cp.iter().sum::<f32>() + horiz.iter().sum::<f32>() - own.vertical();
    record_rows(rows, ctx);
    let frags = finalize(rows, laid, &geom, cx, cy);
    record_row_metrics(rows, &frags, ctx);
    let frame = GroupFrame::of(&geom, cx, cy + geom.row_y[0]);
    (wrap_row_groups(frags, grid.groups, frame, ctx), height)
}

// --------------------------------------------------------------------------
// cell layout, row heights, placement
// --------------------------------------------------------------------------

struct LaidCell<'a> {
    p: &'a Placed<'a>,
    frag: Fragment,
    valign: VAlign,
    content_h: f32,
}

fn layout_one<'a>(
    p: &'a Placed<'a>,
    cols: &[f32],
    hs: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> LaidCell<'a> {
    let w = cell_span_width(cols, p.col, p.colspan) + hs * (p.colspan - 1) as f32;
    let bs = p.lb.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: w,
    });
    let frag = block::layout_box_sized_isolated(p.lb, &bs, 0.0, 0.0, w, ctx);
    let content_h = frag.height;
    LaidCell {
        p,
        frag,
        valign: bs.vertical_align,
        content_h,
    }
}

/// Lay out every cell at its columns' width (a spanning cell also over the
/// border-spacing `hs` between them; miaskiewicz/turbo-dom#56).
fn layout_cells<'a>(
    placed: &'a [Placed],
    cols: &[f32],
    hs: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> Vec<LaidCell<'a>> {
    placed
        .iter()
        .map(|p| layout_one(p, cols, hs, fs, ctx))
        .collect()
}

fn expand_rows(h: &mut [f32], c: &LaidCell) {
    let (start, span) = (c.p.row, c.p.rowspan);
    let have: f32 = h[start..start + span].iter().sum();
    if c.content_h > have {
        h[start + span - 1] += c.content_h - have;
    }
}

fn row_heights(laid: &[LaidCell], rows: &[RowRef]) -> Vec<f32> {
    // Seed each row at its explicit `height` floor (empty spacer rows), then grow
    // to fit content.
    let mut h: Vec<f32> = rows.iter().map(|r| r.min_height).collect();
    for c in laid {
        if c.p.rowspan == 1 {
            h[c.p.row] = h[c.p.row].max(c.content_h);
        }
    }
    for c in laid {
        if c.p.rowspan > 1 {
            expand_rows(&mut h, c);
        }
    }
    h
}

struct Geom {
    col_x: Vec<f32>,
    row_y: Vec<f32>,
    row_h: Vec<f32>,
    /// A row box's left edge (from the table content box) and width: from the first
    /// column's left edge to the last column's right edge — inside the border-spacing
    /// (separated model) or between the outer grid-line centres (collapsed), as Chrome
    /// lays a row out over its cells.
    row_x: f32,
    row_w: f32,
}

fn valign_offset(v: VAlign, cell_h: f32, content_h: f32) -> f32 {
    let slack = (cell_h - content_h).max(0.0);
    match v {
        VAlign::Middle | VAlign::Baseline => slack / 2.0,
        VAlign::Bottom => slack,
        _ => 0.0,
    }
}

fn spanned(row_h: &[f32], start: usize, span: usize) -> f32 {
    row_h[start..start + span].iter().sum()
}

fn position_cell(c: LaidCell, geom: &Geom, cx: f32, cy: f32) -> Fragment {
    let cell_h = spanned(&geom.row_h, c.p.row, c.p.rowspan);
    let dy = valign_offset(c.valign, cell_h, c.content_h);
    let mut frag = c.frag;
    frag.translate(cx + geom.col_x[c.p.col], cy + geom.row_y[c.p.row]);
    for child in &mut frag.children {
        child.translate(0.0, dy);
    }
    frag.height = cell_h;
    frag
}

fn make_row_frag(row: &RowRef, r: usize, geom: &Geom, cx: f32, cy: f32) -> Fragment {
    let content = FragmentContent::Box {
        background: None,
        border: BorderEdges::default(),
        border_radius: 0.0,
        shadow: None,
        gradient: None,
        transform: None,
    };
    let mut f = Fragment::new(
        row.node_id,
        cx + geom.row_x,
        cy + geom.row_y[r],
        geom.row_w,
        geom.row_h[r],
        content,
    );
    f.break_meta.repeatable = row.repeat;
    f.break_meta.break_inside_avoid = row.avoid_break_inside;
    f.break_meta.break_before = row.break_before;
    f.break_meta.break_after = row.break_after;
    f
}

/// In source-recording mode, note which boxes were laid out as table rows (their
/// one fragment is the synthetic row box above, not an ordinary box fragment).
fn record_rows(rows: &[RowRef], ctx: &mut Ctx) {
    if let Some(recorded) = &mut ctx.table_rows {
        recorded.extend(rows.iter().map(|row| row.node_id));
    }
}

/// In live geometry, each row box's used metrics: no border, padding or margin of
/// its own (borders belong to the cells), the box itself as the content box
/// (miaskiewicz/turbo-surf#31).
fn record_row_metrics(rows: &[RowRef], frags: &[Fragment], ctx: &mut Ctx) {
    let Some(metrics) = &mut ctx.used_metrics else {
        return;
    };
    for (row, frag) in rows.iter().zip(frags) {
        metrics.insert(
            row.node_id,
            empty_box_metrics(frag, row.position, Display::TableRow),
        );
    }
}

/// The used metrics of a frameless table-internal box (a row or a row group) laid
/// out as `frag`.
fn empty_box_metrics(frag: &Fragment, position: Position, display: Display) -> UsedBoxMetrics {
    UsedBoxMetrics {
        inline_fragment: false,
        border: Edges::default(),
        padding: Edges::default(),
        margin: Edges::default(),
        content_width: frag.width,
        content_height: frag.height,
        position,
        display,
    }
}

/// Where the rows of a table sit (galley-absolute): their left edge and width, and the
/// block offset an empty row group takes when no row precedes it.
#[derive(Clone, Copy)]
struct GroupFrame {
    x: f32,
    width: f32,
    top: f32,
}

impl GroupFrame {
    fn of(geom: &Geom, cx: f32, top: f32) -> Self {
        Self {
            x: cx + geom.row_x,
            width: geom.row_w,
            top,
        }
    }
}

/// In live geometry, nest each row group's rows under a transparent box for the group:
/// the union of its rows, with frameless used metrics (miaskiewicz/turbo-surf#31), so
/// a `<tbody>` answers its CSSOM reads. An empty row group is a zero-height box as
/// wide as the rows, at the bottom of the rows before it — the table's first row edge
/// when there are none (miaskiewicz/turbo-dom#28 item 5, Chrome 156). Other layouts keep
/// the rows flat (the PDF fragmenter repeats `<thead>`/`<tfoot>` rows from the table's
/// own children).
fn wrap_row_groups(
    frags: Vec<Fragment>,
    groups: &[GroupRef],
    frame: GroupFrame,
    ctx: &mut Ctx,
) -> Vec<Fragment> {
    let Some(metrics) = &mut ctx.used_metrics else {
        return frags;
    };
    let mut bottom = frame.top;
    let mut rows: Vec<Option<Fragment>> = frags.into_iter().map(Some).collect();
    let mut out = Vec::new();
    let mut next = 0;
    for group in groups {
        out.extend(rows[next..group.start].iter_mut().filter_map(Option::take));
        if let Some(row) = out.last() {
            bottom = row.y + row.height;
        }
        next = group.start + group.count;
        let children: Vec<Fragment> = rows[group.start..next]
            .iter_mut()
            .filter_map(Option::take)
            .collect();
        let frag = if children.is_empty() {
            let mut frag = group_fragment(group.node_id, Vec::new());
            (frag.x, frag.y, frag.width) = (frame.x, bottom, frame.width);
            frag
        } else {
            let frag = group_fragment(group.node_id, children);
            bottom = frag.y + frag.height;
            frag
        };
        metrics.insert(
            group.node_id,
            empty_box_metrics(&frag, group.position, group.display),
        );
        out.push(frag);
    }
    out.extend(rows[next..].iter_mut().filter_map(Option::take));
    out
}

/// A transparent box over `rows` (their union; zero-size at the origin for none).
fn group_fragment(node_id: NodeId, rows: Vec<Fragment>) -> Fragment {
    let (x, y, width, height) = match (rows.first(), rows.last()) {
        (Some(first), Some(last)) => {
            let x = rows.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
            let right = rows
                .iter()
                .map(|r| r.x + r.width)
                .fold(f32::NEG_INFINITY, f32::max);
            (x, first.y, right - x, last.y + last.height - first.y)
        }
        _ => (0.0, 0.0, 0.0, 0.0),
    };
    let content = FragmentContent::Box {
        background: None,
        border: BorderEdges::default(),
        border_radius: 0.0,
        shadow: None,
        gradient: None,
        transform: None,
    };
    let mut frag = Fragment::new(node_id, x, y, width, height, content);
    frag.children = rows;
    frag
}

fn finalize(rows: &[RowRef], laid: Vec<LaidCell>, geom: &Geom, cx: f32, cy: f32) -> Vec<Fragment> {
    let mut row_frags: Vec<Fragment> = rows
        .iter()
        .enumerate()
        .map(|(r, row)| make_row_frag(row, r, geom, cx, cy))
        .collect();
    for c in laid {
        let r = c.p.row;
        row_frags[r].children.push(position_cell(c, geom, cx, cy));
    }
    row_frags
}

/// Lay out a table's rows into the content box at `(cx, cy)` of width `cw`.
/// The table's min-content width: the sum of its columns' natural widths (each the
/// widest cell in the column, colspans distributed). A table never shrinks below
/// this — an explicit `width` narrower than the content is overridden (CSS auto
/// table layout), so a fixed-width infobox still grows to fit a wide image row
/// instead of overflowing it.
/// `(horizontal, vertical)` `border-spacing` in px — the gap around/between cells
/// in the default `separate` model (`border-collapse:collapse` has none: every caller
/// takes the collapsed model's own path before it reads this). The property
/// is inherited and initially 0: a `<table>` has `2px` from the UA `table` rule
/// (miaskiewicz/turbo-dom#63). Without it a table's cells butt together (the infobox
/// label ran straight into its value: "`SpeciesF`. catus").
fn border_spacing(style: &ComputedStyle) -> (f32, f32) {
    let mut it = style
        .get("border-spacing")
        .map(str::split_whitespace)
        .into_iter()
        .flatten();
    let h = it.next().and_then(|t| parse_px(t, DEFAULT_FONT_SIZE));
    let v = it.next().and_then(|t| parse_px(t, DEFAULT_FONT_SIZE));
    (h.unwrap_or(0.0), v.or(h).unwrap_or(0.0))
}

/// Prefix offsets for `n` tracks separated (and bracketed) by `gap`: track `i`
/// starts at `gap*(i+1) + sum(tracks[..i])`.
fn spaced_offsets(tracks: &[f32], gap: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(tracks.len());
    let mut acc = gap;
    for t in tracks {
        out.push(acc);
        acc += t + gap;
    }
    out
}

// --------------------------------------------------------------------------
// the table wrapper box (captions)
// --------------------------------------------------------------------------

/// The anonymous grid box among a table's children: `Some` for a table wrapper box.
fn grid_of(items: &[LayoutBox]) -> Option<&LayoutBox> {
    items.iter().find(|item| item.table_grid())
}

/// The rows and row groups of a table: its children, or its grid box's when the table is
/// a table wrapper box.
fn grid_items(items: &[LayoutBox]) -> &[LayoutBox] {
    match grid_of(items).map(|grid| &grid.kind) {
        Some(BoxKind::Table(rows)) => rows,
        _ => items,
    }
}

/// Whether a table's box is a table wrapper box (it has captions).
pub(crate) fn is_wrapper(lb: &LayoutBox) -> bool {
    matches!(&lb.kind, BoxKind::Table(items) if grid_of(items).is_some())
}

/// The widest caption's min-content margin box: the narrowest a table wrapper box can be
/// (Chrome 156). `None` without captions.
pub(crate) fn caption_min(items: &[LayoutBox], fonts: &FontRegistry) -> Option<f32> {
    grid_of(items)?;
    Some(
        items
            .iter()
            .filter(|item| !item.table_grid())
            .map(|caption| {
                let bs = caption.resolved(ResolveCtx {
                    parent_font_size: DEFAULT_FONT_SIZE,
                    cb_width: 0.0,
                });
                super::flex::min_content_width(caption, fonts) + bs.margin.horizontal()
            })
            .fold(0.0_f32, f32::max),
    )
}

/// [`caption_min`] as a content width of the table box whose frame is `style`'s, or
/// `0` without captions.
fn caption_floor(items: &[LayoutBox], style: &ComputedStyle, fonts: &FontRegistry) -> f32 {
    caption_min(items, fonts).map_or(0.0, |min| {
        let frame = resolve_box_style(
            style,
            ResolveCtx {
                parent_font_size: DEFAULT_FONT_SIZE,
                cb_width: 0.0,
            },
        );
        min - frame.padding.horizontal() - frame.border.widths().horizontal()
    })
}

/// The style `lb` lays out with: `bs`, or for a table wrapper box [`wrapper_style`].
pub(crate) fn own_style<'a>(lb: &LayoutBox, bs: &'a BoxStyle) -> std::borrow::Cow<'a, BoxStyle> {
    if is_wrapper(lb) {
        std::borrow::Cow::Owned(wrapper_style(bs))
    } else {
        std::borrow::Cow::Borrowed(bs)
    }
}

/// The used style of a table wrapper box: the table's, without the frame, background,
/// shadow and heights that belong to its grid box (they paint and size the grid).
fn wrapper_style(bs: &BoxStyle) -> BoxStyle {
    BoxStyle {
        padding: Edges::default(),
        border: BorderEdges::default(),
        height: LengthPct::Auto,
        min_height: LengthPct::Px(0.0),
        max_height: LengthPct::Auto,
        aspect_ratio: None,
        border_radius: LengthPct::Px(0.0),
        background: None,
        box_shadow: None,
        background_gradient: None,
        ..bs.clone()
    }
}

/// Whether a caption sits below the grid (`caption-side: bottom`).
fn below(caption: &LayoutBox) -> bool {
    caption.style.get("caption-side").map(str::trim) == Some("bottom")
}

/// Lay out a table wrapper box's content at `(cx, cy)`, `cw` wide (its border box: the
/// wrapper has no frame): the `caption-side: top` captions in source order, the grid box
/// stretched to `cw`, then the `bottom` captions. Each caption is a block formatting
/// context at the wrapper's width, its margins inside the wrapper (Chrome 156). Returns
/// the fragments and the content height.
pub(crate) fn layout_wrapper(
    items: &[LayoutBox],
    cx: f32,
    cy: f32,
    cw: f32,
    fs: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    let mut out = Vec::with_capacity(items.len());
    let mut y = cy;
    let captions = || items.iter().filter(|item| !item.table_grid());
    for caption in captions().filter(|c| !below(c)) {
        y = place_caption(caption, (cx, y, cw, fs), ctx, &mut out);
    }
    if let Some(grid) = grid_of(items) {
        let bs = grid.resolved(ResolveCtx {
            parent_font_size: fs,
            cb_width: cw,
        });
        let frag = block::layout_box_sized(grid, &bs, cx, y, cw, ctx);
        y += frag.height;
        out.push(frag);
    }
    for caption in captions().filter(|c| below(c)) {
        y = place_caption(caption, (cx, y, cw, fs), ctx, &mut out);
    }
    (out, y - cy)
}

/// Lay out one caption at the top of `y` (`at` = `(cx, y, cw, parent font size)`);
/// returns the y below its bottom margin.
fn place_caption(
    caption: &LayoutBox,
    (cx, y, cw, fs): (f32, f32, f32, f32),
    ctx: &mut Ctx,
    out: &mut Vec<Fragment>,
) -> f32 {
    let bs = caption.resolved(ResolveCtx {
        parent_font_size: fs,
        cb_width: cw,
    });
    let width = block::border_box_width(&bs, cw);
    let frag = block::layout_box_sized_isolated(
        caption,
        &bs,
        cx + bs.margin.left,
        y + bs.margin.top,
        width,
        ctx,
    );
    let next = y + bs.margin.vertical() + frag.height;
    out.push(frag);
    next
}

pub(crate) fn min_content_width(
    items: &[LayoutBox],
    style: &ComputedStyle,
    fonts: &FontRegistry,
) -> f32 {
    let grid = grid_min_content(grid_items(items), style, fonts);
    if grid_of(items).is_some() {
        return grid.max(caption_floor(items, style, fonts));
    }
    grid
}

/// The grid's min-content (see [`min_content_width`]).
fn grid_min_content(items: &[LayoutBox], style: &ComputedStyle, fonts: &FontRegistry) -> f32 {
    let rows = collect_rows(items);
    let (placed, ncols) = build_grid(&rows);
    if ncols == 0 {
        return 0.0;
    }
    // Collapsed table: floor = its columns' content+padding minima (a fixed one's
    // first row) plus one border per grid line — the used-width basis, not each cell's
    // full borders — less the table's own border, which its box frame adds back.
    if collapsed(style) {
        let vert = vborders(&placed, ncols, style);
        let cp = if fixed_layout(style) {
            fixed_min_cp(&placed, ncols, fonts)
        } else {
            collapse_columns(&placed, &vert, fonts, super::flex::min_content_width)
        };
        return collapsed_width(&cp, &vert, style);
    }
    let (hs, _) = border_spacing(style);
    separated_min_content(&placed, ncols, hs, fonts)
}

/// The table's max-content width: its columns' natural widths (colspans
/// distributed) plus the border-spacing around and between them; a fixed-layout
/// collapsed table's is its min-content (first-row columns + grid-line borders).
pub(crate) fn max_content_width(
    items: &[LayoutBox],
    style: &ComputedStyle,
    fonts: &FontRegistry,
) -> f32 {
    max_content(items, style, fonts).0
}

/// [`max_content_width`] and whether the table's max-content is unbounded (a
/// percentage column of 100% or more beside other content; see
/// [`percent_aware_total`]): a `width: auto` table then fills its line.
pub(crate) fn max_content(
    items: &[LayoutBox],
    style: &ComputedStyle,
    fonts: &FontRegistry,
) -> (f32, bool) {
    let (grid, unbounded) = grid_max_content(grid_items(items), style, fonts);
    if grid_of(items).is_some() {
        // A caption widens the table to its min-content only (Chrome 156).
        return (grid.max(caption_floor(items, style, fonts)), unbounded);
    }
    (grid, unbounded)
}

/// The grid's [`max_content`].
fn grid_max_content(
    items: &[LayoutBox],
    style: &ComputedStyle,
    fonts: &FontRegistry,
) -> (f32, bool) {
    let rows = collect_rows(items);
    let (placed, ncols) = build_grid(&rows);
    if ncols == 0 || fixed_layout(style) && collapsed(style) {
        return (grid_min_content(items, style, fonts), false);
    }
    let pcts = column_percents(&placed, ncols);
    if collapsed(style) {
        let vert = vborders(&placed, ncols, style);
        let cp = collapse_columns(&placed, &vert, fonts, natural_width);
        let (total, unbounded) = percent_aware_total(&cp, &pcts);
        return (collapsed_width(&[total], &vert, style), unbounded);
    }
    let (hs, _) = border_spacing(style);
    let cols = auto_columns(&placed, ncols, hs, fonts);
    let (total, unbounded) = percent_aware_total(&cols, &pcts);
    (total + hs * (ncols as f32 + 1.0), unbounded)
}

/// Separated-border table min-content. Each column's min-content is the widest
/// single-column cell (text wraps to its longest word; a fixed-width cell keeps its
/// size); a spanning cell wider than its columns and the spacing between them (e.g.
/// the infobox's 267px cat-image row that spans both columns) widens them in
/// proportion, as for the max-content (miaskiewicz/turbo-dom#56). Plus the
/// border-spacing between and around the columns.
fn separated_min_content(placed: &[Placed], ncols: usize, hs: f32, fonts: &FontRegistry) -> f32 {
    let mut w = vec![0.0_f32; ncols];
    for p in placed.iter().filter(|p| p.colspan == 1) {
        w[p.col] = w[p.col].max(super::flex::min_content_width(p.lb, fonts));
    }
    for p in placed.iter().filter(|p| p.colspan > 1) {
        let have = cell_span_width(&w, p.col, p.colspan) + hs * (p.colspan - 1) as f32;
        let need = super::flex::min_content_width(p.lb, fonts) - have;
        if need > 0.0 {
            spread(&mut w[p.col..p.col + p.colspan], need);
        }
    }
    w.iter().sum::<f32>() + hs * (ncols as f32 + 1.0)
}

/// Lay out a table's rows into its content box at `(cx, cy)`, `cw` wide; rows taller
/// than their content share `fill`, the table's definite content height (0: none).
/// Returns the row fragments (galley-absolute) and the table content height.
pub(crate) fn layout_table(
    table: &LayoutBox,
    items: &[LayoutBox],
    (cx, cy, cw, fill): (f32, f32, f32, f32),
    fs: f32,
    ctx: &mut Ctx,
) -> (Vec<Fragment>, f32) {
    let rows = collect_rows(items);
    let (placed, ncols) = build_grid(&rows);
    if ncols == 0 {
        // No columns: (empty) row groups sit at the content-box origin, zero-size.
        let frame = GroupFrame {
            x: cx,
            width: 0.0,
            top: cy,
        };
        return (
            wrap_row_groups(Vec::new(), &row_groups(items), frame, ctx),
            0.0,
        );
    }
    // Collapsed borders need the merged-grid geometry (shared edges counted once;
    // miaskiewicz/turbo-dom#49 for auto layout). Separated tables keep the
    // track+spacing model below.
    if collapsed(&table.style) {
        let grid = Grid {
            rows: &rows,
            placed: &placed,
            ncols,
            groups: &row_groups(items),
            sections: &sections(items),
            fill,
        };
        return layout_table_collapsed(table, &grid, cx, cy, cw, fs, ctx);
    }
    let (hs, vs) = border_spacing(&table.style);
    // Reserve the horizontal spacing (gaps + edges) out of the content width so the
    // columns + gaps still fit the table box.
    let inner = (cw - hs * (ncols as f32 + 1.0)).max(0.0);
    let cols = column_widths(&table.style, &placed, ncols, inner, ctx.fonts);
    let laid = layout_cells(&placed, &cols, hs, fs, ctx);
    let mut row_h = row_heights(&laid, &rows);
    let nrows = row_h.len();
    let spacing = vs * (nrows as f32 + 1.0);
    let weights = row_h.clone();
    let extra = fill - row_h.iter().sum::<f32>() - spacing;
    distribute_extra(&mut row_h, &weights, (&rows, &sections(items)), extra);
    let geom = Geom {
        col_x: spaced_offsets(&cols, hs),
        row_y: spaced_offsets(&row_h, vs),
        row_x: hs,
        row_w: cols.iter().sum::<f32>() + hs * (ncols as f32 - 1.0),
        row_h,
    };
    let height = geom.row_h.iter().sum::<f32>() + spacing;
    record_rows(&rows, ctx);
    let frags = finalize(&rows, laid, &geom, cx, cy);
    record_row_metrics(&rows, &frags, ctx);
    let frame = GroupFrame::of(&geom, cx, cy);
    (
        wrap_row_groups(frags, &row_groups(items), frame, ctx),
        height,
    )
}

#[cfg(test)]
mod percent_tests {
    use super::{fit_columns, grow_columns, percent_aware_total, Limits};

    #[test]
    fn percent_aware_total_matches_chromes_auto_table_max_content() {
        assert_eq!(
            percent_aware_total(&[32.0, 22.0], &[None, None]),
            (54.0, false)
        );
        // 50%: the 32px column must be half of the table.
        assert_eq!(
            percent_aware_total(&[32.0, 22.0], &[Some(50.0), None]),
            (64.0, false)
        );
        // Two 25% columns: 32 / 0.25 = 128 wins over 22 / 0.5 = 44.
        let two = percent_aware_total(&[32.0, 22.0, 22.0], &[Some(25.0), Some(25.0), None]);
        assert_eq!(two, (128.0, false));
        // 100% beside other content is unbounded; alone it is not.
        assert!(percent_aware_total(&[32.0, 22.0], &[Some(100.0), None]).1);
        assert!(!percent_aware_total(&[32.0, 22.0], &[Some(60.0), Some(40.0)]).1);
        assert!(!percent_aware_total(&[32.0, 0.0], &[Some(100.0), None]).1);
    }

    #[test]
    fn fit_columns_gives_percentage_columns_their_share() {
        let fit = |cols: &[f32], mins: &[f32], pcts: &[Option<f32>], target| {
            let mut cols = cols.to_vec();
            let limits = Limits {
                mins: mins.to_vec(),
                fixed: vec![false; mins.len()],
            };
            fit_columns(&mut cols, &limits, pcts, target);
            cols
        };
        // No percentages: plain proportional growth.
        assert_eq!(
            fit(&[10.0, 30.0], &[10.0, 30.0], &[None, None], 80.0),
            [20.0, 60.0]
        );
        // 25% + 25% of 128, the rest to the auto column.
        let quarters = fit(
            &[32.0, 22.0, 22.0],
            &[32.0, 22.0, 22.0],
            &[Some(25.0), Some(25.0), None],
            128.0,
        );
        assert_eq!(quarters, [32.0, 32.0, 64.0]);
        // 100%: the other column keeps its min-content.
        assert_eq!(
            fit(&[32.0, 22.0], &[32.0, 22.0], &[Some(100.0), None], 1274.0),
            [1252.0, 22.0]
        );
        // Shares over 100% scale down; percentage-only tables fill the target.
        assert_eq!(
            fit(
                &[10.0, 10.0],
                &[10.0, 10.0],
                &[Some(100.0), Some(100.0)],
                100.0
            ),
            [50.0, 50.0]
        );
        assert_eq!(
            fit(
                &[10.0, 10.0],
                &[10.0, 10.0],
                &[Some(20.0), Some(30.0)],
                200.0
            ),
            [80.0, 120.0]
        );
        // A share below the column's min-content keeps the min-content.
        assert_eq!(
            fit(&[50.0, 10.0], &[50.0, 10.0], &[Some(10.0), None], 100.0),
            [50.0, 50.0]
        );
    }

    #[test]
    fn grow_columns_prefers_auto_columns_like_chrome() {
        let grow = |cols: &[f32], fixed: &[bool], target| {
            let mut cols = cols.to_vec();
            let sum = cols.iter().sum();
            grow_columns(&mut cols, fixed, target, sum);
            cols
        };
        // Every column auto (or every one fixed): scaled in proportion.
        assert_eq!(grow(&[10.0, 30.0], &[false, false], 80.0), [20.0, 60.0]);
        assert_eq!(grow(&[10.0, 30.0], &[true, true], 80.0), [20.0, 60.0]);
        // Empty auto columns share equally; a px column keeps its width.
        assert_eq!(grow(&[0.0, 0.0], &[false, false], 100.0), [50.0, 50.0]);
        assert_eq!(grow(&[100.0, 0.0], &[true, false], 400.0), [100.0, 300.0]);
        assert_eq!(
            grow(&[100.0, 30.0, 0.0], &[true, false, false], 400.0),
            [100.0, 300.0, 0.0]
        );
        assert_eq!(grow(&[0.0, 0.0], &[true, true], 10.0), [5.0, 5.0]);
    }
}

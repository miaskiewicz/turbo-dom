//! Physical block-axis fill for quirks-mode HTML documents (WHATWG Quirks 3.6/3.7).
use super::{
    empty_quirky_flow, leading_margin, native_height_basis, quirks_line_height,
    resolved_content_height, trailing_margin_walk, BoxStyle, Ctx, Float, LayoutBox, LengthPct,
};

#[derive(Clone, Copy)]
pub(crate) struct Parent {
    pub(crate) writing: super::super::writing::WritingMode,
    content: f32,
    multicol: bool,
    percentage: Option<f32>,
}

fn frame(bs: &BoxStyle) -> f32 {
    bs.padding.vertical() + bs.border.widths().vertical()
}

fn root_floor(lb: &LayoutBox, bs: &BoxStyle, ctx: &Ctx) -> Option<f32> {
    if !lb.document_root() || !quirks_line_height(lb) || bs.height != LengthPct::Auto {
        return None;
    }
    viewport_block(ctx).map(|viewport| (viewport - bs.margin.vertical() - frame(bs)).max(0.0))
}

fn viewport_block(ctx: &Ctx) -> Option<f32> {
    if ctx.writing.vertical() {
        Some(ctx.root_w)
    } else if ctx.live_geometry {
        Some(ctx.root_h)
    } else {
        (ctx.icb_h > 0.0).then_some(ctx.icb_h)
    }
}

pub(super) fn child_basis(lb: &LayoutBox, bs: &BoxStyle, ctx: &Ctx) -> Option<Parent> {
    if !lb.document_root() || !quirks_line_height(lb) {
        return None;
    }
    Some(Parent {
        writing: ctx.writing,
        content: resolved_content_height(bs, root_floor(lb, bs, ctx).unwrap_or(0.0), ctx),
        multicol: super::super::multicol::is_multicol_container(&lb.style),
        percentage: (bs.height == LengthPct::Auto)
            .then(|| viewport_block(ctx))
            .flatten(),
    })
}

fn eligible_body(lb: &LayoutBox, bs: &BoxStyle, parent: Parent) -> bool {
    let inline = matches!(
        lb.display,
        super::super::value::Display::Inline
            | super::super::value::Display::InlineBlock
            | super::super::value::Display::InlineFlex
            | super::super::value::Display::InlineGrid
            | super::super::value::Display::InlineTable
    );
    let spanning = parent.multicol
        && lb
            .style
            .get("column-span")
            .is_some_and(|s| s.trim().eq_ignore_ascii_case("all"));
    lb.style.get(crate::style::QUIRKS_DOCUMENT_BODY).is_some()
        && !bs.position.is_out_of_flow()
        && bs.float == Float::None
        && !inline
        && !spanning
}

fn body_floor(lb: &LayoutBox, bs: &BoxStyle, cw: f32, ctx: &Ctx) -> Option<f32> {
    let parent = ctx.quirks_parent?;
    if !eligible_body(lb, bs, parent) {
        return None;
    }
    // An empty child's adjoining margins already escape through the leading
    // edge; do not subtract that same run twice from the body's fill basis.
    let end = if empty_quirky_flow(lb, bs, cw) {
        bs.margin.bottom
    } else {
        trailing_margin_walk(lb, bs, cw, &ctx.dropped_end_margins)
    };
    let margins = leading_margin(lb, bs, cw, bs.font_size) + end;
    Some((parent.content - margins - frame(bs)).max(0.0))
}

pub(super) fn auto_height(lb: &LayoutBox, bs: &BoxStyle, cw: f32, natural: f32, ctx: &Ctx) -> f32 {
    if bs.height != LengthPct::Auto {
        return natural;
    }
    root_floor(lb, bs, ctx)
        .or_else(|| body_floor(lb, bs, cw, ctx))
        .map_or(natural, |floor| natural.max(floor))
}

/// A percentage body skips its auto-height HTML ancestor to the initial
/// containing block (Quirks 3.5). Only this body's declared used height becomes
/// definite; the auto root and unrelated descendants retain their CSS basis.
pub(super) fn percentage_body_style(lb: &LayoutBox, bs: &BoxStyle, ctx: &Ctx) -> Option<BoxStyle> {
    if lb.style.get(crate::style::QUIRKS_DOCUMENT_BODY).is_none()
        || bs.position.is_out_of_flow()
        || native_height_basis(bs, ctx).is_some()
        || !matches!(bs.height, LengthPct::Pct(_) | LengthPct::Calc { .. })
        || table_internal(lb.display)
    {
        return None;
    }
    // Writing-root entry filters this basis to the active logical space.
    let parent = ctx.quirks_parent?;
    let height = bs.height.resolve(parent.percentage?)?;
    Some(BoxStyle {
        height: LengthPct::Px(height),
        ..bs.clone()
    })
}

/// Quirk-only growth keeps the trailing collapsed margin; an author constraint
/// that wins over that floor retains ordinary min/max-size margin behavior.
pub(super) fn used_height(
    lb: &LayoutBox,
    bs: &BoxStyle,
    (width, natural_auto, content): (f32, f32, f32),
    ctx: &mut Ctx,
) -> (f32, bool) {
    ctx.dropped_end_margins.remove(&lb.node_id);
    let auto = auto_height(lb, bs, width, natural_auto, ctx);
    let used = resolved_content_height(bs, auto, ctx);
    let quirk_won = auto > natural_auto && used == auto;
    (used, !quirk_won && used != content)
}

/// Bit zero describes the UA-origin margin now at logical block-start. Writing
/// mode relabelling permutes this metadata with the corresponding physical sides.
pub(super) fn drops_start(style: &crate::ComputedStyle) -> bool {
    style
        .get(crate::style::UA_QUIRKY_MARGIN_START)
        .and_then(|mask| mask.parse::<u8>().ok())
        .is_some_and(|mask| mask & 1 != 0)
}

/// Quirks 3.5 excludes table-internal percentage-height rules.
fn table_internal(display: super::super::value::Display) -> bool {
    use super::super::value::Display;
    matches!(
        display,
        Display::TableRow
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableCell
            | Display::TableCaption
    )
}

/// A UA block-end margin of a self-collapsing child is ignored only while it
/// collapses through the beginning of a quirky container.
pub(super) fn drops_end(style: &crate::ComputedStyle) -> bool {
    style
        .get(crate::style::UA_QUIRKY_MARGIN_START)
        .and_then(|mask| mask.parse::<u8>().ok())
        .is_some_and(|mask| mask & 4 != 0)
}

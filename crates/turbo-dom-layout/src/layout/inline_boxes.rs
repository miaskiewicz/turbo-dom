//! Native HTML inline boundaries survive flattening and line breaking. Border
//! fragments use the inline's own font box, not a union of descendant glyphs.
use super::block::Ctx;
use super::boxgen::InlineItem;
use super::fragment::{Fragment, FragmentContent, NodeId};
use super::inline::{InlineEdge, ParagraphLayout};
use super::value::{
    resolve_box_style, BorderEdges, BoxStyle, CornerRadius, Display, Edges, ResolveCtx,
};
use super::writing::WritingMode;
use crate::layout_snapshot::UsedBoxMetrics;
use crate::ComputedStyle;
use std::collections::HashMap;

pub(crate) fn atomic_element(element: &crate::StyledElement) -> bool {
    matches!(&element.tag,crate::Tag::Html(name) if matches!(name.as_str(),"img"|"canvas"|"iframe"|"object"|"embed"|"input"|"select"|"textarea"|"button"|"video"|"audio"|"svg"))
}
/// The elements whose ordinary inline layout the native inline producer does not
/// support (top/bottom/middle alignment, vertical writing, a displaced
/// inline split around a block, a displaced block split out of an inline), each
/// with its reason. The rest of the document lays out as usual: the snapshot fails
/// these elements' geometry reads only (miaskiewicz/turbo-surf#31), not the whole
/// snapshot.
pub(crate) fn unsupported(
    nodes: &[crate::StyledNode],
) -> Vec<(&crate::StyledElement, crate::layout_snapshot::SnapshotError)> {
    let mut out = Vec::new();
    collect_unsupported(nodes, (false, false, WritingMode::HorizontalTb), &mut out);
    out
}
type Unsupported<'a> = Vec<(
    &'a crate::StyledElement,
    crate::layout_snapshot::SnapshotError,
)>;
/// `(inside an ordinary inline, the parent is RTL, the parent's writing mode)`.
type Context = (bool, bool, WritingMode);
fn collect_unsupported<'a>(
    nodes: &'a [crate::StyledNode],
    context: Context,
    out: &mut Unsupported<'a>,
) {
    for node in nodes {
        if let crate::StyledNode::Element(element) = node {
            element_unsupported(element, context, out);
        }
    }
}
fn element_unsupported<'a>(
    element: &'a crate::StyledElement,
    (inside, parent_rtl, parent): Context,
    out: &mut Unsupported<'a>,
) {
    let rtl = is_rtl(&element.style);
    let display = super::value::display_of(&element.style);
    if display == Display::None || matches!(&element.tag,crate::Tag::Html(name) if name=="br") {
        return;
    }
    if element.style.get("display") == Some("contents") {
        return collect_unsupported(&element.children, (inside, parent_rtl, parent), out);
    }
    let ordinary = ordinary_inline(element, display);
    let mode = WritingMode::of(&element.style);
    match validate_rendered_element(element, display, inside, ordinary) {
        Err(error) => out.push((element, error)),
        Ok(()) => collect_unsupported(
            &element.children,
            (inline_context(display, inside, ordinary), rtl, mode),
            out,
        ),
    }
}
/// A character `text-orientation: mixed` sets upright: its Unicode
/// `Vertical_Orientation` (UAX #50) is U or Tu (the generated
/// [`super::vertical_orientation::UPRIGHT`] table).
pub(super) fn sets_upright(c: char) -> bool {
    in_ranges(super::vertical_orientation::UPRIGHT, c)
}

/// A character that takes the previous character's orientation instead of its own: a
/// combining mark, variation selector, ZWJ, emoji modifier or tag (its
/// `Grapheme_Cluster_Break` is `Extend`, `ZWJ` or `SpacingMark`), so a sequence is not split.
pub(super) fn inherits_orientation(c: char) -> bool {
    in_ranges(super::vertical_orientation::INHERIT, c)
}

fn in_ranges(ranges: &[(u32, u32)], c: char) -> bool {
    let c = u32::from(c);
    let i = ranges.partition_point(|&(_, last)| last < c);
    ranges.get(i).is_some_and(|&(first, _)| first <= c)
}
fn validate_rendered_element(
    element: &crate::StyledElement,
    display: Display,
    inside: bool,
    ordinary: bool,
) -> Result<(), crate::layout_snapshot::SnapshotError> {
    validate_context(display, inside, ordinary, &element.style)?;
    validate_displaced_split(element, ordinary)
}
fn validate_displaced_split(
    element: &crate::StyledElement,
    ordinary: bool,
) -> Result<(), crate::layout_snapshot::SnapshotError> {
    if ordinary && displaced_inline(&element.style) && contains_block(&element.children) {
        return Err(crate::layout_snapshot::SnapshotError::UnsupportedInlineBlockSplit);
    }
    Ok(())
}
fn contains_block(nodes: &[crate::StyledNode]) -> bool {
    let mut found = false;
    super::contents::visit(nodes, &ComputedStyle::default(), &mut |node, _| {
        found |= block_in_inline_node(node);
    });
    found
}
fn block_in_inline_node(node: &crate::StyledNode) -> bool {
    let crate::StyledNode::Element(element) = node else {
        return false;
    };
    let display = super::value::display_of(&element.style);
    if splits_nothing(display, &element.style) {
        return false;
    }
    if is_block_split(display) {
        return true;
    }
    display == Display::Inline && !atomic_element(element) && contains_block(&element.children)
}
/// A box that never splits an inline: one not rendered, an absolutely positioned one
/// or a float (it stays in its inline run).
fn splits_nothing(display: Display, style: &ComputedStyle) -> bool {
    display == Display::None
        || super::value::position_of(style).is_out_of_flow()
        || super::value::float_of(style) != super::value::Float::None
}
fn displaced_inline(style: &ComputedStyle) -> bool {
    style.get("position") == Some("relative")
        || style.get("float").is_some_and(|value| value != "none")
}

fn ordinary_inline(element: &crate::StyledElement, display: Display) -> bool {
    display == Display::Inline && !atomic_element(element)
}
fn inline_context(display: Display, inside: bool, ordinary: bool) -> bool {
    matches!(display, Display::Inline) && (inside || ordinary)
}
fn validate_context(
    display: Display,
    inside: bool,
    ordinary: bool,
    style: &ComputedStyle,
) -> Result<(), crate::layout_snapshot::SnapshotError> {
    if unsupported_block_split(display, inside, style) {
        return Err(crate::layout_snapshot::SnapshotError::UnsupportedInlineBlockSplit);
    }
    if ordinary {
        validate_inline_style(style)?;
    }
    Ok(())
}
fn unsupported_block_split(display: Display, inside: bool, style: &ComputedStyle) -> bool {
    inside && is_block_split(display) && displaced_block(style)
}
/// A block split out of an inline that the producer cannot place: a `sticky` one
/// (an absolutely positioned box or a float is not split out; it stays in the run).
fn displaced_block(style: &ComputedStyle) -> bool {
    style.get("position") == Some("sticky")
}
fn is_block_split(display: Display) -> bool {
    !matches!(
        display,
        Display::Inline
            | Display::InlineBlock
            | Display::InlineTable
            | Display::InlineFlex
            | Display::InlineGrid
    )
}
/// An ordinary inline's own unsupported style. `absolute` / `fixed` never reach here:
/// they blockify ([`super::value::display_of`]).
fn validate_inline_style(
    style: &ComputedStyle,
) -> Result<(), crate::layout_snapshot::SnapshotError> {
    use crate::layout_snapshot::SnapshotError;
    if matches!(
        style.get("vertical-align"),
        Some("top" | "bottom" | "middle")
    ) {
        return Err(SnapshotError::UnsupportedInlineAlignment);
    }
    Ok(())
}
fn is_rtl(style: &ComputedStyle) -> bool {
    style.get("direction").map(str::trim) == Some("rtl")
}

fn style(style: &ComputedStyle, font: f32, width: f32, scale: Option<f32>) -> BoxStyle {
    let mut used = resolve_box_style(
        style,
        ResolveCtx {
            parent_font_size: font,
            cb_width: width,
        },
    );
    super::value::quantize_html_box_style(&mut used, scale);
    used
}

pub(super) fn edge(
    node_id: NodeId,
    source: &ComputedStyle,
    start: bool,
    font: f32,
    width: f32,
    ctx: &Ctx,
) -> Option<InlineEdge> {
    let used = style(source, font, width, ctx.html_font_scale);
    let families = used
        .font_families
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let face = ctx
        .fonts
        .select(&families, used.font_weight, used.italic)?
        .clone();
    let border = used.border.widths();
    let left = used.margin.left + border.left + used.padding.left;
    let right = used.padding.right + border.right + used.margin.right;
    // An RTL box starts at its right side (miaskiewicz/turbo-dom#238).
    let rtl = is_rtl(source);
    let (advance, other) = if start == rtl {
        (right, left)
    } else {
        (left, right)
    };
    Some(InlineEdge {
        node_id,
        start,
        advance,
        face,
        font_size: used.font_size,
        line_height: used.line_height,
        contributes_metrics: true,
        valign: used.vertical_align,
        baseline_shift: 0.0,
        rtl,
        bidi: super::inline::UnicodeBidi::parse(source.get("unicode-bidi")),
        clone_advance: cloned(source).then_some(other),
    })
}

/// Whether the box's fragments each repeat its decoration
/// (`box-decoration-break: clone`, or the `-webkit-` alias).
pub(super) fn cloned(source: &ComputedStyle) -> bool {
    ["box-decoration-break", "-webkit-box-decoration-break"]
        .iter()
        .any(|property| source.get(property).map(str::trim) == Some("clone"))
}

pub(super) struct SourcePlacement {
    pub offsets: HashMap<NodeId, (f32, f32)>,
    pub baselines: HashMap<NodeId, f32>,
}
struct ActiveInline {
    offset: (f32, f32),
    font: f32,
    shift: f32,
}
struct SourceOffsets {
    placement: SourcePlacement,
    stack: Vec<ActiveInline>,
}
impl SourceOffsets {
    fn current(&self) -> (f32, f32) {
        self.stack
            .last()
            .map(|value| value.offset)
            .unwrap_or_default()
    }
    fn shift(&self) -> f32 {
        self.stack
            .last()
            .map(|value| value.shift)
            .unwrap_or_default()
    }
    fn record(&mut self, id: NodeId) {
        self.placement.offsets.insert(id, self.current());
        self.placement.baselines.insert(id, self.shift());
    }
    fn boundary(
        &mut self,
        id: NodeId,
        source: &ComputedStyle,
        start: bool,
        font: f32,
        width: f32,
        ctx: &Ctx,
    ) {
        if !start {
            self.stack.pop();
            return;
        }
        let used = style(source, font, width, ctx.html_font_scale);
        let parent_font = self.stack.last().map_or(font, |value| value.font);
        let shift = self.shift()
            + native_baseline_shift(
                used.vertical_align,
                parent_font,
                ctx.html_font_scale.unwrap_or(1.0),
            );
        // A sticky inline is shifted once placed (`block::inline_sticky`).
        let (dx, dy) = if used.position == super::value::Position::Sticky {
            (0.0, 0.0)
        } else {
            super::block::flow_relative_offset(&used, width, ctx)
        };
        let (px, py) = self.current();
        self.stack.push(ActiveInline {
            offset: (px + dx, py + dy),
            font: used.font_size,
            shift,
        });
        self.record(id);
    }
    fn item(&mut self, item: &InlineItem, font: f32, width: f32, ctx: &Ctx) {
        match item {
            InlineItem::Boundary {
                node_id,
                style,
                start,
            } => self.boundary(*node_id, style, *start, font, width, ctx),
            InlineItem::Text { node_id, .. } | InlineItem::SourceBreak { node_id, .. } => {
                self.record(*node_id);
            }
            InlineItem::Atomic(child) => {
                self.record(child.node_id);
                let used = style(&child.style, font, width, ctx.html_font_scale);
                let parent_font = self.stack.last().map_or(font, |parent| parent.font);
                self.placement.baselines.insert(
                    child.node_id,
                    self.shift()
                        + native_baseline_shift(
                            used.vertical_align,
                            parent_font,
                            ctx.html_font_scale.unwrap_or(1.0),
                        ),
                );
            }
            _ => {}
        }
    }
}
fn native_baseline_shift(align: super::value::VAlign, parent_font: f32, scale: f32) -> f32 {
    let units = (parent_font * scale * 64.0).round() as i64;
    match align {
        super::value::VAlign::Super => (units / 3 + 64) as f32 / 64.0 / scale,
        super::value::VAlign::Sub => -(units / 5 + 64) as f32 / 64.0 / scale,
        _ => 0.0,
    }
}
pub(super) fn positions(items: &[InlineItem], font: f32, width: f32, ctx: &Ctx) -> SourcePlacement {
    let mut source = SourceOffsets {
        placement: SourcePlacement {
            offsets: HashMap::new(),
            baselines: HashMap::new(),
        },
        stack: Vec::new(),
    };
    for item in items {
        source.item(item, font, width, ctx);
    }
    source.placement
}
/// The block's strut `(ascent, descent)`, about the central baseline when `central`.
pub(super) fn strut(bs: &BoxStyle, ctx: &Ctx, central: bool) -> Option<(f32, f32)> {
    let families = bs
        .font_families
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let face = ctx.fonts.select(&families, bs.font_weight, bs.italic)?;
    let scale = ctx.html_font_scale.unwrap_or(1.0);
    let font = face.html_metrics(bs.font_size * scale);
    let (ascent, descent) = if central {
        super::inline::central_metrics(font.ascent / scale, font.descent / scale, scale)
    } else {
        (font.ascent / scale, font.descent / scale)
    };
    Some(super::inline::native_font_height(
        ascent,
        descent,
        bs.line_height
            .unwrap_or((font.ascent + font.descent + font.line_gap) / scale),
        0.0,
        scale,
    ))
}

pub(super) fn native_atoms(
    pieces: &mut [super::inline::Piece],
    atoms: &[Fragment],
    baselines: &HashMap<NodeId, f32>,
) {
    for piece in pieces {
        if let super::inline::Piece::Atom(atom) = piece {
            let shift = baselines
                .get(&atoms[atom.id].node_id)
                .copied()
                .unwrap_or_default();
            *piece = super::inline::Piece::NativeAtom(*atom, shift);
        }
    }
}
fn visible_inline_item(item: &InlineItem, font: f32, width: f32, ctx: &Ctx) -> bool {
    match item {
        InlineItem::Boundary {
            style: source,
            start: true,
            ..
        } => has_horizontal_edges(&style(source, font, width, ctx.html_font_scale)),
        InlineItem::Boundary { .. } => false,
        InlineItem::Text { text, style, .. } => {
            text.chars()
                .any(|character| !matches!(character, ' ' | '\t' | '\r' | '\n' | '\u{c}'))
                || preserved_space(style, text)
        }
        _ => true,
    }
}
/// Whether white-space-only `text` renders: preserved spaces do, and so does a
/// `pre-line` newline (a forced line break, miaskiewicz/turbo-dom#121).
fn preserved_space(style: &ComputedStyle, text: &str) -> bool {
    match style.get("white-space") {
        Some("pre" | "pre-wrap" | "break-spaces") => !text.is_empty(),
        Some("pre-line") => text.contains('\n'),
        _ => false,
    }
}
fn collapse_text_sources(items: &[InlineItem], ctx: &mut Ctx) {
    if let Some(capture) = &mut ctx.text_capture {
        for item in items {
            if let InlineItem::Text { node_id, .. } = item {
                capture.collapsed_sources.insert(*node_id);
            }
        }
    }
}
fn has_horizontal_edges(used: &BoxStyle) -> bool {
    let border = used.border.widths();
    [
        used.margin.left,
        used.margin.right,
        used.padding.left,
        used.padding.right,
        border.left,
        border.right,
    ]
    .iter()
    .any(|value| *value != 0.0)
}
pub(super) fn collapsible_run(items: &[InlineItem], font: f32, width: f32) -> bool {
    items.iter().all(|item| match item {
        InlineItem::Boundary { style: source, .. } => {
            !has_horizontal_edges(&style(source, font, width, None))
        }
        InlineItem::Text { text, style, .. } => {
            !matches!(
                style.get("white-space"),
                Some("pre" | "pre-wrap" | "break-spaces")
            ) && !preserved_space(style, text)
                && text
                    .chars()
                    .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{c}'))
        }
        _ => false,
    })
}

pub(super) fn empty_fragments(
    items: &[InlineItem],
    font: f32,
    origin: (f32, f32),
    width: f32,
    placement: &SourcePlacement,
    ctx: &mut Ctx,
) -> Option<Vec<Fragment>> {
    if !ctx.html_font_metrics
        || items
            .iter()
            .any(|item| visible_inline_item(item, font, width, ctx))
    {
        return None;
    }
    collapse_text_sources(items, ctx);
    let mut out = Vec::new();
    for item in items {
        if let InlineItem::Boundary {
            node_id,
            start: true,
            ..
        } = item
        {
            out.push(empty_fragment(*node_id, origin, placement, ctx));
        }
    }
    Some(out)
}
fn empty_fragment(
    id: NodeId,
    origin: (f32, f32),
    placement: &SourcePlacement,
    ctx: &mut Ctx,
) -> Fragment {
    if let Some(metrics) = &mut ctx.used_metrics {
        metrics.insert(
            id,
            UsedBoxMetrics {
                inline_fragment: true,
                border: Edges::default(),
                padding: Edges::default(),
                margin: Edges::default(),
                content_width: 0.0,
                content_height: 0.0,
                position: super::value::Position::Static,
                display: Display::Inline,
            },
        );
    }
    let (dx, dy) = placement.offsets.get(&id).copied().unwrap_or_default();
    Fragment::new(
        id,
        origin.0 + dx,
        origin.1 + dy,
        0.0,
        0.0,
        FragmentContent::Box {
            background: None,
            border: BorderEdges::default(),
            border_radius: super::fragment::BorderRadii::ZERO,
            shadow: None,
            gradient: None,
            transform: None,
        },
    )
}
#[derive(Clone)]
struct Open {
    edge: InlineEdge,
    x: f32,
    first: bool,
    paused: bool,
}
struct InlineFragments<'a, 'b> {
    styles: HashMap<NodeId, BoxStyle>,
    open: Vec<Open>,
    out: Vec<Fragment>,
    ctx: &'a mut Ctx<'b>,
    origin: (f32, f32, f32),
    offsets: &'a HashMap<NodeId, (f32, f32)>,
    /// The inline boxes whose content area centres on the line's central baseline
    /// ([`super::writing::central_baseline`]).
    central: std::collections::HashSet<NodeId>,
}
impl InlineFragments<'_, '_> {
    fn line(&mut self, line: &super::inline::InlineLine, cx: f32, cy: f32) {
        self.origin = (cx + line.indent, cy + line.top, line.baseline);
        for value in &mut self.open {
            value.x = line.inline_origin;
            value.first = false;
        }
        for placed in &line.inline_edges {
            if placed.edge.start {
                self.start(placed);
            } else {
                self.end(placed);
            }
        }
        self.finish_line(line.inline_origin + line.width);
    }
    fn start(&mut self, placed: &super::inline::PlacedInlineEdge) {
        let parent = self
            .open
            .last()
            .filter(|parent| transparent(&self.styles[&parent.edge.node_id]))
            .cloned();
        if let Some(parent) = parent {
            self.flush(&parent, placed.x, false);
            if let Some(parent) = self.open.last_mut() {
                parent.paused = true;
            }
        }
        self.open.push(Open {
            edge: placed.edge.clone(),
            x: placed.x,
            first: placed.decorated,
            paused: false,
        });
    }
    fn end(&mut self, placed: &super::inline::PlacedInlineEdge) {
        let Some(index) = self
            .open
            .iter()
            .rposition(|value| value.edge.node_id == placed.edge.node_id)
        else {
            return;
        };
        let value = self.open.remove(index);
        if !value.paused {
            self.flush(&value, placed.x + placed.edge.advance, placed.decorated);
        }
        if let Some(parent) = self.open.last_mut().filter(|parent| parent.paused) {
            parent.x = placed.x + placed.edge.advance;
            parent.paused = false;
            parent.first = false;
        }
    }
    fn finish_line(&mut self, end: f32) {
        let active = self
            .open
            .iter()
            .filter(|value| !value.paused)
            .cloned()
            .collect::<Vec<_>>();
        for value in active {
            self.flush(&value, end, false);
        }
    }
    fn flush(&mut self, value: &Open, end: f32, last: bool) {
        let used = &self.styles[&value.edge.node_id];
        if empty_tail(value, end, used) {
            return;
        }
        let central = self.central.contains(&value.edge.node_id);
        let mut part = fragment(value, used, (end, last, central), self.origin, self.ctx);
        let (dx, dy) = self.offsets.get(&part.node_id).copied().unwrap_or_default();
        part.translate(dx, dy);
        if skip_empty_part(&part, value, last, used) {
            return;
        }
        append_part(&mut self.out, part, &self.open, &self.styles, self.ctx);
    }
}
pub(super) fn fragments(
    layout: &ParagraphLayout,
    items: &[InlineItem],
    font: f32,
    origin: (f32, f32),
    width: f32,
    ctx: &mut Ctx,
    offsets: &HashMap<NodeId, (f32, f32)>,
) -> Vec<Fragment> {
    let styles = items
        .iter()
        .filter_map(|item| match item {
            InlineItem::Boundary {
                node_id,
                style: source,
                start: true,
            }
            | InlineItem::SourceBreak {
                node_id,
                style: source,
            } => Some((*node_id, style(source, font, width, ctx.html_font_scale))),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    let central = items
        .iter()
        .filter_map(|item| match item {
            InlineItem::Boundary {
                node_id,
                style,
                start: true,
            } if super::writing::central_baseline(ctx.writing, style) => Some(*node_id),
            _ => None,
        })
        .collect();
    let mut builder = InlineFragments {
        styles,
        open: Vec::new(),
        out: Vec::new(),
        ctx,
        origin: (origin.0, origin.1, 0.0),
        offsets,
        central,
    };
    for line in &layout.lines {
        builder.line(line, origin.0, origin.1);
    }
    let InlineFragments {
        mut out,
        styles,
        ctx,
        ..
    } = builder;
    let sources = items
        .iter()
        .filter_map(|item| match item {
            InlineItem::Boundary {
                node_id,
                style: source,
                start: true,
            } => Some((*node_id, source)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    for (id, source) in sources {
        background_layers(&mut out, (id, source, &styles[&id]), ctx);
    }
    out
}

/// Move an inline box's background image and gradient (and a colour clipped short of
/// the border box) off its line fragments onto background layers
/// (miaskiewicz/turbo-dom#244, #248). The positioning area is the box's strip: every
/// fragment's origin box laid end to end in line order (`box-decoration-break: slice`),
/// or the fragment's own (`clone`); each fragment paints its layers clipped to its own
/// clip box. The fragments' vertical padding bounds a `content-box` area (their
/// horizontal padding is not recorded per fragment).
fn background_layers(
    out: &mut [Fragment],
    (id, source, used): (NodeId, &ComputedStyle, &BoxStyle),
    ctx: &Ctx,
) {
    use super::background::{
        area_radius, area_rect, border_overlay, color_layer, layer, props, Area, Source,
    };
    let image = super::boxgen::background_image(source).and_then(|name| {
        let intrinsic = super::imgsize::box_intrinsic(ctx.images.resolver, &name, source)?;
        Some(super::imgsize::placement_of(name, intrinsic))
    });
    let props = props(source, used.font_size);
    let gradient = used.background_gradient.is_some();
    let colour = used.background.is_some() && props.clip != Area::Border;
    if image.is_none() && !gradient && !colour {
        return;
    }
    let cloned = cloned(source);
    let parts: Vec<usize> = (0..out.len()).filter(|&i| out[i].node_id == id).collect();
    // Each fragment's padding: all of it when cloned, else the start side on the first
    // fragment and the end side on the last.
    let padding = |n: usize| {
        let (first, last) = (cloned || n == 0, cloned || n + 1 == parts.len());
        let (left, right) = if used.rtl {
            (last, first)
        } else {
            (first, last)
        };
        super::value::Edges {
            left: if left { used.padding.left } else { 0.0 },
            right: if right { used.padding.right } else { 0.0 },
            ..used.padding
        }
    };
    let mut areas = Vec::new();
    for (n, &index) in parts.iter().enumerate() {
        let frag = &out[index];
        if let FragmentContent::Box { border, .. } = &frag.content {
            let rect = (frag.x, frag.y, frag.width, frag.height);
            let pad = padding(n);
            areas.push((
                index,
                area_rect(rect, border, &pad, props.origin),
                area_rect(rect, border, &pad, props.clip),
                pad,
            ));
        }
    }
    let total: f32 = areas.iter().map(|(_, origin, _, _)| origin.2).sum();
    let mut offset = 0.0;
    for (index, origin, paint, pad) in areas {
        let strip = if cloned {
            origin
        } else {
            (origin.0 - offset, origin.1, total, origin.3)
        };
        offset += origin.2;
        let frag = &mut out[index];
        if let FragmentContent::Box {
            background,
            gradient,
            border,
            border_radius,
            ..
        } = &mut frag.content
        {
            let radius = area_radius(*border_radius, border, &pad, props.clip);
            let mut layers = Vec::new();
            if let Some(color) = background.take_if(|_| props.clip != Area::Border) {
                layers.push(color_layer(id, color, paint, radius));
            }
            let sources = gradient
                .take()
                .map(Source::Gradient)
                .into_iter()
                .chain(image.clone().map(Source::Image));
            for source in sources {
                layers.extend(layer(id, &props, &source, (strip, paint, radius)));
            }
            if !layers.is_empty() {
                // The border paints over the background layers.
                let rect = (frag.x, frag.y, frag.width, frag.height);
                layers.push(border_overlay(
                    id,
                    rect,
                    std::mem::take(border),
                    *border_radius,
                ));
            }
            frag.children.extend(layers);
        }
    }
}

fn skip_empty_part(part: &Fragment, value: &Open, last: bool, used: &BoxStyle) -> bool {
    part.width == 0.0 && transparent(used) && (!last || !value.first)
}
fn empty_tail(value: &Open, end: f32, used: &BoxStyle) -> bool {
    !value.first && transparent(used) && end <= value.x
}
fn transparent(used: &BoxStyle) -> bool {
    used.padding == super::value::Edges::default()
        && used.border.widths() == super::value::Edges::default()
        && used.background.is_none_or(|color| color.a == 0)
        && used.background_gradient.is_none()
        && used.box_shadow.is_none()
}
fn append_part(
    out: &mut Vec<Fragment>,
    part: Fragment,
    open: &[Open],
    styles: &HashMap<NodeId, BoxStyle>,
    ctx: &mut Ctx,
) {
    // A boxless inline's rect list consists of its own runs and the actual
    // descendant inline boxes; decoration establishes one enclosing box instead.
    for parent in open
        .iter()
        .rev()
        .filter(|parent| parent.edge.node_id != part.node_id)
        .take_while(|parent| parent.paused && transparent(&styles[&parent.edge.node_id]))
    {
        let mut inherited = part.clone();
        inherited.node_id = parent.edge.node_id;
        inherited.content = FragmentContent::Box {
            background: None,
            border: BorderEdges::default(),
            border_radius: super::fragment::BorderRadii::ZERO,
            shadow: None,
            gradient: None,
            transform: None,
        };
        if let Some(metrics) = &mut ctx.used_metrics {
            metrics.entry(inherited.node_id).or_insert(UsedBoxMetrics {
                inline_fragment: true,
                border: Edges::default(),
                padding: Edges::default(),
                margin: styles[&inherited.node_id].margin,
                content_width: part.width,
                content_height: part.height,
                position: styles[&inherited.node_id].position,
                display: Display::Inline,
            });
        }
        out.push(inherited);
    }
    out.push(part);
}

fn fragment(
    open: &Open,
    used: &BoxStyle,
    (end, last, central): (f32, bool, bool),
    origin: (f32, f32, f32),
    ctx: &mut Ctx,
) -> Fragment {
    let scale = ctx.html_font_scale.unwrap_or(1.0);
    let font = open.edge.face.html_metrics(open.edge.font_size * scale);
    let content_height = (font.ascent + font.descent) / scale;
    let ascent = if central {
        super::inline::central_metrics(font.ascent / scale, font.descent / scale, scale).0
    } else {
        font.ascent / scale
    };
    let mut border = used.border;
    let mut padding = used.padding;
    // `[top-left, top-right, bottom-right, bottom-left]`: a fragment without an edge has
    // square corners on that side (with `clone` every fragment has both edges).
    let mut corners = used.border_radius;
    let x = open.x
        + if open.first {
            used.margin.left
        } else {
            border.left.width = 0;
            padding.left = 0.0;
            (corners[0], corners[3]) = (CornerRadius::ZERO, CornerRadius::ZERO);
            0.0
        };
    let end = end
        - if last {
            used.margin.right
        } else {
            border.right.width = 0;
            padding.right = 0.0;
            (corners[1], corners[2]) = (CornerRadius::ZERO, CornerRadius::ZERO);
            0.0
        };
    let border_width = border.widths();
    let height = content_height + padding.vertical() + border_width.vertical();
    let width = (end - x).max(0.0);
    let shift = open.edge.baseline_shift;
    let y = origin.1 + origin.2 - shift - ascent - padding.top - border_width.top;
    if let Some(metrics) = &mut ctx.used_metrics {
        metrics.entry(open.edge.node_id).or_insert(UsedBoxMetrics {
            inline_fragment: true,
            border: border_width,
            padding,
            margin: used.margin,
            content_width: (width - padding.horizontal() - border_width.horizontal()).max(0.0),
            content_height,
            position: used.position,
            display: Display::Inline,
        });
    }
    Fragment::new(
        open.edge.node_id,
        origin.0 + x,
        y,
        width,
        height,
        FragmentContent::Box {
            background: used.background,
            border,
            border_radius: super::value::resolve_radii(&corners, width, height),
            shadow: used.box_shadow,
            gradient: used.background_gradient.clone(),
            transform: None,
        },
    )
}

#[cfg(test)]
mod defensive_contract_tests {
    use super::*;

    #[test]
    fn upright_characters_follow_unicode_vertical_orientation() {
        // R2 #8 (UAX #50 Vertical_Orientation 16.0): U / Tu upright, R / Tr turned.
        let cp = |c: u32| sets_upright(char::from_u32(c).unwrap());
        // Halfwidth kana and Hangul are R.
        for r in [0xFF61, 0xFF65, 0xFF9F, 0xFFA0, 0xFFDC] {
            assert!(!cp(r), "{r:04X} is R");
        }
        // ©, ®, enclosed alphanumerics, geometric shapes, miscellaneous symbols and
        // dingbats (to U+2767), emoji and their skin-tone modifiers are U.
        for u in [
            0xA9, 0xAE, 0x2460, 0x24FF, 0x25A0, 0x25FF, 0x2600, 0x2767, 0x2776, 0x1F468, 0x1F3FB,
            0x6C34, 0x3042, 0xAC00,
        ] {
            assert!(cp(u), "{u:04X} is U");
        }
        // The rest of Dingbats from U+2768 (ornamental brackets) on, and Latin, are R.
        for r in [0x2768, 0x2794, 0x27BF, 0x61, 0x2010, 0xFE0F, 0x200D] {
            assert!(!cp(r), "{r:04X} is R");
        }
    }

    #[test]
    fn marks_selectors_and_zwj_take_the_previous_orientation() {
        use super::super::inline::UprightText;
        let inherits = |c: u32| inherits_orientation(char::from_u32(c).unwrap());
        // ZWJ, variation selectors (and the supplement), combining marks (U+20DD is U on
        // its own), a spacing mark, emoji modifiers and tags.
        for c in [
            0x200D, 0xFE0E, 0xFE0F, 0xE0100, 0x0301, 0x20DD, 0x0903, 0x1F3FB, 0xE0020,
        ] {
            assert!(inherits(c), "{c:04X} inherits");
        }
        for c in [0x61, 0xA9, 0x1F468, 0x200C - 1] {
            assert!(!inherits(c), "{c:04X} has its own");
        }
        let mixed = UprightText::Mixed;
        // After a turned `a`, U+20DD (U alone) turns; after an upright `©`, a variation
        // selector (R alone) stands; at a run's start a character has its own.
        assert!(!mixed.sets_after('\u{20DD}', Some(false)));
        assert!(mixed.sets_after('\u{FE0F}', Some(true)));
        assert!(mixed.sets_after('\u{20DD}', None));
        // The character after a ZWJ keeps its own (Chrome: `© ZWJ a` turns the `a`).
        assert!(!mixed.sets_after('a', Some(true)));
        // `upright` and `sideways` ignore the sequence.
        assert!(UprightText::All.sets_after('a', Some(false)));
        assert!(!UprightText::None.sets_after('\u{FE0F}', Some(true)));
    }

    #[test]
    fn unmatched_closing_marker_does_not_create_an_element_fragment() {
        // Valid DOM capture emits balanced boundaries. A corrupt intermediate
        // closing marker must not acquire a synthetic source box.
        let fonts = crate::FontRegistry::new();
        let images = crate::ImageCtx {
            resolver: &crate::NoImages,
            body_height: None,
        };
        let mut diagnostics = crate::Diagnostics::default();
        let mut ctx = Ctx {
            fonts: &fonts,
            images: &images,
            diags: &mut diagnostics,
            abs_cb_x: 0.0,
            abs_cb_y: 0.0,
            abs_cb_w: 200.0,
            abs_cb_h: 100.0,
            abs_cb_h_known: true,
            cb_h: 100.0,
            cb_h_known: true,
            root_w: 200.0,
            root_h: 100.0,
            icb_h: 0.0,
            abs_cb_icb: true,
            live_geometry: true,
            used_metrics: Some(HashMap::new()),
            text_capture: None,
            html_font_metrics: true,
            html_font_scale: Some(2.0),
            original_units: None,
            native_transforms: Some(HashMap::new()),
            floats: Vec::new(),
            table_rows: None,
            dropped_end_margins: std::collections::HashSet::new(),
            sticky_pending: Vec::new(),
            writing: crate::layout::writing::WritingMode::default(),
        };
        let edge = super::super::inline::PlacedInlineEdge {
            edge: InlineEdge {
                node_id: NodeId(7),
                start: false,
                advance: 3.0,
                face: fonts.select(&["sans-serif"], 400, false).unwrap().clone(),
                font_size: 16.0,
                line_height: Some(24.0),
                baseline_shift: 0.0,
                contributes_metrics: true,
                valign: super::super::value::VAlign::Baseline,
                rtl: false,
                bidi: super::super::inline::UnicodeBidi::Normal,
                clone_advance: None,
            },
            x: 20.0,
            decorated: true,
        };
        let offsets = HashMap::new();
        let mut projection = InlineFragments {
            styles: HashMap::new(),
            open: Vec::new(),
            out: Vec::new(),
            ctx: &mut ctx,
            origin: (0.0, 0.0, 18.0),
            offsets: &offsets,
            central: std::collections::HashSet::new(),
        };
        projection.end(&edge);
        assert!(projection.out.is_empty());
        assert!(projection.open.is_empty());
        drop(projection);
        assert!(ctx.used_metrics.unwrap().is_empty());
    }
}

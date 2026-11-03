//! Vertical writing modes (`writing-mode: vertical-rl` / `vertical-lr`,
//! miaskiewicz/turbo-dom#234) through a logical-coordinate layer.
//!
//! The block and inline layout code works in one coordinate space whose `x` is the
//! inline axis and `y` the block axis. A box whose writing mode differs from its
//! parent's (a *writing-mode root*) is laid out in its own logical space: its subtree
//! is restyled so the physical sides and sizes name their logical counterparts
//! (`height` is the inline size of a vertical box, `margin-right` its block-start
//! margin in `vertical-rl`, …), laid out with the ordinary block / inline code (so
//! lines break at the inline — vertical — size, as in Chrome), and the resulting
//! fragments and used box metrics are mapped back into the parent's space. The map
//! composes: a horizontal box in a vertical parent is a root of its own, mapped into
//! the parent's logical space and from there, with the parent, to physical space.
//!
//! A root whose block axis is the parent's inline axis (an *orthogonal flow*) sizes its
//! `auto` inline size to fit its content within the available space (CSS Writing Modes 4
//! §7.3): the parent's definite size along that axis, else the initial containing
//! block's.
use std::collections::HashMap;

use super::block::Ctx;
use super::fragment::{BorderRadii, CornerRadii, Fragment, FragmentContent, NodeId};
use super::value::{
    corner_at, corner_radius_values, css_value_tokens, BorderEdges, Edges, CORNER_RADIUS_LONGHANDS,
    LOGICAL_RADIUS_LONGHANDS,
};
use crate::layout_snapshot::UsedBoxMetrics;
use crate::ComputedStyle;

/// A box's used writing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WritingMode {
    #[default]
    HorizontalTb,
    VerticalRl,
    VerticalLr,
}

/// Physical sides in CSS order.
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

impl WritingMode {
    /// The computed `writing-mode` of `style` (the `sideways-*` modes lay out as their
    /// `vertical-*` counterparts; the legacy SVG `tb*` / `rl` keywords map as Chrome).
    pub(crate) fn of(style: &ComputedStyle) -> WritingMode {
        match style.get("writing-mode").map(str::trim) {
            Some("vertical-rl" | "sideways-rl" | "tb-rl" | "tb") => WritingMode::VerticalRl,
            Some("vertical-lr" | "sideways-lr") => WritingMode::VerticalLr,
            _ => WritingMode::HorizontalTb,
        }
    }

    pub(crate) fn vertical(self) -> bool {
        self != WritingMode::HorizontalTb
    }

    /// For each logical side (`top` = block-start, `left` = inline-start, …, in
    /// [`SIDES`] order), the physical side it is.
    fn logical_sides(self) -> [usize; 4] {
        match self {
            WritingMode::HorizontalTb => [0, 1, 2, 3],
            WritingMode::VerticalRl => [1, 2, 3, 0],
            WritingMode::VerticalLr => [3, 2, 1, 0],
        }
    }

    /// For each physical side, the logical side it is (the inverse of
    /// [`Self::logical_sides`]).
    fn physical_sides(self) -> [usize; 4] {
        let from = self.logical_sides();
        let mut inverse = [0; 4];
        for (logical, &physical) in from.iter().enumerate() {
            inverse[physical] = logical;
        }
        inverse
    }
}

/// Whether lines of a box with `style`, laid out in the space of `mode`, align on the
/// ideographic central baseline: a vertical mode but `text-orientation: sideways` (or a
/// `sideways-*` writing mode), Blink's `ComputedStyle::GetFontBaseline`
/// (miaskiewicz/turbo-dom#239).
pub(crate) fn central_baseline(mode: WritingMode, style: &ComputedStyle) -> bool {
    let sideways = style.get("text-orientation").map(str::trim) == Some("sideways")
        || style
            .get("writing-mode")
            .is_some_and(|value| value.trim().starts_with("sideways"));
    mode.vertical() && !sideways
}

/// How sides and sizes of one logical space name those of another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Relabel {
    /// For each side of the target space, the source side it is.
    sides: [usize; 4],
    /// Whether the inline and block axes swap.
    swap: bool,
}

impl Relabel {
    /// From the space of `source` to the space of `target`.
    pub(crate) fn between(source: WritingMode, target: WritingMode) -> Relabel {
        let to_physical = source.physical_sides();
        let from = target.logical_sides();
        Relabel {
            sides: from.map(|physical| to_physical[physical]),
            swap: source.vertical() != target.vertical(),
        }
    }

    /// This relabel followed by `next` (from this one's target space to another): one
    /// relabel from this one's source space to `next`'s target space.
    pub(crate) fn then(self, next: Relabel) -> Relabel {
        Relabel {
            sides: next.sides.map(|side| self.sides[side]),
            swap: self.swap != next.swap,
        }
    }

    /// A compact code (two bits per side, then the swap) for memo keys.
    pub(crate) fn code(self) -> u16 {
        let sides = self
            .sides
            .iter()
            .enumerate()
            .fold(0, |code, (i, &side)| code | ((side as u16) << (2 * i)));
        sides | (u16::from(self.swap) << 8)
    }

    /// The relabel [`Relabel::code`] encoded.
    pub(crate) fn decode(code: u16) -> Relabel {
        Relabel {
            sides: std::array::from_fn(|i| usize::from((code >> (2 * i)) & 3)),
            swap: code >> 8 == 1,
        }
    }

    /// Whether the inline and block axes swap.
    pub(crate) fn swaps(self) -> bool {
        self.swap
    }

    fn edges(self, edges: Edges) -> Edges {
        let source = [edges.top, edges.right, edges.bottom, edges.left];
        Edges {
            top: source[self.sides[0]],
            right: source[self.sides[1]],
            bottom: source[self.sides[2]],
            left: source[self.sides[3]],
        }
    }

    /// `style` with its physical sides and sizes relabelled into the target space.
    /// Flow-relative properties (`margin-inline-start`, …) already name logical sides
    /// and stay. A multi-value side shorthand fills the longhands it would set (a
    /// longhand present wins, as in [`super::value`]) before they move.
    pub(crate) fn style(self, style: &ComputedStyle) -> ComputedStyle {
        let mut out = style.clone();
        for (shorthand, longhand) in SIDE_GROUPS {
            spread_shorthand(&mut out, shorthand, longhand);
            let source: Vec<Option<String>> = SIDES
                .iter()
                .map(|side| out.remove(&longhand.replace('*', side)))
                .collect();
            for (target, side) in SIDES.iter().enumerate() {
                if let Some(value) = &source[self.sides[target]] {
                    out.set(&longhand.replace('*', side), value.clone());
                }
            }
        }
        self.relabel_radii(style, &mut out);
        if self.swap {
            for (a, b) in AXIS_PAIRS {
                let (va, vb) = (out.remove(a), out.remove(b));
                if let Some(v) = vb {
                    out.set(a, v);
                }
                if let Some(v) = va {
                    out.set(b, v);
                }
            }
        }
        out
    }
}

impl Relabel {
    /// Move `style`'s corner radii (shorthand, physical and flow-relative longhands, the
    /// latter mapped through `style`'s own writing mode and direction) into `out` as the
    /// physical longhands of the target space, their two radii trading places when the
    /// axes swap (miaskiewicz/turbo-dom#253).
    fn relabel_radii(self, style: &ComputedStyle, out: &mut ComputedStyle) {
        let corners = corner_radius_values(style);
        out.remove("border-radius");
        for name in CORNER_RADIUS_LONGHANDS
            .iter()
            .chain(&LOGICAL_RADIUS_LONGHANDS)
        {
            out.remove(name);
        }
        for (target, (edge, side)) in CORNER_SIDES.into_iter().enumerate() {
            let source = corner_at(self.sides[edge], self.sides[side]);
            if let Some(value) = &corners[source] {
                out.set(CORNER_RADIUS_LONGHANDS[target], self.axes(value));
            }
        }
    }

    /// A `<h> [<v>]` corner radius in the target space's axes.
    fn axes(self, value: &str) -> String {
        match css_value_tokens(value).as_slice() {
            [h, v] if self.swap => format!("{v} {h}"),
            _ => value.to_string(),
        }
    }

    /// `radii` (physical corners of the source space) in the target space.
    fn radii(self, radii: BorderRadii) -> BorderRadii {
        let corners = radii.corners();
        BorderRadii::from_corners(std::array::from_fn(|target| {
            let (edge, side) = CORNER_SIDES[target];
            let c = corners[corner_at(self.sides[edge], self.sides[side])];
            if self.swap {
                CornerRadii { h: c.v, v: c.h }
            } else {
                c
            }
        }))
    }

    /// `border` (sides of the source space) in the target space.
    fn border(self, border: BorderEdges) -> BorderEdges {
        let source = [border.top, border.right, border.bottom, border.left];
        BorderEdges {
            top: source[self.sides[0]],
            right: source[self.sides[1]],
            bottom: source[self.sides[2]],
            left: source[self.sides[3]],
        }
    }
}

/// The sides (`top` / `bottom`, `left` / `right` in [`SIDES`] order) at each corner, in
/// CSS corner order.
const CORNER_SIDES: [(usize, usize); 4] = [(0, 3), (0, 1), (2, 1), (2, 3)];

/// Side groups: the multi-value shorthand (if any) and the longhand pattern.
const SIDE_GROUPS: [(&str, &str); 7] = [
    ("margin", "margin-*"),
    ("padding", "padding-*"),
    ("inset", "*"),
    ("border-width", "border-*-width"),
    ("border-style", "border-*-style"),
    ("border-color", "border-*-color"),
    ("", "border-*"),
];

/// Properties that trade places when the axes swap.
const AXIS_PAIRS: [(&str, &str); 4] = [
    ("width", "height"),
    ("min-width", "min-height"),
    ("max-width", "max-height"),
    ("overflow-x", "overflow-y"),
];

/// Replace a 2–4 value side shorthand by the longhands it sets (those not already
/// present). A single value names every side alike and stays.
fn spread_shorthand(style: &mut ComputedStyle, shorthand: &str, longhand: &str) {
    if shorthand.is_empty() {
        return;
    }
    let Some(value) = style.get(shorthand).map(str::to_string) else {
        return;
    };
    let Some(expanded) = crate::style::expand_quad(shorthand, &value) else {
        return;
    };
    if expanded.iter().all(|(_, v)| *v == expanded[0].1) {
        return;
    }
    style.remove(shorthand);
    for (side, (_, v)) in SIDES.iter().zip(expanded) {
        let name = longhand.replace('*', side);
        if style.get(&name).is_none() {
            style.set(&name, v.to_string());
        }
    }
}

/// A fragment rectangle `(x, y, width, height)`.
type Rect = (f32, f32, f32, f32);

/// `rect` of a box laid out in the space of `mode` (relative to the root's origin, the
/// root's block size being `block`) in physical coordinates relative to the root's
/// physical top-left.
fn to_physical(mode: WritingMode, (x, y, w, h): Rect, block: f32) -> Rect {
    match mode {
        WritingMode::HorizontalTb => (x, y, w, h),
        WritingMode::VerticalLr => (y, x, h, w),
        WritingMode::VerticalRl => (block - y - h, x, h, w),
    }
}

/// The inverse of [`to_physical`] for the parent space of `mode`, local to a root
/// whose physical width is `width`.
fn from_physical(mode: WritingMode, (x, y, w, h): Rect, width: f32) -> Rect {
    match mode {
        WritingMode::HorizontalTb => (x, y, w, h),
        WritingMode::VerticalLr => (y, x, h, w),
        WritingMode::VerticalRl => (y, width - x - w, h, w),
    }
}

/// Map a root's fragment tree (laid out at the origin of the space of `mode`) into
/// the space of `parent`, local to the root's top-left there.
fn map_tree(frag: &mut Fragment, mode: WritingMode, parent: WritingMode) {
    let block = frag.height;
    walk(frag, &mut |f| {
        let clip = overflow_rect(f);
        set_rect(f, to_physical(mode, rect(f), block), mode);
        set_overflow_rect(f, clip.map(|c| to_physical(mode, c, block)));
    });
    let width = frag.width;
    walk(frag, &mut |f| {
        let clip = overflow_rect(f);
        set_rect(f, from_physical(parent, rect(f), width), parent);
        set_overflow_rect(f, clip.map(|c| from_physical(parent, c, width)));
    });
    let relabel = Relabel::between(mode, parent);
    walk(frag, &mut |f| {
        if let FragmentContent::Box {
            border,
            border_radius,
            ..
        } = &mut f.content
        {
            *border = relabel.border(*border);
            *border_radius = relabel.radii(*border_radius);
        }
        if let Some(clip) = &mut f.overflow_clip {
            clip.radii = relabel.radii(clip.radii);
        }
    });
}

/// `f`'s overflow clip region as an absolute rectangle.
fn overflow_rect(f: &Fragment) -> Option<Rect> {
    f.overflow_clip
        .map(|c| (f.x + c.x, f.y + c.y, c.width, c.height))
}

/// Set `f`'s overflow clip region from an absolute rectangle (`f` already moved).
fn set_overflow_rect(f: &mut Fragment, r: Option<Rect>) {
    if let (Some(clip), Some((x, y, w, h))) = (&mut f.overflow_clip, r) {
        (clip.x, clip.y, clip.width, clip.height) = (x - f.x, y - f.y, w, h);
    }
}

fn rect(f: &Fragment) -> Rect {
    (f.x, f.y, f.width, f.height)
}

/// Move `f` to `r`; a step through a vertical space turns its glyph runs a quarter.
fn set_rect(f: &mut Fragment, r: Rect, mode: WritingMode) {
    (f.x, f.y, f.width, f.height) = r;
    if let (true, FragmentContent::TextLine { vertical, .. }) = (mode.vertical(), &mut f.content) {
        *vertical = !*vertical;
    }
}

fn walk(frag: &mut Fragment, f: &mut impl FnMut(&mut Fragment)) {
    f(frag);
    for child in &mut frag.children {
        walk(child, f);
    }
}

/// Used box metrics recorded in the space of `mode`, in the space of `parent`.
fn map_metrics(metrics: &mut UsedBoxMetrics, mode: WritingMode, parent: WritingMode) {
    let relabel = Relabel::between(mode, parent);
    metrics.border = relabel.edges(metrics.border);
    metrics.padding = relabel.edges(metrics.padding);
    metrics.margin = relabel.edges(metrics.margin);
    if relabel.swap {
        std::mem::swap(&mut metrics.content_width, &mut metrics.content_height);
    }
}

/// The context a writing-mode root swaps out while its subtree lays out in its own
/// space.
struct Saved {
    writing: WritingMode,
    floats: Vec<super::block::FloatRect>,
    metrics: Option<HashMap<NodeId, UsedBoxMetrics>>,
    cb: (f32, bool),
}

/// Lay out `lb` when it is a writing-mode root (its writing mode differs from the
/// current space's): its border box at `(bx, by)` in the current space, `cb_width` the
/// current containing block's inline size. `None` for any other box.
pub(crate) fn layout_root(
    lb: &super::boxgen::LayoutBox,
    origin: (f32, f32),
    cb_width: f32,
    parent_fs: f32,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    layout_root_in(lb, origin, (cb_width, None), parent_fs, ctx)
}

/// [`layout_root`] for a root whose border-box width (in the current space) is
/// already decided: a flex or grid item or a table cell (`layout_box_sized`). The
/// width is the root's block size when it is orthogonal, else its inline size.
pub(crate) fn layout_root_sized(
    lb: &super::boxgen::LayoutBox,
    origin: (f32, f32),
    bbw: f32,
    parent_fs: f32,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    layout_root_in(lb, origin, (bbw, Some(bbw)), parent_fs, ctx)
}

fn layout_root_in(
    lb: &super::boxgen::LayoutBox,
    (bx, by): (f32, f32),
    (cb_width, sized): (f32, Option<f32>),
    parent_fs: f32,
    ctx: &mut Ctx,
) -> Option<Fragment> {
    let mode = WritingMode::of(&lb.style);
    let parent = ctx.writing;
    if !lb.writing_root || mode == parent {
        return None;
    }
    crate::hot!("layout.writing.root");
    let relabel = Relabel::between(parent, mode);
    let mut root = lb.relabelled(relabel);
    // In its own space the root is no orthogonal flow (its intrinsic widths measure
    // its content there, not through [`orthogonal_extent`]).
    root.orthogonal_root = false;
    if let Some(bbw) = sized {
        let axis = if relabel.swap { "height" } else { "width" };
        root.style.set(axis, format!("{bbw}px"));
        root.style.set("box-sizing", "border-box".to_string());
        for limit in ["min-", "max-"] {
            root.style.remove(&format!("{limit}{axis}"));
        }
    }
    let orthogonal = relabel.swap;
    let available = if orthogonal {
        orthogonal_available(parent, ctx)
    } else {
        cb_width
    };
    let saved = Saved {
        writing: std::mem::replace(&mut ctx.writing, mode),
        floats: std::mem::take(&mut ctx.floats),
        metrics: ctx.used_metrics.as_mut().map(std::mem::take),
        cb: (ctx.cb_h, ctx.cb_h_known),
    };
    if orthogonal {
        // The root's block size resolves percentages against the parent's inline size.
        (ctx.cb_h, ctx.cb_h_known) = (cb_width, true);
    }
    let mut frag = super::block::layout_root_box(&root, available, parent_fs, orthogonal, ctx);
    ctx.writing = saved.writing;
    ctx.floats = saved.floats;
    (ctx.cb_h, ctx.cb_h_known) = saved.cb;
    if let (Some(inner), Some(mut outer)) = (ctx.used_metrics.take(), saved.metrics) {
        for (id, mut used) in inner {
            map_metrics(&mut used, mode, parent);
            outer.insert(id, used);
        }
        ctx.used_metrics = Some(outer);
    }
    map_tree(&mut frag, mode, parent);
    frag.translate(bx, by);
    Some(frag)
}

/// An orthogonal root's border-box width in its parent's space when its size is not
/// given: laid out on its own (a scratch layout at the viewport's available size), its
/// block size fits its content. The max- and min-content width a flex / grid / table
/// container sizes it by (CSS Writing Modes 4 §7.3; miaskiewicz/turbo-dom#239).
pub(crate) fn orthogonal_extent(lb: &super::boxgen::LayoutBox, fonts: &crate::FontRegistry) -> f32 {
    let inputs = super::boxgen::measure_inputs(lb, fonts);
    let kind = super::boxgen::MemoKind::OrthogonalExtent;
    lb.relabel_memo
        .get_or(kind, inputs, || (measure_orthogonal_extent(lb, fonts), 0.0))
        .0
}

/// [`orthogonal_extent`], uncached.
fn measure_orthogonal_extent(lb: &super::boxgen::LayoutBox, fonts: &crate::FontRegistry) -> f32 {
    crate::hot!("layout.writing.extent");
    let images = crate::ImageCtx {
        resolver: &crate::NoImages,
        body_height: None,
    };
    let mut diags = crate::Diagnostics::default();
    let (width, height) = crate::style::viewport_px();
    let parent = if WritingMode::of(&lb.style).vertical() {
        WritingMode::HorizontalTb
    } else {
        WritingMode::VerticalRl
    };
    let (html_font_metrics, html_font_scale) = lb.html_layout_policy_pair();
    let mut ctx = Ctx {
        fonts,
        images: &images,
        diags: &mut diags,
        abs_cb_x: 0.0,
        abs_cb_y: 0.0,
        abs_cb_w: width,
        abs_cb_h: 0.0,
        abs_cb_h_known: false,
        cb_h: 0.0,
        cb_h_known: false,
        root_w: width,
        root_h: height,
        icb_h: height,
        abs_cb_icb: false,
        live_geometry: html_font_metrics,
        used_metrics: None,
        text_capture: None,
        html_font_metrics,
        html_font_scale,
        original_units: None,
        native_transforms: None,
        floats: Vec::new(),
        table_rows: None,
        dropped_end_margins: std::collections::HashSet::new(),
        sticky_pending: Vec::new(),
        writing: parent,
    };
    let available = if parent.vertical() { height } else { width };
    layout_root(
        lb,
        (0.0, 0.0),
        available,
        super::value::DEFAULT_FONT_SIZE,
        &mut ctx,
    )
    .map_or(0.0, |frag| frag.width)
}

/// The space available to an orthogonal root's inline size: the containing block's
/// definite size along it, else the initial containing block's (CSS Writing Modes 4
/// §7.3).
fn orthogonal_available(parent: WritingMode, ctx: &Ctx) -> f32 {
    if ctx.cb_h_known && ctx.cb_h > 0.0 && ctx.cb_h < ctx.root_h.max(ctx.root_w) {
        return ctx.cb_h;
    }
    if parent.vertical() {
        ctx.root_w
    } else if ctx.icb_h > 0.0 {
        ctx.icb_h
    } else {
        ctx.root_h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout_boxes, Diagnostics};

    fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
        ComputedStyle::from_pairs(pairs.iter().copied())
    }

    #[test]
    fn writing_mode_keywords() {
        for (value, mode) in [
            ("vertical-rl", WritingMode::VerticalRl),
            ("sideways-rl", WritingMode::VerticalRl),
            ("tb-rl", WritingMode::VerticalRl),
            ("tb", WritingMode::VerticalRl),
            ("vertical-lr", WritingMode::VerticalLr),
            ("sideways-lr", WritingMode::VerticalLr),
            ("horizontal-tb", WritingMode::HorizontalTb),
            ("lr", WritingMode::HorizontalTb),
        ] {
            assert_eq!(
                WritingMode::of(&style(&[("writing-mode", value)])),
                mode,
                "{value}"
            );
        }
        assert_eq!(WritingMode::of(&style(&[])), WritingMode::HorizontalTb);
    }

    /// Physical sides name their logical counterparts: in vertical-rl the block-start
    /// side is the right one and the inline-start side the top one; in vertical-lr the
    /// block-start side is the left one. Sizes swap; logical properties stay.
    #[test]
    fn relabel_sides_and_sizes() {
        let source = style(&[
            ("margin", "1px 2px 3px 4px"),
            ("margin-left", "9px"),
            ("padding", "5px"),
            ("border-right", "1px solid red"),
            ("border-width", "1px 2px"),
            ("top", "7px"),
            ("width", "10px"),
            ("min-height", "8px"),
            ("overflow-x", "hidden"),
            ("margin-inline-start", "6px"),
        ]);
        let rl =
            Relabel::between(WritingMode::HorizontalTb, WritingMode::VerticalRl).style(&source);
        for (name, value) in [
            ("margin-top", Some("2px")),
            ("margin-right", Some("3px")),
            ("margin-bottom", Some("9px")),
            ("margin-left", Some("1px")),
            ("margin", None),
            ("padding", Some("5px")),
            ("border-top", Some("1px solid red")),
            ("border-top-width", Some("2px")),
            ("border-left-width", Some("1px")),
            ("left", Some("7px")),
            ("height", Some("10px")),
            ("width", None),
            ("min-width", Some("8px")),
            ("overflow-y", Some("hidden")),
            ("margin-inline-start", Some("6px")),
        ] {
            assert_eq!(rl.get(name), value, "vertical-rl {name}");
        }
        let lr =
            Relabel::between(WritingMode::HorizontalTb, WritingMode::VerticalLr).style(&source);
        assert_eq!(lr.get("margin-top"), Some("9px"));
        assert_eq!(lr.get("margin-bottom"), Some("2px"));
        // Back again, and between the two vertical modes (no axis swap).
        let back = Relabel::between(WritingMode::VerticalRl, WritingMode::HorizontalTb).style(&rl);
        assert_eq!(back.get("margin-left"), Some("9px"));
        assert_eq!(back.get("width"), Some("10px"));
        let flipped = Relabel::between(WritingMode::VerticalRl, WritingMode::VerticalLr).style(&rl);
        assert_eq!(flipped.get("margin-top"), Some("9px"));
        assert_eq!(flipped.get("height"), Some("10px"));
        // An invalid shorthand stays as it is.
        let odd = style(&[("margin", "1px 2px 3px 4px 5px")]);
        let kept = Relabel::between(WritingMode::HorizontalTb, WritingMode::VerticalRl).style(&odd);
        assert_eq!(kept.get("margin"), Some("1px 2px 3px 4px 5px"));
    }

    #[test]
    fn used_metrics_map_to_the_parent_space() {
        let edges = |t, r, b, l| Edges {
            top: t,
            right: r,
            bottom: b,
            left: l,
        };
        let mut used = UsedBoxMetrics {
            inline_fragment: false,
            border: edges(1.0, 2.0, 3.0, 4.0),
            padding: edges(0.0, 0.0, 0.0, 0.0),
            margin: edges(5.0, 6.0, 7.0, 8.0),
            content_width: 10.0,
            content_height: 20.0,
            position: super::super::value::Position::Static,
            display: super::super::value::Display::Block,
        };
        map_metrics(
            &mut used,
            WritingMode::VerticalRl,
            WritingMode::HorizontalTb,
        );
        // Logical top (block-start) is physical right, logical left (inline-start) top.
        assert_eq!(used.border, edges(4.0, 1.0, 2.0, 3.0));
        assert_eq!(used.margin, edges(8.0, 5.0, 6.0, 7.0));
        assert_eq!((used.content_width, used.content_height), (20.0, 10.0));
    }

    fn boxes(html: &str, viewport: (f32, f32)) -> Vec<crate::CidBox> {
        let mut diags = Diagnostics::default();
        layout_boxes(
            html,
            "",
            viewport.0,
            viewport.1,
            &crate::test_support::fixture_fonts(),
            &mut diags,
        )
        .unwrap()
    }

    fn rect(boxes: &[crate::CidBox], cid: &str) -> (f32, f32, f32, f32) {
        let b = boxes.iter().find(|b| b.cid == cid).unwrap();
        (b.x, b.y, b.width, b.height)
    }

    /// An orthogonal root's auto inline size fits its content within the containing
    /// block's definite size, else the initial containing block's: here the viewport
    /// height (static path) wraps the text, a definite parent height narrows it more.
    #[test]
    fn orthogonal_roots_fit_the_available_size() {
        let html = "<body style='margin:0;font:16px/20px monospace'>\
            <div data-cid=v style='writing-mode:vertical-rl'>aaaa bbbb cccc</div>\
            <div style='height:50px'><div data-cid=w style='writing-mode:vertical-lr'>aaaa bbbb</div></div>\
            <div data-cid=h style='writing-mode:vertical-rl;height:60px'><div data-cid=k style='writing-mode:horizontal-tb'>abc</div></div>";
        let laid = boxes(html, (800.0, 70.0));
        assert_eq!(
            rect(&laid, "v").2,
            60.0,
            "three lines in the 70px viewport height"
        );
        assert_eq!(rect(&laid, "w").2, 40.0, "two lines in the 50px parent");
        let h = rect(&laid, "h");
        let k = rect(&laid, "k");
        // A horizontal box in a vertical-rl parent fits its text, and sits at the
        // parent's block-start (right) edge; the parent's block size is its width.
        assert!(k.2 > 20.0 && k.2 < 40.0, "{k:?}");
        assert_eq!((h.2, h.3), (k.2, 60.0));
        assert_eq!((k.0 + k.2, k.1, k.3), (h.0 + h.2, h.1, 20.0));
    }

    /// Glyph runs laid out in a vertical space are flagged to paint turned; a horizontal
    /// box inside keeps its runs upright.
    #[test]
    fn vertical_text_lines_are_flagged() {
        fn collect(f: &Fragment, out: &mut Vec<(String, bool)>) {
            if let FragmentContent::TextLine { text, vertical, .. } = &f.content {
                out.push((text.clone(), *vertical));
            }
            for c in &f.children {
                collect(c, out);
            }
        }
        let mut diags = Diagnostics::default();
        let galley = crate::layout_html(
            "<body style='margin:0'><div style='writing-mode:vertical-rl;height:80px'>ab<div style='writing-mode:horizontal-tb'>cd</div></div>",
            "",
            400.0,
            &crate::test_support::fixture_fonts(),
            &mut diags,
        )
        .unwrap();
        let mut flags = Vec::new();
        collect(&galley, &mut flags);
        assert_eq!(flags, [("ab".to_string(), true), ("cd".to_string(), false)]);
    }

    /// Every box kind relabels (the unsupported ones still lay out, and their reads
    /// fail per element in a live snapshot).
    #[test]
    fn subtrees_of_every_kind_relabel() {
        let html = "<body style='margin:0'><div data-cid=v style='writing-mode:vertical-lr;height:200px'>\
            <ul><li>item</li></ul><ul style='list-style-position:inside'><li>in</li></ul><div style='display:flex'><i>f</i></div><div style='display:grid'><i>g</i></div>\
            <table><tr><td>t</td></tr></table><p>a<br><span style='padding:1px'>b</span><b style='display:inline-block'>c</b></p></div>";
        let laid = boxes(html, (800.0, 600.0));
        assert_eq!(rect(&laid, "v").3, 200.0);
        assert!(rect(&laid, "v").2 > 0.0);
    }

    /// A consumer marker element (html2pdf's directives) relabels as an opaque box.
    #[test]
    fn consumer_markers_relabel() {
        fn markers(b: &super::super::boxgen::LayoutBox) -> usize {
            use super::super::boxgen::BoxKind;
            match &b.kind {
                BoxKind::Marker(kind) => usize::from(kind.id == 7),
                BoxKind::Block(kids)
                | BoxKind::Flex(kids)
                | BoxKind::Grid(kids)
                | BoxKind::Table(kids) => kids.iter().map(markers).sum(),
                BoxKind::Lines(_) => 0,
            }
        }
        use crate::{StyledElement, StyledNode, Tag};
        let element = |tag, children| StyledElement {
            tag,
            attrs: Vec::new(),
            style: ComputedStyle::default(),
            children,
            source: None,
        };
        let marker = element(
            Tag::Marker(crate::MarkerKind {
                id: 7,
                inline: false,
            }),
            Vec::new(),
        );
        let root = super::super::boxgen::build_box_tree(&[StyledNode::Element(element(
            Tag::Html("div".into()),
            vec![StyledNode::Element(marker)],
        ))]);
        let turned = root.relabelled(Relabel::between(
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
        ));
        assert_eq!(markers(&turned), 1);
    }

    /// Every box of a vertical layout lays out: upright text, img / svg / iframe, canvas
    /// (#252) and form controls (#239) included.
    #[test]
    fn vertical_boxes_lay_out_without_per_element_failures() {
        let (tree, snapshot, _) = crate::live::testing::layout(
            "<body><div id=v style='writing-mode:vertical-rl'>ok <span id=s>span</span>\
             <div id=f style='display:flex'>f</div><img id=i><b id=ib style='display:inline-block'>b</b>\
             <svg id=sv width=5 height=6></svg><svg id=ss style='width:5px;height:6px'></svg>\
             <svg id=sw width=5></svg><svg id=sa width=5 style='height:auto'></svg><iframe id=if></iframe><input id=in><canvas id=cv></canvas>\
             <p id=c>\u{6f22}\u{5b57}</p><p id=u style='text-orientation:upright'>up</p><p id=e> </p>\
             <span id=n style='display:none'><input></span></div>\
             <p id=h style='writing-mode:vertical-lr;text-orientation:sideways'>side</p></body>",
            "",
            (800.0, 600.0),
            Some(1.0),
        );
        let id = |selector| crate::live::testing::id(&tree, selector);
        for supported in [
            "#v", "#s", "#e", "#h", "#f", "#i", "#ib", "#sv", "#ss", "#n", "#cv", "#in",
        ] {
            assert_eq!(
                snapshot.unsupported_inline_geometry(id(supported)),
                None,
                "{supported}"
            );
        }
    }

    /// Upright clusters advance once (a combining mark adds nothing), a horizontal flex
    /// item in a vertical flex container sizes by its laid-out width, and an inside list
    /// marker relabels with its line.
    #[test]
    fn upright_clusters_orthogonal_items_and_inside_markers() {
        let mut fonts = crate::test_support::fixture_fonts();
        fonts.add(crate::test_support::ic_fixture_face());
        let (tree, snapshot, _) = crate::live::testing::layout_with_fonts(
            "<!doctype html><body style='margin:0;font:16px/20px \"Ic Fixture\"'>\
             <div style='writing-mode:vertical-rl;height:100px'><span id=c>\u{6c34}\u{301}\u{6c34}</span>\
             <div style='display:flex'><div id=h style='writing-mode:horizontal-tb'>ab</div></div>\
             <ul style='list-style-position:inside;margin:0'><li id=li>x</li></ul></div>",
            "",
            (800.0, 600.0),
            Some(1.0),
            |_| {},
            &fonts,
        );
        let bounds = |selector| {
            crate::cssom_view::metrics::element_geometry(
                &snapshot,
                crate::live::testing::id(&tree, selector),
            )
            .unwrap()
            .bounds
        };
        assert_eq!(bounds("#c").height, 32.0);
        let h = bounds("#h");
        assert!(h.width > 15.0 && h.width < 18.0, "{h:?}");
        assert_eq!(h.height, 20.0);
        assert!(bounds("#li").height > 0.0);
        // A cluster of two glyphs in one face (a base and its combining mark) stands
        // upright once: one vertical advance, its mark's glyph none.
        let html = "<!doctype html><body style='margin:0;font:16px/20px monospace'>\
             <div style='writing-mode:vertical-rl;height:100px;text-orientation:upright'>\
             <span id=m>q\u{301}q</span><span id=n>qq</span></div>";
        for scale in [Some(1.0), Some(2.0)] {
            let (tree, snapshot, _) = crate::live::testing::layout_with_fonts(
                html,
                "",
                (800.0, 600.0),
                scale,
                |_| {},
                &crate::test_support::fixture_fonts(),
            );
            let height = |selector| {
                crate::cssom_view::metrics::element_geometry(
                    &snapshot,
                    crate::live::testing::id(&tree, selector),
                )
                .unwrap()
                .bounds
                .height
            };
            assert_eq!(height("#m"), height("#n"), "{scale:?}");
        }
    }

    /// The central baseline: a vertical line's fonts and baseline-less atoms centre on
    /// it, a sideways one keeps the alphabetic baseline.
    #[test]
    fn vertical_lines_use_the_central_baseline() {
        let style = |pairs: &[(&str, &str)]| ComputedStyle::from_pairs(pairs.iter().copied());
        assert!(central_baseline(WritingMode::VerticalRl, &style(&[])));
        assert!(!central_baseline(WritingMode::HorizontalTb, &style(&[])));
        assert!(!central_baseline(
            WritingMode::VerticalLr,
            &style(&[("text-orientation", "sideways")])
        ));
        assert!(!central_baseline(
            WritingMode::VerticalRl,
            &style(&[("writing-mode", "sideways-rl")])
        ));
        // An empty inline-block centres in a 20px line (Chrome: x 4 of 12 wide); with
        // `text-orientation: sideways` it sits on the alphabetic baseline instead.
        let atom = |orientation: &str| {
            let html = format!(
                "<!doctype html><body style='margin:0;font:16px/20px monospace'>\
                 <div style='writing-mode:vertical-rl;height:100px;text-orientation:{orientation}'>\
                 a<span id=i style='display:inline-block;width:12px;height:30px'></span></div>"
            );
            let (tree, snapshot, _) = crate::live::testing::layout_with_fonts(
                &html,
                "",
                (800.0, 600.0),
                Some(1.0),
                |_| {},
                &crate::test_support::fixture_fonts(),
            );
            let id = crate::live::testing::id(&tree, "#i");
            crate::cssom_view::metrics::element_geometry(&snapshot, id)
                .unwrap()
                .bounds
                .x
        };
        assert_eq!(atom("mixed"), 4.0);
        assert_ne!(atom("sideways"), 4.0);
    }

    /// A live snapshot relabels `<br>` source breaks and text list markers too, and a
    /// vertical-lr box in a vertical-rl one is a root without an axis swap.
    #[test]
    fn live_breaks_markers_and_same_axis_roots() {
        let (tree, snapshot, _) = crate::live::testing::layout_with_fonts(
            "<body style='margin:0;font:16px/20px monospace'><div style='writing-mode:vertical-rl;height:100px'>\
             a<br>b<ol><li>x</li></ol><div id=lr style='writing-mode:vertical-lr;padding-left:3px'>ab</div></div></body>",
            "",
            (800.0, 600.0),
            Some(1.0),
            |_| {},
            &crate::test_support::fixture_fonts(),
        );
        let id = crate::live::testing::id(&tree, "#lr");
        let geometry = crate::cssom_view::metrics::element_geometry(&snapshot, id).unwrap();
        assert_eq!(
            (geometry.bounds.width, geometry.bounds.height),
            (23.0, 100.0)
        );
    }
}

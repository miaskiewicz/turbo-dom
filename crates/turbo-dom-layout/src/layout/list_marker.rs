//! List markers: the `::marker` box of a `display: list-item` element (CSS Lists 3,
//! miaskiewicz/turbo-dom#128), as Chrome 156 generates and places it
//! (`scripts/list-marker-chrome-control.mjs` -> `tests/fixtures/list_markers_chrome156.json`).
//!
//! - **Counters.** Each list item takes the next value of its list's `list-item` counter:
//!   `ol`, `ul` and `menu` start a list (`<ol start>`, `<ol reversed>` counting down from
//!   its own list items), `<li value>` sets the value, and items outside any list count in
//!   one document-wide list.
//! - **Content.** `list-style-type` picks a counter style (`decimal`,
//!   `decimal-leading-zero`, `lower`/`upper-alpha`/`-latin`, `lower`/`upper-roman`,
//!   `lower-greek`; out-of-range values and unknown names fall back to `decimal`) with the
//!   `". "` suffix, a `<string>`, `none`, or one of the `disc` / `circle` / `square` symbols,
//!   which Chrome paints as shapes sized from the font's ascent, not as glyphs.
//! - **Outside** (the default): the marker box ends at the list item's border-box start
//!   (a symbol's box sits a 7px gap further out) and its baseline is the item's first line
//!   box's baseline; an item without a line box gets the marker at its content top and is
//!   at least the marker's height. The marker belongs to the item: it is a sourceless
//!   box/text run whose snapshot owner is the item (`LayoutSnapshot::list_marker_owner`).
//! - **Inside**: a text marker is text at the start of the item's first line (collapsible
//!   like the line's own text); a symbol is an inline box 1px into the line, 1em before the
//!   content; a block first child puts the marker on a line of its own before it.
use crate::style::{ComputedStyle, StyledElement, StyledNode};
use crate::tag::Tag;
use crate::{Fragment, FragmentContent};

use super::boxgen::{BoxKind, InlineItem, LayoutBox};
use super::fragment::NodeId;
use super::value::{display_of, BorderEdges, BorderSide, BoxStyle, Display, Rgba};

/// A `disc` / `circle` / `square` marker, painted as a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Symbol {
    Disc,
    Circle,
    Square,
}

/// What a marker shows.
#[derive(Debug, Clone)]
pub(crate) enum MarkerContent {
    /// The marker text, laid out in this anonymous inline-block box.
    Text(Box<LayoutBox>),
    Symbol(Symbol),
}

/// A generated `::marker` box ([`LayoutBox::list_marker`]).
#[derive(Debug, Clone)]
pub(crate) struct ListMarker {
    /// The marker box's (sourceless) id.
    pub(crate) node_id: NodeId,
    pub(crate) style: ComputedStyle,
    pub(crate) content: MarkerContent,
}

/// The `list-item` counters of the lists being built, innermost last; the bottom entry
/// numbers the items outside any list.
#[derive(Debug)]
pub(crate) struct ListCounters(Vec<Counter>);

#[derive(Debug, Clone, Copy)]
struct Counter {
    next: i64,
    step: i64,
}

impl Default for ListCounters {
    fn default() -> Self {
        ListCounters(vec![Counter { next: 1, step: 1 }])
    }
}

impl ListCounters {
    /// Start `element`'s list if it is one (`ol`, `ul`, `menu`); whether it was.
    pub(crate) fn enter(&mut self, element: &StyledElement) -> bool {
        let Tag::Html(name) = &element.tag else {
            return false;
        };
        if !matches!(name.as_str(), "ol" | "ul" | "menu") {
            return false;
        }
        let reversed = name == "ol" && attr(element, "reversed").is_some();
        let step = if reversed { -1 } else { 1 };
        let start = (name == "ol")
            .then(|| attr(element, "start").and_then(parse_integer))
            .flatten();
        let next = start.unwrap_or_else(|| {
            if reversed {
                owned_items(&element.children)
            } else {
                1
            }
        });
        self.0.push(Counter { next, step });
        true
    }

    /// End the innermost list.
    pub(crate) fn leave(&mut self) {
        self.0.pop();
    }

    /// The counter value of the list item `element` (its `value`, else the next one).
    pub(crate) fn ordinal(&mut self, element: &StyledElement) -> i64 {
        let counter = self.0.last_mut().expect("the document-wide list");
        let value = attr(element, "value")
            .and_then(parse_integer)
            .unwrap_or(counter.next);
        // Chrome counts in a 32-bit int with ClampAdd: the ordinal stops at i32::MAX / MIN.
        counter.next = value
            .saturating_add(counter.step)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX));
        value
    }
}

fn attr<'a>(element: &'a StyledElement, name: &str) -> Option<&'a str> {
    element
        .attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .map(|a| a.value.as_str())
}

/// HTML's rules for parsing integers: optional leading whitespace and sign, then digits
/// (anything after them is ignored); a value outside a signed 32-bit integer is none,
/// as in Chrome.
fn parse_integer(text: &str) -> Option<i64> {
    let text = text.trim_start_matches([' ', '\t', '\n', '\x0c', '\r']);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    let signed = if negative {
        format!("-{}", &digits[..end])
    } else {
        digits[..end].to_string()
    };
    signed.parse::<i32>().ok().map(i64::from)
}

/// The list items a reversed list numbers: its descendant list items, not those of a
/// nested list. An explicit stack, so a deeply nested page cannot overflow the call
/// stack.
fn owned_items(children: &[StyledNode]) -> i64 {
    let mut stack: Vec<&StyledNode> = children.iter().collect();
    let mut count = 0;
    while let Some(node) = stack.pop() {
        let StyledNode::Element(element) = node else {
            continue;
        };
        count += i64::from(display_of(&element.style) == Display::ListItem);
        let nested =
            matches!(&element.tag, Tag::Html(n) if matches!(n.as_str(), "ol" | "ul" | "menu"));
        if !nested {
            stack.extend(&element.children);
        }
    }
    count
}

/// What `list-style-type` shows for `ordinal`: text, a symbol, or nothing (`none`).
pub(crate) enum Content {
    Text(String),
    Symbol(Symbol),
}

pub(crate) fn content(list_style_type: Option<&str>, ordinal: i64) -> Option<Content> {
    let value = list_style_type.map_or("disc", str::trim);
    let value = if value == "initial" { "disc" } else { value };
    if let Some(text) = quoted(value) {
        return Some(Content::Text(text.to_string()));
    }
    let lower = value.to_ascii_lowercase();
    let text = match lower.as_str() {
        "none" => return None,
        "disc" => return Some(Content::Symbol(Symbol::Disc)),
        "circle" => return Some(Content::Symbol(Symbol::Circle)),
        "square" => return Some(Content::Symbol(Symbol::Square)),
        "decimal-leading-zero" => Some(leading_zero(ordinal)),
        "lower-alpha" | "lower-latin" => alphabetic(ordinal, LATIN, false),
        "upper-alpha" | "upper-latin" => alphabetic(ordinal, LATIN, true),
        "lower-greek" => alphabetic(ordinal, GREEK, false),
        "lower-roman" => roman(ordinal, false),
        "upper-roman" => roman(ordinal, true),
        _ => None,
    };
    Some(Content::Text(format!(
        "{}. ",
        text.unwrap_or_else(|| ordinal.to_string())
    )))
}

fn quoted(value: &str) -> Option<&str> {
    let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    value
        .strip_prefix(quote)
        .and_then(|rest| rest.strip_suffix(quote))
}

fn leading_zero(ordinal: i64) -> String {
    let sign = if ordinal < 0 { "-" } else { "" };
    format!("{sign}{:02}", ordinal.unsigned_abs())
}

const LATIN: &str = "abcdefghijklmnopqrstuvwxyz";
const GREEK: &str = "αβγδεζηθικλμνξοπρστυφχψω";

/// The `alphabetic` counter system (CSS Counter Styles 3 §3.1.3): 1-based, no zero.
fn alphabetic(ordinal: i64, symbols: &str, upper: bool) -> Option<String> {
    let symbols: Vec<char> = symbols.chars().collect();
    let base = symbols.len() as u64;
    let mut n = u64::try_from(ordinal).ok().filter(|&n| n >= 1)?;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(symbols[(n % base) as usize]);
        n /= base;
    }
    let text: String = out.into_iter().rev().collect();
    Some(if upper { text.to_uppercase() } else { text })
}

/// The `additive` roman numerals, 1 to 3999.
fn roman(ordinal: i64, upper: bool) -> Option<String> {
    const PARTS: [(i64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    if !(1..=3999).contains(&ordinal) {
        return None;
    }
    let mut n = ordinal;
    let mut text = String::new();
    for (value, part) in PARTS {
        while n >= value {
            text.push_str(part);
            n -= value;
        }
    }
    Some(if upper { text.to_uppercase() } else { text })
}

/// The `::marker` style: the list item's inherited properties (`text-transform: none`).
pub(crate) fn marker_style(item: &ComputedStyle) -> ComputedStyle {
    let mut style = item.inherited_only();
    style.set("text-transform", "none".into());
    style
}

/// Whether `style` puts the marker inside the first line.
pub(crate) fn inside(style: &ComputedStyle) -> bool {
    style.get("list-style-position").map(str::trim) == Some("inside")
}

/// The anonymous inline-block box a symbol marker lays out as: inside, 1px into the line
/// and 1em before the content (Chrome 156).
pub(crate) fn symbol_box(marker: ListMarker) -> LayoutBox {
    let mut style = marker.style.clone();
    style.set("display", "inline-block".into());
    style.set("margin-left", "-1px".into());
    style.set("margin-right", "1em".into());
    let mut b = super::boxgen::anonymous_box(marker.node_id, style, BoxKind::Lines(Vec::new()));
    b.display = Display::InlineBlock;
    b.list_marker = Some(Box::new(marker));
    b
}

/// The anonymous inline-block box an outside text marker lays out in.
pub(crate) fn text_box(
    node_id: NodeId,
    text_id: NodeId,
    style: &ComputedStyle,
    text: String,
) -> LayoutBox {
    let mut style = style.clone();
    style.set("display", "inline-block".into());
    // The marker box keeps its trailing space (its width is the whole marker string).
    style.set("white-space", "pre".into());
    let item = InlineItem::Text {
        node_id: text_id,
        style: style.clone(),
        text,
    };
    let mut b = super::boxgen::anonymous_box(node_id, style, BoxKind::Lines(vec![item]));
    b.display = Display::InlineBlock;
    b
}

/// Put an inside marker's inline `item` at the start of the list item's first line: into
/// its leading inline run, else on a line of its own before its first block.
pub(crate) fn insert_inside(
    kind: &mut BoxKind,
    item: InlineItem,
    line: impl FnOnce(Vec<InlineItem>) -> LayoutBox,
) {
    match kind {
        BoxKind::Lines(items) => items.insert(0, item),
        BoxKind::Block(children) => match children.first_mut() {
            Some(first) if first.anonymous_inline_flow() => {
                if let BoxKind::Lines(items) = &mut first.kind {
                    items.insert(0, item);
                }
            }
            _ => children.insert(0, line(vec![item])),
        },
        // `display: list-item` always builds a flow (`boxgen::box_kind_for`).
        _ => unreachable!("a list item is a block container"),
    }
}

// --------------------------------------------------------------------------
// layout
// --------------------------------------------------------------------------

/// A symbol's geometry from the font's (device-pixel) ascent `a`, as Blink sizes it
/// (`ListMarker::WidthOfSymbol` / `InlineMarginsForOutside` / `RelativeSymbolMarkerRect`):
/// `o = a*2/3`, the shape is `(o+1)/2` square, the box 2px wider; outside, the box starts
/// `o + 8` before the item; the shape's top is `3*(a-o)/2` below the ascent line.
struct SymbolGeometry {
    box_width: f32,
    outside_offset: f32,
    shape: f32,
    /// The shape's top relative to the baseline.
    top: f32,
}

fn symbol_geometry(bs: &BoxStyle, ctx: &super::block::Ctx) -> Option<SymbolGeometry> {
    let families: Vec<&str> = bs.font_families.iter().map(String::as_str).collect();
    let face = ctx.fonts.select(&families, bs.font_weight, bs.italic)?;
    let scale = ctx.html_font_scale.unwrap_or(1.0);
    let a = face.html_metrics(bs.font_size * scale).ascent as i32;
    let o = a * 2 / 3;
    let shape = (o + 1) / 2;
    let px = |v: i32| v as f32 / scale;
    Some(SymbolGeometry {
        box_width: px(shape + 2),
        outside_offset: px(o + 8),
        shape: px(shape),
        top: px(3 * (a - o) / 2 - a),
    })
}

/// The shape fragment of `symbol` with its top-left at `(x, y)`.
fn shape_fragment(
    node_id: NodeId,
    symbol: Symbol,
    (x, y): (f32, f32),
    size: f32,
    color: Rgba,
) -> Fragment {
    let stroke = BorderSide {
        width: 1,
        color: Some(color),
    };
    let (rect, background, border, radius) = match symbol {
        Symbol::Disc => (
            (x, y, size),
            Some(color),
            BorderEdges::default(),
            size / 2.0,
        ),
        // A 1px stroke centred on the shape's edge.
        Symbol::Circle => (
            (x - 0.5, y - 0.5, size + 1.0),
            None,
            BorderEdges {
                top: stroke,
                right: stroke,
                bottom: stroke,
                left: stroke,
            },
            (size + 1.0) / 2.0,
        ),
        Symbol::Square => ((x, y, size), Some(color), BorderEdges::default(), 0.0),
    };
    Fragment::new(
        node_id,
        rect.0,
        rect.1,
        rect.2,
        rect.2,
        FragmentContent::Box {
            background,
            border,
            border_radius: radius.into(),
            shadow: None,
            gradient: None,
            transform: None,
        },
    )
}

fn empty_box(node_id: NodeId, x: f32, y: f32, width: f32, height: f32) -> Fragment {
    Fragment::new(
        node_id,
        x,
        y,
        width,
        height,
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

/// A symbol marker box at the origin and its baseline: one line of the marker's font tall.
fn symbol_fragment(
    marker: &ListMarker,
    symbol: Symbol,
    bs: &BoxStyle,
    ctx: &super::block::Ctx,
) -> Option<(Fragment, f32, f32)> {
    let geometry = symbol_geometry(bs, ctx)?;
    let (above, below) = super::inline_boxes::strut(bs, ctx, false)?;
    let mut frag = empty_box(marker.node_id, 0.0, 0.0, geometry.box_width, above + below);
    let at = (1.0, above + geometry.top);
    frag.children.push(shape_fragment(
        marker.node_id,
        symbol,
        at,
        geometry.shape,
        bs.color,
    ));
    Some((frag, above, geometry.outside_offset))
}

/// An inside symbol marker's inline-block fragment at the origin: as tall as the font's
/// ascent, so its bottom (its baseline) is the line's baseline.
pub(crate) fn inside_symbol_fragment(
    b: &LayoutBox,
    bs: &BoxStyle,
    ctx: &super::block::Ctx,
) -> Option<Fragment> {
    // A list item's own (outside) marker is placed with the item, not as an atom.
    let marker = b
        .list_marker
        .as_deref()
        .filter(|_| b.display != Display::ListItem)?;
    let MarkerContent::Symbol(symbol) = marker.content else {
        unreachable!("only an inside symbol marker lays out as its own box (`symbol_box`)")
    };
    let geometry = symbol_geometry(bs, ctx)?;
    let scale = ctx.html_font_scale.unwrap_or(1.0);
    let families: Vec<&str> = bs.font_families.iter().map(String::as_str).collect();
    let face = ctx.fonts.select(&families, bs.font_weight, bs.italic)?;
    let ascent = face.html_metrics(bs.font_size * scale).ascent / scale;
    let mut frag = empty_box(marker.node_id, 0.0, 0.0, geometry.box_width, ascent);
    let at = (1.0, ascent + geometry.top);
    frag.children.push(shape_fragment(
        marker.node_id,
        symbol,
        at,
        geometry.shape,
        bs.color,
    ));
    Some(frag)
}

/// The baseline of the first in-flow text line in `frags` (document order).
pub(crate) fn first_baseline(frags: &[Fragment]) -> Option<f32> {
    frags.iter().find_map(|f| match &f.content {
        FragmentContent::TextLine { glyphs, .. } => glyphs.first().map(|g| f.y + g.y),
        _ if f.is_positioned => None,
        _ => first_baseline(&f.children),
    })
}

/// Place `lb`'s outside marker beside its first line box (`children`, laid out with the
/// border box at `border_x` and the content box at `content_y`), growing `content_h` to
/// the marker when there is no line.
pub(crate) fn place_outside(
    lb: &LayoutBox,
    (border_x, content_y): (f32, f32),
    cw: f32,
    children: &mut Vec<Fragment>,
    content_h: &mut f32,
    ctx: &mut super::block::Ctx,
) {
    let Some(marker) = lb.list_marker.as_deref() else {
        return;
    };
    let bs = super::block::resolve_marker_style(&marker.style, cw);
    let placed = match &marker.content {
        MarkerContent::Text(text) => {
            let (frag, baseline) = super::block::lay_marker_text(text, cw, bs.font_size, ctx);
            let width = frag.width;
            Some((frag, baseline, width))
        }
        MarkerContent::Symbol(symbol) => symbol_fragment(marker, *symbol, &bs, ctx),
    };
    // (No marker without a font face to size it.)
    if let Some((mut frag, baseline, offset)) = placed {
        let y = if let Some(line) = first_baseline(children) {
            line - baseline
        } else {
            *content_h = content_h.max(frag.height);
            content_y
        };
        frag.translate(border_x - offset - frag.x, y - frag.y);
        children.push(frag);
    }
}

#[cfg(test)]
#[path = "list_marker_tests.rs"]
mod tests;

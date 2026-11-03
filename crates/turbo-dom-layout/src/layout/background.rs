//! Background layers placed like Chrome 156 (CSS Backgrounds 3 §3,
//! miaskiewicz/turbo-dom#248): `background-size`, `-position`, `-repeat`, `-origin` and
//! `-clip` turn one background image or gradient into tiles (`Image` / gradient `Box`
//! fragments) under one container fragment clipped to the painting area — rounded by
//! the clip box's per-corner radii (miaskiewicz/turbo-dom#253) — which the painters draw like any other fragment. Longhands
//! win over the `background` shorthand's parts. Only the tiles that meet the painting
//! area are laid out.
use super::fragment::{
    BorderRadii, ClipRect, Fragment, FragmentContent, ImagePlacement, LinearGradient, NodeId,
};
use super::value::{css_value_tokens, parse_length_pct, BorderEdges, LengthPct, Rgba};
use crate::ComputedStyle;

/// A rectangle `(x, y, width, height)` in absolute px.
pub(crate) type Rect = (f32, f32, f32, f32);

/// What one background layer paints.
#[derive(Clone)]
pub(crate) enum Source {
    Image(ImagePlacement),
    Gradient(LinearGradient),
}

/// A box area keyword of `background-origin` / `-clip`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Area {
    Border,
    Padding,
    Content,
}

/// One position axis: an offset from the start (or, `from_end`, the end) edge.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Axis {
    from_end: bool,
    offset: LengthPct,
}

/// One axis of `background-repeat`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tiling {
    Repeat,
    NoRepeat,
    Space,
    Round,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Size {
    Cover,
    Contain,
    Lengths(LengthPct, LengthPct),
}

/// The resolved background properties of a box.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Props {
    size: Size,
    position: (Axis, Axis),
    repeat: (Tiling, Tiling),
    pub(crate) origin: Area,
    pub(crate) clip: Area,
}

const START: Axis = Axis {
    from_end: false,
    offset: LengthPct::Pct(0.0),
};

impl Props {
    /// The initial values: `auto` size at `0% 0%`, repeated, origin padding box, painted
    /// over the border box.
    pub(crate) fn is_initial(&self) -> bool {
        *self
            == Props {
                size: Size::Lengths(LengthPct::Auto, LengthPct::Auto),
                position: (START, START),
                repeat: (Tiling::Repeat, Tiling::Repeat),
                origin: Area::Padding,
                clip: Area::Border,
            }
    }
}

/// The box's background properties, longhands over the shorthand's parts.
pub(crate) fn props(style: &ComputedStyle, font_size: f32) -> Props {
    let shorthand = Shorthand::parse(style.get("background").unwrap_or(""));
    let size = style
        .get("background-size")
        .or(shorthand.size.as_deref())
        .and_then(|v| parse_size(v, font_size))
        .unwrap_or(Size::Lengths(LengthPct::Auto, LengthPct::Auto));
    let position = style
        .get("background-position")
        .map(|v| {
            css_value_tokens(v)
                .into_iter()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or(shorthand.position);
    let position = parse_position(&position, font_size).unwrap_or((START, START));
    let repeat = style
        .get("background-repeat")
        .map(|v| {
            css_value_tokens(v)
                .into_iter()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or(shorthand.repeat);
    let boxes = &shorthand.boxes;
    let origin = style
        .get("background-origin")
        .and_then(area)
        .or_else(|| boxes.first().copied())
        .unwrap_or(Area::Padding);
    let clip = style
        .get("background-clip")
        .and_then(area)
        .or_else(|| boxes.get(1).or(boxes.first()).copied())
        .unwrap_or(Area::Border);
    Props {
        size,
        position,
        repeat: parse_repeat(&repeat),
        origin,
        clip,
    }
}

/// The parts of a `background` shorthand this module reads.
#[derive(Default)]
struct Shorthand {
    size: Option<String>,
    position: Vec<String>,
    repeat: Vec<String>,
    boxes: Vec<Area>,
}

impl Shorthand {
    fn parse(value: &str) -> Shorthand {
        let mut out = Shorthand::default();
        let mut after_slash = 0;
        let mut size = Vec::new();
        for token in css_value_tokens(value) {
            // A `url(...)` / function keeps its slashes.
            let parts: Vec<&str> = if token.contains('(') {
                vec![token]
            } else {
                token.split('/').collect()
            };
            for (index, part) in parts.into_iter().enumerate() {
                if index > 0 {
                    after_slash = 2;
                }
                if !part.is_empty() {
                    out.token(part, &mut after_slash, &mut size);
                }
            }
        }
        out.size = (!size.is_empty()).then(|| size.join(" "));
        out
    }

    fn token(&mut self, part: &str, after_slash: &mut usize, size: &mut Vec<String>) {
        let lower = part.to_ascii_lowercase();
        if *after_slash > 0 && sizeish(&lower) {
            *after_slash -= 1;
            size.push(lower);
            return;
        }
        *after_slash = 0;
        if let Some(area) = area(&lower) {
            self.boxes.push(area);
        } else if matches!(
            lower.as_str(),
            "repeat" | "no-repeat" | "repeat-x" | "repeat-y" | "space" | "round"
        ) {
            self.repeat.push(lower);
        } else if positionish(&lower) {
            self.position.push(lower);
        }
    }
}

fn sizeish(token: &str) -> bool {
    matches!(token, "auto" | "cover" | "contain") || lengthish(token)
}

fn positionish(token: &str) -> bool {
    matches!(token, "left" | "right" | "top" | "bottom" | "center") || lengthish(token)
}

fn lengthish(token: &str) -> bool {
    token.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        || token.starts_with("calc(")
}

fn area(value: &str) -> Option<Area> {
    match value.trim() {
        "border-box" => Some(Area::Border),
        "padding-box" => Some(Area::Padding),
        "content-box" => Some(Area::Content),
        _ => None,
    }
}

fn parse_size(value: &str, font_size: f32) -> Option<Size> {
    let tokens = css_value_tokens(value);
    match tokens.as_slice() {
        ["cover"] => Some(Size::Cover),
        ["contain"] => Some(Size::Contain),
        [w] => Some(Size::Lengths(
            parse_length_pct(w, font_size)?,
            LengthPct::Auto,
        )),
        [w, h] => Some(Size::Lengths(
            parse_length_pct(w, font_size)?,
            parse_length_pct(h, font_size)?,
        )),
        _ => None,
    }
}

fn parse_repeat(tokens: &[String]) -> (Tiling, Tiling) {
    let one = |t: &str| match t {
        "no-repeat" => Tiling::NoRepeat,
        "space" => Tiling::Space,
        "round" => Tiling::Round,
        _ => Tiling::Repeat,
    };
    match tokens {
        [x] if x == "repeat-x" => (Tiling::Repeat, Tiling::NoRepeat),
        [y] if y == "repeat-y" => (Tiling::NoRepeat, Tiling::Repeat),
        [both] => (one(both), one(both)),
        [x, y, ..] => (one(x), one(y)),
        [] => (Tiling::Repeat, Tiling::Repeat),
    }
}

fn keyword(token: &str) -> Option<(bool, Option<bool>, f32)> {
    // (horizontal?, None = either axis, percentage)
    match token {
        "left" => Some((true, Some(true), 0.0)),
        "right" => Some((true, Some(true), 100.0)),
        "top" => Some((false, Some(false), 0.0)),
        "bottom" => Some((false, Some(false), 100.0)),
        "center" => Some((true, None, 50.0)),
        _ => None,
    }
}

/// `background-position`: one or two values, or edge-offset pairs (3-4 values).
fn parse_position(tokens: &[String], font_size: f32) -> Option<(Axis, Axis)> {
    let pct = |p: f32| Axis {
        from_end: false,
        offset: LengthPct::Pct(p),
    };
    let value = |t: &str| -> Option<Axis> {
        match keyword(t) {
            Some((_, _, p)) => Some(pct(p)),
            None => Some(Axis {
                from_end: false,
                offset: parse_length_pct(t, font_size)?,
            }),
        }
    };
    let vertical = |t: &str| matches!(t, "top" | "bottom");
    let horizontal = |t: &str| matches!(t, "left" | "right");
    match tokens {
        [] => None,
        [one] if vertical(one) => Some((pct(50.0), value(one)?)),
        [one] => Some((value(one)?, pct(50.0))),
        [a, b] if vertical(a) || horizontal(b) => Some((value(b)?, value(a)?)),
        [a, b] => Some((value(a)?, value(b)?)),
        _ => edge_offsets(tokens, font_size),
    }
}

/// The 3-/4-value form: each edge keyword optionally followed by its offset.
fn edge_offsets(tokens: &[String], font_size: f32) -> Option<(Axis, Axis)> {
    let mut x = None;
    let mut y = None;
    let mut index = 0;
    while index < tokens.len() {
        let (_, axis, percent) = keyword(&tokens[index])?;
        let offset = tokens
            .get(index + 1)
            .filter(|t| keyword(t).is_none())
            .and_then(|t| parse_length_pct(t, font_size));
        index += if offset.is_some() { 2 } else { 1 };
        let value = match offset {
            Some(offset) => Axis {
                from_end: percent == 100.0,
                offset,
            },
            None => Axis {
                from_end: false,
                offset: LengthPct::Pct(percent),
            },
        };
        let horizontal = axis.unwrap_or(x.is_none());
        if horizontal {
            x = Some(value);
        } else {
            y = Some(value);
        }
    }
    let center = Axis {
        from_end: false,
        offset: LengthPct::Pct(50.0),
    };
    Some((x.unwrap_or(center), y.unwrap_or(center)))
}

/// A border box's `area`, given its border widths and padding.
pub(crate) fn area_rect(
    (x, y, w, h): Rect,
    border: &BorderEdges,
    padding: &super::value::Edges,
    which: Area,
) -> Rect {
    let b = border.widths();
    let mut rect = (x, y, w, h);
    if which != Area::Border {
        rect = (
            rect.0 + b.left,
            rect.1 + b.top,
            rect.2 - b.horizontal(),
            rect.3 - b.vertical(),
        );
    }
    if which == Area::Content {
        rect = (
            rect.0 + padding.left,
            rect.1 + padding.top,
            rect.2 - padding.horizontal(),
            rect.3 - padding.vertical(),
        );
    }
    (rect.0, rect.1, rect.2.max(0.0), rect.3.max(0.0))
}

/// The tile size of `source` over a positioning area `(width, height)`.
fn tile_size(size: Size, source: &Source, (aw, ah): (f32, f32)) -> (f32, f32) {
    let intrinsic = match source {
        Source::Image(image) => Some((image.intrinsic_w as f32, image.intrinsic_h as f32))
            .filter(|(w, h)| *w > 0.0 && *h > 0.0),
        Source::Gradient(_) => None,
    };
    match (size, intrinsic) {
        (Size::Cover | Size::Contain, None) => (aw, ah),
        (Size::Cover, Some((iw, ih))) => {
            let scale = (aw / iw).max(ah / ih);
            (iw * scale, ih * scale)
        }
        (Size::Contain, Some((iw, ih))) => {
            let scale = (aw / iw).min(ah / ih);
            (iw * scale, ih * scale)
        }
        (Size::Lengths(w, h), _) => {
            let w = w.resolve(aw);
            let h = h.resolve(ah);
            match (w, h, intrinsic) {
                (Some(w), Some(h), _) => (w, h),
                (Some(w), None, Some((iw, ih))) => (w, w * ih / iw),
                (None, Some(h), Some((iw, ih))) => (h * iw / ih, h),
                (None, None, Some(size)) => size,
                (w, h, None) => (w.unwrap_or(aw), h.unwrap_or(ah)),
            }
        }
    }
}

fn place(axis: Axis, area: f32, tile: f32) -> f32 {
    let offset = match axis.offset {
        LengthPct::Pct(p) => (area - tile) * p / 100.0,
        other => other.resolve(area - tile).unwrap_or(0.0),
    };
    if axis.from_end {
        area - tile - offset
    } else {
        offset
    }
}

/// `round`: the tile shrinks or grows so a whole number of tiles fills the area.
fn rounded(tile: f32, area: f32) -> f32 {
    let count = (area / tile).round().max(1.0);
    area / count
}

/// The tile starts along one axis, from `start` (a tile's place) across `paint`, every
/// `step` (0 = one tile only).
fn starts(start: f32, step: f32, (from, len): (f32, f32)) -> Vec<f32> {
    if step <= 0.0 {
        return vec![start];
    }
    // The last tile start at or before the painting area's start.
    let first = start - ((start - from) / step).ceil() * step;
    let count = ((from + len - first) / step).ceil().max(0.0) as usize;
    (0..count).map(|i| first + i as f32 * step).collect()
}

/// One axis's tile start and step (0: not repeated): `space` spreads the whole tiles
/// that fit from the area's edges (one or none fit: placed once, by position).
fn axis(repeat: Tiling, placed: f32, (area_from, area): (f32, f32), tile: f32) -> (f32, f32) {
    match repeat {
        Tiling::NoRepeat => (placed, 0.0),
        Tiling::Repeat | Tiling::Round => (placed, tile),
        Tiling::Space => {
            let count = (area / tile).floor();
            if count < 2.0 {
                (placed, 0.0)
            } else {
                (area_from, tile + (area - count * tile) / (count - 1.0))
            }
        }
    }
}

/// The fragments of one background layer of box `id`: `source` tiled over the
/// positioning area `origin` under a container clipped to the painting area `paint`
/// with corner `radius` (its per-corner radii).
pub(crate) fn layer(
    id: NodeId,
    props: &Props,
    source: &Source,
    (origin, paint, radius): (Rect, Rect, BorderRadii),
) -> Vec<Fragment> {
    let (mut tw, mut th) = tile_size(props.size, source, (origin.2, origin.3));
    if tw <= 0.0 || th <= 0.0 || paint.2 <= 0.0 || paint.3 <= 0.0 {
        return Vec::new();
    }
    let (round_x, round_y) = (
        props.repeat.0 == Tiling::Round,
        props.repeat.1 == Tiling::Round,
    );
    let auto = |side: LengthPct| matches!(props.size, Size::Lengths(..)) && side == LengthPct::Auto;
    let (auto_w, auto_h) = match props.size {
        Size::Lengths(w, h) => (auto(w), auto(h)),
        _ => (false, false),
    };
    if round_x {
        let w = rounded(tw, origin.2);
        if !round_y && auto_h {
            th *= w / tw;
        }
        tw = w;
    }
    if round_y {
        let h = rounded(th, origin.3);
        if !round_x && auto_w {
            tw *= h / th;
        }
        th = h;
    }
    let x = origin.0 + place(props.position.0, origin.2, tw);
    let y = origin.1 + place(props.position.1, origin.3, th);
    let (x, step_x) = axis(props.repeat.0, x, (origin.0, origin.2), tw);
    let (y, step_y) = axis(props.repeat.1, y, (origin.1, origin.3), th);
    let xs = starts(x, step_x, (paint.0, paint.2));
    let ys = starts(y, step_y, (paint.1, paint.3));
    let mut tiles = Vec::new();
    for &ty in &ys {
        for &tx in &xs {
            let outside = tx >= paint.0 + paint.2
                || ty >= paint.1 + paint.3
                || tx + tw <= paint.0
                || ty + th <= paint.1;
            if outside {
                continue;
            }
            let content = match source {
                Source::Image(image) => FragmentContent::Image(image.clone()),
                Source::Gradient(gradient) => {
                    gradient_box(None, Some(gradient.clone()), BorderRadii::ZERO)
                }
            };
            // Tiles snap to whole pixels, so neighbours meet without a seam.
            let (x0, y0) = (tx.round(), ty.round());
            let (w, h) = ((tx + tw).round() - x0, (ty + th).round() - y0);
            tiles.push(Fragment::new(id, x0, y0, w, h, content));
        }
    }
    if tiles.is_empty() {
        return Vec::new();
    }
    let mut container = Fragment::new(
        id,
        paint.0,
        paint.1,
        paint.2,
        paint.3,
        gradient_box(None, None, BorderRadii::ZERO),
    );
    container.clip = Some(ClipRect {
        x: 0.0,
        y: 0.0,
        width: paint.2,
        height: paint.3,
        radii: radius,
    });
    container.children = tiles;
    vec![container]
}

/// A background colour painted over `paint` only (a `background-clip` other than the
/// border box).
pub(crate) fn color_layer(id: NodeId, color: Rgba, paint: Rect, radius: BorderRadii) -> Fragment {
    Fragment::new(
        id,
        paint.0,
        paint.1,
        paint.2,
        paint.3,
        gradient_box(Some(color), None, radius),
    )
}

/// The corner radii of `which` area of a box with `radius` on its border box: each
/// corner's radii shrink by the adjacent sides' border (and padding) widths
/// (miaskiewicz/turbo-dom#253).
pub(crate) fn area_radius(
    radius: BorderRadii,
    border: &BorderEdges,
    padding: &super::value::Edges,
    which: Area,
) -> BorderRadii {
    let b = border.widths();
    match which {
        Area::Border => radius,
        Area::Padding => radius.inset(b.top, b.right, b.bottom, b.left),
        Area::Content => radius.inset(
            b.top + padding.top,
            b.right + padding.right,
            b.bottom + padding.bottom,
            b.left + padding.left,
        ),
    }
}

/// The box's border alone, painted over its background layers (the box keeps its
/// colour, shadow and transform).
pub(crate) fn border_overlay(
    id: NodeId,
    rect: Rect,
    border: BorderEdges,
    radius: BorderRadii,
) -> Fragment {
    let mut overlay = Fragment::new(
        id,
        rect.0,
        rect.1,
        rect.2,
        rect.3,
        gradient_box(None, None, radius),
    );
    if let FragmentContent::Box { border: own, .. } = &mut overlay.content {
        *own = border;
    }
    overlay
}

fn gradient_box(
    background: Option<Rgba>,
    gradient: Option<LinearGradient>,
    border_radius: BorderRadii,
) -> FragmentContent {
    FragmentContent::Box {
        background,
        border: BorderEdges::default(),
        border_radius,
        shadow: None,
        gradient,
        transform: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        for (property, value) in pairs {
            style.set(property, (*value).to_string());
        }
        style
    }

    #[test]
    fn the_shorthand_parts_and_longhands_resolve() {
        let props = props(
            &style(&[(
                "background",
                "red url(a.png) repeat-x right 10px bottom/40px auto content-box padding-box",
            )]),
            16.0,
        );
        assert_eq!(
            props.size,
            Size::Lengths(LengthPct::Px(40.0), LengthPct::Auto)
        );
        assert_eq!(props.repeat, (Tiling::Repeat, Tiling::NoRepeat));
        assert_eq!((props.origin, props.clip), (Area::Content, Area::Padding));
        assert_eq!(
            props.position.0,
            Axis {
                from_end: true,
                offset: LengthPct::Px(10.0)
            }
        );
        assert_eq!(
            props.position.1,
            Axis {
                from_end: false,
                offset: LengthPct::Pct(100.0)
            }
        );
        let props = props_of(&[
            ("background-position", "top"),
            ("background-repeat", "repeat-y"),
        ]);
        assert_eq!(props.position.1, START);
        assert_eq!(props.position.0.offset, LengthPct::Pct(50.0));
        assert_eq!(props.repeat, (Tiling::NoRepeat, Tiling::Repeat));
        assert!(props_of(&[]).is_initial());
        let props = props_of(&[
            ("background-repeat", "no-repeat repeat"),
            ("background-origin", "border-box"),
            ("background-clip", "content-box"),
            ("background-size", "cover"),
        ]);
        assert_eq!(props.repeat, (Tiling::NoRepeat, Tiling::Repeat));
        assert_eq!(
            (props.origin, props.clip, props.size),
            (Area::Border, Area::Content, Size::Cover)
        );
        assert_eq!(props_of(&[("background", "border-box")]).clip, Area::Border);
        assert_eq!(
            props_of(&[("background-size", "1px 2px 3px")]).size,
            Size::Lengths(LengthPct::Auto, LengthPct::Auto)
        );
        assert_eq!(
            props_of(&[("background-size", "contain")]).size,
            Size::Contain
        );
        assert_eq!(
            props_of(&[("background-position", "10px left")])
                .position
                .0
                .offset,
            LengthPct::Pct(0.0)
        );
        assert_eq!(
            props_of(&[("background-position", "center top 5px")]).position,
            (
                Axis {
                    from_end: false,
                    offset: LengthPct::Pct(50.0)
                },
                Axis {
                    from_end: false,
                    offset: LengthPct::Px(5.0)
                }
            )
        );
        assert_eq!(
            props_of(&[("background-position", "left bogus 1px")]).position,
            (START, START)
        );
        assert_eq!(
            props_of(&[("background-position", "bogus")]).position,
            (START, START)
        );
    }

    fn props_of(pairs: &[(&str, &str)]) -> Props {
        props(&style(pairs), 16.0)
    }

    #[test]
    fn gradients_fill_the_area_unless_sized() {
        let gradient = Source::Gradient(LinearGradient {
            angle_deg: 90.0,
            stops: Vec::new(),
        });
        assert_eq!(
            tile_size(Size::Cover, &gradient, (10.0, 20.0)),
            (10.0, 20.0)
        );
        assert_eq!(
            tile_size(
                Size::Lengths(LengthPct::Px(4.0), LengthPct::Auto),
                &gradient,
                (10.0, 20.0)
            ),
            (4.0, 20.0)
        );
        let props = props_of(&[("background-size", "0px 4px")]);
        let square = (0.0, 0.0, 10.0, 10.0);
        assert!(layer(
            NodeId(1),
            &props,
            &gradient,
            (square, square, BorderRadii::ZERO)
        )
        .is_empty());
        // Only the tiles meeting the painting area: 1px tiles over a 4x3 window of a
        // 100px area, positioned off the pixel grid.
        let props = props_of(&[
            ("background-size", "1px 1px"),
            ("background-position", "0.5px 0.5px"),
        ]);
        let layers = layer(
            NodeId(1),
            &props,
            &gradient,
            ((0.0, 0.0, 100.0, 100.0), (10.0, 20.0, 4.0, 3.0), 2.0.into()),
        );
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].clip.unwrap().radii, BorderRadii::uniform(2.0));
        assert_eq!(layers[0].children.len(), 5 * 4);
        assert_eq!(
            (layers[0].children[0].x, layers[0].children[0].y),
            (10.0, 20.0)
        );
        // A tile placed wholly outside a no-repeat painting area paints nothing.
        let props = props_of(&[
            ("background-size", "1px 1px"),
            ("background-repeat", "no-repeat"),
            ("background-position", "50px 50px"),
        ]);
        assert!(layer(
            NodeId(1),
            &props,
            &gradient,
            (square, square, BorderRadii::ZERO)
        )
        .is_empty());
        // `space` with fewer than two whole tiles places one by position; `round` with
        // an auto side keeps the ratio.
        let props = props_of(&[
            ("background-size", "8px 4px"),
            ("background-repeat", "space"),
            ("background-position", "1px 2px"),
        ]);
        let tiles = &layer(
            NodeId(1),
            &props,
            &gradient,
            (square, square, BorderRadii::ZERO),
        )[0]
        .children;
        assert_eq!((tiles.len(), tiles[0].x, tiles[0].y), (2, 1.0, 0.0));
        let image = Source::Image(ImagePlacement {
            name: "a".into(),
            intrinsic_w: 4,
            intrinsic_h: 2,
            has_alpha: false,
            tint: None,
            orientation: crate::image::Orientation::IDENTITY,
        });
        let big = (0.0, 0.0, 120.0, 120.0);
        let props = props_of(&[
            ("background-size", "35px auto"),
            ("background-repeat", "round no-repeat"),
        ]);
        let tile = &layer(NodeId(1), &props, &image, (big, big, BorderRadii::ZERO))[0].children[0];
        assert_eq!((tile.width, tile.height), (40.0, 20.0));
        let props = props_of(&[
            ("background-size", "auto 35px"),
            ("background-repeat", "no-repeat round"),
        ]);
        let tile = &layer(NodeId(1), &props, &image, (big, big, BorderRadii::ZERO))[0].children[0];
        assert_eq!((tile.width, tile.height), (80.0, 40.0));
    }

    #[test]
    fn areas_radii_and_placement_follow_the_box_model() {
        let side = super::super::value::BorderSide {
            width: 2,
            color: None,
        };
        let border = BorderEdges {
            left: side,
            top: side,
            ..BorderEdges::default()
        };
        let padding = super::super::value::Edges {
            left: 3.0,
            top: 3.0,
            right: 1.0,
            bottom: 1.0,
        };
        let rect = (0.0, 0.0, 20.0, 20.0);
        assert_eq!(
            area_rect(rect, &border, &padding, Area::Content),
            (5.0, 5.0, 14.0, 14.0)
        );
        let c = |h, v| super::super::fragment::CornerRadii { h, v };
        let ten = BorderRadii::uniform(10.0);
        assert_eq!(area_radius(ten, &border, &padding, Area::Border), ten);
        // Each corner shrinks by its own adjacent sides (miaskiewicz/turbo-dom#253).
        assert_eq!(
            area_radius(ten, &border, &padding, Area::Padding),
            BorderRadii::from_corners([c(8.0, 8.0), c(10.0, 8.0), c(10.0, 10.0), c(8.0, 10.0)])
        );
        assert_eq!(
            area_radius(BorderRadii::uniform(4.0), &border, &padding, Area::Content),
            BorderRadii::from_corners([c(0.0, 0.0), c(3.0, 0.0), c(3.0, 3.0), c(0.0, 3.0)])
        );
        let red = Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let colour = color_layer(NodeId(3), red, rect, 6.0.into());
        assert!(matches!(
            colour.content,
            FragmentContent::Box { border_radius, background: Some(_), .. } if border_radius == BorderRadii::uniform(6.0)
        ));
        let from_end = Axis {
            from_end: true,
            offset: LengthPct::Px(10.0),
        };
        assert_eq!(place(from_end, 100.0, 20.0), 70.0);
        // A repeated tile placed past the painting area's start steps back over it.
        assert_eq!(starts(15.0, 10.0, (0.0, 20.0)), vec![-5.0, 5.0, 15.0]);
        assert_eq!(starts(10.0, 10.0, (0.0, 20.0)), vec![0.0, 10.0]);
        assert_eq!(
            props_of(&[("background-size", "10px")]).size,
            Size::Lengths(LengthPct::Px(10.0), LengthPct::Auto)
        );
        assert_eq!(
            props_of(&[("background-size", "bogus")]).size,
            Size::Lengths(LengthPct::Auto, LengthPct::Auto)
        );
    }

    #[test]
    fn cover_and_contain_scale_the_image_and_round_refits_them() {
        let image = Source::Image(ImagePlacement {
            name: "a".into(),
            intrinsic_w: 100,
            intrinsic_h: 40,
            has_alpha: false,
            tint: None,
            orientation: crate::image::Orientation::IDENTITY,
        });
        assert_eq!(
            tile_size(Size::Cover, &image, (200.0, 100.0)),
            (250.0, 100.0)
        );
        assert_eq!(
            tile_size(Size::Contain, &image, (200.0, 100.0)),
            (200.0, 80.0)
        );
        let props = props_of(&[
            ("background-size", "contain"),
            ("background-repeat", "round"),
        ]);
        let area = (0.0, 0.0, 300.0, 100.0);
        let tiles = &layer(NodeId(1), &props, &image, (area, area, BorderRadii::ZERO))[0].children;
        // A 250x100 contain tile rounds to 300 / 1 and 100 / 1.
        assert_eq!((tiles[0].width, tiles[0].height), (300.0, 100.0));
    }

    #[test]
    fn a_tile_with_one_auto_side_keeps_the_image_ratio() {
        let image = Source::Image(ImagePlacement {
            name: "a".into(),
            intrinsic_w: 100,
            intrinsic_h: 40,
            has_alpha: false,
            tint: None,
            orientation: crate::image::Orientation::IDENTITY,
        });
        assert_eq!(
            tile_size(
                Size::Lengths(LengthPct::Auto, LengthPct::Px(20.0)),
                &image,
                (1.0, 1.0)
            ),
            (50.0, 20.0)
        );
        assert_eq!(
            tile_size(
                Size::Lengths(LengthPct::Auto, LengthPct::Auto),
                &image,
                (1.0, 1.0)
            ),
            (100.0, 40.0)
        );
    }
}

//! Background layers placed like Chrome 156 (CSS Backgrounds 3 §3,
//! miaskiewicz/turbo-dom#248): `background-size`, `-position`, `-repeat`, `-origin` and
//! `-clip` turn one background image or gradient into one repeating tile
//! pattern (a [`FragmentContent::Tiles`] fragment over the painting area: the tile, its
//! phase and its step, however many tiles that is; 0.7.0 round-2 review, finding 1) under
//! one container fragment clipped to the painting area — rounded by the clip box's
//! per-corner radii (miaskiewicz/turbo-dom#253). Longhands win over the `background`
//! shorthand's parts.
use super::fragment::{
    BorderRadii, ClipRect, Fragment, FragmentContent, ImagePlacement, LinearGradient, NodeId,
    TilePattern,
};
use super::value::{css_value_tokens, parse_length_pct, BorderEdges, LengthPct, Rgba};
use crate::ComputedStyle;

/// A rectangle `(x, y, width, height)` in absolute px.
pub(crate) type Rect = (f32, f32, f32, f32);

/// What one background layer paints.
#[derive(Clone)]
pub(crate) enum Source {
    Image(ImagePlacement),
    /// Its stops resolve against the tile size (miaskiewicz/turbo-dom#264).
    Gradient(super::value::GradientSpec),
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

/// The box's background properties, from the longhands (the cascade expands the
/// `background` shorthand into them, miaskiewicz/turbo-dom#256).
pub(crate) fn props(style: &ComputedStyle, font_size: f32) -> Props {
    let tokens = |name: &str| -> Vec<String> {
        style
            .get(name)
            .map(|v| {
                css_value_tokens(v)
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let size = style
        .get("background-size")
        .and_then(|v| parse_size(v, font_size))
        .unwrap_or(Size::Lengths(LengthPct::Auto, LengthPct::Auto));
    let position =
        parse_position(&tokens("background-position"), font_size).unwrap_or((START, START));
    let repeat = tokens("background-repeat");
    let origin = style
        .get("background-origin")
        .and_then(area)
        .unwrap_or(Area::Padding);
    let clip = style
        .get("background-clip")
        .and_then(area)
        .unwrap_or(Area::Border);
    Props {
        size,
        position,
        repeat: parse_repeat(&repeat),
        origin,
        clip,
    }
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

/// The offset of an `object`-sized replaced object in its `area` box under an
/// `object-position` value (the `background-position` grammar, CSS Images 3 §5.6):
/// `50% 50%` when unset or invalid (miaskiewicz/turbo-dom#291).
pub(crate) fn object_position(
    value: Option<&str>,
    font_size: f32,
    area: (f32, f32),
    object: (f32, f32),
) -> (f32, f32) {
    let tokens: Vec<String> = css_value_tokens(value.unwrap_or_default())
        .into_iter()
        .map(str::to_string)
        .collect();
    let center = Axis {
        from_end: false,
        offset: LengthPct::Pct(50.0),
    };
    let (x, y) = parse_position(&tokens, font_size).unwrap_or((center, center));
    (place(x, area.0, object.0), place(y, area.1, object.1))
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

/// The smallest tile painted, in px: a sub-pixel tile is painted at 1px (the only
/// degenerate case capped; a layer is one fragment however many tiles it has).
const MIN_TILE: f32 = 1.0;

/// `round`: the tile shrinks or grows so a whole number of tiles fills the area.
fn rounded(tile: f32, area: f32) -> f32 {
    let count = (area / tile).round().max(1.0);
    area / count
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

/// One layer's tile size: `background-size`, refitted by `round` along an axis with
/// room to fill (Chrome keeps the tile on an empty axis), at least `MIN_TILE`. None: no
/// image (an empty or non-finite tile).
fn sized_tile(props: &Props, source: &Source, origin: Rect) -> Option<(f32, f32)> {
    let (mut tw, mut th) = tile_size(props.size, source, (origin.2, origin.3));
    if !(tw > 0.0 && th > 0.0 && tw.is_finite() && th.is_finite()) {
        return None;
    }
    let (round_x, round_y) = (
        props.repeat.0 == Tiling::Round && origin.2 > 0.0,
        props.repeat.1 == Tiling::Round && origin.3 > 0.0,
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
    Some((tw.max(MIN_TILE), th.max(MIN_TILE)))
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
    if paint.2 <= 0.0 || paint.3 <= 0.0 {
        return Vec::new();
    }
    let Some((tw, th)) = sized_tile(props, source, origin) else {
        return Vec::new();
    };
    let x = origin.0 + place(props.position.0, origin.2, tw);
    let y = origin.1 + place(props.position.1, origin.3, th);
    let (x, step_x) = axis(props.repeat.0, x, (origin.0, origin.2), tw);
    let (y, step_y) = axis(props.repeat.1, y, (origin.1, origin.3), th);
    if !x.is_finite() || !y.is_finite() {
        return Vec::new();
    }
    // One tile at or before the painting area's start on a repeated axis (the offset
    // reduced modulo the step, so a huge position keeps its phase and stays finite).
    let phase = |start: f32, step: f32, from: f32| {
        super::fragment::axis_starts(start, step, (from, 0.0))
            .next()
            .unwrap_or(start)
    };
    let tile = match source {
        Source::Image(image) => FragmentContent::Image(image.clone()),
        Source::Gradient(gradient) => {
            let Some(gradient) = gradient.resolve(tw, th) else {
                return Vec::new();
            };
            gradient_box(None, Some(gradient), BorderRadii::ZERO)
        }
    };
    let pattern = TilePattern {
        tile: Box::new(tile),
        x: phase(x, step_x, paint.0),
        y: phase(y, step_y, paint.1),
        tile_width: tw,
        tile_height: th,
        step_x,
        step_y,
    };
    if pattern.tiles(paint).next().is_none() {
        return Vec::new();
    }
    let tiles = Fragment::new(
        id,
        paint.0,
        paint.1,
        paint.2,
        paint.3,
        FragmentContent::Tiles(pattern),
    );
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
    container.children = vec![tiles];
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

    /// A two-stop gradient (red to blue, positions unset) at `angle_deg`.
    fn test_gradient(angle_deg: f32) -> super::super::value::GradientSpec {
        let stop = |color| super::super::value::StopSpec { color, pos: None };
        super::super::value::GradientSpec {
            angle_deg,
            stops: vec![
                stop(Rgba::new(255, 0, 0, 255)),
                stop(Rgba::new(0, 0, 255, 255)),
            ],
            repeating: false,
        }
    }

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
            &crate::test_support::cascaded(
                "background: red url(a.png) repeat-x right 10px bottom/40px auto content-box padding-box",
            ),
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
        assert_eq!(
            super::props(
                &crate::test_support::cascaded("background: border-box"),
                16.0
            )
            .clip,
            Area::Border
        );
        // A shorthand's layered (`border-box, content-box`) or unknown box keeps the
        // initial areas.
        let layered = props_of(&[
            ("background-origin", "border-box, content-box"),
            ("background-clip", "bogus"),
        ]);
        assert_eq!(
            (layered.origin, layered.clip),
            (Area::Padding, Area::Border)
        );
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
        let gradient = Source::Gradient(test_gradient(90.0));
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
        let shown = expand(&layers[0]);
        assert_eq!(shown.len(), 5 * 4);
        assert_eq!((shown[0].x, shown[0].y), (10.0, 20.0));
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
        let tiles = expand(
            &layer(
                NodeId(1),
                &props,
                &gradient,
                (square, square, BorderRadii::ZERO),
            )[0],
        );
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
        let tile = &expand(&layer(NodeId(1), &props, &image, (big, big, BorderRadii::ZERO))[0])[0];
        assert_eq!((tile.width, tile.height), (40.0, 20.0));
        let props = props_of(&[
            ("background-size", "auto 35px"),
            ("background-repeat", "no-repeat round"),
        ]);
        let tile = &expand(&layer(NodeId(1), &props, &image, (big, big, BorderRadii::ZERO))[0])[0];
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
        let starts = |start, step, area| {
            super::super::fragment::axis_starts(start, step, area).collect::<Vec<f32>>()
        };
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
        let tiles = expand(&layer(NodeId(1), &props, &image, (area, area, BorderRadii::ZERO))[0]);
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

    /// The pattern of a layer's container: its one child.
    fn pattern(container: &Fragment) -> &TilePattern {
        assert_eq!(container.children.len(), 1, "one fragment per layer");
        match &container.children[0].content {
            FragmentContent::Tiles(pattern) => pattern,
            other => panic!("{other:?}"),
        }
    }

    /// Every tile of a layer's container as a fragment of its own (the painting area's
    /// tiles; keep the area small).
    fn expand(container: &Fragment) -> Vec<Fragment> {
        let child = &container.children[0];
        let pattern = pattern(container);
        pattern
            .tiles((child.x, child.y, child.width, child.height))
            .map(|(x, y, w, h)| Fragment::new(child.node_id, x, y, w, h, (*pattern.tile).clone()))
            .collect()
    }

    /// The tile fragments of one layer (none: an empty list).
    fn tiles(pairs: &[(&str, &str)], source: &Source, area: Rect) -> Vec<Fragment> {
        let props = props_of(pairs);
        layer(NodeId(1), &props, source, (area, area, BorderRadii::ZERO))
            .pop()
            .map(|container| expand(&container))
            .unwrap_or_default()
    }

    fn sprite() -> Source {
        Source::Image(ImagePlacement {
            name: "a".into(),
            intrinsic_w: 100,
            intrinsic_h: 40,
            has_alpha: false,
            tint: None,
            orientation: crate::image::Orientation::IDENTITY,
        })
    }

    #[test]
    fn translating_a_layer_moves_its_tile_origin() {
        // miaskiewicz/turbo-dom#274: a 100x50 box tiled with a 25px image, moved by an
        // embedder (html2pdf's pagination) or a parent layout, keeps its 4x2 tiles in
        // phase: the pattern origin moves with the box.
        let props = props_of(&[("background-size", "25px 25px")]);
        let area = (0.0, 0.0, 100.0, 50.0);
        let container = layer(
            NodeId(1),
            &props,
            &sprite(),
            (area, area, BorderRadii::ZERO),
        )
        .pop()
        .unwrap();
        let before = expand(&container);
        assert_eq!(before.len(), 8);
        let snapped = |v: f32| (v + 0.5).floor();
        for (dx, dy) in [(12.5, 37.25), (-25.0, 100.0), (1.0, 2.0)] {
            let mut moved = container.clone();
            moved.translate(dx, dy);
            let (was, now) = (pattern(&container), pattern(&moved));
            assert_eq!((now.x, now.y), (was.x + dx, was.y + dy));
            let after = expand(&moved);
            assert_eq!(after.len(), 8, "({dx}, {dy})");
            for (a, b) in before.iter().zip(&after) {
                let (x, y) = (a.x + dx, a.y + dy);
                assert_eq!(
                    (b.x, b.y, b.width, b.height),
                    (
                        snapped(x),
                        snapped(y),
                        snapped(x + 25.0) - snapped(x),
                        snapped(y + 25.0) - snapped(y)
                    )
                );
            }
        }
    }

    #[test]
    fn hostile_tiles_stay_bounded() {
        let gradient = Source::Gradient(test_gradient(90.0));
        let big = (0.0, 0.0, 1.0e6, 1.0e6);
        let image = sprite();
        // 1e30px and 1e400px (an f32 infinity), spelled out: lengths take no exponent.
        let e30 = format!("1{}px", "0".repeat(30));
        let inf = format!("1{}px", "0".repeat(400));
        let (huge, negative, huge_x) = (
            format!("{e30} {e30}"),
            format!("-{e30} -{e30}"),
            format!("{e30} 0"),
        );
        let cases: [(&[(&str, &str)], &Source); 7] = [
            (&[("background-size", "0.01px")], &image),
            (&[("background-size", "0.01px 0.01px")], &gradient),
            (
                &[
                    ("background-size", "0.01px"),
                    ("background-repeat", "space"),
                ],
                &image,
            ),
            (
                &[
                    ("background-size", "0.01px 0.01px"),
                    ("background-repeat", "round"),
                ],
                &gradient,
            ),
            (&[("background-position", &huge)], &image),
            (&[("background-position", &negative)], &image),
            (
                &[
                    ("background-position", &huge_x),
                    ("background-repeat", "space"),
                ],
                &image,
            ),
        ];
        // The spelled-out lengths parse (the cases are not the default position).
        assert_ne!(
            props_of(&[("background-position", &huge)]).position,
            props_of(&[]).position
        );
        // Each layer is one fragment however many tiles it has (0.7.0 round-2 review,
        // finding 1), and a painter's window of it holds at most one tile per pixel.
        let window = (0.0, 0.0, 300.0, 200.0);
        for (pairs, source) in cases {
            let layers = layer(
                NodeId(1),
                &props_of(pairs),
                source,
                (big, big, BorderRadii::ZERO),
            );
            assert_eq!(layers.len(), 1, "{pairs:?}");
            let tiles: Vec<_> = pattern(&layers[0]).tiles(window).collect();
            assert!(!tiles.is_empty(), "{pairs:?}");
            assert!(tiles.len() <= 300 * 200, "{pairs:?}: {}", tiles.len());
            assert!(
                tiles
                    .iter()
                    .all(|t| t.2 >= 1.0 && t.3 >= 1.0 && t.0.is_finite() && t.1.is_finite()),
                "{pairs:?}"
            );
        }
        // A tile or an offset that overflows an f32 paints nothing.
        for pairs in [
            [("background-size", format!("{inf} 10px"))],
            [("background-position", format!("{inf} 0"))],
        ] {
            let pairs = [(pairs[0].0, pairs[0].1.as_str())];
            assert!(tiles(&pairs, &image, big).is_empty(), "{pairs:?}");
        }
        // A sub-pixel tile is painted 1px wide, so a small box gets one tile per pixel.
        let fine = tiles(
            &[("background-size", "0.25px 0.25px")],
            &gradient,
            (0.0, 0.0, 30.0, 20.0),
        );
        assert_eq!(fine.len(), 600);
        // An empty painting area paints nothing, whatever the tile.
        let empty = (0.0, 0.0, 0.0, 10.0);
        assert!(layer(
            NodeId(1),
            &props_of(&[]),
            &image,
            (big, empty, BorderRadii::ZERO)
        )
        .is_empty());
    }

    #[test]
    fn a_big_box_of_small_tiles_is_one_pattern_down_to_its_last_tile() {
        // 0.7.0 round-2 review, finding 1: a 4px grid over a 1200 x 5000 page is
        // 375000 tiles; the layer is one pattern, and its bottom-right tile is there.
        let gradient = Source::Gradient(test_gradient(180.0));
        let page = (0.0, 0.0, 1200.0, 5000.0);
        let layers = layer(
            NodeId(1),
            &props_of(&[("background-size", "4px 4px")]),
            &gradient,
            (page, page, BorderRadii::ZERO),
        );
        let pattern = pattern(&layers[0]);
        assert_eq!(pattern.period(), Some((4.0, 4.0)));
        assert_eq!(pattern.tiles(page).count(), 300 * 1250);
        let corner: Vec<_> = pattern.tiles((1199.0, 4999.0, 1.0, 1.0)).collect();
        assert_eq!(corner, [(1196.0, 4996.0, 4.0, 4.0)]);
        // Fractional steps snap to alternating sizes: no period.
        let fine = layer(
            NodeId(1),
            &props_of(&[("background-size", "2.5px 4px")]),
            &gradient,
            (page, page, BorderRadii::ZERO),
        );
        assert_eq!(pattern_of(&fine).period(), None);
        let widths: Vec<f32> = pattern_of(&fine)
            .tiles((0.0, 0.0, 10.0, 1.0))
            .map(|t| t.2)
            .collect();
        assert_eq!(widths, [3.0, 2.0, 3.0, 2.0]);
        // `space` steps by a whole pixel past a whole tile: a period with gaps; a
        // single tile on an axis has no step there.
        let spaced = layer(
            NodeId(1),
            &props_of(&[
                ("background-size", "40px 20px"),
                ("background-repeat", "space no-repeat"),
            ]),
            &gradient,
            (
                (0.0, 0.0, 220.0, 60.0),
                (0.0, 0.0, 220.0, 60.0),
                BorderRadii::ZERO,
            ),
        );
        assert_eq!(pattern_of(&spaced).period(), Some((45.0, 0.0)));
    }

    fn pattern_of(layers: &[Fragment]) -> &TilePattern {
        pattern(&layers[0])
    }

    #[test]
    fn a_huge_offset_keeps_the_tile_phase() {
        // 1e7 + 30 is 30 past a 100px tile boundary: the tiles start 70px left of the box.
        let tiles = tiles(
            &[("background-position", "10000030px 0")],
            &sprite(),
            (0.0, 0.0, 200.0, 40.0),
        );
        let xs: Vec<f32> = tiles.iter().map(|t| t.x).collect();
        assert_eq!(xs, [-70.0, 30.0, 130.0]);
    }

    #[test]
    fn round_keeps_the_tile_on_an_empty_axis() {
        // Chrome (paint fixture `bg-zero-area-round`): with no height to fill, `round`
        // fits the width (100 / 1) and keeps the image's height.
        let props = props_of(&[("background-repeat", "round")]);
        let layers = layer(
            NodeId(1),
            &props,
            &sprite(),
            (
                (40.0, 40.0, 100.0, 0.0),
                (20.0, 20.0, 140.0, 40.0),
                BorderRadii::ZERO,
            ),
        );
        let tiles = expand(&layers[0]);
        assert!(tiles.iter().all(|t| (t.width, t.height) == (100.0, 40.0)));
        assert!(
            tiles.iter().any(|t| (t.x, t.y) == (40.0, 40.0)),
            "{tiles:?}"
        );
    }
}

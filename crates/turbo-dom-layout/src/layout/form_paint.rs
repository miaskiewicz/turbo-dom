//! Chrome 156's native appearance of the form controls (miaskiewicz/turbo-dom#255),
//! drawn with the galley's own box primitives so every painter (the raster PNG / SVG
//! screenshots, html2pdf) shows it: a control with its default `appearance` paints
//! Chrome's control frame instead of its CSS border and background.
//!
//! - a text field, textarea and menu list: a 1px `#767676` frame with 2px corners on
//!   white (the CSS border's width stays in the layout), a menu list its chevron 7px from
//!   its right edge;
//! - a button: the same frame on `#efefef`; a colour well also its swatch, a 1px
//!   `#777777` frame filled with its value (black by default) inset 3px / 4px in its
//!   content box;
//! - a checkbox: the frame on white, `checked` a `#0075ff` square with a white check, an
//!   indeterminate one ([`super::form_controls::INDETERMINATE_ATTR`]) the square with a
//!   white dash; a radio button a circle, `checked` a blue disc with a white ring;
//! - a range slider: a 7px track (blue up to the thumb, `#efefef` framed in `#b2b2b2`
//!   after it) and a blue round thumb (1px inside its box) at its value;
//! - a `<progress>` / `<meter>` (miaskiewicz/turbo-dom#293): a bar half the box tall
//!   across it, rounded, `#efefef` framed in `#b2b2b2` (a meter's `#cbcbcb`), filled from
//!   its start up to its value: a progress in `#0075ff`, a meter in the colour of its
//!   value's region (optimum `#107c10`, suboptimal `#ffb900`, even less good `#d83b01`).
//!   An indeterminate progress (no `value`) shows its track: Chrome's animated bar is
//!   not drawn.
//!
//! In a vertical writing mode the control is drawn in its own logical space, so the map
//! to physical space turns it as Chrome does (#293): a range, progress or meter runs down
//! its inline axis, a menu list's chevron points along the block axis, a colour well's
//! insets trade axes; the check, dash and chevron glyphs keep their physical shape
//! ([`upright`]).
//!
//! Text fields, textareas, menu lists, buttons, colour wells, progress bars and meters
//! fall back to their CSS painting when the author restyles their border or background
//! (as Chrome does: any author `border*` declaration, even one equal to the UA's,
//! miaskiewicz/turbo-dom#283), and every control with `appearance: none`. Those CSS
//! borders are the UA sheet's: a text field's `2px inset #767676` and a button's
//! `2px outset black`, shaded like Chrome's inset / outset sides (miaskiewicz/turbo-dom#257).
//! Measured against the paint Chrome 156 control's `form-*` cases and the form-native
//! control (`scripts/form-native-chrome-control.mjs`).
use super::fragment::{BorderRadii, CornerRadii, Fragment, FragmentContent, Transform2D};
use super::value::{border_style_colour, BorderEdges, BorderSide, BoxStyle, Rgba};
use super::writing::{Relabel, WritingMode};
use crate::Attr;

use super::form_controls::{Control, INDETERMINATE_ATTR};

const FRAME: Rgba = rgb(0x76, 0x76, 0x76);
const WHITE: Rgba = rgb(0xff, 0xff, 0xff);
const FACE: Rgba = rgb(0xef, 0xef, 0xef);
const ACCENT: Rgba = rgb(0x00, 0x75, 0xff);
const TRACK: Rgba = rgb(0xb2, 0xb2, 0xb2);
const SWATCH: Rgba = rgb(0x77, 0x77, 0x77);
const METER_TRACK: Rgba = rgb(0xcb, 0xcb, 0xcb);
const OPTIMUM: Rgba = rgb(0x10, 0x7c, 0x10);
const SUBOPTIMAL: Rgba = rgb(0xff, 0xb9, 0x00);
const EVEN_LESS_GOOD: Rgba = rgb(0xd8, 0x3b, 0x01);
/// A `<progress>`'s shadow-bar track when the author turned the native look off
/// (miaskiewicz/turbo-dom#346). Measured on Chrome 156 headless.
const SHADOW_TRACK: Rgba = rgb(0x80, 0x80, 0x80);
/// A determinate `<progress>`'s shadow value bar (above).
const SHADOW_VALUE: Rgba = rgb(0x00, 0x80, 0x00);

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

/// What the control box `frag` (of `control`, with `attrs` and used style `bs`) paints:
/// Chrome's native look in place of its CSS decoration, when it has it. `frag` is laid
/// out in the space of the control's writing mode.
pub(crate) fn apply(
    control: Control,
    attrs: &[Attr],
    style: &crate::ComputedStyle,
    bs: &BoxStyle,
    frag: &mut Fragment,
) {
    let mode = WritingMode::of(style);
    if !native(control, style, frag, mode) {
        // A `<progress>` with `appearance:none` or an author background still shows
        // Chrome's shadow bars (grey track + green value, or blue indeterminate first
        // frame; miaskiewicz/turbo-dom#346). A `<meter>`'s shadow DOM paints a gradient
        // which this layer does not try to reproduce — the test assertions skip the
        // meter's inside.
        if matches!(control, Control::Progress) {
            shadow_progress(attrs, frag);
        }
        return;
    }
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let mut parts = Vec::new();
    let own = match control {
        Control::TextField | Control::TextArea => framed(WHITE, 2.0),
        Control::Select => {
            parts.push(chevron(frag, mode));
            framed(WHITE, 2.0)
        }
        Control::Button => framed(FACE, 2.0),
        Control::Color => {
            let b = bs.border.widths();
            let p = bs.padding;
            // 3px / 4px along the physical width / height: the block / inline axes of a
            // vertical well.
            let (inset_x, inset_y) = if mode.vertical() {
                (4.0, 3.0)
            } else {
                (3.0, 4.0)
            };
            parts.push(rect(
                frag,
                (x + b.left + p.left + inset_x, y + b.top + p.top + inset_y),
                (
                    w - b.horizontal() - p.horizontal() - 2.0 * inset_x,
                    h - b.vertical() - p.vertical() - 2.0 * inset_y,
                ),
                swatch_colour(attrs),
                Some(SWATCH),
                0.0,
            ));
            framed(FACE, 2.0)
        }
        Control::Toggle => return toggle(attrs, frag, mode),
        Control::Progress | Control::Meter => return gauge(control, attrs, frag),
    };
    frag.content = own;
    frag.children.splice(0..0, parts);
}

/// Whether `frag` paints Chrome's native look: a default `appearance` and (for the
/// controls an author can restyle) the UA sheet's border and background. Its border
/// sides are those of `mode`'s space.
fn native(
    control: Control,
    style: &crate::ComputedStyle,
    frag: &Fragment,
    mode: WritingMode,
) -> bool {
    let appearance = style.get("appearance").map(str::trim);
    if matches!(appearance, Some("none")) {
        return false;
    }
    let FragmentContent::Box {
        background,
        border,
        border_radius,
        gradient,
        ..
    } = &frag.content
    else {
        // A form control's box is a block-level box (boxgen's `box_kind_for`), never a
        // marker, so `content_kind` gave it a `Box`.
        unreachable!("a form control's fragment is a box");
    };
    // The UA border: (width, colour, inset / outset / solid) and background.
    let (width, colour, shade, face) = match control {
        Control::Toggle => return true,
        // Any author border declaration turns the native look off, even one setting the
        // UA's own values (Chrome 156, miaskiewicz/turbo-dom#283).
        _ if style.get(crate::style::AUTHOR_BORDER).is_some() => return false,
        Control::Progress | Control::Meter => {
            return background.is_none() && gradient.is_none();
        }
        Control::TextField => (2, FRAME, Some(true), WHITE),
        Control::Button => (2, Rgba::BLACK, Some(false), FACE),
        Control::TextArea | Control::Select => (1, FRAME, None, WHITE),
        Control::Color => (1, FRAME, None, FACE),
    };
    // An inset border's physical top and left are its dark sides, an outset one's
    // bottom and right.
    let side = |logical: usize| BorderSide {
        width,
        color: Some(shade.map_or(colour, |inset| {
            border_style_colour(colour, mode.top_or_left(logical) == inset)
        })),
    };
    *border
        == (BorderEdges {
            top: side(0),
            right: side(1),
            bottom: side(2),
            left: side(3),
        })
        && *background == Some(face)
        && gradient.is_none()
        && *border_radius == BorderRadii::ZERO
}

/// A box decoration: `background` inside a 1px `frame`, with `radius` corners.
fn decoration(background: Rgba, frame: Option<Rgba>, radius: f32) -> FragmentContent {
    let side = BorderSide {
        width: u16::from(frame.is_some()),
        color: frame,
    };
    FragmentContent::Box {
        background: Some(background),
        border: BorderEdges {
            top: side,
            right: side,
            bottom: side,
            left: side,
        },
        border_radius: BorderRadii::uniform(radius),
        shadow: None,
        gradient: None,
        transform: None,
    }
}

fn framed(background: Rgba, radius: f32) -> FragmentContent {
    decoration(background, Some(FRAME), radius)
}

fn rect(
    owner: &Fragment,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
    background: Rgba,
    frame: Option<Rgba>,
    radius: f32,
) -> Fragment {
    Fragment::new(
        owner.node_id,
        x,
        y,
        w.max(0.0),
        h.max(0.0),
        decoration(background, frame, radius),
    )
}

/// The point `(px, py)` px right of and below the physical top-left of the box `owner`
/// (laid out in `mode`'s space), in that space.
fn logical_point(owner: &Fragment, mode: WritingMode, (px, py): (f32, f32)) -> (f32, f32) {
    match mode {
        WritingMode::HorizontalTb => (owner.x + px, owner.y + py),
        WritingMode::VerticalLr => (owner.x + py, owner.y + px),
        WritingMode::VerticalRl => (owner.x + py, owner.y + owner.height - px),
    }
}

/// A glyph that keeps its physical shape in every writing mode: `part` is built
/// physically (its size, border sides, radii and transform as painted) centred on the
/// point `(cx, cy)` of `mode`'s space; its size, sides and radii move into that space,
/// so the map to physical space restores them about the mapped centre.
fn upright(mut part: Fragment, mode: WritingMode, (cx, cy): (f32, f32)) -> Fragment {
    if mode.vertical() {
        std::mem::swap(&mut part.width, &mut part.height);
        let into = Relabel::between(WritingMode::HorizontalTb, mode);
        if let FragmentContent::Box {
            border,
            border_radius,
            ..
        } = &mut part.content
        {
            *border = into.border(*border);
            *border_radius = into.radii(*border_radius);
        }
    }
    part.x = cx - part.width / 2.0;
    part.y = cy - part.height / 2.0;
    part
}

/// An L of two `width`px white or black strokes (a box's right and bottom border)
/// turned `angle` radians clockwise about its middle: `(w, h)` is the (physical) box.
fn turned_l(owner: &Fragment, (w, h): (f32, f32), stroke: (u16, Rgba), angle: f32) -> Fragment {
    let (width, color) = stroke;
    let side = BorderSide {
        width,
        color: Some(color),
    };
    let none = BorderSide::default();
    let (sin, cos) = angle.sin_cos();
    Fragment::new(
        owner.node_id,
        0.0,
        0.0,
        w,
        h,
        FragmentContent::Box {
            background: None,
            border: BorderEdges {
                top: none,
                right: side,
                bottom: side,
                left: none,
            },
            border_radius: BorderRadii::ZERO,
            shadow: None,
            gradient: None,
            transform: Some(Transform2D {
                matrix: [cos, sin, -sin, cos, 0.0, 0.0],
                origin_x: w / 2.0,
                origin_y: h / 2.0,
            }),
        },
    )
}

/// A menu list's chevron: a 2px stroke, the turned L of a 7px square centred 7px from
/// its inline end and 1.5px before its block middle (its tip 2px past the middle;
/// #257), pointing along the block axis: down, or right in `vertical-lr`, left in
/// `vertical-rl` (#293).
fn chevron(frag: &Fragment, mode: WritingMode) -> Fragment {
    let quarter = std::f32::consts::FRAC_PI_4;
    let angle = match mode {
        WritingMode::HorizontalTb => quarter,
        WritingMode::VerticalLr => -quarter,
        WritingMode::VerticalRl => 3.0 * quarter,
    };
    let centre = (frag.x + frag.width - 7.0, frag.y + frag.height / 2.0 - 1.5);
    upright(
        turned_l(frag, (7.0, 7.0), (2, Rgba::BLACK), angle),
        mode,
        centre,
    )
}

/// A checkbox, radio button or range slider (`frag` is its border box).
fn toggle(attrs: &[Attr], frag: &mut Fragment, mode: WritingMode) {
    let kind = attr(attrs, "type").unwrap_or("").to_ascii_lowercase();
    let checked = attr(attrs, "checked").is_some();
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let mut parts = Vec::new();
    frag.content = match (kind.as_str(), checked) {
        ("radio", false) => framed(WHITE, w.min(h) / 2.0),
        ("radio", true) => {
            let r = w.min(h) / 2.0;
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let disc = |radius: f32, colour| {
                rect(
                    frag,
                    (cx - radius, cy - radius),
                    (2.0 * radius, 2.0 * radius),
                    colour,
                    None,
                    radius,
                )
            };
            parts.push(disc(r - 1.0, WHITE));
            parts.push(disc(r - 2.5, ACCENT));
            decoration(ACCENT, None, r)
        }
        ("range", _) => {
            parts = range(attrs, frag);
            bare()
        }
        _ if attr(attrs, INDETERMINATE_ATTR).is_some() => {
            parts.push(dash(frag, mode));
            decoration(ACCENT, None, 2.0)
        }
        (_, false) => framed(WHITE, 2.0),
        (_, true) => {
            let k = w.min(h) / 13.0;
            let check = turned_l(
                frag,
                (3.0 * k, 9.0 * k),
                (2, WHITE),
                std::f32::consts::FRAC_PI_4,
            );
            let centre = logical_point(frag, mode, (6.8 * k, 5.4 * k));
            parts.push(upright(check, mode, centre));
            decoration(ACCENT, None, 2.0)
        }
    };
    frag.children.splice(0..0, parts);
}

/// A box that paints nothing itself.
fn bare() -> FragmentContent {
    FragmentContent::Box {
        background: None,
        border: BorderEdges::default(),
        border_radius: BorderRadii::ZERO,
        shadow: None,
        gradient: None,
        transform: None,
    }
}

/// An indeterminate checkbox's dash: a white 7.8 x 2 bar with round ends (of a 13px box),
/// its middle 1px right of the box's and level with it (Chrome 156).
fn dash(frag: &Fragment, mode: WritingMode) -> Fragment {
    let k = frag.width.min(frag.height) / 13.0;
    let bar = rect(frag, (0.0, 0.0), (7.8 * k, 2.0 * k), WHITE, None, k);
    upright(bar, mode, logical_point(frag, mode, (7.5 * k, 6.5 * k)))
}

/// A `<progress>` or `<meter>` (`frag` is its border box, in its own writing mode's
/// space): a rounded bar half the box tall across its middle, filled from its inline
/// start up to its value.
fn gauge(control: Control, attrs: &[Attr], frag: &mut Fragment) {
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let (top, bar_h) = (y + h / 4.0, h / 2.0);
    let radius = bar_h / 2.0;
    let (frame, fill) = match control {
        Control::Progress => (TRACK, progress_position(attrs).map(|p| (p, ACCENT))),
        _ => (METER_TRACK, Some(meter_value(attrs))),
    };
    let mut parts = vec![rect(frag, (x, top), (w, bar_h), FACE, Some(frame), radius)];
    if let Some((ratio, colour)) = fill.filter(|(ratio, _)| *ratio > 0.0) {
        let end = if ratio >= 1.0 { radius } else { 0.0 };
        let mut value = rect(frag, (x, top), (w * ratio as f32, bar_h), colour, None, 0.0);
        if let FragmentContent::Box { border_radius, .. } = &mut value.content {
            let (start, end) = (CornerRadii::circle(radius), CornerRadii::circle(end));
            *border_radius = BorderRadii::from_corners([start, end, end, start]);
        }
        parts.push(value);
    } else if matches!(control, Control::Progress) && attr(attrs, "value").is_none() {
        // An indeterminate `<progress>`'s first frame: a blue bar at the inline start,
        // roughly 5%..19% of the box (Chrome 156 animates the bar; the first frame is
        // what this layer captures, miaskiewicz/turbo-dom#346).
        parts.push(rect(
            frag,
            (x + w * 0.05, top),
            (w * 0.14, bar_h),
            ACCENT,
            None,
            0.0,
        ));
    }
    frag.content = bare();
    frag.children.splice(0..0, parts);
}

/// A `<progress>`'s shadow-DOM painting under `appearance:none` or an author background
/// (Chrome 156 headless, miaskiewicz/turbo-dom#346): a grey #808080 track across the
/// whole box with a green #008000 value bar at the fraction of the box the `value`
/// gives. An indeterminate `<progress>` here also uses the first frame blue bar.
fn shadow_progress(attrs: &[Attr], frag: &mut Fragment) {
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let mut parts = vec![rect(frag, (x, y), (w, h), SHADOW_TRACK, None, 0.0)];
    if let Some(ratio) = progress_position(attrs) {
        parts.push(rect(
            frag,
            (x, y),
            (w * ratio as f32, h),
            SHADOW_VALUE,
            None,
            0.0,
        ));
    } else if attr(attrs, "value").is_none() {
        parts.push(rect(
            frag,
            (x + w * 0.05, y),
            (w * 0.14, h),
            ACCENT,
            None,
            0.0,
        ));
    }
    frag.content = bare();
    frag.children.splice(0..0, parts);
}

/// A number attribute (an HTML valid floating-point number, see [`html_float`]).
fn number(attrs: &[Attr], name: &str) -> Option<f64> {
    attr(attrs, name).and_then(html_float)
}

/// A determinate progress's position (its `value` over its `max`), `None` without a
/// `value` attribute. The max is 1 unless a positive number; the value is 0 unless a
/// non-negative number, and at most the max (HTML's `<progress>`).
fn progress_position(attrs: &[Attr]) -> Option<f64> {
    attr(attrs, "value")?;
    let max = number(attrs, "max").filter(|m| *m > 0.0).unwrap_or(1.0);
    let value = number(attrs, "value").filter(|v| *v >= 0.0).unwrap_or(0.0);
    Some(value.min(max) / max)
}

/// A meter's filled fraction and the colour of its value's region (HTML's `<meter>`:
/// the bounds default to 0 and 1, a max below the min is the min; the value, low, high
/// and optimum are clamped into them, the high to at least the low; Blink's gauge
/// regions).
fn meter_value(attrs: &[Attr]) -> (f64, Rgba) {
    let min = number(attrs, "min").unwrap_or(0.0);
    let max = number(attrs, "max").unwrap_or(1.0).max(min);
    let clamp = |name, default: f64| number(attrs, name).unwrap_or(default).clamp(min, max);
    let value = clamp("value", 0.0);
    let low = clamp("low", min);
    let high = clamp("high", max).max(low);
    let optimum = clamp("optimum", min / 2.0 + max / 2.0);
    let colour = if optimum < low {
        region(value <= low, value <= high)
    } else if optimum > high {
        region(value >= high, value >= low)
    } else {
        region(value >= low && value <= high, true)
    };
    // Halved, so `max - min` cannot overflow to infinity.
    let span = max / 2.0 - min / 2.0;
    let ratio = if span > 0.0 {
        (value / 2.0 - min / 2.0) / span
    } else {
        0.0
    };
    (ratio, colour)
}

/// The colour of a meter value's region: optimum, else suboptimal, else even less good.
fn region(optimum: bool, suboptimal: bool) -> Rgba {
    if optimum {
        OPTIMUM
    } else if suboptimal {
        SUBOPTIMAL
    } else {
        EVEN_LESS_GOOD
    }
}

/// A range slider's track and thumb, at its `value` between `min` and `max` (default
/// 0 and 100; a max below the min is the min; the value defaults to their middle and is
/// clamped to them). Attributes that are not finite valid floating-point numbers are
/// ignored, as in Chrome.
fn range(attrs: &[Attr], frag: &Fragment) -> Vec<Fragment> {
    let number = |name| attr(attrs, name).and_then(html_float);
    let min = number("min").unwrap_or(0.0);
    let max = number("max").unwrap_or(100.0).max(min);
    let value = number("value").unwrap_or(min / 2.0 + max / 2.0);
    let step = step_of(attr(attrs, "step"));
    // The step base: the `min` attribute, else the `value` attribute, else 0 (HTML).
    let base = number("min").or(number("value")).unwrap_or(0.0);
    // The track still spans min..max: `step=3 max=10 value=10` sits at 90% (Chrome 156,
    // paint fixture `form-native-range-step-max`).
    let value = step_align(min, max, value.clamp(min, max), step, base);
    // Halved first, so `max - min` cannot overflow to infinity.
    let (half_min, half_max) = (min / 2.0, max / 2.0);
    let span = half_max - half_min;
    let ratio = if span > 0.0 {
        (value / 2.0 - half_min) / span
    } else {
        0.0
    };
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let r = h / 2.0;
    let thumb = x + r + (w - 2.0 * r) * ratio as f32;
    let (top, track_h) = (y + h / 2.0 - 3.5, 7.0);
    vec![
        rect(
            frag,
            (x + 0.5, top),
            (w - 1.0, track_h),
            FACE,
            Some(TRACK),
            track_h / 2.0,
        ),
        rect(
            frag,
            (x + 0.5, top),
            (thumb - x - 0.5, track_h),
            ACCENT,
            None,
            track_h / 2.0,
        ),
        rect(
            frag,
            (thumb - r + 1.0, y + 1.0),
            (2.0 * r - 2.0, h - 2.0),
            ACCENT,
            None,
            r - 1.0,
        ),
    ]
}

/// HTML's valid floating-point number (`-`? digits with an optional fraction, or a bare
/// fraction, then an optional exponent; no whitespace, `+` or trailing text), when it is
/// finite as a double.
/// A range's `step`: `None` for `any`; a missing, invalid or non-positive step is the
/// default 1 (HTML's allowed value step).
fn step_of(text: Option<&str>) -> Option<f64> {
    match text {
        Some(t) if t.eq_ignore_ascii_case("any") => None,
        t => Some(t.and_then(html_float).filter(|s| *s > 0.0).unwrap_or(1.0)),
    }
}

/// The (clamped) `value` aligned to `step` from `base`, as Chrome's `StepRange`: snapped to
/// the nearest step value, at most the last step value at or below the max (the max
/// lowered to it), at least the min. No step (`any`), or a step finer than an `f64` can
/// count across the range, leaves it as it is.
fn step_align(min: f64, max: f64, value: f64, step: Option<f64>, base: f64) -> f64 {
    let Some(step) = step else {
        return value;
    };
    // Halved, so `max - base` cannot overflow to infinity.
    let steps = (max / 2.0 - base / 2.0) / (step / 2.0);
    if steps.is_nan() || steps.abs() >= 9_007_199_254_740_992.0 {
        return value;
    }
    let aligned = base + steps.floor() * step;
    let max = if aligned >= min { aligned } else { max };
    let snapped = base + ((value - base) / step).round() * step;
    // Rounded up past the (lowered) max: one step back.
    let value = if snapped > max {
        snapped - step
    } else {
        snapped
    };
    value.max(min)
}

fn html_float(text: &str) -> Option<f64> {
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    let rest = text.strip_prefix('-').unwrap_or(text);
    let int = digits(rest);
    let mut at = int;
    if rest[at..].starts_with('.') {
        let frac = digits(&rest[at + 1..]);
        if frac == 0 {
            return None;
        }
        at += 1 + frac;
    } else if int == 0 {
        return None;
    }
    if let Some(exp) = rest[at..].strip_prefix(['e', 'E']) {
        let unsigned = exp.strip_prefix(['+', '-']).unwrap_or(exp);
        let n = digits(unsigned);
        if n == 0 {
            return None;
        }
        at = rest.len() - unsigned.len() + n;
    }
    let parsed = if at == rest.len() {
        text.parse::<f64>().ok()
    } else {
        None
    };
    parsed.filter(|v| v.is_finite())
}

/// A colour well's value (`#rrggbb`, case-insensitive), black when absent or invalid.
fn swatch_colour(attrs: &[Attr]) -> Rgba {
    let hex = attr(attrs, "value")
        .and_then(|v| v.strip_prefix('#'))
        .filter(|v| v.len() == 6 && v.bytes().all(|b| b.is_ascii_hexdigit()));
    hex.map_or(Rgba::BLACK, |v| {
        let byte = |i: usize| u8::from_str_radix(&v[i..i + 2], 16).unwrap_or(0);
        rgb(byte(0), byte(2), byte(4))
    })
}

fn attr<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

#[cfg(test)]
mod tests {
    use super::super::fragment::{Fragment, FragmentContent};
    use super::super::value::Rgba;
    use super::{
        meter_value, progress_position, step_align, step_of, ACCENT, EVEN_LESS_GOOD, FACE, FRAME,
        INDETERMINATE_ATTR, METER_TRACK, OPTIMUM, SUBOPTIMAL, SWATCH, TRACK, WHITE,
    };

    /// Every box fragment of `body` laid out at 400px: `(x, y, w, h, background,
    /// right border colour and width, top-left radius, turned)`.
    type Part = (
        f32,
        f32,
        f32,
        f32,
        Option<Rgba>,
        Option<Rgba>,
        u16,
        f32,
        bool,
    );

    fn parts(body: &str) -> Vec<Part> {
        fn walk(f: &Fragment, out: &mut Vec<Part>) {
            if let FragmentContent::Box {
                background,
                border,
                border_radius,
                transform,
                ..
            } = &f.content
            {
                out.push((
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    *background,
                    border.right.color,
                    border.right.width,
                    border_radius.top_left.h,
                    transform.is_some(),
                ));
            }
            for child in &f.children {
                walk(child, out);
            }
        }
        let mut out = Vec::new();
        walk(&galley(body), &mut out);
        out
    }

    /// `body` laid out at 400px as `layout_html` lays it out, each element with a
    /// `data-indeterminate` attribute marked indeterminate as a live embedder marks it.
    fn galley(body: &str) -> Fragment {
        fn mark(nodes: &mut [crate::StyledNode]) {
            for node in nodes {
                if let crate::StyledNode::Element(el) = node {
                    if el.attrs.iter().any(|a| a.name == "data-indeterminate") {
                        el.attrs.push(crate::Attr {
                            name: INDETERMINATE_ATTR.into(),
                            value: String::new(),
                        });
                    }
                    mark(&mut el.children);
                }
            }
        }
        let html = format!("<!DOCTYPE html><style>body{{margin:0}}</style><body>{body}");
        let tree = crate::parse_html_tree(&html);
        let mut styled = crate::style_html_tree(&tree, "", 400.0, &crate::HtmlOptions::default());
        mark(&mut styled);
        let _ = crate::html_layout::propagate_canvas_background(&mut styled);
        let _root = crate::root_font_scope(crate::document_root_font_px(&styled));
        let fonts = crate::FontRegistry::new();
        let mut diags = crate::Diagnostics::default();
        crate::layout_in_viewport(
            &styled,
            (400.0, 0.0),
            &fonts,
            &super::super::ImageCtx::none(),
            &crate::LayoutOptions::default(),
            &mut diags,
        )
    }

    /// Every transformed box of `body`'s galley: `(x, y, w, h, matrix, sides with a
    /// colour [top, right, bottom, left])`.
    type Turned = (f32, f32, f32, f32, [f32; 4], [bool; 4]);

    fn turned(body: &str) -> Vec<Turned> {
        fn walk(f: &Fragment, out: &mut Vec<Turned>) {
            if let FragmentContent::Box {
                border,
                transform: Some(t),
                ..
            } = &f.content
            {
                let m = t.matrix;
                let sides = [border.top, border.right, border.bottom, border.left];
                let round = |v: f32| (v * 1000.0).round() / 1000.0;
                out.push((
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    [round(m[0]), round(m[1]), round(m[2]), round(m[3])],
                    sides.map(|s| s.color.is_some() && s.width > 0),
                ));
            }
            for child in &f.children {
                walk(child, out);
            }
        }
        let mut out = Vec::new();
        walk(&galley(body), &mut out);
        out
    }

    fn has(parts: &[Part], want: Part) -> bool {
        parts.iter().any(|p| {
            let close = |a: f32, b: f32| (a - b).abs() < 0.01;
            close(p.0, want.0)
                && close(p.1, want.1)
                && close(p.2, want.2)
                && close(p.3, want.3)
                && (p.4, p.5, p.6) == (want.4, want.5, want.6)
                && close(p.7, want.7)
                && p.8 == want.8
        })
    }

    const ABS: &str = "position:absolute;left:0;top:0;margin:0";

    #[test]
    fn checked_toggles_draw_chrome_glyphs() {
        let check = parts(&format!("<input type=checkbox checked style='{ABS}'>"));
        assert!(
            has(
                &check,
                (0.0, 0.0, 13.0, 13.0, Some(ACCENT), None, 0, 2.0, false)
            ),
            "{check:?}"
        );
        // The check: a 3 x 9 box with white right and bottom strokes, turned.
        assert!(
            has(
                &check,
                (5.3, 0.9, 3.0, 9.0, None, Some(WHITE), 2, 0.0, true)
            ),
            "{check:?}"
        );
        let radio = parts(&format!("<input type=radio checked style='{ABS}'>"));
        assert!(has(
            &radio,
            (0.0, 0.0, 13.0, 13.0, Some(ACCENT), None, 0, 6.5, false)
        ));
        assert!(has(
            &radio,
            (1.0, 1.0, 11.0, 11.0, Some(WHITE), None, 0, 5.5, false)
        ));
        assert!(has(
            &radio,
            (2.5, 2.5, 8.0, 8.0, Some(ACCENT), None, 0, 4.0, false)
        ));
        let unchecked = parts(&format!("<input type=radio style='{ABS}'>"));
        assert!(has(
            &unchecked,
            (
                0.0,
                0.0,
                13.0,
                13.0,
                Some(WHITE),
                Some(FRAME),
                1,
                6.5,
                false
            )
        ));
    }

    #[test]
    fn range_thumb_sits_at_its_clamped_value() {
        let at = |attrs: &str| {
            parts(&format!(
                "<input type=range {attrs} style='{ABS};width:60px'>"
            ))
        };
        // value 18 of 10..20: the thumb centre 8 + 44 * 0.8 from the left edge.
        let value = at("min=10 max=20 value=18");
        assert!(has(
            &value,
            (0.5, 4.5, 59.0, 7.0, Some(FACE), Some(TRACK), 1, 3.5, false)
        ));
        assert!(has(
            &value,
            (0.5, 4.5, 42.7, 7.0, Some(ACCENT), None, 0, 3.5, false)
        ));
        assert!(has(
            &value,
            (36.2, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false)
        ));
        // A max below the min is the min, so the value clamps to it and the thumb sits at
        // the start, as in Chrome (paint fixture `form-native-range-max-below-min`).
        let start = (1.0, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false);
        for below in [
            "min=10 max=5 value=500",
            "min=10 max=10 value=3",
            "min=1e38",
        ] {
            let clamped = at(below);
            assert!(has(&clamped, start), "{below}: {clamped:?}");
        }
        // Non-finite or invalid numbers are ignored (`form-native-range-nan`), and huge
        // finite bounds neither overflow nor divide by zero (`form-native-range-huge`).
        let middle = (23.0, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false);
        for ignored in [
            "min=NaN max=Infinity value=NaN",
            "min=inf max=-inf value=+5",
            "min=-1e308 max=1e308 value=1e400",
            "min=5. max=' 100' value=50abc",
            "min=1e max=1e+ value=-",
            "min=-1e39 max=1e39",
        ] {
            let parts = at(ignored);
            assert!(has(&parts, middle), "{ignored}: {parts:?}");
        }
        // A leading `.` and an exponent are valid: 0.5e2 of 0..100 is the middle too.
        assert!(has(&at("min=.0e1 value=.5e2"), middle));
        // No value: the middle.
        assert!(has(
            &at(""),
            (23.0, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false)
        ));
    }

    #[test]
    fn range_values_snap_to_the_step() {
        // R2 #9 (paint fixtures `form-native-range-step*`): the nearest min + n * step,
        // the default step 1; `any` does not snap.
        let snap = |min, max, value, step| step_align(min, max, value, step, min);
        assert_eq!(step_of(None), Some(1.0));
        assert_eq!(step_of(Some("ANY")), None);
        for invalid in ["0", "-2", "x"] {
            assert_eq!(step_of(Some(invalid)), Some(1.0), "{invalid}");
        }
        assert_eq!(step_of(Some("0.5")), Some(0.5));
        assert_eq!(snap(0.0, 10.0, 4.6, Some(1.0)), 5.0);
        assert_eq!(snap(0.0, 10.0, 7.0, Some(5.0)), 5.0);
        assert_eq!(snap(0.0, 10.0, 4.6, None), 4.6);
        // The max is lowered to the last step value (9): 10 and 9.6 are 9.
        assert_eq!(snap(0.0, 10.0, 10.0, Some(3.0)), 9.0);
        assert_eq!(snap(0.0, 10.0, 8.0, Some(3.0)), 9.0);
        // A step larger than the range keeps the max (no step value but the min fits).
        assert_eq!(snap(0.0, 10.0, 10.0, Some(20.0)), 0.0);
        // From another base (the `value` attribute): 2 + 3n.
        assert_eq!(step_align(0.0, 10.0, 4.0, Some(3.0), 2.0), 5.0);
        // A base above the min: a snap below the min is the min.
        assert_eq!(step_align(1.0, 10.0, 1.0, Some(3.0), 2.5), 1.0);
        // A value rounding past the max from an odd base goes one step back: 5 + 2n
        // within 5..10 is 9 (paint fixture `form-native-range-step-base`).
        assert_eq!(step_align(5.0, 10.0, 10.0, Some(2.0), 5.0), 9.0);
        // Steps an f64 cannot count leave the value (Chrome's Decimal snaps 0.5 to 0 here,
        // `input.value`, a difference no thumb position across 2e308 can show).
        assert_eq!(snap(-1e308, 1e308, 0.5, Some(1.0)), 0.5);
        // A step-less range whose base is the value attribute: no `min`.
        let parts = parts(&format!(
            "<input type=range max=10 step=4 value=7 style='{ABS};width:60px'>"
        ));
        // Base 7: 7 + 4n within 0..10 is 7 (lowered max 7 + 0 = 7, the max stays 10).
        let thumb = 1.0 + 44.0 * 0.7;
        assert!(
            parts
                .iter()
                .any(|p| (p.0 - thumb).abs() < 0.01 && p.2 == 14.0),
            "{parts:?}"
        );
    }

    #[test]
    fn colour_wells_show_their_value() {
        let well =
            |value: &str| parts(&format!("<input type=color value='{value}' style='{ABS}'>"));
        let red = Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        });
        assert!(has(
            &well("#FF0000"),
            (6.0, 6.0, 38.0, 15.0, red, Some(SWATCH), 1, 0.0, false)
        ));
        assert!(has(
            &well("red"),
            (
                6.0,
                6.0,
                38.0,
                15.0,
                Some(Rgba::BLACK),
                Some(SWATCH),
                1,
                0.0,
                false
            )
        ));
        assert!(has(
            &well("#12345"),
            (0.0, 0.0, 50.0, 27.0, Some(FACE), Some(FRAME), 1, 2.0, false)
        ));
    }

    #[test]
    fn fields_draw_a_thin_rounded_frame_and_a_menu_list_its_chevron() {
        let field = parts(&format!(
            "<textarea style='{ABS};width:30px;height:16px'></textarea>"
        ));
        assert!(has(
            &field,
            (
                0.0,
                0.0,
                36.0,
                22.0,
                Some(WHITE),
                Some(FRAME),
                1,
                2.0,
                false
            )
        ));
        let select = parts(&format!(
            "<select style='{ABS};width:42px;height:22px'></select>"
        ));
        assert!(
            has(
                &select,
                (31.5, 6.0, 7.0, 7.0, None, Some(Rgba::BLACK), 2, 0.0, true)
            ),
            "{select:?}"
        );
    }

    #[test]
    fn restyled_and_appearance_none_controls_keep_css_painting() {
        // The UA `2px inset #767676` border's lightened right side (Chrome 156: 202).
        let css = Some(Rgba {
            r: 202,
            g: 202,
            b: 202,
            a: 255,
        });
        for style in [
            "appearance:none",
            "appearance:none;writing-mode:vertical-rl",
        ] {
            let field = parts(&format!(
                "<input style='{ABS};width:30px;height:16px;{style}'>"
            ));
            assert!(
                field.iter().any(|p| p.5 == css && p.6 == 2),
                "{style}: {field:?}"
            );
        }
        // A vertical field is native too (miaskiewicz/turbo-dom#293; it kept its CSS
        // painting before): the 1px frame with 2px corners, its sides relabelled.
        for mode in ["vertical-rl", "vertical-lr"] {
            let field = parts(&format!(
                "<input style='{ABS};width:30px;height:16px;writing-mode:{mode}'>"
            ));
            assert!(
                field
                    .iter()
                    .any(|p| p.4 == Some(WHITE) && p.5 == Some(FRAME) && p.6 == 1 && p.7 == 2.0),
                "{mode}: {field:?}"
            );
            assert!(field.iter().all(|p| p.5 != css), "{mode}: {field:?}");
        }
        let red = Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        });
        let border = parts(&format!("<input style='{ABS};border:2px solid #f00'>"));
        assert!(border.iter().any(|p| p.5 == red && p.6 == 2 && p.7 == 0.0));
        // An unstyled checkbox with `appearance: none` has no size.
        let none = parts(&format!(
            "<input type=checkbox style='{ABS};appearance:none'>"
        ));
        assert!(none.iter().all(|p| p.2 != 13.0), "{none:?}");
    }

    #[test]
    fn an_author_border_equal_to_the_uas_still_turns_the_native_look_off() {
        // miaskiewicz/turbo-dom#283: the UA outset border's darkened right (Chrome 156: 84)
        // paints instead of the native 1px #767676 frame.
        let dark = Some(Rgba {
            r: 84,
            g: 84,
            b: 84,
            a: 255,
        });
        for style in [
            "border-width:2px",
            "border-top-color:#000",
            "border-radius:0",
        ] {
            let button = parts(&format!(
                "<button style='{ABS};width:42px;height:22px;{style}'></button>"
            ));
            assert!(
                button.iter().any(|p| p.5 == dark && p.6 == 2),
                "{style}: {button:?}"
            );
            assert!(
                button.iter().all(|p| p.5 != Some(FRAME)),
                "{style}: {button:?}"
            );
        }
        // A table's `border-spacing` / `border-collapse` is no border.
        let native = parts(&format!(
            "<button style='{ABS};width:42px;height:22px;border-spacing:3px;border-collapse:collapse'></button>"
        ));
        assert!(native.iter().any(|p| p.5 == Some(FRAME)), "{native:?}");
    }

    fn attrs(pairs: &[(&str, &str)]) -> Vec<crate::Attr> {
        pairs
            .iter()
            .map(|(name, value)| crate::Attr {
                name: (*name).into(),
                value: (*value).into(),
            })
            .collect()
    }

    #[test]
    fn progress_bars_fill_their_track_to_the_value() {
        // A 160 x 16 box: the 8px bar 4px down, rounded 4px.
        let at = |a: &str| parts(&format!("<progress {a} style='{ABS}'></progress>"));
        let track = (0.0, 4.0, 160.0, 8.0, Some(FACE), Some(TRACK), 1, 4.0, false);
        let value = at("value=0.3");
        assert!(has(&value, track), "{value:?}");
        assert!(has(
            &value,
            (0.0, 4.0, 48.0, 8.0, Some(ACCENT), None, 0, 4.0, false)
        ));
        // No value: indeterminate. Chrome's first-frame bar at ~5%..19% of the box
        // (miaskiewicz/turbo-dom#346).
        let none = at("");
        assert!(has(&none, track));
        assert!(
            none.iter()
                .any(|p| p.4 == Some(ACCENT) && (p.2 - 160.0 * 0.14).abs() < 0.01),
            "{none:?}"
        );
        // A 0 value fills nothing; an author background or border, or `appearance:
        // none`, turns the native look off.
        for plain in [
            "value=0",
            "value=0.5 style='background:#ff0'",
            "value=0.5 style='border:1px solid'",
        ] {
            let p = parts(&format!("<progress {plain}></progress>"));
            assert!(p.iter().all(|p| p.4 != Some(ACCENT)), "{plain}: {p:?}");
        }
        let none = parts(&format!(
            "<progress value=0.5 style='{ABS};appearance:none'></progress>"
        ));
        assert!(none.iter().all(|p| p.4 != Some(FACE)), "{none:?}");
        // Positions: value over max (max 1 unless positive; value 0 unless a
        // non-negative number, at most the max).
        let position = |pairs: &[(&str, &str)]| progress_position(&attrs(pairs));
        assert_eq!(position(&[]), None);
        assert_eq!(position(&[("value", "3"), ("max", "4")]), Some(0.75));
        assert_eq!(position(&[("value", "2")]), Some(1.0));
        assert_eq!(position(&[("value", "-1")]), Some(0.0));
        assert_eq!(position(&[("value", "abc")]), Some(0.0));
        assert_eq!(position(&[("value", "0.5"), ("max", "0")]), Some(0.5));
        assert_eq!(position(&[("value", "0.5"), ("max", "-2")]), Some(0.5));
    }

    #[test]
    fn a_full_bar_rounds_its_end_too() {
        fn value_bar(f: &Fragment) -> Option<&Fragment> {
            match &f.content {
                FragmentContent::Box {
                    background: Some(b),
                    ..
                } if *b == ACCENT => Some(f),
                _ => f.children.iter().find_map(value_bar),
            }
        }
        let radii = |value: &str| {
            let g = galley(&format!(
                "<progress value={value} style='{ABS}'></progress>"
            ));
            let FragmentContent::Box { border_radius, .. } = &value_bar(&g).unwrap().content else {
                unreachable!("a value bar is a box")
            };
            border_radius.corners().map(|c| (c.h, c.v))
        };
        let round = (4.0, 4.0);
        assert_eq!(radii("1"), [round; 4]);
        assert_eq!(radii("0.5"), [round, (0.0, 0.0), (0.0, 0.0), round]);
    }

    #[test]
    fn meters_colour_their_value_by_region() {
        let meter = parts(&format!("<meter value=0.6 style='{ABS}'></meter>"));
        assert!(has(
            &meter,
            (
                0.0,
                4.0,
                80.0,
                8.0,
                Some(FACE),
                Some(METER_TRACK),
                1,
                4.0,
                false
            )
        ));
        assert!(has(
            &meter,
            (0.0, 4.0, 48.0, 8.0, Some(OPTIMUM), None, 0, 4.0, false)
        ));
        let value = |pairs: &[(&str, &str)]| meter_value(&attrs(pairs));
        let region = |pairs: &[(&str, &str)]| value(pairs).1;
        let bounds = [("low", "0.3"), ("high", "0.7")];
        let with = |extra: &[(&'static str, &'static str)]| {
            let mut all = bounds.to_vec();
            all.extend_from_slice(extra);
            region(&all)
        };
        // Optimum below the low.
        let low = ("optimum", "0.1");
        assert_eq!(with(&[low, ("value", "0.2")]), OPTIMUM);
        assert_eq!(with(&[low, ("value", "0.3")]), OPTIMUM);
        assert_eq!(with(&[low, ("value", "0.5")]), SUBOPTIMAL);
        assert_eq!(with(&[low, ("value", "0.7")]), SUBOPTIMAL);
        assert_eq!(with(&[low, ("value", "0.9")]), EVEN_LESS_GOOD);
        // Optimum above the high.
        let high = ("optimum", "0.9");
        assert_eq!(with(&[high, ("value", "0.8")]), OPTIMUM);
        assert_eq!(with(&[high, ("value", "0.7")]), OPTIMUM);
        assert_eq!(with(&[high, ("value", "0.5")]), SUBOPTIMAL);
        assert_eq!(with(&[high, ("value", "0.3")]), SUBOPTIMAL);
        assert_eq!(with(&[high, ("value", "0.2")]), EVEN_LESS_GOOD);
        // Optimum between (the default, the middle).
        assert_eq!(with(&[("value", "0.5")]), OPTIMUM);
        assert_eq!(with(&[("value", "0.3")]), OPTIMUM);
        assert_eq!(with(&[("value", "0.7")]), OPTIMUM);
        assert_eq!(with(&[("value", "0.9")]), SUBOPTIMAL);
        assert_eq!(with(&[("value", "0.1")]), SUBOPTIMAL);
        // The defaults: 0..1, value 0; a high below the low is the low (Chrome 156: the
        // boundary values here and above measured on its meter).
        assert_eq!(value(&[]), (0.0, OPTIMUM));
        assert_eq!(
            region(&[("low", "0.6"), ("high", "0.2"), ("value", "0.6")]),
            OPTIMUM
        );
        assert_eq!(
            region(&[("low", "0.6"), ("high", "0.2"), ("value", "0.61")]),
            EVEN_LESS_GOOD
        );
        // Clamped values and a range over its bounds; a max below the min is the min,
        // and fills nothing.
        assert_eq!(
            value(&[("min", "10"), ("max", "20"), ("value", "15")]).0,
            0.5
        );
        assert_eq!(value(&[("value", "5")]).0, 1.0);
        assert_eq!(value(&[("value", "-5")]).0, 0.0);
        assert_eq!(value(&[("min", "10"), ("max", "5"), ("value", "7")]).0, 0.0);
        assert_eq!(
            value(&[("min", "-1e308"), ("max", "1e308"), ("value", "0")]).0,
            0.5
        );
    }

    #[test]
    fn an_indeterminate_checkbox_paints_a_dash() {
        let dash = (3.6, 5.5, 7.8, 2.0, Some(WHITE), None, 0, 1.0, false);
        for checked in ["", "checked"] {
            let p = parts(&format!(
                "<input type=checkbox {checked} data-indeterminate style='{ABS}'>"
            ));
            assert!(has(&p, dash), "{checked}: {p:?}");
            assert!(has(
                &p,
                (0.0, 0.0, 13.0, 13.0, Some(ACCENT), None, 0, 2.0, false)
            ));
            assert!(p.iter().all(|p| !p.8), "no check: {p:?}");
        }
        // A radio button's flag changes nothing.
        let radio = parts(&format!(
            "<input type=radio data-indeterminate style='{ABS}'>"
        ));
        assert!(radio.iter().all(|p| p.4 != Some(ACCENT)), "{radio:?}");
        // In a vertical mode the dash keeps its physical shape and place.
        for mode in ["vertical-lr", "vertical-rl"] {
            let p = parts(&format!(
                "<div style='{ABS};writing-mode:{mode}'><input type=checkbox data-indeterminate style='display:block;margin:0'></div>"
            ));
            assert!(has(&p, dash), "{mode}: {p:?}");
        }
    }

    #[test]
    fn vertical_controls_turn_with_the_flow_and_keep_their_glyphs() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let s = (s * 1000.0).round() / 1000.0;
        // A 42 x 22 menu list: its chevron 7px from the bottom (the inline end), 1.5px
        // before the block middle (left of it in `vertical-lr`, right in `vertical-rl`),
        // pointing right in `vertical-lr` and left in `vertical-rl`; the right and bottom
        // strokes it is drawn with stay physical.
        let select = |mode: &str| {
            turned(&format!(
                "<div style='{ABS};writing-mode:{mode}'><select style='display:block;width:42px;height:22px'></select></div>"
            ))
        };
        let strokes = [false, true, true, false];
        assert_eq!(
            select("vertical-lr"),
            vec![(16.0, 11.5, 7.0, 7.0, [s, -s, s, s], strokes)]
        );
        assert_eq!(
            select("vertical-rl"),
            vec![(19.0, 11.5, 7.0, 7.0, [-s, s, -s, -s], strokes)]
        );
        // A checked checkbox's check stands as in a horizontal one.
        for mode in ["vertical-lr", "vertical-rl"] {
            let check = turned(&format!(
                "<div style='{ABS};writing-mode:{mode}'><input type=checkbox checked style='display:block;margin:0'></div>"
            ));
            assert_eq!(check.len(), 1, "{mode}: {check:?}");
            let (x, y, w, h, matrix, sides) = check[0];
            assert!(
                (x - 5.3).abs() < 1e-4 && (y - 0.9).abs() < 1e-4,
                "{mode}: {check:?}"
            );
            assert_eq!(
                (w, h, matrix, sides),
                (3.0, 9.0, [s, s, -s, s], strokes),
                "{mode}"
            );
        }
        // A vertical range's track runs down: blue from the top to the thumb.
        let range = parts(&format!(
            "<div style='{ABS};writing-mode:vertical-lr'><input type=range value=20 style='display:block;margin:0'></div>"
        ));
        assert!(
            has(
                &range,
                (4.5, 0.5, 7.0, 128.0, Some(FACE), Some(TRACK), 1, 3.5, false)
            ),
            "{range:?}"
        );
        // A vertical colour well's swatch: 4px in from its sides, 3px from top and bottom.
        let well = parts(&format!(
            "<div style='{ABS};writing-mode:vertical-rl'><input type=color style='display:block;margin:0'></div>"
        ));
        assert!(
            well.iter()
                .any(|p| p.5 == Some(SWATCH) && p.2 == 15.0 && p.3 == 38.0),
            "{well:?}"
        );
        // A vertical progress fills from the top.
        let progress = parts(&format!(
            "<div style='{ABS};writing-mode:vertical-rl'><progress value=0.25 style='display:block'></progress></div>"
        ));
        assert!(
            has(
                &progress,
                (4.0, 0.0, 8.0, 40.0, Some(ACCENT), None, 0, 4.0, false)
            ),
            "{progress:?}"
        );
    }

    #[test]
    fn a_progress_baseline_is_lowered_by_a_fifth_of_its_font() {
        let boxes = |body: &str| {
            let html = format!(
                "<!DOCTYPE html><style>body{{margin:0;font:20px/30px 'Liberation Serif'}}</style><body>{body}"
            );
            let mut diags = crate::Diagnostics::default();
            crate::layout_boxes(
                &html,
                "",
                400.0,
                300.0,
                &crate::test_support::fixture_fonts(),
                &mut diags,
            )
            .unwrap()
            .into_iter()
            .map(|b| (b.cid, b.x, b.y, b.width, b.height))
            .collect::<Vec<_>>()
        };
        // Chrome 156 (form-native control `progress-meter-baseline`): 20px font, 30px
        // lines, the 200 x 20 progress at 6px down.
        let inline = boxes("<div>x<progress data-cid=m value=0.5></progress>x</div>");
        assert_eq!(inline[0].2, 6.0, "{inline:?}");
        assert_eq!((inline[0].3, inline[0].4), (200.0, 20.0));
        // An author `vertical-align` replaces the UA's.
        let baseline =
            boxes("<div>x<progress data-cid=m value=0.5 style='vertical-align:baseline'></progress>x</div>");
        assert_eq!(baseline[0].2, 2.0, "{baseline:?}");
        // Vertical lines: lowered toward the left in both modes (Chrome 156: 1px in).
        for mode in ["vertical-lr", "vertical-rl"] {
            let vertical = boxes(&format!(
                "<div style='writing-mode:{mode}'><progress data-cid=m value=0.5></progress></div>"
            ));
            assert_eq!(vertical[0].1, 1.0, "{mode}: {vertical:?}");
        }
    }
}

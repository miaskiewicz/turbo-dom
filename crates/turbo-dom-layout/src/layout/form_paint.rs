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
//! - a checkbox: the frame on white, `checked` a `#0075ff` square with a white check; a
//!   radio button a circle, `checked` a blue disc with a white ring;
//! - a range slider: a 7px track (blue up to the thumb, `#efefef` framed in `#b2b2b2`
//!   after it) and a blue round thumb (1px inside its box) at its value.
//!
//! Text fields, textareas, menu lists, buttons and colour wells fall back to their CSS
//! painting when the author restyles their border or background (as Chrome does), and
//! every control with `appearance: none` (and in vertical writing modes, whose rotated
//! native look is not drawn).
//! Measured against the paint Chrome 156 control's `form-*` cases.
use super::fragment::{BorderRadii, Fragment, FragmentContent, Transform2D};
use super::value::{BorderEdges, BorderSide, BoxStyle, Rgba};
use crate::Attr;

use super::form_controls::Control;

const FRAME: Rgba = rgb(0x76, 0x76, 0x76);
const WHITE: Rgba = rgb(0xff, 0xff, 0xff);
const FACE: Rgba = rgb(0xef, 0xef, 0xef);
const ACCENT: Rgba = rgb(0x00, 0x75, 0xff);
const TRACK: Rgba = rgb(0xb2, 0xb2, 0xb2);
const SWATCH: Rgba = rgb(0x77, 0x77, 0x77);

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba { r, g, b, a: 255 }
}

/// What the control box `frag` (of `control`, with `attrs` and used style `bs`) paints:
/// Chrome's native look in place of its CSS decoration, when it has it.
pub(crate) fn apply(
    control: Control,
    attrs: &[Attr],
    style: &crate::ComputedStyle,
    bs: &BoxStyle,
    frag: &mut Fragment,
) {
    if !native(control, style, frag) {
        return;
    }
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let mut parts = Vec::new();
    let own = match control {
        Control::TextField | Control::TextArea => framed(WHITE, 2.0),
        Control::Select => {
            parts.push(chevron(frag));
            framed(WHITE, 2.0)
        }
        Control::Button => framed(FACE, 2.0),
        Control::Color => {
            let b = bs.border.widths();
            let p = bs.padding;
            parts.push(rect(
                frag,
                (x + b.left + p.left + 3.0, y + b.top + p.top + 4.0),
                (
                    w - b.horizontal() - p.horizontal() - 6.0,
                    h - b.vertical() - p.vertical() - 8.0,
                ),
                swatch_colour(attrs),
                Some(SWATCH),
                0.0,
            ));
            framed(FACE, 2.0)
        }
        Control::Toggle => return toggle(attrs, frag),
    };
    frag.content = own;
    frag.children.splice(0..0, parts);
}

/// Whether `frag` paints Chrome's native look: a default `appearance`, horizontal, and
/// (for the controls an author can restyle) the UA sheet's border and background.
fn native(control: Control, style: &crate::ComputedStyle, frag: &Fragment) -> bool {
    let appearance = style.get("appearance").map(str::trim);
    if matches!(appearance, Some("none")) || super::writing::WritingMode::of(style).vertical() {
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
    let (width, face) = match control {
        Control::Toggle => return true,
        Control::TextField => (2, WHITE),
        Control::Button => (2, FACE),
        Control::TextArea | Control::Select => (1, WHITE),
        Control::Color => (1, FACE),
    };
    let side = BorderSide {
        width,
        color: Some(FRAME),
    };
    *border
        == (BorderEdges {
            top: side,
            right: side,
            bottom: side,
            left: side,
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

/// An L of two `width`px white or black strokes (a box's right and bottom border)
/// turned 45° clockwise: `(w, h)` is the box, centred at `(cx, cy)`.
fn turned_l(
    owner: &Fragment,
    (cx, cy): (f32, f32),
    (w, h): (f32, f32),
    stroke: (u16, Rgba),
) -> Fragment {
    let (width, color) = stroke;
    let side = BorderSide {
        width,
        color: Some(color),
    };
    let none = BorderSide::default();
    let s = std::f32::consts::FRAC_1_SQRT_2;
    Fragment::new(
        owner.node_id,
        cx - w / 2.0,
        cy - h / 2.0,
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
                matrix: [s, s, -s, s, 0.0, 0.0],
                origin_x: w / 2.0,
                origin_y: h / 2.0,
            }),
        },
    )
}

/// A menu list's chevron: tip 7px from its right edge, 2px below its middle.
fn chevron(frag: &Fragment) -> Fragment {
    let side = 5.0;
    turned_l(
        frag,
        (frag.x + frag.width - 7.0, frag.y + frag.height / 2.0 - 1.5),
        (side, side),
        (2, Rgba::BLACK),
    )
}

/// A checkbox, radio button or range slider (`frag` is its border box).
fn toggle(attrs: &[Attr], frag: &mut Fragment) {
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
            FragmentContent::Box {
                background: None,
                border: BorderEdges::default(),
                border_radius: BorderRadii::ZERO,
                shadow: None,
                gradient: None,
                transform: None,
            }
        }
        (_, false) => framed(WHITE, 2.0),
        (_, true) => {
            let k = w.min(h) / 13.0;
            parts.push(turned_l(
                frag,
                (x + 6.8 * k, y + 5.4 * k),
                (3.0 * k, 9.0 * k),
                (2, WHITE),
            ));
            decoration(ACCENT, None, 2.0)
        }
    };
    frag.children.splice(0..0, parts);
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

/// The (clamped) `value` aligned to `step` from `base`, as Chrome's StepRange: snapped to
/// the nearest step value, at most the last step value at or below the max (the max
/// lowered to it), at least the min. No step (`any`), or a step finer than an `f64` can
/// count across the range, leaves it as it is.
fn step_align(min: f64, max: f64, value: f64, step: Option<f64>, base: f64) -> f64 {
    let Some(step) = step else {
        return value;
    };
    // Halved, so `max - base` cannot overflow to infinity.
    let steps = (max / 2.0 - base / 2.0) / (step / 2.0);
    if !(steps.abs() < 9_007_199_254_740_992.0) {
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
    use super::{step_align, step_of, ACCENT, FACE, FRAME, SWATCH, TRACK, WHITE};

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
        let html = format!("<!DOCTYPE html><style>body{{margin:0}}</style><body>{body}");
        let fonts = crate::FontRegistry::new();
        let mut diags = crate::Diagnostics::default();
        let galley = crate::layout_html(&html, "", 400.0, &fonts, &mut diags).unwrap();
        let mut out = Vec::new();
        walk(&galley, &mut out);
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
                (32.5, 7.0, 5.0, 5.0, None, Some(Rgba::BLACK), 2, 0.0, true)
            ),
            "{select:?}"
        );
    }

    #[test]
    fn restyled_vertical_and_appearance_none_controls_keep_css_painting() {
        let css = Some(Rgba {
            r: 0x76,
            g: 0x76,
            b: 0x76,
            a: 255,
        });
        for style in ["appearance:none", "writing-mode:vertical-rl"] {
            let field = parts(&format!(
                "<input style='{ABS};width:30px;height:16px;{style}'>"
            ));
            assert!(
                field.iter().any(|p| p.5 == css && p.6 == 2),
                "{style}: {field:?}"
            );
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
}

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
/// 0 and 100; the value defaults to their middle and is clamped to them).
fn range(attrs: &[Attr], frag: &Fragment) -> Vec<Fragment> {
    let number = |name| attr(attrs, name).and_then(|v| v.trim().parse::<f32>().ok());
    let min = number("min").unwrap_or(0.0);
    let max = number("max").filter(|m| *m > min).unwrap_or(min + 100.0);
    let value = number("value").unwrap_or((min + max) / 2.0).clamp(min, max);
    let (x, y, w, h) = (frag.x, frag.y, frag.width, frag.height);
    let r = h / 2.0;
    let thumb = x + r + (w - 2.0 * r) * (value - min) / (max - min);
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
    use super::{ACCENT, FACE, FRAME, SWATCH, TRACK, WHITE};

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
        // A max not above min falls back to min + 100; a value past it clamps to it.
        let clamped = at("min=10 max=5 value=500");
        assert!(
            has(
                &clamped,
                (45.0, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false)
            ),
            "{clamped:?}"
        );
        // No value: the middle.
        assert!(has(
            &at(""),
            (23.0, 1.0, 14.0, 14.0, Some(ACCENT), None, 0, 7.0, false)
        ));
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

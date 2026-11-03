//! Chrome 156's intrinsic sizes of the text-bearing form controls
//! (miaskiewicz/turbo-dom#239). The UA stylesheet (`style::UA_CSS`) gives every control
//! Chrome's `html.css` font, border and padding; what depends on the font is sized here,
//! from the control's primary face at its used font size:
//!
//! - a text field (`<input>` of a text-like type) is `size` (default 20) average
//!   characters wide plus one maximum character less one average one, rounded up, and
//!   one line tall (Blink's `TextFieldIntrinsicInlineSize`);
//! - a `<textarea>` is `cols` (default 20) average characters plus the 16px scrollbar wide
//!   and `rows` (default 2) lines tall;
//! - a `<select>` (a drop-down menu list) is its widest option's text plus 16px for the
//!   arrow wide, and its face's ascent plus descent plus 2px tall.
//!
//! The metrics follow Blink's rounded font metrics: the line height is the rounded
//! ascent, descent and line gap; the average character is the `x` advance (rounded for
//! a proportional TrueType face, as Chrome's control metrics come out for the fixture
//! faces: Liberation Serif, Roboto and PT Serif round, Fira Code, IBM Plex Mono and Inter
//! do not); the maximum character is the rounded ascent. Measured against the
//! live-geometry Chrome 156 control's `form-*` cases.
use crate::{FontRegistry, StyledElement, StyledNode, Tag};

/// The form controls this module sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    TextField,
    TextArea,
    Select,
    /// A checkbox, radio button or range slider: a fixed-size box (the UA sheet's)
    /// whose baseline is its border-box bottom.
    Toggle,
    /// A `<button>` or a button `<input>` (`submit` / `reset` / `button`): its label lays
    /// out as usual; an empty one's baseline is its content-box top.
    Button,
}

impl Control {
    /// Whether the control shows text this module supplies and sizes
    /// ([`intrinsic_size`]): a text field, textarea or menu list.
    pub(crate) fn sized(self) -> bool {
        matches!(
            self,
            Control::TextField | Control::TextArea | Control::Select
        )
    }
}

/// The control `el` is, if any: a text-like `<input>` (no `type`, an unknown one, or
/// `text` / `search` / `url` / `tel` / `email` / `password` / `number`), a `<textarea>`
/// or a single-line `<select>` (no `multiple`, no `size` above 1).
pub(crate) fn control_of(el: &StyledElement) -> Option<Control> {
    let html = |tag: &str| matches!(&el.tag, Tag::Html(name) if name == tag);
    if html("input") {
        input_control(attr(el, "type"))
    } else if html("button") {
        Some(Control::Button)
    } else if html("textarea") {
        Some(Control::TextArea)
    } else {
        (html("select")
            && attr(el, "multiple").is_none()
            && positive(attr(el, "size")).unwrap_or(1) <= 1)
            .then_some(Control::Select)
    }
}

fn input_control(kind: Option<&str>) -> Option<Control> {
    let kind = kind.unwrap_or("text").trim().to_ascii_lowercase();
    match kind.as_str() {
        "checkbox" | "radio" | "range" => Some(Control::Toggle),
        "submit" | "reset" | "button" => Some(Control::Button),
        "hidden" | "color" | "file" | "image" | "date" | "datetime-local" | "month" | "week"
        | "time" => None,
        _ => Some(Control::TextField),
    }
}

fn attr<'a>(el: &'a StyledElement, name: &str) -> Option<&'a str> {
    el.attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

/// A valid positive integer attribute (HTML's rules: leading whitespace, the leading
/// digits; `0` and garbage are not).
pub(crate) fn positive(value: Option<&str>) -> Option<u32> {
    let value = value?.trim_start_matches([' ', '\t', '\n', '\x0c', '\r']);
    let value = value.strip_prefix('+').unwrap_or(value);
    let digits = value.len() - value.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    value[..digits].parse().ok().filter(|n| *n > 0)
}

/// The text a `<select>`'s options show (whitespace collapsed) and the text its width
/// measures (an option in an `<optgroup>` indented by four spaces, as Chrome's menu list
/// sizes it), in document order, the selected one (the last with `selected`, else the
/// first) first.
pub(crate) fn select_options(el: &StyledElement) -> Vec<(String, String)> {
    let mut options = Vec::new();
    let mut selected = None;
    collect_options(&el.children, "", &mut options, &mut selected);
    if let Some(index) = selected {
        let chosen = options.remove(index);
        options.insert(0, chosen);
    }
    options
}

fn collect_options(
    nodes: &[StyledNode],
    indent: &str,
    out: &mut Vec<(String, String)>,
    selected: &mut Option<usize>,
) {
    for node in nodes {
        let StyledNode::Element(el) = node else {
            continue;
        };
        match &el.tag {
            Tag::Html(name) if name == "option" => {
                if attr(el, "selected").is_some() {
                    *selected = Some(out.len());
                }
                let text = collapse(&text_of(&el.children));
                out.push((text.clone(), format!("{indent}{text}")));
            }
            Tag::Html(name) if name == "optgroup" => {
                collect_options(&el.children, "    ", out, selected);
            }
            _ => {}
        }
    }
}

/// The text children of an `<option>` or a `<textarea>` (the parser keeps only text
/// in them; an element child contributes nothing).
pub(crate) fn text_of(nodes: &[StyledNode]) -> String {
    nodes
        .iter()
        .filter_map(|node| match node {
            StyledNode::Text(t) => Some(t.text.as_str()),
            StyledNode::Element(_) => None,
        })
        .collect()
}

fn collapse(text: &str) -> String {
    text.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

/// The attribute that carries a `<select>`'s measured option texts to layout (one per
/// option, in [`select_options`] order) on its box: the box has no option children.
pub(crate) const OPTION_ATTR: &str = "\u{0}option";

/// The rounded font metrics a control sizes with.
struct Metrics {
    ascent: f32,
    descent: f32,
    line: f32,
    avg: f32,
}

fn metrics(face: &crate::FontFace, size: f32) -> Metrics {
    let ascent = face.ascent_px(size).round();
    let descent = face.descent_px(size).round();
    let gap = (face.line_height_px(size) - face.ascent_px(size) - face.descent_px(size)).round();
    let x = face.measure("x", size, 0.0);
    let avg = if face.is_fixed_pitch() || face.is_cff() {
        x
    } else {
        x.round()
    };
    Metrics {
        ascent,
        descent,
        line: ascent + descent + gap,
        avg,
    }
}

/// The content-box `(width, height)` of a `control` box with `attrs`, in the face its
/// style selects (`None` when no face is registered).
pub(crate) fn intrinsic_size(
    control: Control,
    attrs: &[crate::Attr],
    style: &super::value::BoxStyle,
    fonts: &FontRegistry,
) -> Option<(f32, f32)> {
    let families: Vec<&str> = style.font_families.iter().map(String::as_str).collect();
    let face = fonts.select(&families, style.font_weight, style.italic)?;
    let size = style.font_size;
    let m = metrics(face, size);
    let number = |name: &str, default: u32| {
        let value = attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str());
        positive(value).unwrap_or(default) as f32
    };
    Some(match control {
        Control::Toggle | Control::Button => return None,
        Control::TextField => (
            (number("size", 20) * m.avg + m.ascent - m.avg).ceil(),
            m.line,
        ),
        Control::TextArea => (
            (number("cols", 20) * m.avg).ceil() + SCROLLBAR,
            number("rows", 2) * m.line,
        ),
        Control::Select => {
            let widest = attrs
                .iter()
                .filter(|a| a.name == OPTION_ATTR)
                .map(|a| face.measure(&a.value, size, 0.0))
                .fold(0.0_f32, f32::max);
            ((widest + ARROW).ceil(), m.ascent + m.descent + 2.0)
        }
    })
}

/// The width of a textarea's vertical scrollbar.
const SCROLLBAR: f32 = 16.0;

/// The room a menu list keeps for its arrow.
const ARROW: f32 = 16.0;

#[cfg(test)]
mod tests {
    use super::{positive, text_of};

    #[test]
    fn only_text_children_count() {
        let element = crate::StyledElement {
            tag: crate::Tag::Html("b".into()),
            attrs: Vec::new(),
            style: crate::ComputedStyle::default(),
            children: vec![crate::StyledNode::Text("x".into())],
            source: None,
        };
        let nodes = [
            crate::StyledNode::Text("a".into()),
            crate::StyledNode::Element(element),
            crate::StyledNode::Text("b".into()),
        ];
        assert_eq!(text_of(&nodes), "ab");
    }

    #[test]
    fn positive_integers_parse_like_html() {
        assert_eq!(positive(Some(" +12px")), Some(12));
        assert_eq!(positive(Some("0")), None);
        assert_eq!(positive(Some("x")), None);
        assert_eq!(positive(None), None);
    }
}

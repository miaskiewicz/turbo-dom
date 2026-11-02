//! CanvasRenderingContext2D `measureText` (miaskiewicz/turbo-dom#237): the context's `font`
//! resolves a face from the host's font registry ([`set_canvas_fonts`]) and
//! turbo-dom-layout measures the run like Chrome 156
//! ([`FontFace::canvas_text_metrics`](turbo_dom_layout::FontFace::canvas_text_metrics));
//! this module only parses the font and applies the `textAlign` / `textBaseline` offsets.
use super::*;
use turbo_dom_layout::{CanvasTextMetrics, FontRegistry};
use turbo_dom_parser::rtdom::css_shorthand;

/// The fonts canvas `measureText` resolves `ctx.font` against, per context.
struct CanvasFonts(FontRegistry);

/// Give the canvases of `context` their fonts: the host's registry of the page's loaded
/// `@font-face` faces and whichever system fonts it lays the page out with (turbo-surf
/// passes the registry its live layout uses, e.g. `FontRegistry::system(Html)` plus the
/// fetched faces). Without one the canvas measures with turbo-dom-layout's bundled faces.
pub fn set_canvas_fonts(context: v8::Local<v8::Context>, fonts: FontRegistry) {
    context.set_slot(std::rc::Rc::new(CanvasFonts(fonts)));
}

/// A canvas `font` resolved to what face selection needs.
#[derive(Debug, PartialEq)]
struct CanvasFont {
    size: f32,
    weight: u16,
    italic: bool,
    families: Vec<String>,
}

/// The canvas default, `10px sans-serif`.
fn default_font() -> CanvasFont {
    CanvasFont {
        size: 10.0,
        weight: 400,
        italic: false,
        families: vec!["sans-serif".into()],
    }
}

/// An absolute `font-size` in px (relative sizes resolve against the 10px default).
fn font_size(value: &str) -> Option<f32> {
    const UNITS: &[(&str, f32)] = &[
        ("px", 1.0),
        ("pt", 4.0 / 3.0),
        ("pc", 16.0),
        ("in", 96.0),
        ("cm", 96.0 / 2.54),
        ("mm", 96.0 / 25.4),
        ("rem", 10.0),
        ("em", 10.0),
        ("%", 0.1),
    ];
    UNITS.iter().find_map(|(unit, px)| {
        let number = value.strip_suffix(unit)?.parse::<f32>().ok()?;
        Some(number * px).filter(|size| size.is_finite() && *size >= 0.0)
    })
}

fn font_weight(value: &str) -> u16 {
    match value {
        "bold" | "bolder" => 700,
        "lighter" => 100,
        other => other
            .parse::<f32>()
            .map_or(400, |weight| weight.clamp(1.0, 1000.0) as u16),
    }
}

/// `ctx.font` through turbo-dom's `font` shorthand grammar; `None` for a value Chrome's
/// canvas ignores.
fn parse_font(value: &str) -> Option<CanvasFont> {
    let mut font = default_font();
    for longhand in css_shorthand::expand("font", value)? {
        let value = longhand.value.trim().to_ascii_lowercase();
        match longhand.name {
            "font-size" => font.size = font_size(&value)?,
            "font-weight" => font.weight = font_weight(&value),
            "font-style" => font.italic = value != "normal",
            "font-family" => {
                font.families = longhand
                    .value
                    .split(',')
                    .map(|family| family.trim().trim_matches(['"', '\'']).trim().to_string())
                    .filter(|family| !family.is_empty())
                    .collect();
            }
            _ => {}
        }
    }
    Some(font)
}

/// The run's metrics at the alphabetic baseline under `font` (an unparsable font measures
/// as the default).
fn measure(fonts: Option<&FontRegistry>, text: &str, font: &str) -> CanvasTextMetrics {
    let font = parse_font(font).unwrap_or_else(default_font);
    let bundled;
    let registry = if let Some(fonts) = fonts {
        fonts
    } else {
        bundled = FontRegistry::new();
        &bundled
    };
    let families: Vec<&str> = font.families.iter().map(String::as_str).collect();
    let face = registry
        .select(&families, font.weight, font.italic)
        .expect("turbo-dom-layout's bundled faces are always registered");
    face.canvas_text_metrics(text, font.size)
}

/// How far the `textAlign` anchor sits right of the run's start.
fn align_offset(align: &str, direction: &str, width: f32) -> f32 {
    let rtl = direction == "rtl";
    match align {
        "center" => width / 2.0,
        "right" => width,
        "end" if !rtl => width,
        "start" if rtl => width,
        _ => 0.0,
    }
}

/// The hanging baseline: `kHangingAsPercentOfAscent` (80) of the ascent, divided by 100 in
/// `f32` as Blink does (not a multiply by 0.8, which rounds differently).
fn hanging(m: &CanvasTextMetrics) -> f32 {
    m.font_ascent * 80.0 / 100.0
}

/// How far the `textBaseline` sits above the alphabetic baseline (Blink
/// `TextMetrics::GetFontBaseline`).
fn baseline_offset(baseline: &str, m: &CanvasTextMetrics) -> f32 {
    match baseline {
        "top" => m.em_ascent,
        "hanging" => hanging(m),
        "middle" => (m.em_ascent - m.em_descent) / 2.0,
        "ideographic" => -m.font_descent,
        "bottom" => -m.em_descent,
        _ => 0.0,
    }
}

/// The `TextMetrics` fields in the binding's order: width, actualBoundingBox{Left, Right,
/// Ascent, Descent}, fontBoundingBox{Ascent, Descent}, hangingBaseline,
/// alphabeticBaseline, ideographicBaseline.
fn fields(m: &CanvasTextMetrics, align: &str, baseline: &str, direction: &str) -> [f64; 10] {
    let dx = align_offset(align, direction, m.width as f32);
    let dy = baseline_offset(baseline, m);
    [
        m.width,
        f64::from(m.ink_left + dx),
        f64::from(m.ink_right - dx),
        f64::from(m.ink_ascent - dy),
        f64::from(m.ink_descent + dy),
        f64::from(m.font_ascent - dy),
        f64::from(m.font_descent + dy),
        f64::from(hanging(m) - dy),
        f64::from(-dy),
        f64::from(-m.font_descent - dy),
    ]
}

/// `__canvasMeasureTextNative(text, font, textAlign, textBaseline, direction)`: the
/// `TextMetrics` fields ([`fields`]) as an array.
pub(super) fn measure_text_native(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let text = arg_str(scope, &args, 0);
    let font = arg_str(scope, &args, 1);
    let align = arg_str(scope, &args, 2);
    let baseline = arg_str(scope, &args, 3);
    let direction = arg_str(scope, &args, 4);
    let fonts = scope.get_current_context().get_slot::<CanvasFonts>();
    let metrics = measure(fonts.as_deref().map(|fonts| &fonts.0), &text, &font);
    let values = fields(&metrics, &align, &baseline, &direction)
        .map(|value| v8::Number::new(scope, value).into());
    rv.set(v8::Array::new_with_elements(scope, &values).into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_shorthand_resolves_size_weight_style_and_families() {
        let font = parse_font("italic bold 21.5px \"Liberation Serif\", 'Inter', serif").unwrap();
        assert_eq!(
            font,
            CanvasFont {
                size: 21.5,
                weight: 700,
                italic: true,
                families: vec!["Liberation Serif".into(), "Inter".into(), "serif".into()],
            }
        );
        let font = parse_font("oblique 300 18pt Inter").unwrap();
        assert_eq!((font.size, font.weight, font.italic), (24.0, 300, true));
        assert_eq!(parse_font("lighter 2em x").unwrap().weight, 100);
        assert_eq!(parse_font("bolder 1in x").unwrap().size, 96.0);
        assert_eq!(parse_font("normal 50% x").unwrap().size, 5.0);
        assert_eq!(parse_font("not a font"), None);
        assert_eq!(parse_font("12px"), None);
    }

    #[test]
    fn font_size_units() {
        assert_eq!(font_size("12pc"), Some(192.0));
        assert_eq!(font_size("2rem"), Some(20.0));
        assert!((font_size("2.54cm").unwrap() - 96.0).abs() < 1e-3);
        assert!((font_size("25.4mm").unwrap() - 96.0).abs() < 1e-3);
        assert_eq!(font_size("large"), None);
    }

    #[test]
    fn rtl_start_and_end_swap_anchors() {
        assert_eq!(align_offset("start", "rtl", 10.0), 10.0);
        assert_eq!(align_offset("end", "rtl", 10.0), 0.0);
        assert_eq!(align_offset("end", "ltr", 10.0), 10.0);
        assert_eq!(align_offset("left", "rtl", 10.0), 0.0);
    }

    #[test]
    fn an_unparsable_font_measures_as_the_default() {
        let fonts = FontRegistry::new();
        assert_eq!(
            measure(Some(&fonts), "Wave", "bogus"),
            measure(None, "Wave", "10px sans-serif")
        );
    }
}

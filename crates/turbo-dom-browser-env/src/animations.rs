//! Web Animations host seam: trusted playback-event dispatch, the keyframe property/value
//! admission Chrome applies while processing keyframes, pseudo-element validation and the
//! document's browsing-context state. The timing model itself lives in animations.js.
use super::*;
use cssparser::{Parser, ParserInput, Token};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::stylesheet::ParserOptions;

/// Properties Chrome refuses as keyframe members even though they are CSS properties
/// (measured: Chrome 156 drops every one of these from getKeyframes()).
const NOT_ANIMATABLE: &[&str] = &[
    "all",
    "animation",
    "animation-composition",
    "animation-delay",
    "animation-direction",
    "animation-duration",
    "animation-fill-mode",
    "animation-iteration-count",
    "animation-name",
    "animation-play-state",
    "animation-range",
    "animation-range-end",
    "animation-range-start",
    "animation-timeline",
    "animation-timing-function",
    "contain",
    "container",
    "container-name",
    "container-type",
    "direction",
    "text-orientation",
    "transition",
    "transition-behavior",
    "transition-delay",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    "unicode-bidi",
    "will-change",
    "writing-mode",
];

/// Keyframe member name (the CSS property to IDL attribute mapping) to its CSS property.
/// Prefixed IDL names, dashed names and non-animatable properties are not keyframe members.
fn keyframe_property(name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix("--") {
        return (!rest.is_empty()).then(|| name.to_owned());
    }
    let css = match name {
        "cssFloat" => "float".to_owned(),
        "cssOffset" => "offset".to_owned(),
        "float" | "offset" => return None,
        _ => {
            if name.starts_with("webkit")
                || !name.starts_with(|c: char| c.is_ascii_lowercase())
                || !name.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return None;
            }
            let mut css = String::with_capacity(name.len() + 4);
            for c in name.chars() {
                if c.is_ascii_uppercase() {
                    css.push('-');
                    css.push(c.to_ascii_lowercase());
                } else {
                    css.push(c);
                }
            }
            css
        }
    };
    // Lightning CSS does not model the motion-path `offset` shorthand Chrome animates.
    ((super::cssom_parser::is_supported_property(&css) || css == "offset")
        && !NOT_ANIMATABLE.contains(&css.as_str()))
    .then_some(css)
}

fn number(value: f32) -> String {
    let value = f64::from(value);
    if value == 0.0 {
        return "0".into();
    }
    let digits = 5 - value.abs().log10().floor() as i32;
    let scale = 10f64.powi(digits.clamp(0, 12));
    let rounded = (value * scale).round() / scale;
    let text = format!("{rounded}");
    if text == "-0" {
        "0".into()
    } else {
        text
    }
}

/// Canonical spelling of a transform function name; other function names lowercase.
fn function_name(name: &str) -> String {
    const CAMEL: &[&str] = &[
        "translateX",
        "translateY",
        "translateZ",
        "translate3d",
        "scaleX",
        "scaleY",
        "scaleZ",
        "scale3d",
        "rotateX",
        "rotateY",
        "rotateZ",
        "rotate3d",
        "skewX",
        "skewY",
        "matrix3d",
    ];
    CAMEL
        .iter()
        .find(|known| known.eq_ignore_ascii_case(name))
        .map_or_else(|| name.to_ascii_lowercase(), |known| (*known).to_owned())
}

/// Specified-value serialization of a validated declaration value: numbers and dimensions
/// at six significant digits, keywords and units lowercased, single spaces, `, ` lists.
fn serialize_tokens(input: &mut Parser<'_, '_>, out: &mut String, zero_length: bool) {
    let mut space = false;
    let push = |out: &mut String, space: &mut bool, text: &str| {
        if *space && !out.is_empty() && !out.ends_with('(') {
            out.push(' ');
        }
        *space = false;
        out.push_str(text);
    };
    while let Ok(token) = input.next_including_whitespace() {
        let token = token.clone();
        match token {
            Token::WhiteSpace(_) => space = true,
            Token::Comma => {
                out.push_str(", ");
                space = false;
            }
            Token::Number { value, .. } => {
                let text = number(value);
                let text = if zero_length && text == "0" {
                    "0px".into()
                } else {
                    text
                };
                push(out, &mut space, &text)
            }
            Token::Percentage { unit_value, .. } => {
                push(out, &mut space, &format!("{}%", number(unit_value * 100.0)))
            }
            Token::Dimension { value, unit, .. } => push(
                out,
                &mut space,
                &format!("{}{}", number(value), unit.to_ascii_lowercase()),
            ),
            Token::Ident(name) => push(out, &mut space, &name.to_ascii_lowercase()),
            Token::Function(name) => {
                push(out, &mut space, &format!("{}(", function_name(&name)));
                let _ = input.parse_nested_block(|input| {
                    serialize_tokens(input, out, false);
                    Ok::<(), cssparser::ParseError<'_, ()>>(())
                });
                out.push(')');
            }
            Token::ParenthesisBlock => {
                push(out, &mut space, "(");
                let _ = input.parse_nested_block(|input| {
                    serialize_tokens(input, out, false);
                    Ok::<(), cssparser::ParseError<'_, ()>>(())
                });
                out.push(')');
            }
            Token::Delim(c) => {
                let text = c.to_string();
                push(out, &mut space, &text);
                if c == '/' || c == '+' || c == '-' || c == '*' {
                    space = true;
                }
            }
            other => {
                let mut text = String::new();
                let _ = cssparser::ToCss::to_css(&other, &mut text);
                push(out, &mut space, &text);
            }
        }
    }
}

/// The value Chrome keeps for a keyframe property, or None when the declaration is invalid
/// (Chrome drops invalid members from getKeyframes()).
fn keyframe_value(property: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if property.starts_with("--") {
        return Some(value.to_owned());
    }
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "inherit" | "initial" | "unset" | "revert" | "revert-layer"
    ) {
        return Some(lower);
    }
    let id = PropertyId::from(lightningcss::values::string::CowArcStr::from(
        property.to_owned(),
    ));
    let native = !matches!(id, PropertyId::Custom(_));
    if native {
        let mut input = ParserInput::new(value);
        let mut parser = Parser::new(&mut input);
        let parsed = parser.parse_entirely(|input| {
            Property::parse(id, input, &ParserOptions::default())
                .map_err(|_| input.new_custom_error::<(), ()>(()))
        });
        match parsed {
            Ok(Property::Unparsed(unparsed)) => {
                let deferred = unparsed.value.0.iter().any(|token| {
                    matches!(
                        token,
                        lightningcss::properties::custom::TokenOrValue::Var(_)
                            | lightningcss::properties::custom::TokenOrValue::Env(_)
                    )
                });
                if !deferred {
                    return None;
                }
            }
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    let zero_length = length_property(property);
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let mut out = String::new();
    serialize_tokens(&mut parser, &mut out, zero_length);
    Some(out)
}

/// Length-valued properties whose bare zero serializes as `0px` (the computed-style
/// snapshot's length set).
fn length_property(property: &str) -> bool {
    [
        "width",
        "height",
        "margin",
        "padding",
        "gap",
        "radius",
        "letter-spacing",
        "word-spacing",
        "text-indent",
    ]
    .iter()
    .any(|part| property.contains(part))
        || property.ends_with("size")
        || property.starts_with("inset")
        || matches!(
            property,
            "top"
                | "right"
                | "bottom"
                | "left"
                | "flex-basis"
                | "outline-offset"
                | "column-width"
                | "column-gap"
        )
}

/// Pseudo-element selectors Chrome accepts as a KeyframeEffect target pseudo.
fn pseudo_element(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let normalized = match lower.as_str() {
        ":before" | ":after" | ":first-line" | ":first-letter" => format!(":{lower}"),
        _ => lower,
    };
    if !normalized.starts_with("::")
        || normalized.starts_with("::part(")
        || normalized.starts_with("::slotted(")
        || normalized.starts_with("::cue")
    {
        return None;
    }
    let admitted = super::css_selector_syntax::selector_list(&format!("div{normalized}"))?;
    admitted.text.starts_with("div::").then_some(normalized)
}

fn native(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let set_string =
        |scope: &mut v8::PinScope, rv: &mut v8::ReturnValue, value: Option<String>| match value
            .and_then(|value| v8::String::new(scope, &value))
        {
            Some(value) => rv.set(value.into()),
            None => rv.set(v8::null(scope).into()),
        };
    match arg_str(scope, &args, 0).as_str() {
        "dispatch" => {
            let (Ok(target), Ok(event)) = (
                v8::Local::<v8::Object>::try_from(args.get(1)),
                v8::Local::<v8::Object>::try_from(args.get(2)),
            ) else {
                return;
            };
            dispatch_host_event(scope, event, target);
        }
        "property" => {
            let name = arg_str(scope, &args, 1);
            set_string(scope, &mut rv, keyframe_property(&name));
        }
        "value" => {
            let property = arg_str(scope, &args, 1);
            let value = arg_str(scope, &args, 2);
            set_string(scope, &mut rv, keyframe_value(&property, &value));
        }
        "named" => {
            let name = arg_str(scope, &args, 1).to_ascii_lowercase();
            let rgb = (name.bytes().all(|byte| byte.is_ascii_lowercase()))
                .then(|| turbo_dom_parser::rtdom::color::canonicalize_color(&name, true))
                .flatten();
            set_string(scope, &mut rv, rgb);
        }
        "pseudo" => {
            let value = arg_str(scope, &args, 1);
            set_string(scope, &mut rv, pseudo_element(&value));
        }
        "active" => {
            let Ok(document) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
                rv.set_bool(false);
                return;
            };
            let _dom = DomScope::for_object(scope, document);
            rv.set_bool(DOM.with(|dom| {
                dom.borrow()
                    .as_ref()
                    .is_some_and(|dom| dom.browsing_context)
            }));
        }
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__animationsNative", native);
    run_js(scope, include_str!("animations.js"));
}

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
    // Lightning CSS does not model the motion-path `offset` shorthand Chrome animates, nor
    // the LENGTH_ONLY properties.
    ((super::cssom_parser::is_supported_property(&css)
        || css == "offset"
        || LENGTH_ONLY.iter().any(|row| row.0 == css))
        && !NOT_ANIMATABLE.contains(&css.as_str()))
    .then_some(css)
}

/// Chrome's specified-value number text: printf `%.6g` (six significant digits, exponent
/// form below 1e-4 and from 1e6 with a two-digit exponent), and `0` for either zero.
fn number(value: f64) -> String {
    let trim = |text: &str| {
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            text.to_owned()
        }
    };
    if value == 0.0 || !value.is_finite() {
        return "0".into();
    }
    let scientific = format!("{value:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    if (-4..6).contains(&exponent) {
        trim(&format!("{value:.*}", (5 - exponent) as usize))
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mantissa), exponent.abs())
    }
}

/// The numeric value of a number-bearing token from its source text (the token's f32 loses
/// precision Chrome keeps); `suffix` is the unit or `%` the text ends with.
fn token_number(text: &str, suffix: &str, fallback: f32) -> f64 {
    text.len()
        .checked_sub(suffix.len())
        .and_then(|end| text.get(..end))
        .filter(|_| text[text.len() - suffix.len()..].eq_ignore_ascii_case(suffix))
        .and_then(|number| number.parse::<f64>().ok())
        .unwrap_or_else(|| f64::from(fallback))
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
    loop {
        let start = input.position();
        let Ok(token) = input.next_including_whitespace_and_comments() else {
            break;
        };
        let token = token.clone();
        let text = input.slice_from(start);
        match token {
            Token::Comment(_) => {}
            Token::WhiteSpace(_) => space = true,
            Token::Comma => {
                out.push_str(", ");
                space = false;
            }
            Token::Number { value, .. } => {
                let text = number(token_number(text, "", value));
                let text = if zero_length && text == "0" {
                    "0px".into()
                } else {
                    text
                };
                push(out, &mut space, &text)
            }
            Token::Percentage { unit_value, .. } => push(
                out,
                &mut space,
                &format!("{}%", number(token_number(text, "%", unit_value * 100.0))),
            ),
            Token::Dimension { value, unit, .. } => push(
                out,
                &mut space,
                &format!(
                    "{}{}",
                    number(token_number(text, &unit, value)),
                    unit.to_ascii_lowercase()
                ),
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

/// Whether a declaration value has a `var()` / `env()` reference at any depth (its
/// validity is deferred to computed-value time), or None when it holds a token no
/// declaration value can (an unmatched closing bracket, a bad string / URL, `;`).
fn deferred(input: &mut Parser<'_, '_>) -> Option<bool> {
    let mut found = false;
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        match token.clone() {
            Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket
            | Token::BadString(_)
            | Token::BadUrl(_)
            | Token::Semicolon => return None,
            token @ (Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock) => {
                if let Token::Function(name) = &token {
                    found |= name.eq_ignore_ascii_case("var") || name.eq_ignore_ascii_case("env");
                }
                let nested = input
                    .parse_nested_block(|input| {
                        Ok::<_, cssparser::ParseError<'_, ()>>(deferred(input))
                    })
                    .ok()
                    .flatten()?;
                found |= nested;
            }
            _ => {}
        }
    }
    Some(found)
}

/// A declaration value's specified text: the source between its first and last significant
/// tokens (whitespace and comments trimmed, a trailing `!important` dropped), or None when
/// the value cannot be a declaration value at all.
fn specified_text(value: &str) -> Option<String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    deferred(&mut parser)?;
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    // (start, end, is `!`, is the `important` keyword) of each top-level significant token.
    let mut spans = Vec::new();
    loop {
        let start = parser.position();
        let Ok(token) = parser.next_including_whitespace_and_comments() else {
            break;
        };
        let (bang, important, block) = match token {
            Token::WhiteSpace(_) | Token::Comment(_) => continue,
            Token::Delim('!') => (true, false, false),
            Token::Ident(name) => (false, name.eq_ignore_ascii_case("important"), false),
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => (false, false, true),
            _ => (false, false, false),
        };
        if block {
            // Consume the block: its end is the position after its closing bracket (or the
            // end of the input when it is unclosed).
            let _ = parser.parse_nested_block(|input| {
                while input.next_including_whitespace_and_comments().is_ok() {}
                Ok::<(), cssparser::ParseError<'_, ()>>(())
            });
        }
        spans.push((
            start.byte_index(),
            parser.position().byte_index(),
            bang,
            important,
        ));
    }
    if let [.., (_, _, true, false), (_, _, false, true)] = spans.as_slice() {
        spans.truncate(spans.len() - 2);
    }
    Some(match (spans.first(), spans.last()) {
        (Some(first), Some(last)) => value[first.0..last.1].to_owned(),
        _ => String::new(),
    })
}

/// The value Chrome keeps for a keyframe property, or None when the declaration is invalid
/// (Chrome drops invalid members from getKeyframes()).
fn keyframe_value(property: &str, value: &str) -> Option<String> {
    if property.starts_with("--") {
        return Some(specified_text(value).unwrap_or_else(|| value.trim().to_owned()));
    }
    let value = value.trim();
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
    // A var() / env() reference keeps the specified text (Chrome 156).
    let mut input = ParserInput::new(value);
    if deferred(&mut Parser::new(&mut input))? {
        return specified_text(value);
    }
    if let Some(&row) = LENGTH_ONLY.iter().find(|row| row.0 == property) {
        if !length_only(value, row) {
            return None;
        }
    }
    if property == "line-height" && negative(value) {
        return None;
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

/// One LENGTH_ONLY row: (name, keyword values, non-negative, percentages allowed).
type LengthOnly = (&'static str, &'static [&'static str], bool, bool);

/// Animatable length properties Lightning CSS does not model. Chrome 156 keeps them as
/// keyframe members.
const LENGTH_ONLY: &[LengthOnly] = &[
    ("column-width", &["auto"], true, false),
    (
        "column-rule-width",
        &["thin", "medium", "thick"],
        true,
        false,
    ),
    ("outline-offset", &[], false, false),
    ("offset-distance", &[], false, true),
    ("shape-margin", &[], true, true),
    ("text-underline-offset", &["auto"], false, true),
    ("x", &[], false, true),
    ("r", &[], true, true),
];

/// Whether `value` is one of the property's keywords, a zero, a length or percentage
/// (non-negative when required) or a math function.
fn length_only(value: &str, (_, keywords, non_negative, percentage): LengthOnly) -> bool {
    const UNITS: &[&str] = &[
        "px", "em", "rem", "ex", "rex", "ch", "rch", "ic", "ric", "cap", "rcap", "lh", "rlh", "vw",
        "vh", "vi", "vb", "vmin", "vmax", "svw", "svh", "svi", "svb", "svmin", "svmax", "lvw",
        "lvh", "lvi", "lvb", "lvmin", "lvmax", "dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax", "cqw",
        "cqh", "cqi", "cqb", "cqmin", "cqmax", "cm", "mm", "q", "in", "pt", "pc",
    ];
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(|input| {
            let valid = match input.next()?.clone() {
                Token::Ident(name) => keywords.iter().any(|k| name.eq_ignore_ascii_case(k)),
                Token::Number { value, .. } => value == 0.0,
                Token::Dimension { value, unit, .. } => {
                    UNITS.iter().any(|u| unit.eq_ignore_ascii_case(u))
                        && !(non_negative && value < 0.0)
                }
                Token::Percentage { unit_value, .. } => {
                    percentage && !(non_negative && unit_value < 0.0)
                }
                Token::Function(name) => {
                    ["calc", "min", "max", "clamp"]
                        .iter()
                        .any(|f| name.eq_ignore_ascii_case(f))
                        && input
                            .parse_nested_block(|input| {
                                while input.next().is_ok() {}
                                Ok::<(), cssparser::ParseError<'_, ()>>(())
                            })
                            .is_ok()
                }
                _ => false,
            };
            if valid {
                Ok(())
            } else {
                Err(input.new_custom_error::<(), ()>(()))
            }
        })
        .is_ok()
}

/// Whether `value` is one negative number, percentage or dimension (Lightning CSS admits a
/// negative line-height, Chrome 156 does not).
fn negative(value: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(|input| match input.next()? {
            Token::Number { value, .. } | Token::Dimension { value, .. } => Ok(*value < 0.0),
            Token::Percentage { unit_value, .. } => Ok(*unit_value < 0.0),
            _ => Err(input.new_custom_error::<(), ()>(())),
        })
        .unwrap_or(false)
}

/// Length-valued properties whose bare zero serializes as `0px` (the computed-style
/// snapshot's length set).
fn length_property(property: &str) -> bool {
    // Number-valued despite their names (Chrome 156 keeps a bare `0`).
    if matches!(property, "line-height" | "stroke-width" | "tab-size") {
        return false;
    }
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
        || property.starts_with("background-position-")
        || matches!(
            property,
            "top"
                | "right"
                | "bottom"
                | "left"
                | "flex-basis"
                | "outline"
                | "outline-offset"
                | "offset-distance"
                | "shape-margin"
                | "text-underline-offset"
                | "x"
                | "r"
                | "column-width"
                | "column-gap"
                | "perspective"
                | "border-spacing"
                | "vertical-align"
                | "text-decoration-thickness"
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
            // animations.js dispatches its Animation objects' events.
            let target = v8::Local::<v8::Object>::try_from(args.get(1)).expect("animation");
            let event = v8::Local::<v8::Object>::try_from(args.get(2)).expect("event");
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
        // "active", the only other operation animations.js calls, with a timeline's
        // document (the realm document object).
        _ => {
            let document = v8::Local::<v8::Object>::try_from(args.get(1)).expect("document");
            let _dom = DomScope::for_object(scope, document);
            rv.set_bool(DOM.with(|dom| {
                dom.borrow()
                    .as_ref()
                    .is_some_and(|dom| dom.browsing_context)
            }));
        }
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__animationsNative", native);
    run_js(scope, include_str!("animations.js"));
}

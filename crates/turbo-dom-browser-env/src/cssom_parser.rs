//! CSS Syntax parsing with selector/property validation, preserving authored values.
use cssparser::{
    parse_important, AtRuleParser, CowRcStr, DeclarationParser, Delimiter, ParseError, Parser,
    ParserInput, ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser,
    StyleSheetParser,
};
use lightningcss::media_query::MediaList;
use lightningcss::properties::{
    custom::{Token, TokenList, TokenOrValue},
    Property, PropertyId,
};
use lightningcss::rules::{font_face::FontFaceProperty, CssRule};
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::targets::{Features, Targets};
use lightningcss::traits::ToCss;

pub fn is_supported_property(name: &str) -> bool {
    !matches!(PropertyId::from(name), PropertyId::Custom(_))
        || interest_delay_arity(name).is_some()
        || name == "writing-mode"
        || matches!(
            name,
            "content"
                | "zoom"
                | "font-variant"
                | "float"
                | "clear"
                | "direction"
                | "unicode-bidi"
                | "table-layout"
                | "caption-side"
                | "empty-cells"
                | "cursor"
                | "user-select"
                | "-webkit-user-select"
                | "vertical-align"
                | "z-index"
                | "visibility"
                | "pointer-events"
                | "touch-action"
                | "will-change"
                | "appearance"
                | "-webkit-appearance"
        )
        || SCROLL_KEYWORD_PROPERTIES.contains(&name)
}

// Lightning CSS does not model these CSSOM View / CSS Scroll Snap keyword
// properties; admit exactly their grammar with Chrome156's serialization.
const SCROLL_KEYWORD_PROPERTIES: [&str; 4] = [
    "scroll-behavior",
    "scroll-snap-align",
    "scroll-snap-stop",
    "scroll-snap-type",
];
fn scroll_keyword_value(name: &str, value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("var(") || lower.contains("env(") {
        return Some(value.trim().to_string());
    }
    let words: Vec<&str> = lower.split_ascii_whitespace().collect();
    let wide = ["initial", "inherit", "unset", "revert", "revert-layer"];
    if let [word] = words.as_slice() {
        if wide.contains(word) {
            return Some((*word).to_string());
        }
    }
    let one_of = |word: &&str, options: &[&str]| options.contains(word);
    match (name, words.as_slice()) {
        ("scroll-behavior", [word]) if one_of(word, &["auto", "smooth"]) => Some((*word).into()),
        ("scroll-snap-stop", [word]) if one_of(word, &["normal", "always"]) => Some((*word).into()),
        ("scroll-snap-type", ["none"]) => Some("none".into()),
        ("scroll-snap-type", [axis, rest @ ..])
            if one_of(axis, &["x", "y", "block", "inline", "both"]) =>
        {
            match rest {
                [] | ["proximity"] => Some((*axis).into()),
                ["mandatory"] => Some(format!("{axis} mandatory")),
                _ => None,
            }
        }
        ("scroll-snap-align", [first, rest @ ..]) if rest.len() <= 1 => {
            let align = ["none", "start", "end", "center"];
            if !one_of(first, &align) || rest.iter().any(|word| !one_of(word, &align)) {
                return None;
            }
            match rest {
                [second] if second != first => Some(format!("{first} {second}")),
                _ => Some((*first).into()),
            }
        }
        _ => None,
    }
}
const WRITING_MODES: [&str; 16] = [
    "horizontal-tb",
    "vertical-rl",
    "vertical-lr",
    "sideways-rl",
    "sideways-lr",
    "lr",
    "lr-tb",
    "rl",
    "rl-tb",
    "tb",
    "tb-rl",
    "initial",
    "inherit",
    "unset",
    "revert",
    "revert-layer",
];
fn interest_delay_arity(name: &str) -> Option<usize> {
    if name.eq_ignore_ascii_case("interest-delay") {
        Some(2)
    } else if name.eq_ignore_ascii_case("interest-delay-start")
        || name.eq_ignore_ascii_case("interest-delay-end")
    {
        Some(1)
    } else {
        None
    }
}
fn time<'i>(input: &mut Parser<'i, '_>) -> Result<(), ParseError<'i, ()>> {
    match input.next()? {
        cssparser::Token::Dimension { unit, value, .. }
            if *value >= 0.0
                && (unit.eq_ignore_ascii_case("s") || unit.eq_ignore_ascii_case("ms")) =>
        {
            Ok(())
        }
        _ => Err(input.new_custom_error(())),
    }
}

fn deferred_value(tokens: &TokenList<'_>) -> bool {
    tokens.0.iter().any(|token| match token {
        TokenOrValue::Var(_) | TokenOrValue::Env(_) => true,
        TokenOrValue::Function(function) => deferred_value(&function.arguments),
        TokenOrValue::Token(Token::Ident(name)) => {
            ["inherit", "initial", "unset", "revert", "revert-layer"]
                .iter()
                .any(|keyword| name.eq_ignore_ascii_case(keyword))
        }
        _ => false,
    })
}

pub(super) fn valid_selector(source: &str) -> bool {
    super::css_selector_syntax::selector_list(source).is_some()
}

fn valid_rule(source: &str) -> bool {
    StyleSheet::parse(source, ParserOptions::default()).is_ok_and(|sheet| {
        !sheet.rules.0.is_empty()
            && !sheet
                .rules
                .0
                .iter()
                .any(|rule| matches!(rule, CssRule::Unknown(_)))
    })
}

// cssparser supplies tokenization, block boundaries and error recovery. Lightning
// CSS supplies selector/property grammar; its printer would rewrite blue to #00f,
// so CSSOM keeps validated declaration lexemes rather than minifier serialization.
#[derive(Default)]
struct CssParser {
    keyframes: bool,
    font_face: bool,
    raw_descriptors: bool,
}

fn consume(input: &mut Parser<'_, '_>) -> String {
    let start = input.position();
    while input.next_including_whitespace_and_comments().is_ok() {}
    input.slice_from(start).trim().to_string()
}

impl<'i> QualifiedRuleParser<'i> for CssParser {
    type Prelude = String;
    type QualifiedRule = String;
    type Error = ();
    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<String, ParseError<'i, ()>> {
        let prelude = consume(input);
        if !self.keyframes {
            // Selector admission follows Chrome's pseudo inventory and returns
            // its serialization (forgiving drops, lowercase pseudo names).
            return super::css_selector_syntax::selector_list(&prelude)
                .map(|admitted| admitted.text)
                .ok_or_else(|| input.new_custom_error(()));
        }
        if !valid_rule(&format!("@keyframes test {{ {prelude} {{}} }}")) {
            return Err(input.new_custom_error(()));
        }
        Ok(prelude)
    }
    fn parse_block<'t>(
        &mut self,
        prelude: String,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<String, ParseError<'i, ()>> {
        let body = RuleBodyParser::new(input, self)
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .join(" ");
        Ok(format!("{prelude} {{ {body} }}"))
    }
}

// Font descriptors have their own grammar: ordinary property validation drops
// `src` and wrongly admits layout properties inside @font-face.
fn font_descriptor(name: &str, value: &str) -> Option<String> {
    let source = format!("@font-face {{ {name}: {value}; }}");
    let sheet = StyleSheet::parse(&source, ParserOptions::default()).ok()?;
    let CssRule::FontFace(rule) = sheet.rules.0.first()? else {
        return None;
    };
    let descriptor = rule.properties.first()?;
    if !matches!(descriptor, FontFaceProperty::Custom(_)) {
        return if matches!(descriptor, FontFaceProperty::Source(_)) {
            descriptor
                .to_css_string(PrinterOptions::default())
                .ok()
                .map(|text| format!("{text};"))
        } else {
            Some(format!("{name}: {value};"))
        };
    }
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    let valid = match name {
        "font-display" => input.expect_ident().is_ok_and(|value| {
            matches!(
                value.as_ref(),
                "auto" | "block" | "swap" | "fallback" | "optional"
            )
        }),
        "size-adjust" | "ascent-override" | "descent-override" | "line-gap-override" => {
            (name != "size-adjust"
                && input
                    .try_parse(|input| input.expect_ident_matching("normal"))
                    .is_ok())
                || input.expect_percentage().is_ok_and(|value| value >= 0.0)
        }
        "font-feature-settings" | "font-variation-settings" => Property::parse(
            PropertyId::from(name),
            &mut input,
            &ParserOptions::default(),
        )
        .is_ok_and(|property| !matches!(property, Property::Unparsed(_) | Property::Custom(_))),
        _ => false,
    };
    (valid && input.expect_exhausted().is_ok()).then(|| format!("{name}: {value};"))
}

impl<'i> DeclarationParser<'i> for CssParser {
    type Declaration = String;
    type Error = ();
    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _: &ParserState,
    ) -> Result<String, ParseError<'i, ()>> {
        let start = input.position();
        if self.raw_descriptors {
            return Ok(format!(
                "{}: {};",
                name.to_ascii_lowercase(),
                consume(input)
            ));
        }
        if self.font_face {
            let value = consume(input);
            return font_descriptor(&name.to_ascii_lowercase(), &value)
                .ok_or_else(|| input.new_custom_error(()));
        }
        if name.eq_ignore_ascii_case("zoom") {
            // Lightning CSS has no typed `zoom`; Chrome serializes its own grammar.
            let value = input.parse_until_before(Delimiter::Bang, zoom_declaration)?;
            let important = input.try_parse(parse_important).is_ok();
            input.expect_exhausted()?;
            return Ok(format!(
                "zoom: {value}{};",
                if important { " !important" } else { "" }
            ));
        }
        let property = name.to_ascii_lowercase();
        if SCROLL_KEYWORD_PROPERTIES.contains(&property.as_str()) {
            let text = input.parse_until_before(Delimiter::Bang, |input| {
                Ok::<_, ParseError<'i, ()>>(consume(input))
            })?;
            let value =
                scroll_keyword_value(&property, &text).ok_or_else(|| input.new_custom_error(()))?;
            let important = input.try_parse(parse_important).is_ok();
            input.expect_exhausted()?;
            return Ok(format!(
                "{property}: {value}{};",
                if important { " !important" } else { "" }
            ));
        }
        let valid = input.parse_until_before(Delimiter::Bang, |input| {
            // Lightning CSS does not model the CSS2 font-variant shorthand.
            // Admit its measured caps subset without accepting arbitrary custom tokens.
            if name.eq_ignore_ascii_case("font-variant") {
                let value = input.expect_ident_cloned()?.to_ascii_lowercase();
                input.expect_exhausted()?;
                return Ok(matches!(value.as_str(), "normal" | "small-caps" | "initial" | "inherit" | "unset" | "revert" | "revert-layer"));
            }
            // Lightning CSS does not model `writing-mode`; admit Chrome's keywords.
            if name.eq_ignore_ascii_case("writing-mode") {
                let value = input.expect_ident_cloned()?.to_ascii_lowercase();
                input.expect_exhausted()?;
                return Ok(WRITING_MODES.contains(&value.as_str()));
            }
            // Lightning CSS does not model interest-invoker delays: `normal | <time>`, and the
            // shorthand takes one or two of those.
            if let Some(count) = interest_delay_arity(&name) {
                let mut values = 0;
                while !input.is_exhausted() {
                    let valid = input
                        .try_parse(|input| input.expect_ident_matching("normal"))
                        .is_ok()
                        || input.try_parse(time).is_ok();
                    if !valid {
                        return Ok(false);
                    }
                    values += 1;
                }
                return Ok(values >= 1 && values <= count);
            }
            let property=Property::parse(
                PropertyId::from(lightningcss::values::string::CowArcStr::from(name.to_string())),
                input,
                &ParserOptions::default(),
            )
            .map_err(|_| input.new_custom_error(()))?;
            Ok(!matches!(&property, Property::Unparsed(property) if !deferred_value(&property.value)))
        })?;
        if !valid {
            return Err(input.new_custom_error(()));
        }
        let value = input.slice_from(start).trim().to_string();
        let important = input.try_parse(parse_important).is_ok();
        input.expect_exhausted()?;
        let name = if name.starts_with("--") {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        };
        Ok(format!(
            "{name}: {value}{};",
            if important { " !important" } else { "" }
        ))
    }
}

/// Chrome156 standardized `zoom`: `normal`, a non-negative `<number>` or
/// `<percentage>`, or one calc() of either kind. Returns Chrome's specified-value
/// serialization and the factor it computes to (zero still means "no zoom").
pub fn zoom_value(source: &str) -> Option<(String, f32)> {
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    let value = zoom_component(&mut input).ok()?;
    input.expect_exhausted().ok()?;
    Some(value)
}

fn zoom_declaration<'i>(input: &mut Parser<'i, '_>) -> Result<String, ParseError<'i, ()>> {
    let start = input.state();
    let begin = input.position();
    while input.next().is_ok() {}
    if let Some((text, _)) = zoom_value(input.slice_from(begin)) {
        return Ok(text);
    }
    input.reset(&start);
    let start = input.position();
    let property = Property::parse(PropertyId::from("zoom"), input, &ParserOptions::default())
        .map_err(|_| input.new_custom_error(()))?;
    let deferred = match &property {
        Property::Custom(property) => deferred_value(&property.value),
        Property::Unparsed(property) => deferred_value(&property.value),
        _ => false,
    };
    if !deferred {
        return Err(input.new_custom_error(()));
    }
    let text = input.slice_from(start).trim();
    let keyword = ["inherit", "initial", "unset", "revert", "revert-layer"]
        .into_iter()
        .find(|keyword| text.eq_ignore_ascii_case(keyword));
    Ok(keyword.map_or_else(|| text.to_string(), str::to_string))
}

#[derive(Clone, Copy, PartialEq)]
enum ZoomKind {
    Number,
    Percentage,
}

type ZoomResult<'i> = Result<(String, f32), ParseError<'i, ()>>;

fn zoom_component<'i>(input: &mut Parser<'i, '_>) -> ZoomResult<'i> {
    input.skip_whitespace();
    let location = input.current_source_location();
    let start = input.position();
    match input.next()?.clone() {
        cssparser::Token::Ident(name) if name.eq_ignore_ascii_case("normal") => {
            Ok(("normal".into(), 1.0))
        }
        cssparser::Token::Number { value, .. } if value >= 0.0 => Ok((css_number(value), value)),
        cssparser::Token::Percentage { unit_value, .. } if unit_value >= 0.0 => {
            let percent = percentage(input.slice_from(start), unit_value);
            Ok((format!("{}%", css_number(percent)), ratio(percent)))
        }
        cssparser::Token::Function(name) if name.eq_ignore_ascii_case("calc") => input
            .parse_nested_block(|input| {
                let (value, kind) = calc_sum(input)?;
                input.expect_exhausted()?;
                let suffix = if kind == ZoomKind::Percentage {
                    "%"
                } else {
                    ""
                };
                let factor = if kind == ZoomKind::Percentage {
                    ratio(value)
                } else {
                    value
                };
                Ok((
                    format!("calc({}{suffix})", css_number(value)),
                    factor.max(0.0),
                ))
            }),
        _ => Err(location.new_custom_error(())),
    }
}

// Blink keeps the authored percentage as a float and divides it for the factor;
// the tokenizer's pre-divided unit value would round differently.
fn percentage(token: &str, unit_value: f32) -> f32 {
    token
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or_else(|_| (f64::from(unit_value) * 100.0) as f32)
}
fn ratio(percent: f32) -> f32 {
    (f64::from(percent) / 100.0) as f32
}

fn calc_sum<'i>(input: &mut Parser<'i, '_>) -> Result<(f32, ZoomKind), ParseError<'i, ()>> {
    let (mut total, kind) = calc_product(input)?;
    while let Ok(sign) = input.try_parse(|input| match input.next()?.clone() {
        cssparser::Token::Delim(sign @ ('+' | '-')) => Ok(sign),
        _ => Err(input.new_custom_error::<_, ()>(())),
    }) {
        let (value, other) = calc_product(input)?;
        if other != kind {
            return Err(input.new_custom_error(()));
        }
        total = if sign == '+' {
            total + value
        } else {
            total - value
        };
    }
    Ok((total, kind))
}

fn calc_product<'i>(input: &mut Parser<'i, '_>) -> Result<(f32, ZoomKind), ParseError<'i, ()>> {
    let (mut total, mut kind) = calc_value(input)?;
    while let Ok(operator) = input.try_parse(|input| match input.next()?.clone() {
        cssparser::Token::Delim(operator @ ('*' | '/')) => Ok(operator),
        _ => Err(input.new_custom_error::<_, ()>(())),
    }) {
        let (value, other) = calc_value(input)?;
        match (operator, kind, other) {
            ('*', _, ZoomKind::Number) => total *= value,
            ('*', ZoomKind::Number, ZoomKind::Percentage) => {
                total *= value;
                kind = ZoomKind::Percentage;
            }
            ('/', _, ZoomKind::Number) if value != 0.0 => total /= value,
            _ => return Err(input.new_custom_error(())),
        }
    }
    Ok((total, kind))
}

fn calc_value<'i>(input: &mut Parser<'i, '_>) -> Result<(f32, ZoomKind), ParseError<'i, ()>> {
    input.skip_whitespace();
    let location = input.current_source_location();
    let start = input.position();
    match input.next()?.clone() {
        cssparser::Token::Number { value, .. } => Ok((value, ZoomKind::Number)),
        cssparser::Token::Percentage { unit_value, .. } => Ok((
            percentage(input.slice_from(start), unit_value),
            ZoomKind::Percentage,
        )),
        cssparser::Token::ParenthesisBlock => input.parse_nested_block(calc_sum),
        cssparser::Token::Function(name) if name.eq_ignore_ascii_case("calc") => {
            input.parse_nested_block(calc_sum)
        }
        _ => Err(location.new_custom_error(())),
    }
}

/// Blink's CSS number serialization: six significant digits (printf `%g`).
pub fn css_number(value: f32) -> String {
    let value = f64::from(value);
    if value == 0.0 || !value.is_finite() {
        return "0".into();
    }
    let exponent = value.abs().log10().floor() as i32;
    if !(-5..6).contains(&exponent) {
        let text = format!("{value:.5e}");
        let (mantissa, power) = text.split_once('e').unwrap_or((&text, "0"));
        let mantissa = trim_fraction(mantissa);
        let power: i32 = power.parse().unwrap_or(0);
        let sign = if power < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", power.abs());
    }
    let decimals = usize::try_from(5 - exponent).unwrap_or(0);
    trim_fraction(&format!("{value:.decimals$}")).to_string()
}

fn trim_fraction(text: &str) -> &str {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    }
}

// These rules require descriptors to validate the rule itself. Testing an empty
// shell loses valid @property rules, while ordinary CSS properties lose navigation.
fn typed_descriptor_rule(name: &str, prelude: &str, body: &str) -> Option<String> {
    use lightningcss::rules::view_transition::ViewTransitionProperty;
    let source = format!("@{name} {prelude} {{ {body} }}");
    let sheet = StyleSheet::parse(&source, ParserOptions::default()).ok()?;
    match sheet.rules.0.first()? {
        CssRule::Property(rule) => {
            let mut source = ParserInput::new(body);
            let mut input = Parser::new(&mut source);
            let mut parser = CssParser {
                raw_descriptors: true,
                ..Default::default()
            };
            let descriptors: Vec<_> = RuleBodyParser::new(&mut input, &mut parser)
                .filter_map(Result::ok)
                .collect();
            let initial = descriptors
                .iter()
                .rev()
                .find(|value| value.starts_with("initial-value:"));
            let syntax = rule.syntax.to_css_string(PrinterOptions::default()).ok()?;
            Some(format!(
                "@property {prelude} {{ syntax: {syntax}; inherits: {};{} }}",
                rule.inherits,
                initial.map_or(String::new(), |value| format!(" {value}"))
            ))
        }
        CssRule::ViewTransition(rule) => {
            let descriptors: Vec<_> = rule
                .properties
                .iter()
                .filter(|value| !matches!(value, ViewTransitionProperty::Custom(_)))
                .filter_map(|value| {
                    value
                        .to_css_string(PrinterOptions::default())
                        .ok()
                        .map(|value| format!("{value};"))
                })
                .collect();
            Some(format!("@view-transition {{ {} }}", descriptors.join(" ")))
        }
        _ => None,
    }
}

impl<'i> AtRuleParser<'i> for CssParser {
    type Prelude = (String, String);
    type AtRule = String;
    type Error = ();
    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, ()>> {
        Ok((name.to_ascii_lowercase(), consume(input)))
    }
    fn rule_without_block(
        &mut self,
        (name, prelude): Self::Prelude,
        _: &ParserState,
    ) -> Result<String, ()> {
        let rule = format!("@{name} {prelude};");
        if valid_rule(&rule) {
            Ok(rule)
        } else {
            Err(())
        }
    }
    fn parse_block<'t>(
        &mut self,
        (name, prelude): Self::Prelude,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<String, ParseError<'i, ()>> {
        if matches!(name.as_str(), "property" | "view-transition") {
            let body = consume(input);
            return typed_descriptor_rule(&name, &prelude, &body)
                .ok_or_else(|| input.new_custom_error(()));
        }
        let test = format!("@{name} {prelude} {{}}");
        if !valid_rule(&test) {
            return Err(input.new_custom_error(()));
        }
        let body = if matches!(
            name.as_str(),
            "font-face"
                | "page"
                | "property"
                | "counter-style"
                | "font-palette-values"
                | "viewport"
                | "position-try"
                | "view-transition"
        ) {
            let mut declarations = CssParser {
                font_face: name == "font-face",
                ..Default::default()
            };
            RuleBodyParser::new(input, &mut declarations)
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            let mut nested = CssParser {
                keyframes: name == "keyframes" || name.ends_with("-keyframes"),
                ..Default::default()
            };
            StyleSheetParser::new(input, &mut nested)
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
                .join(" ")
        };
        Ok(format!("@{name} {prelude} {{ {body} }}"))
    }
}

impl<'i> RuleBodyItemParser<'i, String, ()> for CssParser {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        !self.keyframes
    }
}

pub fn parse(source: &str) -> String {
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    StyleSheetParser::new(&mut input, &mut CssParser::default())
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serialize a query using the media grammar rather than stylesheet prelude text.
pub fn parse_media(source: &str) -> String {
    // The typed AST normalizes min/max features into ranges. Preserve the common
    // authored legacy form instead of introducing a new range syntax in CSSOM.
    let legacy =
        (source.contains("min-") || source.contains("max-")) && !source.contains(['<', '>']);
    let options = PrinterOptions {
        targets: Targets {
            include: if legacy {
                Features::MediaRangeSyntax
            } else {
                Features::empty()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    MediaList::parse(&mut input, &ParserOptions::default())
        .ok()
        .filter(|_| input.expect_exhausted().is_ok())
        .and_then(|media| media.to_css_string(options).ok())
        .unwrap_or_else(|| "not all".to_string())
}

/// Parse a single rule without dropping invalid neighboring rules. Declaration
/// recovery remains CSSOM-compatible inside a syntactically valid rule.
pub fn parse_one(source: &str) -> Option<String> {
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    let mut parser = CssParser::default();
    let mut rules = StyleSheetParser::new(&mut input, &mut parser);
    let rule = rules.next()?.ok()?;
    if rules.next().is_some() {
        return None;
    }
    Some(rule)
}

#[cfg(test)]
mod tests {
    use super::{css_number, is_supported_property, parse, parse_media, parse_one, zoom_value};

    #[test]
    fn zoom_uses_chrome156_grammar_serialization_and_factors() {
        for (source, text, factor) in [
            ("2", "2", 2.0),
            ("200%", "200%", 2.0),
            ("NORMAL", "normal", 1.0),
            ("-0", "0", 0.0),
            ("0.0", "0", 0.0),
            (".25", "0.25", 0.25),
            ("+2", "2", 2.0),
            ("1E1", "10", 10.0),
            ("0.3333333", "0.333333", 0.333_333_3),
            ("33.33333%", "33.3333%", 0.333_333_28),
            ("calc(1 + 1)", "calc(2)", 2.0),
            ("calc(50% + 50%)", "calc(100%)", 1.0),
            ("calc(2 * 1.5)", "calc(3)", 3.0),
            ("calc((1 + 1) * 50%)", "calc(100%)", 1.0),
        ] {
            assert_eq!(
                zoom_value(source),
                Some((text.to_string(), factor)),
                "{source}"
            );
        }
        for source in [
            "reset",
            "document",
            "-1",
            "-50%",
            "abc",
            "2px",
            "2 3",
            "calc(1 + 10%)",
            "calc(1 +1)",
            "calc(10% * 10%)",
            "calc(1 / 0)",
            "",
        ] {
            assert_eq!(zoom_value(source), None, "{source}");
        }
        let parsed = parse("a { zoom: +2; } b { zoom: reset; } c { zoom: INHERIT !important; } d { zoom: var(--z); }");
        assert!(parsed.contains("a { zoom: 2; }"), "{parsed}");
        assert!(!parsed.contains("reset"), "{parsed}");
        assert!(parsed.contains("zoom: inherit !important;"), "{parsed}");
        assert!(parsed.contains("zoom: var(--z);"), "{parsed}");
        assert!(super::is_supported_property("zoom"));
        for (value, text) in [
            (1e6, "1e+06"),
            (1.5e-7, "1.5e-07"),
            (123_456.7, "123457"),
            (0.1, "0.1"),
        ] {
            assert_eq!(css_number(value), text);
        }
    }

    #[test]
    fn scroll_keyword_properties_validate_and_serialize_like_chrome156() {
        for (declaration, expected) in [
            ("scroll-behavior: SMOOTH", Some("scroll-behavior: smooth;")),
            ("scroll-behavior: instant", None),
            ("scroll-behavior: smooth auto", None),
            (
                "scroll-behavior: var(--x)",
                Some("scroll-behavior: var(--x);"),
            ),
            (
                "scroll-snap-type: Y Proximity",
                Some("scroll-snap-type: y;"),
            ),
            (
                "scroll-snap-type: x  mandatory",
                Some("scroll-snap-type: x mandatory;"),
            ),
            (
                "scroll-snap-type: both mandatory",
                Some("scroll-snap-type: both mandatory;"),
            ),
            ("scroll-snap-type: mandatory", None),
            ("scroll-snap-type: y x", None),
            ("scroll-snap-type: none mandatory", None),
            (
                "scroll-snap-type: initial",
                Some("scroll-snap-type: initial;"),
            ),
            (
                "scroll-snap-align: START end",
                Some("scroll-snap-align: start end;"),
            ),
            (
                "scroll-snap-align: center center",
                Some("scroll-snap-align: center;"),
            ),
            (
                "scroll-snap-align: none start",
                Some("scroll-snap-align: none start;"),
            ),
            ("scroll-snap-align: start end center", None),
            ("scroll-snap-align: left", None),
            (
                "scroll-snap-stop: Normal",
                Some("scroll-snap-stop: normal;"),
            ),
            ("scroll-snap-stop: never", None),
            (
                "scroll-snap-stop: always !important",
                Some("scroll-snap-stop: always !important;"),
            ),
        ] {
            let parsed = parse(&format!("a {{ {declaration} }}"));
            match expected {
                Some(expected) => assert!(parsed.contains(expected), "{declaration}: {parsed}"),
                None => assert!(!parsed.contains("scroll-"), "{declaration}: {parsed}"),
            }
        }
        for name in [
            "scroll-behavior",
            "scroll-snap-type",
            "scroll-snap-align",
            "scroll-snap-stop",
        ] {
            assert!(is_supported_property(name));
        }
    }

    #[test]
    fn recovers_bad_selectors_and_declarations_without_losing_valid_neighbors() {
        let parsed = parse("a { color: red; width: garbage; } ??? { color: blue; } sentinel {}");
        assert!(parsed.contains("color: red"), "{parsed}");
        assert!(!parsed.contains("garbage"), "{parsed}");
        assert!(!parsed.contains("???"), "{parsed}");
        assert!(parsed.contains("sentinel"), "{parsed}");
    }

    #[test]
    fn strings_comments_custom_properties_and_grouping_use_css_grammar() {
        let parsed = parse(
            r#"/* { */ @media screen { a[data-x="}"] { content: "a;b}"; --token: url("a;b"); } }"#,
        );
        assert!(parsed.contains("@media screen"), "{parsed}");
        assert!(parsed.contains("a[data-x="), "{parsed}");
        assert!(parsed.contains("a;b}"), "{parsed}");
        assert!(parsed.contains("--token"), "{parsed}");
    }
    #[test]
    fn single_rule_parsing_does_not_recover_invalid_neighbors() {
        assert!(parse_one("??? {} a {}").is_none());
        assert!(parse_one("a {} b {}").is_none());
        assert!(parse_one("a { width: nonsense; color: blue }")
            .unwrap()
            .contains("color: blue"));
    }

    #[test]
    fn full_typed_descriptors_validate_before_empty_shells() {
        let property = parse_one(
            r#"@property --paint {syntax:"<color>";initial-value:transparent;inherits:false}"#,
        )
        .unwrap();
        assert!(property.contains("initial-value: transparent;"));
        assert!(property.contains("inherits: false;"));
        assert!(parse_one(r#"@property --paint {syntax:"<color>";inherits:false}"#).is_none());
        let transition = parse_one("@view-transition {navigation:auto;width:20px}").unwrap();
        assert!(transition.contains("navigation: auto;"));
        assert!(!transition.contains("width"));
    }

    #[test]
    fn media_queries_use_their_typed_grammar() {
        assert_eq!(parse_media("SCREEN"), "screen");
        assert_eq!(
            parse_media("screen and (min-width:10px)"),
            "screen and (min-width: 10px)"
        );
        assert_eq!(parse_media("???"), "not all");
    }
}

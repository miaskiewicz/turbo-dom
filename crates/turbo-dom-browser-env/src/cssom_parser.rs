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
// The `zoom` grammar lives in base turbo-dom so native layout consumers share it.
pub use turbo_dom_parser::rtdom::css_values::{css_number, zoom_value};

pub fn is_supported_property(name: &str) -> bool {
    // Chrome 156 refuses every `-moz-*` vendor-prefixed name, even the ones Lightning CSS
    // maps to a known PropertyId (e.g. `-moz-appearance`): see turbo-dom#390.
    if name.len() > 5 && name.as_bytes()[0] == b'-' && name[1..].starts_with("moz-") {
        return false;
    }
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
        TokenOrValue::Function(function) if function.name.eq_ignore_ascii_case("if") => {
            if_function(&function.arguments)
        }
        TokenOrValue::Function(function) => deferred_value(&function.arguments),
        TokenOrValue::Token(Token::Ident(name)) => {
            ["inherit", "initial", "unset", "revert", "revert-layer"]
                .iter()
                .any(|keyword| name.eq_ignore_ascii_case(keyword))
        }
        _ => false,
    })
}

/// CSS `if()`, an arbitrary substitution function like `var()` (turbo-dom#322): branches
/// `<if-condition> : <declaration-value>?` separated by `;` (the last one's optional), where a
/// condition is `else` or a `<boolean-expr>` over `style()` / `media()` / `supports()`.
/// Chrome 156 checks this shape at parse time and substitutes at computed-value time.
fn if_function(arguments: &TokenList<'_>) -> bool {
    let (mut branches, mut condition, mut in_condition, mut depth) = (0, Vec::new(), true, 0usize);
    for token in &arguments.0 {
        match token {
            TokenOrValue::Token(Token::WhiteSpace(_)) => continue,
            TokenOrValue::Token(Token::Colon) if in_condition && depth == 0 => {
                if !if_condition(&condition) {
                    return false;
                }
                branches += 1;
                in_condition = false;
                continue;
            }
            TokenOrValue::Token(Token::Semicolon) if depth == 0 => {
                // A condition without its `:`, or an empty branch (`;;`).
                if in_condition {
                    return false;
                }
                in_condition = true;
                condition.clear();
                continue;
            }
            TokenOrValue::Token(Token::ParenthesisBlock) => depth += 1,
            TokenOrValue::Token(Token::CloseParenthesis) => depth = depth.saturating_sub(1),
            _ => {}
        }
        if in_condition {
            condition.push(token);
        }
    }
    // Only a trailing `;` may leave the last condition empty.
    branches > 0 && (!in_condition || condition.is_empty())
}

/// An `if()` condition: `else`, or a `<boolean-expr>` of `style()` / `media()` / `supports()`
/// tests: `not <test>`, or tests joined by one of `and` / `or`.
fn if_condition(tokens: &[&TokenOrValue<'_>]) -> bool {
    if let [TokenOrValue::Token(Token::Ident(name))] = tokens {
        if name.eq_ignore_ascii_case("else") {
            return true;
        }
    }
    boolean_expression(tokens)
}

fn boolean_expression(tokens: &[&TokenOrValue<'_>]) -> bool {
    let keyword = |token: Option<&&TokenOrValue<'_>>| match token {
        Some(TokenOrValue::Token(Token::Ident(name))) => Some(name.to_ascii_lowercase()),
        _ => None,
    };
    if keyword(tokens.first()).as_deref() == Some("not") {
        return if_test(&tokens[1..]) == Some(tokens.len() - 1);
    }
    let Some(mut at) = if_test(tokens) else {
        return false;
    };
    let mut operator = None;
    while at < tokens.len() {
        let joiner = keyword(tokens.get(at));
        if !matches!(joiner.as_deref(), Some("and" | "or"))
            || operator.is_some_and(|op| op != joiner)
        {
            return false;
        }
        operator = Some(joiner);
        let Some(length) = if_test(&tokens[at + 1..]) else {
            return false;
        };
        at += 1 + length;
    }
    true
}

/// The token count of the `if()` test that starts `tokens`: a `style()` / `media()` /
/// `supports()` function, or a parenthesized `<boolean-expr>`.
fn if_test(tokens: &[&TokenOrValue<'_>]) -> Option<usize> {
    match tokens.first()? {
        TokenOrValue::Function(function)
            if ["style", "media", "supports"]
                .iter()
                .any(|name| function.name.eq_ignore_ascii_case(name)) =>
        {
            Some(1)
        }
        TokenOrValue::Token(Token::ParenthesisBlock) => {
            let mut depth = 0usize;
            for (index, token) in tokens.iter().enumerate() {
                match token {
                    TokenOrValue::Token(Token::ParenthesisBlock) => depth += 1,
                    TokenOrValue::Token(Token::CloseParenthesis) => {
                        depth -= 1;
                        if depth == 0 {
                            return boolean_expression(&tokens[1..index]).then_some(index + 1);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
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
    /// Nested inside a style rule's body (turbo-dom#358): the group at-rules that admit
    /// nested rules (`@media`, `@supports`, `@container`, `@scope`, `@starting-style`,
    /// `@layer`) also admit bare declarations (CSSNestedDeclarations). Set on the parser
    /// that walks a style rule's body.
    nested: bool,
}

fn consume(input: &mut Parser<'_, '_>) -> String {
    let start = input.position();
    while input.next_including_whitespace_and_comments().is_ok() {}
    input.slice_from(start).trim().to_string()
}

/// Chrome's CSSKeyframeRule.keyText: `from` / `to` become `0%` / `100%` and each percentage
/// is a CSS number (`50.0%` -> `50%`, `1e1%` -> `10%`), joined by `", "`.
fn keyframe_keys(prelude: &str) -> Option<String> {
    let mut source = ParserInput::new(prelude);
    let mut input = Parser::new(&mut source);
    let keys = input
        .parse_comma_separated(|input| {
            let location = input.current_source_location();
            match input.next()?.clone() {
                cssparser::Token::Ident(name) if name.eq_ignore_ascii_case("from") => {
                    Ok("0%".to_owned())
                }
                cssparser::Token::Ident(name) if name.eq_ignore_ascii_case("to") => {
                    Ok("100%".to_owned())
                }
                // A keyframe selector is a percentage in 0%..100%.
                cssparser::Token::Percentage { unit_value, .. }
                    if (0.0..=1.0).contains(&unit_value) =>
                {
                    Ok(format!("{}%", css_number(unit_value * 100.0)))
                }
                _ => Err(location.new_custom_error::<(), ()>(())),
            }
        })
        .ok()?;
    Some(keys.join(", "))
}

/// A @keyframes name as Chrome serializes it: a `<string>` name becomes an identifier
/// (escaped as needed: `"a b"` -> `a\ b`); an identifier keeps its case.
fn keyframes_name(prelude: &str) -> Option<String> {
    let mut source = ParserInput::new(prelude);
    let mut input = Parser::new(&mut source);
    let name = match input.next().ok()?.clone() {
        cssparser::Token::Ident(name) => name.to_string(),
        cssparser::Token::QuotedString(name) if !name.is_empty() => {
            let mut text = String::new();
            cssparser::serialize_identifier(&name, &mut text).ok()?;
            text
        }
        _ => return None,
    };
    input.expect_exhausted().ok()?;
    Some(name)
}

/// The animation properties a keyframe block drops (Blink's IsValidForKeyframe).
fn keyframe_ignored(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("animation")
        && !matches!(
            name.as_str(),
            "animation-timing-function" | "animation-composition" | "animation-timeline"
        )
}

/// The names @counter-style may not define (CSS Counter Styles 3 §3.1, Chrome 156):
/// `none`, the CSS-wide keywords and the predefined styles that cannot be overridden.
fn reserved_counter_style(name: &str) -> bool {
    [
        "none",
        "inherit",
        "initial",
        "unset",
        "default",
        "revert",
        "revert-layer",
        "decimal",
        "disc",
        "square",
        "circle",
        "disclosure-open",
        "disclosure-closed",
    ]
    .iter()
    .any(|reserved| name.eq_ignore_ascii_case(reserved))
}

/// `@layer a , b ;` -> `@layer a, b;` (Chrome's CSSLayerStatementRule.cssText).
fn layer_names(prelude: &str) -> Option<String> {
    let mut source = ParserInput::new(prelude);
    let mut input = Parser::new(&mut source);
    let names = input
        .parse_comma_separated(|input| {
            let start = input.position();
            while input.next().is_ok() {}
            Ok::<_, ParseError<'_, ()>>(input.slice_from(start).trim().to_owned())
        })
        .ok()?;
    Some(names.join(", "))
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
        keyframe_keys(&prelude).ok_or_else(|| input.new_custom_error(()))
    }
    fn parse_block<'t>(
        &mut self,
        prelude: String,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<String, ParseError<'i, ()>> {
        // A style rule's body is a nesting context (turbo-dom#358): children at-rules admit
        // bare declarations (CSSNestedDeclarations).
        let was = self.nested;
        self.nested = !self.keyframes;
        let body = RuleBodyParser::new(input, self)
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .join(" ");
        self.nested = was;
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
        "font-feature-settings" | "font-variation-settings" => {
            return font_settings(name == "font-feature-settings", &mut input)
                .filter(|_| input.expect_exhausted().is_ok())
                .map(|value| format!("{name}: {value};"));
        }
        _ => false,
    };
    (valid && input.expect_exhausted().is_ok()).then(|| format!("{name}: {value};"))
}

/// `font-feature-settings` (`normal | [<string> [<integer> | on | off]?]#`) or
/// `font-variation-settings` (`normal | [<string> <number>?]#`) in Chrome 156's serialization:
/// four-character printable-ASCII tags, double-quoted; a feature value of 1 (`on`) is omitted
/// and `off` is 0; a variation value defaults to 1 and prints as a CSS number.
fn font_settings(feature: bool, input: &mut Parser<'_, '_>) -> Option<String> {
    if input
        .try_parse(|input| input.expect_ident_matching("normal"))
        .is_ok()
    {
        return Some("normal".to_owned());
    }
    let settings = input
        .parse_comma_separated(|input| {
            let location = input.current_source_location();
            let invalid = || location.new_custom_error::<(), ()>(());
            let tag = input.expect_string_cloned()?;
            if tag.chars().count() != 4 || !tag.chars().all(|c| (' '..='~').contains(&c)) {
                return Err(invalid());
            }
            let mut text = String::new();
            cssparser::serialize_string(&tag, &mut text).map_err(|_| invalid())?;
            if feature {
                let value = if input
                    .try_parse(|input| input.expect_ident_matching("on"))
                    .is_ok()
                {
                    1
                } else if input
                    .try_parse(|input| input.expect_ident_matching("off"))
                    .is_ok()
                {
                    0
                } else {
                    input.try_parse(Parser::expect_integer).unwrap_or(1)
                };
                if value != 1 {
                    text.push_str(&format!(" {value}"));
                }
            } else {
                let value = input.try_parse(Parser::expect_number).unwrap_or(1.0);
                text.push_str(&format!(" {}", css_number(value)));
            }
            Ok(text)
        })
        .ok()?;
    Some(settings.join(", "))
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
        if let Some(early) = parse_value_early(self.raw_descriptors, self.font_face, &name, input)?
        {
            return Ok(early);
        }
        let valid =
            input.parse_until_before(Delimiter::Bang, |input| parse_value_body(&name, input))?;
        if !valid {
            return Err(input.new_custom_error(()));
        }
        let value = input.slice_from(start).trim().to_string();
        let important = important(input)?;
        if self.keyframes && (!important.is_empty() || keyframe_ignored(&name)) {
            // Keyframe blocks ignore !important declarations and the animation properties
            // other than timing-function / composition / timeline (Chrome 156).
            return Err(input.new_custom_error(()));
        }
        let name = if name.starts_with("--") {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        };
        Ok(format!("{name}: {value}{important};"))
    }
}

fn parse_value_early<'i, 't>(
    raw_descriptors: bool,
    font_face: bool,
    name: &CowRcStr<'i>,
    input: &mut Parser<'i, 't>,
) -> Result<Option<String>, ParseError<'i, ()>> {
    if raw_descriptors {
        return Ok(Some(format!(
            "{}: {};",
            name.to_ascii_lowercase(),
            consume(input)
        )));
    }
    if font_face {
        let value = consume(input);
        return font_descriptor(&name.to_ascii_lowercase(), &value)
            .map(Some)
            .ok_or_else(|| input.new_custom_error(()));
    }
    if name.eq_ignore_ascii_case("zoom") {
        let value = input.parse_until_before(Delimiter::Bang, zoom_declaration)?;
        let important = important(input)?;
        return Ok(Some(format!("zoom: {value}{important};")));
    }
    let property = name.to_ascii_lowercase();
    if SCROLL_KEYWORD_PROPERTIES.contains(&property.as_str()) {
        let text = input.parse_until_before(Delimiter::Bang, |input| {
            Ok::<_, ParseError<'i, ()>>(consume(input))
        })?;
        let value =
            scroll_keyword_value(&property, &text).ok_or_else(|| input.new_custom_error(()))?;
        let important = important(input)?;
        return Ok(Some(format!("{property}: {value}{important};")));
    }
    Ok(None)
}

fn parse_value_body<'i>(
    name: &CowRcStr<'i>,
    input: &mut Parser<'i, '_>,
) -> Result<bool, ParseError<'i, ()>> {
    if let Some(valid) = engine_grammar(name, input) {
        return Ok(valid);
    }
    if let Some(valid) = keyword_property(name, input) {
        return valid;
    }
    // Lightning CSS does not model interest-invoker delays: `normal | <time>`, and the
    // shorthand takes one or two of those.
    if let Some(count) = interest_delay_arity(name) {
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
    // Lightning CSS reads a shadow's `none` as an unparsed value; Chrome keeps it.
    if matches!(
        name.to_ascii_lowercase().as_str(),
        "box-shadow" | "-webkit-box-shadow" | "text-shadow"
    ) && input
        .try_parse(|input| {
            input.expect_ident_matching("none")?;
            input.expect_exhausted()
        })
        .is_ok()
    {
        return Ok(true);
    }
    let property = Property::parse(
        PropertyId::from(lightningcss::values::string::CowArcStr::from(
            name.to_string(),
        )),
        input,
        &ParserOptions::default(),
    )
    .map_err(|_| input.new_custom_error(()))?;
    Ok(!matches!(&property, Property::Unparsed(property) if !deferred_value(&property.value)))
}

/// The keyword properties Lightning CSS does not model: the CSS2 `font-variant` shorthand (its
/// measured caps subset, without arbitrary custom tokens) and `writing-mode` (Chrome's
/// keywords). `None` for any other property.
fn keyword_property<'i>(
    name: &str,
    input: &mut Parser<'i, '_>,
) -> Option<Result<bool, ParseError<'i, ()>>> {
    let allowed: &[&str] = if name.eq_ignore_ascii_case("font-variant") {
        &[
            "normal",
            "small-caps",
            "initial",
            "inherit",
            "unset",
            "revert",
            "revert-layer",
        ]
    } else if name.eq_ignore_ascii_case("writing-mode") {
        &WRITING_MODES
    } else {
        return None;
    };
    let mut read = || {
        let value = input.expect_ident_cloned()?.to_ascii_lowercase();
        input.expect_exhausted()?;
        Ok(allowed.contains(&value.as_str()))
    };
    Some(read())
}

/// turbo-dom#338: Lightning CSS models neither the tree-counting functions
/// (`sibling-index()` / `sibling-count()`) nor `shape()` (nor `clip-path: path()`). A value
/// holding one takes the shared declaration grammar's answer (the one `el.style` and
/// `CSS.supports` use) when that grammar decides. For a property it leaves to Lightning CSS,
/// a tree-counting function is checked as the integer it computes to; `None` leaves any
/// other value to Lightning CSS (a `var()` value, a property without such a function).
fn engine_grammar(name: &str, input: &mut Parser<'_, '_>) -> Option<bool> {
    use turbo_dom_parser::rtdom::css_shorthand::{
        expand, is_shorthand, longhand_alias, longhand_value, Spelled,
    };
    let start = input.state();
    let value = consume(input);
    let lower = value.to_ascii_lowercase();
    let tree = lower.contains("sibling-index(") || lower.contains("sibling-count(");
    let name = name.to_ascii_lowercase();
    let name = longhand_alias(&name).unwrap_or(&name);
    let answer = match () {
        () if !tree && !lower.contains("shape(") && !lower.contains("path(") => None,
        () if is_shorthand(name) => Some(expand(name, &value).is_some()),
        () => match longhand_value(name, &value) {
            Spelled::Valid(_) => Some(true),
            Spelled::Invalid => Some(false),
            Spelled::Unchecked(_) if tree && !lower.contains("var(") => {
                Some(lightning_accepts(name, &tree_counted(&lower)))
            }
            Spelled::Unchecked(_) => None,
        },
    };
    if answer.is_none() {
        input.reset(&start);
    }
    answer
}

/// `value` with each tree-counting function as the integer 1 (both compute to one or more).
fn tree_counted(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(at) = rest.find("sibling-") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let function = ["sibling-index(", "sibling-count("]
            .iter()
            .find(|name| tail.starts_with(**name))
            .and_then(|name| {
                let close = tail[name.len()..].find(')')?;
                tail[name.len()..name.len() + close]
                    .trim()
                    .is_empty()
                    .then_some(name.len() + close + 1)
            });
        match function {
            Some(length) => {
                out.push('1');
                rest = &tail[length..];
            }
            None => {
                out.push_str("sibling-");
                rest = &tail[8..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether Lightning CSS parses `value` as property `name` (not as an unparsed value).
fn lightning_accepts(name: &str, value: &str) -> bool {
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    Property::parse(
        PropertyId::from(lightningcss::values::string::CowArcStr::from(
            name.to_string(),
        )),
        &mut input,
        &ParserOptions::default(),
    )
    .is_ok_and(|property| !matches!(property, Property::Unparsed(_)))
        && input.expect_exhausted().is_ok()
}

/// A declaration's optional `!important` and the end of its value: the suffix to serialize.
fn important<'i>(input: &mut Parser<'i, '_>) -> Result<&'static str, ParseError<'i, ()>> {
    let important = input.try_parse(parse_important).is_ok();
    input.expect_exhausted()?;
    Ok(if important { " !important" } else { "" })
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
            if name == "layer" {
                return layer_names(&prelude)
                    .map(|names| format!("@layer {names};"))
                    .ok_or(());
            }
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
        if !valid_rule(&test) || (name == "counter-style" && reserved_counter_style(&prelude)) {
            return Err(input.new_custom_error(()));
        }
        let prelude = if name == "keyframes" || name.ends_with("-keyframes") {
            keyframes_name(&prelude).ok_or_else(|| input.new_custom_error(()))?
        } else {
            prelude
        };
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
        } else if self.nested
            && matches!(
                name.as_str(),
                "media" | "supports" | "container" | "scope" | "starting-style" | "layer"
            )
        {
            // Nested inside a style rule's body (turbo-dom#358): the group at-rule admits
            // bare declarations (CSSNestedDeclarations) and nested style rules.
            let mut nested = CssParser {
                nested: true,
                ..Default::default()
            };
            RuleBodyParser::new(input, &mut nested)
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
    fn a_rule_keeps_a_none_shadow() {
        // Chrome 156: `.a { box-shadow: none }` keeps the declaration (MUI's Card writes it;
        // turbo-dom#227). Lightning CSS reads `none` as an unparsed value.
        assert_eq!(
            parse(".a{box-shadow:none;color:red}"),
            ".a { box-shadow: none; color: red; }"
        );
        assert_eq!(
            parse_one(".a{text-shadow:none}").as_deref(),
            Some(".a { text-shadow: none; }")
        );
        assert!(!parse(".a{box-shadow:none none}").contains("box-shadow"));
    }

    #[test]
    fn if_function_admits_chrome156_conditions_only() {
        // Chrome 156 control: css_supports_chrome156.json (the browser-env CSS.supports test).
        for (value, valid) in [
            ("if(style(--x: 1): red; else: blue)", true),
            (
                "if((style(--x) or (media(print))) and supports(display: grid): red)",
                true,
            ),
            ("if(not media(print): red;)", true),
            ("if(else:)", true),
            ("if(red)", false),
            ("if()", false),
            ("if(: red)", false),
            ("if(style(--x); red)", false),
            ("if(not style(--x) and media(print): red)", false),
            (
                "if(style(--x) and media(print) or supports(a: b): red)",
                false,
            ),
            ("if(style(--x) and: red)", false),
            ("if(style(--x) red: blue)", false),
            ("if((style(--x): red)", false),
            ("if((): red)", false),
            ("if(else: red;;)", false),
        ] {
            let parsed = parse(&format!("a {{ color: {value} }}"));
            assert_eq!(parsed.contains("color:"), valid, "{value}: {parsed}");
        }
    }

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

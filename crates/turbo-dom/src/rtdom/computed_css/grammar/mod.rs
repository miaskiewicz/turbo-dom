//! Shared ordinary-property grammar, extracted from the native CSSOM adapter.
//! This optional module validates scalar grammar and preserves admitted lexemes; raw
//! custom-property DOM strings remain the separate UTF-16 computation contract.
mod backend_limits;
mod deferred_tokens;
mod general_enclosed;

use super::{ComputationLimits, ComputedCssError, CssText};
use crate::rtdom::css_values::zoom_value;
use cssparser::{
    parse_important, CowRcStr, Delimiter, ParseError, ParseErrorKind, Parser, ParserInput,
};
use lightningcss::properties::{
    custom::{Token, TokenList, TokenOrValue},
    Property, PropertyId,
};
use lightningcss::stylesheet::ParserOptions;

/// Whether native CSSOM admits an ordinary property name.
#[must_use]
pub fn is_supported_property(name: &str) -> bool {
    if let Some(admitted) = vendor_property(name) {
        return admitted;
    }
    !matches!(PropertyId::from(name), PropertyId::Custom(_))
        || interest_delay_arity(name).is_some()
        || EXTRA_PROPERTIES.contains(&name)
        || is_scroll_keyword_property(name)
}

fn vendor_property(name: &str) -> Option<bool> {
    // Validated f7e74b275db7f23649db9dc01b7f2ab4a3dddeb1 admission policy:
    // Lightning CSS models legacy names Chromium ignores, in authored ASCII case.
    if ["-moz-", "-ms-", "-o-"].iter().any(|prefix| {
        name.get(..prefix.len())
            .is_some_and(|text| text.eq_ignore_ascii_case(prefix))
    }) {
        return Some(false);
    }
    // turbo-dom#441: a -webkit-* name whose Chrome 156 acceptance disagrees with Lightning
    // CSS is pinned here against `css_webkit_prefix_chrome156.json`. Accept the seven
    // properties Chrome serves but Lightning CSS models as Custom (ASCII case-insensitive
    // per CSS 2.2 §4.1.1 — the Chrome page admits `-WEBKIT-LINE-CLAMP`); reject
    // `-webkit-backdrop-filter`, the one Lightning CSS models but Chrome 156 does not.
    if name.starts_with('-') {
        // R1 L3: match the -webkit-* allowlist case-insensitively on the borrowed
        // name instead of allocating a lowercase copy on this hot path.
        // CSS 2.2 §4.1.1 says property names are ASCII case-insensitive,
        // so `-WEBKIT-text-fill-color` must still be accepted.
        let admitted = [
            "-webkit-text-fill-color",
            "-webkit-text-stroke",
            "-webkit-text-stroke-color",
            "-webkit-text-stroke-width",
            "-webkit-font-smoothing",
            "-webkit-tap-highlight-color",
            "-webkit-line-clamp",
            // turbo-dom R1 M1: Chrome 156 admits -webkit- aliases of the
            // CSS Multicol 1 column-* longhands + shorthand (the
            // css_webkit_prefix_chrome156.json control). Lightning CSS
            // models them as Custom (no -webkit- alias to the
            // column-* PropertyId), so admit the names here.
            "-webkit-column-count",
            "-webkit-column-width",
            "-webkit-column-gap",
            "-webkit-column-rule",
        ];
        if admitted.iter().any(|n| name.eq_ignore_ascii_case(n)) {
            return Some(true);
        }
        if name.eq_ignore_ascii_case("-webkit-backdrop-filter") {
            return Some(false);
        }
    }
    None
}

const EXTRA_PROPERTIES: &[&str] = &[
    "writing-mode",
    "content",
    "zoom",
    "font-variant",
    "float",
    "clear",
    "direction",
    "unicode-bidi",
    "table-layout",
    "caption-side",
    "empty-cells",
    "cursor",
    "user-select",
    "-webkit-user-select",
    "vertical-align",
    "z-index",
    "visibility",
    "pointer-events",
    "touch-action",
    "will-change",
    "appearance",
    "-webkit-appearance",
];

// Lightning CSS does not model these CSSOM View / CSS Scroll Snap keyword
// properties; admit exactly their grammar with Chrome156's serialization.
const SCROLL_KEYWORD_PROPERTIES: [&str; 4] = [
    "scroll-behavior",
    "scroll-snap-align",
    "scroll-snap-stop",
    "scroll-snap-type",
];
/// Serialize the native CSSOM scroll keyword grammar, including deferred values.
#[must_use]
pub fn scroll_keyword_value(name: &str, value: &str) -> Option<String> {
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
        ("scroll-snap-type", [axis, rest @ ..]) => scroll_snap_type(axis, rest),
        ("scroll-snap-align", words) => scroll_snap_align(words),
        _ => None,
    }
}
fn scroll_snap_type(axis: &str, rest: &[&str]) -> Option<String> {
    if !["x", "y", "block", "inline", "both"].contains(&axis) {
        return None;
    }
    match rest {
        [] | ["proximity"] => Some(axis.into()),
        ["mandatory"] => Some(format!("{axis} mandatory")),
        _ => None,
    }
}

fn scroll_snap_align(words: &[&str]) -> Option<String> {
    let [first, rest @ ..] = words else {
        return None;
    };
    let allowed = ["none", "start", "end", "center"];
    if rest.len() > 1 || !allowed.contains(first) || rest.iter().any(|word| !allowed.contains(word))
    {
        return None;
    }
    match rest {
        [second] if second != first => Some(format!("{first} {second}")),
        _ => Some((*first).into()),
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

/// Validate a declaration token stream without rewriting its source spelling.
/// This lower-level entry preserves the existing native CSSOM parser contract.
///
/// # Errors
/// Returns the tokenizer/property parser error for malformed values.
pub fn parse_value_body<'i>(
    name: &CowRcStr<'i>,
    input: &mut Parser<'i, '_>,
) -> Result<bool, ParseError<'i, ()>> {
    if let Some(valid) = engine_grammar(name, input) {
        return Ok(valid);
    }
    if let Some(valid) = keyword_property(name, input) {
        return valid;
    }
    if let Some(count) = interest_delay_arity(name) {
        return Ok(interest_delay_value(input, count));
    }
    if shadow_none(name, input) {
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

fn interest_delay_value(input: &mut Parser<'_, '_>, count: usize) -> bool {
    let mut values = 0;
    while !input.is_exhausted() {
        let valid = input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
            || input.try_parse(time).is_ok();
        if !valid {
            return false;
        }
        values += 1;
    }
    values >= 1 && values <= count
}

// Lightning CSS reads a shadow's `none` as an unparsed value; Chrome keeps it.
fn shadow_none(name: &str, input: &mut Parser<'_, '_>) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "box-shadow" | "-webkit-box-shadow" | "text-shadow"
    ) && input
        .try_parse(|input| {
            input.expect_ident_matching("none")?;
            input.expect_exhausted()
        })
        .is_ok()
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
    use crate::rtdom::css_shorthand::{
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
        if let Some(length) = function {
            out.push('1');
            rest = &tail[length..];
        } else {
            out.push_str("sibling-");
            rest = &tail[8..];
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

/// Parse the native zoom grammar with its established keyword/deferred serialization.
///
/// # Errors
/// Returns a CSS parser error when the value is neither zoom nor an admitted substitution.
pub fn zoom_declaration<'i>(input: &mut Parser<'i, '_>) -> Result<String, ParseError<'i, ()>> {
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
    let deferred = property_deferred(&property);
    if !deferred {
        return Err(input.new_custom_error(()));
    }
    let text = input.slice_from(start).trim();
    let keyword = ["inherit", "initial", "unset", "revert", "revert-layer"]
        .into_iter()
        .find(|keyword| text.eq_ignore_ascii_case(keyword));
    Ok(keyword.map_or_else(|| text.to_string(), str::to_string))
}

/// Whether a name uses the native CSSOM scroll-keyword serialization path.
#[must_use]
pub fn is_scroll_keyword_property(name: &str) -> bool {
    SCROLL_KEYWORD_PROPERTIES.contains(&name)
}

fn consume(input: &mut Parser<'_, '_>) -> String {
    let start = input.position();
    while input.next_including_whitespace_and_comments().is_ok() {}
    input.slice_from(start).trim().to_string()
}

/// Admit an ordinary declaration value using the same grammar as native CSSOM.
/// The caller retains custom-property raw tokens separately; unknown property names
/// and custom-property names are not ordinary-property admissions.
#[must_use]
pub fn declaration_value(name: &str, raw: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    if !is_supported_property(&name) {
        return None;
    }
    declaration_value_known(&name, raw)
}

fn declaration_value_known(name: &str, raw: &str) -> Option<String> {
    if is_scroll_keyword_property(name) {
        return scroll_keyword_value(name, raw);
    }
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    if name == "zoom" {
        return input.parse_entirely(zoom_declaration).ok();
    }
    let name = CowRcStr::from(name);
    input
        .parse_entirely(|input| parse_value_body(&name, input))
        .ok()?
        .then(|| raw.trim().to_string())
}

/// Canonical media grammar types. They borrow the source only while preparation
/// evaluates it, so a compiled model need not create a self-referential syntax tree.
pub use lightningcss::media_query::{
    MediaCondition, MediaFeature, MediaFeatureComparison, MediaFeatureId, MediaFeatureName,
    MediaFeatureValue, MediaList, MediaQuery, MediaType, Operator, Qualifier, QueryFeature,
};
pub use lightningcss::rules::supports::SupportsCondition;
use lightningcss::traits::{Parse, ParseWithOptions};
pub use lightningcss::values::{
    length::{Length, LengthValue},
    ratio::Ratio,
    resolution::Resolution,
};

/// Invalid condition syntax, distinct from an unavailable environment capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionError {
    InvalidSyntax,
    NestingLimit,
    Computation(ComputedCssError),
    RequiresGrammar(String),
}

pub(super) fn parser_nesting(error: &ParseError<'_, lightningcss::error::ParserError<'_>>) -> bool {
    matches!(
        &error.kind,
        ParseErrorKind::Custom(lightningcss::error::ParserError::MaximumNestingDepth)
    )
}

fn condition_error(error: &ParseError<'_, lightningcss::error::ParserError<'_>>) -> ConditionError {
    if parser_nesting(error) {
        ConditionError::NestingLimit
    } else {
        ConditionError::InvalidSyntax
    }
}

/// Parse a complete media query list using the shared canonical grammar.
/// This raw AST helper applies its own pinned global backend component bound;
/// use the context seam for an additional explicit caller quota.
/// Unknown/general-enclosed nodes remain explicit in the returned AST.
///
/// # Errors
/// Returns [`ConditionError`] for invalid syntax or trailing input.
pub fn parse_media_condition(source: &str) -> Result<MediaList<'_>, ConditionError> {
    check_raw_ast_depth(source)?;
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    let authored_empty = input.is_exhausted();
    let start = input.state();
    match input.parse_entirely(|input| MediaList::parse(input, &ParserOptions::default())) {
        Ok(list) if !authored_empty && list.media_queries.is_empty() => {
            // Lightning's list parser treats an EndOfInput query failure as an
            // empty list. Its CSSOM spelling is `not all`, while a genuinely
            // empty authored media attribute matches all. Keep those distinct.
            Ok(MediaList {
                media_queries: vec![MediaQuery {
                    qualifier: Some(Qualifier::Not),
                    media_type: MediaType::All,
                    condition: None,
                }],
            })
        }
        Ok(list) => Ok(list),
        Err(error) if parser_nesting(&error) => Err(ConditionError::NestingLimit),
        Err(_) => {
            input.reset(&start);
            input
                .parse_entirely(general_enclosed::parse)
                .map_err(|error| condition_error(&error))
        }
    }
}

/// Parse a complete supports condition without evaluating declaration/selector leaves.
/// This raw AST helper applies its own pinned global backend component bound;
/// use the context seam for an additional explicit caller quota.
///
/// # Errors
/// Returns [`ConditionError`] for invalid syntax or trailing input.
pub fn parse_supports_condition(source: &str) -> Result<SupportsCondition<'_>, ConditionError> {
    check_raw_ast_depth(source)?;
    parse_supports_condition_body(source)
}
fn parse_supports_condition_body(source: &str) -> Result<SupportsCondition<'_>, ConditionError> {
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    input
        .parse_entirely(SupportsCondition::parse)
        .map_err(|error| condition_error(&error))
}

/// Parse an ordered comma list of canonical layer paths, retaining escaped identifiers
/// as single components. This is syntax only: the caller owns layer registration/order.
/// Chrome159 admits CSS-wide spellings here; no separate custom-ident blacklist applies.
///
/// # Errors
/// Returns [`ConditionError`] for malformed paths, empty lists or trailing input.
pub fn parse_layer_names(source: &str) -> Result<Vec<Vec<String>>, ConditionError> {
    use lightningcss::rules::layer::LayerName;
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    input
        .parse_entirely(|input| input.parse_comma_separated(LayerName::parse))
        .map(|names| {
            names
                .into_iter()
                .map(|name| name.0.iter().map(ToString::to_string).collect())
                .collect()
        })
        .map_err(|error| condition_error(&error))
}

/// Evaluate declaration admission for a supports leaf, including custom names and
/// an optional valid priority. Canonical token parsing preserves library resource
/// failures instead of converting them to an unsupported declaration.
///
/// # Errors
/// Returns a resource error from the canonical value parser; invalid values are false.
pub fn supports_declaration(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<bool, ConditionError> {
    check_grammar_limits(&CssText::from_units(raw.encode_utf16().collect()), limits)?;
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    let admitted = input.parse_until_before(Delimiter::Bang | Delimiter::Semicolon, |input| {
        let start = input.position();
        TokenList::parse_with_options(input, &ParserOptions::default())?;
        let text = input.slice_from(start);
        if name.starts_with("--") {
            return Ok(true);
        }
        Ok(declaration_value(name, text).is_some())
    });
    let admitted = match admitted {
        Ok(admitted) => admitted,
        Err(error) if parser_nesting(&error) => return Err(ConditionError::NestingLimit),
        Err(_) => return Ok(false),
    };
    let _ = input.try_parse(parse_important);
    Ok(admitted && input.expect_exhausted().is_ok())
}

/// Checked ordinary-property admission for new computed-style callers. The native
/// Option API stays compatible; this wrapper disambiguates its invalid-value result
/// from the canonical parser's own resource ceiling. Callers still guard input with
/// their explicit component limits before invoking grammar.
///
/// # Errors
/// Returns an upstream grammar resource failure rather than CSS invalidity.
pub fn declaration_value_checked(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<Option<String>, ConditionError> {
    check_grammar_limits(&CssText::from_units(raw.encode_utf16().collect()), limits)?;
    let name = name.to_ascii_lowercase();
    if !is_supported_property(&name) {
        return Ok(None);
    }
    let admitted = declaration_value_known(&name, raw);
    if admitted.is_some() {
        return Ok(admitted);
    }
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    match input
        .parse_entirely(|input| TokenList::parse_with_options(input, &ParserOptions::default()))
    {
        Err(error) if parser_nesting(&error) => Err(ConditionError::NestingLimit),
        _ => Ok(None),
    }
}

/// Full computed-style admission can require a grammar not shared by the native
/// Typed OM adapter yet. It must not turn that missing capability into CSS false.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationAdmission {
    Valid(String),
    Invalid,
    RequiresGrammar(String),
}
/// Admit an ordinary computed-style declaration using the measured registry and
/// canonical longhand grammar, retaining the legacy raw-value contract.
///
/// # Errors
/// Caller resource failures are preserved before recursive parsing.
pub fn computed_declaration_value(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<DeclarationAdmission, ConditionError> {
    check_grammar_limits(&CssText::from_units(raw.encode_utf16().collect()), limits)?;
    let name = name.to_ascii_lowercase();
    if !computed_property_known(&name) {
        return Ok(DeclarationAdmission::Invalid);
    }
    if let Some(valid) = checked_deferred(raw, limits)? {
        return Ok(deferred_admission(valid, raw));
    }
    if crate::rtdom::css_shorthand::is_shorthand(&name) {
        return Ok(
            if crate::rtdom::css_shorthand::expand_css(&name, raw).is_some() {
                DeclarationAdmission::Valid(trim_css(raw).to_owned())
            } else {
                DeclarationAdmission::Invalid
            },
        );
    }
    let legacy = is_supported_property(&name);
    if name == "zoom" || is_scroll_keyword_property(&name) {
        return checked_admission(&name, raw, limits);
    }
    computed_longhand(&name, raw, limits, legacy)
}
fn computed_longhand(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
    legacy: bool,
) -> Result<DeclarationAdmission, ConditionError> {
    use crate::rtdom::css_shorthand::{longhand_value_css, Spelled};
    match longhand_value_css(name, raw) {
        Spelled::Valid(_) => Ok(DeclarationAdmission::Valid(trim_css(raw).to_string())),
        Spelled::Invalid => Ok(DeclarationAdmission::Invalid),
        Spelled::Unchecked(normalized) => {
            if registry_keyword(name, raw) {
                return Ok(DeclarationAdmission::Valid(trim_css(raw).to_string()));
            }
            if legacy {
                let validation = if name == "transform" {
                    normalized.as_str()
                } else {
                    raw
                };
                checked_admission(name, validation, limits).map(|value| match value {
                    DeclarationAdmission::Valid(_) => {
                        DeclarationAdmission::Valid(trim_css(raw).to_string())
                    }
                    value => value,
                })
            } else {
                Ok(DeclarationAdmission::RequiresGrammar(name.to_string()))
            }
        }
    }
}

fn checked_admission(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<DeclarationAdmission, ConditionError> {
    declaration_value_checked(name, raw, limits)
        .map(|value| value.map_or(DeclarationAdmission::Invalid, DeclarationAdmission::Valid))
}

fn computed_property_known(name: &str) -> bool {
    vendor_property(name) != Some(false)
        && !name.starts_with("--")
        && (is_supported_property(name) || super::property_registry::is_known(name))
}

fn trim_css(value: &str) -> &str {
    value.trim_matches(|c| matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' '))
}

// None is ordinary grammar; Some(false) is invalid deferred syntax, never an
// unsupported-property fallback. All recursive consumers follow the caller guard.
fn checked_deferred(raw: &str, limits: ComputationLimits) -> Result<Option<bool>, ConditionError> {
    let value = CssText::from_units(raw.encode_utf16().collect());
    check_grammar_limits(&value, limits)?;
    if !super::valid_custom_value_syntax(&value, limits).map_err(ConditionError::Computation)? {
        return Ok(Some(false));
    }
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    let tokens = input.parse_until_before(Delimiter::Bang | Delimiter::Semicolon, |input| {
        deferred_tokens::parse(input)
    });
    let tokens = match tokens {
        Ok(tokens) => tokens,
        Err(error) if parser_nesting(&error) => return Err(ConditionError::NestingLimit),
        Err(_) => return Ok(Some(false)),
    };
    if input.expect_exhausted().is_err() {
        return Ok(Some(false));
    }
    match checked_deferred_tokens(&tokens, true) {
        Some(true) => Ok(Some(true)),
        Some(false) => Ok(None),
        None => Ok(Some(false)),
    }
}
fn checked_deferred_tokens(tokens: &TokenList<'_>, allow_semicolon: bool) -> Option<bool> {
    let mut found = false;
    for token in &tokens.0 {
        found |= checked_deferred_token(token, allow_semicolon)?;
    }
    Some(found)
}
fn checked_deferred_token(token: &TokenOrValue<'_>, allow_semicolon: bool) -> Option<bool> {
    match token {
        TokenOrValue::Var(value) => {
            check_deferred_fallback(value.fallback.as_ref())?;
            Some(true)
        }
        TokenOrValue::Env(value) => {
            check_deferred_fallback(value.fallback.as_ref())?;
            Some(true)
        }
        TokenOrValue::Function(value) if value.name.eq_ignore_ascii_case("if") => {
            if !if_function(&value.arguments) {
                return None;
            }
            checked_deferred_tokens(&value.arguments, true)?;
            Some(true)
        }
        TokenOrValue::Function(value) => checked_deferred_tokens(&value.arguments, allow_semicolon),
        TokenOrValue::Token(Token::Delim('!')) => None,
        TokenOrValue::Token(Token::Semicolon) if !allow_semicolon => None,
        _ => Some(false),
    }
}
fn check_deferred_fallback(fallback: Option<&TokenList<'_>>) -> Option<()> {
    if let Some(fallback) = fallback {
        checked_deferred_tokens(fallback, false)?;
    }
    Some(())
}

fn deferred_admission(valid: bool, raw: &str) -> DeclarationAdmission {
    if valid {
        DeclarationAdmission::Valid(trim_css(raw).to_string())
    } else {
        DeclarationAdmission::Invalid
    }
}

fn registry_keyword(name: &str, raw: &str) -> bool {
    let Some(keywords) = super::property_registry::keywords(name) else {
        return false;
    };
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    let parsed: Result<bool, ParseError<'_, ()>> = input.parse_entirely(|input| {
        let value = input.expect_ident_cloned()?;
        Ok(keywords
            .iter()
            .any(|keyword| value.eq_ignore_ascii_case(keyword)))
    });
    parsed.unwrap_or(false)
}

/// Full declaration admission in a CSS supports CONDITION, not the distinct
/// two-argument CSS.supports(property,value) API. Supports conditions admit an
/// optional priority and an empty custom value; ordinary declarations do not.
///
/// # Errors
/// Missing property grammar and caller resources remain explicit typed errors.
pub fn supports_condition_declaration(
    name: &str,
    raw: &str,
    limits: ComputationLimits,
) -> Result<bool, ConditionError> {
    check_grammar_limits(&CssText::from_units(raw.encode_utf16().collect()), limits)?;
    let mut source = ParserInput::new(raw);
    let mut input = Parser::new(&mut source);
    let start = input.position();
    let parsed = input.parse_until_before(Delimiter::Bang | Delimiter::Semicolon, |input| {
        TokenList::parse_with_options(input, &ParserOptions::default())
    });
    match parsed {
        Ok(_) => {}
        Err(error) if parser_nesting(&error) => return Err(ConditionError::NestingLimit),
        Err(_) => return Ok(false),
    }
    let body = input.slice_from(start);
    let _ = input.try_parse(parse_important);
    if input.expect_exhausted().is_err() {
        return Ok(false);
    }
    if name.starts_with("--") {
        return custom_supports(name, body, limits);
    }
    match computed_declaration_value(name, body, limits)? {
        DeclarationAdmission::Valid(_) => Ok(true),
        DeclarationAdmission::Invalid => Ok(false),
        DeclarationAdmission::RequiresGrammar(name) => Err(ConditionError::RequiresGrammar(name)),
    }
}
fn custom_supports(
    name: &str,
    body: &str,
    limits: ComputationLimits,
) -> Result<bool, ConditionError> {
    if name.len() <= 2 {
        return Ok(false);
    }
    let mut source = ParserInput::new(name);
    let mut input = Parser::new(&mut source);
    let name_valid: Result<(), ParseError<'_, ()>> = input.parse_entirely(|input| {
        input.expect_ident()?;
        Ok(())
    });
    if name_valid.is_err() {
        return Ok(false);
    }
    // Full deferred validation also checks unused fallback var/if grammar; unlike
    // ordinary admission a valid custom declaration need not contain a substitution.
    Ok(checked_deferred(body, limits)? != Some(false))
}

/// Context-aware CSS media lists use forgiving per-query syntax recovery. This
/// differs from the legacy strict parser: one invalid comma-delimited query is
/// `not all`, while other queries retain their ordinary matching semantics.
///
/// # Errors
/// Caller computation limits and upstream parser nesting failures remain typed.
pub fn parse_media_condition_context(
    source: &str,
    limits: ComputationLimits,
) -> Result<MediaList<'_>, ConditionError> {
    check_grammar_limits(
        &CssText::from_units(source.encode_utf16().collect()),
        limits,
    )?;
    let mut source = ParserInput::new(source);
    let mut input = Parser::new(&mut source);
    let mut queries = Vec::new();
    if input.is_exhausted() {
        return Ok(MediaList {
            media_queries: queries,
        });
    }
    loop {
        let mut used = false;
        let parsed = input.parse_until_before(Delimiter::Comma, |input| {
            input.parse_entirely(|input| general_enclosed::query(input, &mut used))
        });
        let query = match parsed {
            Ok(query) => query,
            Err(error) if parser_nesting(&error) => return Err(ConditionError::NestingLimit),
            Err(_) => MediaQuery {
                qualifier: Some(Qualifier::Not),
                media_type: MediaType::All,
                condition: None,
            },
        };
        queries.push(query);
        if input.is_exhausted() {
            break;
        }
        input
            .expect_comma()
            .expect("comma-delimited parser stops at its comma boundary");
    }
    Ok(MediaList {
        media_queries: queries,
    })
}

/// Inclusive nesting depth admitted by pinned `LightningCSS` alpha.72 `TokenList`.
/// Its `parse_raw`/`parse_into` both reject `depth > 500`, starting at root depth zero.
/// The upstream crate exports no constant; review this source-pinned ceiling when
/// updating that dependency. This is its existing parser bound, not adapter policy.
pub const BACKEND_MAX_COMPONENT_DEPTH: usize = 500;
/// Resource-only preflight failures, separate from CSS syntax/capability errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarLimitError {
    NestingLimit,
    Computation(ComputedCssError),
}
impl From<GrammarLimitError> for ConditionError {
    fn from(error: GrammarLimitError) -> Self {
        match error {
            GrammarLimitError::NestingLimit => Self::NestingLimit,
            GrammarLimitError::Computation(error) => Self::Computation(error),
        }
    }
}
/// Guard checked grammar entrypoints before the recursive backend parser runs.
/// The caller's stricter policy and original unit quota always take precedence.
/// This performs resource inspection only, not CSS syntax/value admission.
///
/// # Errors
/// Returns the original computation error for a caller bound; returns the backend
/// nesting error when the pinned parser's own bound is the stricter constraint.
pub fn check_grammar_limits(
    value: &CssText,
    limits: ComputationLimits,
) -> Result<(), GrammarLimitError> {
    backend_limits::check(value, limits)
}

/// Checked context supports grammar. The legacy raw parser remains available;
/// explicit-limit callers must preflight the pinned backend before recursive AST
/// parsing, not only when evaluating a declaration leaf.
///
/// # Errors
/// Preserves caller resources, the backend ceiling and original syntax errors.
pub fn parse_supports_condition_context(
    source: &str,
    limits: ComputationLimits,
) -> Result<SupportsCondition<'_>, ConditionError> {
    check_grammar_limits(
        &CssText::from_units(source.encode_utf16().collect()),
        limits,
    )?;
    parse_supports_condition_body(source)
}

// Raw AST helpers have no caller quota. Bound only their own pinned backend
// component depth, and derive the unit allowance from the actual source length.
// The global component bound is deliberately conservative for condition wrappers
// whose backend TokenList begins inside a subtree. Native legacy parsers are not
// routed through these new optional-Core AST helpers.
fn check_raw_ast_depth(source: &str) -> Result<(), GrammarLimitError> {
    let value = CssText::from_units(source.encode_utf16().collect());
    let limits = ComputationLimits {
        max_nesting: usize::MAX,
        max_output_units: value.as_units().len(),
        max_total_units: 0,
    };
    check_grammar_limits(&value, limits)
}

#[cfg(test)]
mod backend_conversion_contract {
    use super::*;
    #[test]
    fn structured_backend_error_preserves_the_resource_category() {
        // No recursive backend input is executed: this is an error API contract.
        let mut source = ParserInput::new("");
        let parser = Parser::new(&mut source);
        let error = parser.new_custom_error(lightningcss::error::ParserError::MaximumNestingDepth);
        assert_eq!(condition_error(&error), ConditionError::NestingLimit);
    }
}

// Keep the classifier separate from zoom's current PropertyId. A future backend
// may model zoom directly; only genuinely deferred token payloads are admitted.
fn property_deferred(property: &Property<'_>) -> bool {
    match property {
        Property::Custom(property) => deferred_value(&property.value),
        Property::Unparsed(property) => deferred_value(&property.value),
        _ => false,
    }
}

#[cfg(test)]
mod defensive_token_contracts {
    use super::*;
    #[test]
    fn incomplete_private_boolean_block_is_rejected() {
        // Deliberately incomplete helper IR, not a purported public parse result.
        let tokens = [TokenOrValue::Token(Token::ParenthesisBlock)];
        let refs: Vec<_> = tokens.iter().collect();
        assert_eq!(if_test(&refs), None);
        let tokens = [
            TokenOrValue::Token(Token::ParenthesisBlock),
            TokenOrValue::Token(Token::CloseParenthesis),
        ];
        let refs: Vec<_> = tokens.iter().collect();
        assert_eq!(if_test(&refs), None);
    }
    fn deferred_property(name: &str, source: &str) -> bool {
        let mut source = ParserInput::new(source);
        let mut parser = Parser::new(&mut source);
        let property = Property::parse(
            PropertyId::from(name),
            &mut parser,
            &ParserOptions::default(),
        )
        .unwrap();
        parser.expect_exhausted().unwrap();
        property_deferred(&property)
    }
    #[test]
    fn property_classifier_distinguishes_parsed_and_unparsed_real_grammar() {
        assert!(deferred_property("zoom", "var(--scale)"));
        assert!(deferred_property("width", "var(--width)"));
        assert!(!deferred_property("width", "1px"));
    }
}

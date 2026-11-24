//! Token-aware substitution. Custom token payloads never pass through UTF-8.
use super::{CssSource, CssText};
use cssparser::{ParseError, ParseErrorKind, Parser, ParserInput, Token};
use std::collections::{BTreeMap, BTreeSet};

/// Caller policy for this computation; no embedding defaults are selected here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComputationLimits {
    /// Exclusive bound on dependency and nested-token recursion depth.
    pub max_nesting: usize,
    /// Maximum UTF-16 units in each computed token stream.
    pub max_output_units: usize,
    /// Maximum summed UTF-16 units in memoized valid custom properties.
    pub max_total_units: usize,
}

/// Computation failures are separate from CSS's guaranteed-invalid value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputedCssError {
    NestingLimit,
    ResourceLimit,
    RequiresCascade,
    InvalidSyntax,
}

/// Already-computed custom values; valid empty tokens remain present.
#[derive(Debug, Default)]
pub struct CustomProperties {
    values: BTreeMap<String, CssText>,
}

impl CustomProperties {
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CssText> {
        self.values.get(name)
    }

    #[must_use]
    pub fn values(&self) -> BTreeMap<String, CssText> {
        self.values.clone()
    }
}

struct Resolver<'a> {
    parent: &'a BTreeMap<String, CssText>,
    own: &'a BTreeMap<String, CssText>,
    computed: BTreeMap<String, Option<CssText>>,
    stack: Vec<String>,
    cyclic: BTreeSet<String>,
    limits: ComputationLimits,
    retained_units: usize,
}

impl<'a> Resolver<'a> {
    fn new(
        parent: &'a BTreeMap<String, CssText>,
        own: &'a BTreeMap<String, CssText>,
        limits: ComputationLimits,
    ) -> Self {
        Self {
            parent,
            own,
            computed: BTreeMap::new(),
            stack: Vec::new(),
            cyclic: BTreeSet::new(),
            limits,
            retained_units: 0,
        }
    }

    fn cycle(&mut self, name: &str) -> bool {
        let Some(start) = self.stack.iter().position(|item| item == name) else {
            return false;
        };
        self.cyclic.extend(self.stack[start..].iter().cloned());
        true
    }

    fn property(&mut self, name: &str, depth: usize) -> Result<Option<CssText>, ComputedCssError> {
        if let Some(value) = self.computed.get(name) {
            return Ok(value.clone());
        }
        if self.cycle(name) {
            return Ok(None);
        }
        check_depth(depth, self.limits)?;
        self.stack.push(name.to_string());
        let value = self.property_value(name, depth);
        self.stack.pop();
        let value = value?;
        let value = if self.cyclic.contains(name) {
            None
        } else {
            value
        };
        if let Some(value) = &value {
            checked_length(0, value.0.len(), self.limits.max_output_units)?;
            self.retained_units = checked_length(
                self.retained_units,
                value.0.len(),
                self.limits.max_total_units,
            )?;
        }
        self.computed.insert(name.to_string(), value.clone());
        Ok(value)
    }

    fn property_value(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<Option<CssText>, ComputedCssError> {
        let Some(value) = self.own.get(name) else {
            return self.parent_value(name);
        };
        match single_ident(value).as_deref() {
            Some("initial") => Ok(None),
            Some("inherit" | "unset") => self.parent_value(name),
            Some("revert" | "revert-layer") => Err(ComputedCssError::RequiresCascade),
            _ => self.value(value, depth),
        }
    }

    fn parent_value(&self, name: &str) -> Result<Option<CssText>, ComputedCssError> {
        let value = self.parent.get(name);
        if let Some(value) = value {
            checked_length(0, value.0.len(), self.limits.max_output_units)?;
            checked_length(
                self.retained_units,
                value.0.len(),
                self.limits.max_total_units,
            )?;
        }
        Ok(value.cloned())
    }

    fn value(
        &mut self,
        value: &CssText,
        depth: usize,
    ) -> Result<Option<CssText>, ComputedCssError> {
        let source = CssSource::new(&value.0);
        let mut parser_input = ParserInput::new(source.scalar());
        let mut parser = Parser::new(&mut parser_input);
        let start = parser.state();
        let result = validate_syntax(&mut parser, depth, self.limits).and_then(|()| {
            parser.reset(&start);
            stream(&source, &mut parser, self, depth)
        });
        match result {
            Ok(units) => Ok(Some(CssText(units).trim_css())),
            Err(ComputedCssError::InvalidSyntax) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn single_ident(value: &CssText) -> Option<String> {
    let source = CssSource::new(value.as_units());
    let mut parser_input = ParserInput::new(source.scalar());
    let mut input = Parser::new(&mut parser_input);
    let name = input.expect_ident_cloned().ok()?;
    input.expect_exhausted().ok()?;
    Some(name.to_ascii_lowercase())
}

fn check_depth(depth: usize, limits: ComputationLimits) -> Result<(), ComputedCssError> {
    if depth >= limits.max_nesting {
        Err(ComputedCssError::NestingLimit)
    } else {
        Ok(())
    }
}

fn checked_length(
    current: usize,
    additional: usize,
    maximum: usize,
) -> Result<usize, ComputedCssError> {
    current
        .checked_add(additional)
        .filter(|&length| length <= maximum)
        .ok_or(ComputedCssError::ResourceLimit)
}

fn append(
    output: &mut Vec<u16>,
    next: Vec<u16>,
    limits: ComputationLimits,
) -> Result<(), ComputedCssError> {
    checked_length(output.len(), next.len(), limits.max_output_units)?;
    output.extend(next);
    Ok(())
}

fn error_kind(error: &ParseError<'_, ComputedCssError>) -> ComputedCssError {
    match error.kind {
        ParseErrorKind::Custom(error) => error,
        // cssparser can add Basic only when a successful block callback leaves
        // tokens. Our stream/validation callbacks exhaust the block on success;
        // variable callbacks either check exhaustion or consume the fallback.
        // Malformed guest tokens are rejected separately by validate_syntax.
        ParseErrorKind::Basic(_) => unreachable!("successful block callbacks exhaust their input"),
    }
}

fn validate_syntax(
    input: &mut Parser<'_, '_>,
    depth: usize,
    limits: ComputationLimits,
) -> Result<(), ComputedCssError> {
    check_depth(depth, limits)?;
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        let token = token.clone();
        validate_token(token, input, depth, limits)?;
    }
    Ok(())
}

fn validate_token(
    token: Token<'_>,
    input: &mut Parser<'_, '_>,
    depth: usize,
    limits: ComputationLimits,
) -> Result<(), ComputedCssError> {
    match token {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => input
            .parse_nested_block(|input| {
                validate_variable_syntax(input, depth + 1, limits)
                    .map_err(|error| input.new_custom_error(error))
            })
            .map_err(|error| error_kind(&error)),
        Token::Function(_)
        | Token::ParenthesisBlock
        | Token::SquareBracketBlock
        | Token::CurlyBracketBlock => input
            .parse_nested_block(|input| {
                validate_syntax(input, depth + 1, limits)
                    .map_err(|error| input.new_custom_error(error))
            })
            .map_err(|error| error_kind(&error)),
        token if token.is_parse_error() => Err(ComputedCssError::InvalidSyntax),
        _ => Ok(()),
    }
}

fn validate_variable_syntax(
    input: &mut Parser<'_, '_>,
    depth: usize,
    limits: ComputationLimits,
) -> Result<(), ComputedCssError> {
    check_depth(depth, limits)?;
    variable_name(input)?;
    if input.is_exhausted() {
        return Ok(());
    }
    input
        .expect_comma()
        .map_err(|_| ComputedCssError::InvalidSyntax)?;
    validate_syntax(input, depth, limits)
}

fn stream(
    source: &CssSource,
    input: &mut Parser<'_, '_>,
    resolver: &mut Resolver<'_>,
    depth: usize,
) -> Result<Vec<u16>, ComputedCssError> {
    check_depth(depth, resolver.limits)?;
    let mut result = Vec::new();
    loop {
        let start = input.position().byte_index();
        let token = match input.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        let next = component(source, input, resolver, depth, start, token)?;
        append(&mut result, next, resolver.limits)?;
    }
    Ok(result)
}

fn component(
    source: &CssSource,
    input: &mut Parser<'_, '_>,
    resolver: &mut Resolver<'_>,
    depth: usize,
    start: usize,
    token: Token<'_>,
) -> Result<Vec<u16>, ComputedCssError> {
    match token {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => input
            .parse_nested_block(|input| {
                variable(source, input, resolver, depth + 1)
                    .map_err(|error| input.new_custom_error(error))
            })
            .map_err(|error| error_kind(&error)),
        Token::Function(_)
        | Token::ParenthesisBlock
        | Token::SquareBracketBlock
        | Token::CurlyBracketBlock => nested(source, input, resolver, depth, start),
        _ => Ok(source
            .raw_range(start..input.position().byte_index())
            .expect("CSS parser token range")
            .0),
    }
}

fn nested(
    source: &CssSource,
    input: &mut Parser<'_, '_>,
    resolver: &mut Resolver<'_>,
    depth: usize,
    start: usize,
) -> Result<Vec<u16>, ComputedCssError> {
    let mut units = source
        .raw_range(start..input.position().byte_index())
        .expect("CSS parser block prefix")
        .0;
    let (body, end) = input
        .parse_nested_block(|input| {
            let body = stream(source, input, resolver, depth + 1)
                .map_err(|error| input.new_custom_error(error))?;
            Ok((body, input.position().byte_index()))
        })
        .map_err(|error| error_kind(&error))?;
    append(&mut units, body, resolver.limits)?;
    append(
        &mut units,
        source
            .raw_range(end..input.position().byte_index())
            .expect("CSS parser block suffix")
            .0,
        resolver.limits,
    )?;
    Ok(units)
}

fn variable(
    source: &CssSource,
    input: &mut Parser<'_, '_>,
    resolver: &mut Resolver<'_>,
    depth: usize,
) -> Result<Vec<u16>, ComputedCssError> {
    check_depth(depth, resolver.limits)?;
    let name = variable_name(input)?;
    let value = resolver.property(&name, depth)?;
    if input.is_exhausted() {
        return value
            .map(|value| value.0)
            .ok_or(ComputedCssError::InvalidSyntax);
    }
    input
        .expect_comma()
        .map_err(|_| ComputedCssError::InvalidSyntax)?;
    if let Some(value) = value {
        // Syntax was validated before substitution. Exhaust this fallback without
        // resolving its dependencies (unused fallback cycles remain inert).
        while input.next_including_whitespace_and_comments().is_ok() {}
        return Ok(value.0);
    }
    stream(source, input, resolver, depth).map(|units| CssText(units).trim_css().0)
}

fn variable_name(input: &mut Parser<'_, '_>) -> Result<String, ComputedCssError> {
    let name = input
        .expect_ident_cloned()
        .map_err(|_| ComputedCssError::InvalidSyntax)?;
    if !is_custom_name(&name) {
        return Err(ComputedCssError::InvalidSyntax);
    }
    Ok(name.to_string())
}

fn is_custom_name(name: &str) -> bool {
    name.starts_with("--") && name.len() > 2
}

/// Resolve custom-property winners against the parent's already-computed values.
/// Cascade rollback keywords must be handled by the caller's cascade stage.
pub fn compute_custom_properties(
    parent: &BTreeMap<String, CssText>,
    own: &BTreeMap<String, CssText>,
    limits: ComputationLimits,
) -> Result<CustomProperties, ComputedCssError> {
    let mut resolver = Resolver::new(parent, own, limits);
    let names: BTreeSet<_> = parent
        .keys()
        .chain(own.keys())
        .filter(|name| is_custom_name(name))
        .cloned()
        .collect();
    let mut values = BTreeMap::new();
    for name in names {
        if let Some(value) = resolver.property(&name, 0)? {
            values.insert(name, value);
        }
    }
    Ok(CustomProperties { values })
}

/// Substitute outside quoted strings/URLs, retaining raw custom token payloads.
pub fn substitute(
    value: &CssText,
    variables: &CustomProperties,
    limits: ComputationLimits,
) -> Result<Option<CssText>, ComputedCssError> {
    Resolver::new(&variables.values, &BTreeMap::new(), limits).value(value, 0)
}

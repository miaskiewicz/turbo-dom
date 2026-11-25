//! CSS general-enclosed extension for the canonical media parser's strict grammar.
//! Only structural function/parenthesis components are added; feature/type/value
//! grammar remains Lightning CSS. Callers check component resource limits first.
use super::{MediaCondition, MediaFeature, MediaList, MediaQuery, MediaType, Operator, Qualifier};
use cssparser::{Delimiter, ParseError, Parser, Token};
use lightningcss::{
    error::ParserError,
    properties::custom::{Token as RawToken, TokenList, TokenOrValue},
    stylesheet::ParserOptions,
    traits::{Parse, ParseWithOptions},
};

type Result<'i, T> = std::result::Result<T, ParseError<'i, ParserError<'i>>>;

fn invalid<'i, T>(input: &Parser<'i, '_>) -> Result<'i, T> {
    Err(input.new_custom_error(ParserError::InvalidMediaQuery))
}

pub(super) fn parse<'i>(input: &mut Parser<'i, '_>) -> Result<'i, MediaList<'i>> {
    let mut used = false;
    let mut queries = Vec::new();
    while !input.is_exhausted() {
        queries.push(input.parse_until_before(Delimiter::Comma, |input| query(input, &mut used))?);
        if !input.is_exhausted() {
            input.expect_comma()?;
        }
    }
    if !used {
        return invalid(input);
    }
    Ok(MediaList {
        media_queries: queries,
    })
}

pub(super) fn query<'i>(input: &mut Parser<'i, '_>, used: &mut bool) -> Result<'i, MediaQuery<'i>> {
    match input.try_parse(|input| {
        input.parse_entirely(|input| {
            MediaQuery::parse_with_options(input, &ParserOptions::default())
        })
    }) {
        Ok(query) => return Ok(query),
        Err(error) if super::parser_nesting(&error) => return Err(error),
        Err(_) => {}
    }
    let prefix = input
        .try_parse(|input| {
            let qualifier = input.try_parse(Qualifier::parse).ok();
            let media_type = MediaType::parse(input)?;
            Ok::<_, ParseError<'i, ParserError<'i>>>((qualifier, media_type))
        })
        .ok();
    let (qualifier, media_type, condition) = match prefix {
        Some((qualifier, media_type)) => {
            let condition = prefix_tail(input, used)?;
            (qualifier, media_type, condition)
        }
        None => (None, MediaType::All, Some(condition(input, true, used)?)),
    };
    Ok(MediaQuery {
        qualifier,
        media_type,
        condition,
    })
}

fn condition<'i>(
    input: &mut Parser<'i, '_>,
    allow_or: bool,
    used: &mut bool,
) -> Result<'i, MediaCondition<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("not"))
        .is_ok()
    {
        return Ok(MediaCondition::Not(Box::new(enclosed(input, used)?)));
    }
    chained(input, allow_or, used)
}

fn chained<'i>(
    input: &mut Parser<'i, '_>,
    allow_or: bool,
    used: &mut bool,
) -> Result<'i, MediaCondition<'i>> {
    let first = enclosed(input, used)?;
    if input.is_exhausted() {
        return Ok(first);
    }
    let operator = operation(input, allow_or)?;
    let mut conditions = vec![first, enclosed(input, used)?];
    while !input.is_exhausted() {
        if Operator::parse(input)? != operator {
            return invalid(input);
        }
        conditions.push(enclosed(input, used)?);
    }
    Ok(MediaCondition::Operation {
        operator,
        conditions,
    })
}

fn operation<'i>(input: &mut Parser<'i, '_>, allow_or: bool) -> Result<'i, Operator> {
    let operator = Operator::parse(input)?;
    if operator == Operator::Or && !allow_or {
        return invalid(input);
    }
    Ok(operator)
}

fn enclosed<'i>(input: &mut Parser<'i, '_>, used: &mut bool) -> Result<'i, MediaCondition<'i>> {
    let token = input.next()?.clone();
    match token {
        Token::ParenthesisBlock => input.parse_nested_block(|input| parenthesis(input, used)),
        Token::Function(_) => {
            let mut tokens = vec![RawToken::from(&token).into()];
            tokens.extend(input.parse_nested_block(raw_tokens)?);
            tokens.push(RawToken::CloseParenthesis.into());
            *used = true;
            Ok(MediaCondition::Unknown(TokenList(tokens)))
        }
        _ => invalid(input),
    }
}

fn parenthesis<'i>(input: &mut Parser<'i, '_>, used: &mut bool) -> Result<'i, MediaCondition<'i>> {
    if let Ok(feature) = input.try_parse(|input| {
        input.parse_entirely(|input| {
            MediaFeature::parse_with_options(input, &ParserOptions::default())
        })
    }) {
        return Ok(MediaCondition::Feature(feature));
    }
    if let Ok(condition) =
        input.try_parse(|input| input.parse_entirely(|input| condition(input, true, used)))
    {
        return Ok(condition);
    }
    let mut tokens = vec![RawToken::ParenthesisBlock.into()];
    tokens.extend(raw_tokens(input)?);
    tokens.push(RawToken::CloseParenthesis.into());
    *used = true;
    Ok(MediaCondition::Unknown(TokenList(tokens)))
}

// This converts the canonical tokenizer's components to its public raw TokenList
// representation. Unlike declaration-value parsing it assigns no var()/env()
// meaning to general-enclosed content; an invalid var leaf is still opaque here.
fn raw_tokens<'i>(input: &mut Parser<'i, '_>) -> Result<'i, Vec<TokenOrValue<'i>>> {
    let mut tokens = Vec::new();
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        let token = token.clone();
        if token.is_parse_error() {
            return invalid(input);
        }
        let close = closing_token(&token);
        tokens.push(RawToken::from(&token).into());
        if let Some(close) = close {
            tokens.extend(input.parse_nested_block(raw_tokens)?);
            tokens.push(close.into());
        }
    }
    Ok(tokens)
}

fn closing_token(token: &Token<'_>) -> Option<RawToken<'static>> {
    match token {
        Token::Function(_) | Token::ParenthesisBlock => Some(RawToken::CloseParenthesis),
        Token::SquareBracketBlock => Some(RawToken::CloseSquareBracket),
        Token::CurlyBracketBlock => Some(RawToken::CloseCurlyBracket),
        _ => None,
    }
}

#[cfg(test)]
mod defensive_fallback_contracts {
    use super::*;
    #[test]
    fn fallback_requires_an_actual_general_enclosed_extension() {
        // Direct subgrammar contract: valid canonical media is not an extension.
        for source in ["", "screen", "screen, print"] {
            let mut source = cssparser::ParserInput::new(source);
            let mut parser = Parser::new(&mut source);
            assert!(parse(&mut parser).is_err());
        }
        let mut source = cssparser::ParserInput::new("(future(foo))");
        let mut parser = Parser::new(&mut source);
        assert!(parse(&mut parser).is_ok());
        parser.expect_exhausted().unwrap();
    }
}

// The optional tail grammar is shared by media-type prefixes. The outer fallback
// normally sees a nonempty extension after canonical query parsing was attempted.
fn prefix_tail<'i>(
    input: &mut Parser<'i, '_>,
    used: &mut bool,
) -> Result<'i, Option<MediaCondition<'i>>> {
    if input.is_exhausted() {
        return Ok(None);
    }
    input.expect_ident_matching("and")?;
    Ok(Some(condition(input, false, used)?))
}
#[cfg(test)]
mod prefix_tail_contract {
    use super::*;
    #[test]
    fn absent_tail_and_present_extension_keep_distinct_grammar_results() {
        let mut empty = cssparser::ParserInput::new("");
        let mut empty = Parser::new(&mut empty);
        let mut used = false;
        assert!(prefix_tail(&mut empty, &mut used).unwrap().is_none());
        assert!(!used);
        let mut present = cssparser::ParserInput::new("and (future(foo))");
        let mut present = Parser::new(&mut present);
        assert!(prefix_tail(&mut present, &mut used).unwrap().is_some());
        present.expect_exhausted().unwrap();
        assert!(used);
        let mut invalid = cssparser::ParserInput::new("or (future(foo))");
        let mut invalid = Parser::new(&mut invalid);
        assert!(prefix_tail(&mut invalid, &mut used).is_err());
    }
}

//! Mechanical component AST for deferred-value inspection. The ordinary color
//! parser must not evaluate raw tokens in an unused substitution fallback.
use cssparser::{ParseError, Parser};
use lightningcss::{
    error::ParserError,
    properties::custom::{
        EnvironmentVariable, EnvironmentVariableName, Function, Token, TokenList, TokenOrValue,
        Variable,
    },
    stylesheet::ParserOptions,
    traits::{Parse, ParseWithOptions},
    values::{
        ident::{DashedIdentReference, Ident},
        number::CSSInteger,
    },
};
type Result<'i, T> = std::result::Result<T, ParseError<'i, ParserError<'i>>>;

// Caller first applies check_grammar_limits and valid_custom_value_syntax. This
// preserves their typed depth/resource failures and prevents malformed blocks.
pub(super) fn parse<'i>(input: &mut Parser<'i, '_>) -> Result<'i, TokenList<'i>> {
    let mut tokens = Vec::new();
    while !input.is_exhausted() {
        let token = input.next_including_whitespace_and_comments()?.clone();
        append(token, input, &mut tokens)?;
    }
    Ok(TokenList(tokens))
}
fn append<'i>(
    token: cssparser::Token<'i>,
    input: &mut Parser<'i, '_>,
    tokens: &mut Vec<TokenOrValue<'i>>,
) -> Result<'i, ()> {
    match token {
        cssparser::Token::Function(name) => {
            let value = input.parse_nested_block(|input| function(name.as_ref(), input))?;
            tokens.push(value);
        }
        cssparser::Token::ParenthesisBlock => block(
            input,
            tokens,
            Token::ParenthesisBlock,
            Token::CloseParenthesis,
        )?,
        cssparser::Token::SquareBracketBlock => block(
            input,
            tokens,
            Token::SquareBracketBlock,
            Token::CloseSquareBracket,
        )?,
        cssparser::Token::CurlyBracketBlock => block(
            input,
            tokens,
            Token::CurlyBracketBlock,
            Token::CloseCurlyBracket,
        )?,
        token => tokens.push(Token::from(&token).into()),
    }
    Ok(())
}
fn block<'i>(
    input: &mut Parser<'i, '_>,
    tokens: &mut Vec<TokenOrValue<'i>>,
    open: Token<'i>,
    close: Token<'i>,
) -> Result<'i, ()> {
    tokens.push(open.into());
    let nested = input.parse_nested_block(parse)?;
    tokens.extend(nested.0);
    tokens.push(close.into());
    Ok(())
}
fn fallback<'i>(input: &mut Parser<'i, '_>) -> Result<'i, Option<TokenList<'i>>> {
    if input.try_parse(Parser::expect_comma).is_ok() {
        Ok(Some(parse(input)?))
    } else {
        Ok(None)
    }
}
fn function<'i>(name: &str, input: &mut Parser<'i, '_>) -> Result<'i, TokenOrValue<'i>> {
    if name.eq_ignore_ascii_case("var") {
        let name = DashedIdentReference::parse_with_options(input, &ParserOptions::default())?;
        let fallback = fallback(input)?;
        return Ok(TokenOrValue::Var(Variable { name, fallback }));
    }
    if name.eq_ignore_ascii_case("env") {
        let name = EnvironmentVariableName::parse(input)?;
        let mut indices = Vec::new();
        while let Ok(index) = input.try_parse(CSSInteger::parse) {
            indices.push(index);
        }
        let fallback = fallback(input)?;
        return Ok(TokenOrValue::Env(EnvironmentVariable {
            name,
            indices,
            fallback,
        }));
    }
    let arguments = parse(input)?;
    Ok(TokenOrValue::Function(Function {
        name: Ident(name.to_owned().into()),
        arguments,
    }))
}

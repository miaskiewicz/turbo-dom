//! Resource-only inspection over the canonical tokenizer, with pinned backend edges.
use super::super::variables::{check_depth, checked_component_source, error_kind};
use super::{
    ComputationLimits, ComputedCssError, CssText, GrammarLimitError, BACKEND_MAX_COMPONENT_DEPTH,
};
use cssparser::{Parser, ParserInput, Token};
type Result<T> = std::result::Result<T, GrammarLimitError>;
#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Tokens,
    VariablePrefix,
    EnvironmentPrefix,
    IgnoredPrefix,
}
#[derive(Clone, Copy)]
struct Frame {
    raw: usize,
    backend: usize,
    mode: Mode,
}
fn semantic(depth: usize) -> Result<()> {
    if depth > BACKEND_MAX_COMPONENT_DEPTH {
        Err(GrammarLimitError::NestingLimit)
    } else {
        Ok(())
    }
}
impl Frame {
    fn check(self, limits: ComputationLimits) -> Result<()> {
        let effective = ComputationLimits {
            max_nesting: limits.max_nesting.min(BACKEND_MAX_COMPONENT_DEPTH + 1),
            ..limits
        };
        check_depth(self.raw, effective).map_err(|error| match error {
            ComputedCssError::NestingLimit if limits.max_nesting > effective.max_nesting => {
                GrammarLimitError::NestingLimit
            }
            error => GrammarLimitError::Computation(error),
        })?;
        if self.mode == Mode::Tokens {
            semantic(self.backend)?;
        }
        Ok(())
    }
    fn comma(&mut self) -> Result<()> {
        match self.mode {
            Mode::VariablePrefix => {
                self.mode = Mode::Tokens;
                semantic(self.backend)
            }
            Mode::EnvironmentPrefix => {
                self.backend += 1;
                self.mode = Mode::Tokens;
                semantic(self.backend)
            }
            _ => Ok(()),
        }
    }
    fn child(self, name: Option<&str>) -> Self {
        let (backend, mode) = if self.mode == Mode::Tokens {
            (
                self.backend + 1,
                match name {
                    Some("var") => Mode::VariablePrefix,
                    Some("env") => Mode::EnvironmentPrefix,
                    _ => Mode::Tokens,
                },
            )
        } else {
            (0, Mode::IgnoredPrefix)
        };
        Self {
            raw: self.raw + 1,
            backend,
            mode,
        }
    }
}
fn nested(input: &mut Parser<'_, '_>, frame: Frame, limits: ComputationLimits) -> Result<()> {
    input
        .parse_nested_block(|input| {
            walk(input, frame, limits).map_err(|error| input.new_custom_error(error))
        })
        .map_err(|error| error_kind(&error))
}
fn walk(input: &mut Parser<'_, '_>, mut frame: Frame, limits: ComputationLimits) -> Result<()> {
    frame.check(limits)?;
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        match token.clone() {
            Token::Comma => frame.comma()?,
            Token::Function(name) => nested(input, frame.child(Some(&name)), limits)?,
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                nested(input, frame.child(None), limits)?;
            }
            _ => {}
        }
    }
    Ok(())
}
pub(super) fn check(value: &CssText, limits: ComputationLimits) -> Result<()> {
    let source = checked_component_source(value, limits).map_err(GrammarLimitError::Computation)?;
    let mut input = ParserInput::new(source.scalar());
    walk(
        &mut Parser::new(&mut input),
        Frame {
            raw: 0,
            backend: 0,
            mode: Mode::Tokens,
        },
        limits,
    )
}

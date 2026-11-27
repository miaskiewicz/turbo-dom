//! Shared font computation and CSSOM projection; authored declarations stay raw.
use super::{
    grammar::{check_grammar_limits, GrammarLimitError},
    ComputationLimits, ComputedCssError, CssSource, CssText,
};
use crate::rtdom::css_values::css_number;
use cssparser::{Parser, ParserInput};
use lightningcss::{
    properties::font::{AbsoluteFontWeight, FontWeight},
    traits::Parse,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FontValueError {
    Computation(ComputedCssError),
    InvalidFontWeight,
    RequiresParentFontWeight,
    InvalidLineHeight,
    RequiresFontSize,
    InvalidFontSize,
    NonFiniteUsedLineHeight,
}
impl From<GrammarLimitError> for FontValueError {
    fn from(error: GrammarLimitError) -> Self {
        match error {
            GrammarLimitError::Computation(error) => Self::Computation(error),
            GrammarLimitError::NestingLimit => Self::Computation(ComputedCssError::NestingLimit),
        }
    }
}
fn parsed_weight(value: &CssText, limits: ComputationLimits) -> Result<FontWeight, FontValueError> {
    check_grammar_limits(value, limits)?;
    let source = CssSource::new(value.as_units());
    let mut input = ParserInput::new(source.scalar());
    Parser::new(&mut input)
        .parse_entirely(FontWeight::parse)
        .map_err(|_| FontValueError::InvalidFontWeight)
}
fn absolute(value: &AbsoluteFontWeight) -> f32 {
    match value {
        AbsoluteFontWeight::Normal => 400.0,
        AbsoluteFontWeight::Bold => 700.0,
        AbsoluteFontWeight::Weight(value) => *value,
    }
}
fn valid_weight(value: f32) -> Result<f32, FontValueError> {
    if value.is_finite() && (1.0..=1000.0).contains(&value) {
        Ok(value)
    } else {
        Err(FontValueError::InvalidFontWeight)
    }
}
fn inherited_weight(
    parent: Option<&CssText>,
    limits: ComputationLimits,
) -> Result<f32, FontValueError> {
    let value = parent.ok_or(FontValueError::RequiresParentFontWeight)?;
    match parsed_weight(value, limits)? {
        FontWeight::Absolute(value) => valid_weight(absolute(&value)),
        _ => Err(FontValueError::RequiresParentFontWeight),
    }
}
fn bolder(parent: f32) -> f32 {
    if parent < 350.0 {
        400.0
    } else if parent < 550.0 {
        700.0
    } else {
        900.0
    }
}
fn lighter(parent: f32) -> f32 {
    if parent < 550.0 {
        100.0
    } else if parent < 750.0 {
        400.0
    } else {
        700.0
    }
}
fn checked_text(value: &str, limit: usize) -> Result<CssText, FontValueError> {
    if value.encode_utf16().count() > limit {
        return Err(FontValueError::Computation(ComputedCssError::ResourceLimit));
    }
    Ok(CssText::from_units(value.encode_utf16().collect()))
}
/// Normalize admitted weight before inheritance. Relative values require an actual
/// computed parent weight or explicitly supplied canonical initial weight.
pub fn compute_font_weight(
    value: &CssText,
    parent: Option<&CssText>,
    limits: ComputationLimits,
) -> Result<CssText, FontValueError> {
    let value = match parsed_weight(value, limits)? {
        FontWeight::Absolute(value) => valid_weight(absolute(&value))?,
        FontWeight::Bolder => bolder(inherited_weight(parent, limits)?),
        FontWeight::Lighter => lighter(inherited_weight(parent, limits)?),
    };
    checked_text(&css_number(value), limits.max_output_units)
}
/// The existing canonical numeric line-height parser, shared by computed and used
/// stages. Other admitted forms are handled by their existing length/keyword owner.
pub fn unitless_line_height(value: &str) -> Result<Option<f32>, FontValueError> {
    let mut source = ParserInput::new(value);
    let mut input = Parser::new(&mut source);
    let parsed: Result<f32, cssparser::ParseError<'_, ()>> =
        input.parse_entirely(|input| Ok(input.expect_number()?));
    let Ok(number) = parsed else {
        return Ok(None);
    };
    if number < 0.0 || !number.is_finite() {
        return Err(FontValueError::InvalidLineHeight);
    }
    Ok(Some(number))
}
/// Measured Chrome159 font-size finalization, applied before inheritance and em
/// bases. This is CSS value policy, not an execution/resource budget or UA map.
pub const MAX_CSS_FONT_SIZE_PX: f64 = 10000.0;
pub fn finalize_font_size_px(value: f64) -> Result<f64, FontValueError> {
    if !value.is_finite() || value < 0.0 {
        return Err(FontValueError::InvalidFontSize);
    }
    Ok(value.min(MAX_CSS_FONT_SIZE_PX))
}
/// Canonical nonnegative used-font length serialization. Finite values retain the
/// shared CSS dimension saturation at `f32::MAX`; nonfinite arithmetic stays typed.
pub fn font_used_pixels(value: f64, max_output_units: usize) -> Result<CssText, FontValueError> {
    if !value.is_finite() {
        return Err(FontValueError::NonFiniteUsedLineHeight);
    }
    if value < 0.0 {
        return Err(FontValueError::InvalidLineHeight);
    }
    checked_text(
        &format!("{}px", css_number(value.min(f64::from(f32::MAX)) as f32)),
        max_output_units,
    )
}
fn actual_font_size(value: Option<f64>) -> Result<f64, FontValueError> {
    let font = value.ok_or(FontValueError::RequiresFontSize)?;
    if !font.is_finite() || !(font as f32).is_finite() || font < 0.0 {
        return Err(FontValueError::InvalidFontSize);
    }
    Ok(font)
}
/// Only unitless numbers project here. Percentages/lengths were already computed
/// in the declaring element's context and are never multiplied a second time.
/// The actual own computed font size is not an authored-box geometry estimate.
pub fn line_height_used(
    value: &CssText,
    font_size_px: Option<f64>,
    limits: ComputationLimits,
) -> Result<Option<CssText>, FontValueError> {
    check_grammar_limits(value, limits)?;
    let source = CssSource::new(value.as_units());
    let Some(number) = unitless_line_height(source.scalar())? else {
        return Ok(None);
    };
    let pixels = if number == 0.0 {
        0.0
    } else {
        let font = actual_font_size(font_size_px)?;
        f64::from(number) * font
    };
    font_used_pixels(pixels, limits.max_output_units).map(Some)
}

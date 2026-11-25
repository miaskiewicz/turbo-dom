//! Requested-key CSSOM projection keeps computed inheritance independent of layout.
use super::{
    grammar::{check_grammar_limits, GrammarLimitError},
    properties, ComputationLimits, ComputedCssError, CssSource, CssText,
};
use crate::rtdom::css_values::css_number;
use cssparser::{Parser, ParserInput};
use lightningcss::{
    properties::{
        display::{Display, DisplayKeyword},
        size::BoxSizing,
    },
    traits::Parse,
};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoxBasis {
    Content,
    Border,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisMeasurement {
    Unknown,
    NoBox,
    NonAtomicInline,
    Pixels {
        content: Option<f32>,
        border: Option<f32>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UsedMeasurement<'a> {
    pub target: u32,
    pub world: &'a [u16],
    pub horizontal: AxisMeasurement,
    pub vertical: AxisMeasurement,
}
pub struct UsedContext<'a> {
    pub target: u32,
    pub world: &'a [u16],
    pub namespace: &'a [u16],
    pub pseudo: Option<&'a str>,
    pub ordinary: &'a BTreeMap<String, CssText>,
    pub public_keys: &'a BTreeSet<String>,
    pub measurement: Option<&'a UsedMeasurement<'a>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Projection {
    Unchanged,
    NeedsMeasurement { axis: Axis, basis: BoxBasis },
    Value(CssText),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UsedValueError {
    Computation(ComputedCssError),
    UnsupportedNamespace,
    UnsupportedPseudo,
    InvalidProperty(String),
    ForeignMeasurement,
    InvalidMeasurement,
}
impl From<GrammarLimitError> for UsedValueError {
    fn from(error: GrammarLimitError) -> Self {
        match error {
            GrammarLimitError::Computation(error) => Self::Computation(error),
            GrammarLimitError::NestingLimit => Self::Computation(ComputedCssError::NestingLimit),
        }
    }
}
fn parsed<T: for<'i> Parse<'i>>(
    name: &str,
    context: &UsedContext<'_>,
    limits: ComputationLimits,
) -> Result<T, UsedValueError> {
    let value = if let Some(value) = context.ordinary.get(name) {
        check_grammar_limits(value, limits)?;
        Cow::Borrowed(value)
    } else {
        // These fixed canonical defaults are metadata, not caller CSS or the
        // projected value. Their bounded parsing does not consume its quota.
        Cow::Owned(
            properties::context_free_initial(name)
                .ok_or_else(|| UsedValueError::InvalidProperty(name.into()))?,
        )
    };
    let source = CssSource::new(value.as_units());
    let mut input = ParserInput::new(source.scalar());
    Parser::new(&mut input)
        .parse_entirely(T::parse)
        .map_err(|_| UsedValueError::InvalidProperty(name.into()))
}
fn basis(context: &UsedContext<'_>, limits: ComputationLimits) -> Result<BoxBasis, UsedValueError> {
    Ok(match parsed::<BoxSizing>("box-sizing", context, limits)? {
        BoxSizing::ContentBox => BoxBasis::Content,
        BoxSizing::BorderBox => BoxBasis::Border,
    })
}
fn pixels(value: f32, limits: ComputationLimits) -> Result<Projection, UsedValueError> {
    if !value.is_finite() || value < 0.0 {
        return Err(UsedValueError::InvalidMeasurement);
    }
    let value = format!("{}px", css_number(value));
    if value.len() > limits.max_output_units || value.len() > limits.max_total_units {
        return Err(UsedValueError::Computation(ComputedCssError::ResourceLimit));
    }
    Ok(Projection::Value(CssText::from_units(
        value.encode_utf16().collect(),
    )))
}
fn measured(
    axis: Axis,
    basis: BoxBasis,
    context: &UsedContext<'_>,
    limits: ComputationLimits,
) -> Result<Projection, UsedValueError> {
    let needed = Projection::NeedsMeasurement { axis, basis };
    let Some(measurement) = context.measurement else {
        return Ok(needed);
    };
    if measurement.target != context.target || measurement.world != context.world {
        return Err(UsedValueError::ForeignMeasurement);
    }
    let axis = match axis {
        Axis::Horizontal => measurement.horizontal,
        Axis::Vertical => measurement.vertical,
    };
    match axis {
        AxisMeasurement::Unknown => Ok(needed),
        AxisMeasurement::NoBox | AxisMeasurement::NonAtomicInline => Ok(Projection::Unchanged),
        AxisMeasurement::Pixels { content, border } => {
            let value = match basis {
                BoxBasis::Content => content,
                BoxBasis::Border => border,
            };
            value.map_or(Ok(needed), |value| pixels(value, limits))
        }
    }
}
/// Classify a canonical CSS property without reading style or layout facts.
#[must_use]
pub fn property_axis(name: &str) -> Option<Axis> {
    match name {
        "width" => Some(Axis::Horizontal),
        "height" => Some(Axis::Vertical),
        _ => None,
    }
}
/// Project only the requested public width/height; unrelated reads never need layout.
///
/// Actual provider facts decide non-atomic inline applicability: authored display
/// alone cannot prove it because layout may blockify a flex/grid/floating box.
/// # Errors
/// Namespace/pseudo support, grammar quotas, provenance and invalid selected metrics
/// remain typed. Provider errors belong to the caller and are never swallowed here.
pub fn project_used_property(
    name: &str,
    context: &UsedContext<'_>,
    limits: ComputationLimits,
) -> Result<Projection, UsedValueError> {
    if !context.public_keys.contains(name) {
        return Ok(Projection::Unchanged);
    }
    let Some(axis) = property_axis(name) else {
        return Ok(Projection::Unchanged);
    };
    if context.pseudo.is_some() {
        return Err(UsedValueError::UnsupportedPseudo);
    }
    if !context
        .namespace
        .iter()
        .copied()
        .eq("http://www.w3.org/1999/xhtml".encode_utf16())
    {
        return Err(UsedValueError::UnsupportedNamespace);
    }
    if matches!(
        parsed::<Display>("display", context, limits)?,
        Display::Keyword(DisplayKeyword::None | DisplayKeyword::Contents)
    ) {
        return Ok(Projection::Unchanged);
    }
    measured(axis, basis(context, limits)?, context, limits)
}

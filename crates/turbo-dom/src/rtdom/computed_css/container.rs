//! Canonical container conditions over actual ancestor containment and used-size facts.
use super::{
    check_component_limits,
    conditions::{self, ConditionEvaluationError, FeatureValue, MediaTruth, MEDIA_TRUTHS},
    ComputationLimits, ComputedCssError, CssSource, CssText,
};
use crate::rtdom::css_shorthand::LengthContext;
use cssparser::{ParseError, ParseErrorKind, Parser, ParserInput};
use lightningcss::{
    error::ParserError,
    media_query::{
        MediaFeatureComparison, MediaFeatureName, MediaFeatureValue, Operator, QueryFeature,
    },
    rules::container::{
        ContainerCondition, ContainerName, ContainerSizeFeature, ContainerSizeFeatureId,
    },
    stylesheet::ParserOptions,
    traits::{Parse, ParseWithOptions},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SizeKind {
    Normal,
    InlineSize,
    Size,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineAxis {
    Horizontal,
    Vertical,
}
/// Actual layout-resolved CONTENT-box dimensions, not border rect/authored widths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentSize {
    pub width_px: f64,
    pub height_px: f64,
}
/// Independently proven actual content-box axes. None never means zero.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PartialContentSize {
    pub width_px: Option<f64>,
    pub height_px: Option<f64>,
}
pub type ContainerMeasurements = std::collections::BTreeMap<u64, PartialContentSize>;
/// Ancestors are supplied nearest-first in one validated current DOM/style world.
/// names are decoded canonical container-name identifiers; metrics belong to this
/// container's font (unlike media queries' initial-font context).
#[derive(Clone, Copy)]
pub struct ContainerAncestor<'a> {
    pub id: u64,
    pub names: &'a [String],
    pub kind: SizeKind,
    pub inline_axis: InlineAxis,
    pub scroll_state: bool,
    pub size: Option<ContentSize>,
    pub length_context: &'a LengthContext<'a>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContainerMatch {
    pub matches: bool,
    pub container: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContainerError {
    InvalidSyntax,
    Computation(ComputedCssError),
    RequiresMeasurement(u64),
    RequiresStyleQuery(u64),
    RequiresScrollState(u64),
    InvalidMeasurement(u64),
    Condition(ConditionEvaluationError),
}
impl From<ComputedCssError> for ContainerError {
    fn from(error: ComputedCssError) -> Self {
        Self::Computation(error)
    }
}
impl From<ConditionEvaluationError> for ContainerError {
    fn from(error: ConditionEvaluationError) -> Self {
        match error {
            ConditionEvaluationError::Computation(error) => Self::Computation(error),
            error => Self::Condition(error),
        }
    }
}
impl From<super::grammar::GrammarLimitError> for ContainerError {
    fn from(error: super::grammar::GrammarLimitError) -> Self {
        match error {
            super::grammar::GrammarLimitError::NestingLimit => {
                Self::Computation(ComputedCssError::NestingLimit)
            }
            super::grammar::GrammarLimitError::Computation(error) => Self::Computation(error),
        }
    }
}
type Result<T> = std::result::Result<T, ContainerError>;

fn grammar_error(error: &ParseError<'_, ParserError<'_>>) -> ContainerError {
    match error.kind {
        ParseErrorKind::Custom(ParserError::MaximumNestingDepth) => {
            ComputedCssError::NestingLimit.into()
        }
        _ => ContainerError::InvalidSyntax,
    }
}
struct Prelude<'a> {
    name: Option<ContainerName<'a>>,
    condition: Option<ContainerCondition<'a>>,
}
fn prelude<'i>(
    input: &mut Parser<'i, '_>,
) -> std::result::Result<Prelude<'i>, ParseError<'i, ParserError<'i>>> {
    let name = input.try_parse(ContainerName::parse).ok();
    let condition = if name.is_some() && input.is_exhausted() {
        None
    } else {
        Some(ContainerCondition::parse_with_options(
            input,
            &ParserOptions::default(),
        )?)
    };
    Ok(Prelude { name, condition })
}

/// Decode computed container-name data using the same canonical identifier grammar.
///
/// # Errors
/// Returns input resource/grammar errors; a computed `none` is an empty name list.
pub fn container_names(value: &CssText, limits: ComputationLimits) -> Result<Vec<String>> {
    check_component_limits(value, limits)?;
    let source = CssSource::new(value.as_units());
    let mut input = ParserInput::new(source.scalar());
    let mut input = Parser::new(&mut input);
    if input
        .try_parse(|input| {
            input.expect_ident_matching("none")?;
            input.expect_exhausted()
        })
        .is_ok()
    {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    while !input.is_exhausted() {
        names.push(
            ContainerName::parse(&mut input)
                .map_err(|error| grammar_error(&error))?
                .0
                 .0
                .to_string(),
        );
    }
    if names.is_empty() {
        return Err(ContainerError::InvalidSyntax);
    }
    Ok(names)
}

#[derive(Default)]
struct Requirements {
    width: bool,
    height: bool,
    inline: bool,
    block: bool,
    scroll: bool,
}
fn feature_name<'a, 'i>(
    feature: &'a ContainerSizeFeature<'i>,
) -> &'a MediaFeatureName<'i, ContainerSizeFeatureId> {
    match feature {
        QueryFeature::Boolean { name }
        | QueryFeature::Plain { name, .. }
        | QueryFeature::Range { name, .. }
        | QueryFeature::Interval { name, .. } => name,
    }
}
fn require_feature(
    name: &MediaFeatureName<'_, ContainerSizeFeatureId>,
    required: &mut Requirements,
) {
    let MediaFeatureName::Standard(name) = name else {
        return;
    };
    match name {
        ContainerSizeFeatureId::Width => required.width = true,
        ContainerSizeFeatureId::Height => required.height = true,
        ContainerSizeFeatureId::InlineSize => required.inline = true,
        ContainerSizeFeatureId::BlockSize => required.block = true,
        ContainerSizeFeatureId::AspectRatio | ContainerSizeFeatureId::Orientation => {
            required.width = true;
            required.height = true;
        }
    }
}
fn requirements(
    condition: &ContainerCondition<'_>,
    required: &mut Requirements,
    limits: ComputationLimits,
    at: usize,
) -> Result<()> {
    conditions::depth(at, limits)?;
    match condition {
        ContainerCondition::Feature(feature) => require_feature(feature_name(feature), required),
        ContainerCondition::Not(inner) => requirements(inner, required, limits, at + 1)?,
        ContainerCondition::Operation {
            conditions: items, ..
        } => {
            for item in items {
                requirements(item, required, limits, at + 1)?;
            }
        }
        ContainerCondition::ScrollState(_) => required.scroll = true,
        ContainerCondition::Style(_) | ContainerCondition::Unknown(_) => {}
    }
    Ok(())
}
fn physical_axes(required: &Requirements, axis: InlineAxis) -> (bool, bool) {
    match axis {
        InlineAxis::Horizontal => (
            required.width || required.inline,
            required.height || required.block,
        ),
        InlineAxis::Vertical => (
            required.width || required.block,
            required.height || required.inline,
        ),
    }
}

fn eligible(ancestor: &ContainerAncestor<'_>, required: &Requirements) -> bool {
    if required.scroll && !ancestor.scroll_state {
        return false;
    }
    let (width, height) = physical_axes(required, ancestor.inline_axis);
    match ancestor.kind {
        SizeKind::Normal => !width && !height,
        SizeKind::Size => true,
        SizeKind::InlineSize => match ancestor.inline_axis {
            InlineAxis::Horizontal => !height,
            InlineAxis::Vertical => !width,
        },
    }
}

struct MeasuredAncestor<'a, 'facts> {
    facts: &'a ContainerAncestor<'facts>,
    measurement: PartialContentSize,
}
impl<'facts> std::ops::Deref for MeasuredAncestor<'_, 'facts> {
    type Target = ContainerAncestor<'facts>;
    fn deref(&self) -> &Self::Target {
        self.facts
    }
}
impl MeasuredAncestor<'_, '_> {
    fn validate(&self) -> Result<()> {
        if [self.measurement.width_px, self.measurement.height_px]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || value < 0.0)
        {
            return Err(ContainerError::InvalidMeasurement(self.id));
        }
        Ok(())
    }
    fn width(&self) -> Result<f64> {
        self.measurement
            .width_px
            .ok_or(ContainerError::RequiresMeasurement(self.id))
    }
    fn height(&self) -> Result<f64> {
        self.measurement
            .height_px
            .ok_or(ContainerError::RequiresMeasurement(self.id))
    }
}
fn measured<'a, 'facts>(
    facts: &'a ContainerAncestor<'facts>,
    measurements: Option<&ContainerMeasurements>,
) -> MeasuredAncestor<'a, 'facts> {
    let measurement = measurements
        .and_then(|values| values.get(&facts.id))
        .copied()
        .unwrap_or_else(|| {
            facts
                .size
                .map_or(PartialContentSize::default(), |size| PartialContentSize {
                    width_px: Some(size.width_px),
                    height_px: Some(size.height_px),
                })
        });
    MeasuredAncestor { facts, measurement }
}
fn axis(ancestor: &MeasuredAncestor<'_, '_>, horizontal: bool) -> Result<f64> {
    if horizontal {
        ancestor.width()
    } else {
        ancestor.height()
    }
}
fn pair(ancestor: &MeasuredAncestor<'_, '_>) -> Result<(f64, f64)> {
    Ok((ancestor.width()?, ancestor.height()?))
}
fn orientation(ancestor: &MeasuredAncestor<'_, '_>) -> Result<FeatureValue> {
    pair(ancestor).map(|(width, height)| {
        FeatureValue::Ident(
            if width > height {
                "landscape"
            } else {
                "portrait"
            }
            .to_string(),
        )
    })
}
fn value(
    name: &MediaFeatureName<'_, ContainerSizeFeatureId>,
    ancestor: &MeasuredAncestor<'_, '_>,
) -> Result<Option<FeatureValue>> {
    let MediaFeatureName::Standard(name) = name else {
        return Ok(None);
    };
    let value = match name {
        ContainerSizeFeatureId::Width => ancestor.width().map(FeatureValue::LengthPx),
        ContainerSizeFeatureId::Height => ancestor.height().map(FeatureValue::LengthPx),
        ContainerSizeFeatureId::InlineSize => {
            axis(ancestor, ancestor.inline_axis == InlineAxis::Horizontal)
                .map(FeatureValue::LengthPx)
        }
        ContainerSizeFeatureId::BlockSize => {
            axis(ancestor, ancestor.inline_axis == InlineAxis::Vertical).map(FeatureValue::LengthPx)
        }
        ContainerSizeFeatureId::AspectRatio => {
            pair(ancestor).map(|(width, height)| FeatureValue::Ratio(width, height))
        }
        ContainerSizeFeatureId::Orientation => orientation(ancestor),
    };
    value.map(Some)
}

fn name_text<'a>(name: &'a MediaFeatureName<'_, ContainerSizeFeatureId>) -> &'a str {
    match name {
        MediaFeatureName::Standard(name) => name.as_str(),
        MediaFeatureName::Custom(name) => name.0.as_ref(),
        MediaFeatureName::Unknown(name) => name.0.as_ref(),
    }
}
fn compare(
    name: &MediaFeatureName<'_, ContainerSizeFeatureId>,
    operator: MediaFeatureComparison,
    query: &MediaFeatureValue<'_>,
    ancestor: &MeasuredAncestor<'_, '_>,
    limits: ComputationLimits,
) -> Result<MediaTruth> {
    let Some(value) = value(name, ancestor)? else {
        return Ok(MediaTruth::Unknown);
    };
    let order = conditions::compare_value(
        name_text(name),
        &value,
        query,
        ancestor.length_context,
        limits,
    )?;
    Ok(MediaTruth::from(conditions::compare_order(operator, order)))
}
fn feature(
    feature: &ContainerSizeFeature<'_>,
    ancestor: &MeasuredAncestor<'_, '_>,
    limits: ComputationLimits,
) -> Result<MediaTruth> {
    match feature {
        QueryFeature::Boolean { name } => match value(name, ancestor)? {
            Some(value) => Ok(MediaTruth::from(conditions::boolean_value(
                name_text(name),
                &value,
            )?)),
            None => Ok(MediaTruth::Unknown),
        },
        QueryFeature::Plain { name, value } => {
            compare(name, MediaFeatureComparison::Equal, value, ancestor, limits)
        }
        QueryFeature::Range {
            name,
            operator,
            value,
        } => compare(name, *operator, value, ancestor, limits),
        QueryFeature::Interval {
            name,
            start,
            start_operator,
            end,
            end_operator,
        } => {
            let start = compare(
                name,
                conditions::opposite(*start_operator),
                start,
                ancestor,
                limits,
            )?;
            if start == MediaTruth::False {
                return Ok(start);
            }
            Ok(start.min(compare(name, *end_operator, end, ancestor, limits)?))
        }
    }
}

struct Evaluation {
    values: u8,
    unavailable: Option<ContainerError>,
}
fn unavailable(error: &ContainerError) -> bool {
    match error {
        ContainerError::RequiresMeasurement(_)
        | ContainerError::RequiresStyleQuery(_)
        | ContainerError::RequiresScrollState(_) => true,
        ContainerError::Condition(error) => conditions::missing(error),
        _ => false,
    }
}
impl Evaluation {
    fn known(value: MediaTruth) -> Self {
        Self {
            values: value.bit(),
            unavailable: None,
        }
    }
    fn defer(result: Result<MediaTruth>) -> Result<Self> {
        match result {
            Ok(value) => Ok(Self::known(value)),
            Err(error) if unavailable(&error) => Ok(Self {
                values: MEDIA_TRUTHS.iter().fold(0, |all, value| all | value.bit()),
                unavailable: Some(error),
            }),
            Err(error) => Err(error),
        }
    }
    fn negate(mut self) -> Self {
        self.values = MEDIA_TRUTHS
            .iter()
            .filter(|value| self.values & value.bit() != 0)
            .fold(0, |all, value| all | value.negate().bit());
        self
    }
    fn matches(self) -> Result<bool> {
        if self.values & MediaTruth::True.bit() == 0 {
            return Ok(false);
        }
        if self.values == MediaTruth::True.bit() {
            return Ok(true);
        }
        Err(self
            .unavailable
            .expect("uncertain container result retains its missing capability"))
    }
}
fn combine(
    any: bool,
    items: &[ContainerCondition<'_>],
    ancestor: &MeasuredAncestor<'_, '_>,
    limits: ComputationLimits,
    at: usize,
) -> Result<Evaluation> {
    let decisive = MediaTruth::from(any);
    let mut result = Evaluation::known(MediaTruth::from(!any));
    for item in items {
        let next = evaluate(item, ancestor, limits, at)?;
        result.values = conditions::merged_truth(any, result.values, next.values);
        if result.unavailable.is_none() {
            result.unavailable = next.unavailable;
        }
        if result.values == decisive.bit() {
            return Ok(Evaluation::known(decisive));
        }
    }
    Ok(result)
}
fn evaluate(
    condition: &ContainerCondition<'_>,
    ancestor: &MeasuredAncestor<'_, '_>,
    limits: ComputationLimits,
    at: usize,
) -> Result<Evaluation> {
    conditions::depth(at, limits)?;
    match condition {
        ContainerCondition::Feature(query) => Evaluation::defer(feature(query, ancestor, limits)),
        ContainerCondition::Not(inner) => Ok(evaluate(inner, ancestor, limits, at + 1)?.negate()),
        ContainerCondition::Operation {
            operator,
            conditions: items,
        } => combine(
            matches!(operator, Operator::Or),
            items,
            ancestor,
            limits,
            at + 1,
        ),
        ContainerCondition::Style(_) => {
            Evaluation::defer(Err(ContainerError::RequiresStyleQuery(ancestor.id)))
        }
        ContainerCondition::ScrollState(_) => {
            Evaluation::defer(Err(ContainerError::RequiresScrollState(ancestor.id)))
        }
        // The pinned parser creates Unknown only under error_recovery=true;
        // prelude() is the only AST constructor here and uses the false default.
        ContainerCondition::Unknown(_) => {
            unreachable!("container parsing does not recover unknown conditions")
        }
    }
}

/// Evaluate a canonical container prelude using the nearest eligible ancestor.
/// Measurements must come from the actual renderer's current content box. A cold
/// selected container never falls through to a farther measured ancestor.
///
/// # Errors
/// Preserves explicit caller limits, invalid facts and needed measurement/style/
/// scroll-state capabilities. Provider exceptions are propagated by the caller before
/// constructing these facts; this API neither calls nor catches a provider.
pub fn container_matches(
    source: &CssText,
    ancestors: &[ContainerAncestor<'_>],
    limits: ComputationLimits,
) -> Result<ContainerMatch> {
    container_matches_with_measurements(source, ancestors, None, limits)
}
/// Match with independently measured content-box axes. An explicit map record is
/// authoritative for its original ancestor ID; missing axes never fall back to
/// the legacy full box, a farther container, or a synthetic zero.
///
/// # Errors
/// Preserves required-axis measurement errors, invalid present metrics and all
/// original parser/computation/capability errors.
pub fn container_matches_with_measurements(
    source: &CssText,
    ancestors: &[ContainerAncestor<'_>],
    measurements: Option<&ContainerMeasurements>,
    limits: ComputationLimits,
) -> Result<ContainerMatch> {
    super::grammar::check_grammar_limits(source, limits)?;
    let source = CssSource::new(source.as_units());
    let mut input = ParserInput::new(source.scalar());
    let mut input = Parser::new(&mut input);
    let parsed = input
        .parse_entirely(prelude)
        .map_err(|error| grammar_error(&error))?;
    let mut required = Requirements::default();
    if let Some(condition) = &parsed.condition {
        requirements(condition, &mut required, limits, 0)?;
    }
    let name = parsed.name.as_ref().map(|name| name.0 .0.as_ref());
    let selected = ancestors.iter().find(|ancestor| {
        name.is_none_or(|name| ancestor.names.iter().any(|value| value == name))
            && eligible(ancestor, &required)
    });
    let Some(ancestor) = selected else {
        return Ok(ContainerMatch {
            matches: false,
            container: None,
        });
    };
    let ancestor = measured(ancestor, measurements);
    ancestor.validate()?;
    let matches = parsed.condition.as_ref().map_or(Ok(true), |condition| {
        evaluate(condition, &ancestor, limits, 0)?.matches()
    })?;
    Ok(ContainerMatch {
        matches,
        container: Some(ancestor.id),
    })
}

/// Decoded computed containment metadata; measurements remain a separate provider fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContainerContext {
    pub names: Vec<String>,
    pub kind: SizeKind,
    pub inline_axis: InlineAxis,
    pub scroll_state: bool,
}
fn parsed_type(
    source: &CssText,
    limits: ComputationLimits,
) -> Result<lightningcss::properties::contain::ContainerType> {
    check_component_limits(source, limits)?;
    let source = CssSource::new(source.as_units());
    let mut input = ParserInput::new(source.scalar());
    Parser::new(&mut input)
        .parse_entirely(lightningcss::properties::contain::ContainerType::parse)
        .map_err(|error| grammar_error(&error))
}
fn parsed_axis(source: &CssText, limits: ComputationLimits) -> Result<InlineAxis> {
    super::grammar::check_grammar_limits(source, limits)?;
    let source = CssSource::new(source.as_units());
    // This property has a single-identifier grammar shared with native CSSOM;
    // its legacy Option seam cannot request a host capability or recurse through
    // a backend declaration-value AST. The resource guard above owns limits.
    super::grammar::declaration_value("writing-mode", source.scalar())
        .ok_or(ContainerError::InvalidSyntax)?;
    let mut input = ParserInput::new(source.scalar());
    let value = Parser::new(&mut input)
        .parse_entirely(|input| {
            let value = input.expect_ident_cloned()?;
            Ok::<_, cssparser::ParseError<'_, ()>>(value.to_ascii_lowercase())
        })
        .map_err(|_| ContainerError::InvalidSyntax)?;
    match value.as_str() {
        "horizontal-tb" | "lr" | "lr-tb" | "rl" | "rl-tb" => Ok(InlineAxis::Horizontal),
        "vertical-rl" | "vertical-lr" | "sideways-rl" | "sideways-lr" | "tb" | "tb-rl" => {
            Ok(InlineAxis::Vertical)
        }
        _ => Err(ContainerError::InvalidSyntax),
    }
}
/// Interpret computed container metadata with the shared canonical CSS grammar.
/// CSS-wide keywords must already have been resolved by the computed-value stage.
///
/// # Errors
/// Preserves explicit resource limits and rejects invalid/unresolved computed facts.
pub fn parse_container_context(
    names: &CssText,
    kind: &CssText,
    writing_mode: &CssText,
    limits: ComputationLimits,
) -> Result<ContainerContext> {
    use lightningcss::properties::contain::ContainerType;
    let names = container_names(names, limits)?;
    let kind = parsed_type(kind, limits)?;
    let inline_axis = parsed_axis(writing_mode, limits)?;
    let scroll_state = kind == ContainerType::ScrollState;
    let kind = match kind {
        ContainerType::Normal | ContainerType::ScrollState => SizeKind::Normal,
        ContainerType::InlineSize => SizeKind::InlineSize,
        ContainerType::Size => SizeKind::Size,
    };
    Ok(ContainerContext {
        names,
        kind,
        inline_axis,
        scroll_state,
    })
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
        assert_eq!(
            grammar_error(&error),
            ContainerError::Computation(ComputedCssError::NestingLimit)
        );
    }
}

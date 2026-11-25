//! Rust-owned condition evaluation over canonical CSS grammar and explicit host facts.
use super::{
    grammar::{
        parse_media_condition_context, parse_supports_condition_context,
        supports_condition_declaration, ConditionError, Length, MediaCondition, MediaFeature,
        MediaFeatureComparison, MediaFeatureId, MediaFeatureName, MediaFeatureValue, MediaQuery,
        MediaType, Operator, Qualifier, QueryFeature, Resolution, SupportsCondition,
    },
    ComputationLimits, ComputedCssError, CssSource, CssText,
};
use crate::rtdom::css_shorthand::{computed_length, LengthContext, LengthError};
use lightningcss::{stylesheet::PrinterOptions, traits::ToCss};
use std::{cmp::Ordering, collections::BTreeMap};

/// Actual host feature measurements. They are not inferred from authored element styles.
#[derive(Clone, Debug, PartialEq)]
pub enum FeatureValue {
    Number(f64),
    LengthPx(f64),
    Ratio(f64, f64),
    ResolutionDppx(f64),
    Ident(String),
    Boolean(bool),
}

/// One explicit environment snapshot. Media-query em/rem bases use initial fonts;
/// viewport bases and feature values are host measurements, never authored widths.
pub struct ConditionEnvironment<'a> {
    pub media_type: &'a str,
    pub features: &'a BTreeMap<String, FeatureValue>,
    pub length_context: &'a LengthContext<'a>,
    pub selector_support: &'a BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionEvaluationError {
    InvalidSyntax,
    Computation(ComputedCssError),
    RequiresFeature(String),
    RequiresSelector(String),
    RequiresGrammar(String),
    InvalidFeature(String),
    Length(LengthError),
    UnsupportedValue(String),
}
impl From<ComputedCssError> for ConditionEvaluationError {
    fn from(error: ComputedCssError) -> Self {
        Self::Computation(error)
    }
}
impl From<LengthError> for ConditionEvaluationError {
    fn from(error: LengthError) -> Self {
        Self::Length(error)
    }
}

type Result<T> = std::result::Result<T, ConditionEvaluationError>;

fn grammar_error(error: ConditionError) -> ConditionEvaluationError {
    match error {
        ConditionError::InvalidSyntax => ConditionEvaluationError::InvalidSyntax,
        ConditionError::NestingLimit => ComputedCssError::NestingLimit.into(),
        ConditionError::Computation(error) => error.into(),
        ConditionError::RequiresGrammar(name) => ConditionEvaluationError::RequiresGrammar(name),
    }
}

pub(super) fn depth(depth: usize, limits: ComputationLimits) -> Result<()> {
    if depth >= limits.max_nesting {
        return Err(ComputedCssError::NestingLimit.into());
    }
    Ok(())
}

pub(super) fn missing(error: &ConditionEvaluationError) -> bool {
    matches!(
        error,
        ConditionEvaluationError::RequiresFeature(_)
            | ConditionEvaluationError::RequiresSelector(_)
            | ConditionEvaluationError::RequiresGrammar(_)
            | ConditionEvaluationError::UnsupportedValue(_)
            | ConditionEvaluationError::Length(
                LengthError::RequiresBasis(_) | LengthError::UnsupportedMath(_)
            )
    )
}

// A decisive result can discharge missing capabilities in another branch, even when
// they occurred earlier in source order. Real errors/resources are never swallowed.
fn combine<I, F>(any: bool, items: I, mut evaluate: F) -> Result<bool>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<bool>,
{
    let mut pending = None;
    for item in items {
        match evaluate(item) {
            Ok(value) if value == any => return Ok(value),
            Ok(_) => {}
            Err(error) if missing(&error) => {
                if pending.is_none() {
                    pending = Some(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
    pending.map_or(Ok(!any), Err)
}

// Media general-enclosed values obey MQ three-valued logic. Capability gaps
// represent possible results, so unknown AND a missing measurement can be a final
// non-match while NOT of that expression can still require the measurement.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum MediaTruth {
    False,
    Unknown,
    True,
}
impl From<bool> for MediaTruth {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}
impl MediaTruth {
    pub(super) fn negate(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::True => Self::False,
            Self::Unknown => Self::Unknown,
        }
    }
    pub(super) fn bit(self) -> u8 {
        1 << self as u8
    }
}
pub(super) const MEDIA_TRUTHS: [MediaTruth; 3] =
    [MediaTruth::False, MediaTruth::Unknown, MediaTruth::True];

struct MediaValue {
    values: u8,
    unavailable: Option<ConditionEvaluationError>,
}
impl MediaValue {
    fn known(value: MediaTruth) -> Self {
        Self {
            values: value.bit(),
            unavailable: None,
        }
    }
    fn defer(value: Result<MediaTruth>) -> Result<Self> {
        match value {
            Ok(value) => Ok(Self::known(value)),
            Err(error) if missing(&error) => Ok(Self {
                values: MediaTruth::False.bit() | MediaTruth::True.bit(),
                unavailable: Some(error),
            }),
            Err(error) => Err(error),
        }
    }
    fn negate(mut self) -> Self {
        self.values = MEDIA_TRUTHS
            .iter()
            .filter(|truth| self.values & truth.bit() != 0)
            .fold(0, |values, truth| values | truth.negate().bit());
        self
    }
    fn matches(self) -> Result<bool> {
        if self.values & MediaTruth::True.bit() == 0 {
            return Ok(false);
        }
        if self.values == MediaTruth::True.bit() {
            return Ok(true);
        }
        // Multiple possibilities arise only from a deferred capability error.
        Err(self
            .unavailable
            .expect("uncertain media result retains its missing capability"))
    }
}

pub(super) fn merged_truth(any: bool, left: u8, right: u8) -> u8 {
    let mut values = 0;
    for a in MEDIA_TRUTHS {
        if left & a.bit() == 0 {
            continue;
        }
        for b in MEDIA_TRUTHS {
            if right & b.bit() == 0 {
                continue;
            }
            values |= if any { a.max(b) } else { a.min(b) }.bit();
        }
    }
    values
}

fn combine_media<I, F>(any: bool, items: I, mut evaluate: F) -> Result<MediaValue>
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<MediaValue>,
{
    let decisive = MediaTruth::from(any);
    let mut result = MediaValue::known(MediaTruth::from(!any));
    for item in items {
        let next = evaluate(item)?;
        result.values = merged_truth(any, result.values, next.values);
        if result.unavailable.is_none() {
            result.unavailable = next.unavailable;
        }
        if result.values == decisive.bit() {
            return Ok(MediaValue::known(decisive));
        }
    }
    Ok(result)
}

/// Evaluate a context media list. Invalid individual query syntax is a non-match;
/// other comma-delimited queries still participate in the result.
///
/// # Errors
/// Returns resource/recursion and missing/invalid host capability errors. Use the
/// strict grammar seam when a caller needs per-list syntax rejection.
pub fn media_matches(
    source: &CssText,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
) -> Result<bool> {
    super::grammar::check_grammar_limits(source, limits)
        .map_err(|error| grammar_error(error.into()))?;
    let source = CssSource::new(source.as_units());
    let list = parse_media_condition_context(source.scalar(), limits).map_err(grammar_error)?;
    if list.media_queries.is_empty() {
        return Ok(true);
    }
    combine_media(true, &list.media_queries, |query| {
        media_query(query, environment, limits)
    })?
    .matches()
}

fn media_query(
    query: &MediaQuery<'_>,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
) -> Result<MediaValue> {
    let type_matches = match &query.media_type {
        MediaType::All => true,
        MediaType::Screen => environment.media_type.eq_ignore_ascii_case("screen"),
        MediaType::Print => environment.media_type.eq_ignore_ascii_case("print"),
        MediaType::Custom(_) => false,
    };
    let matches = if type_matches {
        query
            .condition
            .as_ref()
            .map_or(Ok(MediaValue::known(MediaTruth::True)), |condition| {
                media_condition(condition, environment, limits, 0)
            })?
    } else {
        MediaValue::known(MediaTruth::False)
    };
    Ok(if matches!(query.qualifier, Some(Qualifier::Not)) {
        matches.negate()
    } else {
        matches
    })
}

fn media_condition(
    condition: &MediaCondition<'_>,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
    at: usize,
) -> Result<MediaValue> {
    depth(at, limits)?;
    match condition {
        MediaCondition::Feature(feature) => {
            MediaValue::defer(media_feature(feature, environment, limits))
        }
        MediaCondition::Not(inner) => {
            Ok(media_condition(inner, environment, limits, at + 1)?.negate())
        }
        MediaCondition::Operation {
            operator,
            conditions,
        } => combine_media(matches!(operator, Operator::Or), conditions, |inner| {
            media_condition(inner, environment, limits, at + 1)
        }),
        // CSS general-enclosed syntax and unknown features do not match; this is
        // distinct from a known feature for which the host supplied no measurement.
        MediaCondition::Unknown(_) => Ok(MediaValue::known(MediaTruth::Unknown)),
    }
}

fn feature_name(name: &MediaFeatureName<'_, MediaFeatureId>) -> Result<Option<String>> {
    match name {
        MediaFeatureName::Standard(name) => Ok(Some(name.as_str().to_string())),
        MediaFeatureName::Unknown(_) => Ok(None),
        MediaFeatureName::Custom(name) => Err(ConditionEvaluationError::RequiresFeature(
            name.0.to_string(),
        )),
    }
}

fn fact<'a>(name: &str, environment: &'a ConditionEnvironment<'_>) -> Result<&'a FeatureValue> {
    environment
        .features
        .get(name)
        .ok_or_else(|| ConditionEvaluationError::RequiresFeature(name.to_string()))
}

fn media_feature(
    feature: &MediaFeature<'_>,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
) -> Result<MediaTruth> {
    match feature {
        QueryFeature::Boolean { name } => {
            let Some(name) = feature_name(name)? else {
                return Ok(MediaTruth::Unknown);
            };
            boolean_value(&name, fact(&name, environment)?).map(MediaTruth::from)
        }
        QueryFeature::Plain { name, value } => compare_feature(
            name,
            MediaFeatureComparison::Equal,
            value,
            environment,
            limits,
        ),
        QueryFeature::Range {
            name,
            operator,
            value,
        } => compare_feature(name, *operator, value, environment, limits),
        QueryFeature::Interval {
            name,
            start,
            start_operator,
            end,
            end_operator,
        } => {
            let start =
                compare_feature(name, opposite(*start_operator), start, environment, limits)?;
            if start == MediaTruth::False {
                return Ok(start);
            }
            let end = compare_feature(name, *end_operator, end, environment, limits)?;
            Ok(start.min(end))
        }
    }
}

pub(super) fn opposite(operator: MediaFeatureComparison) -> MediaFeatureComparison {
    match operator {
        MediaFeatureComparison::Equal => MediaFeatureComparison::Equal,
        MediaFeatureComparison::GreaterThan => MediaFeatureComparison::LessThan,
        MediaFeatureComparison::GreaterThanEqual => MediaFeatureComparison::LessThanEqual,
        MediaFeatureComparison::LessThan => MediaFeatureComparison::GreaterThan,
        MediaFeatureComparison::LessThanEqual => MediaFeatureComparison::GreaterThanEqual,
    }
}

fn compare_feature(
    name: &MediaFeatureName<'_, MediaFeatureId>,
    operator: MediaFeatureComparison,
    query: &MediaFeatureValue<'_>,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
) -> Result<MediaTruth> {
    let Some(name) = feature_name(name)? else {
        return Ok(MediaTruth::Unknown);
    };
    let value = fact(&name, environment)?;
    let order = compare_value(&name, value, query, environment.length_context, limits)?;
    Ok(MediaTruth::from(compare_order(operator, order)))
}

pub(super) fn compare_order(operator: MediaFeatureComparison, order: Ordering) -> bool {
    match operator {
        MediaFeatureComparison::Equal => order == Ordering::Equal,
        MediaFeatureComparison::GreaterThan => order == Ordering::Greater,
        MediaFeatureComparison::GreaterThanEqual => order != Ordering::Less,
        MediaFeatureComparison::LessThan => order == Ordering::Less,
        MediaFeatureComparison::LessThanEqual => order != Ordering::Greater,
    }
}

fn finite(name: &str, number: f64) -> Result<f64> {
    if !number.is_finite() {
        return Err(ConditionEvaluationError::InvalidFeature(name.to_string()));
    }
    Ok(number)
}

fn measured(name: &str, value: f64) -> Result<f64> {
    let value = finite(name, value)?;
    if value < 0.0 {
        return Err(ConditionEvaluationError::InvalidFeature(name.to_string()));
    }
    Ok(value)
}

fn ident_truth(name: &str, value: &str) -> Result<bool> {
    if value.is_empty() {
        return Err(ConditionEvaluationError::InvalidFeature(name.to_string()));
    }
    Ok(!["none", "no-preference"]
        .iter()
        .any(|zero| value.eq_ignore_ascii_case(zero)))
}

pub(super) fn boolean_value(name: &str, value: &FeatureValue) -> Result<bool> {
    match value {
        FeatureValue::Boolean(value) => Ok(*value),
        FeatureValue::Ident(value) => ident_truth(name, value),
        FeatureValue::Number(value)
        | FeatureValue::LengthPx(value)
        | FeatureValue::ResolutionDppx(value) => Ok(measured(name, *value)? != 0.0),
        FeatureValue::Ratio(numerator, denominator) => {
            Ok(host_ratio(name, *numerator, *denominator)? != 0.0)
        }
    }
}

fn host_ratio(name: &str, numerator: f64, denominator: f64) -> Result<f64> {
    finite(name, numerator)?;
    finite(name, denominator)?;
    if numerator < 0.0 || denominator < 0.0 {
        return Err(ConditionEvaluationError::InvalidFeature(name.to_string()));
    }
    Ok(if numerator == 0.0 {
        0.0
    } else if denominator == 0.0 {
        f64::INFINITY
    } else {
        numerator / denominator
    })
}

fn ratio_order(
    name: &str,
    numerator: f64,
    denominator: f64,
    query: &super::grammar::Ratio,
) -> Result<Ordering> {
    let host = host_ratio(name, numerator, denominator)?;
    let expected = query_ratio(query.0, query.1)?;
    // Actual zero-by-zero query containers compare equal to every ratio by cross
    // products. Query0/0 itself is canonical1/0, as Chrome159 serializes it.
    if numerator == 0.0 && denominator == 0.0 {
        return Ok(Ordering::Equal);
    }
    let (a, b) = if query.0 == 0.0 && query.1 == 0.0 {
        (1.0, 0.0)
    } else {
        (f64::from(query.0), f64::from(query.1))
    };
    let left = numerator * b;
    let right = a * denominator;
    let order = if left.is_finite() && right.is_finite() {
        left.partial_cmp(&right)
    } else {
        host.partial_cmp(&expected)
    };
    order.ok_or_else(|| ConditionEvaluationError::InvalidFeature(name.to_string()))
}

fn query_ratio(numerator: f32, denominator: f32) -> Result<f64> {
    if numerator < 0.0 || denominator < 0.0 {
        return Err(ConditionEvaluationError::InvalidSyntax);
    }
    // Chrome159 canonicalizes 0/0 to1/0 and compares a zero denominator as infinity.
    Ok(if denominator == 0.0 {
        f64::INFINITY
    } else {
        f64::from(numerator) / f64::from(denominator)
    })
}

fn resolution(value: &Resolution) -> f64 {
    match value {
        Resolution::Dpi(value) => f64::from(*value) / 96.0,
        Resolution::Dpcm(value) => f64::from(*value) * 2.54 / 96.0,
        Resolution::Dppx(value) => f64::from(*value),
    }
}

fn length(value: &Length, context: &LengthContext<'_>, limits: ComputationLimits) -> Result<f64> {
    // Parsed typed Length only emits CSS tokens into String; its default printer
    // neither writes to a fallible external sink nor resolves CSS module dependencies.
    let source = value
        .to_css_string(PrinterOptions::default())
        .expect("parsed CSS length serializes into String");
    computed_length(&source, context, limits)?
        .ok_or(ConditionEvaluationError::UnsupportedValue(source))
}

pub(super) fn compare_value(
    name: &str,
    host: &FeatureValue,
    query: &MediaFeatureValue<'_>,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
) -> Result<Ordering> {
    if let (FeatureValue::Ratio(numerator, denominator), MediaFeatureValue::Ratio(query)) =
        (host, query)
    {
        return ratio_order(name, *numerator, *denominator, query);
    }
    if let (FeatureValue::Ident(host), MediaFeatureValue::Ident(query)) = (host, query) {
        return Ok(if host.eq_ignore_ascii_case(query.0.as_ref()) {
            Ordering::Equal
        } else {
            Ordering::Less
        });
    }
    let (host, query) = numeric_values(name, host, query, context, limits)?;
    host.partial_cmp(&query)
        .ok_or_else(|| ConditionEvaluationError::InvalidFeature(name.to_string()))
}

fn numeric_values(
    name: &str,
    host: &FeatureValue,
    query: &MediaFeatureValue<'_>,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
) -> Result<(f64, f64)> {
    let values = match (host, query) {
        (FeatureValue::Number(host), MediaFeatureValue::Number(query)) => {
            (*host, f64::from(*query))
        }
        (FeatureValue::Number(host), MediaFeatureValue::Integer(query)) => {
            (*host, f64::from(*query))
        }
        (FeatureValue::Boolean(host), MediaFeatureValue::Boolean(query)) => {
            (f64::from(u8::from(*host)), f64::from(u8::from(*query)))
        }
        (FeatureValue::LengthPx(host), MediaFeatureValue::Length(query)) => {
            (*host, length(query, context, limits)?)
        }
        (FeatureValue::ResolutionDppx(host), MediaFeatureValue::Resolution(query)) => {
            (*host, resolution(query))
        }
        (_, MediaFeatureValue::Env(_)) => {
            return Err(ConditionEvaluationError::UnsupportedValue(
                "env() media value".to_string(),
            ))
        }
        _ => return Err(ConditionEvaluationError::InvalidFeature(name.to_string())),
    };
    Ok((measured(name, values.0)?, values.1))
}

/// Evaluate supports declaration/selector leaves using canonical grammar and explicit
/// selector support facts. A host-unclassified selector never silently becomes false.
///
/// # Errors
/// Returns resource/recursion, syntax or missing selector capability errors.
pub fn supports_matches(
    source: &CssText,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
) -> Result<bool> {
    super::grammar::check_grammar_limits(source, limits)
        .map_err(|error| grammar_error(error.into()))?;
    let source = CssSource::new(source.as_units());
    let condition =
        parse_supports_condition_context(source.scalar(), limits).map_err(grammar_error)?;
    supports_condition(&condition, environment, limits, 0)
}

fn supports_condition(
    condition: &SupportsCondition<'_>,
    environment: &ConditionEnvironment<'_>,
    limits: ComputationLimits,
    at: usize,
) -> Result<bool> {
    depth(at, limits)?;
    match condition {
        SupportsCondition::Not(inner) => {
            Ok(!supports_condition(inner, environment, limits, at + 1)?)
        }
        SupportsCondition::And(conditions) => combine(false, conditions, |inner| {
            supports_condition(inner, environment, limits, at + 1)
        }),
        SupportsCondition::Or(conditions) => combine(true, conditions, |inner| {
            supports_condition(inner, environment, limits, at + 1)
        }),
        SupportsCondition::Declaration { property_id, value } => {
            // PropertyId serialization preserves vendor prefixes; name() alone does not.
            let name = property_id
                .to_css_string(PrinterOptions::default())
                .expect("parsed property name serializes into String");
            supports_condition_declaration(&name, value, limits).map_err(grammar_error)
        }
        SupportsCondition::Selector(selector) => environment
            .selector_support
            .get(selector.as_ref())
            .copied()
            .ok_or_else(|| ConditionEvaluationError::RequiresSelector(selector.to_string())),
        SupportsCondition::Unknown(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_boolean_groups_keep_identities_and_resources_are_not_deferred() {
        assert_eq!(combine(true, std::iter::empty::<bool>(), Ok), Ok(false));
        assert_eq!(combine(false, std::iter::empty::<bool>(), Ok), Ok(true));
        let resource = Err(ConditionEvaluationError::Computation(
            ComputedCssError::ResourceLimit,
        ));
        assert_eq!(
            combine(true, [resource, Ok(true)], std::convert::identity),
            Err(ConditionEvaluationError::Computation(
                ComputedCssError::ResourceLimit
            ))
        );
        let missing = Err(ConditionEvaluationError::RequiresFeature("width".into()));
        assert_eq!(
            combine(true, [missing, Ok(true)], std::convert::identity),
            Ok(true)
        );
    }
}

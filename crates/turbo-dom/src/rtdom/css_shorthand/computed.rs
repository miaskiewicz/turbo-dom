//! Context-dependent length resolution over the existing component-value grammar.
use super::components::{
    canonical_terms, components, has_substitution, join, literal, math_value, visit, Comp, Kind,
};
use super::math::has_tree_counting;
use crate::rtdom::computed_css::{
    check_component_limits, has_variable_references, substitute, ComputationLimits,
    ComputedCssError, CssText, CustomProperties,
};
use crate::rtdom::css_values::css_number;

/// Real environment/provider measurements, expressed as CSS pixels per one unit.
/// For example `em` is the current computed font size, and `%` is one hundredth
/// of the property's actual percentage basis. Omitted measurements stay unknown.
pub struct LengthContext<'a> {
    pub unit_bases: &'a [(&'a str, f64)],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LengthError {
    RequiresBasis(String),
    InvalidBasis(String),
    UnsupportedMath(String),
    Computation(ComputedCssError),
}

impl From<ComputedCssError> for LengthError {
    fn from(error: ComputedCssError) -> Self {
        Self::Computation(error)
    }
}

impl LengthContext<'_> {
    fn basis(&self, unit: &str) -> Result<f64, LengthError> {
        let mut values = self
            .unit_bases
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(unit));
        let Some((_, value)) = values.next() else {
            return Err(LengthError::RequiresBasis(unit.to_owned()));
        };
        if !value.is_finite() || *value < 0.0 || values.next().is_some() {
            return Err(LengthError::InvalidBasis(unit.to_owned()));
        }
        Ok(*value)
    }
}

fn checked_text(text: String, limits: ComputationLimits) -> Result<String, LengthError> {
    if text.encode_utf16().count() > limits.max_output_units {
        return Err(ComputedCssError::ResourceLimit.into());
    }
    Ok(text)
}

/// Resolve an already-substituted ordinary `<length-percentage>` to a pixel number.
/// Raw shorthand/longhand spelling remains unchanged. This function retains no values
/// or measurements after returning; its caller owns cumulative retained-style limits.
///
/// # Errors
/// Missing or invalid real unit bases and unsupported math stay explicit. Canonical
/// nesting/output failures propagate, and any unresolved variable requires the caller's
/// actual cascade; it is never substituted against an invented empty custom cascade.
pub fn computed_length(
    value: &str,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
) -> Result<Option<f64>, LengthError> {
    let Some(token) = length_token(value, context, limits, PercentageMode::Resolve)? else {
        return Ok(None);
    };
    final_number(&token).map(Some)
}

/// Compute an inheritable length expression without evaluating its percentages.
/// Font/absolute/viewport units resolve in the declaring element's context. Percentages
/// and their math remain computed values until the actual used-value basis is supplied.
/// The `%` entry of `context` is never consulted by this operation.
///
/// # Errors
/// Uses the same resource, unresolved-cascade and real-metric errors as [`computed_length`].
pub fn computed_length_expression(
    value: &str,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
) -> Result<Option<String>, LengthError> {
    let Some(token) = length_token(value, context, limits, PercentageMode::Retain)? else {
        return Ok(None);
    };
    let text = match final_number(&token) {
        Ok(value) => format!("{}px", css_number(value as f32)),
        Err(LengthError::UnsupportedMath(_)) if contains_percentage(&token) => token.text,
        Err(error) => return Err(error),
    };
    checked_text(text, limits).map(Some)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PercentageMode {
    Resolve,
    Retain,
}

fn contains_percentage(token: &Comp) -> bool {
    let mut found = false;
    visit(std::slice::from_ref(token), &mut |token| {
        found |= matches!(token.kind, Kind::Percentage(_));
    });
    found
}

fn length_token(
    value: &str,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
    percentage: PercentageMode,
) -> Result<Option<Comp>, LengthError> {
    if !guard_ordinary(value, limits)? {
        return Ok(None);
    }
    let Some(tokens) = components(value) else {
        return Ok(None);
    };
    let [token] = tokens.as_slice() else {
        return Ok(None);
    };
    if token.length_percentage(true).is_none() {
        return Ok(None);
    }
    if has_tree_counting(&tokens) {
        return Err(LengthError::UnsupportedMath(token.text.clone()));
    }
    let text = resolve_token(token, context, limits, 0, percentage)?;
    // resolve_token emits one finite dimension, the original number/percentage,
    // or one balanced function with each argument replaced by one component.
    // Parsing that spelling also preserves the canonical dimension f32 clamp.
    let mut resolved = components(&text).expect("resolved length has canonical component syntax");
    assert_eq!(resolved.len(), 1, "resolved length is one component");
    Ok(resolved.pop())
}

fn guard_ordinary(value: &str, limits: ComputationLimits) -> Result<bool, LengthError> {
    let raw = CssText::from_units(value.encode_utf16().collect());
    check_component_limits(&raw, limits)?;
    if has_variable_references(&raw, limits)? {
        return Err(ComputedCssError::RequiresCascade.into());
    }
    // With no var() token, substitution copies every original source range;
    // its sole remaining job here is strict syntax and resource validation.
    if substitute(&raw, &CustomProperties::default(), limits)?.is_none() {
        return Ok(false);
    }
    // The existing shared shorthand scanner includes env()/attr()/if()/inherit().
    // This is an ordinary already-substituted value, not a new source of policy.
    if has_substitution(value) {
        return Err(ComputedCssError::RequiresCascade.into());
    }
    Ok(true)
}

fn final_number(token: &Comp) -> Result<f64, LengthError> {
    match &token.kind {
        Kind::Dimension { value, unit } if unit == "px" => Ok(f64::from(*value)),
        Kind::Number { value, .. } if *value == 0.0 => Ok(0.0),
        Kind::Function(_) => math_value(token)
            .filter(|(unit, value)| unit == "px" && value.is_finite())
            .map(|(_, value)| value)
            .ok_or_else(|| LengthError::UnsupportedMath(token.text.clone())),
        _ => Err(LengthError::UnsupportedMath(token.text.clone())),
    }
}

fn dimension(token: &Comp, context: &LengthContext<'_>) -> Result<f64, LengthError> {
    // Only dimension/percentage leaves reach this helper. One such literal has
    // exactly one term, and canonical absolute-unit conversion preserves that.
    let terms = literal(token).expect("dimension/percentage literal");
    let (unit, value) = canonical_terms(terms)
        .into_iter()
        .next()
        .expect("one canonical dimension/percentage term");
    if unit == "px" {
        return Ok(value);
    }
    Ok(value * context.basis(&unit)?)
}

fn resolve_token(
    token: &Comp,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
    depth: usize,
    percentage: PercentageMode,
) -> Result<String, LengthError> {
    if depth >= limits.max_nesting {
        return Err(ComputedCssError::NestingLimit.into());
    }
    if percentage == PercentageMode::Retain && matches!(token.kind, Kind::Percentage(_)) {
        return checked_text(token.text.clone(), limits);
    }
    if token.function_named(&["calc-size"]).is_some() {
        return Err(LengthError::UnsupportedMath(token.text.clone()));
    }
    if matches!(token.kind, Kind::Dimension { .. } | Kind::Percentage(_)) {
        let value = dimension(token, context)?;
        if !value.is_finite() {
            return Err(LengthError::UnsupportedMath(token.text.clone()));
        }
        return checked_text(format!("{value}px"), limits);
    }
    if matches!(token.kind, Kind::Function(_)) {
        return resolve_function(token, context, limits, depth, percentage);
    }
    checked_text(token.text.clone(), limits)
}

fn resolve_function(
    token: &Comp,
    context: &LengthContext<'_>,
    limits: ComputationLimits,
    depth: usize,
    percentage: PercentageMode,
) -> Result<String, LengthError> {
    // The shared parser produced the spelling and balanced delimiters. It can have
    // simplified `min(...)` to `calc(...)`, so use that canonical spelling's opener.
    let open = token.text.find('(').expect("canonical function opener");
    let mut tokens = components(&token.text[open + 1..token.text.len() - 1])
        .expect("canonical function arguments were already parsed");
    // resolve_token admitted depth < max_nesting <= usize::MAX.
    let nested = depth + 1;
    for token in &mut tokens {
        token.text = resolve_token(token, context, limits, nested, percentage)?;
    }
    checked_text(
        format!("{}({})", &token.text[..open], join(&tokens)),
        limits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> ComputationLimits {
        ComputationLimits {
            max_nesting: 16,
            max_output_units: 4096,
            max_total_units: 0,
        }
    }
    fn resolve(value: &str, bases: &[(&str, f64)]) -> Result<Option<f64>, LengthError> {
        computed_length(value, &LengthContext { unit_bases: bases }, limits())
    }
    fn expression(value: &str, bases: &[(&str, f64)]) -> Result<Option<String>, LengthError> {
        computed_length_expression(value, &LengthContext { unit_bases: bases }, limits())
    }

    #[test]
    fn inert_tokens_and_original_whitespace_do_not_require_a_variable_cascade() {
        for input in ["1px/**/", " \r\n1px/*var(--x)*/\t", "calc(1px + 2px)"] {
            assert!(resolve(input, &[]).unwrap().is_some());
        }
        for input in ["1px\0", "1/**/px", "\"var(--x)\""] {
            assert_eq!(resolve(input, &[]), Ok(None));
        }
        assert_eq!(
            resolve("\\76 ar(--x)", &[]),
            Err(LengthError::Computation(ComputedCssError::RequiresCascade))
        );
    }

    #[test]
    fn finite_unit_bases_keep_the_existing_canonical_dimension_clamp() {
        for basis in [1e300, f64::MAX] {
            assert_eq!(
                resolve("1em", &[("em", basis)]),
                Ok(Some(f64::from(f32::MAX)))
            );
        }
        assert_eq!(resolve("1em", &[("em", 100.0)]), Ok(Some(100.0)));
    }

    #[test]
    fn inherited_computed_percentages_and_font_lengths_have_distinct_used_values() {
        let percent = expression("50%", &[("%", f64::NAN)]).unwrap().unwrap();
        assert_eq!(percent, "50%");
        assert_eq!(resolve(&percent, &[("%", 8.0)]), Ok(Some(400.0)));
        assert_eq!(resolve(&percent, &[("%", 4.0)]), Ok(Some(200.0)));
        let pixels = expression("20em", &[("em", 10.0)]).unwrap().unwrap();
        assert_eq!(pixels, "200px");
        assert_eq!(resolve(&pixels, &[("em", 20.0)]), Ok(Some(200.0)));
        assert_eq!(resolve(&pixels, &[("em", f64::NAN)]), Ok(Some(200.0)));
        // Unitless line-height is a separate computed number, not a length. Its
        // caller retains that number for the inheriting element's own font size.
        assert_eq!(expression("1.5", &[("em", 10.0)]), Ok(None));
    }

    #[test]
    fn computed_percentage_math_roundtrips_through_the_shared_used_value_path() {
        let declaring = [("em", 10.0), ("%", f64::NAN)];
        for (value, canonical, used) in [
            ("calc(1em + 50%)", "calc(50% + 10px)", 210.0),
            ("min(1em, 50%)", "min(10px, 50%)", 10.0),
            ("max(2em, 25%)", "max(20px, 25%)", 100.0),
            ("clamp(1em, 50%, 4em)", "clamp(10px, 50%, 40px)", 40.0),
        ] {
            let computed = expression(value, &declaring).unwrap().unwrap();
            assert_eq!(computed, canonical);
            assert_eq!(resolve(&computed, &[("%", 4.0)]), Ok(Some(used)));
        }
        let inherited = expression("calc(1em + 50%)", &declaring).unwrap().unwrap();
        assert_eq!(
            resolve(&inherited, &[("em", 20.0), ("%", 2.0)]),
            Ok(Some(110.0))
        );
    }

    #[test]
    fn computed_expression_keeps_original_errors_and_explicit_output_limits() {
        assert_eq!(
            expression("1ex", &[]),
            Err(LengthError::RequiresBasis("ex".into()))
        );
        assert_eq!(
            expression("var(--missing, 1px)", &[]),
            Err(LengthError::Computation(ComputedCssError::RequiresCascade))
        );
        assert!(matches!(
            expression("calc(50% + sibling-index() * 1px)", &[]),
            Err(LengthError::UnsupportedMath(_))
        ));
        let context = LengthContext {
            unit_bases: &[("em", 10.0)],
        };
        assert_eq!(
            computed_length_expression(
                "20em",
                &context,
                ComputationLimits {
                    max_output_units: 4,
                    ..limits()
                }
            ),
            Err(LengthError::Computation(ComputedCssError::ResourceLimit))
        );
        assert_eq!(
            computed_length_expression(
                "20em",
                &context,
                ComputationLimits {
                    max_output_units: 5,
                    ..limits()
                }
            ),
            Ok(Some("200px".into()))
        );
        assert_eq!(expression("calc(1px +)", &[]), Ok(None));
        assert_eq!(expression("0", &[]), Ok(Some("0px".into())));
    }

    #[test]
    fn shared_math_resolves_relative_absolute_and_viewport_bases() {
        let bases = [("em", 20.0), ("rem", 10.0), ("vw", 12.8), ("vh", 7.2)];
        for (value, expected) in [
            ("2em", 40.0),
            ("2rem", 20.0),
            ("1in", 96.0),
            ("2.54cm", 96.0),
            ("12pt", 16.0),
            ("calc(1em + 2px)", 22.0),
            ("min(2em, 30px)", 30.0),
            ("max(1rem, 12px)", 12.0),
            ("clamp(1rem, 2em, 30px)", 30.0),
            ("calc((1em + 2px) * 2)", 44.0),
            ("1vw", 12.8),
            ("1vh", 7.2),
        ] {
            let actual = resolve(value, &bases).unwrap().unwrap();
            assert!(
                (actual - expected).abs() < 0.00001,
                "{value}: {actual} vs {expected}"
            );
        }
        assert_eq!(resolve("0", &[]), Ok(Some(0.0)));
        assert_eq!(resolve(" \t1px\r\n", &[]), Ok(Some(1.0)));
    }
    #[test]
    fn real_metrics_are_required_and_unused_bad_metrics_are_not_consulted() {
        assert_eq!(
            resolve("2ex", &[]),
            Err(LengthError::RequiresBasis("ex".into()))
        );
        assert_eq!(resolve("2ex", &[("ex", 9.0)]), Ok(Some(18.0)));
        assert_eq!(
            resolve("50%", &[]),
            Err(LengthError::RequiresBasis("%".into()))
        );
        assert_eq!(resolve("50%", &[("%", 2.0)]), Ok(Some(100.0)));
        assert_eq!(
            resolve("0%", &[]),
            Err(LengthError::RequiresBasis("%".into()))
        );
        assert_eq!(resolve("1px", &[("em", f64::NAN)]), Ok(Some(1.0)));
        for invalid in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(
                resolve("1em", &[("em", invalid)]),
                Err(LengthError::InvalidBasis("em".into()))
            );
        }
        assert_eq!(resolve("1em", &[("em", 0.0)]), Ok(Some(0.0)));
        assert!(matches!(
            resolve("2em", &[("em", f64::MAX)]),
            Err(LengthError::UnsupportedMath(_))
        ));
        assert_eq!(
            resolve("1em", &[("em", 20.0), ("EM", 20.0)]),
            Err(LengthError::InvalidBasis("em".into()))
        );
    }
    #[test]
    fn resources_propagate_and_missing_custom_cascade_never_uses_empty_map_fallbacks() {
        let context = LengthContext {
            unit_bases: &[("em", 20.0)],
        };
        let run = |value, limits| computed_length(value, &context, limits);
        assert_eq!(
            run(
                "1px",
                ComputationLimits {
                    max_nesting: 0,
                    ..limits()
                }
            ),
            Err(LengthError::Computation(ComputedCssError::NestingLimit))
        );
        assert_eq!(
            run(
                "1px",
                ComputationLimits {
                    max_nesting: 1,
                    max_output_units: 3,
                    ..limits()
                }
            ),
            Ok(Some(1.0))
        );
        assert_eq!(
            run(
                "1px",
                ComputationLimits {
                    max_output_units: 2,
                    ..limits()
                }
            ),
            Err(LengthError::Computation(ComputedCssError::ResourceLimit))
        );
        assert_eq!(
            run(
                "1em",
                ComputationLimits {
                    max_output_units: 3,
                    ..limits()
                }
            ),
            Err(LengthError::Computation(ComputedCssError::ResourceLimit))
        );
        assert_eq!(
            run(
                "1em",
                ComputationLimits {
                    max_output_units: 4,
                    ..limits()
                }
            ),
            Ok(Some(20.0))
        );
        assert_eq!(
            run(
                "calc(1em + 2px)",
                ComputationLimits {
                    max_nesting: 1,
                    ..limits()
                }
            ),
            Err(LengthError::Computation(ComputedCssError::NestingLimit))
        );
        assert_eq!(
            run(
                "calc(1em + 2px)",
                ComputationLimits {
                    max_nesting: 2,
                    ..limits()
                }
            ),
            Ok(Some(22.0))
        );
        for value in [
            "var(--missing)",
            "var(--missing, 1px)",
            "calc(var(--missing,1px) + 2px)",
            "env(safe-area-inset-top)",
            "attr(data-width px)",
        ] {
            assert_eq!(
                resolve(value, &[]),
                Err(LengthError::Computation(ComputedCssError::RequiresCascade)),
                "{value}"
            );
        }
    }
    #[test]
    fn malformed_or_non_length_values_are_not_computed_numbers() {
        for value in [
            "",
            "2",
            "red",
            "1furlong",
            "calc(1px +)",
            "calc(1px))",
            "calc(1px + 2s)",
            "min()",
            "clamp(1px,2px)",
            "url(\"1em\")",
            "\"var(--x)\"",
        ] {
            assert_eq!(resolve(value, &[]), Ok(None), "{value}");
        }
        assert!(matches!(
            resolve("calc(sibling-index() * 1px)", &[]),
            Err(LengthError::UnsupportedMath(_))
        ));
    }
}

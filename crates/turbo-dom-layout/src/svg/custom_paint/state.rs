//! Property policy over canonical cascade winners and the shared token kernel.
use std::collections::BTreeMap;
use std::sync::Arc;
use turbo_dom::rtdom::computed_css::{
    self, ComputationLimits, ComputedCssError, CssText, CustomProperties,
};

use super::super::SvgResourceError;

type Paint = BTreeMap<String, String>;
const PROPERTIES: &[(&str, &str, bool)] = &[
    ("color", "black", true),
    ("fill", "black", true),
    ("stroke", "none", true),
    ("stop-color", "black", false),
    ("flood-color", "black", false),
    ("lighting-color", "white", false),
];

pub(super) struct Policy {
    pub limits: ComputationLimits,
    retained_units: usize,
}

impl Policy {
    pub fn new(source: &str) -> Self {
        let units = source.encode_utf16().count();
        Self {
            limits: ComputationLimits {
                max_nesting: 16,
                max_output_units: units,
                max_total_units: units,
            },
            retained_units: 0,
        }
    }

    fn variables(
        &mut self,
        parent: &CustomProperties,
        own: &BTreeMap<String, CssText>,
    ) -> Result<Arc<CustomProperties>, SvgResourceError> {
        let remaining = self
            .limits
            .max_total_units
            .checked_sub(self.retained_units)
            .ok_or(SvgResourceError::CustomPropertyBytes)?;
        let limits = ComputationLimits {
            max_total_units: remaining,
            ..self.limits
        };
        let computed = computed_css::compute_custom_properties(parent.as_map(), own, limits)
            .map_err(kernel_error)?;
        let units = computed
            .as_map()
            .values()
            .try_fold(0usize, |total, value| {
                total.checked_add(value.as_units().len())
            })
            .ok_or(SvgResourceError::CustomPropertyBytes)?;
        self.retained_units = self
            .retained_units
            .checked_add(units)
            .ok_or(SvgResourceError::CustomPropertyBytes)?;
        Ok(Arc::new(computed))
    }
}

pub(super) fn text(value: &str) -> CssText {
    CssText::from_units(value.encode_utf16().collect())
}

pub(super) fn has_reference(value: &str, policy: &Policy) -> Result<bool, SvgResourceError> {
    // Inspection is structural, not variable computation: retain the existing
    // SVG depth bound without rejecting ordinary non-variable CSS functions.
    let limits = ComputationLimits {
        max_nesting: super::super::MAX_DEPTH,
        ..policy.limits
    };
    computed_css::has_variable_references(&text(value), limits).map_err(kernel_error)
}

pub(super) fn kernel_error(error: ComputedCssError) -> SvgResourceError {
    match error {
        ComputedCssError::NestingLimit => SvgResourceError::CustomPropertyNesting,
        ComputedCssError::ResourceLimit => SvgResourceError::CustomPropertyBytes,
        ComputedCssError::RequiresCascade | ComputedCssError::InvalidSyntax => {
            SvgResourceError::UnsupportedCustomProperties
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    variables: Arc<CustomProperties>,
    paint: BTreeMap<String, Arc<str>>,
}

impl State {
    pub fn resolve(
        parent: &Self,
        winners: &Paint,
        policy: &mut Policy,
    ) -> Result<(Self, Paint), SvgResourceError> {
        let own: BTreeMap<_, _> = winners
            .iter()
            .filter(|(name, _)| name.starts_with("--"))
            .map(|(name, value)| (name.clone(), text(value)))
            .collect();
        let variables = if own.is_empty() {
            parent.variables.clone()
        } else {
            policy.variables(&parent.variables, &own)?
        };
        let mut paint = BTreeMap::new();
        let mut edits = Paint::new();
        for &(name, initial, inherited) in PROPERTIES {
            let Some(own) = winners.get(name) else {
                let value = if inherited {
                    parent.paint.get(name).cloned()
                } else {
                    None
                };
                paint.insert(name.to_owned(), value.unwrap_or_else(|| Arc::from(initial)));
                continue;
            };
            let fallback = if inherited {
                parent
                    .paint
                    .get(name)
                    .map_or(initial, |value| value.as_ref())
            } else {
                initial
            };
            let value = property(name, own, fallback, parent, &variables, policy)?;
            certify_color(name, &value)?;
            edits.insert(name.to_owned(), value.clone());
            paint.insert(name.to_owned(), Arc::from(value));
        }
        reject_other_variable_properties(winners, policy)?;
        Ok((Self { variables, paint }, edits))
    }
}

fn property(
    name: &str,
    own: &str,
    fallback: &str,
    parent: &State,
    variables: &CustomProperties,
    policy: &Policy,
) -> Result<String, SvgResourceError> {
    if !has_reference(own, policy)? {
        return wide_value(name, own, fallback, parent);
    }
    let computed =
        computed_css::substitute(&text(own), variables, policy.limits).map_err(kernel_error)?;
    let Some(computed) = computed else {
        return Ok(fallback.to_owned());
    };
    let value = String::from_utf16(computed.as_units())
        .map_err(|_| SvgResourceError::UnsupportedCustomProperties)?;
    if value.trim().is_empty() {
        return Ok(fallback.to_owned());
    }
    let value = wide_value(name, &value, fallback, parent)?;
    Ok(value)
}

fn wide_value(
    name: &str,
    value: &str,
    fallback: &str,
    parent: &State,
) -> Result<String, SvgResourceError> {
    let (_, initial, _) = PROPERTIES
        .iter()
        .find(|(property, _, _)| *property == name)
        .expect("known SVG paint property");
    let result = match value.trim().to_ascii_lowercase().as_str() {
        "inherit" => parent
            .paint
            .get(name)
            .map_or(*initial, |value| value.as_ref()),
        "initial" => initial,
        "unset" => fallback,
        "revert" | "revert-layer" => return Err(SvgResourceError::UnsupportedCustomProperties),
        "currentcolor" if name == "color" => parent
            .paint
            .get(name)
            .map_or(*initial, |value| value.as_ref()),
        _ => value.trim(),
    };
    Ok(result.to_owned())
}

fn certify_color(name: &str, value: &str) -> Result<(), SvgResourceError> {
    if value.eq_ignore_ascii_case("currentcolor")
        || ((name == "fill" || name == "stroke") && value.eq_ignore_ascii_case("none"))
    {
        return Ok(());
    }
    turbo_dom::rtdom::color::parse_color(value)
        .map(|_| ())
        .ok_or(SvgResourceError::UnsupportedCustomProperties)
}

pub(super) fn is_paint(name: &str) -> bool {
    PROPERTIES.iter().any(|(property, _, _)| name == *property)
}

fn reject_other_variable_properties(
    winners: &Paint,
    policy: &Policy,
) -> Result<(), SvgResourceError> {
    for (name, value) in winners {
        if name.starts_with("--") || PROPERTIES.iter().any(|(property, _, _)| name == property) {
            continue;
        }
        if forbidden_property(name, value) || nonpaint_reference(value, policy)? {
            return Err(SvgResourceError::UnsupportedCustomProperties);
        }
    }
    Ok(())
}

fn forbidden_property(name: &str, value: &str) -> bool {
    matches!(name, "all" | "color-scheme")
        || name.starts_with("animation")
        || name.starts_with("transition")
        || matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "revert" | "revert-layer"
        )
}

fn nonpaint_reference(value: &str, policy: &Policy) -> Result<bool, SvgResourceError> {
    let environment = computed_css::has_function_reference(&text(value), "env", policy.limits)
        .map_err(kernel_error)?;
    Ok(environment || has_reference(value, policy)?)
}

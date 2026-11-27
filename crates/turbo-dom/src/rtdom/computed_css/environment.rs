//! Explicit viewport facts shared by media evaluation and computed values.
//! This builder does not infer device, logical-axis, font-metric or selector facts.
use super::{
    conditions::FeatureValue, font_values::finalize_font_size_px,
    preferences::InitialPreferenceValues, ComputationLimits, ComputedCssError,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportAxis {
    Known(f64),
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportFacts {
    pub width: ViewportAxis,
    pub height: ViewportAxis,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedEnvironmentFacts {
    pub features: BTreeMap<String, FeatureValue>,
    /// Media em/rem use the initial preference, independent of the document font.
    pub media_unit_bases: Vec<(String, f64)>,
    /// Target em/rem remain owned by `computed_values`, not this viewport builder.
    pub value_unit_bases: Vec<(String, f64)>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentError {
    InvalidViewportAxis(&'static str),
    InvalidInitialFontBasis,
    Computation(ComputedCssError),
}
struct Budget {
    remaining: usize,
    per_text: usize,
}
impl Budget {
    fn text(&mut self, value: &str) -> Result<String, EnvironmentError> {
        let units = value.encode_utf16().count();
        if units > self.per_text {
            return Err(EnvironmentError::Computation(
                ComputedCssError::ResourceLimit,
            ));
        }
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or(EnvironmentError::Computation(
                ComputedCssError::ResourceLimit,
            ))?;
        Ok(value.to_owned())
    }
}
fn axis(value: ViewportAxis, name: &'static str) -> Result<Option<f64>, EnvironmentError> {
    match value {
        ViewportAxis::Unknown => Ok(None),
        ViewportAxis::Known(value) if value.is_finite() && value >= 0.0 => Ok(Some(value)),
        ViewportAxis::Known(_) => Err(EnvironmentError::InvalidViewportAxis(name)),
    }
}
fn initial_font(initial: &InitialPreferenceValues) -> Result<f64, EnvironmentError> {
    let basis = initial.computed_font_size_px;
    let finalized =
        finalize_font_size_px(basis).map_err(|_| EnvironmentError::InvalidInitialFontBasis)?;
    if finalized != basis || f64::from(basis as f32) != basis {
        return Err(EnvironmentError::InvalidInitialFontBasis);
    }
    Ok(basis)
}
fn single_axis(
    facts: &mut ComputedEnvironmentFacts,
    budget: &mut Budget,
    name: &'static str,
    unit: &'static str,
    value: Option<f64>,
) -> Result<(), EnvironmentError> {
    if let Some(value) = value {
        facts
            .features
            .insert(budget.text(name)?, FeatureValue::LengthPx(value));
        facts
            .value_unit_bases
            .push((budget.text(unit)?, value / 100.0));
    }
    Ok(())
}
fn paired_axes(
    facts: &mut ComputedEnvironmentFacts,
    budget: &mut Budget,
    width: f64,
    height: f64,
) -> Result<(), EnvironmentError> {
    let orientation = if width > height {
        "landscape"
    } else {
        "portrait"
    };
    let orientation = FeatureValue::Ident(budget.text(orientation)?);
    facts
        .features
        .insert(budget.text("orientation")?, orientation);
    facts.features.insert(
        budget.text("aspect-ratio")?,
        FeatureValue::Ratio(width, height),
    );
    facts
        .value_unit_bases
        .push((budget.text("vmin")?, width.min(height) / 100.0));
    facts
        .value_unit_bases
        .push((budget.text("vmax")?, width.max(height) / 100.0));
    Ok(())
}
fn if_known_pair(
    facts: &mut ComputedEnvironmentFacts,
    budget: &mut Budget,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), EnvironmentError> {
    if let (Some(width), Some(height)) = (width, height) {
        paired_axes(facts, budget, width, height)?;
    }
    Ok(())
}
fn font_facts(
    font: f64,
    budget: &mut Budget,
) -> Result<ComputedEnvironmentFacts, EnvironmentError> {
    Ok(ComputedEnvironmentFacts {
        features: BTreeMap::new(),
        media_unit_bases: vec![(budget.text("em")?, font), (budget.text("rem")?, font)],
        value_unit_bases: Vec::new(),
    })
}
fn copy_viewport_bases(
    facts: &mut ComputedEnvironmentFacts,
    budget: &mut Budget,
) -> Result<(), EnvironmentError> {
    for (name, value) in &facts.value_unit_bases {
        facts.media_unit_bases.push((budget.text(name)?, *value));
    }
    Ok(())
}
/// Build only facts actually supplied by this viewport and initial-font context.
/// Missing axes remain absent so existing media and length evaluators can report
/// the precise required feature/basis, or resolve a decisive conditional branch.
/// No DOM nodes or measurements survive this owned result.
/// # Errors
/// Invalid host facts and noncanonical initial font bases are rejected. Actual
/// retained key/ident/unit strings are charged before every owned copy.
pub fn build_environment_facts(
    viewport: ViewportFacts,
    initial: &InitialPreferenceValues,
    limits: ComputationLimits,
) -> Result<ComputedEnvironmentFacts, EnvironmentError> {
    let font = initial_font(initial)?;
    let width = axis(viewport.width, "width")?;
    let height = axis(viewport.height, "height")?;
    let mut budget = Budget {
        remaining: limits.max_total_units,
        per_text: limits.max_output_units,
    };
    let mut facts = font_facts(font, &mut budget)?;
    single_axis(&mut facts, &mut budget, "width", "vw", width)?;
    single_axis(&mut facts, &mut budget, "height", "vh", height)?;
    if_known_pair(&mut facts, &mut budget, width, height)?;
    copy_viewport_bases(&mut facts, &mut budget)?;
    Ok(facts)
}

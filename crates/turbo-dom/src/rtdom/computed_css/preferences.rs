//! Bounded projection of host preference data into an explicit computed context.
//! This is not CSS declaration admission or a compatibility-mode selector.
use super::{ComputationLimits, ComputedCssError, CssText};
use crate::rtdom::computed_preferences::{
    computed_font_preferences, default_initial_color, ComputedFontPreferences, FontPreferenceError,
};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub struct InitialPreferenceValues {
    pub font_family: CssText,
    pub font_size_px: f64,
    pub computed_font_size: CssText,
    pub font_size_input: CssText,
    pub computed_font_size_px: f64,
    pub color: CssText,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferenceBuildError {
    Font(FontPreferenceError),
    Computation(ComputedCssError),
}
fn retained_text(
    value: &str,
    limits: ComputationLimits,
    remaining: &mut usize,
) -> Result<CssText, ComputedCssError> {
    let count = value.encode_utf16().count();
    if count > limits.max_output_units {
        return Err(ComputedCssError::ResourceLimit);
    }
    *remaining = remaining
        .checked_sub(count)
        .ok_or(ComputedCssError::ResourceLimit)?;
    Ok(CssText::from_units(value.encode_utf16().collect()))
}
/// Preserve raw host preferences, charging each retained UTF-16 field before copy.
/// `max_nesting` is not consulted: this projects data, never parses CSS tokens.
/// # Errors
/// Invalid raw preferences and original per-field/cumulative resource limits remain typed.
pub fn initial_preference_values(
    fonts: &ComputedFontPreferences<'_>,
    limits: ComputationLimits,
) -> Result<InitialPreferenceValues, PreferenceBuildError> {
    computed_font_preferences(Cow::Borrowed(fonts.family.as_ref()), fonts.size_px)
        .map_err(PreferenceBuildError::Font)?;
    // The raw validator above proved finite positive input, exactly the
    // precondition of the canonical CSS font-size finalizer.
    let finalized = super::font_values::finalize_font_size_px(fonts.size_px)
        .expect("validated finite positive raw font preference");
    // Canonical length components store absolute dimensions as f32. Normalize
    // once to that same grammar basis, including subnormal underflow, rather
    // than parsing the deliberately rounded public CSSOM serialization.
    let basis = finalized as f32;
    let computed_font_size_px = f64::from(basis);
    let mut remaining = limits.max_total_units;
    let font_family = retained_text(fonts.family.as_ref(), limits, &mut remaining)
        .map_err(PreferenceBuildError::Computation)?;
    let color = retained_text(default_initial_color(), limits, &mut remaining)
        .map_err(PreferenceBuildError::Computation)?;
    let computed_font_size = retained_text(
        &format!(
            "{}px",
            crate::rtdom::css_values::css_number(computed_font_size_px as f32)
        ),
        limits,
        &mut remaining,
    )
    .map_err(PreferenceBuildError::Computation)?;
    // Rust's shortest round-trip spelling preserves the typed component basis;
    // this is internal computation input, never public CSSOM presentation.
    let font_size_input = retained_text(&format!("{basis}px"), limits, &mut remaining)
        .map_err(PreferenceBuildError::Computation)?;
    Ok(InitialPreferenceValues {
        font_family,
        font_size_px: fonts.size_px,
        computed_font_size,
        font_size_input,
        computed_font_size_px,
        color,
    })
}

/// Renderer-only initial preferences missing from a sparse computed map.
/// Public CSSOM provenance is deliberately not changed. The renderer must see
/// the same initial font/color context used by explicit `initial` declarations.
/// # Errors
/// Original per-value/cumulative resource limits propagate before output copies.
pub fn renderer_initial_preferences(
    values: &std::collections::BTreeMap<String, CssText>,
    initial: &super::properties::InitialContext<'_>,
    limits: ComputationLimits,
) -> Result<Vec<(&'static str, CssText)>, ComputedCssError> {
    let mut remaining = limits.max_total_units;
    let mut result = Vec::new();
    for (name, value) in [
        ("font-family", initial.font_family),
        ("color", initial.color),
    ] {
        if !values.contains_key(name) {
            remaining = remaining
                .checked_sub(name.len())
                .ok_or(ComputedCssError::ResourceLimit)?;
            let value = retained_text(value, limits, &mut remaining)?;
            result.push((name, value));
        }
    }
    Ok(result)
}

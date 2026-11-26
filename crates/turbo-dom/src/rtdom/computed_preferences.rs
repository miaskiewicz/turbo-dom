//! Host font preferences shared by native bindings and explicit CSS contexts.
//! Values are data, not declarations: no CSS normalization or size cap is applied.
use super::computed_preference_seeds;
use std::borrow::Cow;

/// The native binding's existing platform distinction, not a compatibility mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontDefaultsPlatform {
    MacOs,
    /// Existing conventional non-mac preference; not an all-platform oracle claim.
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComputedFontPreferences<'a> {
    pub family: Cow<'a, str>,
    pub size_px: f64,
}
impl ComputedFontPreferences<'_> {
    /// Retain a validated host override independently of its original input.
    #[must_use]
    pub fn into_owned(self) -> ComputedFontPreferences<'static> {
        ComputedFontPreferences {
            family: Cow::Owned(self.family.into_owned()),
            size_px: self.size_px,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontPreferenceError {
    EmptyFamily,
    InvalidSize,
}

/// Existing native setter validation, independent of property computation.
/// # Errors
/// Empty/whitespace family or nonfinite/nonpositive size remains invalid.
pub fn computed_font_preferences(
    family: Cow<'_, str>,
    size_px: f64,
) -> Result<ComputedFontPreferences<'_>, FontPreferenceError> {
    if family.trim().is_empty() {
        return Err(FontPreferenceError::EmptyFamily);
    }
    if !size_px.is_finite() || size_px <= 0.0 {
        return Err(FontPreferenceError::InvalidSize);
    }
    Ok(ComputedFontPreferences { family, size_px })
}

/// Borrow the canonical measured default; explicit WASM callers supply host facts.
#[must_use]
pub fn default_computed_font_preferences(
    platform: FontDefaultsPlatform,
) -> ComputedFontPreferences<'static> {
    let family = match platform {
        FontDefaultsPlatform::MacOs => computed_preference_seeds::INITIAL_FONT_FAMILY,
        FontDefaultsPlatform::Other => "Times New Roman",
    };
    ComputedFontPreferences {
        family: Cow::Borrowed(family),
        size_px: computed_preference_seeds::INITIAL_FONT_SIZE_PX,
    }
}

/// Canonical measured light-context initial color, without a duplicated JS table.
#[must_use]
pub fn default_initial_color() -> &'static str {
    computed_preference_seeds::INITIAL_COLOR
}
/// Source identity of the shared generated seed data.
pub const PREFERENCE_SOURCE_SHA256: &str = computed_preference_seeds::SOURCE_SHA256;

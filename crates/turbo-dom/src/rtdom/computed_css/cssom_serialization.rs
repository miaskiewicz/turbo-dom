//! Post-used-value CSSOM projection. This module neither cascades nor computes
//! layout: its input is the canonical ordinary map finalized by the real provider.
use super::{
    font_values::{line_height_used, FontValueError},
    prepared::CascadeOrigin,
    resolve::DeclaredTextDecoration,
    ComputationLimits, CssText,
};
use crate::rtdom::css_shorthand::{
    computed_shorthand_value, longhands, shorthands_of, Declaration, LONGHAND_ALIASES,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CssomSerializationLimits {
    /// UTF-16 units retained in map keys/values and the separate public-key set.
    pub retained_units: usize,
    /// Original caller source/output bounds for the used-font projection.
    pub computation: ComputationLimits,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CssomSerializationError {
    ResourceLimit,
    Font(FontValueError),
    CustomPropertyInOrdinary(String),
    NonScalarLonghand(String),
    ConflictingDerivedValue(String),
}
#[derive(Debug, Eq, PartialEq)]
pub struct SerializedCssom {
    pub values: BTreeMap<String, CssText>,
    pub public_keys: BTreeSet<String>,
}
/// One property-specific presentation choice, independent of media, motion or
/// any environment-wide compatibility mode. The caller maps existing semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextDecorationSerialization {
    Computed,
    Declared,
}
/// Already selected raw declaration; cascade ranking stays with the resolver.
#[derive(Clone, Copy)]
pub struct SelectedTextDecoration<'a> {
    pub value: &'a CssText,
    pub origin: CascadeOrigin,
}
impl<'a> From<&'a DeclaredTextDecoration> for SelectedTextDecoration<'a> {
    fn from(marker: &'a DeclaredTextDecoration) -> Self {
        Self {
            value: &marker.value,
            origin: marker.origin,
        }
    }
}
pub struct CssomSerializationContext<'a> {
    pub text_decoration: TextDecorationSerialization,
    pub declared_text_decoration: Option<SelectedTextDecoration<'a>>,
    /// Actual Core computed own font size, even when not a public CSS key.
    pub font_size_px: Option<f64>,
}
impl CssomSerializationContext<'_> {
    fn omit_computed(&self, name: &str) -> bool {
        self.text_decoration == TextDecorationSerialization::Declared && name == "text-decoration"
    }
}
type Result<T> = std::result::Result<T, CssomSerializationError>;
fn charge(remaining: &mut usize, units: usize) -> Result<()> {
    *remaining = remaining
        .checked_sub(units)
        .ok_or(CssomSerializationError::ResourceLimit)?;
    Ok(())
}
fn ordinary_name(name: &str) -> Result<()> {
    if name.starts_with("--") {
        return Err(CssomSerializationError::CustomPropertyInOrdinary(
            name.to_owned(),
        ));
    }
    Ok(())
}
fn projected_value<'a>(
    name: &str,
    value: &'a CssText,
    line_height: Option<&'a CssText>,
) -> &'a CssText {
    if name == "line-height" {
        line_height.unwrap_or(value)
    } else {
        value
    }
}
fn preflight(
    values: &BTreeMap<String, CssText>,
    keys: &BTreeSet<String>,
    context: &CssomSerializationContext<'_>,
    line_height: Option<&CssText>,
    remaining: &mut usize,
) -> Result<()> {
    for (name, value) in values
        .iter()
        .filter(|(name, _)| !context.omit_computed(name))
    {
        ordinary_name(name)?;
        charge(remaining, name.encode_utf16().count())?;
        charge(
            remaining,
            projected_value(name, value, line_height).as_units().len(),
        )?;
    }
    for name in keys.iter().filter(|name| !context.omit_computed(name)) {
        ordinary_name(name)?;
        charge(remaining, name.encode_utf16().count())?;
    }
    Ok(())
}
impl SerializedCssom {
    fn insert(&mut self, name: &str, value: &CssText, remaining: &mut usize) -> Result<()> {
        if let Some(existing) = self.values.get(name) {
            if existing != value {
                return Err(CssomSerializationError::ConflictingDerivedValue(
                    name.to_owned(),
                ));
            }
        } else {
            charge(remaining, name.encode_utf16().count())?;
            charge(remaining, value.as_units().len())?;
            self.values.insert(name.to_owned(), value.clone());
        }
        if !self.public_keys.contains(name) {
            charge(remaining, name.encode_utf16().count())?;
            self.public_keys.insert(name.to_owned());
        }
        Ok(())
    }
}
fn aliases(
    result: &mut SerializedCssom,
    ordinary: &BTreeMap<String, CssText>,
    original_public: &BTreeSet<String>,
    remaining: &mut usize,
) -> Result<()> {
    for (alias, canonical) in LONGHAND_ALIASES {
        if !original_public.contains(*canonical) {
            continue;
        }
        if let Some(value) = ordinary.get(*canonical) {
            result.insert(alias, value, remaining)?;
        }
    }
    Ok(())
}
fn candidates(public: &BTreeSet<String>) -> BTreeSet<&'static str> {
    public.iter().flat_map(|name| shorthands_of(name)).collect()
}
fn shorthand(
    name: &str,
    values: &BTreeMap<String, CssText>,
    public: &BTreeSet<String>,
) -> Result<Option<String>> {
    // Only canonical candidates from shorthands_of reach this private helper.
    let required = longhands(name).expect("canonical shorthand candidate has a table entry");
    if required
        .iter()
        .any(|name| !public.contains(*name) || !values.contains_key(*name))
    {
        return Ok(None);
    }
    let owned: Vec<_> = required
        .iter()
        .map(|name| {
            String::from_utf16(values[*name].as_units())
                .map_err(|_| CssomSerializationError::NonScalarLonghand((*name).to_owned()))
        })
        .collect::<Result<_>>()?;
    let declarations: Vec<_> = required
        .iter()
        .zip(&owned)
        .map(|(name, value)| Declaration {
            name,
            value,
            important: false,
            pending: None,
        })
        .collect();
    let value = computed_shorthand_value(name, &declarations);
    Ok((!value.is_empty()).then_some(value))
}
/// Preserve partial ordinary values and authored/inherited public provenance.
/// No aliases or shorthands are manufactured from UA-only/default longhands.
/// Every required shorthand longhand must already be both present and public.
/// Custom properties belong to their separate unchanged UTF-16 map.
pub fn serialize_cssom(
    ordinary: &BTreeMap<String, CssText>,
    public_keys: &BTreeSet<String>,
    limits: CssomSerializationLimits,
) -> Result<SerializedCssom> {
    serialize_cssom_with_context(
        ordinary,
        public_keys,
        &CssomSerializationContext {
            text_decoration: TextDecorationSerialization::Computed,
            declared_text_decoration: None,
            font_size_px: None,
        },
        limits,
    )
}
/// Project final ordinary values using the existing selected text-decoration
/// semantics. Declared mode uses only the canonical winner annotation; it does
/// not infer a marker from longhands, inherit it or expose a UA-origin marker.
/// All other unknown/partial ordinary values preserve the original API contract.
pub fn serialize_cssom_with_context(
    ordinary: &BTreeMap<String, CssText>,
    public_keys: &BTreeSet<String>,
    context: &CssomSerializationContext<'_>,
    limits: CssomSerializationLimits,
) -> Result<SerializedCssom> {
    let line_height = ordinary
        .get("line-height")
        .map(|value| line_height_used(value, context.font_size_px, limits.computation))
        .transpose()
        .map_err(CssomSerializationError::Font)?
        .flatten();
    let mut remaining = limits.retained_units;
    preflight(
        ordinary,
        public_keys,
        context,
        line_height.as_ref(),
        &mut remaining,
    )?;
    let mut result = SerializedCssom {
        values: ordinary
            .iter()
            .filter(|(name, _)| !context.omit_computed(name))
            .map(|(name, value)| {
                (
                    name.clone(),
                    projected_value(name, value, line_height.as_ref()).clone(),
                )
            })
            .collect(),
        public_keys: public_keys
            .iter()
            .filter(|name| !context.omit_computed(name))
            .cloned()
            .collect(),
    };
    aliases(&mut result, ordinary, public_keys, &mut remaining)?;
    append_shorthands(&mut result, public_keys, context, &mut remaining)?;
    apply_declared_decoration(&mut result, context, &mut remaining)?;
    Ok(result)
}

fn apply_declared_decoration(
    result: &mut SerializedCssom,
    context: &CssomSerializationContext<'_>,
    remaining: &mut usize,
) -> Result<()> {
    if context.text_decoration != TextDecorationSerialization::Declared {
        return Ok(());
    }
    let Some(marker) = context.declared_text_decoration else {
        return Ok(());
    };
    if marker.origin == CascadeOrigin::UserAgent {
        return Ok(());
    }
    result.insert("text-decoration", marker.value, remaining)
}

fn append_shorthands(
    result: &mut SerializedCssom,
    public_keys: &BTreeSet<String>,
    context: &CssomSerializationContext<'_>,
    remaining: &mut usize,
) -> Result<()> {
    for name in candidates(public_keys) {
        if context.omit_computed(name) {
            continue;
        }
        if let Some(value) = shorthand(name, &result.values, public_keys)? {
            let units = value.encode_utf16().count();
            // Check the final payload before allocating its UTF-16 representation.
            if units > *remaining {
                return Err(CssomSerializationError::ResourceLimit);
            }
            result.insert(
                name,
                &CssText::from_units(value.encode_utf16().collect()),
                remaining,
            )?;
        }
    }
    Ok(())
}

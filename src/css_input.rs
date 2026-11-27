//! Shared ownership/quota readers for already-finalized CSS input rows.
use std::collections::{BTreeMap, BTreeSet};
use turbo_dom_core::rtdom::computed_css::CssText;
#[derive(Clone, Copy, Debug)]
pub(super) enum InputError {
    ResourceLimit,
    DuplicateProperty,
    DuplicatePublicKey,
}
pub(super) fn charge(remaining: &mut usize, amount: usize) -> Result<(), InputError> {
    *remaining = remaining
        .checked_sub(amount)
        .ok_or(InputError::ResourceLimit)?;
    Ok(())
}
pub(super) fn ordinary(
    source: Vec<(String, Vec<u16>)>,
    input: &mut usize,
) -> Result<BTreeMap<String, CssText>, InputError> {
    let mut result = BTreeMap::new();
    for (name, units) in source {
        charge(input, name.encode_utf16().count())?;
        charge(input, units.len())?;
        if result.insert(name, CssText::from_units(units)).is_some() {
            return Err(InputError::DuplicateProperty);
        }
    }
    Ok(result)
}
pub(super) fn provenance(
    source: Vec<String>,
    input: &mut usize,
) -> Result<BTreeSet<String>, InputError> {
    let mut result = BTreeSet::new();
    for name in source {
        charge(input, name.encode_utf16().count())?;
        if !result.insert(name) {
            return Err(InputError::DuplicatePublicKey);
        }
    }
    Ok(result)
}

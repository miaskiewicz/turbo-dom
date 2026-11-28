//! Native-owned computed results. No DOM handles or JavaScript references.
#[cfg(test)]
use super::encode_rows;
use super::{result_row, ComputedStyle, RendererDefaults, ResultRow};
use std::collections::BTreeMap;
use std::mem::size_of;
use turbo_dom_core::rtdom::computed_css::CssText;

pub(crate) struct NativeStyleState {
    pub(crate) geometry_only: bool,
    pub(crate) order: Box<[u32]>,
    pub(crate) styles: BTreeMap<u32, ComputedStyle>,
    pub(crate) extras: BTreeMap<u32, RendererDefaults>,
}
fn text_bytes(value: &CssText) -> usize {
    value.as_units().len().saturating_mul(size_of::<u16>())
}
fn map_bytes(values: &BTreeMap<String, CssText>) -> usize {
    values.iter().fold(0usize, |sum, (key, value)| {
        sum.saturating_add(key.len())
            .saturating_add(text_bytes(value))
    })
}
fn style_bytes(style: &ComputedStyle) -> usize {
    let mut bytes =
        map_bytes(&style.values.values).saturating_add(map_bytes(style.custom.as_map()));
    bytes = bytes.saturating_add(2 * size_of::<f64>());
    for key in &style.public_keys {
        bytes = bytes.saturating_add(key.len());
    }
    if let Some(marker) = &style.declared_text_decoration {
        bytes = bytes
            .saturating_add(text_bytes(&marker.value))
            .saturating_add(size_of_val(&marker.origin))
            .saturating_add(2 * size_of::<bool>())
            .saturating_add(2 * size_of::<usize>());
    }
    bytes
}
impl NativeStyleState {
    pub(crate) fn payload_bytes(&self) -> usize {
        let mut bytes =
            size_of::<bool>().saturating_add(self.order.len().saturating_mul(size_of::<u32>()));
        for style in self.styles.values() {
            bytes = bytes
                .saturating_add(size_of::<u32>())
                .saturating_add(style_bytes(style));
        }
        for extras in self.extras.values() {
            bytes = bytes.saturating_add(size_of::<u32>());
            for (name, value) in extras {
                bytes = bytes
                    .saturating_add(name.len())
                    .saturating_add(text_bytes(value));
            }
        }
        bytes
    }
    pub(crate) fn renderer(&self, id: u32) -> Option<BTreeMap<String, CssText>> {
        let mut values = self.styles.get(&id)?.values.values.clone();
        for (name, value) in self.extras.get(&id)? {
            values.insert((*name).to_owned(), value.clone());
        }
        Some(values)
    }
    pub(crate) fn rows(&self) -> Vec<ResultRow> {
        self.order
            .iter()
            .map(|id| result_row(*id, &self.styles[id], self.extras[id].clone()))
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn legacy_wire(&self) -> Result<String, String> {
        encode_rows(self.geometry_only, self.rows())
    }
}

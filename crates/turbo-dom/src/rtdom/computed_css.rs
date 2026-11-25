//! CSS source primitives for native and JavaScript computed-style adapters.
//! Raw declaration payloads retain DOM code units; the scalar view is only a
//! structural tokenizer input, never a replacement for authored custom tokens.
use std::ops::Range;
pub mod stylesheet;
mod variables;
pub use variables::{
    check_component_limits, compute_custom_properties, has_function_reference,
    has_variable_references, substitute, valid_custom_value_syntax, ComputationLimits,
    ComputedCssError, CustomProperties,
};

/// Original CSS text, including any unpaired UTF-16 units.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CssText(Vec<u16>);

impl CssText {
    #[must_use]
    pub fn from_units(units: Vec<u16>) -> Self {
        Self(units)
    }

    #[must_use]
    pub fn as_units(&self) -> &[u16] {
        &self.0
    }

    fn trim_css(mut self) -> Self {
        let space = |unit: &u16| matches!(unit, 9 | 10 | 12 | 13 | 32);
        let start = self
            .0
            .iter()
            .position(|unit| !space(unit))
            .unwrap_or(self.0.len());
        let end = self
            .0
            .iter()
            .rposition(|unit| !space(unit))
            .map_or(start, |end| end + 1);
        self.0 = self.0[start..end].to_vec();
        self
    }
}

/// A scalar tokenizer view with exact source-coordinate mapping.
pub struct CssSource {
    raw: Vec<u16>,
    scalar: String,
    offsets: Vec<usize>,
}

impl CssSource {
    #[must_use]
    pub fn new(raw: &[u16]) -> Self {
        let mut scalar = String::new();
        let mut offsets = Vec::new();
        let mut unit = 0;
        for value in char::decode_utf16(raw.iter().copied()) {
            let (value, count) = match value {
                Ok(value) => (value, value.len_utf16()),
                Err(_) => ('\u{fffd}', 1),
            };
            offsets.resize(offsets.len() + value.len_utf8(), unit);
            scalar.push(value);
            unit += count;
        }
        offsets.push(raw.len());
        Self {
            raw: raw.to_vec(),
            scalar,
            offsets,
        }
    }

    /// The scalar mirror is suitable for structural parsing only.
    #[must_use]
    pub fn scalar(&self) -> &str {
        &self.scalar
    }

    /// Recover original units at valid scalar byte boundaries. Unknown ranges
    /// are rejected rather than rounding into or repairing a source character.
    #[must_use]
    pub fn raw_range(&self, range: Range<usize>) -> Option<CssText> {
        self.scalar.get(range.clone())?;
        Some(CssText(
            self.raw[self.offsets[range.start]..self.offsets[range.end]].to_vec(),
        ))
    }
}

/// A declaration parsed by the existing shared CSS Syntax declaration parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CssDeclaration {
    pub name: String,
    pub value: CssText,
    pub important: bool,
}

impl CssDeclaration {
    /// Serialize a normalized name through CSS Syntax while retaining raw value units.
    #[must_use]
    pub fn serialized(&self) -> CssText {
        let mut name = String::new();
        cssparser::serialize_identifier(&self.name, &mut name).expect("String serialization");
        let mut units: Vec<u16> = name.encode_utf16().collect();
        units.push(u16::from(b':'));
        units.extend_from_slice(self.value.as_units());
        if self.important {
            units.extend(" !important".encode_utf16());
        }
        units.push(u16::from(b';'));
        CssText::from_units(units)
    }
}

/// Parse declarations without normalizing their raw custom-property payloads.
#[must_use]
pub fn declarations(units: &[u16]) -> Vec<CssDeclaration> {
    let source = CssSource::new(units);
    super::css_shorthand::declaration_ranges(source.scalar())
        .into_iter()
        .map(|(name, range, important)| CssDeclaration {
            name,
            // cssparser ranges are within this source at scalar boundaries.
            value: source
                .raw_range(range)
                .expect("CSS parser source range")
                .trim_css(),
            important,
        })
        .collect()
}

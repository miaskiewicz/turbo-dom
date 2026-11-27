//! A payload-only lower bound, not native heap accounting or additive owner usage.
use super::model::{CompiledStyles, StyleEvent};
use super::prepared::PreparedStyles;
use super::CssText;
use crate::rtdom::query::{SelectorError, SelectorList};
fn text(value: &CssText) -> usize {
    value.0.capacity() * std::mem::size_of::<u16>()
}
fn selectors(value: &Result<SelectorList, SelectorError>) -> usize {
    value.as_ref().map_or(0, |list| {
        list.selectors()
            .iter()
            .map(crate::rtdom::query::CascadeSelector::source_payload_bytes)
            .sum()
    })
}
fn event(event: &StyleEvent) -> usize {
    match event {
        StyleEvent::Statement { name, prelude } => name.capacity() + text(prelude),
        StyleEvent::Descriptor {
            name,
            prelude,
            body,
        } => name.capacity() + text(prelude) + text(body),
        _ => 0,
    }
}
impl CompiledStyles {
    /// Retained source/value payload bytes only; excludes parsed AST allocations,
    /// containers, allocator bookkeeping and shared-owner double-count correction.
    /// Prepared and compiled views report the same shared CSS payload, not additive usage.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> usize {
        let data = self.metadata();
        let mut bytes = 0;
        for selector in &data.selectors {
            bytes += text(&selector.source) + selectors(&selector.compiled);
        }
        // Boundary policy vectors, like ordinary selector policies, retain only
        // scalar classification records. Containers are outside this metric.
        for group in &data.groups {
            bytes += text(&group.prelude);
            if let Some(scope) = &group.scope {
                bytes += scope.start.as_ref().map_or(0, text) + scope.end.as_ref().map_or(0, text);
            }
            bytes += group.scope_start.as_ref().map_or(0, selectors)
                + group.scope_end.as_ref().map_or(0, selectors);
        }
        for rule in &data.rules {
            for declaration in &rule.declarations {
                bytes += declaration.name.capacity() + text(&declaration.value);
            }
            for declaration in &rule.compiled_declarations {
                bytes += declaration
                    .declared_text_decoration
                    .as_ref()
                    .map_or(0, text);
                for property in &declaration.properties {
                    bytes += property.name.capacity() + property.value.as_ref().map_or(0, text);
                }
            }
        }
        bytes + data.events.iter().map(event).sum::<usize>()
    }
}
impl PreparedStyles {
    /// The same lower-bound shared CSS payload metric as its compiled owner.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> usize {
        self.metadata().compiled.retained_payload_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{CompileLimits, SheetSource};
    use super::*;
    #[test]
    fn payload_measurement_includes_owned_scope_and_descriptor_sources() {
        let css = "@layer named;@font-face{font-family:'Example';src:url(example.woff2)}@scope(.card) to(.stop){.x{margin:1px;--Token:'raw'}}";
        let compiled = CompiledStyles::compile(
            &[SheetSource {
                id: 1,
                text: CssText::from_units(css.encode_utf16().collect()),
            }],
            CompileLimits {
                sources: 1,
                source_units: 4096,
                rules: 16,
                source_rule_preludes: 16,
                selectors: 16,
                declarations: 16,
                properties: 64,
                group_depth: 8,
                token_nesting: 16,
            },
        )
        .unwrap();
        let payload = compiled.retained_payload_bytes();
        assert!(
            payload
                >= "font-family:'Example';src:url(example.woff2)"
                    .encode_utf16()
                    .count()
                    * 2
        );
        let retained = compiled.clone();
        drop(compiled);
        assert_eq!(payload, retained.retained_payload_bytes());
    }
}

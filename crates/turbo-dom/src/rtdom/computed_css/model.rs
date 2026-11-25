//! Immutable CSS-only compilation records shared by bulk computed-style adapters.
//! No node identity, DOM arena, callbacks or per-element result cache is retained here.
use super::declaration_model::{compile_declarations, CompiledDeclaration};
use super::scope_plan::{parse_scope_prelude, ScopePrelude};
use super::stylesheet::{
    parse_sheet_with_unknown_rules, GroupKind, GroupMode, SheetError, SheetLimits, SheetRule,
    UnknownRulePolicy,
};
use super::{
    check_component_limits, declarations, ComputationLimits, ComputedCssError, CssDeclaration,
    CssText,
};
use crate::rtdom::query::{
    parse_for_cascade_units, parse_for_cascade_units_scoped, SelectorError, SelectorList,
};
use std::sync::Arc;

/// A source identity belongs to this compiled object, never to a DOM node.
pub struct SheetSource {
    pub id: u32,
    pub text: CssText,
}

/// All bounds are supplied by the embedding adapter; compilation has no hidden budget.
#[derive(Clone, Copy)]
pub struct CompileLimits {
    pub sources: usize,
    pub source_units: usize,
    /// Maximum retained compiled rule records across all sources.
    pub rules: usize,
    /// Maximum encountered structural rule preludes within each source.
    pub source_rule_preludes: usize,
    pub selectors: usize,
    pub declarations: usize,
    pub properties: usize,
    pub group_depth: usize,
    pub token_nesting: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileError {
    SourceCount,
    SourceUnits,
    DuplicateSource,
    RuleCount,
    SelectorCount,
    DeclarationCount,
    PropertyCount,
    GroupDepth,
    Computation(ComputedCssError),
}

/// Selector source is retained even when the current static compiler cannot classify it.
/// Preparation must resolve that capability rather than silently discard a valid rule.
pub struct MatchSelector {
    pub source: CssText,
    pub compiled: Result<SelectorList, SelectorError>,
    pub scoped: bool,
    pub policy: super::selector_policy::SelectorPolicyInspection,
}

pub struct CompiledGroup {
    pub kind: GroupKind,
    pub prelude: CssText,
    pub parent: Option<usize>,
    pub scope: Option<ScopePrelude>,
    pub invalid_scope: bool,
    pub scope_start: Option<Result<SelectorList, SelectorError>>,
    pub scope_end: Option<Result<SelectorList, SelectorError>>,
}

pub struct CompiledRule {
    pub source_id: u32,
    pub selector_id: usize,
    pub groups: Vec<usize>,
    pub declarations: Vec<CssDeclaration>,
    pub compiled_declarations: Vec<CompiledDeclaration>,
}

/// Source-order events preserve layer statements, including target-dependent groups.
/// Media/supports preparation may suppress events; scope/container matching cannot.
pub enum StyleEvent {
    Source(u32),
    Enter(usize),
    Exit(usize),
    Rule(usize),
    Statement {
        name: String,
        prelude: CssText,
    },
    Descriptor {
        name: String,
        prelude: CssText,
        body: CssText,
    },
}

pub struct CompiledData {
    pub selectors: Vec<MatchSelector>,
    pub groups: Vec<CompiledGroup>,
    pub rules: Vec<CompiledRule>,
    pub events: Vec<StyleEvent>,
    source_ids: Vec<u32>,
}

/// Clone shares only immutable CSS data. Dropping an adapter releases its ownership;
/// another prepared owner may keep the same data alive without any global registry.
#[derive(Clone)]
pub struct CompiledStyles(Arc<CompiledData>);

impl CompiledStyles {
    #[must_use]
    pub fn metadata(&self) -> &CompiledData {
        &self.0
    }

    /// Compile source syntax/declarations once. Selector IDs refer to complete lists;
    /// the matching adapter reports matching list-item IDs, not a guessed specificity.
    pub fn compile(sources: &[SheetSource], limits: CompileLimits) -> Result<Self, CompileError> {
        if sources.len() > limits.sources {
            return Err(CompileError::SourceCount);
        }
        let mut compiler = Compiler::new(limits);
        for source in sources {
            compiler.source(source)?;
        }
        Ok(Self(Arc::new(compiler.data)))
    }
}

struct Compiler {
    data: CompiledData,
    limits: CompileLimits,
    source_units: usize,
    declarations: usize,
    properties: usize,
    selectors: usize,
    rules: usize,
}

fn charge(
    count: &mut usize,
    amount: usize,
    maximum: usize,
    error: CompileError,
) -> Result<(), CompileError> {
    let next = count.checked_add(amount).ok_or(error)?;
    if next > maximum {
        return Err(error);
    }
    *count = next;
    Ok(())
}

impl Compiler {
    fn new(limits: CompileLimits) -> Self {
        Self {
            data: CompiledData {
                selectors: Vec::new(),
                groups: Vec::new(),
                rules: Vec::new(),
                events: Vec::new(),
                source_ids: Vec::new(),
            },
            limits,
            source_units: 0,
            declarations: 0,
            properties: 0,
            selectors: 0,
            rules: 0,
        }
    }

    fn computation_limits(&self) -> ComputationLimits {
        ComputationLimits {
            max_nesting: self.limits.token_nesting,
            max_output_units: self.limits.source_units,
            max_total_units: self.limits.source_units,
        }
    }

    fn source(&mut self, source: &SheetSource) -> Result<(), CompileError> {
        if self.data.source_ids.contains(&source.id) {
            return Err(CompileError::DuplicateSource);
        }
        charge(
            &mut self.source_units,
            source.text.as_units().len(),
            self.limits.source_units,
            CompileError::SourceUnits,
        )?;
        check_component_limits(&source.text, self.computation_limits())
            .map_err(CompileError::Computation)?;
        let rules = parse_sheet_with_unknown_rules(
            source.text.as_units(),
            SheetLimits {
                source_units: source.text.as_units().len(),
                rules: self.limits.source_rule_preludes,
                group_depth: self.limits.group_depth,
            },
            GroupMode::Recursive,
            UnknownRulePolicy::PreserveOpaque,
        )
        .map_err(|error| match error {
            // The parser receives the already-owned source's exact unit length.
            SheetError::SourceUnits => unreachable!("exact source quota cannot reject itself"),
            SheetError::RuleCount => CompileError::RuleCount,
            SheetError::GroupDepth => CompileError::GroupDepth,
        })?;
        self.data.source_ids.push(source.id);
        self.data.events.push(StyleEvent::Source(source.id));
        self.nodes(source.id, rules, &mut Vec::new())
    }

    fn nodes(
        &mut self,
        source_id: u32,
        rules: Vec<SheetRule>,
        groups: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        for rule in rules {
            charge(
                &mut self.rules,
                1,
                self.limits.rules,
                CompileError::RuleCount,
            )?;
            self.node(source_id, rule, groups)?;
        }
        Ok(())
    }

    fn node(
        &mut self,
        source_id: u32,
        rule: SheetRule,
        groups: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        match rule {
            SheetRule::Style { selector, body } => self.style(source_id, selector, &body, groups),
            SheetRule::Group {
                kind,
                prelude,
                rules,
            } => {
                let scope = if kind == GroupKind::Scope {
                    parse_scope_prelude(&prelude, self.computation_limits())
                        .map_err(CompileError::Computation)?
                } else {
                    None
                };
                let scope_start = scope
                    .as_ref()
                    .and_then(|value| value.start.as_ref())
                    .map(|value| parse_for_cascade_units_scoped(value.as_units()));
                let scope_end = scope
                    .as_ref()
                    .and_then(|value| value.end.as_ref())
                    .map(|value| parse_for_cascade_units_scoped(value.as_units()));
                let invalid_scope = kind == GroupKind::Scope && scope.is_none();
                let id = self.data.groups.len();
                self.data.groups.push(CompiledGroup {
                    kind,
                    prelude,
                    parent: groups.last().copied(),
                    scope,
                    invalid_scope,
                    scope_start,
                    scope_end,
                });
                self.data.events.push(StyleEvent::Enter(id));
                groups.push(id);
                self.nodes(source_id, rules, groups)?;
                groups.pop();
                self.data.events.push(StyleEvent::Exit(id));
                Ok(())
            }
            SheetRule::Statement { name, prelude } => {
                self.data
                    .events
                    .push(StyleEvent::Statement { name, prelude });
                Ok(())
            }
            SheetRule::Opaque {
                name,
                prelude,
                body,
            } => {
                self.data.events.push(StyleEvent::Descriptor {
                    name,
                    prelude,
                    body,
                });
                Ok(())
            }
            // collect_rules removes ignored records, recursively, before this walk.
            SheetRule::Ignored => unreachable!("ignored parser records are not collected"),
        }
    }

    fn style(
        &mut self,
        source_id: u32,
        source: CssText,
        body: &CssText,
        groups: &[usize],
    ) -> Result<(), CompileError> {
        let scoped = groups
            .iter()
            .any(|id| self.data.groups[*id].kind == GroupKind::Scope);
        let compiled = if scoped {
            parse_for_cascade_units_scoped(source.as_units())
        } else {
            parse_for_cascade_units(source.as_units())
        };
        let policy =
            super::selector_policy::inspect_selector_policy(&source, self.computation_limits())
                .map_err(CompileError::Computation)?;
        let count = compiled.as_ref().map_or(1, |list| list.selectors().len());
        charge(
            &mut self.selectors,
            count,
            self.limits.selectors,
            CompileError::SelectorCount,
        )?;
        let declarations = declarations(body.as_units());
        charge(
            &mut self.declarations,
            declarations.len(),
            self.limits.declarations,
            CompileError::DeclarationCount,
        )?;
        let compiled_declarations = compile_declarations(&declarations, self.computation_limits())
            .map_err(CompileError::Computation)?;
        let property_count = compiled_declarations
            .iter()
            .try_fold(0usize, |count, declaration| {
                count.checked_add(declaration.properties.len())
            })
            .ok_or(CompileError::PropertyCount)?;
        charge(
            &mut self.properties,
            property_count,
            self.limits.properties,
            CompileError::PropertyCount,
        )?;
        let selector_id = self.data.selectors.len();
        self.data.selectors.push(MatchSelector {
            source,
            compiled,
            scoped,
            policy,
        });
        let id = self.data.rules.len();
        self.data.rules.push(CompiledRule {
            source_id,
            selector_id,
            groups: groups.to_vec(),
            declarations,
            compiled_declarations,
        });
        self.data.events.push(StyleEvent::Rule(id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(id: u32, text: &str) -> SheetSource {
        SheetSource {
            id,
            text: CssText::from_units(text.encode_utf16().collect()),
        }
    }
    fn limits() -> CompileLimits {
        CompileLimits {
            sources: 4,
            source_units: 8192,
            rules: 64,
            source_rule_preludes: 64,
            selectors: 64,
            declarations: 128,
            properties: 1024,
            group_depth: 8,
            token_nesting: 32,
        }
    }

    #[test]
    fn compile_once_owns_raw_declarations_and_context_records_without_dom_facts() {
        let mut sources=vec![source(7,"@layer reset,app;.x{--Token:\"a; } { b\";color:var(--Token)}@media screen{@scope(.root){.x{color:blue}}}")];
        let styles = CompiledStyles::compile(&sources, limits()).unwrap();
        sources.clear();
        let metadata = styles.metadata();
        assert_eq!(metadata.rules.len(), 2);
        assert_eq!(metadata.rules[0].source_id, 7);
        assert!(metadata.rules[0].groups.is_empty());
        assert_eq!(metadata.rules[1].groups, [0, 1]);
        assert_eq!(metadata.groups[1].parent, Some(0));
        assert_eq!(metadata.groups[1].kind, GroupKind::Scope);
        let raw = &metadata.rules[0].declarations[0];
        assert_eq!(raw.name, "--Token");
        assert_eq!(
            String::from_utf16(raw.value.as_units()).unwrap(),
            "\"a; } { b\""
        );
        assert!(matches!(&metadata.events[0], StyleEvent::Source(7)));
        assert!(matches!(&metadata.events[1],StyleEvent::Statement{name,..} if name=="layer"));
        let cloned = styles.clone();
        assert!(std::ptr::eq(styles.metadata(), cloned.metadata()));
    }

    #[test]
    fn recursive_sheet_bounds_fail_before_a_partial_compiled_model_is_returned() {
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x{}")],
                CompileLimits {
                    source_rule_preludes: 0,
                    ..limits()
                }
            ),
            Err(CompileError::RuleCount)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, "@media screen{.x{color:red}}")],
                CompileLimits {
                    group_depth: 0,
                    ..limits()
                }
            ),
            Err(CompileError::GroupDepth)
        ));
        let empty = CompiledStyles::compile(
            &[source(1, "/*comment*/")],
            CompileLimits {
                source_rule_preludes: 0,
                ..limits()
            },
        )
        .unwrap();
        assert!(empty.metadata().rules.is_empty());
    }
    #[test]
    fn source_ids_are_not_document_ids_and_input_limits_do_not_become_empty_styles() {
        assert!(matches!(
            CompiledStyles::compile(&[source(1, ".x{}"), source(1, ".y{}")], limits()),
            Err(CompileError::DuplicateSource)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x{}")],
                CompileLimits {
                    source_units: 0,
                    ..limits()
                }
            ),
            Err(CompileError::SourceUnits)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x{}")],
                CompileLimits {
                    sources: 0,
                    ..limits()
                }
            ),
            Err(CompileError::SourceCount)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x{color:red}")],
                CompileLimits {
                    declarations: 0,
                    properties: 0,
                    ..limits()
                }
            ),
            Err(CompileError::DeclarationCount)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x,.y{}")],
                CompileLimits {
                    selectors: 1,
                    ..limits()
                }
            ),
            Err(CompileError::SelectorCount)
        ));
        assert!(matches!(
            CompiledStyles::compile(
                &[source(1, ".x{}"), source(2, ".y{}")],
                CompileLimits {
                    rules: 1,
                    ..limits()
                }
            ),
            Err(CompileError::RuleCount)
        ));
    }

    #[test]
    fn valid_but_not_classified_scope_syntax_is_retained_for_preparation_capability() {
        let styles = CompiledStyles::compile(
            &[source(9, "@scope(.root){& .target{color:red}}")],
            limits(),
        )
        .unwrap();
        let selector = &styles.metadata().selectors[0];
        assert_eq!(
            String::from_utf16(selector.source.as_units()).unwrap(),
            "& .target"
        );
        assert_eq!(styles.metadata().rules[0].groups, [0]);
    }
}

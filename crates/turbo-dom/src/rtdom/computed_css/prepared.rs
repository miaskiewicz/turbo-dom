//! Environment preparation is shared across target reads; no DOM graph is retained.
use super::conditions::{
    media_matches, supports_matches, ConditionEnvironment, ConditionEvaluationError,
};
use super::grammar::parse_layer_names;
use super::layers::{LayerError, LayerRegistry};
use super::model::{CompiledStyles, StyleEvent};
use super::stylesheet::GroupKind;
use super::{check_component_limits, ComputationLimits, ComputedCssError, CssSource};
use rustc_hash::FxHashMap;
use std::sync::Arc;

pub use super::cascade_order::CascadeOrigin;

/// Numeric scope identities are request provenance, never a retained node wrapper.
pub struct SheetUse {
    pub source_id: u32,
    pub origin: CascadeOrigin,
    pub tree_scope: u32,
    pub scope_depth: u32,
    /// Original owner root for this stylesheet use; None means unavailable.
    pub implicit_scope_root: Option<u32>,
    pub disabled: bool,
    pub media: Option<super::CssText>,
}

#[derive(Clone, Copy)]
pub struct PreparationLimits {
    pub uses: usize,
    pub rules: usize,
    pub layers: usize,
    pub computation: ComputationLimits,
}

#[derive(Debug)]
pub enum PreparationError {
    UseCount,
    RuleCount,
    UnknownSource(u32),
    Condition(ConditionEvaluationError),
    Computation(ComputedCssError),
    Layer(LayerError),
    RequiresStylePhase,
}

#[derive(Clone)]
pub struct LayerRank {
    pub normal: Arc<[usize]>,
    pub important: Arc<[usize]>,
}

pub struct PreparedRule {
    pub rule_id: usize,
    pub use_id: usize,
    pub origin: CascadeOrigin,
    pub tree_scope: u32,
    pub scope_depth: u32,
    /// Original owner root for this stylesheet use; None means unavailable.
    pub implicit_scope_root: Option<u32>,
    pub layer_context: usize,
    pub layer: Option<usize>,
    pub layer_rank: LayerRank,
}

/// Starting-style participation must come from the host's real style lifecycle.
#[derive(Clone, Copy)]
pub enum StylePhase {
    Steady,
    Starting,
}

pub struct PreparationEnvironment<'a, 'env> {
    pub conditions: &'a ConditionEnvironment<'env>,
    pub style_phase: Option<StylePhase>,
}

pub struct PreparedData {
    pub compiled: CompiledStyles,
    pub rules: Vec<PreparedRule>,
    geometry_definition: Option<usize>,
}

/// A target read borrows this immutable preparation and supplies matching facts only.
#[derive(Clone)]
pub struct PreparedStyles(Arc<PreparedData>, Option<Arc<[bool]>>);
impl PreparedStyles {
    #[must_use]
    pub fn metadata(&self) -> &PreparedData {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContainerGeometryError {
    RequiresDefinition(String),
    RequiresLayoutDependency(String),
}
/// A CSS-only projection for native geometry, never final computed/paint styles.
/// Original rule IDs, order, layers and scopes remain owned by the same preparation.
pub struct ContainerGeometryPlan(PreparedStyles);
impl ContainerGeometryPlan {
    #[must_use]
    pub fn styles_for_measurement(&self) -> &PreparedStyles {
        &self.0
    }
}
impl PreparedStyles {
    #[must_use]
    pub fn is_geometry_only(&self) -> bool {
        self.1.is_some()
    }
    pub(super) fn includes_rule(&self, id: usize) -> bool {
        self.1
            .as_ref()
            .is_none_or(|mask| mask.get(id).copied().unwrap_or(false))
    }
    /// Remove only conditional text colors after proving that no definition or
    /// resource can change geometry. This never produces final CSSOM values.
    ///
    /// # Errors
    /// Refuses all unknown definitions and any conditional non-color property.
    pub fn container_geometry_plan(&self) -> Result<ContainerGeometryPlan, ContainerGeometryError> {
        let data = self.metadata();
        let compiled = data.compiled.metadata();
        if let Some(id) = data.geometry_definition {
            let (StyleEvent::Statement { name, .. } | StyleEvent::Descriptor { name, .. }) =
                &compiled.events[id]
            else {
                unreachable!("definition audit retains only named definition events")
            };
            return Err(ContainerGeometryError::RequiresDefinition(name.clone()));
        }
        let mut include = Vec::with_capacity(data.rules.len());
        for binding in &data.rules {
            let rule = &compiled.rules[binding.rule_id];
            let conditional = rule
                .groups
                .iter()
                .any(|id| compiled.groups[*id].kind == GroupKind::Container);
            if conditional {
                for declaration in &rule.compiled_declarations {
                    for property in &declaration.properties {
                        if property.name != "color" {
                            return Err(ContainerGeometryError::RequiresLayoutDependency(
                                property.name.clone(),
                            ));
                        }
                    }
                }
            }
            include.push(!conditional);
        }
        Ok(ContainerGeometryPlan(PreparedStyles(
            self.0.clone(),
            Some(include.into()),
        )))
    }
}

struct LayerContext {
    origin: CascadeOrigin,
    scope: u32,
    registry: LayerRegistry,
    ranks: FxHashMap<Option<usize>, LayerRank>,
}
impl LayerContext {
    fn rank(&mut self, layer: Option<usize>) -> Result<LayerRank, PreparationError> {
        if let Some(rank) = self.ranks.get(&layer) {
            return Ok(rank.clone());
        }
        let rank = LayerRank {
            normal: self
                .registry
                .rank(layer, false)
                .map_err(PreparationError::Layer)?
                .into(),
            important: self
                .registry
                .rank(layer, true)
                .map_err(PreparationError::Layer)?
                .into(),
        };
        self.ranks.insert(layer, rank.clone());
        Ok(rank)
    }
}

struct Frame {
    active: bool,
    layer: Option<usize>,
}
struct Preparer<'a, 'env> {
    compiled: &'a CompiledStyles,
    environment: &'a PreparationEnvironment<'a, 'env>,
    limits: PreparationLimits,
    contexts: Vec<LayerContext>,
    rules: Vec<PreparedRule>,
    geometry_definition: Option<usize>,
}
impl CompiledStyles {
    pub fn prepare(
        &self,
        uses: &[SheetUse],
        environment: &PreparationEnvironment<'_, '_>,
        limits: PreparationLimits,
    ) -> Result<PreparedStyles, PreparationError> {
        if uses.len() > limits.uses {
            return Err(PreparationError::UseCount);
        }
        let mut preparer = Preparer {
            compiled: self,
            environment,
            limits,
            contexts: Vec::new(),
            rules: Vec::new(),
            geometry_definition: None,
        };
        for (id, binding) in uses.iter().enumerate() {
            preparer.sheet(id, binding)?;
        }
        Ok(PreparedStyles(
            Arc::new(PreparedData {
                compiled: self.clone(),
                rules: preparer.rules,
                geometry_definition: preparer.geometry_definition,
            }),
            None,
        ))
    }
}
impl Preparer<'_, '_> {
    fn context(&mut self, binding: &SheetUse) -> usize {
        if let Some(id) = self.contexts.iter().position(|context| {
            context.origin == binding.origin && context.scope == binding.tree_scope
        }) {
            return id;
        }
        let id = self.contexts.len();
        self.contexts.push(LayerContext {
            origin: binding.origin,
            scope: binding.tree_scope,
            registry: LayerRegistry::new(self.limits.layers),
            ranks: FxHashMap::default(),
        });
        id
    }

    fn enabled(&self, binding: &SheetUse) -> Result<bool, PreparationError> {
        let known_source = self
            .compiled
            .metadata()
            .events
            .iter()
            .any(|event| matches!(event,StyleEvent::Source(id) if *id==binding.source_id));
        if !known_source {
            return Err(PreparationError::UnknownSource(binding.source_id));
        }
        if binding.disabled {
            return Ok(false);
        }
        match &binding.media {
            Some(media) => Self::condition(media_matches(
                media,
                self.environment.conditions,
                self.limits.computation,
            )),
            None => Ok(true),
        }
    }
    fn sheet(&mut self, use_id: usize, binding: &SheetUse) -> Result<(), PreparationError> {
        if !self.enabled(binding)? {
            return Ok(());
        }
        let context = self.context(binding);
        let mut selected = false;
        let mut stack = vec![Frame {
            active: true,
            layer: None,
        }];
        let compiled = self.compiled;
        for (event_id, event) in compiled.metadata().events.iter().enumerate() {
            if let StyleEvent::Source(source) = event {
                selected = *source == binding.source_id;
                continue;
            }
            if selected {
                self.event(use_id, binding, context, (event_id, event), &mut stack)?;
            }
        }
        Ok(())
    }
    fn event(
        &mut self,
        use_id: usize,
        binding: &SheetUse,
        context: usize,
        event: (usize, &StyleEvent),
        stack: &mut Vec<Frame>,
    ) -> Result<(), PreparationError> {
        let (event_id, event) = event;
        match event {
            StyleEvent::Enter(id) => {
                let parent = stack.last().expect("outer source frame");
                let frame = self.enter(context, *id, parent)?;
                stack.push(frame);
            }
            StyleEvent::Exit(_) => {
                stack.pop().expect("balanced compiled group event");
            }
            StyleEvent::Rule(id) => {
                let frame = stack.last().expect("outer source frame");
                if frame.active {
                    self.rule(use_id, binding, context, *id, frame.layer)?;
                }
            }
            StyleEvent::Statement { name, prelude } if name == "layer" => {
                let frame = stack.last().expect("outer source frame");
                if frame.active {
                    self.statement(context, frame.layer, prelude)?;
                }
            }
            StyleEvent::Statement { name, .. } | StyleEvent::Descriptor { name, .. } => {
                if stack.last().expect("outer source frame").active && name != "charset" {
                    self.geometry_definition.get_or_insert(event_id);
                }
            }
            StyleEvent::Source(_) => {
                // sheet consumes source boundaries before dispatching selected events.
                unreachable!("source boundaries never reach preparation event dispatch")
            }
        }
        Ok(())
    }
    fn active_group(
        &self,
        kind: GroupKind,
        prelude: &super::CssText,
    ) -> Result<bool, PreparationError> {
        match kind {
            GroupKind::Media => Self::condition(media_matches(
                prelude,
                self.environment.conditions,
                self.limits.computation,
            )),
            GroupKind::Supports => Self::condition(supports_matches(
                prelude,
                self.environment.conditions,
                self.limits.computation,
            )),
            GroupKind::StartingStyle => match self.environment.style_phase {
                Some(StylePhase::Steady) => Ok(false),
                Some(StylePhase::Starting) => Ok(true),
                None => Err(PreparationError::RequiresStylePhase),
            },
            _ => Ok(true),
        }
    }

    fn enter(
        &mut self,
        context: usize,
        id: usize,
        parent: &Frame,
    ) -> Result<Frame, PreparationError> {
        if !parent.active {
            return Ok(Frame {
                active: false,
                layer: parent.layer,
            });
        }
        let compiled = self.compiled;
        let group = &compiled.metadata().groups[id];
        let active = !group.invalid_scope && self.active_group(group.kind, &group.prelude)?;
        let mut active = active;
        let layer = if active && group.kind == GroupKind::Layer {
            if let Some(layer) = self.block_layer(context, parent.layer, &group.prelude)? {
                Some(layer)
            } else {
                active = false;
                parent.layer
            }
        } else {
            parent.layer
        };
        Ok(Frame { active, layer })
    }

    fn condition(result: Result<bool, ConditionEvaluationError>) -> Result<bool, PreparationError> {
        match result {
            Err(ConditionEvaluationError::InvalidSyntax) => Ok(false),
            result => result.map_err(PreparationError::Condition),
        }
    }
    fn names(&self, text: &super::CssText) -> Result<Option<Vec<Vec<String>>>, PreparationError> {
        check_component_limits(text, self.limits.computation)
            .map_err(PreparationError::Computation)?;
        let source = CssSource::new(text.as_units());
        match parse_layer_names(source.scalar()) {
            Ok(names) => Ok(Some(names)),
            Err(super::grammar::ConditionError::InvalidSyntax) => Ok(None),
            // LayerName parses only ident ('.' ident)* and comma separators;
            // it has no recursive, computation or capability-producing parser.
            Err(_) => unreachable!("layer names only produce basic syntax errors"),
        }
    }
    fn registry(&mut self, context: usize) -> &mut LayerRegistry {
        let used = self.contexts.iter().fold(0usize, |total, item| {
            total.saturating_add(item.registry.layer_count())
        });
        self.contexts[context]
            .registry
            .set_remaining(self.limits.layers.saturating_sub(used));
        &mut self.contexts[context].registry
    }

    fn block_layer(
        &mut self,
        context: usize,
        parent: Option<usize>,
        text: &super::CssText,
    ) -> Result<Option<usize>, PreparationError> {
        let id = if text.as_units().is_empty() {
            self.registry(context).anonymous(parent)
        } else {
            let Some(names) = self.names(text)? else {
                return Ok(None);
            };
            let [name] = names.as_slice() else {
                return Ok(None);
            };
            self.registry(context).named(parent, name)
        }
        .map_err(PreparationError::Layer)?;
        Ok(Some(id))
    }
    fn statement(
        &mut self,
        context: usize,
        parent: Option<usize>,
        text: &super::CssText,
    ) -> Result<(), PreparationError> {
        if let Some(names) = self.names(text)? {
            for name in names {
                self.registry(context)
                    .named(parent, &name)
                    .map_err(PreparationError::Layer)?;
            }
        }
        Ok(())
    }
    fn rule(
        &mut self,
        use_id: usize,
        binding: &SheetUse,
        context: usize,
        id: usize,
        layer: Option<usize>,
    ) -> Result<(), PreparationError> {
        if self.rules.len() >= self.limits.rules {
            return Err(PreparationError::RuleCount);
        }
        let layer_rank = self.contexts[context].rank(layer)?;
        self.rules.push(PreparedRule {
            rule_id: id,
            use_id,
            origin: binding.origin,
            tree_scope: binding.tree_scope,
            implicit_scope_root: binding.implicit_scope_root,
            scope_depth: binding.scope_depth,
            layer_context: context,
            layer,
            layer_rank,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdom::computed_css::{
        conditions::FeatureValue,
        model::{CompileLimits, SheetSource},
        CssText,
    };
    use crate::rtdom::css_shorthand::LengthContext;
    use std::collections::BTreeMap;

    #[test]
    fn layer_limit_is_shared_across_origin_and_tree_scope_contexts() {
        let base = prepare("").unwrap();
        let length = LengthContext { unit_bases: &[] };
        let features = BTreeMap::new();
        let selectors = BTreeMap::new();
        let conditions = ConditionEnvironment {
            media_type: "screen",
            features: &features,
            length_context: &length,
            selector_support: &selectors,
        };
        let environment = PreparationEnvironment {
            conditions: &conditions,
            style_phase: None,
        };
        let mut preparer = Preparer {
            compiled: &base.metadata().compiled,
            environment: &environment,
            limits: PreparationLimits {
                uses: 2,
                rules: 2,
                layers: 1,
                computation: ComputationLimits {
                    max_nesting: 8,
                    max_output_units: 64,
                    max_total_units: 128,
                },
            },
            contexts: vec![
                LayerContext {
                    origin: CascadeOrigin::Author,
                    scope: 0,
                    registry: LayerRegistry::new(1),
                    ranks: FxHashMap::default(),
                },
                LayerContext {
                    origin: CascadeOrigin::Author,
                    scope: 1,
                    registry: LayerRegistry::new(1),
                    ranks: FxHashMap::default(),
                },
            ],
            rules: Vec::new(),
            geometry_definition: None,
        };
        assert!(preparer
            .registry(0)
            .named(None, &["one".to_owned()])
            .is_ok());
        assert!(preparer
            .registry(0)
            .named(None, &["one".to_owned()])
            .is_ok());
        assert_eq!(
            preparer.registry(1).anonymous(None),
            Err(LayerError::LayerCount)
        );
        assert_eq!(
            preparer.registry(0).anonymous(None),
            Err(LayerError::LayerCount)
        );
    }

    fn prepare(css: &str) -> Result<PreparedStyles, PreparationError> {
        prepare_with(css, |_, _, _| {})
    }
    fn prepare_with(
        css: &str,
        config: impl FnOnce(&mut [SheetUse], &mut PreparationLimits, &mut Option<StylePhase>),
    ) -> Result<PreparedStyles, PreparationError> {
        let source = SheetSource {
            id: 7,
            text: CssText::from_units(css.encode_utf16().collect()),
        };
        let compiled = CompiledStyles::compile(
            &[source],
            CompileLimits {
                sources: 1,
                source_units: 8192,
                rules: 128,
                source_rule_preludes: 128,
                selectors: 128,
                declarations: 128,
                properties: 1024,
                group_depth: 16,
                token_nesting: 32,
            },
        )
        .unwrap();
        let units = [("em", 16.0), ("rem", 16.0)];
        let length = LengthContext { unit_bases: &units };
        let features = BTreeMap::from([("width".to_owned(), FeatureValue::LengthPx(800.0))]);
        let selectors = BTreeMap::new();
        let conditions = ConditionEnvironment {
            media_type: "screen",
            features: &features,
            length_context: &length,
            selector_support: &selectors,
        };
        let mut phase = Some(StylePhase::Steady);
        let mut uses = [SheetUse {
            source_id: 7,
            origin: CascadeOrigin::Author,
            tree_scope: 0,
            implicit_scope_root: None,
            scope_depth: 0,
            disabled: false,
            media: None,
        }];
        let mut policy = PreparationLimits {
            uses: 4,
            rules: 128,
            layers: 128,
            computation: ComputationLimits {
                max_nesting: 32,
                max_output_units: 8192,
                max_total_units: 16384,
            },
        };
        config(&mut uses, &mut policy, &mut phase);
        let environment = PreparationEnvironment {
            conditions: &conditions,
            style_phase: phase,
        };
        compiled.prepare(&uses, &environment, policy)
    }

    fn ranked_colors(styles: &PreparedStyles, important: bool) -> Vec<String> {
        let data = styles.metadata();
        let mut rules: Vec<_> = data.rules.iter().collect();
        rules.sort_by(|left, right| {
            let (left, right) = if important {
                (&left.layer_rank.important, &right.layer_rank.important)
            } else {
                (&left.layer_rank.normal, &right.layer_rank.normal)
            };
            left.cmp(right)
        });
        rules
            .into_iter()
            .map(|rule| {
                let declaration = &data.compiled.metadata().rules[rule.rule_id].declarations[0];
                String::from_utf16(declaration.value.as_units()).unwrap()
            })
            .collect()
    }
    #[test]
    fn source_use_media_and_phase_are_independent_preparation_inputs() {
        assert!(matches!(
            prepare_with(".x{color:red}", |_, policy, _| policy.uses = 0),
            Err(PreparationError::UseCount)
        ));
        assert!(matches!(
            prepare_with(".x{color:red}", |uses, _, _| uses[0].source_id = 99),
            Err(PreparationError::UnknownSource(99))
        ));
        let disabled = prepare_with(".x{color:red}", |uses, _, _| uses[0].disabled = true).unwrap();
        assert!(disabled.metadata().rules.is_empty());
        let print = prepare_with(".x{color:red}", |uses, _, _| {
            uses[0].media = Some(CssText::from_units("print".encode_utf16().collect()));
        })
        .unwrap();
        assert!(print.metadata().rules.is_empty());
        let screen = prepare_with(".x{color:red}", |uses, _, _| {
            uses[0].media = Some(CssText::from_units("screen".encode_utf16().collect()));
        })
        .unwrap();
        assert_eq!(screen.metadata().rules.len(), 1);
        let css = "@starting-style{.x{color:red}}";
        assert!(prepare(css).unwrap().metadata().rules.is_empty());
        assert_eq!(
            prepare_with(css, |_, _, phase| *phase = Some(StylePhase::Starting))
                .unwrap()
                .metadata()
                .rules
                .len(),
            1
        );
        assert!(matches!(
            prepare_with(css, |_, _, phase| *phase = None),
            Err(PreparationError::RequiresStylePhase)
        ));
    }
    #[test]
    fn invalid_layers_are_inactive_without_changing_adjacent_valid_rules() {
        let styles=prepare("@font-face{font-family:Example;src:url(example.woff2)}@layer {.x{color:red}}@layer a,b{.x{color:blue}}@layer 123{.x{color:green}}.x{color:purple}").unwrap();
        assert_eq!(ranked_colors(&styles, false), ["red", "purple"]);
        assert!(matches!(
            prepare_with(".x{color:red}", |_, policy, _| policy.rules = 0),
            Err(PreparationError::RuleCount)
        ));
    }
    #[test]
    fn actual_media_and_supports_preparation_omits_only_inactive_rules() {
        let styles=prepare("#t{color:red}@media(min-width:10000px){#t{color:blue}}@supports(display:grid){#t{color:green}}@supports(display:invented-display){#t{color:purple}}").unwrap();
        assert_eq!(ranked_colors(&styles, false), ["red", "green"]);
        assert_eq!(styles.metadata().compiled.metadata().groups.len(), 3);
    }
    #[test]
    fn globally_inactive_layers_do_not_register_but_target_dependent_layers_do() {
        for global in [
            "@media(width>10000px)",
            "@supports(display:invented-display)",
        ] {
            let styles=prepare(&format!("{global}{{@layer x{{}}}}@layer y,x;@layer x{{#t{{color:red}}}}@layer y{{#t{{color:blue}}}}")).unwrap();
            assert_eq!(ranked_colors(&styles, false), ["blue", "red"]);
        }
        for target in ["@container(width>10000px)", "@scope(.missing)"] {
            let styles=prepare(&format!("{target}{{@layer x{{}}}}@layer y,x;@layer x{{#t{{color:red}}}}@layer y{{#t{{color:blue}}}}")).unwrap();
            assert_eq!(ranked_colors(&styles, false), ["red", "blue"]);
        }
    }
    #[test]
    fn nested_layer_hierarchy_and_unlayered_reverse_for_importance() {
        let styles =
            prepare("@layer app{#t{color:red}@layer nested{#t{color:blue}}}#t{color:green}")
                .unwrap();
        assert_eq!(ranked_colors(&styles, false), ["blue", "red", "green"]);
        assert_eq!(ranked_colors(&styles, true), ["green", "red", "blue"]);
        assert!(styles.metadata().rules[0].layer_rank.normal.len() > 1);
    }
    #[test]
    fn malformed_condition_is_css_recovery_not_a_resource_or_host_capability_fallback() {
        assert!(!Preparer::condition(Err(ConditionEvaluationError::InvalidSyntax)).unwrap());
        assert!(matches!(
            Preparer::condition(Err(ConditionEvaluationError::RequiresFeature(
                "color".to_owned()
            ))),
            Err(PreparationError::Condition(
                ConditionEvaluationError::RequiresFeature(_)
            ))
        ));
        assert!(matches!(
            Preparer::condition(Err(ConditionEvaluationError::Computation(
                ComputedCssError::ResourceLimit
            ))),
            Err(PreparationError::Condition(
                ConditionEvaluationError::Computation(ComputedCssError::ResourceLimit)
            ))
        ));
    }
}

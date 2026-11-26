//! Bulk ABI transport owns primitive facts and results, never JavaScript DOM nodes.
use crate::css_backend::ComputationPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use turbo_dom_core::rtdom::computed_css::{
    computed_values::{ValueContext, ValueLimits},
    container::{
        parse_container_context, ContainerAncestor, ContainerContext, ContainerMeasurements,
        PartialContentSize,
    },
    custom_cascade::CustomCascadeLimits,
    facts::{AnswerDomain, MechanicalAnswer},
    inline_style::InlineLimits,
    preferences::renderer_initial_preferences,
    prepared::PreparedStyles,
    properties::InitialContext,
    resolve::{
        compute_style, compute_style_with_context_answers, ComputedStyle, ResolveLimits,
        TargetContext,
    },
    rule_matches::{
        ContainerMatchContext, ContextAnswers, ContextSelectorAnswers, SelectorAnswers, TargetFacts,
    },
    selector_policy::GeneratedPseudo,
    shadow_routes::{AssignedSlot, ContextRoot, RouteBudget, RouteFacts, ValidatedRouteFacts},
    target_scope::{Boundary, BoundaryAnswers, ContextBoundaryAnswers},
    CssText,
};

use turbo_dom_core::rtdom::css_shorthand::LengthContext;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    abi_version: u32,
    targets: Vec<Target>,
    #[serde(default)]
    context_roots: Vec<ContextRow>,
    domains: Vec<DomainRow>,
    selector_answers: Vec<SelectorRow>,
    scope_answers: Vec<BoundaryRow>,
    #[serde(default)]
    context_answers: Option<ContextAnswerRows>,
    limits: Limits,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    id: u32,
    subject_id: Option<u32>,
    parent_id: Option<u32>,
    path: Vec<u32>,
    implicit_scope_root: u32,
    tree_scope: u32,
    scope_depth: u32,
    pseudo: Option<String>,
    inline: Option<Vec<u16>>,
    context: Context,
    host_of: Option<u32>,
    assigned_slot: Option<SlotRow>,
    container_ancestors: Option<Vec<ContainerRow>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextRow {
    tree_scope: u32,
    root: u32,
    host: Option<u32>,
    parent_tree_scope: Option<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SlotRow {
    slot: u32,
    tree_scope: u32,
    path: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContainerRow {
    id: u64,
    container_name: Option<Vec<u16>>,
    container_type: Option<Vec<u16>>,
    writing_mode: Option<Vec<u16>>,
    unit_bases: Vec<(String, f64)>,
    width_px: Option<f64>,
    height_px: Option<f64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DomainRow {
    id: u32,
    node_ids: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectorRow {
    selector_id: usize,
    item_id: usize,
    root_id: Option<u32>,
    domain_id: u32,
    matches: Vec<u32>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum BoundaryKind {
    Start,
    End,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundaryRow {
    group_id: usize,
    boundary: BoundaryKind,
    root_id: u32,
    domain_id: u32,
    matches: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextAnswerRows {
    selector_answers: Vec<ContextSelectorRow>,
    scope_answers: Vec<ContextBoundaryRow>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextSelectorRow {
    tree_scope: u32,
    selector_id: usize,
    item_id: usize,
    root_id: Option<u32>,
    domain_id: u32,
    matches: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextBoundaryRow {
    tree_scope: u32,
    group_id: usize,
    boundary: BoundaryKind,
    root_id: u32,
    domain_id: u32,
    matches: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    initial_color: String,
    initial_font_family: String,
    initial_font_size: String,
    unit_bases: Vec<(String, f64)>,
    font_size_unit_bases: Vec<(String, f64)>,
    is_root: bool,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Limits {
    targets: usize,
    answer_rows: usize,
    answer_ids: usize,
    path_nodes: usize,
    input_units: usize,
    retained_result_units: usize,
    matching_decisions: usize,
    #[serde(default)]
    routing_contexts: usize,
    inline_declarations: usize,
    inline_properties: usize,
    custom_rollbacks: usize,
    value_retained_units: usize,
    computation: ComputationPolicy,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultRow {
    id: u32,
    values: BTreeMap<String, Vec<u16>>,
    renderer_values: BTreeMap<String, Vec<u16>>,
    public_keys: Vec<String>,
    declared_text_decoration: Option<DeclaredDecoration>,
    custom: BTreeMap<String, Vec<u16>>,
    font_size_px: f64,
    root_font_size_px: f64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredDecoration {
    value: Vec<u16>,
    origin: &'static str,
    important: bool,
    inline: bool,
    order: (usize, usize),
}
fn origin_name(
    origin: turbo_dom_core::rtdom::computed_css::prepared::CascadeOrigin,
) -> &'static str {
    use turbo_dom_core::rtdom::computed_css::prepared::CascadeOrigin;
    match origin {
        CascadeOrigin::UserAgent => "user-agent",
        CascadeOrigin::User => "user",
        CascadeOrigin::Author => "author",
    }
}
fn fail(reason: &str) -> String {
    format!("CSSBackendResolve:{reason}")
}
fn add(total: &mut usize, count: usize, maximum: usize) -> Result<(), String> {
    *total = total
        .checked_add(count)
        .filter(|sum| *sum <= maximum)
        .ok_or_else(|| fail("ResourceLimit"))?;
    Ok(())
}
struct Usage {
    rows: usize,
    ids: usize,
    paths: usize,
    input: usize,
    output: usize,
}
impl Usage {
    fn target(&mut self, target: &Target, limits: Limits) -> Result<(), String> {
        add(&mut self.paths, target.path.len(), limits.path_nodes)?;
        self.containers(target.container_ancestors.as_deref().unwrap_or(&[]), limits)?;
        self.context(&target.context, target.inline.as_deref(), limits)
    }
    fn containers(&mut self, rows: &[ContainerRow], limits: Limits) -> Result<(), String> {
        add(&mut self.paths, rows.len(), limits.path_nodes)?;
        for row in rows {
            for value in [&row.container_name, &row.container_type, &row.writing_mode]
                .into_iter()
                .flatten()
            {
                add(&mut self.input, value.len(), limits.input_units)?;
            }
            for (name, _) in &row.unit_bases {
                add(
                    &mut self.input,
                    name.encode_utf16().count(),
                    limits.input_units,
                )?;
            }
        }
        Ok(())
    }
    fn context(
        &mut self,
        context: &Context,
        inline: Option<&[u16]>,
        limits: Limits,
    ) -> Result<(), String> {
        if let Some(inline) = inline {
            add(&mut self.input, inline.len(), limits.input_units)?;
        }
        for text in [
            &context.initial_color,
            &context.initial_font_family,
            &context.initial_font_size,
        ] {
            add(
                &mut self.input,
                text.encode_utf16().count(),
                limits.input_units,
            )?;
        }
        for (name, _) in context
            .unit_bases
            .iter()
            .chain(&context.font_size_unit_bases)
        {
            add(
                &mut self.input,
                name.encode_utf16().count(),
                limits.input_units,
            )?;
        }
        Ok(())
    }
    fn row_counts(&mut self, request: &Request) -> Result<(), String> {
        let limits = request.limits;
        add(&mut self.rows, request.domains.len(), limits.answer_rows)?;
        add(
            &mut self.rows,
            request.selector_answers.len(),
            limits.answer_rows,
        )?;
        add(
            &mut self.rows,
            request.scope_answers.len(),
            limits.answer_rows,
        )?;
        if let Some(context) = &request.context_answers {
            add(
                &mut self.rows,
                context.selector_answers.len(),
                limits.answer_rows,
            )?;
            add(
                &mut self.rows,
                context.scope_answers.len(),
                limits.answer_rows,
            )?;
        }
        Ok(())
    }
    fn common(&mut self, request: &Request) -> Result<(), String> {
        let limits = request.limits;
        self.row_counts(request)?;
        for domain in &request.domains {
            add(&mut self.ids, domain.node_ids.len(), limits.answer_ids)?;
        }
        for row in &request.selector_answers {
            add(&mut self.ids, row.matches.len(), limits.answer_ids)?;
        }
        for row in &request.scope_answers {
            add(&mut self.ids, row.matches.len(), limits.answer_ids)?;
        }
        self.context_ids(request.context_answers.as_ref(), limits)
    }
    fn context_ids(
        &mut self,
        context: Option<&ContextAnswerRows>,
        limits: Limits,
    ) -> Result<(), String> {
        if let Some(context) = context {
            for row in &context.selector_answers {
                add(&mut self.ids, row.matches.len(), limits.answer_ids)?;
            }
            for row in &context.scope_answers {
                add(&mut self.ids, row.matches.len(), limits.answer_ids)?;
            }
        }
        Ok(())
    }
    fn ordinary_result(&mut self, style: &ComputedStyle, limits: Limits) -> Result<(), String> {
        for (name, value) in style.values.values.iter().chain(style.custom.as_map()) {
            add(
                &mut self.output,
                name.encode_utf16().count(),
                limits.retained_result_units,
            )?;
            add(
                &mut self.output,
                value.as_units().len(),
                limits.retained_result_units,
            )?;
        }
        Ok(())
    }
    fn public_result(&mut self, style: &ComputedStyle, limits: Limits) -> Result<(), String> {
        for name in &style.public_keys {
            add(
                &mut self.output,
                name.encode_utf16().count(),
                limits.retained_result_units,
            )?;
            if let Some(value) = style.values.values.get(name) {
                add(
                    &mut self.output,
                    name.encode_utf16().count(),
                    limits.retained_result_units,
                )?;
                add(
                    &mut self.output,
                    value.as_units().len(),
                    limits.retained_result_units,
                )?;
            }
        }

        Ok(())
    }
    fn result(&mut self, style: &ComputedStyle, limits: Limits) -> Result<(), String> {
        self.ordinary_result(style, limits)?;
        self.public_result(style, limits)?;
        if let Some(marker) = &style.declared_text_decoration {
            add(
                &mut self.output,
                marker.value.as_units().len(),
                limits.retained_result_units,
            )?;
            add(
                &mut self.output,
                origin_name(marker.origin).len(),
                limits.retained_result_units,
            )?;
        }
        Ok(())
    }
}
type Domains = BTreeMap<u32, Arc<AnswerDomain>>;
fn domains(rows: Vec<DomainRow>) -> Result<Domains, String> {
    let mut domains = BTreeMap::new();
    for row in rows {
        let domain = AnswerDomain::new(row.node_ids)
            .map_err(|error| format!("CSSBackendFacts:{error:?}"))?;
        if domains.insert(row.id, domain).is_some() {
            return Err(fail("DuplicateDomain"));
        }
    }
    Ok(domains)
}
fn answer(domains: &Domains, id: u32, matches: Vec<u32>) -> Result<MechanicalAnswer, String> {
    let domain = domains.get(&id).ok_or_else(|| fail("MissingDomain"))?;
    MechanicalAnswer::new(domain.clone(), matches)
        .map_err(|error| format!("CSSBackendFacts:{error:?}"))
}
fn selector_answers(rows: Vec<SelectorRow>, domains: &Domains) -> Result<SelectorAnswers, String> {
    let mut answers = BTreeMap::new();
    for row in rows {
        let value = answer(domains, row.domain_id, row.matches)?;
        if answers
            .insert((row.selector_id, row.item_id, row.root_id), value)
            .is_some()
        {
            return Err(fail("DuplicateSelectorAnswer"));
        }
    }
    Ok(answers)
}
fn boundary_answers(rows: Vec<BoundaryRow>, domains: &Domains) -> Result<BoundaryAnswers, String> {
    let mut answers = BTreeMap::new();
    for row in rows {
        let boundary = match row.boundary {
            BoundaryKind::Start => Boundary::Start,
            BoundaryKind::End => Boundary::End,
        };
        let value = answer(domains, row.domain_id, row.matches)?;
        if answers
            .insert((row.group_id, boundary, row.root_id), value)
            .is_some()
        {
            return Err(fail("DuplicateScopeAnswer"));
        }
    }
    Ok(answers)
}
#[derive(Default)]
struct LegacyAnswers {
    selectors: SelectorAnswers,
    boundaries: BoundaryAnswers,
}
struct OwnedContextAnswers {
    selectors: ContextSelectorAnswers,
    boundaries: ContextBoundaryAnswers,
}
enum AnswerMaps {
    Legacy(LegacyAnswers),
    Context(OwnedContextAnswers),
}
fn ensure_answer_context(
    scope: u32,
    allowed: &std::collections::BTreeSet<u32>,
) -> Result<(), String> {
    if allowed.contains(&scope) {
        Ok(())
    } else {
        Err(fail("UnknownAnswerContext"))
    }
}
fn contextual_selectors(
    rows: Vec<ContextSelectorRow>,
    domains: &Domains,
    allowed: &std::collections::BTreeSet<u32>,
) -> Result<ContextSelectorAnswers, String> {
    let mut answers = BTreeMap::new();
    for row in rows {
        ensure_answer_context(row.tree_scope, allowed)?;
        let value = answer(domains, row.domain_id, row.matches)?;
        if answers
            .insert(
                (row.tree_scope, row.selector_id, row.item_id, row.root_id),
                value,
            )
            .is_some()
        {
            return Err(fail("DuplicateContextSelectorAnswer"));
        }
    }
    Ok(answers)
}
fn contextual_boundaries(
    rows: Vec<ContextBoundaryRow>,
    domains: &Domains,
    allowed: &std::collections::BTreeSet<u32>,
) -> Result<ContextBoundaryAnswers, String> {
    let mut answers = BTreeMap::new();
    for row in rows {
        ensure_answer_context(row.tree_scope, allowed)?;
        let edge = match row.boundary {
            BoundaryKind::Start => Boundary::Start,
            BoundaryKind::End => Boundary::End,
        };
        let value = answer(domains, row.domain_id, row.matches)?;
        if answers
            .insert((row.tree_scope, row.group_id, edge, row.root_id), value)
            .is_some()
        {
            return Err(fail("DuplicateContextScopeAnswer"));
        }
    }
    Ok(answers)
}
fn answer_maps(
    contexts: &[ContextRow],
    selectors: Vec<SelectorRow>,
    boundaries: Vec<BoundaryRow>,
    contextual: Option<ContextAnswerRows>,
    domains: &Domains,
) -> Result<AnswerMaps, String> {
    let Some(contextual) = contextual else {
        return Ok(AnswerMaps::Legacy(LegacyAnswers {
            selectors: selector_answers(selectors, domains)?,
            boundaries: boundary_answers(boundaries, domains)?,
        }));
    };
    if !selectors.is_empty() || !boundaries.is_empty() {
        return Err(fail("MixedAnswerModes"));
    }
    // Prepared rules omit inactive/disabled uses. Declared contexts are validated
    // by the existing per-target route constructor; Core consumes only exact keys.
    let allowed = contexts.iter().map(|context| context.tree_scope).collect();
    Ok(AnswerMaps::Context(OwnedContextAnswers {
        selectors: contextual_selectors(contextual.selector_answers, domains, &allowed)?,
        boundaries: contextual_boundaries(contextual.scope_answers, domains, &allowed)?,
    }))
}
fn routes<'a>(
    target: &'a Target,
    contexts: &'a [ContextRoot],
    budget: &mut RouteBudget,
) -> Result<Option<ValidatedRouteFacts<'a>>, String> {
    if contexts.is_empty() {
        if target.host_of.is_some() || target.assigned_slot.is_some() {
            return Err(fail("MissingRouteContext"));
        }
        return Ok(None);
    }
    let requested_pseudo = match target.pseudo.as_deref() {
        None => None,
        Some("before") => Some(GeneratedPseudo::Before),
        Some("after") => Some(GeneratedPseudo::After),
        Some(_) => return Err(fail("UnsupportedRoutePseudo")),
    };
    let facts = RouteFacts {
        target: target.subject_id.unwrap_or(target.id),
        own_tree_scope: target.tree_scope,
        own_path: &target.path,
        host_of: target.host_of,
        assigned_slot: target.assigned_slot.as_ref().map(|slot| AssignedSlot {
            slot: slot.slot,
            tree_scope: slot.tree_scope,
            path: &slot.path,
        }),
        context_roots: contexts,
        requested_pseudo,
    };
    ValidatedRouteFacts::new(&facts, budget)
        .map(Some)
        .map_err(|error| format!("CSSBackendResolve:Route:{error:?}"))
}
fn container_property(value: Option<&[u16]>, name: &str) -> Result<CssText, String> {
    value
        .map(|units| CssText::from_units(units.to_vec()))
        .or_else(|| turbo_dom_core::rtdom::computed_css::properties::context_free_initial(name))
        .ok_or_else(|| format!("CSSBackendResolve:RequiresInitial({name})"))
}
fn container_contexts(
    rows: &[ContainerRow],
    limits: turbo_dom_core::rtdom::computed_css::ComputationLimits,
) -> Result<Vec<ContainerContext>, String> {
    let mut ids = std::collections::BTreeSet::new();
    rows.iter()
        .map(|row| {
            if !ids.insert(row.id) {
                return Err(fail("DuplicateContainerAncestor"));
            }
            parse_container_context(
                &container_property(row.container_name.as_deref(), "container-name")?,
                &container_property(row.container_type.as_deref(), "container-type")?,
                &container_property(row.writing_mode.as_deref(), "writing-mode")?,
                limits,
            )
            .map_err(|error| format!("CSSBackendResolve:ContainerContext:{error:?}"))
        })
        .collect()
}
fn with_container_context<T>(
    rows: Option<&[ContainerRow]>,
    computation: turbo_dom_core::rtdom::computed_css::ComputationLimits,
    read: impl FnOnce(Option<ContainerMatchContext<'_>>) -> Result<T, String>,
) -> Result<T, String> {
    let Some(rows) = rows else {
        return read(None);
    };
    let parsed = container_contexts(rows, computation)?;
    let units: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            row.unit_bases
                .iter()
                .map(|(name, value)| (name.as_str(), *value))
                .collect()
        })
        .collect();
    let lengths: Vec<_> = units
        .iter()
        .map(|unit_bases| LengthContext { unit_bases })
        .collect();
    let ancestors: Vec<_> = rows
        .iter()
        .zip(&parsed)
        .zip(&lengths)
        .map(|((row, context), length_context)| ContainerAncestor {
            id: row.id,
            names: &context.names,
            kind: context.kind,
            inline_axis: context.inline_axis,
            scroll_state: context.scroll_state,
            size: None,
            length_context,
        })
        .collect();
    let measurements: ContainerMeasurements = rows
        .iter()
        .map(|row| {
            (
                row.id,
                PartialContentSize {
                    width_px: row.width_px,
                    height_px: row.height_px,
                },
            )
        })
        .collect();
    read(Some(ContainerMatchContext {
        ancestors: &ancestors,
        measurements: Some(&measurements),
        limits: computation,
    }))
}
struct StyleInput<'a> {
    answers: &'a AnswerMaps,
    contexts: &'a [ContextRoot],
    route_budget: &'a mut RouteBudget,
}
fn compute_with_answers(
    prepared: &PreparedStyles,
    facts: &TargetFacts<'_>,
    inline: Option<&CssText>,
    parent: Option<&ComputedStyle>,
    context: &TargetContext<'_>,
    limits: ResolveLimits,
    answers: &AnswerMaps,
) -> Result<ComputedStyle, turbo_dom_core::rtdom::computed_css::resolve::ResolveError> {
    match answers {
        AnswerMaps::Legacy(_) => compute_style(prepared, facts, inline, parent, context, limits),
        AnswerMaps::Context(answers) => compute_style_with_context_answers(
            prepared,
            facts,
            inline,
            parent,
            context,
            limits,
            ContextAnswers {
                selectors: &answers.selectors,
                boundaries: &answers.boundaries,
            },
        ),
    }
}
type RendererDefaults = Vec<(&'static str, CssText)>;
fn style(
    mut target: Target,
    prepared: &PreparedStyles,
    parent: Option<&ComputedStyle>,
    limits: Limits,
    input: StyleInput<'_>,
    usage: &mut Usage,
) -> Result<(ComputedStyle, RendererDefaults), String> {
    let StyleInput {
        answers,
        contexts,
        route_budget,
    } = input;
    let empty = LegacyAnswers::default();
    let legacy = match answers {
        AnswerMaps::Legacy(legacy) => legacy,
        AnswerMaps::Context(_) => &empty,
    };
    let inline = target.inline.take().map(CssText::from_units);
    let routes = routes(&target, contexts, route_budget)?;
    let bases: Vec<_> = target
        .context
        .unit_bases
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect();
    let font_bases: Vec<_> = target
        .context
        .font_size_unit_bases
        .iter()
        .map(|(name, value)| (name.as_str(), *value))
        .collect();
    let context = TargetContext {
        tree_scope: target.tree_scope,
        scope_depth: target.scope_depth,
        values: ValueContext {
            initial: InitialContext {
                color: &target.context.initial_color,
                font_family: &target.context.initial_font_family,
                font_size: &target.context.initial_font_size,
            },
            unit_bases: &bases,
            font_size_unit_bases: &font_bases,
            is_root: target.context.is_root,
        },
    };
    let renderer_limits = limits;
    let computation = limits.computation.into();
    let limits = ResolveLimits {
        matching_decisions: limits.matching_decisions,
        inline: InlineLimits {
            computation,
            declarations: limits.inline_declarations,
            properties: limits.inline_properties,
        },
        custom: CustomCascadeLimits {
            computation,
            rollbacks: limits.custom_rollbacks,
        },
        values: ValueLimits {
            computation,
            retained_units: limits.value_retained_units,
        },
    };
    with_container_context(
        target.container_ancestors.as_deref(),
        computation,
        |containers| {
            let facts = TargetFacts {
                path: &target.path,
                implicit_scope_root: target.implicit_scope_root,
                pseudo: target.pseudo.as_deref(),
                selectors: &legacy.selectors,
                boundaries: &legacy.boundaries,
                routes: routes.as_ref(),
                containers,
            };
            let computed = compute_with_answers(
                prepared,
                &facts,
                inline.as_ref(),
                parent,
                &context,
                limits,
                answers,
            )
            .map_err(|error| format!("CSSBackendResolve:{error:?}"))?;
            usage.result(&computed, renderer_limits)?;
            let mut retained: turbo_dom_core::rtdom::computed_css::ComputationLimits =
                renderer_limits.computation.into();
            retained.max_total_units = retained
                .max_total_units
                .min(renderer_limits.retained_result_units - usage.output);
            let extra = renderer_initial_preferences(
                &computed.values.values,
                &context.values.initial,
                retained,
            )
            .map_err(|error| format!("CSSBackendResolve:{error:?}"))?;
            for (name, value) in &extra {
                add(
                    &mut usage.output,
                    name.len() + value.as_units().len(),
                    renderer_limits.retained_result_units,
                )?;
            }
            Ok((computed, extra))
        },
    )
}
fn result_row(id: u32, style: &ComputedStyle, extra: RendererDefaults) -> ResultRow {
    let mut renderer_values: BTreeMap<_, _> = style
        .values
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.as_units().to_vec()))
        .collect();
    for (name, value) in extra {
        renderer_values.insert(name.to_owned(), value.as_units().to_vec());
    }
    let values = style
        .public_keys
        .iter()
        .filter_map(|name| {
            style
                .values
                .values
                .get(name)
                .map(|value| (name.clone(), value.as_units().to_vec()))
        })
        .collect();
    ResultRow {
        id,
        values,
        renderer_values,
        public_keys: style.public_keys.iter().cloned().collect(),
        declared_text_decoration: style.declared_text_decoration.as_ref().map(|marker| {
            DeclaredDecoration {
                value: marker.value.as_units().to_vec(),
                origin: origin_name(marker.origin),
                important: marker.important,
                inline: marker.inline,
                order: marker.order,
            }
        }),
        custom: style
            .custom
            .as_map()
            .iter()
            .map(|(name, value)| (name.clone(), value.as_units().to_vec()))
            .collect(),
        font_size_px: style.values.font_size_px,
        root_font_size_px: style.values.root_font_size_px,
    }
}

struct Batch<'a> {
    prepared: &'a PreparedStyles,
    limits: Limits,
    usage: Usage,
    styles: BTreeMap<u32, ComputedStyle>,
    output: Vec<ResultRow>,
    answers: AnswerMaps,
    contexts: Vec<ContextRoot>,
    route_budget: RouteBudget,
}
impl Batch<'_> {
    fn target(&mut self, target: Target) -> Result<(), String> {
        self.usage.target(&target, self.limits)?;
        let id = target.id;
        if self.styles.contains_key(&id) {
            return Err(fail("DuplicateTarget"));
        }
        let parent = target
            .parent_id
            .map(|id| self.styles.get(&id).ok_or_else(|| fail("MissingParent")))
            .transpose()?;
        let (computed, extra) = style(
            target,
            self.prepared,
            parent,
            self.limits,
            StyleInput {
                answers: &self.answers,
                contexts: &self.contexts,
                route_budget: &mut self.route_budget,
            },
            &mut self.usage,
        )?;
        self.output.push(result_row(id, &computed, extra));
        self.styles.insert(id, computed);
        Ok(())
    }
}
fn new_batch(
    prepared: &PreparedStyles,
    request: Request,
) -> Result<(Batch<'_>, Vec<Target>), String> {
    let mut usage = Usage {
        rows: 0,
        ids: 0,
        paths: 0,
        input: 0,
        output: 0,
    };
    usage.common(&request)?;
    let domains = domains(request.domains)?;
    let answers = answer_maps(
        &request.context_roots,
        request.selector_answers,
        request.scope_answers,
        request.context_answers,
        &domains,
    )?;
    let batch = Batch {
        prepared,
        limits: request.limits,
        usage,
        styles: BTreeMap::new(),
        output: Vec::new(),
        answers,
        contexts: request
            .context_roots
            .into_iter()
            .map(|row| ContextRoot {
                tree_scope: row.tree_scope,
                root: row.root,
                host: row.host,
                parent_tree_scope: row.parent_tree_scope,
            })
            .collect(),
        route_budget: RouteBudget {
            contexts: request.limits.routing_contexts,
            path_units: request.limits.path_nodes,
        },
    };
    Ok((batch, request.targets))
}
pub(super) fn resolve_batch(prepared: &PreparedStyles, source: &str) -> Result<String, String> {
    let request: Request =
        serde_json::from_str(source).map_err(|error| format!("CSSBackendTransport:{error}"))?;
    if request.abi_version != 1 {
        return Err(fail("AbiVersion"));
    }
    if request.targets.len() > request.limits.targets {
        return Err(fail("ResourceLimit"));
    }
    let (mut batch, targets) = new_batch(prepared, request)?;
    for target in targets {
        batch.target(target)?;
    }
    serde_json::to_string(&serde_json::json!({"abiVersion":1,"geometryOnly":prepared.is_geometry_only(),"targets":batch.output}))
        .map_err(|error| format!("CSSBackendTransport:{error}"))
}

#[cfg(test)]
#[path = "css_resolve_context_tests.rs"]
mod context_tests;

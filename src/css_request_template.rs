//! Context references are issued only after the complete request succeeds.
use super::{available_result_bytes, next_token, CachedResult, MatchingSlot, PreparedStyles};
use crate::css_resolve::{apply_matching, fail, parse, Context, ResolveInput, Target};

const MAX_CONTEXTS: usize = 16;
pub(super) struct Template {
    token: String,
    contexts: std::sync::Arc<[Context]>,
    bytes: usize,
}
impl Template {
    pub(super) fn bytes(&self) -> usize {
        self.bytes
    }
}
fn context_bytes(context: &Context) -> usize {
    let strings = context
        .initial_color
        .len()
        .saturating_add(context.initial_font_family.len())
        .saturating_add(context.initial_font_size.len());
    context
        .unit_bases
        .iter()
        .chain(&context.font_size_unit_bases)
        .fold(
            strings.saturating_add(size_of::<bool>()),
            |bytes, (unit, _)| {
                bytes
                    .saturating_add(unit.len())
                    .saturating_add(size_of::<f64>())
            },
        )
}
fn project(target: Target<u32>, context: Context) -> Target {
    Target {
        id: target.id,
        subject_id: target.subject_id,
        parent_id: target.parent_id,
        path: target.path,
        implicit_scope_root: target.implicit_scope_root,
        tree_scope: target.tree_scope,
        scope_depth: target.scope_depth,
        pseudo: target.pseudo,
        inline: target.inline,
        context,
        host_of: target.host_of,
        assigned_slot: target.assigned_slot,
        container_ancestors: target.container_ancestors,
    }
}
impl MatchingSlot {
    pub(crate) fn resolve_request_template(
        &self,
        prepared: &PreparedStyles,
        wire: &str,
    ) -> Result<String, String> {
        let resolved = self.resolve_with_contexts(prepared, wire, true)?;
        let contexts = if resolved.cache_hit {
            #[cfg(test)]
            self.parse_calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Self::cached_contexts(wire)?
        } else {
            resolved.contexts
        };
        let token = contexts
            .map(|contexts| self.install_template(contexts))
            .transpose()?
            .flatten();
        let response = resolved.response;
        serde_json::to_string(&serde_json::json!({"response":response,"templateToken":token}))
            .map_err(|error| format!("CSSBackendTransport:{error}"))
    }
    fn cached_contexts(wire: &str) -> Result<Option<Vec<Context>>, String> {
        // A successful exact full-wire cache hit has no retained typed input.
        let input: ResolveInput = parse(wire)?;
        Ok((input.targets.len() <= MAX_CONTEXTS).then(|| {
            input
                .targets
                .into_iter()
                .map(|target| target.context)
                .collect()
        }))
    }
    fn install_template(&self, contexts: Vec<Context>) -> Result<Option<String>, String> {
        let token = next_token()?;
        let bytes = contexts.iter().fold(token.len(), |bytes, context| {
            bytes.saturating_add(context_bytes(context))
        });
        if bytes > available_result_bytes(self.payload_bytes) {
            return Ok(None);
        }
        let template = Template {
            token: token.clone(),
            contexts: contexts.into(),
            bytes,
        };
        let mut retained = self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let retained = retained.ensure_wire();
        retained
            .cache
            .retain_full(available_result_bytes(self.payload_bytes).saturating_sub(bytes));
        retained.template = Some(template);
        Ok(Some(token))
    }
    pub(crate) fn resolve_template(
        &self,
        prepared: &PreparedStyles,
        wire: &str,
        token: &str,
    ) -> Result<String, String> {
        let contexts = {
            let mut retained = self
                .result
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let retained = retained
                .wire_cache_mut()
                .ok_or_else(|| fail("StaleRequestTemplate"))?;
            if retained
                .template
                .as_ref()
                .is_none_or(|template| template.token != token)
            {
                return Err(fail("StaleRequestTemplate"));
            }
            if let Some(response) = retained.cache.get_template(wire, Some(token)) {
                return Ok(response);
            }
            let template = retained
                .template
                .as_ref()
                .ok_or_else(|| fail("StaleRequestTemplate"))?;
            std::sync::Arc::clone(&template.contexts)
        };
        let compact: ResolveInput<u32> = parse(wire)?;
        compact.validate()?;
        let targets = compact
            .targets
            .into_iter()
            .map(|target| {
                let context = contexts
                    .get(target.context as usize)
                    .ok_or_else(|| fail("ContextReference"))?;
                Ok(project(target, context.clone()))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let input = ResolveInput {
            abi_version: compact.abi_version,
            targets,
            limits: compact.limits,
        };
        let response = apply_matching(prepared, input, &self.facts)?;
        self.retain_template_response(wire, &response, token);
        Ok(response)
    }
    fn retain_template_response(&self, wire: &str, response: &str, token: &str) {
        let mut retained = self
            .result
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // A concurrent state result may have retired the entire wire payload.
        let Some(retained) = retained.wire_cache_mut() else {
            return;
        };
        // A concurrent successful installation may have retired this bundle.
        if retained
            .template
            .as_ref()
            .is_some_and(|template| template.token == token)
        {
            let available = available_result_bytes(self.payload_bytes)
                .saturating_sub(retained.template_bytes());
            let bytes = wire
                .len()
                .saturating_add(response.len())
                .saturating_add(token.len());
            let entry = (bytes <= available).then(|| CachedResult {
                template: Some(token.to_owned()),
                wire: wire.into(),
                response: response.to_owned().into_boxed_str(),
            });
            retained.cache.retain(entry, available);
        }
    }
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    #[test]
    fn state_publication_between_template_phases_drops_late_retention() {
        use crate::css_resolve::{
            native_state::NativeStyleState, AnswerMaps, LegacyAnswers, ValidatedMatching,
        };
        use std::{
            collections::BTreeMap,
            sync::{Arc, Mutex},
        };
        let slot = MatchingSlot {
            facts: ValidatedMatching {
                answers: AnswerMaps::Legacy(LegacyAnswers::default()),
                contexts: Vec::new(),
                rows: 0,
                ids: 0,
            },
            token: "matching".into(),
            payload_bytes: 0,
            result: Mutex::default(),
            cascade_calls: std::sync::atomic::AtomicUsize::default(),
            parse_calls: std::sync::atomic::AtomicUsize::default(),
        };
        let state = Arc::new(NativeStyleState {
            geometry_only: false,
            order: Box::new([]),
            styles: BTreeMap::new(),
            extras: BTreeMap::new(),
        });
        slot.result
            .lock()
            .unwrap()
            .retain_state("state", Arc::clone(&state), 0);
        let before = slot.payload_bytes();
        // A template computation finished after another call published state.
        slot.retain_template_response("late", "result", "retired");
        assert_eq!(slot.payload_bytes(), before);
        assert!(Arc::ptr_eq(
            &slot.result.lock().unwrap().state("state").unwrap(),
            &state
        ));
        // Installation after a previously successful full resolve deliberately
        // publishes its wire mode even if state was published in between.
        let token = slot.install_template(Vec::new()).unwrap().unwrap();
        assert_eq!(Arc::strong_count(&state), 1);
        assert!(slot.result.lock().unwrap().state("state").is_none());
        slot.retain_template_response("late", "result", &token);
        assert!(slot.payload_bytes() <= 64 * 1024);
    }
}

//! Host-only, synchronous sampling of the canonical presentation stack.
use crate::browser_env::{crate_surface::engine_namespace, DomContextId, DomScope};
use serde::Deserialize;

use super::{CssEffectLifecycle, Handle};

/// A canonically sampled property, ready for its native cascade origin.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresentedProperty {
    pub name: String,
    pub value: String,
}

/// Unsupported interpolation is never an underlying-style certificate.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentedTargetState {
    Certified {
        animation: Vec<PresentedProperty>,
        transition: Vec<PresentedProperty>,
    },
    Unknown {
        properties: Vec<String>,
    },
}

/// A native target, including script effects without any CSS animation name.
#[derive(Clone, Debug)]
pub struct PresentedTarget {
    pub element: Handle,
    /// Current declared CSS names, decoded in order; excludes only the `none` keyword.
    pub css_animation_names: Vec<String>,
    pub effects: Vec<CssEffectLifecycle>,
    pub presentation: PresentedTargetState,
    /// Context established by canonical effect ownership/timing, including a
    /// non-filling delay with no sampled property contribution.
    pub establishes_stacking_context: bool,
}

/// Owned data for one synchronous capture, not a reusable cache permission.
#[derive(Clone, Debug)]
pub struct PresentedEffectsSnapshot {
    pub document_id: u64,
    pub sample_time: Option<f64>,
    pub targets: Vec<PresentedTarget>,
    pub css_animations_enabled: bool,
    pub css_transitions_enabled: bool,
    pub presentation_suppressed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    sample_time: Option<f64>,
    css_animations_enabled: bool,
    css_transitions_enabled: bool,
    presentation_suppressed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetData {
    css_animation_names: Vec<String>,
    effects: Vec<CssEffectLifecycle>,
    presentation: PresentedTargetState,
    establishes_stacking_context: bool,
}

#[cfg(test)]
mod context_wire_tests {
    use super::TargetData;

    #[test]
    fn absent_context_evidence_is_not_a_false_certificate() {
        let old = r#"{"effects":[],"presentation":{"status":"certified","animation":[],"transition":[]}}"#;
        assert!(serde_json::from_str::<TargetData>(old).is_err());
        let explicit = r#"{"effects":[],"presentation":{"status":"certified","animation":[],"transition":[]},"css_animation_names":[],"establishes_stacking_context":false}"#;
        assert!(
            !serde_json::from_str::<TargetData>(explicit)
                .unwrap()
                .establishes_stacking_context
        );
    }
}

/// Sample one exact native browsing Document. Candidate handles belong to that
/// document; canonical script/transition targets are discovered independently.
/// No geometry, frame advancement, guest getters or microtask checkpoints run.
/// The isolate and separately attached context queues must use explicit microtasks.
/// Consume synchronously without guest reentry; this owned data is not permission
/// to reuse styles after DOM/CSSOM/effect changes or advancing the host clock.
/// `None` or an Unknown target preserves the embedder's strict unsupported path.
/// Contributions must enter the cascade before inheritance and variable resolution,
/// at Animation or Transition origin respectively. Sample before accepting cache
/// hits: script effect changes need not change a native tree version.
pub fn sample_presented_effects(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    css_candidates: &[Handle],
) -> Option<PresentedEffectsSnapshot> {
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return None;
    }
    let context = document.get_creation_context(scope)?;
    let owner = context
        .get_slot::<DomContextId>()
        .map_or(0, |owner| owner.0);
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, document);
    if !valid_document(scope, document, owner) {
        return None;
    }
    if !crate::browser_env::animation_ownership::owns_all_targets(scope, document, owner) {
        return None;
    }
    let candidates = native_candidates(scope, css_candidates)?;
    v8::tc_scope!(let scope, scope);
    let snapshot = invoke_sampler(scope, document, candidates)
        .and_then(|value| decode_snapshot(scope, value, owner));
    if scope.has_caught() || scope.has_terminated() {
        scope.rethrow();
        return None;
    }
    snapshot
}

fn native_candidates<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    handles: &[Handle],
) -> Option<v8::Local<'s, v8::Array>> {
    let valid = super::with_tree(|tree| {
        handles
            .iter()
            .all(|handle| tree.is_live(*handle) && tree.node_type_id(*handle) == 1)
    })?;
    if !valid {
        return None;
    }
    let candidates: Vec<_> = handles
        .iter()
        .map(|handle| super::wrap(scope, *handle).into())
        .collect();
    Some(v8::Array::new_with_elements(scope, &candidates))
}

fn invoke_sampler<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<v8::Object>,
    candidates: v8::Local<v8::Array>,
) -> Option<v8::Local<'s, v8::Object>> {
    let namespace = engine_namespace(scope)?;
    let function = field(scope, namespace, "samplePresentedEffects")?;
    let function = v8::Local::<v8::Function>::try_from(function).ok()?;
    let value = function.call(
        scope,
        namespace.into(),
        &[document.into(), candidates.into()],
    )?;
    v8::Local::<v8::Object>::try_from(value).ok()
}

fn decode_snapshot(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Object>,
    owner: u64,
) -> Option<PresentedEffectsSnapshot> {
    let metadata = field(scope, value, "metadata")?;
    let header: Header = decode(scope, metadata)?;
    let rows = field(scope, value, "targets")?;
    let rows = v8::Local::<v8::Array>::try_from(rows).ok()?;
    let targets = read_targets(scope, rows, owner)?;
    Some(PresentedEffectsSnapshot {
        document_id: owner,
        sample_time: header.sample_time,
        targets,
        css_animations_enabled: header.css_animations_enabled,
        css_transitions_enabled: header.css_transitions_enabled,
        presentation_suppressed: header.presentation_suppressed,
    })
}

fn valid_document(scope: &mut v8::PinScope, document: v8::Local<v8::Object>, owner: u64) -> bool {
    let context = scope.get_current_context();
    if super::current_document_context(scope) != Some(context) {
        return false;
    }
    if owner != super::current_document_id() || !super::is_browsing_context() {
        return false;
    }
    let Some(handle) = super::handle_of(scope, document) else {
        return false;
    };
    super::with_tree(|tree| handle == tree.root() && tree.node_type_id(handle) == 9)
        .unwrap_or(false)
}

fn field<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Value>> {
    let name = v8::String::new(scope, name)?;
    object.get(scope, name.into())
}

fn decode<T: serde::de::DeserializeOwned>(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Option<T> {
    let value = v8::Local::<v8::String>::try_from(value).ok()?;
    serde_json::from_str(&value.to_rust_string_lossy(scope)).ok()
}

fn read_targets(
    scope: &mut v8::PinScope,
    rows: v8::Local<v8::Array>,
    owner: u64,
) -> Option<Vec<PresentedTarget>> {
    let mut targets = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for index in 0..rows.length() {
        let row = rows.get_index(scope, index)?;
        let target = read_target(scope, row, owner)?;
        if !seen.insert(target.element) {
            return None;
        }
        targets.push(target);
    }
    Some(targets)
}

fn read_target(
    scope: &mut v8::PinScope,
    row: v8::Local<v8::Value>,
    owner: u64,
) -> Option<PresentedTarget> {
    let row = v8::Local::<v8::Array>::try_from(row).ok()?;
    let element = row.get_index(scope, 0)?;
    let element = v8::Local::<v8::Object>::try_from(element).ok()?;
    let handle = native_target_handle(scope, element, owner)?;
    let value = row.get_index(scope, 1)?;
    let data: TargetData = decode(scope, value)?;
    Some(PresentedTarget {
        element: handle,
        css_animation_names: data.css_animation_names,
        effects: data.effects,
        presentation: data.presentation,
        establishes_stacking_context: data.establishes_stacking_context,
    })
}

fn native_target_handle(
    scope: &mut v8::PinScope,
    element: v8::Local<v8::Object>,
    owner: u64,
) -> Option<Handle> {
    if super::document_id_of(scope, element) != owner {
        return None;
    }
    let handle = super::handle_of(scope, element)?;
    let valid = super::with_tree(|tree| tree.is_live(handle) && tree.node_type_id(handle) == 1)?;
    valid.then_some(handle)
}

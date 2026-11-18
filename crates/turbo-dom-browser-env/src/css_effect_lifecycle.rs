//! The private animation engine's bounded, non-interpolating host inspection.
use crate::browser_env::{crate_surface::engine_namespace, DomScope};
use serde::Deserialize;

/// Canonical kind of an effect indexed on the target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CssEffectKind {
    Animation,
    Transition,
    Script,
}

/// Canonical phase, independent of whether the effect fills outside its interval.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CssEffectPhase {
    Idle,
    Before,
    Active,
    After,
}

/// A read of private animation records, never page-overridable Web Animations getters.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssEffectLifecycle {
    pub identity: u64,
    pub name: String,
    pub kind: CssEffectKind,
    pub play_state: String,
    pub phase: CssEffectPhase,
    pub pending: bool,
    /// Canonical timing contribution, before the separate host presentation policy.
    pub contributes: bool,
    pub fill: String,
    pub finite: bool,
    pub replaced: bool,
    pub script_modified: bool,
    /// A finite, naturally completed, forward-playing CSS animation with no fill,
    /// no script overrides and a settled canonical finish task, still owned by this target.
    pub completed_non_filling: bool,
}

/// An instantaneous sample. Synchronizes only this target's underlying cascade and
/// its style dependencies; does not compose effects, resolve transforms, call native
/// layout, flush the document, advance frames, finish or cancel animations.
///
/// The isolate and any separately attached context microtask queue must use explicit
/// checkpoints. The default isolate policy is checked; embedders installing a custom
/// queue must configure that queue as `Explicit` too. No microtask checkpoint runs here.
/// Consume inside one synchronous host capture,
/// without guest JS reentry or advancing the animation host clock. This is not a
/// reusable permission: CSSOM/DOM changes, animation methods, frame tasks and clock
/// advancement require a fresh sample, even if a DOM tree epoch did not change.
#[derive(Clone, Debug)]
pub struct CssEffectSnapshot {
    /// The owning native document and handle, supplied by Rust rather than JS.
    pub document_id: u64,
    pub element: super::Handle,
    /// One pinned canonical host-clock sample, in milliseconds; absent without a host clock.
    pub sample_time: Option<f64>,
    pub animation_names: Vec<String>,
    pub effects: Vec<CssEffectLifecycle>,
    pub css_animations_enabled: bool,
    pub css_transitions_enabled: bool,
    /// Trusted per-Window policy suppresses all CSS and script effect presentation.
    /// This is independent of lifecycle, CSS compatibility switches and ordinary styles.
    pub presentation_suppressed: bool,
    /// A declared timeline/composition unsupported by this narrow proof.
    pub unsupported: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotData {
    sample_time: Option<f64>,
    animation_names: Vec<String>,
    effects: Vec<CssEffectLifecycle>,
    css_animations_enabled: bool,
    css_transitions_enabled: bool,
    presentation_suppressed: bool,
    unsupported: bool,
}

impl CssEffectSnapshot {
    /// Whether the trusted host policy suppresses all effect presentation, or every
    /// declared CSS animation is proven complete and non-filling,
    /// with no other indexed effect. When presentation is enabled, missing rules,
    /// disabled kinds, script effects, transitions and unsupported declarations
    /// fail closed. This says nothing about
    /// transforms or effects on ancestors/descendants: inspect each captured element.
    pub fn allows_underlying_geometry(&self) -> bool {
        self.presentation_suppressed
            || (!self.unsupported
                && self.css_animations_enabled
                && self.css_transitions_enabled
                && self.animation_names.len() == self.effects.len()
                && self
                    .animation_names
                    .iter()
                    .zip(&self.effects)
                    .all(|(name, effect)| name == &effect.name && effect.completed_non_filling))
    }
}

/// Inspect a native element in its creation realm and owning document. `None`
/// means unavailable (including reentry, invalid/non-element wrappers, foreign
/// ownership, a retired context, modified standard builtin/shared/promise shapes or an
/// exception); callers must retain strict guards.
/// Exceptions are caught locally, never translated into an inactive-effect proof.
/// The returned data contains no guest objects and exposes no guest-callable control.
pub fn css_effect_lifecycle(
    scope: &mut v8::PinScope,
    element: v8::Local<v8::Object>,
) -> Option<CssEffectSnapshot> {
    // Auto checkpoints at Function::call's return could run page promise reactions
    // after the sample was built. The host must own its microtask checkpoints.
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return None;
    }
    let context = element.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let _owner = DomScope::for_object(scope, element);
    let handle = super::handle_of(scope, element)?;
    if !super::with_tree(|tree| tree.node_type_id(handle) == 1)? || !super::is_browsing_context() {
        return None;
    }
    let document_id = super::current_document_id();
    v8::tc_scope!(let scope, scope);
    let namespace = engine_namespace(scope)?;
    let name = v8::String::new(scope, "inspectElementCssEffects")?;
    let function = namespace.get(scope, name.into())?;
    let function = v8::Local::<v8::Function>::try_from(function).ok()?;
    let value = function.call(scope, namespace.into(), &[element.into()])?;
    let value = v8::Local::<v8::String>::try_from(value).ok()?;
    let data: SnapshotData = serde_json::from_str(&value.to_rust_string_lossy(scope)).ok()?;
    Some(CssEffectSnapshot {
        document_id,
        element: handle,
        sample_time: data.sample_time,
        animation_names: data.animation_names,
        effects: data.effects,
        css_animations_enabled: data.css_animations_enabled,
        css_transitions_enabled: data.css_transitions_enabled,
        presentation_suppressed: data.presentation_suppressed,
        unsupported: data.unsupported,
    })
}

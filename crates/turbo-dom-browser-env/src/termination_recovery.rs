//! Explicit recovery at an embedder-owned, fully unwound execution boundary.
use crate::browser_env::{
    activate_isolate, interrupted_before_recovery_capture, DomContextId, DomScope,
    TerminationRecovery, DOM_CONTEXTS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationRecoveryError {
    StillTerminating,
    ImplicitMicrotasks,
    ActiveJavaScript,
    StackUnavailable,
    ContextOwnership,
    IsolateUnavailable,
    CallbackUnavailable,
    CleanupFailed,
}

/// Notify every live native DOM realm in this isolate after an interrupted turn.
///
/// The host must have observed/requested termination, joined every watchdog for
/// that turn, allowed the entire guest stack to unwind, and canceled termination.
/// This function never cancels execution, pumps microtasks, advances a clock or
/// dispatches events. Retained and inert realms are included; live effects keep
/// their identities and timing. Errors do not replace the original turn error.
/// The isolate must use explicit microtasks. The host must also configure any
/// independently attached context microtask queues as explicit before calling.
/// A subsequent recovery can retry an incomplete context walk. A fresh realm
/// whose bootstrap actually terminated before recovery callback admission is
/// skipped, not repaired or made publishable: the embedder must still discard
/// that incomplete context as required by `SetupTerminated`. Ready or unknown
/// realms with an unavailable callback remain errors.
pub fn recover_after_termination(scope: &mut v8::PinScope) -> Result<(), TerminationRecoveryError> {
    if scope.is_execution_terminating() {
        return Err(TerminationRecoveryError::StillTerminating);
    }
    if scope.get_microtasks_policy() != v8::MicrotasksPolicy::Explicit {
        return Err(TerminationRecoveryError::ImplicitMicrotasks);
    }
    let stack = v8::StackTrace::current_stack_trace(scope, 1)
        .ok_or(TerminationRecoveryError::StackUnavailable)?;
    if stack.get_frame_count() != 0 {
        return Err(TerminationRecoveryError::ActiveJavaScript);
    }
    // Activate before localizing ANY weak handle: another isolate may have been
    // the most recent native caller on this OS thread.
    if !crate::browser_env::dom_isolation::has_native_world(scope) {
        return Err(TerminationRecoveryError::IsolateUnavailable);
    }
    let _isolate = activate_isolate(scope);
    let contexts = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .iter()
            .map(|(&id, context)| (id, context.clone()))
            .collect::<Vec<_>>()
    });
    let mut visited = Vec::new();
    for (_registered_id, context) in contexts {
        let Some(context) = context.to_local(scope) else {
            continue;
        };
        if visited.contains(&context) {
            continue;
        }
        visited.push(context);
        // Inert Documents share their creator's realm and have alias entries.
        // Validate the canonical realm owner, then clean that realm once.
        let id = context.get_slot::<DomContextId>().map_or(0, |id| id.0);
        let owner = DOM_CONTEXTS.with(|contexts| contexts.borrow().get(&id).cloned());
        if owner.and_then(|owner| owner.to_local(scope)) != Some(context) {
            return Err(TerminationRecoveryError::ContextOwnership);
        }
        let scope = &mut v8::ContextScope::new(scope, context);
        let _document = DomScope::enter(id);
        recover_context(scope, context)?;
    }
    Ok(())
}

fn recover_context(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
) -> Result<(), TerminationRecoveryError> {
    // The registry precedes bootstrap. A newly interrupted, never-admitted
    // realm is incomplete and still must be discarded by the embedder; it has
    // no captured inspection machinery to repair. Missing callbacks in ready
    // or otherwise unknown realms remain errors.
    if context.get_slot::<TerminationRecovery>().is_none()
        && interrupted_before_recovery_capture(context)
    {
        return Ok(());
    }
    let callback = context
        .get_slot::<TerminationRecovery>()
        .and_then(|callback| callback.0.to_local(scope))
        .ok_or(TerminationRecoveryError::CallbackUnavailable)?;
    if callback.get_creation_context(scope) != Some(context) {
        return Err(TerminationRecoveryError::ContextOwnership);
    }
    let receiver = v8::undefined(scope).into();
    callback
        .call(scope, receiver, &[])
        .map(|_| ())
        .ok_or(TerminationRecoveryError::CleanupFailed)
}

//! A V8 startup snapshot of the bootstrapped shell window (turbo-dom#249).
//!
//! The blob is tied to the exact V8 build and flags it was made with, so it is never
//! shipped: it is built at run time, from this process's own V8.
use super::*;
use std::borrow::Cow;

thread_local! {
    static BUILDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether this thread is building a startup snapshot (V8 then refuses code caches).
pub(crate) fn building() -> bool {
    BUILDING.with(std::cell::Cell::get)
}

/// A startup blob and the external reference table it was made with. Every isolate made
/// from the blob must be given exactly this table.
pub(crate) struct Blob {
    pub(crate) data: v8::StartupData,
    pub(crate) references: &'static [v8::ExternalReference],
}

/// Run a full engine bootstrap in a throwaway isolate so the external reference table holds
/// every callback an installed window reaches.
fn record_references() {
    let isolate = &mut v8::Isolate::new(Default::default());
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        bootstrap_engine(scope);
    }
    reset();
}

/// Build a startup blob whose default context is a bootstrapped shell window. `None` when
/// the bootstrap in the snapshot isolate handed V8 a callback the recording pass did not
/// (the blob would abort V8 on serialization).
pub(crate) fn build() -> Option<Blob> {
    record_references();
    let references: &'static [v8::ExternalReference] =
        Box::leak(external_refs::table().into_boxed_slice());
    let mut isolate = v8::Isolate::snapshot_creator(Some(Cow::Borrowed(references)), None);
    BUILDING.with(|building| building.set(true));
    external_refs::begin_build_log();
    {
        v8::scope!(let scope, &mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        bootstrap_engine(scope);
        strip_wrappers(scope);
        // The slot annex is a heap pointer (and a Weak) in the context's embedder data:
        // serialized, a context made from the blob would read the freed annex.
        context.clear_all_slots();
        scope.set_default_context(context);
    }
    reset();
    BUILDING.with(|building| building.set(false));
    if !external_refs::end_build_log(references) {
        // A creator isolate can only be disposed through create_blob, which would abort on
        // the unregistered callback: leak it (a bootstrap that took a code path the
        // recording pass did not; never expected).
        std::mem::forget(isolate);
        return None;
    }
    let data = isolate.create_blob(v8::FunctionCodeHandling::Keep)?;
    Some(Blob { data, references })
}

/// Rehydrate the current context, deserialized from a [`build`] blob (turbo-dom#249).
pub(crate) fn rehydrate(_scope: &mut v8::PinScope) -> bool {
    false
}

/// Clear the node wrappers' internal fields (handle id, owner tag) before serialization.
/// They are Smis, which V8's snapshot serializer cannot tell from embedder pointers; and
/// the ids name nodes of a tree that does not outlive the build.
fn strip_wrappers(scope: &mut v8::PinScope) {
    let mut wrappers = Vec::new();
    DOM.with(|dom| {
        if let Some(state) = dom.borrow().as_ref() {
            wrappers.extend(state.cache.values().cloned());
        }
    });
    OTHER_DOMS.with(|doms| {
        for saved in doms.borrow().values() {
            if let Some(state) = &saved.dom {
                wrappers.extend(state.cache.values().cloned());
            }
        }
    });
    let undefined = v8::undefined(scope);
    for wrapper in wrappers {
        if let Some(wrapper) = wrapper.to_local(scope) {
            wrapper.set_internal_field(0, undefined.into());
            wrapper.set_internal_field(1, undefined.into());
        }
    }
}

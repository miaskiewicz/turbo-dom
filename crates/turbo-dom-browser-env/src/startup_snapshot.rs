//! A V8 startup snapshot of the bootstrapped shell window (turbo-dom#249).
//!
//! The blob is tied to the exact V8 build and flags it was made with, so it is never
//! shipped: it is built at run time, from this process's own V8.
use super::*;
use std::borrow::Cow;

#[cfg(test)]
thread_local! {
    /// Test hook: how many of them were still reachable when the last build serialized.
    pub(crate) static RETAINED_FOR_TEST: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Test hook: make the next build on this thread fail at one step before `create_blob`.
    pub(crate) static FAIL_BUILD_FOR_TEST: std::cell::Cell<Option<BuildFailure>> = const { std::cell::Cell::new(None) };
}

/// A build failure [`FAIL_BUILD_FOR_TEST`] can force.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum BuildFailure {
    /// Isolate data added before the record key, so `persist_dom_state` finds the key at
    /// another index.
    IsolateData,
    /// A panic in the bootstrap.
    Panic,
    /// Page state keeps a shell node wrapper (`document.body`) reachable.
    RetainWrapper,
}

#[cfg(test)]
fn forced_failure(scope: &mut v8::PinScope, at: BuildFailure) {
    let Some(failure) = FAIL_BUILD_FOR_TEST.with(std::cell::Cell::get) else {
        return;
    };
    match (failure, at) {
        (BuildFailure::IsolateData, BuildFailure::IsolateData) => {
            let filler = v8::Private::new(scope, None);
            scope.add_isolate_data(filler);
        }
        (BuildFailure::Panic, BuildFailure::Panic) => panic!("forced snapshot build panic"),
        (BuildFailure::RetainWrapper, BuildFailure::RetainWrapper) => {
            let source = v8::String::new(scope, "globalThis.__kept = document.body").unwrap();
            v8::Script::compile(scope, source, None)
                .and_then(|script| script.run(scope))
                .unwrap();
        }
        _ => {}
    }
}

thread_local! {
    static BUILDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The FNV-1a offset basis the engine-source hash starts from.
pub(crate) const SOURCES_SEED: u64 = 0xcbf2_9ce4_8422_2325;
thread_local! {
    /// The running engine-source hash while this thread records references (`None` otherwise).
    static RECORDING_SOURCES: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}
static ENGINE_SOURCES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(SOURCES_SEED);
/// The hash of every engine script the last reference-recording bootstrap ran, in order
/// ([`snapshot_references`] folds it into the blob fingerprint).
pub(crate) fn engine_sources_hash() -> u64 {
    ENGINE_SOURCES.load(std::sync::atomic::Ordering::Relaxed)
}
/// Fold one engine script into the hash while a recording bootstrap runs (every engine script
/// passes here: `run_engine_script_with`); free otherwise.
pub(crate) fn note_engine_source(source: &str) {
    RECORDING_SOURCES.with(|hash| {
        if let Some(mut value) = hash.get() {
            for byte in source.bytes() {
                value = (value ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
            }
            hash.set(Some(value));
        }
    });
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
        RECORDING_SOURCES.with(|hash| hash.set(Some(SOURCES_SEED)));
        bootstrap_engine(scope);
        let sources = RECORDING_SOURCES
            .with(|hash| hash.take())
            .unwrap_or(SOURCES_SEED);
        ENGINE_SOURCES.store(sources, std::sync::atomic::Ordering::Relaxed);
    }
    reset();
}

/// The external reference table a [`build_with`] blob is made with: one throwaway bootstrap
/// records it (as a build does first). With nothing recorded before, its order is the
/// bootstrap's, the same in every process of one binary.
pub(crate) fn references() -> &'static [v8::ExternalReference] {
    record_references();
    Box::leak(external_refs::table().into_boxed_slice())
}

/// Build a startup blob whose default context is a bootstrapped shell window. `None` (and a
/// log line; installs then run the engine scripts) when the build fails before `create_blob`:
/// the bootstrap in the snapshot isolate handed V8 a callback the recording pass did not (the
/// blob would abort V8 on serialization), a shell node wrapper stayed reachable, the window's
/// native state could not be persisted, or the build panicked.
///
/// A snapshot creator can only be disposed through `create_blob`: rusty_v8 aborts the
/// process when one is dropped, including during a panic's unwind. So no failure may unwind
/// past it: each one leaks the creator instead (a panic is caught for that).
pub(crate) fn build() -> Option<Blob> {
    build_from(None)
}

/// [`build`] for an embedder with its own startup context: the blob's default context is
/// `prelude` alone, and context [`SNAPSHOT_WINDOW_CONTEXT`] ran `prelude`, then the engine
/// bootstrap (the order of a prelude, then an install). The prelude must hand V8 no native
/// callback (none is in the reference table).
pub(crate) fn build_with(prelude: fn(&mut v8::PinScope)) -> Option<Blob> {
    build_from(Some(prelude))
}

fn build_from(prelude: Option<fn(&mut v8::PinScope)>) -> Option<Blob> {
    record_references();
    let references: &'static [v8::ExternalReference] =
        Box::leak(external_refs::table().into_boxed_slice());
    // Anchor V8's process-shared read-only heap on this thread for the snapshot
    // creator's lifetime (turbo-dom#392). V8 initializes the shared RO heap from
    // whichever isolate first touches it in the process: a snapshot creator started
    // with no default-snapshot isolate alive serializes read-only references into a
    // creator-shaped layout, and any blob isolate deserialized later alongside a
    // default-snapshot isolate (a worker, a leaked creator from a failed build) then
    // reads the shared layout and crashes in `Deserializer::ReadReadOnlyHeapRef`.
    // Keeping a default-snapshot isolate alive here forces the shared layout before
    // the creator reads from it, so blobs are compatible with any later isolate.
    let _ro_anchor = v8::Isolate::new(Default::default());
    let mut isolate = v8::Isolate::snapshot_creator(Some(Cow::Borrowed(references)), None);
    BUILDING.with(|building| building.set(true));
    external_refs::begin_build_log();
    let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare(&mut isolate, prelude)
    }));
    reset();
    let failure = match prepared {
        Ok(Ok(stripped)) => {
            let retained = still_reachable(&mut isolate, stripped);
            #[cfg(test)]
            RETAINED_FOR_TEST.with(|count| count.set(Some(retained)));
            (retained > 0).then_some("a shell node wrapper is still reachable")
        }
        Ok(Err(reason)) => Some(reason),
        Err(_) => Some("the build panicked"),
    };
    BUILDING.with(|building| building.set(false));
    let registered = external_refs::end_build_log(references);
    let failure = failure.or((!registered).then_some("an unregistered callback"));
    if let Some(reason) = failure {
        // Never expected: a bootstrap that took a code path the recording pass did not, or an
        // engine closure that kept a stripped node wrapper (its rehydrated copy would name no
        // node), or a broken build invariant.
        eprintln!(
            "[rust-dom] startup snapshot not built ({reason}): installs run the engine scripts"
        );
        std::mem::forget(isolate);
        return None;
    }
    let data = isolate.create_blob(v8::FunctionCodeHandling::Keep)?;
    Some(Blob { data, references })
}

/// The snapshot isolate's default context: the bootstrapped shell window, its native state
/// persisted into the heap and its node wrappers stripped (returned, see [`strip_wrappers`]).
fn prepare(
    isolate: &mut v8::OwnedIsolate,
    prelude: Option<fn(&mut v8::PinScope)>,
) -> Result<Vec<v8::Weak<v8::Object>>, &'static str> {
    v8::scope!(let scope, isolate);
    if let Some(prelude) = prelude {
        // The embedder's own default context: its prelude, no window.
        let plain = v8::Context::new(scope, Default::default());
        {
            let scope = &mut v8::ContextScope::new(scope, plain);
            prelude(scope);
        }
        scope.set_default_context(plain);
    }
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    #[cfg(test)]
    forced_failure(scope, BuildFailure::IsolateData);
    if let Some(prelude) = prelude {
        prelude(scope);
    }
    bootstrap_engine(scope);
    #[cfg(test)]
    forced_failure(scope, BuildFailure::Panic);
    #[cfg(test)]
    forced_failure(scope, BuildFailure::RetainWrapper);
    persist_dom_state(scope)?;
    range_live::persist(scope);
    let stripped = strip_wrappers(scope);
    // The slot annex is a heap pointer (and a Weak) in the context's embedder data:
    // serialized, a context made from the blob would read the freed annex.
    context.clear_all_slots();
    if prelude.is_some() {
        let index = scope.add_context(context);
        debug_assert_eq!(index, SNAPSHOT_WINDOW_CONTEXT);
    } else {
        scope.set_default_context(context);
    }
    Ok(stripped)
}

/// Rehydrate the current context, deserialized from a [`build`] blob (turbo-dom#249).
///
/// Rebuilds what the bootstrap left outside the V8 heap: the function-holding context slots
/// (`rehydrate_context`), the isolate's collection-record key (carried as isolate data),
/// DOM isolation and GC tracking, and the window document's native state (`DomState` over
/// the shell tree, a fresh element template, the identity-cache and ownership-graph entry
/// of the document wrapper, whose internal fields `strip_wrappers` cleared, and the engine
/// helpers its install retains). The isolate slots the binding creates on first use
/// (mutation, Range and storage registries, reaction frames, private-key caches) need
/// nothing. Embedder capabilities stay the embedder's to register again on the new
/// context, as after any install: the activation host (`set_activation_host`), the event
/// target observer, timers (`capture_popover_scheduler`), the view-transition and file
/// hosts, computed and canvas fonts. `false` when the context has no snapshotted window
/// document (it was not made from a [`build`] blob).
pub(crate) fn rehydrate(scope: &mut v8::PinScope) -> bool {
    let context = scope.get_current_context();
    let Some(document) = window_document(scope, context) else {
        return false;
    };
    collection_records::rehydrate(scope, collection_records::SNAPSHOT_DATA);
    rehydrate_context(scope, context);
    dom_isolation::install(scope);
    dom_ownership::track_gc(scope);
    // The install-time isolate keys (registered privates, so the same keys again).
    shadow_html::ensure_keys(scope);
    shadow_slots::ensure_keys(scope);
    reference_target::ensure_key(scope);
    STYLE.with(|s| s.borrow_mut().clear());
    LISTENERS.with(|s| s.borrow_mut().clear());
    ACTIVE.with(|s| *s.borrow_mut() = None);
    let el_template = {
        let template = build_el_template(scope);
        v8::Global::new(scope, template)
    };
    let tree = Tree::parse_with_processing_instructions(SHELL_HTML);
    let root = tree.root();
    let mut state = window_dom_state(tree, Vec::new(), el_template);
    state.basics_state = stored::<v8::Object>(scope, context, BASICS_STATE)
        .map(|object| v8::Weak::new(scope, object));
    DOM.with(|d| *d.borrow_mut() = Some(state));
    let weak = v8::Weak::new(scope, context);
    DOM_CONTEXTS.with(|d| {
        d.borrow_mut()
            .insert(DOM_ID.with(std::cell::Cell::get), weak);
    });
    form_checked_state::parsed(root);
    let handle = v8::Integer::new_from_unsigned(scope, root.raw());
    document.set_internal_field(0, handle.into());
    let owner = owner_tag(scope, DOM_ID.with(std::cell::Cell::get));
    document.set_internal_field(1, owner.into());
    let weak = v8::Weak::new(scope, document);
    DOM.with(|d| {
        // invariant: set above
        d.borrow_mut().as_mut().unwrap().cache.insert(root, weak);
    });
    dom_ownership::observe(scope, root, document);
    capture_control_clone(scope);
    html_popover::capture(scope);
    template_contents::rehydrate(scope);
    range_live::rehydrate(scope);
    true
}

/// The captured-function store name of the document's `basics_state` object.
const BASICS_STATE: &str = "DomState::basics_state";

/// The window document the bootstrap bound to the context's global.
fn window_document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
) -> Option<v8::Local<'s, v8::Object>> {
    let context = v8::Local::new(scope, context);
    let name = v8::String::new(scope, "turbo-dom.ownership.window-document")?;
    let key = v8::Private::for_api(scope, Some(name));
    let value = context.global(scope).get_private(scope, key)?;
    v8::Local::<v8::Object>::try_from(value).ok()
}

/// A value [`persist_dom_state`] stored.
fn stored<'s, T>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
    name: &str,
) -> Option<v8::Local<'s, T>>
where
    v8::Local<'s, T>: TryFrom<v8::Local<'s, v8::Value>>,
{
    let object = context_rehydrate::store_object(scope, context)?;
    let key = v8::String::new(scope, name)?;
    let value = object.get(scope, key.into())?;
    v8::Local::<T>::try_from(value).ok()
}

/// Keep the window document's heap-held native state where [`rehydrate`] finds it: its
/// `basics_state` object in the captured-function store, the collection-record key as
/// isolate data.
/// `Err` when the record key is not the isolate's only data ([`rehydrate`] reads it at
/// `collection_records::SNAPSHOT_DATA`).
fn persist_dom_state(scope: &mut v8::PinScope) -> Result<(), &'static str> {
    let basics = DOM.with(|d| d.borrow().as_ref()?.basics_state.clone());
    if let Some(basics) = basics.and_then(|weak| weak.to_local(scope)) {
        context_rehydrate::store(scope, BASICS_STATE, basics.into());
    }
    if let Some(key) = collection_records::key(scope) {
        if scope.add_isolate_data(key) != collection_records::SNAPSHOT_DATA {
            return Err("the record key is not the only isolate data");
        }
    }
    Ok(())
}

/// Clear the node wrappers' internal fields (handle id, owner tag) before serialization.
/// They are Smis, which V8's snapshot serializer cannot tell from embedder pointers; and
/// the ids name nodes of a tree that does not outlive the build.
///
/// Only the window document's wrapper is relinked by [`rehydrate`], so every other node
/// wrapper must stay out of the blob: the document's ownership edges to its connected nodes
/// are released here (as `attach_document` does), and the returned wrappers (every one but
/// the document's) are checked unreachable once the build's native state is gone.
fn strip_wrappers(scope: &mut v8::PinScope) -> Vec<v8::Weak<v8::Object>> {
    let mut wrappers = Vec::new();
    let (root, graph) = DOM.with(|dom| {
        let dom = dom.borrow();
        // invariant: bootstrap_engine installed a DOM
        let state = dom.as_ref().expect("the bootstrap installed a DOM");
        wrappers.extend(state.cache.values().cloned());
        (state.tree.root(), state.ownership.clone())
    });
    OTHER_DOMS.with(|doms| {
        for saved in doms.borrow().values() {
            if let Some(state) = &saved.dom {
                wrappers.extend(state.cache.values().cloned());
            }
        }
    });
    let document = DOM.with(|dom| dom.borrow().as_ref()?.cache.get(&root).cloned());
    let document = document.and_then(|weak| weak.to_local(scope));
    if let Some(document) = document {
        template_contents::forget_associated(scope, document);
    }
    if let Some(graph) = graph {
        graph.borrow_mut().retire_nodes(scope, root.raw());
    }
    let undefined = v8::undefined(scope);
    let mut stripped = Vec::new();
    for weak in wrappers {
        if let Some(wrapper) = weak.to_local(scope) {
            wrapper.set_internal_field(0, undefined.into());
            wrapper.set_internal_field(1, undefined.into());
            if document != Some(wrapper) {
                stripped.push(weak);
            }
        }
    }
    stripped
}

/// How many of the `stripped` wrappers survive a full GC (none may reach the blob).
fn still_reachable(isolate: &mut v8::OwnedIsolate, stripped: Vec<v8::Weak<v8::Object>>) -> usize {
    v8::scope!(let scope, isolate);
    scope.low_memory_notification();
    stripped.iter().filter(|weak| !weak.is_empty()).count()
}

//! Weak native lookups with V8-private document and subtree ownership.
use super::*;
#[path = "realm_ownership.rs"]
mod graph;
pub(super) type Graph = std::rc::Rc<std::cell::RefCell<graph::DocumentGraph>>;

pub(super) fn helper_function<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    function: v8::Local<'_, v8::Function>,
) -> Option<v8::Weak<v8::Function>> {
    let function = v8::Local::new(scope, function);
    retain_helper(scope, function.into())?;
    Some(v8::Weak::new(scope, function))
}
pub(super) fn helper_object<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'_, v8::Object>,
) -> Option<v8::Weak<v8::Object>> {
    let object = v8::Local::new(scope, object);
    retain_helper(scope, object.into())?;
    Some(v8::Weak::new(scope, object))
}
#[cfg(test)]
thread_local! {
    /// Test-only watchdog: the number of helpers still retained before execution terminates
    /// (at the next one), so a test can terminate at every helper an install retains.
    pub(super) static TERMINATE_AT_HELPER: std::cell::Cell<Option<usize>> =
        const { std::cell::Cell::new(None) };
}
/// Test-only: terminate at the helper `TERMINATE_AT_HELPER` counts down to, and let the
/// interrupt land as an embedder's watchdog does mid-install.
#[cfg(test)]
fn helper_checkpoint(scope: &mut v8::PinScope) {
    let Some(remaining) = TERMINATE_AT_HELPER.with(|at| at.get()) else {
        return;
    };
    TERMINATE_AT_HELPER.with(|at| at.set(remaining.checked_sub(1)));
    if remaining == 0 {
        scope.terminate_execution();
        let code = v8::String::new(scope, "0").unwrap();
        let script = v8::Script::compile(scope, code, None);
        assert!(script.and_then(|script| script.run(scope)).is_none());
    }
}
fn retain_helper<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Option<()> {
    #[cfg(test)]
    helper_checkpoint(scope);
    if scope.is_execution_terminating() {
        return None;
    }
    let graph = DOM.with(|state| state.borrow().as_ref()?.ownership.clone())?;
    // V8 may materialize termination while allocating the private edge. Keep
    // that exception pending and let the installer abort without publishing it.
    let retained = graph.borrow_mut().retain_helper(scope, value);
    retained
}
pub(super) trait NativeHandle<T> {
    fn local<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, T>>;
}
impl<T> NativeHandle<T> for &v8::Global<T> {
    fn local<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, T>> {
        Some(v8::Local::new(scope, self))
    }
}
impl<T> NativeHandle<T> for v8::Global<T> {
    fn local<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, T>> {
        Some(v8::Local::new(scope, self))
    }
}
impl<T> NativeHandle<T> for &v8::Weak<T> {
    fn local<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, T>> {
        self.to_local(scope)
    }
}
impl<T> NativeHandle<T> for v8::Weak<T> {
    fn local<'s>(self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, T>> {
        self.to_local(scope)
    }
}
pub(super) fn local<'s, T>(
    scope: &mut v8::PinScope<'s, '_>,
    handle: impl NativeHandle<T>,
) -> v8::Local<'s, T> {
    handle
        .local(scope)
        .expect("native caller retains its document owner")
}
pub(super) fn observe<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    handle: Handle,
    object: v8::Local<'s, v8::Object>,
) {
    // invariant: an installed DOM tree is in scope
    let root = with_tree(Tree::root).unwrap();
    let graph = DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.ownership.clone())
    });
    let graph = match graph {
        Some(graph) => graph,
        None => {
            if handle != root {
                return;
            }
            let graph = std::rc::Rc::new(std::cell::RefCell::new(
                // invariant: value produced by engine code, never page input
                graph::DocumentGraph::new(scope, object).unwrap(),
            ));
            // invariant: natives run under the DomScope of an installed DOM
            DOM.with(|state| state.borrow_mut().as_mut().unwrap().ownership = Some(graph.clone()));
            graph
        }
    };
    let composed = with_tree(|tree| {
        let mut root = handle;
        while let Some(parent) = tree
            .parent(root)
            .or_else(|| tree.attribute_owner(root))
            .or_else(|| tree.shadow_host(root))
        {
            root = parent;
        }
        root
    })
    // invariant: value produced by engine code, never page input
    .unwrap();
    graph.borrow_mut().observe_component(
        scope,
        handle.raw(),
        object,
        composed.raw(),
        composed == root,
    );
}
pub(super) fn reconcile_current(scope: &mut v8::PinScope) {
    let Some((graph, roots, connected)) = DOM.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let graph = state.ownership.clone()?;
        let size = state.cache.len();
        let mut roots = rustc_hash::FxHashMap::with_capacity_and_hasher(size, Default::default());
        let mut connected =
            rustc_hash::FxHashSet::with_capacity_and_hasher(size, Default::default());
        #[cfg(test)]
        WORK.with(|work| {
            work.wrappers_visited
                .set(work.wrappers_visited.get() + state.cache.len())
        });
        // Every node walked on the way up shares the root found: memoized for this pass
        // (cached wrappers and their uncached ancestors), each edge is walked once.
        let mut found: rustc_hash::FxHashMap<u32, u32> =
            rustc_hash::FxHashMap::with_capacity_and_hasher(size * 2, Default::default());
        let mut path = Vec::new();
        for handle in state.cache.keys() {
            let mut node = *handle;
            let root = loop {
                if let Some(root) = found.get(&node.raw()) {
                    break *root;
                }
                path.push(node.raw());
                match state
                    .tree
                    .parent(node)
                    .or_else(|| state.tree.attribute_owner(node))
                    .or_else(|| state.tree.shadow_host(node))
                {
                    Some(parent) => {
                        #[cfg(test)]
                        WORK.with(|work| work.root_steps.set(work.root_steps.get() + 1));
                        node = parent;
                    }
                    None => break node.raw(),
                }
            };
            for walked in path.drain(..) {
                found.insert(walked, root);
            }
            roots.insert(handle.raw(), root);
            if root == state.tree.root().raw() {
                connected.insert(handle.raw());
            }
        }
        Some((graph, roots, connected))
    }) else {
        return;
    };
    let mut graph = graph.borrow_mut();
    if !graph.document_alive() {
        return;
    }
    graph.sweep();
    if graph.reconcile(scope, &connected).is_none() {
        return;
    }
    // invariant: value produced by engine code, never page input
    graph.reconcile_components(scope, |id| *roots.get(&id).unwrap());
}

/// Work counters for complexity tests (turbo-dom#112): wrappers walked by `reconcile_current`
/// and collected-listener/document sweeps run by `reconcile_all`.
#[cfg(test)]
#[derive(Default)]
pub(super) struct Work {
    pub(super) wrappers_visited: std::cell::Cell<usize>,
    pub(super) sweeps: std::cell::Cell<usize>,
    /// Wrappers (re)placed in an ownership component or re-retained by their document.
    pub(super) placements: std::cell::Cell<usize>,
    /// Parent steps `reconcile_current` took to find the wrappers' roots.
    pub(super) root_steps: std::cell::Cell<usize>,
}
#[cfg(test)]
thread_local! {
    pub(super) static WORK: Work = Work::default();
}

pub(super) fn link_style<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    style: v8::Local<'s, v8::Object>,
) {
    let name = v8::String::new(scope, "turbo-dom.ownership.inline-style").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    node.set_private(scope, key, style.into());
    let name = v8::String::new(scope, "turbo-dom.ownership.style-owner").unwrap();
    let key = v8::Private::for_api(scope, Some(name));
    style.set_private(scope, key, node.into());
}
/// What `pin_callback_owners` did for one native call: how many owner documents it pinned
/// (as Locals in the call's handle scope) and whether the call's wrappers were pinned for a
/// deferred reconcile (see `reconcile_after_call`).
pub(super) struct CallOwners {
    documents: usize,
    deferrable: bool,
}
#[cfg(test)]
impl CallOwners {
    /// Owner documents this call pinned.
    pub(super) fn documents(&self) -> usize {
        self.documents
    }
}

/// Distinct owner documents `pin_callback_owners` deduplicates before it stops bothering: a
/// call names a handful of nodes, and pinning a document twice is harmless.
const OWNER_SLOTS: usize = 8;

/// Runs before every native DOM method and setter, so it does no heap allocation
/// (turbo-test#127): owners are deduplicated in a fixed array and each document is pinned by
/// localizing it, which keeps it alive for the call's handle scope.
pub(super) fn pin_callback_owners<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: v8::Local<'s, v8::Object>,
    arguments: impl IntoIterator<Item = v8::Local<'s, v8::Value>>,
) -> CallOwners {
    let current = DOM_ID.with(|id| id.get());
    let deferrable = large_tree();
    let mut seen = [current; OWNER_SLOTS];
    let mut distinct = 1;
    let mut documents = usize::from(pin_document(scope, current, current));
    for value in std::iter::once(receiver.into()).chain(arguments) {
        let Ok(object) = v8::Local::<v8::Object>::try_from(value) else {
            continue;
        };
        if object.internal_field_count() != 2 || handle_of(scope, object).is_none() {
            continue;
        }
        if deferrable {
            pin_until_reconciled(scope, object);
        }
        let Some(Some(owner)) = super::tagged_owner(scope, object) else {
            continue;
        };
        if seen[..distinct].contains(&owner) {
            continue;
        }
        if distinct < OWNER_SLOTS {
            seen[distinct] = owner;
            distinct += 1;
        }
        documents += usize::from(pin_document(scope, owner, current));
    }
    CallOwners {
        documents,
        deferrable,
    }
}

/// Localize `owner`'s document wrapper in the caller's handle scope (Source documents can lose
/// their last node edge during adoption; this keeps them alive through the call).
fn pin_document(scope: &mut v8::PinScope, owner: u64, current: u64) -> bool {
    let document = if owner == current {
        DOM.with(|state| {
            let state = state.borrow();
            let state = state.as_ref()?;
            state.cache.get(&state.tree.root())?.to_local(scope)
        })
    } else {
        OTHER_DOMS.with(|states| {
            let states = states.borrow();
            let state = states.get(&owner)?.dom.as_ref()?;
            state.cache.get(&state.tree.root())?.to_local(scope)
        })
    };
    document.is_some()
}

/// Up to this many live wrappers a mutating DOM call reconciles ownership itself (exact
/// per-call ownership, what every small GC-exactness test exercises). A reconcile walks every
/// wrapper and rebuilds a V8 Map per ownership component, so doing it after EVERY call made
/// building n nodes O(n^2) (turbo-dom#112; approach from turbo-test#105).
const EAGER_WRAPPER_LIMIT: usize = 256;
/// Past the limit, reconcile once per `wrappers / DEFER_DIVISOR` mutating calls: amortised
/// O(DEFER_DIVISOR) wrapper visits per call instead of O(wrappers).
const DEFER_DIVISOR: usize = 32;
/// Bound on wrappers pinned while reconciles are deferred.
const MAX_PINNED: usize = 4096;

thread_local! {
    static DEFERRED_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Wrappers (receivers and arguments) of calls whose reconcile is deferred. A call can attach
    /// a node whose wrapper (and, through its component edges, the subtree it owns) only the
    /// pending reconcile would hand to the document; until then this keeps it strongly reachable,
    /// so a GC between the mutation and the reconcile cannot drop its identity or expandos (e.g.
    /// React's fiber pointers). Deferral only ever delays releases, never an edge it needs.
    static DEFERRED_PINS: std::cell::RefCell<Vec<v8::Global<v8::Object>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn large_tree() -> bool {
    DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.cache.len() > EAGER_WRAPPER_LIMIT)
    })
}

/// Keep `object` strongly reachable until the next `reconcile_all` (the end of this call when
/// it reconciles eagerly). Its ownership component comes along: every wrapper placed with it,
/// e.g. the wrappers a call creates in a detached source fragment before moving its children
/// elsewhere. Only a reconcile re-places those moved wrappers; until then nothing else
/// reaches them (turbo-dom#324).
pub(super) fn pin_until_reconciled(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) {
    let global = v8::Global::new(scope, object);
    DEFERRED_PINS.with(|pins| pins.borrow_mut().push(global));
}

/// Epilogue of every mutating native call (`ownership_callback` / `ownership_setter`):
/// reconcile now, or, on a large tree whose call wrappers `pin_callback_owners` pinned, once
/// per `wrappers / DEFER_DIVISOR` calls. Hosts settle the rest at their event-loop turn
/// (`browser_env::flush_ownership`); any full `reconcile_all` does too.
pub(super) fn reconcile_after_call(scope: &mut v8::PinScope, owners: &CallOwners) {
    let wrappers = DOM.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        state.ownership_dirty.then_some(state.cache.len())
    });
    let Some(wrappers) = wrappers.filter(|n| owners.deferrable && *n > EAGER_WRAPPER_LIMIT) else {
        return reconcile_all(scope);
    };
    let pinned = DEFERRED_PINS.with(|pins| pins.borrow().len());
    let calls = DEFERRED_CALLS.with(|calls| {
        calls.set(calls.get() + 1);
        calls.get()
    });
    if calls >= wrappers / DEFER_DIVISOR || pinned >= MAX_PINNED {
        reconcile_all(scope);
    }
}

/// Drop the deferral state with the DOM it belonged to (`browser_env::reset`).
pub(super) fn forget_deferred() {
    let _ = DEFERRED_CALLS.try_with(|calls| calls.set(0));
    let _ = DEFERRED_PINS.try_with(|pins| std::mem::take(&mut *pins.borrow_mut()));
}

thread_local! {
    /// Bumped by every GC on an isolate with an installed DOM (`track_gc`).
    pub(super) static GC_EPOCH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// `GC_EPOCH` at the last collected-listener/document sweep.
    static SWEPT_EPOCH: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) };
}

struct GcTracked;

unsafe extern "C" fn gc_epilogue(
    _isolate: v8::UnsafeRawIsolatePtr,
    _type: v8::GCType,
    _flags: v8::GCCallbackFlags,
    _data: *mut std::ffi::c_void,
) {
    let _ = GC_EPOCH.try_with(|epoch| epoch.set(epoch.get().wrapping_add(1)));
}

/// Count this isolate's GCs (once per isolate; `install` calls it). The collected-listener and
/// document sweeps look for weak handles V8 has emptied, which only a GC does, so
/// `reconcile_all` runs them only when a GC has happened since the last sweep instead of after
/// every native call (each walks every registered listener). A new install sweeps once.
pub(super) fn track_gc(isolate: &mut v8::Isolate) {
    if isolate.get_slot::<GcTracked>().is_none() {
        isolate.add_gc_epilogue_callback(gc_epilogue, std::ptr::null_mut(), v8::GCType::kGCTypeAll);
        isolate.set_slot(GcTracked);
    }
    SWEPT_EPOCH.with(|swept| swept.set(GC_EPOCH.with(|epoch| epoch.get()).wrapping_sub(1)));
}

pub(super) fn reconcile_all(scope: &mut v8::PinScope) {
    if scope.is_execution_terminating() {
        return;
    }
    // The reconcile localizes every live wrapper. Outside a native callback (a host's
    // `flush_ownership` at its event-loop turn) those Locals would otherwise live in the host's
    // long-lived handle scope and keep detached wrappers from being collected.
    v8::scope!(let scope, scope);
    let epoch = GC_EPOCH.with(|epoch| epoch.get());
    if SWEPT_EPOCH.with(|swept| swept.replace(epoch)) != epoch {
        #[cfg(test)]
        WORK.with(|work| work.sweeps.set(work.sweeps.get() + 1));
        super::event_listeners::sweep_collected_listeners(scope);
        sweep_collected();
    }
    let current = DOM_ID.with(|id| id.get());
    let mut owners = OTHER_DOMS.with(|states| {
        states
            .borrow()
            .iter()
            .filter(|(_, state)| state.dom.as_ref().is_some_and(|dom| dom.ownership_dirty))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    });
    if DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.ownership_dirty)
    }) {
        owners.push(current);
    }
    for owner in owners {
        let _dom = DomScope::enter(owner);
        reconcile_current(scope);
        DOM.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.ownership_dirty = false;
            }
        });
    }
    // Ownership is exact again: release what deferred calls pinned (`reconcile_after_call`).
    DEFERRED_CALLS.with(|calls| calls.set(0));
    let pins = DEFERRED_PINS.with(|pins| std::mem::take(&mut *pins.borrow_mut()));
    drop(pins);
}

fn listener_roots<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Map>> {
    let name = v8::String::new(scope, "turbo-dom.ownership.node-listeners")?;
    let key = v8::Private::for_api(scope, Some(name));
    if let Some(map) = node
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
    {
        return Some(map);
    }
    let map = v8::Map::new(scope);
    (node.set_private(scope, key, map.into()) == Some(true)).then_some(map)
}
pub(super) fn retain_listener<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    id: u32,
    record: v8::Local<'s, v8::Object>,
) -> Option<()> {
    let roots = listener_roots(scope, node)?;
    let id = v8::Integer::new_from_unsigned(scope, id);
    roots.set(scope, id.into(), record.into())?;
    Some(())
}
pub(super) fn release_listener<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
    id: u32,
) {
    if let Some(roots) = listener_roots(scope, node) {
        let id = v8::Integer::new_from_unsigned(scope, id);
        roots.delete(scope, id.into());
    }
}

/// Pure inner loop of `sweep_collected`, generic over the id type so a test can
/// instrument equality/hash calls on the key. The two containers that drive the
/// inner loop (`stale_set`, `other_dom_set`) MUST stay `HashSet`-backed: the
/// second-pass filter runs once per empty-weak context entry, and each call hits
/// both sets. A regression to `Vec::contains` turns this from O(N + M) into
/// O(N * M). The hash-set equality-call count on a test input of N pairs stays
/// in O(N); a Vec-backed version would show O(N²) — that is what
/// `dom_ownership_sweep_tests` asserts.
pub(super) fn compute_stale_ids<K: Eq + std::hash::Hash + Copy>(
    other_dom_rows: impl IntoIterator<Item = (K, bool)>,
    empty_context_ids: impl IntoIterator<Item = K>,
) -> Vec<K> {
    let mut stale: Vec<K> = Vec::new();
    let mut other_dom_set: std::collections::HashSet<K> = std::collections::HashSet::new();
    for (id, graph_stale) in other_dom_rows {
        other_dom_set.insert(id);
        if graph_stale {
            stale.push(id);
        }
    }
    let stale_set: std::collections::HashSet<K> = stale.iter().copied().collect();
    for id in empty_context_ids {
        // Only prune a DOM_CONTEXTS entry when (a) its weak context is empty
        // (filtered already by the caller) and (b) OTHER_DOMS records this id —
        // i.e. the binding owns the entry. A non-surf embedder's standalone
        // registration is excluded; its lifecycle is the embedder's to drive.
        if other_dom_set.contains(&id) && !stale_set.contains(&id) {
            stale.push(id);
        }
    }
    stale
}

pub(super) fn sweep_collected() -> usize {
    // A retired frame's context dies once nothing holds its document wrapper. The ownership
    // graph's weak document goes empty then; its weak context in DOM_CONTEXTS does too, so
    // either signal identifies a frame whose native-document record is now stale (turbo-dom
    // #397: before 0.8.1 only the first was consulted, so a retired frame whose graph had
    // never been materialised, or whose ownership was `None` under a non-standard install
    // path, kept its OTHER_DOMS and DOM_CONTEXTS entries forever).
    //
    // Review R2 V8 M5: the second pass must NOT prune a DOM_CONTEXTS entry that
    // the binding did not originate itself. A non-surf embedder (host-internals
    // feature) can register its own DOM_CONTEXTS id without a paired OTHER_DOMS
    // record; V8's weak signal there is the embedder's to interpret. Narrow the
    // gate so the second pass only adds entries whose id is in OTHER_DOMS AND
    // whose weak handle is empty.
    let other_dom_rows: Vec<(u64, bool)> = OTHER_DOMS.with(|states| {
        states
            .borrow()
            .iter()
            .map(|(id, state)| {
                let graph_stale = state
                    .dom
                    .as_ref()
                    .and_then(|dom| dom.ownership.as_ref())
                    .is_some_and(|graph| !graph.borrow().document_alive());
                (*id, graph_stale)
            })
            .collect()
    });
    let empty_context_ids: Vec<u64> = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .iter()
            .filter_map(|(id, weak)| weak.is_empty().then_some(*id))
            .collect()
    });
    let mut stale = compute_stale_ids(other_dom_rows, empty_context_ids);
    OTHER_DOMS.with(|states| {
        let mut states = states.borrow_mut();
        for id in &stale {
            states.remove(id);
        }
    });
    DOM_CONTEXTS.with(|contexts| {
        let mut contexts = contexts.borrow_mut();
        for id in &stale {
            contexts.remove(id);
        }
    });
    stale.len()
}

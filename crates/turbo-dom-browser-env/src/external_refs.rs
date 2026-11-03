//! The binding's V8 external references (turbo-dom#249).
//!
//! A startup snapshot stores a native callback as an index into the isolate's external
//! reference table, so every callback reachable from the snapshotted heap must be in that
//! table, both when the snapshot is created and on every isolate built from it. One missing
//! entry makes V8 abort.
//!
//! Every place the binding hands V8 a native callback (function templates, functions,
//! accessors, interceptors) passes it through [`ext!`](crate::ext), which records the
//! adapter V8 actually calls. The table is therefore the set of callbacks the binding has
//! created in this process, in first-use order: after one install it holds every callback
//! a snapshot of an installed window can reach.
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct Registry {
    order: Vec<usize>,
    seen: HashSet<usize>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

/// Record the V8-facing adapter of `callback` (of callback type `C`) and return the
/// callback unchanged. `map_fn_to` is a pure function of `F`'s type, so the builder that
/// receives `callback` hands V8 the same address.
pub(crate) fn record<C: Copy, F: v8::MapFnTo<C> + Copy>(callback: F) -> F {
    record_raw(callback.map_fn_to());
    callback
}

/// Record an already mapped callback pointer.
pub(crate) fn record_raw<C: Copy>(callback: C) -> C {
    assert_eq!(
        std::mem::size_of::<C>(),
        std::mem::size_of::<usize>(),
        "a V8 callback is one function pointer"
    );
    // SAFETY: `C` is one of V8's `extern "C" fn` callback types (asserted pointer-sized).
    let address = unsafe { std::mem::transmute_copy::<C, usize>(&callback) };
    BUILD_LOG.with(|log| {
        if let Some(log) = log.borrow_mut().as_mut() {
            log.insert(address);
        }
    });
    let mut registry = registry().lock().unwrap();
    if registry.seen.insert(address) {
        registry.order.push(address);
    }
    callback
}

thread_local! {
    /// While a snapshot is built on this thread: every callback it handed V8.
    static BUILD_LOG: std::cell::RefCell<Option<HashSet<usize>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// Test hook: an address the next build logs although no table holds it.
    pub(crate) static UNTABLED_FOR_TEST: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Start logging this thread's callbacks (a snapshot build begins).
pub(crate) fn begin_build_log() {
    BUILD_LOG.with(|log| *log.borrow_mut() = Some(HashSet::new()));
}

/// Stop logging; whether every callback logged since [`begin_build_log`] is in `table`.
pub(crate) fn end_build_log(table: &[v8::ExternalReference]) -> bool {
    #[allow(unused_mut)]
    let mut logged = BUILD_LOG
        .with(|log| log.borrow_mut().take())
        .unwrap_or_default();
    #[cfg(test)]
    logged.extend(UNTABLED_FOR_TEST.with(std::cell::Cell::take));
    // SAFETY: every table entry was built from an address (`pointer`).
    let table = table
        .iter()
        .map(|entry| unsafe { entry.pointer } as usize)
        .collect::<HashSet<_>>();
    logged.is_subset(&table)
}

/// The table for `CreateParams::external_references` / `Isolate::snapshot_creator`.
pub(crate) fn table() -> Vec<v8::ExternalReference> {
    registry()
        .lock()
        .unwrap()
        .order
        .iter()
        .map(|address| v8::ExternalReference {
            pointer: *address as *mut std::ffi::c_void,
        })
        .collect()
}

//! Test-only primitive operation counts. Never a cache permission or V8 root.
use super::*;
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

#[derive(Clone, Default, Debug, Serialize)]
pub(super) struct Report {
    pub lazy_top: u64,
    pub import_top: u64,
    pub dependency: u64,
    pub world_reads: u64,
    pub owner_reads: u64,
    pub probe_reads: u64,
    pub media_checks: u64,
    pub max_dependencies: u64,
    pub max_validation_depth: u64,
    pub repeated_entry_in_operation: u64,
    pub adjacent_import_roots: u64,
    pub adjacent_import_visits: u64,
    pub callbacks: BTreeMap<&'static str, u64>,
    pub diagnostic_callbacks: u64,
    pub flushes: u64,
    pub diagnostic_flushes: u64,
    pub empty_documents: u64,
    pub nonempty_documents: u64,
    pub foreign_empty_documents: u64,
    pub diagnostic_empty_documents: u64,
    pub slots_max: u64,
    pub claim_scope_switches: u64,
    pub diagnostic_claim_scope_switches: u64,
}
#[derive(Default)]
struct State {
    enabled: bool,
    report: Report,
    callbacks: Vec<(&'static str, bool)>,
    validations: Vec<(u64, u64, u64, bool)>,
    last_top: Option<(u64, u64, bool)>,
    seen: BTreeMap<(u64, u64), u64>,
    next_entry: u64,
    next_operation: u64,
}
#[derive(Clone, Default)]
struct Slot(Rc<RefCell<State>>);

pub(super) fn begin(scope: &mut v8::PinScope) {
    if scope.get_slot::<Slot>().is_none() {
        scope.set_slot(Slot::default());
    }
    let slot = scope.get_slot::<Slot>().unwrap();
    *slot.0.borrow_mut() = State {
        enabled: true,
        next_entry: 1,
        next_operation: 1,
        ..Default::default()
    };
}
pub(super) fn finish(scope: &mut v8::PinScope) -> Report {
    let slot = scope.get_slot::<Slot>().unwrap();
    let mut state = slot.0.borrow_mut();
    assert!(state.callbacks.is_empty());
    assert!(state.validations.is_empty());
    state.enabled = false;
    state.report.clone()
}
fn state(scope: &v8::Isolate) -> Option<Rc<RefCell<State>>> {
    let slot = scope.get_slot::<Slot>()?;
    slot.0.borrow().enabled.then(|| slot.0.clone())
}
pub(super) struct Callback(Rc<RefCell<State>>);
impl Drop for Callback {
    fn drop(&mut self) {
        self.0.borrow_mut().callbacks.pop();
    }
}
pub(super) fn callback(scope: &v8::Isolate, name: &'static str) -> Option<Callback> {
    let slot = state(scope)?;
    {
        let mut state = slot.borrow_mut();
        let diagnostic =
            name.contains("::note_") || state.callbacks.last().is_some_and(|frame| frame.1);
        if diagnostic {
            state.report.diagnostic_callbacks += 1;
        } else {
            *state.report.callbacks.entry(name).or_default() += 1;
        }
        state.callbacks.push((name, diagnostic));
    }
    Some(Callback(slot))
}
pub(super) fn flush(scope: &v8::Isolate) {
    let Some(slot) = state(scope) else {
        return;
    };
    let mut state = slot.borrow_mut();
    if state.callbacks.last().is_some_and(|frame| frame.1) {
        state.report.diagnostic_flushes += 1;
    } else {
        state.report.flushes += 1;
    }
}
pub(super) fn slots(scope: &v8::Isolate, count: usize) {
    if let Some(slot) = state(scope) {
        let mut state = slot.borrow_mut();
        state.report.slots_max = state.report.slots_max.max(count as u64);
    }
}
pub(super) fn document(scope: &v8::Isolate, empty: bool, foreign: bool) {
    let Some(slot) = state(scope) else {
        return;
    };
    let mut state = slot.borrow_mut();
    if state.callbacks.last().is_some_and(|frame| frame.1) {
        state.report.diagnostic_empty_documents += u64::from(empty);
        return;
    }
    state.report.empty_documents += u64::from(empty);
    state.report.nonempty_documents += u64::from(!empty);
    state.report.foreign_empty_documents += u64::from(empty && foreign);
}
fn enter(state: &mut State, origin: u64, entry: u64) {
    let operation = if origin == 3 {
        state.report.dependency += 1;
        state.validations.last().map_or(0, |frame| frame.0)
    } else {
        if origin == 1 {
            state.report.lazy_top += 1;
        } else {
            state.report.import_top += 1;
        }
        let id = state.next_operation;
        state.next_operation += 1;
        id
    };
    let adjacent = if origin == 3 {
        state.validations.last().is_some_and(|frame| frame.3)
    } else {
        origin == 2 && state.last_top == Some((1, entry, true))
    };
    if adjacent {
        state.report.adjacent_import_visits += 1;
        state.report.adjacent_import_roots += u64::from(origin != 3);
    }
    state.validations.push((operation, entry, origin, adjacent));
    state.report.max_validation_depth = state
        .report
        .max_validation_depth
        .max(state.validations.len() as u64);
    let visits = state.seen.entry((operation, entry)).or_default();
    state.report.repeated_entry_in_operation += u64::from(*visits != 0);
    *visits += 1;
}
pub(super) fn note(scope: &v8::Isolate, code: usize, first: u64, second: u64) -> u64 {
    let Some(slot) = state(scope) else {
        return 0;
    };
    let mut state = slot.borrow_mut();
    match code {
        19 => state.report.world_reads += 1,
        20 => state.report.owner_reads += 1,
        21 => state.report.probe_reads += 1,
        22 => state.report.media_checks += 1,
        23 => state.report.max_dependencies = state.report.max_dependencies.max(first),
        24 => {
            let id = state.next_entry;
            state.next_entry += 1;
            return id;
        }
        25 => enter(&mut state, first, second),
        26 => {
            if let Some((_, entry, origin, _)) = state.validations.pop() {
                if origin != 3 {
                    state.last_top = Some((origin, entry, first != 0));
                }
            }
        }
        _ => {}
    }
    0
}

thread_local! {
    // Only a weak bridge to isolate-owned primitive test state; no V8 roots.
    static CLAIM_STATE: RefCell<Option<Weak<RefCell<State>>>> = const { RefCell::new(None) };
}

pub(super) struct ClaimPhase(Option<Weak<RefCell<State>>>);
impl Drop for ClaimPhase {
    fn drop(&mut self) {
        CLAIM_STATE.with(|active| *active.borrow_mut() = self.0.take());
    }
}
pub(super) fn claim_phase(scope: &v8::Isolate) -> ClaimPhase {
    let next = state(scope).map(|state| Rc::downgrade(&state));
    ClaimPhase(CLAIM_STATE.with(|active| active.replace(next)))
}
pub(super) fn claim_scope_switched() {
    let active = CLAIM_STATE.with(|active| active.borrow().as_ref().and_then(Weak::upgrade));
    if let Some(slot) = active {
        let mut state = slot.borrow_mut();
        if state.callbacks.last().is_some_and(|frame| frame.1) {
            state.report.diagnostic_claim_scope_switches += 1;
        } else {
            state.report.claim_scope_switches += 1;
        }
    }
}

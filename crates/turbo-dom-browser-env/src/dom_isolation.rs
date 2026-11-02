//! Each isolate owns its native DOM world; TLS only selects the executing world.
use super::*;
use std::rc::{Rc, Weak};

#[derive(Default)]
struct World {
    saved: RefCell<Option<Data>>,
    primary: std::cell::Cell<Option<u64>>,
    disposed: std::cell::Cell<bool>,
}
struct Slot(Rc<World>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.disposed.set(true);
        if thread_available() && selected(&self.0) {
            let data = Data::take();
            CURRENT.with(|current| *current.borrow_mut() = None);
            drop(data);
        }
        self.0.saved.borrow_mut().take();
    }
}
thread_local! {
    static CURRENT: RefCell<Option<Weak<World>>> = const { RefCell::new(None) };
}
struct Data {
    current: SavedDom,
    id: u64,
    next_id: u64,
    others: HashMap<u64, SavedDom>,
    contexts: HashMap<u64, v8::Weak<v8::Context>>,
    listeners: event_listeners::IsolationState,
    observers: mutation_observers::IsolationState,
    missing: HashMap<String, u64>,
}
impl Default for Data {
    fn default() -> Self {
        Self {
            current: SavedDom::default(),
            id: 0,
            next_id: 1,
            others: HashMap::new(),
            contexts: HashMap::new(),
            listeners: Default::default(),
            observers: Default::default(),
            missing: HashMap::new(),
        }
    }
}
impl Data {
    fn take() -> Self {
        Self {
            current: take_dom(),
            id: DOM_ID.with(|id| id.replace(0)),
            next_id: NEXT_DOM_ID.with(|id| id.replace(1)),
            others: OTHER_DOMS.with(|rows| std::mem::take(&mut *rows.borrow_mut())),
            contexts: DOM_CONTEXTS.with(|rows| std::mem::take(&mut *rows.borrow_mut())),
            listeners: event_listeners::take_isolation_state(),
            observers: mutation_observers::take_isolation_state(),
            missing: MISSING.with(|rows| std::mem::take(&mut *rows.borrow_mut())),
        }
    }
    fn install(self) {
        put_dom(self.current);
        DOM_ID.with(|id| id.set(self.id));
        NEXT_DOM_ID.with(|id| id.set(self.next_id));
        OTHER_DOMS.with(|rows| *rows.borrow_mut() = self.others);
        DOM_CONTEXTS.with(|rows| *rows.borrow_mut() = self.contexts);
        event_listeners::put_isolation_state(self.listeners);
        mutation_observers::put_isolation_state(self.observers);
        MISSING.with(|rows| *rows.borrow_mut() = self.missing);
    }
}
fn thread_available() -> bool {
    CURRENT.try_with(|_| ()).is_ok()
        && DOM.try_with(|_| ()).is_ok()
        && OTHER_DOMS.try_with(|_| ()).is_ok()
        && DOM_CONTEXTS.try_with(|_| ()).is_ok()
        && STYLE.try_with(|_| ()).is_ok()
        && LISTENERS.try_with(|_| ()).is_ok()
        && ACTIVE.try_with(|_| ()).is_ok()
        && MISSING.try_with(|_| ()).is_ok()
        && event_listeners::isolation_available()
        && mutation_observers::isolation_available()
}
fn current() -> Option<Rc<World>> {
    CURRENT
        .try_with(|current| current.borrow().as_ref()?.upgrade())
        .ok()
        .flatten()
}
fn selected(world: &Rc<World>) -> bool {
    CURRENT
        .try_with(|current| {
            current
                .borrow()
                .as_ref()
                .is_some_and(|active| active.as_ptr() == Rc::as_ptr(world))
        })
        .unwrap_or(false)
}
fn select(next: Option<Rc<World>>) {
    // A cached runtime can outlive the binding's TLS cells at thread exit.
    // Its isolate-owned saved world still drops before V8 handle invalidation.
    if !thread_available() {
        return;
    }
    if let Some(next) = &next {
        if selected(next) {
            return;
        }
    } else if CURRENT.with(|current| current.borrow().is_none()) {
        return;
    }
    let previous = current();
    let data = Data::take();
    if let Some(previous) = previous {
        if !previous.disposed.get() {
            *previous.saved.borrow_mut() = Some(data);
        }
    }
    CURRENT.with(|current| *current.borrow_mut() = next.as_ref().map(Rc::downgrade));
    if let Some(next) = next {
        next.saved.borrow_mut().take().unwrap_or_default().install();
    }
}
/// Called by native installation, before any old-world Weak can be localized.
pub(super) fn install(scope: &mut v8::PinScope) {
    let slot = scope
        .get_slot::<Slot>()
        .map(|slot| slot.0.clone())
        .unwrap_or_else(|| {
            let world = Rc::new(World::default());
            scope.set_slot(Slot(world.clone()));
            world
        });
    select(Some(slot.clone()));
    if slot.primary.get().is_none() {
        slot.primary.set(Some(DOM_ID.with(|id| id.get())));
    }
}
/// Host-only activation at a synchronous operation or Future poll boundary.
/// No isolate borrow is retained. Drop this guard before exiting the V8 isolate;
/// never hold it across an asynchronous yield or an active native Tree borrow.
pub struct NativeDomScope {
    previous: Option<Rc<World>>,
    selected: Option<Rc<World>>,
    previous_document: u64,
}
pub fn activate_isolate(isolate: &v8::Isolate) -> NativeDomScope {
    if !thread_available() {
        return NativeDomScope {
            previous: None,
            selected: None,
            previous_document: 0,
        };
    }
    let selected = isolate.get_slot::<Slot>().map(|slot| slot.0.clone());
    let previous = current();
    if let Some(selected) = &selected {
        select(Some(selected.clone()));
    }
    let previous_document = DOM_ID.with(|id| id.get());
    if let Some(selected) = &selected {
        let _document = DomScope::enter(selected.primary.get().unwrap_or(0));
        std::mem::forget(_document);
    }
    NativeDomScope {
        previous,
        selected,
        previous_document,
    }
}
impl Drop for NativeDomScope {
    fn drop(&mut self) {
        if let Some(selected) = &self.selected {
            if !selected.disposed.get() && super::dom_isolation::selected(selected) {
                let document = DomScope::enter(self.previous_document);
                std::mem::forget(document);
            }
        }
        // A previously uninstalled isolate may have installed its world during
        // this operation. Restore the caller even though selection was None.
        let previous = self
            .previous
            .take()
            .filter(|previous| !previous.disposed.get());
        select(previous);
    }
}
/// Clear only the requested isolate's native trees and lookups while it is live.
pub fn reset_isolate(isolate: &v8::Isolate) {
    let Some(world) = isolate.get_slot::<Slot>().map(|slot| slot.0.clone()) else {
        return;
    };
    let _active = activate_isolate(isolate);
    reset();
    world.primary.set(None);
}

#[cfg(test)]
mod contracts {
    use super::*;
    struct Entered {
        isolate: *mut v8::Isolate,
        native: Option<NativeDomScope>,
    }
    impl Entered {
        fn new(isolate: &mut v8::Isolate) -> Self {
            unsafe {
                isolate.enter();
            }
            Self {
                isolate,
                native: Some(activate_isolate(isolate)),
            }
        }
    }
    impl Drop for Entered {
        fn drop(&mut self) {
            self.native.take();
            unsafe {
                (*self.isolate).exit();
            }
        }
    }
    thread_local! {
        static CACHED: RefCell<Option<Runtime>> = const { RefCell::new(None) };
    }
    #[test]
    fn cached_native_world_disposes_after_binding_thread_local_cells() {
        tests::init_v8();
        std::thread::spawn(|| {
            CACHED.with(|cached| {
                let mut runtime = Runtime::new("thread owner");
                assert_eq!(runtime.eval("document.body.textContent"), "thread owner");
                *cached.borrow_mut() = Some(runtime);
            });
        })
        .join()
        .unwrap();
    }
    struct Runtime {
        isolate: Option<v8::OwnedIsolate>,
        context: Option<v8::Global<v8::Context>>,
    }
    impl Runtime {
        fn new(label: &str) -> Self {
            tests::init_v8();
            let mut isolate = v8::Isolate::new(Default::default());
            let context = {
                v8::scope!(let scope,&mut isolate);
                let context = v8::Context::new(scope, Default::default());
                let scope = &mut v8::ContextScope::new(scope, context);
                install_with_html(scope, &format!("<body><p>{label}</p></body>"));
                v8::Global::new(scope, context)
            };
            unsafe {
                isolate.exit();
            }
            Self {
                isolate: Some(isolate),
                context: Some(context),
            }
        }
        fn eval(&mut self, source: &str) -> String {
            let isolate = self.isolate.as_mut().unwrap();
            let _entered = Entered::new(isolate);
            let result = {
                v8::scope!(let scope,isolate);
                let context = v8::Local::new(scope, self.context.as_ref().unwrap());
                let scope = &mut v8::ContextScope::new(scope, context);
                let tc = std::pin::pin!(v8::TryCatch::new(scope));
                let tc = &mut tc.init();
                let source = v8::String::new(tc, source).unwrap();
                let result = v8::Script::compile(tc, source, None).unwrap().run(tc);
                assert!(
                    !tc.has_caught(),
                    "{}",
                    tc.exception()
                        .map(|e| e.to_rust_string_lossy(tc))
                        .unwrap_or_default()
                );
                result.unwrap().to_rust_string_lossy(tc)
            };
            result
        }
        fn collect(&mut self) -> usize {
            let isolate = self.isolate.as_mut().unwrap();
            let _entered = Entered::new(isolate);
            let result = {
                isolate.low_memory_notification();
                v8::scope!(let scope,isolate);
                let context = v8::Local::new(scope, self.context.as_ref().unwrap());
                let scope = &mut v8::ContextScope::new(scope, context);
                dom_ownership::reconcile_all(scope);
                clear_context_capabilities(scope);
                collect_native_storage(
                    scope,
                    turbo_dom_parser::rtdom::tree::CollectionBudget {
                        max_nodes: 100_000,
                        max_edges: 1_000_000,
                        max_payload_bytes: 16 * 1024 * 1024,
                    },
                    32,
                )
                .unwrap()
                .documents
                .len()
            };
            result
        }
    }
    impl Drop for Runtime {
        fn drop(&mut self) {
            if let Some(isolate) = self.isolate.take() {
                unsafe {
                    isolate.enter();
                }
                self.context.take();
                // Deliberately rely on the isolate-owned Slot guard, rather than
                // explicit reset, to prove peer teardown cannot clear our state.
                drop(isolate);
            }
        }
    }
    fn assert_identity(runtime: &mut Runtime, label: &str) {
        assert_eq!(runtime.eval("JSON.stringify([document.body.textContent,held===document.querySelector('p'),held.expando])"),
            format!("[\"{label}\",true,17]"));
    }
    #[test]
    fn independent_worlds_keep_documents_identity_and_gc_separate_after_first_drop() {
        let mut first = Runtime::new("first");
        first.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let mut second = Runtime::new("second");
        second.eval("globalThis.held=document.querySelector('p');held.expando=17");
        assert_identity(&mut first, "first");
        assert_identity(&mut second, "second");
        assert_eq!(first.collect(), 1);
        assert_eq!(second.collect(), 1);
        drop(first);
        assert_identity(&mut second, "second");
        assert_eq!(second.collect(), 1);
    }
    #[test]
    fn independent_worlds_survive_latest_isolate_drop() {
        let mut first = Runtime::new("first");
        first.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let mut second = Runtime::new("second");
        second.eval("globalThis.held=document.querySelector('p');held.expando=17");
        drop(second);
        assert_identity(&mut first, "first");
        assert_eq!(first.collect(), 1);
    }
    #[test]
    fn reset_isolate_clears_only_target_and_preserves_foreign_documents_in_peer() {
        let mut first = Runtime::new("first");
        first.eval("globalThis.held=document.querySelector('p');held.expando=17;globalThis.inert=document.implementation.createHTMLDocument('inert');inert.body.textContent='inert'");
        let mut second = Runtime::new("second");
        second.eval("globalThis.held=document.querySelector('p');held.expando=17");
        reset_isolate(second.isolate.as_ref().unwrap());
        assert_identity(&mut first, "first");
        assert_eq!(first.eval("inert.body.textContent"), "inert");
        assert_eq!(first.collect(), 2);
    }
    #[test]
    fn nested_activation_restores_previous_world_and_document() {
        let mut first = Runtime::new("first");
        first.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let mut second = Runtime::new("second");
        second.eval("globalThis.held=document.querySelector('p');held.expando=17");
        {
            let first_scope = activate_isolate(first.isolate.as_ref().unwrap());
            let before = with_tree(|tree| tree.text_content(tree.root())).unwrap();
            {
                let _second_scope = activate_isolate(second.isolate.as_ref().unwrap());
                assert_eq!(
                    with_tree(|tree| tree.text_content(tree.root())).unwrap(),
                    "second"
                );
            }
            assert_eq!(
                with_tree(|tree| tree.text_content(tree.root())).unwrap(),
                before
            );
            drop(first_scope);
        }
        assert_identity(&mut first, "first");
        assert_identity(&mut second, "second");
    }
    #[test]
    fn uninstalled_isolate_activation_and_reset_preserve_existing_world() {
        let mut first = Runtime::new("first");
        first.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let empty = v8::Isolate::new(Default::default());
        {
            let _empty = activate_isolate(&empty);
            reset_isolate(&empty);
            assert_eq!(
                with_tree(|tree| tree.text_content(tree.root())).unwrap(),
                "first"
            );
        }
        drop(empty);
        assert_identity(&mut first, "first");
    }
    #[test]
    fn activation_guard_outliving_isolate_does_not_restore_disposed_world() {
        let first = Runtime::new("first");
        let mut second = Runtime::new("second");
        second.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let guard = activate_isolate(first.isolate.as_ref().unwrap());
        drop(first);
        drop(guard);
        assert_identity(&mut second, "second");
        assert_eq!(second.collect(), 1);
    }
    #[test]
    fn listener_signal_ids_and_mutation_observers_remain_isolated_after_peer_teardown() {
        const SETUP:&str="globalThis.held=document.querySelector('p');globalThis.calls=0;globalThis.controller=new AbortController();held.addEventListener('isolated',()=>calls++,{signal:controller.signal});globalThis.observer=new MutationObserver(()=>{});observer.observe(held,{attributes:true})";
        let mut first = Runtime::new("first");
        first.eval(SETUP);
        let mut second = Runtime::new("second");
        second.eval(SETUP);
        assert_eq!(first.eval("held.setAttribute('owned','first');JSON.stringify(observer.takeRecords().map(record=>record.target.textContent))"),"[\"first\"]");
        assert_eq!(second.eval("held.setAttribute('owned','second');JSON.stringify(observer.takeRecords().map(record=>record.target.textContent))"),"[\"second\"]");
        drop(second);
        assert_eq!(first.eval("held.dispatchEvent(new Event('isolated'));controller.abort();held.dispatchEvent(new Event('isolated'));String(calls)"),"1");
        assert_eq!(first.eval("held.setAttribute('again','first');JSON.stringify(observer.takeRecords().map(record=>record.target.textContent))"),"[\"first\"]");
    }
    #[test]
    fn cold_install_inside_uninstalled_guard_restores_peer_world_at_operation_end() {
        let mut peer = Runtime::new("peer");
        peer.eval("globalThis.held=document.querySelector('p');held.expando=17");
        let mut cold = v8::Isolate::new(Default::default());
        {
            let _active = activate_isolate(&cold);
            v8::scope!(let scope,&mut cold);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope, "<body><p>cold</p></body>");
            assert_eq!(
                with_tree(|tree| tree.text_content(tree.root())).unwrap(),
                "cold"
            );
        }
        assert_eq!(
            with_tree(|tree| tree.text_content(tree.root())).unwrap(),
            "peer"
        );
        drop(cold);
        assert_identity(&mut peer, "peer");
    }
}

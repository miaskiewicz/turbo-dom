//! Explicit, bounded native storage collection at a host GC boundary.
use super::*;
use std::rc::{Rc, Weak};
use turbo_dom_parser::rtdom::tree::{CollectionBudget, CollectionError, CollectionReport};

struct JobPin {
    registry: Weak<PendingPins>,
    document: RefCell<Option<v8::Global<v8::Object>>>,
    owner: u64,
    node: Handle,
}
#[derive(Default)]
struct PendingPins(RefCell<Vec<Weak<JobPin>>>);
impl Drop for PendingPins {
    fn drop(&mut self) {
        // The isolate drops private slots before V8 disposal. External job pins
        // can outlive it, but their Global owners must not outlive that boundary.
        for pin in self.0.get_mut().iter().filter_map(Weak::upgrade) {
            pin.document.borrow_mut().take();
        }
    }
}

/// A real native job retains its source Document and an unwrapped native handle.
/// Dropping the last pin releases the owner; isolate teardown clears all anchors.
/// This must remain on the isolate's thread, like every other native V8 handle.
#[derive(Clone)]
pub struct NativeNodePin(Rc<JobPin>);
impl NativeNodePin {
    pub fn capture(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) -> Option<Self> {
        let _isolate = activate_isolate(scope);
        let handle = handle_of(scope, node)?;
        let owner = dom_id_of(scope, node);
        let _dom = DomScope::enter(owner);
        let root = with_tree(Tree::root)?;
        let document = DOM.with(|dom| dom.borrow().as_ref()?.cache.get(&root)?.to_local(scope))?;
        let registry = scope
            .get_slot::<Rc<PendingPins>>()
            .cloned()
            .unwrap_or_else(|| {
                let registry = Rc::new(PendingPins::default());
                scope.set_slot(registry.clone());
                registry
            });
        let pin = Rc::new(JobPin {
            registry: Rc::downgrade(&registry),
            document: RefCell::new(Some(v8::Global::new(scope, document))),
            owner,
            node: handle,
        });
        registry
            .0
            .borrow_mut()
            .retain(|entry| entry.strong_count() != 0);
        registry.0.borrow_mut().push(Rc::downgrade(&pin));
        Some(Self(pin))
    }
    /// Reproject only while the genuine job owner remains alive in this isolate.
    pub fn node<'s>(&self, scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Object>> {
        let _isolate = activate_isolate(scope);
        let registered = self.0.registry.upgrade()?;
        if !Rc::ptr_eq(scope.get_slot::<Rc<PendingPins>>()?, &registered) {
            return None;
        }
        let document = self.0.document.borrow();
        let document = document.as_ref()?;
        let _owner = v8::Local::new(scope, document);
        let _dom = DomScope::enter(self.0.owner);
        with_tree(|tree| tree.is_live(self.0.node))?.then(|| wrap(scope, self.0.node))
    }
}
pub(super) fn clear(isolate: &mut v8::Isolate) {
    if let Some(registry) = isolate.remove_slot::<Rc<PendingPins>>() {
        for pin in registry.0.borrow().iter().filter_map(Weak::upgrade) {
            pin.document.borrow_mut().take();
        }
    }
}

#[derive(Debug)]
pub struct NativeStorageReport {
    pub documents: Vec<(u64, CollectionReport)>,
}

/// Collect each live native Document after a V8 GC boundary. No GC is triggered
/// here, and invocation is explicit. Each Tree commits atomically; earlier Trees
/// may already have committed if a later Tree rejects its independent budget.
/// `max_documents` bounds realm enumeration before any storage changes.
pub fn collect_native_storage(
    scope: &mut v8::PinScope,
    budget: CollectionBudget,
    max_documents: usize,
) -> Result<NativeStorageReport, CollectionError> {
    let _isolate = activate_isolate(scope);
    if scope.is_execution_terminating() {
        return Err(CollectionError::BudgetExceeded);
    }
    admit_document_count(max_documents)?;
    let jobs = pending_jobs(scope, budget.max_edges)?;
    // Job Global owners already keep foreign/inert documents alive before this
    // sweep. Holding Locals additionally prevents reentrant GC from losing them.
    let _owners = jobs
        .iter()
        .filter_map(|pin| {
            pin.document
                .borrow()
                .as_ref()
                .map(|document| v8::Local::new(scope, document))
        })
        .collect::<Vec<_>>();
    dom_ownership::reconcile_all(scope);
    let documents = live_documents(max_documents)?;
    let mut reports = Vec::with_capacity(documents.len());
    for owner in documents {
        let _dom = DomScope::enter(owner);
        let report = collect_current(scope, budget, owner, &jobs)?;
        reports.push((owner, report));
    }
    Ok(NativeStorageReport { documents: reports })
}

fn pending_jobs(
    scope: &mut v8::PinScope,
    max_jobs: usize,
) -> Result<Vec<Rc<JobPin>>, CollectionError> {
    Ok(
        if let Some(registry) = scope.get_slot::<Rc<PendingPins>>() {
            if registry.0.borrow().len() > max_jobs {
                return Err(CollectionError::BudgetExceeded);
            }
            registry
                .0
                .borrow_mut()
                .retain(|entry| entry.strong_count() != 0);
            registry
                .0
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        },
    )
}
fn admit_document_count(max_documents: usize) -> Result<(), CollectionError> {
    let count = OTHER_DOMS.with(|states| {
        let states = states.borrow();
        if states.len() > max_documents {
            return max_documents.saturating_add(1);
        }
        states.values().filter(|state| state.dom.is_some()).count()
    }) + DOM.with(|state| usize::from(state.borrow().is_some()));
    if count > max_documents {
        Err(CollectionError::BudgetExceeded)
    } else {
        Ok(())
    }
}
fn live_documents(max_documents: usize) -> Result<Vec<u64>, CollectionError> {
    admit_document_count(max_documents)?;
    let current = DOM_ID.with(|id| id.get());
    let mut documents = OTHER_DOMS.with(|states| {
        states
            .borrow()
            .iter()
            .filter(|(_, state)| state.dom.is_some())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    });
    if DOM.with(|state| state.borrow().is_some()) {
        documents.push(current);
    }
    documents.sort_unstable();
    Ok(documents)
}
fn collect_current(
    scope: &mut v8::PinScope,
    budget: CollectionBudget,
    owner: u64,
    jobs: &[Rc<JobPin>],
) -> Result<CollectionReport, CollectionError> {
    let wrappers = DOM.with(|state| {
        let state = state.borrow();
        let state = state.as_ref().unwrap();
        if state.cache.len() > budget.max_edges {
            return Err(CollectionError::BudgetExceeded);
        }
        Ok(state
            .cache
            .iter()
            .filter_map(|(node, weak)| weak.to_local(scope).map(|object| (*node, object)))
            .collect::<Vec<_>>())
    })?;
    let mut pins = wrappers.iter().map(|(node, _)| *node).collect::<Vec<_>>();
    pins.extend(
        jobs.iter()
            .filter(|pin| pin.owner == owner && pin.document.borrow().is_some())
            .map(|pin| pin.node),
    );
    DOM.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        let report = state.tree.collect_unreachable(&pins, budget)?;
        sweep_scalars(state);
        Ok(report)
    })
}
fn sweep_scalars(state: &mut DomState) {
    state
        .cache
        .retain(|node, weak| state.tree.is_live(*node) && !weak.is_empty());
    state
        .checked_radios
        .retain(|node, _| state.tree.is_live(*node));
    state.nonces.retain(|node, _| state.tree.is_live(*node));
    state
        .shadow_modes
        .retain(|node, _| state.tree.is_live(*node));
    state.scripts.retain(|node, _| state.tree.is_live(*node));
    state.css_policy.sweep_storage(&state.tree);
    state.event_handlers.sweep_storage(&state.tree);
    state.popover.sweep_storage(&state.tree);
    state.dialog.sweep_storage(&state.tree);
    state.activation.sweep_storage(&state.tree);
    state.view_transitions.sweep_storage(&state.tree);
    STYLE.with(|styles| {
        styles
            .borrow_mut()
            .retain(|node, weak| state.tree.is_live(*node) && !weak.is_empty())
    });
    LISTENERS.with(|listeners| {
        listeners
            .borrow_mut()
            .retain(|(node, _), _| state.tree.is_live(*node))
    });
    ACTIVE.with(|active| {
        if active
            .borrow()
            .is_some_and(|node| !state.tree.is_live(node))
        {
            *active.borrow_mut() = None;
        }
    });
}

#[cfg(test)]
mod contracts {
    use super::*;
    fn budget() -> CollectionBudget {
        CollectionBudget {
            max_nodes: 100_000,
            max_edges: 1_000_000,
            max_payload_bytes: 16 * 1024 * 1024,
        }
    }
    fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
        let tc = std::pin::pin!(v8::TryCatch::new(scope));
        let tc = &mut tc.init();
        let code = v8::String::new(tc, source).unwrap();
        let result = v8::Script::compile(tc, code, None).unwrap().run(tc);
        assert!(
            !tc.has_caught(),
            "{}",
            tc.exception()
                .map(|error| error.to_rust_string_lossy(tc))
                .unwrap_or_default()
        );
        result.unwrap()
    }
    fn run(body: impl FnOnce(&mut v8::OwnedIsolate, &v8::Global<v8::Context>)) {
        tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        let context = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope, "<body><p id=connected>keep</p></body>");
            v8::Global::new(scope, context)
        };
        body(&mut isolate, &context);
        {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            clear_context_capabilities(scope);
        }
        clear_interface_records(&mut isolate);
        reset();
        drop(context);
    }
    fn gc(isolate: &mut v8::OwnedIsolate) {
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
    }
    fn collect(
        isolate: &mut v8::OwnedIsolate,
        context: &v8::Global<v8::Context>,
    ) -> NativeStorageReport {
        v8::scope!(let scope, isolate);
        let context = v8::Local::new(scope, context);
        let scope = &mut v8::ContextScope::new(scope, context);
        collect_native_storage(scope, budget(), 32).unwrap()
    }
    #[test]
    fn native_checked_radio_index_does_not_pin_unwrapped_nodes() {
        run(|isolate, context| {
            collect(isolate, context);
            let node = with_tree_mut(|tree| {
                let node = tree.create_element("input");
                tree.set_attribute(node, "type", "radio");
                tree.set_attribute(node, "name", "g");
                node
            })
            .unwrap();
            form_checked_state::set(node, true);
            assert!(DOM.with(|dom| dom
                .borrow()
                .as_ref()
                .unwrap()
                .checked_radios
                .contains_key(&node)));
            with_tree_mut(|tree| {
                tree.take_mutation_events();
            });
            gc(isolate);
            let report = collect(isolate, context);
            assert!(report.documents[0].1.reclaimed_nodes >= 1);
            assert!(!with_tree(|tree| tree.is_live(node)).unwrap());
            assert!(DOM.with(|dom| dom.borrow().as_ref().unwrap().checked_radios.is_empty()));
        });
    }
    #[test]
    fn native_churn_reclaims_detached_storage_without_reusing_handles() {
        run(|isolate, context| {
            collect(isolate, context);
            let old = with_tree_mut(|tree| {
                let mut last = tree.root();
                for _ in 0..1000 {
                    last = tree.create_element("div");
                }
                tree.take_mutation_events();
                last
            })
            .unwrap();
            gc(isolate);
            let report = collect(isolate, context);
            assert_eq!(report.documents[0].1.reclaimed_nodes, 1000);
            assert!(!with_tree(|tree| tree.is_live(old)).unwrap());
            let fresh = with_tree_mut(|tree| tree.create_element("span")).unwrap();
            assert!(fresh.raw() > old.raw());
        });
    }
    #[test]
    fn live_descendant_preserves_disconnected_component_and_identity() {
        run(|isolate, context| {
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                eval(scope, "globalThis.held=(()=>{const parent=document.createElement('div');parent.innerHTML='<b>x</b><i>y</i>';return parent.firstChild;})()");
            }
            gc(isolate);
            collect(isolate, context);
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                assert!(eval(scope, "held.parentNode.lastChild.textContent==='y' && held.parentNode.firstChild===held").is_true());
                eval(scope, "held=null");
            }
            gc(isolate);
            assert!(collect(isolate, context).documents[0].1.reclaimed_nodes >= 5);
        });
    }
    #[test]
    fn range_xpath_style_and_listener_owners_survive_collection() {
        run(|isolate, context| {
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                eval(
                    scope,
                    r#"globalThis.keep=(()=>{
                    const node=document.createElement('p');node.textContent='range';
                    const range=document.createRange();range.selectNodeContents(node);
                    const styled=document.createElement('div');styled.style.color='red';
                    const listener=document.createElement('button');listener.addEventListener('click',()=>listener.setAttribute('ok','yes'));
                    const xpath=document.evaluate('.',listener,null,XPathResult.ORDERED_NODE_SNAPSHOT_TYPE,null);
                    return {range,style:styled.style,xpath};
                })()"#,
                );
            }
            gc(isolate);
            collect(isolate, context);
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                assert!(eval(scope, "keep.xpath.snapshotItem(0).dispatchEvent(new Event('click'));String(keep.range)==='range'&&keep.style.color==='red'&&keep.xpath.snapshotItem(0).getAttribute('ok')==='yes'").is_true());
                eval(scope, "keep=null");
            }
            gc(isolate);
            assert!(collect(isolate, context).documents[0].1.reclaimed_nodes >= 4);
        });
    }
    #[test]
    fn authentic_unwrapped_job_pin_keeps_node_until_job_drops() {
        run(|isolate, context| {
            let pin = {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                let node = eval(scope, "document.createElement('section')")
                    .to_object(scope)
                    .unwrap();
                NativeNodePin::capture(scope, node).unwrap()
            };
            gc(isolate);
            collect(isolate, context);
            let handle = pin.0.node;
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                let node = pin.node(scope).unwrap();
                assert_eq!(handle_of(scope, node), Some(handle));
            }
            drop(pin);
            gc(isolate);
            assert_eq!(collect(isolate, context).documents[0].1.reclaimed_nodes, 1);
        });
    }
    fn foreign_job(frame: bool) {
        run(|isolate, main| {
            let pin = {
                v8::scope!(let scope, isolate);
                let main = v8::Local::new(scope, main);
                let scope = &mut v8::ContextScope::new(scope, main);
                if frame {
                    let context = v8::Context::new(scope, Default::default());
                    let scope = &mut v8::ContextScope::new(scope, context);
                    install_independent_html(scope, "<body>foreign</body>");
                    let node = eval(scope, "document.createElement('article')")
                        .to_object(scope)
                        .unwrap();
                    NativeNodePin::capture(scope, node).unwrap()
                } else {
                    let document = parse_html_document(scope, "<body>inert</body>", false);
                    let _dom = DomScope::for_object(scope, document);
                    let node = with_tree_mut(|tree| tree.create_element("article")).unwrap();
                    let node = wrap(scope, node);
                    NativeNodePin::capture(scope, node).unwrap()
                }
            };
            gc(isolate);
            assert_eq!(collect(isolate, main).documents.len(), 2);
            {
                v8::scope!(let scope, isolate);
                let main = v8::Local::new(scope, main);
                let scope = &mut v8::ContextScope::new(scope, main);
                let node = pin.node(scope).unwrap();
                let _dom = DomScope::for_object(scope, node);
                assert_eq!(
                    with_tree(|tree| tree.local_name(pin.0.node).map(str::to_owned)),
                    Some(Some("article".to_owned()))
                );
            }
            drop(pin);
            gc(isolate);
            assert_eq!(collect(isolate, main).documents.len(), 1);
        });
    }
    #[test]
    fn pending_native_job_retains_foreign_inert_document_then_releases_it() {
        foreign_job(false);
    }
    #[test]
    fn pending_native_job_retains_retired_frame_document_then_releases_it() {
        foreign_job(true);
    }
    #[test]
    fn pending_native_mutation_event_pins_removal_until_delivery() {
        run(|isolate, context| {
            let node = with_tree_mut(|tree| {
                tree.start_recording();
                tree.take_mutation_events();
                let node = tree.create_element("div");
                tree.append_child(tree.root(), node);
                tree.remove_child(tree.root(), node);
                node
            })
            .unwrap();
            gc(isolate);
            collect(isolate, context);
            assert!(with_tree(|tree| tree.is_live(node)).unwrap());
            with_tree_mut(Tree::take_mutation_events);
            assert_eq!(collect(isolate, context).documents[0].1.reclaimed_nodes, 1);
        });
    }
    #[test]
    fn budget_rejection_preserves_storage_and_scalar_state() {
        run(|isolate, context| {
            let node = with_tree_mut(|tree| tree.create_element("script")).unwrap();
            DOM.with(|dom| {
                dom.borrow_mut()
                    .as_mut()
                    .unwrap()
                    .nonces
                    .insert(node, "secret".encode_utf16().collect())
            });
            let before = with_tree(Tree::storage_stats).unwrap();
            {
                v8::scope!(let scope, isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                let tiny = CollectionBudget {
                    max_nodes: 0,
                    ..budget()
                };
                assert!(matches!(
                    collect_native_storage(scope, tiny, 32),
                    Err(CollectionError::BudgetExceeded)
                ));
            }
            assert_eq!(with_tree(Tree::storage_stats).unwrap(), before);
            assert!(DOM.with(|dom| dom.borrow().as_ref().unwrap().nonces.contains_key(&node)));
            collect(isolate, context);
            assert!(!DOM.with(|dom| dom.borrow().as_ref().unwrap().nonces.contains_key(&node)));
        });
    }
    #[test]
    fn queued_microtask_holds_detached_node_through_gc_and_collection() {
        run(|isolate, context| {
            {
                v8::scope!(let scope,isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                eval(scope,"(()=>{const node=document.createElement('p');node.textContent='queued';Promise.resolve().then(()=>globalThis.delivered=node.textContent);})()");
            }
            gc(isolate);
            collect(isolate, context);
            isolate.perform_microtask_checkpoint();
            {
                v8::scope!(let scope,isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                assert!(eval(scope, "delivered==='queued'").is_true());
            }
            gc(isolate);
            assert!(collect(isolate, context).documents[0].1.reclaimed_nodes >= 2);
        });
    }
    #[test]
    fn isolate_teardown_clears_surviving_job_owner_anchors() {
        let mut surviving = None;
        run(|isolate, context| {
            v8::scope!(let scope,isolate);
            let context = v8::Local::new(scope, context);
            let scope = &mut v8::ContextScope::new(scope, context);
            let node = eval(scope, "document.createElement('aside')")
                .to_object(scope)
                .unwrap();
            surviving = NativeNodePin::capture(scope, node);
        });
        let pin = surviving.unwrap();
        assert!(pin.0.document.borrow().is_none());
        drop(pin);
    }
    #[test]
    fn scalar_script_metadata_is_not_a_root_and_detached_focus_is_reset() {
        run(|isolate, context| {
            let node = with_tree_mut(|tree| tree.create_element("script")).unwrap();
            DOM.with(|state| {
                let mut state = state.borrow_mut();
                let state = state.as_mut().unwrap();
                state.scripts.insert(node, ScriptState::default());
                state.shadow_modes.insert(node, "closed".into());
            });
            ACTIVE.with(|active| *active.borrow_mut() = Some(node));
            gc(isolate);
            collect(isolate, context);
            DOM.with(|state| {
                let state = state.borrow();
                let state = state.as_ref().unwrap();
                assert!(!state.tree.is_live(node));
                assert!(!state.scripts.contains_key(&node));
                assert!(!state.shadow_modes.contains_key(&node));
            });
            assert_eq!(ACTIVE.with(|active| *active.borrow()), None);
        });
    }
    #[test]
    fn native_job_pin_rejects_foreign_isolate_before_touching_global_handle() {
        run(|isolate, context| {
            let pin = {
                v8::scope!(let scope,isolate);
                let context = v8::Local::new(scope, context);
                let scope = &mut v8::ContextScope::new(scope, context);
                let node = eval(scope, "document.createElement('div')")
                    .to_object(scope)
                    .unwrap();
                NativeNodePin::capture(scope, node).unwrap()
            };
            let mut foreign = v8::Isolate::new(Default::default());
            {
                v8::scope!(let scope,&mut foreign);
                let context = v8::Context::new(scope, Default::default());
                let scope = &mut v8::ContextScope::new(scope, context);
                assert!(pin.node(scope).is_none());
            }
            drop(foreign);
            gc(isolate);
            collect(isolate, context);
            drop(pin);
            gc(isolate);
            collect(isolate, context);
        });
    }
    #[test]
    fn isolate_slot_disposal_clears_job_that_outlives_isolate_without_explicit_cleanup() {
        tests::init_v8();
        let pin = {
            let mut isolate = v8::Isolate::new(Default::default());
            let pin = {
                v8::scope!(let scope,&mut isolate);
                let context = v8::Context::new(scope, Default::default());
                let scope = &mut v8::ContextScope::new(scope, context);
                install_with_html(scope, "<body>teardown</body>");
                let node = eval(scope, "document.createElement('aside')")
                    .to_object(scope)
                    .unwrap();
                NativeNodePin::capture(scope, node).unwrap()
            };
            // No clear_interface_records, context cleanup, or reset precedes Drop.
            drop(isolate);
            pin
        };
        assert!(pin.0.document.borrow().is_none());
        assert!(pin.0.registry.upgrade().is_none());
        drop(pin);
        reset();
    }
}

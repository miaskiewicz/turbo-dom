//! Authored HTML handler bodies are compiled by V8 with native DOM scope extensions.
use super::*;
type SourceKey = (Handle, String);
type SourceValue = Option<Vec<u16>>;
type AdoptedSource = (SourceKey, SourceValue, bool);
#[derive(Default)]
pub(super) struct State {
    policy: Option<v8::Weak<v8::Function>>,
    base: String,
    sources: HashMap<SourceKey, SourceValue>,
    /// Keys whose current `sources` entry has already been compiled and installed on the
    /// element's slot. Chrome 156 compiles an `on*` content-attribute body lazily (first
    /// handler read, listener add or dispatch), so set-phase CSP and compile-phase CSP
    /// each run their own `script-src-attr` hook — keeping the two apart requires knowing
    /// which keys are still only recorded, not yet compiled (turbo-dom#380).
    compiled: std::collections::HashSet<SourceKey>,
    /// Compile errors of content-attribute handlers not yet read or dispatched to. Chrome
    /// compiles lazily and reports the error then; the eager compile keeps it until that
    /// point (turbo-dom#153).
    errors: HashMap<SourceKey, v8::Global<v8::Value>>,
}
fn source(node: Handle, name: &str) -> Option<Vec<u16>> {
    with_tree(|tree| tree.get_attribute_utf16(node, name)).flatten()
}
pub(super) fn note(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = handle_of(scope, args.this()) else {
        return;
    };
    let name = arg_str(scope, &args, 0);
    let source = source(node, &name);
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            // invariant: natives run under the DomScope of an installed DOM
            .unwrap()
            .event_handlers
            .sources
            .insert((node, name), source);
    });
}
fn prepare(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let _dom = DomScope::for_object(scope, args.this());
    let Some(node) = handle_of(scope, args.this()) else {
        return;
    };
    let name = arg_str(scope, &args, 0);
    refresh(scope, node, &name);
    report(scope, node, &name);
}
/// Report `node`'s pending compile error of handler `name` to the window, once.
pub(super) fn report(scope: &mut v8::PinScope, node: Handle, name: &str) {
    let error = DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()?
            .event_handlers
            .errors
            .remove(&(node, name.to_owned()))
    });
    if let Some(error) = error {
        let error = v8::Local::new(scope, error);
        custom_elements::report_value(scope, error);
    }
}
/// Dispatch reaches the `on<type>` handlers of `nodes`: report their pending errors.
pub(super) fn report_dispatch(scope: &mut v8::PinScope, nodes: &[Handle], ty: &str) {
    let name = format!("on{ty}");
    // Chrome 156 compiles `on*` content-attribute bodies lazily (turbo-dom#380); a
    // compile failure on a path ancestor only surfaces once its event is dispatched, so
    // drive the compile for each path node before the dispatch's single error report.
    for node in nodes {
        refresh(scope, *node, &name);
    }
    let pending = DOM.with(|dom| {
        dom.borrow().as_ref().is_some_and(|dom| {
            nodes.iter().any(|node| {
                dom.event_handlers
                    .errors
                    .contains_key(&(*node, name.clone()))
            })
        })
    });
    if pending {
        for node in nodes {
            report(scope, *node, &name);
        }
    }
}
pub(super) fn changed(scope: &mut v8::PinScope, node: Handle, name: &str) {
    if !name.starts_with("on") {
        return;
    }
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &mut dom.as_mut().unwrap().event_handlers;
        state.sources.remove(&(node, name.to_owned()));
        state.errors.remove(&(node, name.to_owned()));
        state.compiled.remove(&(node, name.to_owned()));
    });
    record_set_phase(scope, node, name);
}

/// Record the current `on*` content-attribute value and fire the set-phase
/// `script-src-attr` policy check; defer compilation to the first handler read, listener
/// add or dispatch (Chrome 156's lazy compile; turbo-dom#380). Clears the handler slot so
/// a stale previously compiled function cannot keep responding.
pub(super) fn record_set_phase(scope: &mut v8::PinScope, node: Handle, name: &str) {
    if with_tree(|tree| tree.node_type_id(node)) != Some(1)
        || !element_members::compiles_content_attribute(node, name)
    {
        return;
    }
    let source = source(node, name);
    let (same, policy) = DOM.with(|dom| {
        let dom = dom.borrow();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &dom.as_ref().unwrap().event_handlers;
        (
            state
                .sources
                .get(&(node, name.to_owned()))
                .is_some_and(|old| old == &source),
            state.policy.clone(),
        )
    });
    if same {
        return;
    }
    let element = wrap(scope, node);
    // invariant: engine-built name, far below V8's string length limit
    let key = v8::String::new(scope, name).unwrap();
    if element.has(scope, key.into()) != Some(true) {
        return;
    }
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &mut dom.as_mut().unwrap().event_handlers;
        state
            .sources
            .insert((node, name.to_owned()), source.clone());
        state.compiled.remove(&(node, name.to_owned()));
    });
    // Fire the set-phase CSP check. Return value is intentionally ignored here: the
    // compile phase runs its own check whose return value can still abort the compile.
    // For a report-only policy both phases just produce reports.
    if let (Some(policy), Some(text)) = (policy, source) {
        // invariant: an installed DOM tree is in scope
        let root = with_tree(Tree::root).unwrap();
        let document = wrap(scope, root);
        let callback = dom_ownership::local(scope, &policy);
        let kind = v8::String::new(scope, "script-src-attr").unwrap();
        if let Some(code) = v8::String::new_from_two_byte(scope, &text, v8::NewStringType::Normal) {
            let receiver = v8::undefined(scope);
            let _ = callback.call(
                scope,
                receiver.into(),
                &[document.into(), element.into(), kind.into(), code.into()],
            );
        }
    }
    install_slot(scope, element, key, v8::null(scope).into());
}

fn install_slot<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    element: v8::Local<'s, v8::Object>,
    key: v8::Local<'s, v8::String>,
    value: v8::Local<'s, v8::Value>,
) {
    let setter = element
        .get_creation_context(scope)
        .and_then(|context| context.get_slot::<InlineHandlerSetter>())
        .map(|setter| dom_ownership::local(scope, &setter.0));
    match setter {
        Some(setter) => {
            let receiver = v8::undefined(scope).into();
            setter.call(scope, receiver, &[element.into(), key.into(), value]);
        }
        None => {
            element.set(scope, key.into(), value);
        }
    }
}
pub(super) fn refresh(scope: &mut v8::PinScope, node: Handle, name: &str) {
    if with_tree(|tree| tree.node_type_id(node)) != Some(1)
        || !element_members::compiles_content_attribute(node, name)
    {
        return;
    }
    let source = source(node, name);
    let (same_stored, in_compiled, policy, base) = DOM.with(|dom| {
        let dom = dom.borrow();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &dom.as_ref().unwrap().event_handlers;
        (
            state
                .sources
                .get(&(node, name.to_owned()))
                .is_some_and(|old| old == &source),
            state.compiled.contains(&(node, name.to_owned())),
            state.policy.clone(),
            state.base.clone(),
        )
    });
    // No-op when the recorded source still matches and there is either nothing to
    // compile (no content attribute) or it has already been compiled. The first clause
    // keeps a JS-assigned `el.on<type> = fn` from being wiped by a later refresh (there
    // is no content-attribute source to re-install), the second is the normal
    // "already lazily compiled since recording" short-circuit (turbo-dom#380).
    if same_stored && (source.is_none() || in_compiled) {
        return;
    }
    let element = wrap(scope, node);
    // invariant: engine-built name, far below V8's string length limit
    let key = v8::String::new(scope, name).unwrap();
    if element.has(scope, key.into()) != Some(true) {
        return;
    }
    DOM.with(|dom| {
        dom.borrow_mut()
            .as_mut()
            // invariant: natives run under the DomScope of an installed DOM
            .unwrap()
            .event_handlers
            .sources
            .insert((node, name.to_owned()), source.clone());
    });
    // invariant: an installed DOM tree is in scope
    let root = with_tree(Tree::root).unwrap();
    let document = wrap(scope, root);
    let active = DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.browsing_context)
    });
    // The compiled function's realm is the SCOPE's current context (V8 contract for
    // `script_compiler::compile_function`). An inline on* handler belongs to its element's
    // own document, so the body's `globalThis`, free variable lookups, and `this` default
    // must resolve against the element's OWNER-DOCUMENT realm — not whichever realm
    // happened to trigger the compile (parent-borrowed
    // `EventTarget.prototype.dispatchEvent.call(childEl, …)` reaches refresh with the
    // parent realm current), and not the V8 wrapper's creation context (that stays
    // pinned at materialization time, so `document.adoptNode(el)` moving `el` between
    // realms would not follow). Look up the element's current owner-DOM realm via its
    // tagged owner. Fall back to the V8 creation context only when the owner DOM has no
    // live realm (truly detached). turbo-dom#478 / turbo-surf#287.
    let owner_id = dom_id_of(scope, element);
    let owner_context = DOM_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .get(&owner_id)
            .and_then(|weak| weak.to_local(scope))
    });
    let element_context = owner_context.or_else(|| element.get_creation_context(scope));
    let function = source.filter(|_| active).and_then(|text| {
        if let Some(policy) = policy {
            let callback = dom_ownership::local(scope, policy);
            let kind = v8::String::new(scope, "script-src-attr").unwrap();
            let code = v8::String::new_from_two_byte(scope, &text, v8::NewStringType::Normal)?;
            let receiver = v8::undefined(scope);
            if !callback
                .call(
                    scope,
                    receiver.into(),
                    &[document.into(), element.into(), kind.into(), code.into()],
                )
                .is_some_and(|value| value.is_true())
            {
                return None;
            }
        }
        let context = match element_context {
            Some(context) => context,
            None => {
                // No live realm for this element: a torn-down context leaves neither an
                // owner-DOM entry nor a creation context, so there is nowhere to compile
                // the handler. Documented invariant: the fallback `get_creation_context`
                // above catches every legitimately detached element on the paths this
                // function is called from (owner-DOM lookup or creation-context fallback),
                // so None here means a genuine bug (a stale cache miss, a wrong
                // `dom_id_of`, a snapshot-restored or post-dispose path we did not model)
                // — but a hard panic here would be fragile for tests that legitimately
                // reach this branch (R2-events M1, 2026-11-09). This crate has no
                // logging facade (see Cargo.toml: no `log`/`tracing` dep), so we return
                // silently and rely on the handler never being invoked as the signal.
                return None;
            }
        };
        let scope = &mut v8::ContextScope::new(scope, context);
        let code = v8::String::new_from_two_byte(scope, &text, v8::NewStringType::Normal)?;
        let resource = v8::String::new(
            scope,
            if base.is_empty() {
                "about:blank"
            } else {
                &base
            },
        )
        // invariant: value produced by engine code, never page input
        .unwrap();
        let origin = v8::ScriptOrigin::new(
            scope,
            resource.into(),
            0,
            0,
            false,
            0,
            None,
            false,
            false,
            false,
            None,
        );
        let mut source = v8::script_compiler::Source::new(code, Some(&origin));
        // invariant: engine-built name, far below V8's string length limit
        let event = v8::String::new(scope, element_members::handler_parameter(node)).unwrap();
        let mut extensions = vec![document];
        let form = with_tree(|tree| {
            let mut parent = tree.parent(node);
            while let Some(current) = parent {
                if tree.local_name(current) == Some("form")
                    && tree.namespace_uri(current) == Some("http://www.w3.org/1999/xhtml")
                {
                    return Some(current);
                }
                parent = tree.parent(current);
            }
            None
        })
        .flatten();
        if let Some(form) = form {
            extensions.push(wrap(scope, form));
        }
        extensions.push(element);
        let catch = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut catch.init();
        let function = v8::script_compiler::compile_function(
            scope,
            &mut source,
            &[event],
            &extensions,
            v8::script_compiler::CompileOptions::NoCompileOptions,
            v8::script_compiler::NoCacheReason::NoReason,
        );
        termination_checkpoint(scope, "handler:compile");
        if function.is_none() && !scope.has_terminated() {
            if let Some(error) = scope.exception() {
                let error = v8::Global::new(scope, error);
                DOM.with(|dom| {
                    if let Some(dom) = dom.borrow_mut().as_mut() {
                        dom.event_handlers
                            .errors
                            .insert((node, name.to_owned()), error);
                    }
                });
            }
            scope.reset();
        }
        if let Some(function) = function {
            function.set_name(key);
        }
        function
    });
    let value = function.map_or_else(|| v8::null(scope).into(), Into::into);
    // Mark as compiled BEFORE install_slot: the captured setter can add a listener
    // (handlerAdd) when the value goes from null to a function, which re-enters
    // `refresh` through `addEventListener`'s refresh call. A second compile here would
    // run the policy a second time and loop.
    DOM.with(|dom| {
        // invariant: natives run under the DomScope of an installed DOM
        dom.borrow_mut()
            .as_mut()
            .unwrap()
            .event_handlers
            .compiled
            .insert((node, name.to_owned()));
    });
    install_slot(scope, element, key, value);
}
pub(super) fn register(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__prepareInlineHandler", prepare);
    bind_method(scope, global, "__noteInlineHandler", note);
}
pub(super) fn install(
    scope: &mut v8::PinScope,
    policy: Option<v8::Local<v8::Function>>,
    base: &str,
) {
    let policy = match policy {
        Some(callback) => {
            let Some(callback) = dom_ownership::helper_function(scope, callback) else {
                return;
            };
            Some(callback)
        }
        None => None,
    };
    DOM.with(|dom| {
        // invariant: natives run under the DomScope of an installed DOM
        dom.borrow_mut().as_mut().unwrap().event_handlers = State {
            policy,
            base: base.to_owned(),
            sources: HashMap::new(),
            compiled: std::collections::HashSet::new(),
            errors: HashMap::new(),
        };
    });
    let attrs = with_tree(|tree| {
        tree.query_selector_all("*")
            .iter()
            .flat_map(|&node| {
                tree.attributes(node)
                    .into_iter()
                    .filter(|(name, _)| name.starts_with("on"))
                    .map(move |(name, _)| (node, name.to_owned()))
            })
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    for (node, name) in attrs {
        record_set_phase(scope, node, &name);
    }
}

/// Keep the native source marker with an adopted node. Losing this marker makes
/// the destination's first refresh overwrite a previously assigned on* callback.
pub(super) fn take_adopted_sources(mapping: &[(Handle, Handle)]) -> Vec<AdoptedSource> {
    let mapping = mapping.iter().copied().collect::<HashMap<_, _>>();
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &mut dom.as_mut().unwrap().event_handlers;
        let keys = state
            .sources
            .keys()
            .filter(|(node, _)| mapping.contains_key(node))
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .map(|key| {
                // invariant: value produced by engine code, never page input
                let value = state.sources.remove(&key).unwrap();
                let was_compiled = state.compiled.remove(&key);
                ((mapping[&key.0], key.1), value, was_compiled)
            })
            .collect()
    })
}
pub(super) fn restore_adopted_sources(sources: Vec<AdoptedSource>) {
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        // invariant: natives run under the DomScope of an installed DOM
        let state = &mut dom.as_mut().unwrap().event_handlers;
        for (key, value, was_compiled) in sources {
            state.sources.insert(key.clone(), value);
            if was_compiled {
                state.compiled.insert(key);
            }
        }
    });
}
impl State {
    pub(super) fn sweep_storage(&mut self, tree: &Tree) {
        self.sources.retain(|(node, _), _| tree.is_live(*node));
        self.compiled.retain(|(node, _)| tree.is_live(*node));
        self.errors.retain(|(node, _), _| tree.is_live(*node));
    }
}

/// Host-installed document URL, never read through guest location accessors.
pub(super) fn document_url() -> String {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .map(|dom| dom.event_handlers.base.clone())
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| "about:blank".to_owned())
    })
}

#[cfg(test)]
mod mutation_tests {
    use super::*;

    #[test]
    fn handler_storage_sweep_forgets_reclaimed_nodes_and_preserves_live_entries() {
        use turbo_dom_parser::rtdom::tree::CollectionBudget;

        crate::browser_env::tests::init_v8();
        let isolate = &mut crate::browser_env::tests::new_isolate();
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let mut tree = Tree::parse("<body></body>");
        let body = tree.query_selector("body").unwrap();
        let live = tree.create_element("div");
        let dead = tree.create_element("div");
        tree.append_child(body, live);
        tree.collect_unreachable(
            &[],
            CollectionBudget {
                max_nodes: 1 << 20,
                max_edges: 1 << 20,
                max_payload_bytes: 1 << 30,
            },
        )
        .unwrap();
        assert!(tree.is_live(live) && !tree.is_live(dead));
        let live_key = (live, "onclick".to_owned());
        let dead_key = (dead, "onclick".to_owned());
        let mut state = State::default();
        for key in [&live_key, &dead_key] {
            state.sources.insert(key.clone(), Some(vec![0xd800, 65]));
            state.compiled.insert(key.clone());
            let error: v8::Local<v8::Value> = v8::Integer::new(scope, 7).into();
            state
                .errors
                .insert(key.clone(), v8::Global::new(scope, error));
        }
        state.sweep_storage(&tree);
        assert_eq!(
            state.sources,
            HashMap::from([(live_key.clone(), Some(vec![0xd800, 65]))])
        );
        assert_eq!(
            state.compiled,
            std::collections::HashSet::from([live_key.clone()])
        );
        assert_eq!(state.errors.len(), 1);
        let error = v8::Local::new(scope, state.errors.get(&live_key).unwrap());
        assert_eq!(error.int32_value(scope), Some(7));
    }
}

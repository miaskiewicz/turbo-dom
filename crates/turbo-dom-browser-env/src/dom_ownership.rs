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
fn retain_helper<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Option<()> {
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
                graph::DocumentGraph::new(scope, object).unwrap(),
            ));
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
        let mut roots = std::collections::HashMap::new();
        let mut connected = std::collections::HashSet::new();
        for handle in state.cache.keys() {
            let mut root = *handle;
            while let Some(parent) = state
                .tree
                .parent(root)
                .or_else(|| state.tree.attribute_owner(root))
                .or_else(|| state.tree.shadow_host(root))
            {
                root = parent;
            }
            roots.insert(handle.raw(), root.raw());
            if root == state.tree.root() {
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
    graph.reconcile_components(scope, |id| *roots.get(&id).unwrap());
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
pub(super) fn pin_callback_owners<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: v8::Local<'s, v8::Object>,
    arguments: impl IntoIterator<Item = v8::Local<'s, v8::Value>>,
) -> Vec<v8::Local<'s, v8::Object>> {
    let current = DOM_ID.with(|id| id.get());
    let mut owners = std::collections::HashSet::from([current]);
    for value in std::iter::once(receiver.into()).chain(arguments) {
        let Ok(object) = v8::Local::<v8::Object>::try_from(value) else {
            continue;
        };
        if object.internal_field_count() != 2 || handle_of(scope, object).is_none() {
            continue;
        }
        if let Some(id) = object
            .get_internal_field(scope, 1)
            .and_then(|value| v8::Local::<v8::BigInt>::try_from(value).ok())
        {
            owners.insert(id.u64_value().0);
        }
    }
    let mut pins = Vec::with_capacity(owners.len());
    for owner in owners {
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
        if let Some(document) = document {
            pins.push(document);
        }
    }
    pins
}

pub(super) fn reconcile_all(scope: &mut v8::PinScope) {
    if scope.is_execution_terminating() {
        return;
    }
    super::event_listeners::sweep_collected_listeners(scope);
    sweep_collected();
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

pub(super) fn sweep_collected() -> usize {
    let stale = OTHER_DOMS.with(|states| {
        states
            .borrow()
            .iter()
            .filter(|(_, state)| {
                state
                    .dom
                    .as_ref()
                    .and_then(|dom| dom.ownership.as_ref())
                    .is_some_and(|graph| !graph.borrow().document_alive())
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    });
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

//! Native weak lookups with V8-owned document and native subtree edges.
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> Option<v8::Local<'s, v8::Private>> {
    let name = v8::String::new(scope, name)?;
    Some(v8::Private::for_api(scope, Some(name)))
}
fn owner_roots<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    owner: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Map>> {
    let field = key(scope, "turbo-dom.ownership.native-owner-roots")?;
    if let Some(map) = owner
        .get_private(scope, field)
        .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
    {
        return Some(map);
    }
    let map = v8::Map::new(scope);
    (owner.set_private(scope, field, map.into()) == Some(true)).then_some(map)
}
/// Real C++ Map operations do not consult guest Map.prototype functions.
pub fn retain<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    owner: v8::Local<'s, v8::Object>,
    id: u32,
    value: v8::Local<'s, v8::Value>,
) -> Option<()> {
    let roots = owner_roots(scope, owner)?;
    let id = v8::Integer::new_from_unsigned(scope, id);
    roots.set(scope, id.into(), value)?;
    Some(())
}
pub fn release<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    owner: v8::Local<'s, v8::Object>,
    id: u32,
) -> Option<()> {
    let roots = owner_roots(scope, owner)?;
    let id = v8::Integer::new_from_unsigned(scope, id);
    roots.delete(scope, id.into())?;
    Some(())
}

// Native map shapes retain their allocation context. Allocate all private graph
// edges in the owning Document realm, including calls from foreign getters.
pub struct DocumentGraph {
    document: v8::Weak<v8::Object>,
    context: v8::Weak<v8::Context>,
    wrappers: HashMap<u32, Rc<v8::Weak<v8::Object>>>,
    connected: HashSet<u32>,
    components: HashMap<u32, v8::Weak<v8::Object>>,
    next_helper: u32,
}
impl DocumentGraph {
    pub fn new<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        document: v8::Local<'s, v8::Object>,
    ) -> Option<Self> {
        let context = document.get_creation_context(scope)?;
        let scope = &mut v8::ContextScope::new(scope, context);
        owner_roots(scope, document)?;
        Some(Self {
            document: v8::Weak::new(scope, document),
            context: v8::Weak::new(scope, context),
            wrappers: HashMap::new(),
            connected: HashSet::new(),
            components: HashMap::new(),
            next_helper: 0,
        })
    }
    pub fn retain_helper<'s>(
        &mut self,
        scope: &mut v8::PinScope<'s, '_>,
        value: v8::Local<'s, v8::Value>,
    ) -> Option<()> {
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let document = self.document.to_local(scope)?;
        let field = key(scope, "turbo-dom.ownership.document-helpers")?;
        let map = document
            .get_private(scope, field)
            .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
            .unwrap_or_else(|| {
                let map = v8::Map::new(scope);
                document.set_private(scope, field, map.into());
                map
            });
        self.next_helper = self.next_helper.checked_add(1)?;
        let id = v8::Integer::new_from_unsigned(scope, self.next_helper);
        map.set(scope, id.into(), value)?;
        Some(())
    }
    pub fn observe<'s>(
        &mut self,
        scope: &mut v8::PinScope<'s, '_>,
        id: u32,
        wrapper: v8::Local<'s, v8::Object>,
        connected: bool,
    ) -> Option<()> {
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let document = self.document.to_local(scope)?;
        let field = key(scope, "turbo-dom.ownership.owner-document")?;
        if wrapper.set_private(scope, field, document.into()) != Some(true) {
            return None;
        }
        self.wrappers
            .insert(id, Rc::new(v8::Weak::new(scope, wrapper)));
        if connected {
            retain(scope, document, id, wrapper.into())?;
            self.connected.insert(id);
        } else {
            release(scope, document, id)?;
            self.connected.remove(&id);
        }
        Some(())
    }
    pub fn observe_component<'s>(
        &mut self,
        scope: &mut v8::PinScope<'s, '_>,
        id: u32,
        wrapper: v8::Local<'s, v8::Object>,
        root: u32,
        connected: bool,
    ) -> Option<()> {
        self.observe(scope, id, wrapper, connected)?;
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let owner_key = key(scope, "turbo-dom.ownership.component-owner")?;
        let refs_key = key(scope, "turbo-dom.ownership.component-nodes")?;
        let component = if let Some(component) = self
            .components
            .get(&root)
            .and_then(|component| component.to_local(scope))
        {
            component
        } else {
            let component = v8::Object::new(scope);
            let null = v8::null(scope);
            component.set_prototype(scope, null.into())?;
            self.components
                .insert(root, v8::Weak::new(scope, component));
            component
        };
        let refs = if let Some(refs) = component
            .get_private(scope, refs_key)
            .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
        {
            refs
        } else {
            let refs = v8::Map::new(scope);
            component.set_private(scope, refs_key, refs.into())?;
            refs
        };
        let id = v8::Integer::new_from_unsigned(scope, id);
        refs.set(scope, id.into(), wrapper.into())?;
        (wrapper.set_private(scope, owner_key, component.into()) == Some(true)).then_some(())
    }
    pub fn reconcile(&mut self, scope: &mut v8::PinScope, connected: &HashSet<u32>) -> Option<()> {
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let document = self.document.to_local(scope)?;
        for id in self.connected.difference(connected) {
            release(scope, document, *id)?;
        }
        for id in connected.difference(&self.connected) {
            if let Some(wrapper) = self.wrappers.get(id).and_then(|weak| weak.to_local(scope)) {
                retain(scope, document, *id, wrapper.into())?;
            }
        }
        self.connected = connected.clone();
        Some(())
    }
    /// Every disconnected native subtree is one collectible ownership component.
    /// Holding a descendant must keep cached siblings/ancestors alive too.
    pub fn reconcile_components(
        &mut self,
        scope: &mut v8::PinScope,
        root: impl Fn(u32) -> u32,
    ) -> Option<()> {
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let document = self.document.to_local(scope)?;
        let mut groups: HashMap<u32, Vec<(u32, v8::Local<v8::Object>)>> = HashMap::new();
        for (id, weak) in &self.wrappers {
            if let Some(wrapper) = weak.to_local(scope) {
                groups.entry(root(*id)).or_default().push((*id, wrapper));
            }
        }
        let owner_key = key(scope, "turbo-dom.ownership.component-owner")?;
        let refs_key = key(scope, "turbo-dom.ownership.component-nodes")?;
        for (id, members) in groups {
            let component = self
                .components
                .get(&id)
                .and_then(|weak| weak.to_local(scope))
                .unwrap_or_else(|| {
                    let component = v8::Object::new(scope);
                    let null = v8::null(scope);
                    component.set_prototype(scope, null.into());
                    self.components.insert(id, v8::Weak::new(scope, component));
                    component
                });
            let refs = v8::Map::new(scope);
            for (node, wrapper) in members {
                let index = v8::Integer::new_from_unsigned(scope, node);
                refs.set(scope, index.into(), wrapper.into())?;
                if wrapper.set_private(scope, owner_key, component.into()) != Some(true) {
                    return None;
                }
            }
            if component.set_private(scope, refs_key, refs.into()) != Some(true) {
                return None;
            }
        }
        // Connected roots remain directly owned by the document. Disconnected
        // component anchors are reachable only through their member wrappers.
        for id in &self.connected {
            if let Some(wrapper) = self.wrappers.get(id).and_then(|weak| weak.to_local(scope)) {
                retain(scope, document, *id, wrapper.into())?;
            }
        }
        Some(())
    }
    pub fn invalidate_wrapper(&mut self, id: u32) {
        // Keep the connected entry until the next non-terminating reconciliation
        // can release its private V8 edge; this operation allocates no JS objects.
        self.wrappers.remove(&id);
    }
    pub fn forget(&mut self, scope: &mut v8::PinScope, id: u32) -> Option<()> {
        if let Some(document) = self.document.to_local(scope) {
            release(scope, document, id)?;
        }
        self.connected.remove(&id);
        self.wrappers.remove(&id);
        Some(())
    }
    pub fn lookup<'s>(
        &self,
        scope: &mut v8::PinScope<'s, '_>,
        id: u32,
    ) -> Option<v8::Local<'s, v8::Object>> {
        self.wrappers.get(&id)?.to_local(scope)
    }
    pub fn document_alive(&self) -> bool {
        !self.document.is_empty()
    }
    pub fn context_alive(&self) -> bool {
        !self.context.is_empty()
    }
    pub fn sweep(&mut self) {
        self.wrappers.retain(|_, weak| !weak.is_empty());
    }
    pub fn cached(&self) -> usize {
        self.wrappers.len()
    }
}

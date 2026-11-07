//! Native weak lookups with V8-owned document and native subtree edges.
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::rc::Rc;

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
    /// The component (root id) each wrapper is a member of, as last placed.
    placed: HashMap<u32, u32>,
    /// Ids whose wrapper object was (re)observed since the last component reconcile.
    fresh: HashSet<u32>,
    next_helper: u32,
}
impl DocumentGraph {
    pub fn new<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        document: v8::Local<'s, v8::Object>,
    ) -> Option<Self> {
        let context = document.get_creation_context(scope)?;
        let scope = &mut v8::ContextScope::new(scope, context);
        // A document deserialized from a startup snapshot (turbo-dom#249) arrives with its
        // edges: node wrappers of a tree that did not survive (dropped here), and helpers
        // (kept; numbering continues after them).
        owner_roots(scope, document)?.clear();
        let field = key(scope, "turbo-dom.ownership.document-helpers")?;
        let next_helper = document
            .get_private(scope, field)
            .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
            // ids are 1..=size: retain_helper numbers them in a u32 and never deletes one
            .map_or(0, |helpers| helpers.size() as u32);
        Some(Self {
            document: v8::Weak::new(scope, document),
            context: v8::Weak::new(scope, context),
            wrappers: HashMap::default(),
            connected: HashSet::default(),
            components: HashMap::default(),
            placed: HashMap::default(),
            fresh: HashSet::default(),
            next_helper,
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
        self.fresh.insert(id);
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
        if let Some(old) = self.placed.insert(id, root).filter(|old| *old != root) {
            self.leave(scope, old, id)?;
        }
        self.fresh.remove(&id);
        let id = v8::Integer::new_from_unsigned(scope, id);
        refs.set(scope, id.into(), wrapper.into())?;
        (wrapper.set_private(scope, owner_key, component.into()) == Some(true)).then_some(())
    }
    /// Drop `id` from the member map of component `root` (when that component is alive), so
    /// a wrapper that moved on is not kept alive by its old component.
    fn leave(&mut self, scope: &mut v8::PinScope, root: u32, id: u32) -> Option<()> {
        let Some(component) = self
            .components
            .get(&root)
            .and_then(|component| component.to_local(scope))
        else {
            return Some(());
        };
        let refs_key = key(scope, "turbo-dom.ownership.component-nodes")?;
        if let Some(refs) = component
            .get_private(scope, refs_key)
            .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
        {
            let id = v8::Integer::new_from_unsigned(scope, id);
            refs.delete(scope, id.into())?;
        }
        Some(())
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
    ///
    /// Incremental: only a wrapper whose component (`root`) changed, or whose wrapper object
    /// is new since the last reconcile, is placed again (leaving its old component's member
    /// map); a wrapper that left the graph leaves its component. Connected ones among them
    /// are retained by the document again.
    pub fn reconcile_components(
        &mut self,
        scope: &mut v8::PinScope,
        root: impl Fn(u32) -> u32,
    ) -> Option<()> {
        let owner = self.context.to_local(scope)?;
        let scope = &mut v8::ContextScope::new(scope, owner);
        let document = self.document.to_local(scope)?;
        let gone = self
            .placed
            .keys()
            .filter(|id| !self.wrappers.contains_key(id))
            .copied()
            .collect::<Vec<_>>();
        for id in gone {
            if let Some(old) = self.placed.remove(&id) {
                self.leave(scope, old, id)?;
            }
        }
        let mut moves: Vec<(u32, u32, v8::Local<v8::Object>)> = Vec::new();
        for (id, weak) in &self.wrappers {
            let target = root(*id);
            if self.placed.get(id) == Some(&target) && !self.fresh.contains(id) {
                continue;
            }
            if let Some(wrapper) = weak.to_local(scope) {
                moves.push((*id, target, wrapper));
            }
        }
        self.fresh.clear();
        let owner_key = key(scope, "turbo-dom.ownership.component-owner")?;
        let refs_key = key(scope, "turbo-dom.ownership.component-nodes")?;
        for (id, target, wrapper) in moves {
            self.apply_move(scope, document, id, target, wrapper, owner_key, refs_key)?;
        }
        Some(())
    }

    fn apply_move<'s>(
        &mut self,
        scope: &mut v8::PinScope<'s, '_>,
        document: v8::Local<'s, v8::Object>,
        id: u32,
        target: u32,
        wrapper: v8::Local<'s, v8::Object>,
        owner_key: v8::Local<'s, v8::Private>,
        refs_key: v8::Local<'s, v8::Private>,
    ) -> Option<()> {
        #[cfg(test)]
        super::WORK.with(|work| work.placements.set(work.placements.get() + 1));
        if let Some(old) = self.placed.insert(id, target).filter(|old| *old != target) {
            self.leave(scope, old, id)?;
        }
        let component = self
            .components
            .get(&target)
            .and_then(|weak| weak.to_local(scope))
            .unwrap_or_else(|| {
                let component = v8::Object::new(scope);
                let null = v8::null(scope);
                component.set_prototype(scope, null.into());
                self.components
                    .insert(target, v8::Weak::new(scope, component));
                component
            });
        let refs = match component
            .get_private(scope, refs_key)
            .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok())
        {
            Some(refs) => refs,
            None => {
                let refs = v8::Map::new(scope);
                if component.set_private(scope, refs_key, refs.into()) != Some(true) {
                    return None;
                }
                refs
            }
        };
        let index = v8::Integer::new_from_unsigned(scope, id);
        refs.set(scope, index.into(), wrapper.into())?;
        if wrapper.set_private(scope, owner_key, component.into()) != Some(true) {
            return None;
        }
        if self.connected.contains(&id) {
            retain(scope, document, id, wrapper.into())?;
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
    /// The document's tree was replaced (turbo-dom#249 attach): every node id but the
    /// document's own `root` now names a node of the new tree, so the old ids' wrapper,
    /// connection and component records leave the graph. Helpers are kept.
    pub fn retire_nodes(&mut self, scope: &mut v8::PinScope, root: u32) -> Option<()> {
        let ids = self
            .connected
            .iter()
            .chain(self.wrappers.keys())
            .copied()
            .filter(|id| *id != root)
            .collect::<HashSet<_>>();
        for id in ids {
            self.forget(scope, id)?;
        }
        self.components.clear();
        self.placed.clear();
        self.fresh.clear();
        // The document's own ownership component lists every wrapper of its composed tree:
        // leave only the document in it, so the old tree's wrappers are not kept alive (nor
        // carried into a startup snapshot; round-1 review #15).
        if let Some(document) = self.document.to_local(scope) {
            let owner_key = key(scope, "turbo-dom.ownership.component-owner")?;
            let refs_key = key(scope, "turbo-dom.ownership.component-nodes")?;
            let refs = document
                .get_private(scope, owner_key)
                .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                .and_then(|component| component.get_private(scope, refs_key))
                .and_then(|value| v8::Local::<v8::Map>::try_from(value).ok());
            if let Some(refs) = refs {
                refs.clear();
                let id = v8::Integer::new_from_unsigned(scope, root);
                refs.set(scope, id.into(), document.into())?;
            }
        }
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

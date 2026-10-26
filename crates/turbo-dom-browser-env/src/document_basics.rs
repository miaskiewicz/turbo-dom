use super::*;

/// A nonce is independent of its reflected attribute. Read only native ownership/state.
pub fn nonce(scope: &mut v8::PinScope, node: v8::Local<v8::Object>) -> Option<String> {
    nonce_units(scope, node).map(|units| String::from_utf16_lossy(&units))
}
pub(super) fn nonce_units(
    scope: &mut v8::PinScope,
    node: v8::Local<v8::Object>,
) -> Option<Vec<u16>> {
    let _dom = DomScope::for_object(scope, node);
    let handle = handle_of(scope, node)?;
    if with_tree(|tree| tree.node_type_id(handle)) != Some(1) {
        return None;
    }
    DOM.with(|dom| dom.borrow().as_ref()?.nonces.get(&handle).cloned())
        .or_else(|| with_tree(|tree| tree.get_attribute_utf16(handle, "nonce")).flatten())
        .or_else(|| Some(Vec::new()))
}

pub fn clone_nonces(source: Handle, target: Handle, deep: bool) {
    fn pairs(
        tree: &Tree,
        source: Handle,
        target: Handle,
        deep: bool,
        out: &mut Vec<(Handle, Handle)>,
    ) {
        let mut pending = vec![(source, target)];
        while let Some(pair) = pending.pop() {
            out.push(pair);
            if deep {
                pending.extend(
                    tree.children(pair.0)
                        .into_iter()
                        .zip(tree.children(pair.1))
                        .rev(),
                );
            }
        }
    }
    let mapping = with_tree(|tree| {
        let mut mapping = Vec::new();
        pairs(tree, source, target, deep, &mut mapping);
        mapping
    })
    .unwrap_or_default();
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = dom.as_mut().unwrap();
        for (source, target) in mapping {
            if let Some(value) = state.nonces.get(&source).cloned() {
                state.nonces.insert(target, value);
            }
        }
    });
}

pub fn copy_nonces(
    scope: &mut v8::PinScope,
    source: v8::Local<v8::Object>,
    target: v8::Local<v8::Object>,
    deep: bool,
) {
    fn flatten(tree: &Tree, node: Handle, deep: bool, out: &mut Vec<Handle>) {
        let mut pending = vec![node];
        while let Some(node) = pending.pop() {
            out.push(node);
            if deep {
                pending.extend(tree.children(node).into_iter().rev());
            }
        }
    }
    let values = {
        let _source = DomScope::for_object(scope, source);
        let Some(handle) = handle_of(scope, source) else {
            return;
        };
        let mut nodes = Vec::new();
        with_tree(|tree| flatten(tree, handle, deep, &mut nodes));
        DOM.with(|dom| {
            let dom = dom.borrow();
            let state = dom.as_ref().unwrap();
            nodes
                .into_iter()
                .map(|handle| state.nonces.get(&handle).cloned())
                .collect::<Vec<_>>()
        })
    };
    let _target = DomScope::for_object(scope, target);
    let Some(handle) = handle_of(scope, target) else {
        return;
    };
    let mut nodes = Vec::new();
    with_tree(|tree| flatten(tree, handle, deep, &mut nodes));
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = dom.as_mut().unwrap();
        for (node, value) in nodes.into_iter().zip(values) {
            if let Some(value) = value {
                state.nonces.insert(node, value);
            }
        }
    });
}

struct Snapshot {
    old: Handle,
    kind: u8,
    name: Vec<u16>,
    namespace: Option<Vec<u16>>,
    value: Vec<u16>,
    doctype: Option<(Vec<u16>, Vec<u16>)>,
    attributes: Vec<(Handle, turbo_dom_parser::rtdom::tree::AttributeDataUtf16)>,
    parent: Option<usize>,
    shadow: bool,
}
fn capture(tree: &mut Tree, node: Handle) -> Result<Vec<Snapshot>, &'static str> {
    let mut records = Vec::new();
    let mut pending = vec![(node, None, false)];
    while let Some((node, parent, shadow)) = pending.pop() {
        let kind = tree.node_type_id(node);
        if !matches!(kind, 1 | 2 | 3 | 7 | 8 | 10 | 11) {
            return Err("Adopting this native node kind is unsupported");
        }
        let attributes = tree
            .attributes_units(node)
            .into_iter()
            .map(|attribute| {
                let handle = tree
                    .attribute_node_units(
                        node,
                        attribute.namespace.as_deref(),
                        &attribute.local_name,
                    )
                    .unwrap();
                (handle, attribute)
            })
            .collect();
        let attr = tree.attribute_data_units(node);
        let doctype = (kind == 10).then(|| {
            (
                tree.doctype_public_id_utf16(node).unwrap_or_default(),
                tree.doctype_system_id_utf16(node).unwrap_or_default(),
            )
        });
        let name = tree
            .qualified_name_utf16(node)
            .or_else(|| tree.processing_instruction_target_utf16(node))
            .or_else(|| tree.doctype_name_utf16(node))
            .unwrap_or_default();
        let namespace = attr
            .as_ref()
            .and_then(|attr| attr.namespace.clone())
            .or_else(|| tree.namespace_uri_utf16(node));
        let value = tree.node_value_utf16(node).unwrap_or_default();

        let index = records.len();
        records.push(Snapshot {
            old: node,
            kind,
            name,
            namespace,
            value,
            doctype,
            attributes,
            parent,
            shadow,
        });
        if let Some(shadow) = tree.shadow_root(node) {
            pending.push((shadow, Some(index), true))
        }
        for child in tree.children(node).into_iter().rev() {
            pending.push((child, Some(index), false))
        }
    }
    Ok(records)
}
fn materialize(
    tree: &mut Tree,
    source: Vec<Snapshot>,
    supplied: Option<Handle>,
    mapping: &mut Vec<(Handle, Handle)>,
) -> Handle {
    let mut nodes = Vec::with_capacity(source.len());
    for record in source {
        let target = if record.shadow {
            tree.attach_shadow(nodes[record.parent.expect("shadow host snapshot")])
        } else if record.parent.is_none() && supplied.is_some() {
            supplied.expect("supplied snapshot root")
        } else {
            match record.kind {
                2 => tree.create_attribute_units(
                    record.namespace.as_deref(),
                    &record.name,
                    &record.value,
                ),
                3 => tree.create_text_node_utf16(&record.value),
                7 => tree.create_processing_instruction_units(&record.name, &record.value),
                8 => tree.create_comment_utf16(&record.value),
                10 => {
                    let (public_id, system_id) = record.doctype.as_ref().expect("doctype snapshot");
                    tree.create_document_type_units(&record.name, public_id, system_id)
                }
                11 => tree.create_document_fragment(),
                _ => tree.create_element_namespace_units(record.namespace.as_deref(), &record.name),
            }
        };
        mapping.push((record.old, target));
        for (old, attribute) in record.attributes {
            let node = tree.create_attribute_units(
                attribute.namespace.as_deref(),
                &attribute.qualified_name,
                &attribute.value,
            );
            tree.set_attribute_node(target, node)
                .expect("validated element attribute");
            mapping.push((old, node));
        }
        if !record.shadow {
            if let Some(parent) = record.parent {
                tree.append_child(nodes[parent], target)
            }
        }
        nodes.push(target);
    }
    nodes[0]
}

// Every already-visible node remains the SAME V8 object. Uncached handles are
// reconstructed in the destination tree; source cache entries cannot alias them.
pub fn adopt<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    document: v8::Local<'s, v8::Object>,
    node: v8::Local<'s, v8::Object>,
) -> Result<v8::Local<'s, v8::Object>, &'static str> {
    let target_id = dom_id_of(scope, document);
    let source_id = dom_id_of(scope, node);
    let source_handle = handle_of(scope, node).ok_or("Illegal invocation")?;
    {
        let _target = DomScope::for_object(scope, document);
        let root = handle_of(scope, document).ok_or("Illegal invocation")?;
        if !matches!(with_tree(|tree| tree.node_type_id(root)), Some(1 | 9)) {
            return Err("Illegal invocation");
        }
    }
    let source = {
        let _source = DomScope::for_object(scope, node);
        if with_tree(|tree| tree.is_shadow_root(source_handle)).unwrap_or(false) {
            return Err("A shadow root cannot be adopted");
        }
        html_popover::attribute_node_change(scope, source_handle, None);
        with_tree_mut(|tree| capture(tree, source_handle)).ok_or("Node tree unavailable")??
    };
    if source_id == target_id {
        let _source = DomScope::for_object(scope, node);
        html_popover::attribute_node_change(scope, source_handle, None);
        html_popover::disconnect(scope, source_handle, true);
        element_relations::attribute_node_changed(scope, source_handle);
        range_live::removing(scope, source_handle);
        with_tree_mut(|tree| {
            if let Some(owner) = tree.attribute_owner(source_handle) {
                let _ = tree.remove_attribute_node(owner, source_handle);
            } else if let Some(parent) = tree.parent(source_handle) {
                tree.remove_child(parent, source_handle);
            }
        });
        mutation_observers::flush(scope);
        return Ok(node);
    }
    custom_elements::adopting(scope, node);
    let mapping = {
        let _target = DomScope::for_object(scope, document);
        let mut mapping = Vec::new();
        with_tree_mut(|tree| materialize(tree, source, None, &mut mapping))
            .ok_or("Document tree unavailable")?;
        mapping
    };
    let (cache, styles, listeners, nonces, modes, handlers, scripts, custom_states, checked_states) = {
        let _source = DomScope::for_object(scope, node);
        html_popover::attribute_node_change(scope, source_handle, None);
        html_popover::disconnect(scope, source_handle, true);
        element_relations::attribute_node_changed(scope, source_handle);
        range_live::removing(scope, source_handle);
        with_tree_mut(|tree| {
            if let Some(owner) = tree.attribute_owner(source_handle) {
                let _ = tree.remove_attribute_node(owner, source_handle);
            } else if let Some(parent) = tree.parent(source_handle) {
                tree.remove_child(parent, source_handle);
            }
        });
        mutation_observers::flush(scope);
        let mut cache = Vec::new();
        let mut nonces = Vec::new();
        let mut scripts = Vec::new();
        let mut modes = Vec::new();
        DOM.with(|dom| {
            let mut dom = dom.borrow_mut();
            let state = dom.as_mut().unwrap();
            for (old, new) in &mapping {
                state.checked_radios.remove(old);
                if let Some(value) = state.cache.remove(old) {
                    cache.push((*new, value));
                }
                if let Some(value) = state.scripts.remove(old) {
                    scripts.push((*new, value));
                }
                if let Some(value) = state.nonces.remove(old) {
                    nonces.push((*new, value));
                }
                if let Some(value) = state.shadow_modes.remove(old) {
                    modes.push((*new, value));
                }
            }
        });
        let ownership = DOM.with(|state| {
            state
                .borrow()
                .as_ref()
                .and_then(|state| state.ownership.clone())
        });
        if let Some(ownership) = ownership {
            for (old, _) in &mapping {
                ownership.borrow_mut().forget(scope, old.raw());
            }
        }
        let styles = STYLE.with(|styles| {
            let mut styles = styles.borrow_mut();
            mapping
                .iter()
                .filter_map(|(old, new)| styles.remove(old).map(|value| (*new, value)))
                .collect::<Vec<_>>()
        });
        let listeners = LISTENERS.with(|listeners| {
            let mut listeners = listeners.borrow_mut();
            let keys = listeners
                .keys()
                .filter(|(handle, _)| mapping.iter().any(|(old, _)| old == handle))
                .cloned()
                .collect::<Vec<_>>();
            keys.into_iter()
                .map(|key| {
                    let new = mapping.iter().find(|(old, _)| old == &key.0).unwrap().1;
                    let value = listeners.remove(&key).unwrap();
                    ((new, key.1), value)
                })
                .collect::<Vec<_>>()
        });
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active.is_some_and(|handle| mapping.iter().any(|(old, _)| old == &handle)) {
                *active = None;
            }
        });
        let handlers = event_handlers::take_adopted_sources(&mapping);
        let custom_states = with_tree_mut(|tree| {
            mapping
                .iter()
                .map(|(old, new)| {
                    let values = tree.custom_states_utf16(*old);
                    tree.clear_custom_states(*old);
                    (*new, values)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        let checked_states = with_tree_mut(|tree| {
            mapping
                .iter()
                .filter_map(|(old, new)| {
                    let value =
                        tree.form_property(*old, turbo_dom_parser::rtdom::tree::FormProp::Checked)?;
                    tree.clear_form_property(
                        *old,
                        turbo_dom_parser::rtdom::tree::FormProp::Checked,
                    );
                    Some((*new, value))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        (
            cache,
            styles,
            listeners,
            nonces,
            modes,
            handlers,
            scripts,
            custom_states,
            checked_states,
        )
    };
    let _target = DomScope::for_object(scope, document);
    event_handlers::restore_adopted_sources(handlers);
    with_tree_mut(|tree| {
        for (node, value) in checked_states {
            tree.set_form_property(
                node,
                turbo_dom_parser::rtdom::tree::FormProp::Checked,
                value,
            );
        }
        for (node, values) in custom_states {
            for value in values {
                tree.set_custom_state_utf16(node, &value, true);
            }
        }
    });
    for (_, node) in &mapping {
        form_checked_state::refresh(*node);
    }
    for (handle, object) in cache {
        let Some(local) = object.to_local(scope) else {
            continue;
        };
        let handle_value = v8::Integer::new_from_unsigned(scope, handle.raw());
        let id = v8::BigInt::new_from_u64(scope, target_id);
        local.set_internal_field(0, handle_value.into());
        local.set_internal_field(1, id.into());
        DOM.with(|dom| {
            dom.borrow_mut()
                .as_mut()
                .unwrap()
                .cache
                .insert(handle, object);
        });
        dom_ownership::observe(scope, handle, local);
        if with_tree(|tree| tree.is_shadow_root(handle)).unwrap_or(false) {
            css_policy::clear_adopted_sheets(scope, local);
        }
    }
    STYLE.with(|state| state.borrow_mut().extend(styles));
    LISTENERS.with(|state| state.borrow_mut().extend(listeners));
    DOM.with(|dom| {
        let mut dom = dom.borrow_mut();
        let state = dom.as_mut().unwrap();
        state.scripts.extend(scripts);
        state.nonces.extend(nonces);
        state.shadow_modes.extend(modes);
    });
    template_contents::relocate(scope, node);
    custom_elements::adopted(scope, node);
    Ok(node)
}

// Preserve native node identity and embedder state: no JS remove/insert hooks run.
// Range adjustment follows pre-remove semantics without running JS node hooks.
fn move_node(
    tree: &Tree,
    parent: Handle,
    node: Handle,
    child: Option<Handle>,
) -> Result<Option<Handle>, &'static str> {
    fn root(tree: &Tree, mut node: Handle) -> Handle {
        while let Some(parent) = tree.parent(node).or_else(|| tree.shadow_host(node)) {
            node = parent;
        }
        node
    }
    if root(tree, parent) != root(tree, node) {
        return Err("HierarchyRequestError");
    }
    let mut ancestor = Some(parent);
    while let Some(current) = ancestor {
        if current == node {
            return Err("HierarchyRequestError");
        }
        ancestor = tree.parent(current).or_else(|| tree.shadow_host(current));
    }
    let child = if child == Some(node) {
        tree.next_sibling(node)
    } else {
        child
    };
    if child.is_some_and(|child| tree.parent(child) != Some(parent)) {
        return Err("NotFoundError");
    }
    let kind = tree.node_type_id(node);
    if !matches!(kind, 1 | 3 | 4 | 7 | 8) || tree.parent(node).is_none() {
        return Err("HierarchyRequestError");
    }
    if tree.node_type_id(parent) == 9 {
        if matches!(kind, 3 | 4) {
            return Err("HierarchyRequestError");
        }
        if kind == 1 {
            let children = tree.children(parent);
            let reference =
                child.and_then(|child| children.iter().position(|value| *value == child));
            if children
                .iter()
                .any(|child| *child != node && tree.node_type_id(*child) == 1)
                || reference.is_some_and(|reference| {
                    children[reference..]
                        .iter()
                        .any(|child| tree.node_type_id(*child) == 10)
                })
            {
                return Err("HierarchyRequestError");
            }
        }
    }
    Ok(child)
}

fn fragment_by_id(tree: &Tree, root: Handle, id: &[u16]) -> Option<Handle> {
    if id.is_empty() {
        return None;
    }
    let scalar = String::from_utf16_lossy(id);
    let mut pending = tree.children(root);
    pending.reverse();
    while let Some(node) = pending.pop() {
        if tree.get_attribute_ns_borrowed(node, None, "id") == Some(scalar.as_str())
            && tree
                .get_attribute_utf16(node, "id")
                .is_some_and(|value| value == id)
        {
            return Some(node);
        }
        // DOM ancestry excludes a nested shadow root and inert template contents.
        pending.extend(tree.children(node).into_iter().rev());
    }
    None
}

pub fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(node) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return type_error(scope);
    };
    let _dom = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return type_error(scope);
    };
    let kind = with_tree(|tree| tree.node_type_id(handle)).unwrap_or(0);
    match operation.as_str() {
        "fragmentById" if kind == 11 => {
            let Some(id) = dom_units(scope, args.get(2)) else {
                return;
            };
            let found = with_tree(|tree| fragment_by_id(tree, handle, &id)).flatten();
            if let Some(found) = found {
                rv.set(wrap(scope, found).into());
            } else {
                rv.set(v8::null(scope).into());
            }
        }
        "normalize" => range_live::normalize(scope, handle),
        "replaceCharacterData" if matches!(kind, 3 | 4 | 7 | 8) => {
            let Some(offset) = args.get(2).uint32_value(scope) else {
                return;
            };
            let Some(count) = args.get(3).uint32_value(scope) else {
                return;
            };
            let Some(data) = dom_units(scope, args.get(4)) else {
                return;
            };
            replace_character_data(scope, handle, offset as usize, count as usize, &data);
        }
        "stamp" => {
            let stamp =
                with_tree(|tree| format!("{}:{}", DOM_ID.with(|id| id.get()), tree.version))
                    .unwrap_or_default();
            rv.set(v8::String::new(scope, &stamp).unwrap().into());
        }

        "state" if kind == 9 => {
            let existing = DOM.with(|dom| {
                dom.borrow()
                    .as_ref()
                    .and_then(|dom| dom.basics_state.clone())
            });
            let state = if let Some(existing) = existing {
                dom_ownership::local(scope, existing)
            } else {
                let state = v8::Object::new(scope);
                let Some(stored) = dom_ownership::helper_object(scope, state) else {
                    return;
                };
                DOM.with(|dom| dom.borrow_mut().as_mut().unwrap().basics_state = Some(stored));
                state
            };
            rv.set(state.into());
        }
        "move" if matches!(kind, 1 | 9 | 11) => {
            let Ok(moved) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return type_error(scope);
            };
            let Some(moved_handle) = handle_of(scope, moved) else {
                return type_error(scope);
            };
            let mut error = (dom_id_of(scope, moved) != dom_id_of(scope, node))
                .then_some("HierarchyRequestError");
            let reference = if args.get(3).is_null_or_undefined() {
                None
            } else {
                let Ok(child) = v8::Local::<v8::Object>::try_from(args.get(3)) else {
                    return type_error(scope);
                };
                let Some(child_handle) = handle_of(scope, child) else {
                    return type_error(scope);
                };
                if dom_id_of(scope, child) != dom_id_of(scope, node) {
                    error = error.or(Some("NotFoundError"));
                }
                Some(child_handle)
            };
            if error.is_none() {
                match with_tree(|tree| move_node(tree, handle, moved_handle, reference)) {
                    Some(Ok(reference)) => {
                        range_live::removing(scope, moved_handle);
                        let index = with_tree(|tree| {
                            let children: Vec<_> = tree
                                .children(handle)
                                .into_iter()
                                .filter(|&child| child != moved_handle)
                                .collect();
                            reference
                                .and_then(|reference| {
                                    children.iter().position(|&node| node == reference)
                                })
                                .unwrap_or(children.len())
                        })
                        .unwrap_or(0);
                        range_live::inserting(scope, handle, index, 1);
                        with_tree_mut(|tree| tree.insert_before(handle, moved_handle, reference));
                        custom_elements::moved(scope, moved);
                    }
                    Some(Err(name)) => error = Some(name),
                    None => return,
                }
            }
            if let Some(error) = error {
                rv.set(v8::String::new(scope, error).unwrap().into());
            }
        }
        "createPI" if kind == 9 => {
            let Some(target) = dom_units(scope, args.get(2)) else {
                return;
            };
            let Some(data) = dom_units(scope, args.get(3)) else {
                return;
            };
            let node =
                with_tree_mut(|tree| tree.create_processing_instruction_units(&target, &data))
                    .unwrap();
            rv.set(wrap(scope, node).into());
        }
        "piTarget" if kind == 7 => {
            let target = with_tree(|tree| {
                tree.processing_instruction_target_utf16(handle)
                    .unwrap_or_default()
            })
            .unwrap();
            rv.set(unit_string(scope, &target).into());
        }
        "compatMode" if kind == 9 => {
            let mode = with_tree(|tree| tree.compat_mode()).unwrap_or("BackCompat");
            rv.set(v8::String::new(scope, mode).unwrap().into());
        }
        "doctype" if kind == 9 => {
            let found = with_tree(|tree| {
                tree.children(handle)
                    .into_iter()
                    .find(|node| tree.node_type_id(*node) == 10)
            })
            .flatten();
            rv.set(wrap_opt(scope, found));
        }
        "doctypeFields" if kind == 10 => {
            let fields = with_tree(|tree| {
                [
                    tree.doctype_name_utf16(handle).unwrap_or_default(),
                    tree.doctype_public_id_utf16(handle).unwrap_or_default(),
                    tree.doctype_system_id_utf16(handle).unwrap_or_default(),
                ]
            })
            .unwrap();
            let array = v8::Array::new(scope, 3);
            for (index, value) in fields.iter().enumerate() {
                let value = unit_string(scope, value);
                array.set_index(scope, index as u32, value.into());
            }
            rv.set(array.into());
        }
        "nonce" if kind == 1 => {
            let value = nonce_units(scope, node).unwrap_or_default();
            rv.set(unit_string(scope, &value).into());
        }
        "setNonce" if kind == 1 => {
            let Some(value) = dom_units(scope, args.get(2)) else {
                return;
            };
            DOM.with(|dom| {
                dom.borrow_mut()
                    .as_mut()
                    .unwrap()
                    .nonces
                    .insert(handle, value);
            });
        }
        "import" if kind == 9 => {
            let Ok(source) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return type_error(scope);
            };
            match import_document_node(scope, node, source, args.get(3).boolean_value(scope)) {
                Ok(node) => rv.set(node.into()),
                Err(error) => {
                    if let Some(message) = v8::String::new(scope, error) {
                        rv.set(message.into())
                    }
                }
            }
        }
        "adopt" if kind == 9 => {
            let Ok(source) = v8::Local::<v8::Object>::try_from(args.get(2)) else {
                return type_error(scope);
            };
            match adopt(scope, node, source) {
                Ok(node) => rv.set(node.into()),
                Err(error) => rv.set(v8::String::new(scope, error).unwrap().into()),
            }
        }
        _ => type_error(scope),
    }
}
fn type_error(scope: &mut v8::PinScope) {
    let message = v8::String::new(scope, "Illegal invocation").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_id_lookup_is_iterative_and_distinguishes_repaired_scalar_ids() {
        let mut tree = Tree::parse("<body></body>");
        let root = tree.create_document_fragment();
        let mut parent = root;
        for _ in 0..8192 {
            let child = tree.create_element("div");
            tree.append_child(parent, child);
            parent = child;
        }
        tree.set_attribute_utf16(parent, "id", &[0xd800]);
        let replacement = tree.create_element("span");
        tree.set_attribute_utf16(replacement, "id", &[0xfffd]);
        tree.append_child(root, replacement);
        assert_eq!(fragment_by_id(&tree, root, &[0xd800]), Some(parent));
        assert_eq!(fragment_by_id(&tree, root, &[0xfffd]), Some(replacement));
        assert_eq!(fragment_by_id(&tree, root, &[]), None);
    }

    #[test]
    fn native_deep_adoption_snapshot_keeps_shadow_namespace_and_original_units_without_recursion() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let source = format!("{}x{}", "<div>".repeat(8192), "</div>".repeat(8192));
                let mut tree = Tree::parse(&source);
                let root = tree.query_selector("body").unwrap();
                let node = tree.first_child(root).unwrap();
                let snapshot = capture(&mut tree, node).unwrap();
                assert_eq!(snapshot.len(), 8193);
                let mut target = Tree::parse("<body>");
                let mut mapping = Vec::new();
                let moved = materialize(&mut target, snapshot, None, &mut mapping);
                assert_eq!(mapping.len(), 8193);
                assert_eq!(target.text_content(moved), "x");
                assert_eq!(
                    turbo_dom_parser::rtdom::serialize::serialize_outer_utf16(&target, moved),
                    source.encode_utf16().collect::<Vec<_>>()
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
    fn check_script(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!DOCTYPE html><html><head></head><body></body></html>",
        );
        let inert = parse_html_document(scope, "<!doctype html><body>", false);
        let global = scope.get_current_context().global(scope);
        let key = v8::String::new(scope, "inertDocument").unwrap();
        global.set(scope, key.into(), inert.into());
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            let error = scope
                .exception()
                .map(|error| error.to_rust_string_lossy(scope));
            panic!("document fixture failed: {error:?}");
        }
        assert!(result.unwrap().is_true());
        reset();
    }

    #[test]
    fn inert_document_factories_preserve_receiver_ownership_and_control_state() {
        check_script(
            r#"(()=>{
          for(const tag of ['div','input','textarea','select','script','link','canvas','iframe']){
            const node=inertDocument.createElement(tag);node.id='inert-'+tag;
            if(node.ownerDocument!==inertDocument)throw new Error('wrong factory owner '+tag);
            inertDocument.body.append(node);
            if(inertDocument.getElementById(node.id)!==node||document.getElementById(node.id)!==null)throw new Error('factory trees alias');
          }
          const input=inertDocument.getElementById('inert-input');input.value='live';input.setAttribute('value','default');
          if(input.value!=='live'||input.defaultValue!=='default')throw new Error('control wrapper lost');
          let calls=0;const coerced=inertDocument.createElement({toString(){calls++;return 'span';}});
          if(calls!==1||coerced.ownerDocument!==inertDocument||coerced.localName!=='span')throw new Error('tag conversion repeated');
          const borrowed=Document.prototype.createElement.call(inertDocument,'em');
          if(borrowed.ownerDocument!==inertDocument||borrowed.parentNode!==null)throw new Error('borrowed factory owner');
          let converted=false,error;try{Document.prototype.createElement.call({}, {toString(){converted=true;return 'div';}})}catch(caught){error=caught.name}
          if(error!=='TypeError'||converted)throw new Error('brand validation after coercion');
          for(const [run,wanted] of [[()=>inertDocument.createElement(),'TypeError'],[()=>inertDocument.createElement(Symbol('tag')),'TypeError'],[()=>inertDocument.createElement('bad tag'),'InvalidCharacterError']]){let error;try{run()}catch(caught){error=caught.name}if(error!==wanted)throw new Error('factory validation '+error);}
          const nodes=[inertDocument.createTextNode('text'),inertDocument.createComment('comment'),inertDocument.createDocumentFragment(),inertDocument.createElementNS('urn:custom','x:node'),inertDocument.createAttribute('data-test'),inertDocument.createAttributeNS('urn:custom','x:value')];
          if(nodes.some(node=>node.ownerDocument!==inertDocument))throw new Error('other factory ownership');
          return true;
        })()"#,
        );
    }

    #[test]
    fn namespace_resolvers_keep_native_node_identity_and_resolution() {
        check_script(
            r#"(()=>{
          const root=inertDocument.documentElement,child=inertDocument.createElementNS('urn:custom','x:node');root.append(child);
          child.setAttributeNS('http://www.w3.org/2000/xmlns/','xmlns:y','urn:declared');
          for(const node of [root,child,inertDocument,child.firstChild||inertDocument.createTextNode('text')])if(document.createNSResolver(node)!==node)throw new Error('resolver identity');
          const resolver=inertDocument.createNSResolver(child);
          if(resolver.lookupNamespaceURI('x')!=='urn:custom'||resolver.lookupNamespaceURI('y')!=='urn:declared'||document.createNSResolver(root).lookupNamespaceURI(null)!=='http://www.w3.org/1999/xhtml')throw new Error('namespace resolution');
          let converted=false;for(const action of [()=>document.createNSResolver(),()=>document.createNSResolver(null),()=>document.createNSResolver({toString(){converted=true;return 'node'}}),()=>Document.prototype.createNSResolver.call({},child)]){let error;try{action()}catch(caught){error=caught.name}if(error!=='TypeError'||converted)throw new Error('resolver brand');}
          return true;
        })()"#,
        );
    }

    #[test]
    fn integrity_reflection_is_branded_and_updates_the_loader_attribute() {
        check_script(
            r#"(()=>{
          for(const [tag,ctor] of [['script',HTMLScriptElement],['link',HTMLLinkElement]]){
            const node=inertDocument.createElement(tag),property=Object.getOwnPropertyDescriptor(ctor.prototype,'integrity');
            if(node.integrity!=='')throw new Error('initial integrity');
            node.integrity='sha384-example';if(node.getAttribute('integrity')!=='sha384-example')throw new Error('loader attribute missing');
            node.setAttribute('integrity','sha256-updated');if(node.integrity!=='sha256-updated')throw new Error('attribute reflection');
            node.integrity=null;if(node.integrity!=='null')throw new Error('DOMString null conversion');
            let calls=0;node.integrity={toString(){calls++;return 'sha512-once';}};if(calls!==1||node.getAttribute('integrity')!=='sha512-once')throw new Error('conversion repeated');
            let converted=false;for(const wrong of [{},document.createElement('div'),document.createElementNS('urn:custom',tag)]){
              let error;try{property.set.call(wrong,{toString(){converted=true;return 'bad';}})}catch(caught){error=caught.name}if(error!=='TypeError'||converted)throw new Error('wrong receiver accepted');
            }
            let error;try{node.integrity=Symbol('bad')}catch(caught){error=caught.name}if(error!=='TypeError'||node.integrity!=='sha512-once')throw new Error('symbol conversion mutated attribute');
            node.removeAttribute('integrity');if(property.get.call(node)!=='')throw new Error('removed integrity');
          }
          return true;
        })()"#,
        );
    }

    #[test]
    fn moves_preserve_native_identity_control_state_focus_listeners_and_order() {
        check_script(
            r#"(()=>{
          const a=document.createElement('section'),b=document.createElement('section'),input=document.createElement('input'),sentinel=document.createElement('span');
          document.body.append(a,b);a.append(input);b.append(sentinel);input.value='live';input.payload={kept:true};input.focus();let events=0;input.addEventListener('probe',()=>events++);
          const children=b.children,result=b.moveBefore(input,sentinel);input.dispatchEvent(new Event('probe'));
          if(result!==undefined||b.firstChild!==input||children[0]!==input||children.length!==2||a.children.length||input.value!=='live'||!input.payload.kept||events!==1||document.activeElement!==input)throw new Error('move lost state');
          b.moveBefore(input,input);if(b.firstChild!==input||input.nextSibling!==sentinel)throw new Error('self reference');
          b.moveBefore(input,null);if(b.lastChild!==input||b.firstChild!==sentinel)throw new Error('append order');
          const host=document.createElement('div');document.body.append(host);const shadow=host.attachShadow({mode:'open'});shadow.moveBefore(input,null);
          if(input.parentNode!==shadow||input.value!=='live'||!input.isConnected)throw new Error('shadow move');
          const f=document.createDocumentFragment(),one=document.createElement('i'),two=document.createElement('b');f.append(one,two);f.moveBefore(two,one);if(f.firstChild!==two)throw new Error('detached root order');
          const comment=document.createComment('kept');document.body.append(comment);document.moveBefore(comment,document.documentElement);if(document.firstChild!==document.doctype||document.doctype.nextSibling!==comment)throw new Error('document comment order');
          return true;
        })()"#,
        );
    }

    #[test]
    fn moves_reject_invalid_roots_references_nodes_and_cycles_before_mutation() {
        check_script(
            r#"(()=>{
          const a=document.createElement('div'),b=document.createElement('div'),c=document.createElement('span'),detached=document.createElement('i');document.body.append(a,b);a.append(c);
          const errors=[];for(const action of [()=>b.moveBefore(detached,null),()=>c.moveBefore(a,null),()=>b.moveBefore(c,document.body),()=>b.moveBefore(document.createDocumentFragment(),null),()=>document.moveBefore(c,null),()=>document.moveBefore(c,document.doctype),()=>b.moveBefore(null,null),()=>b.moveBefore(c),()=>Element.prototype.moveBefore.call({},c,null)])try{action();errors.push('missing')}catch(error){errors.push(error.name)}
          if(JSON.stringify(errors)!==JSON.stringify(['HierarchyRequestError','HierarchyRequestError','NotFoundError','HierarchyRequestError','HierarchyRequestError','HierarchyRequestError','TypeError','TypeError','TypeError']))throw new Error(JSON.stringify(errors));
          if(c.parentNode!==a||a.parentNode!==document.body||b.childNodes.length!==0)throw new Error('invalid move mutated tree');
          const host=document.createElement('div');document.body.append(host);const shadow=host.attachShadow({mode:'open'});try{shadow.moveBefore(host,null);throw new Error('missing')}catch(error){if(error.name!=='HierarchyRequestError')throw error;}
          return true;
        })()"#,
        );
    }

    #[test]
    fn obsolete_document_methods_preserve_contents_and_ignore_arguments_with_real_receiver_checks()
    {
        check_script(
            r#"(()=>{
          inertDocument.body.innerHTML='<p>retained</p>';
          const before=inertDocument.body.innerHTML;
          for(const name of ['clear','captureEvents','releaseEvents']){
            const descriptor=Object.getOwnPropertyDescriptor(Document.prototype,name),fn=descriptor.value;
            if(fn.name!==name||fn.length!==0||!descriptor.enumerable||!descriptor.configurable||!descriptor.writable||Object.hasOwn(fn,'prototype')||!Function.prototype.toString.call(fn).includes('[native code]'))throw new Error('descriptor '+name);
            let hooks=0;const ignored={toString(){hooks++;throw new Error('ignored argument')}};
            if(fn.call(inertDocument,ignored)!==undefined||hooks||inertDocument.body.innerHTML!==before)throw new Error('legacy effects '+name);
            for(const receiver of [{},null,Object.create(Document.prototype)]){
              let denied=false;try{fn.call(receiver)}catch(error){denied=error instanceof TypeError}
              if(!denied)throw new Error('receiver '+name);
            }
          }
          return true;
        })()"#,
        );
    }

    #[test]
    fn unscopables_are_webidl_null_prototype_records_and_aliases_track_documents() {
        check_script(
            r#"(()=>{
          for(const owner of [Document,DocumentFragment,Element,CharacterData,DocumentType]){
            const descriptor=Object.getOwnPropertyDescriptor(owner.prototype,Symbol.unscopables),blocked=descriptor.value;
            if(descriptor.writable||descriptor.enumerable||!descriptor.configurable||Object.getPrototypeOf(blocked)!==null)throw new Error('descriptor '+owner.name);
            for(const name of Object.keys(blocked))if(blocked[name]!==true)throw new Error('blocked value');
          }
          let append='outer';with(document){if(append!=='outer')throw new Error('with document');}
          const body=document.body;let remove='outer';with(body){if(remove!=='outer')throw new Error('with element');}
          if(document.webkitHidden!==document.hidden||document.webkitVisibilityState!==document.visibilityState||document.webkitFullscreenElement!==document.fullscreenElement||document.webkitCurrentFullScreenElement!==document.fullscreenElement||document.webkitIsFullScreen!==false)throw new Error('aliases');
          let denied=false;try{Object.getOwnPropertyDescriptor(Document.prototype,'webkitHidden').get.call({});}catch(error){denied=error instanceof TypeError;}if(!denied)throw new Error('brand');
          return true;
        })()"#,
        );
    }
}

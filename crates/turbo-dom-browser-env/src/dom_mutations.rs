//! Native insertion validation and Chrome155 variadic conversion effects.
use super::*;

/// Validate the complete result before detaching any existing node. In particular,
/// rtdom's ordinary-node insertion primitives do not themselves reject cycles.
struct Plan {
    nodes: Vec<Handle>,
    remove: Vec<Handle>,
    reference: Option<Handle>,
}

fn plan(
    tree: &Tree,
    parent: Handle,
    incoming: &[Handle],
    reference: Option<Handle>,
    remove: &[Handle],
) -> Result<Plan, &'static str> {
    if !matches!(tree.node_type_id(parent), 1 | 9 | 11) {
        return Err("HierarchyRequestError");
    }
    if reference.is_some_and(|node| tree.parent(node) != Some(parent))
        || remove.iter().any(|&node| tree.parent(node) != Some(parent))
    {
        return Err("NotFoundError");
    }
    let mut nodes = Vec::new();
    for &node in incoming {
        if tree.is_shadow_root(node)
            || !matches!(tree.node_type_id(node), 1 | 3 | 4 | 7 | 8 | 10 | 11)
        {
            return Err("HierarchyRequestError");
        }
        let mut ancestor = Some(parent);
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = ancestor {
            if current == node || !visited.insert(current) {
                return Err("HierarchyRequestError");
            }
            ancestor = tree.parent(current).or_else(|| tree.shadow_host(current));
        }
        let expanded = if tree.node_type_id(node) == 11 {
            tree.children(node)
        } else {
            vec![node]
        };
        for child in expanded {
            // A repeated argument moves its earlier occurrence to the last position.
            nodes.retain(|&existing| existing != child);
            nodes.push(child);
        }
    }
    let original = tree.children(parent);
    let retained = |node: &Handle| !nodes.contains(node) && !remove.contains(node);
    let reference = reference.and_then(|reference| {
        original
            .iter()
            .position(|&node| node == reference)
            .and_then(|index| {
                original[index..]
                    .iter()
                    .find(|node| retained(node))
                    .copied()
            })
    });
    let mut final_children: Vec<_> = original.into_iter().filter(retained).collect();
    let index = reference
        .and_then(|reference| final_children.iter().position(|&node| node == reference))
        .unwrap_or(final_children.len());
    final_children.splice(index..index, nodes.iter().copied());
    if tree.node_type_id(parent) == 9 {
        let mut element = false;
        let mut doctype = false;
        for &node in &final_children {
            match tree.node_type_id(node) {
                1 if !element => element = true,
                10 if !doctype && !element => doctype = true,
                7 | 8 => (),
                _ => return Err("HierarchyRequestError"),
            }
        }
    } else if nodes.iter().any(|&node| tree.node_type_id(node) == 10) {
        return Err("HierarchyRequestError");
    }
    Ok(Plan {
        nodes,
        remove: remove.to_vec(),
        reference,
    })
}

pub(super) fn insert(
    scope: &mut v8::PinScope,
    parent: Handle,
    incoming: &[Handle],
    reference: Option<Handle>,
    remove: &[Handle],
) -> bool {
    insert_mode(scope, parent, incoming, reference, remove, false)
}

fn insert_mode(
    scope: &mut v8::PinScope,
    parent: Handle,
    incoming: &[Handle],
    reference: Option<Handle>,
    remove: &[Handle],
    replace_all: bool,
) -> bool {
    let Some(result) = with_tree(|tree| plan(tree, parent, incoming, reference, remove)) else {
        return false;
    };
    let plan = match result {
        Ok(plan) => plan,
        Err(name) => {
            throw_dom_error(
                scope,
                name,
                "The requested mutation would create an invalid DOM tree",
            );
            return false;
        }
    };
    for &node in plan.remove.iter().chain(plan.nodes.iter()) {
        if with_tree(|tree| tree.parent(node).is_some()).unwrap_or(false) {
            html_popover::disconnect(scope, node, true);
        }
    }
    // Detach the complete sequence before insertion, as the DOM insertion algorithm does.
    for &node in &plan.nodes {
        if let Some(old_parent) = with_tree(|tree| tree.parent(node)).flatten() {
            if replace_all && old_parent == parent {
                continue;
            }
            range_live::removing(scope, node);
            with_tree_mut(|tree| tree.remove_child(old_parent, node));
        }
    }
    let removed = with_tree(|tree| {
        plan.remove
            .iter()
            .copied()
            .filter(|node| tree.parent(*node) == Some(parent))
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    let previous = with_tree(|tree| {
        if let Some(first) = removed.first() {
            tree.previous_sibling(*first)
        } else if let Some(reference) = plan.reference {
            tree.previous_sibling(reference)
        } else {
            tree.children(parent).last().copied()
        }
    })
    .flatten();
    mutation_observers::quiet(scope, |scope| {
        for &node in &removed {
            range_live::removing(scope, node);
            with_tree_mut(|tree| tree.remove_child(parent, node));
        }
        let index = with_tree(|tree| {
            let children = tree.children(parent);
            plan.reference
                .and_then(|reference| children.iter().position(|&node| node == reference))
                .unwrap_or(children.len())
        })
        .unwrap_or(0);
        range_live::inserting(scope, parent, index, plan.nodes.len());
        with_tree_mut(|tree| {
            for &node in &plan.nodes {
                tree.insert_before(parent, node, plan.reference);
            }
        });
    });
    form_checked_state::inserted_many(&plan.nodes);
    form_checked_state::owner_changes();
    mutation_observers::child_list(scope, parent, plan.nodes, removed, previous, plan.reference);
    true
}

pub(super) fn sibling_mutation<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: Handle,
    nodes: &[Incoming<'s>],
    after: bool,
    replace: bool,
) {
    let owner = DOM_ID.with(|id| id.get());
    let roots: std::collections::HashSet<_> = nodes
        .iter()
        .filter_map(|node| {
            let Incoming::Node(object) = node else {
                return None;
            };
            (dom_id_of(scope, *object) == owner)
                .then(|| handle_of(scope, *object))
                .flatten()
        })
        .collect();
    let Some((parent, viable)) = with_tree(|tree| {
        let parent = tree.parent(receiver)?;
        let mut viable = if after {
            tree.next_sibling(receiver)
        } else {
            tree.previous_sibling(receiver)
        };
        while viable.is_some_and(|node| roots.contains(&node)) {
            viable = viable.and_then(|node| {
                if after {
                    tree.next_sibling(node)
                } else {
                    tree.previous_sibling(node)
                }
            });
        }
        Some((parent, viable))
    })
    .flatten() else {
        return;
    };
    let Some(nodes) = convert_variadic(scope, nodes) else {
        return;
    };
    let reference = if after {
        viable
    } else {
        with_tree(|tree| match viable {
            Some(node) => tree.next_sibling(node),
            None => tree.first_child(parent),
        })
        .flatten()
    };
    let removed =
        if replace && with_tree(|tree| tree.parent(receiver) == Some(parent)).unwrap_or(false) {
            vec![receiver]
        } else {
            Vec::new()
        };
    insert_inputs(scope, parent, &nodes, reference, &removed);
}

/// Converted WebIDL arguments retain their native source identity until adoption.
#[derive(Clone)]
pub(super) enum Incoming<'s> {
    Node(v8::Local<'s, v8::Object>),
    Text(Vec<u16>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct NodeIdentity(u64, Handle);
#[derive(Clone)]
struct NodeView<'s> {
    object: v8::Local<'s, v8::Object>,
    identity: NodeIdentity,
    kind: u8,
    shadow: bool,
    children: Vec<NodeView<'s>>,
}
#[derive(Clone)]
enum Prepared<'s> {
    Node(NodeView<'s>),
    Text(Vec<u16>),
}
impl Prepared<'_> {
    fn kind(&self) -> u8 {
        match self {
            Self::Node(node) => node.kind,
            Self::Text(..) => 3,
        }
    }
    fn identity(&self) -> Option<NodeIdentity> {
        match self {
            Self::Node(node) => Some(node.identity),
            Self::Text(..) => None,
        }
    }
}
fn view<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<NodeView<'s>> {
    let owner = dom_id_of(scope, object);
    let _source = DomScope::for_object(scope, object);
    let handle = handle_of(scope, object)?;
    let (kind, shadow, children) = with_tree(|tree| {
        let kind = tree.node_type_id(handle);
        (
            kind,
            tree.is_shadow_root(handle),
            if kind == 11 {
                tree.children(handle)
            } else {
                Vec::new()
            },
        )
    })?;
    let mut nodes = Vec::new();
    for child in children {
        let child = wrap(scope, child);
        nodes.push(view(scope, child)?);
    }
    Some(NodeView {
        object,
        identity: NodeIdentity(owner, handle),
        kind,
        shadow,
        children: nodes,
    })
}
fn prepare<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    incoming: &[Incoming<'s>],
) -> Option<Vec<Prepared<'s>>> {
    let mut nodes = Vec::new();
    for input in incoming {
        nodes.push(match input {
            Incoming::Node(node) => Prepared::Node(view(scope, *node)?),
            Incoming::Text(text) => Prepared::Text(text.clone()),
        });
    }
    Some(nodes)
}
fn detach_input<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
) -> Option<()> {
    let _source = DomScope::for_object(scope, object);
    let handle = handle_of(scope, object)?;
    if let Some(parent) = with_tree(|tree| tree.parent(handle)).flatten() {
        html_popover::disconnect(scope, handle, true);
        range_live::removing(scope, handle);
        with_tree_mut(|tree| tree.remove_child(parent, handle));
        mutation_observers::flush(scope);
    }
    Some(())
}

fn drain_fragment(scope: &mut v8::PinScope, node: &NodeView) {
    let _source = DomScope::for_object(scope, node.object);
    let parent = node.identity.1;
    let children = with_tree(|tree| tree.children(parent)).unwrap_or_default();
    mutation_observers::quiet(scope, |scope| {
        for &child in &children {
            html_popover::disconnect(scope, child, true);
            range_live::removing(scope, child);
            with_tree_mut(|tree| tree.remove_child(parent, child));
        }
    });
    mutation_observers::child_list(scope, parent, Vec::new(), children, None, None);
}

/// Chrome155 drains fragments during conversion, then detaches multi-node
/// sequences before destination checks. Failed insertion keeps source owners.
pub(super) fn convert_variadic<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    incoming: &[Incoming<'s>],
) -> Option<Vec<Incoming<'s>>> {
    let mut nodes = Vec::new();
    for input in incoming {
        match input {
            Incoming::Text(text) => nodes.push(Incoming::Text(text.clone())),
            Incoming::Node(object) => {
                let node = view(scope, *object)?;
                if node.kind == 11 {
                    drain_fragment(scope, &node);
                    for child in node.children {
                        nodes.push(Incoming::Node(child.object));
                    }
                } else {
                    nodes.push(input.clone());
                }
            }
        }
    }
    if nodes.len() > 1 {
        let mut detached = std::collections::HashSet::new();
        for input in &nodes {
            if let Incoming::Node(object) = input {
                let identity = NodeIdentity(dom_id_of(scope, *object), handle_of(scope, *object)?);
                if detached.insert(identity) {
                    detach_input(scope, *object)?;
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        nodes.reverse();
        nodes.retain(|input| match input {
            Incoming::Node(object) => handle_of(scope, *object)
                .is_some_and(|handle| seen.insert(NodeIdentity(dom_id_of(scope, *object), handle))),
            Incoming::Text(..) => true,
        });
        nodes.reverse();
    }
    Some(nodes)
}

fn incoming_plan<'s>(
    tree: &Tree,
    owner: u64,
    parent: Handle,
    incoming: &[Prepared<'s>],
    reference: Option<Handle>,
    remove: &[Handle],
) -> Result<Vec<Prepared<'s>>, &'static str> {
    if !matches!(tree.node_type_id(parent), 1 | 9 | 11) {
        return Err("HierarchyRequestError");
    }
    if reference.is_some_and(|node| tree.parent(node) != Some(parent))
        || remove.iter().any(|&node| tree.parent(node) != Some(parent))
    {
        return Err("NotFoundError");
    }
    let mut result: Vec<Prepared<'s>> = Vec::new();
    let mut consumed = std::collections::HashSet::new();
    let mut fragments = std::collections::HashSet::new();
    for input in incoming {
        match input {
            Prepared::Text(..) => result.push(input.clone()),
            Prepared::Node(node) => {
                if !matches!(node.kind, 1 | 3 | 4 | 7 | 8 | 10 | 11) {
                    return Err("HierarchyRequestError");
                }
                if node.identity.0 == owner {
                    let mut ancestor = Some(parent);
                    let mut visited = std::collections::HashSet::new();
                    while let Some(current) = ancestor {
                        if current == node.identity.1 || !visited.insert(current) {
                            return Err("HierarchyRequestError");
                        }
                        ancestor = tree.parent(current).or_else(|| tree.shadow_host(current));
                    }
                }
                let expanded = if node.kind == 11 {
                    if !fragments.insert(node.identity) {
                        continue;
                    }
                    node.children
                        .iter()
                        .filter(|child| !consumed.contains(&child.identity))
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    vec![node.clone()]
                };
                for child in expanded {
                    if child.shadow || !matches!(child.kind, 1 | 3 | 4 | 7 | 8 | 10) {
                        return Err("HierarchyRequestError");
                    }
                    result.retain(|prior| prior.identity() != Some(child.identity));
                    consumed.insert(child.identity);
                    result.push(Prepared::Node(child));
                }
            }
        }
    }
    if tree.node_type_id(parent) == 9 {
        let moving: std::collections::HashSet<_> =
            result.iter().filter_map(Prepared::identity).collect();
        let original = tree.children(parent);
        let retain =
            |node: &Handle| !remove.contains(node) && !moving.contains(&NodeIdentity(owner, *node));
        let adjusted = reference.and_then(|reference| {
            original
                .iter()
                .position(|node| *node == reference)
                .and_then(|index| original[index..].iter().find(|node| retain(node)).copied())
        });
        let mut final_kinds = original
            .into_iter()
            .filter(retain)
            .map(|node| (Some(node), tree.node_type_id(node)))
            .collect::<Vec<_>>();
        let index = adjusted
            .and_then(|node| {
                final_kinds
                    .iter()
                    .position(|(handle, _)| *handle == Some(node))
            })
            .unwrap_or(final_kinds.len());
        final_kinds.splice(index..index, result.iter().map(|node| (None, node.kind())));
        let mut element = false;
        let mut doctype = false;
        for (_, kind) in final_kinds {
            match kind {
                1 if !element => element = true,
                10 if !doctype && !element => doctype = true,
                7 | 8 => (),
                _ => return Err("HierarchyRequestError"),
            }
        }
    } else if result.iter().any(|node| node.kind() == 10) {
        return Err("HierarchyRequestError");
    }
    Ok(result)
}

pub(super) fn incoming_node<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
    index: i32,
) -> Option<Incoming<'s>> {
    let value = args.get(index);
    let object = if value.is_object() {
        value.to_object(scope)
    } else {
        None
    };
    if let Some(object) = object {
        if handle_of(scope, object).is_some() {
            return Some(Incoming::Node(object));
        }
    }
    let message = v8::String::new(scope, "The argument must be a native Node").unwrap();
    let error = v8::Exception::type_error(scope, message);
    scope.throw_exception(error);
    None
}
pub(super) fn reference_argument<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'s>,
    index: i32,
) -> Option<Handle> {
    let Incoming::Node(object) = incoming_node(scope, args, index)? else {
        return None;
    };
    if dom_id_of(scope, object) != DOM_ID.with(|id| id.get()) {
        throw_dom_error(
            scope,
            "NotFoundError",
            "The reference node does not belong to this parent",
        );
        return None;
    }
    handle_of(scope, object)
}
pub(super) fn insert_inputs<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    parent: Handle,
    incoming: &[Incoming<'s>],
    reference: Option<Handle>,
    remove: &[Handle],
) -> bool {
    insert_inputs_mode(scope, parent, incoming, reference, remove, false)
}

pub(super) fn replace_inputs<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    parent: Handle,
    incoming: &[Incoming<'s>],
    remove: &[Handle],
) -> bool {
    insert_inputs_mode(scope, parent, incoming, None, remove, true)
}

fn insert_inputs_mode<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    parent: Handle,
    incoming: &[Incoming<'s>],
    reference: Option<Handle>,
    remove: &[Handle],
    replace_all: bool,
) -> bool {
    let parent_object = wrap(scope, parent);
    for input in incoming {
        if let Incoming::Node(node) = input {
            if template_contents::host_ancestor(scope, parent_object, *node) {
                throw_dom_error(
                    scope,
                    "HierarchyRequestError",
                    "A template host cannot enter its content",
                );
                return false;
            }
        }
    }
    let owner = DOM_ID.with(|id| id.get());
    let Some(prepared) = prepare(scope, incoming) else {
        return false;
    };
    let Some(planned) =
        with_tree(|tree| incoming_plan(tree, owner, parent, &prepared, reference, remove))
    else {
        return false;
    };
    let planned = match planned {
        Ok(nodes) => nodes,
        Err(name) => {
            throw_dom_error(
                scope,
                name,
                "The requested mutation would create an invalid DOM tree",
            );
            return false;
        }
    };
    let root = with_tree(|tree| tree.root()).unwrap();
    let document = wrap(scope, root);
    for input in &prepared {
        if let Prepared::Node(node) = input {
            if node.kind == 11 {
                drain_fragment(scope, node);
            }
        }
    }
    // Fragment/ShadowRoot containers stay in their original document. Adopt
    // exactly the expanded children, preserving the container's native identity.
    for input in &planned {
        if let Prepared::Node(node) = input {
            if dom_id_of(scope, node.object) != owner {
                if let Err(error) = document_basics::adopt(scope, document, node.object) {
                    throw_dom_error(scope, "HierarchyRequestError", error);
                    return false;
                }
            }
        }
    }
    let mut handles = Vec::new();
    for node in planned {
        match node {
            Prepared::Node(node) => {
                if dom_id_of(scope, node.object) != owner {
                    return false;
                }
                let Some(handle) = handle_of(scope, node.object) else {
                    return false;
                };
                handles.push(handle);
            }
            Prepared::Text(text) => {
                handles.push(with_tree_mut(|tree| tree.create_text_node_utf16(&text)).unwrap())
            }
        }
    }
    insert_mode(scope, parent, &handles, reference, remove, replace_all)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(source: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();
        let source = v8::String::new(scope, source).unwrap();
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        if result.is_none() {
            panic!(
                "native mutation fixture: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            );
        }
        assert!(result.unwrap().is_true());
        reset();
    }
    fn independent_control(probe: &str, fixture: &str) {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        let actual = {
            v8::scope!(let scope, isolate);
            let parent = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, parent);
            install_with_html(
                scope,
                "<!doctype html><html><head></head><body></body></html>",
            );
            let child = v8::Context::new(scope, Default::default());
            child.set_security_token(parent.get_security_token(scope));
            {
                let child_scope = &mut v8::ContextScope::new(scope, child);
                install_independent_html(
                    child_scope,
                    "<!doctype html><html><head></head><body></body></html>",
                );
            }
            let global = parent.global(scope);
            let key = v8::String::new(scope, "child").unwrap();
            let child_global = child.global(scope);
            global.create_data_property(scope, key.into(), child_global.into());
            // The canonical binding provides genuine independent contexts; Surf
            // supplies browsing contexts. Adapt only the authored frame setup.
            let start = probe
                .find("  const source = frame.contentDocument;")
                .unwrap();
            let body = &probe[start..];
            let source = format!("JSON.stringify((()=>{{const frame={{contentDocument:child.document,contentWindow:child,remove(){{}}}};{body}");
            let source = source.trim().trim_end_matches(';').to_owned() + ")";
            let scope = std::pin::pin!(v8::TryCatch::new(scope));
            let scope = &mut scope.init();
            let source = v8::String::new(scope, &source).unwrap();
            v8::Script::compile(scope, source, None)
                .and_then(|script| script.run(scope))
                .map(|result| result.to_rust_string_lossy(scope))
                .ok_or_else(|| {
                    scope
                        .exception()
                        .map(|error| {
                            let key = v8::String::new(scope, "stack").unwrap();
                            error
                                .to_object(scope)
                                .and_then(|error| error.get(scope, key.into()))
                                .map(|stack| stack.to_rust_string_lossy(scope))
                                .unwrap_or_else(|| error.to_rust_string_lossy(scope))
                        })
                        .unwrap_or_default()
                })
        };
        reset();
        clear_interface_records(isolate);
        let actual: serde_json::Value = serde_json::from_str(&actual.unwrap()).unwrap();
        let expected: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(actual, expected["result"]);
    }

    #[test]
    fn implicit_adoption_matches_shared_chrome155_insertion_and_failure_control() {
        independent_control(
            include_str!("dom_adoption_probe.js"),
            include_str!("dom_adoption_chrome155.json"),
        );
    }
    #[test]
    fn variadic_conversion_matches_shared_chrome155_failure_and_overlap_control() {
        independent_control(
            include_str!("dom_variadic_errors_probe.js"),
            include_str!("dom_variadic_errors_chrome155.json"),
        );
    }

    #[test]
    fn variadic_webidl_conversion_finishes_before_native_detachment() {
        fixture(
            r#"(()=>{
            const parent=document.createElement('section'), child=document.createElement('b');
            parent.append(child);document.body.append(parent);
            const range=document.createRange();range.selectNode(child);
            const marker=new Error('conversion failed');let calls=0,error;
            try{parent.append(child,{toString(){calls++;throw marker;}})}catch(caught){error=caught;}
            if(error!==marker||calls!==1||child.parentNode!==parent||parent.childNodes.length!==1||
                range.startContainer!==parent||range.startOffset!==0||range.endOffset!==1)
                throw new Error('WebIDL failure mutated native inputs');
            const detached=document.createElement('i'), fragment=document.createDocumentFragment();fragment.append(child);
            detached.before(fragment,'text');
            if(child.parentNode!==fragment||fragment.childNodes.length!==1)
                throw new Error('parentless ChildNode operation performed native conversion');
            return true;
        })()"#,
        );
    }

    #[test]
    fn attribute_child_nodes_have_native_identity_and_reject_forged_receivers() {
        fixture(
            r#"(()=>{
          const attribute=document.createAttribute('authored');attribute.value='text';
          const list=attribute.childNodes;
          if(!(list instanceof NodeList)||list.length!==0||list!==attribute.childNodes||list.item(0)!==null)
            throw new Error('Attr did not expose its real empty SameObject NodeList');
          const element=document.createElement('div');element.setAttributeNode(attribute);
          attribute.value='replacement';
          if(list!==attribute.childNodes||list.length!==0||attribute.ownerElement!==element)
            throw new Error('attribute children changed with its value');
          const getter=Object.getOwnPropertyDescriptor(Node.prototype,'childNodes').get;
          for(const object of [{},Object.create(Attr.prototype),Object.create(CDATASection.prototype)]){
            let error;try{Reflect.apply(getter,object,[])}catch(caught){error=caught.name;}
            if(error!=='TypeError')throw new Error('forged native Node receiver accepted');
          }
          let unsupported;try{document.createCDATASection('authored')}catch(error){unsupported=error.name;}
          if(unsupported!=='NotSupportedError')throw new Error('HTML document CDATA restriction changed');
          return true;
        })()"#,
        );
    }

    #[test]
    fn ordinary_self_append_throws_before_parent_links_change() {
        fixture(
            r#"(()=>{const element=document.createElement('div');let error;try{element.appendChild(element)}catch(caught){error=caught.name;}if(error!=='HierarchyRequestError')throw new Error('self append was accepted');if(element.parentNode!==null||element.childNodes.length!==0)throw new Error('self append changed the tree');return true;})()"#,
        );
    }
    #[test]
    fn every_existing_node_insertion_rejects_ancestor_cycles_atomically() {
        fixture(
            r#"(()=>{
          const methods=[(p,c)=>c.appendChild(p),(p,c)=>c.insertBefore(p,null),
            (p,c)=>c.append('prefix',p),(p,c)=>c.prepend(p,'suffix'),
            (p,c)=>c.replaceChildren('prefix',p),(p,c)=>c.before('prefix',p),
            (p,c)=>c.after(p),(p,c)=>c.replaceWith(p),
            (p,c)=>c.insertAdjacentElement('beforebegin',p),
            (p,c)=>c.insertAdjacentElement('afterend',p),
            (p,c)=>c.insertAdjacentElement('afterbegin',p),
            (p,c)=>c.insertAdjacentElement('beforeend',p),
            (p,c)=>p.replaceChild(p,c)];
          for(let index=0;index<methods.length;index++){
            const parent=document.createElement('section'),child=document.createElement('div');
            parent.appendChild(child);child.textContent='original';
            const before=parent.outerHTML;let error;
            try{methods[index](parent,child)}catch(caught){error=caught.name;}
            if(error!=='HierarchyRequestError')throw new Error('cycle seam '+index+' '+error);
            if(parent.outerHTML!==before||parent.parentNode!==null||child.parentNode!==parent)
              throw new Error('non-atomic cycle seam '+index);
          }
          const host=document.createElement('div'),shadow=host.attachShadow({mode:'open'}),inside=document.createElement('span');
          shadow.appendChild(inside);let error;try{inside.appendChild(host)}catch(caught){error=caught.name;}
          if(error!=='HierarchyRequestError'||host.parentNode!==null||inside.parentNode!==shadow)
            throw new Error('shadow-including ancestor cycle');
          return true;
        })()"#,
        );
    }

    #[test]
    fn document_hierarchy_and_fragment_failures_preserve_existing_nodes() {
        fixture(
            r#"(()=>{
          const root=document.documentElement,doctype=document.doctype;
          const reject=(run)=>{const before=Array.from(document.childNodes);let error;
            try{run()}catch(caught){error=caught.name;}
            if(error!=='HierarchyRequestError'||before.length!==document.childNodes.length||before.some((node,index)=>node!==document.childNodes[index]))throw new Error('invalid document mutation was not atomic '+error);};
          const extra=document.createElement('extra');reject(()=>document.appendChild(extra));
          if(extra.parentNode!==null)throw new Error('extra root moved');
          reject(()=>document.appendChild(document.createTextNode('text')));
          reject(()=>document.insertBefore(root,doctype));
          const fragment=document.createDocumentFragment(),first=document.createElement('first'),second=document.createElement('second');
          fragment.append(first,second);reject(()=>document.replaceChild(fragment,root));
          if(fragment.childNodes.length!==2||first.parentNode!==fragment||second.parentNode!==fragment)throw new Error('failed fragment drained');
          const next=document.createElement('html');
          if(document.replaceChild(next,root)!==root||document.documentElement!==next||root.parentNode!==null||document.doctype!==doctype)throw new Error('document root replacement');
          if(document.insertBefore(next,next)!==next||document.documentElement!==next)throw new Error('self reference');
          next.appendChild(root);let error;try{next.appendChild(doctype)}catch(caught){error=caught.name;}
          if(error!=='HierarchyRequestError'||doctype.parentNode!==document)throw new Error('doctype element child');
          return true;
        })()"#,
        );
    }

    #[test]
    fn native_node_brands_references_and_removal_are_checked_before_mutation() {
        fixture(
            r#"(()=>{
          const parent=document.createElement('div'),child=document.createElement('span'),foreign=document.createElement('b');parent.appendChild(child);
          const reject=(name,run)=>{let error;try{run()}catch(caught){error=caught.name;}
            if(error!==name||parent.firstChild!==child||child.parentNode!==parent||foreign.parentNode!==null)throw new Error('invalid argument changed tree '+error);};
          for(const value of [null,undefined,42,'text',{},Object.create(Node.prototype)]){
            reject('TypeError',()=>parent.appendChild(value));reject('TypeError',()=>parent.insertBefore(value,null));reject('TypeError',()=>parent.replaceChild(value,child));}
          reject('NotFoundError',()=>parent.insertBefore(foreign,foreign));
          reject('NotFoundError',()=>parent.replaceChild(foreign,foreign));
          reject('NotFoundError',()=>parent.removeChild(foreign));
          reject('TypeError',()=>parent.insertBefore(foreign));
          reject('TypeError',()=>parent.append('prefix',Symbol('invalid')));
          reject('TypeError',()=>parent.insertAdjacentElement('beforeend',document.createTextNode('text')));
          reject('SyntaxError',()=>parent.insertAdjacentElement('invalid',foreign));
          reject('SyntaxError',()=>parent.insertAdjacentHTML('invalid','<i>invalid</i>'));
          return true;
        })()"#,
        );
    }

    #[test]
    fn valid_moves_fragments_strings_and_repeated_arguments_keep_specified_order() {
        fixture(
            r#"(()=>{
          const parent=document.createElement('div'),a=document.createElement('a'),b=document.createElement('b'),c=document.createElement('i');
          parent.append(a,b,c);parent.insertBefore(b,b);
          if(parent.children[0]!==a||parent.children[1]!==b||parent.children[2]!==c)throw new Error('insert before self reordered');
          parent.prepend('head',c);parent.append('tail');
          if(parent.firstChild.data!=='head'||parent.childNodes[1]!==c||parent.lastChild.data!=='tail')throw new Error('prepend/string order');
          parent.replaceChildren(a,b,a);
          if(parent.children.length!==2||parent.children[0]!==b||parent.children[1]!==a)throw new Error('duplicate order');
          a.before(a,'before');a.after(a,'after');
          if(parent.innerHTML!=='<b></b><a></a>afterbefore')throw new Error('viable sibling '+parent.innerHTML);
          a.replaceWith(a);if(a.parentNode!==parent)throw new Error('self replacement detached');
          const fragment=document.createDocumentFragment();fragment.append('one',c,'two');
          parent.replaceChild(fragment,a);
          if(fragment.childNodes.length!==0||parent.innerHTML!=='<b></b>one<i></i>twoafterbefore'||a.parentNode!==null)throw new Error('fragment replace');
          const detached=document.createElement('aside');if(detached.insertAdjacentElement('beforebegin',a)!==null||a.parentNode!==null)throw new Error('detached adjacent result');
          let error;try{document.documentElement.insertAdjacentHTML('beforebegin','<x></x>')}catch(caught){error=caught.name;}
          if(error!=='NoModificationAllowedError')throw new Error('document adjacentHTML policy');
          return true;
        })()"#,
        );
    }

    #[test]
    fn native_errors_retain_dom_exception_brands_despite_guest_constructor_overrides() {
        fixture(
            r#"(()=>{
          const Original=DOMException,originalString=String,get=WeakMap.prototype.get,set=WeakMap.prototype.set;
          const element=document.createElement('div');let error;
          try{
            globalThis.DOMException=function(){throw new Error('guest constructor invoked')};
            globalThis.String=function(){throw new Error('guest string invoked')};
            WeakMap.prototype.get=function(){throw new Error('guest weak get invoked')};
            WeakMap.prototype.set=function(){throw new Error('guest weak set invoked')};
            try{element.appendChild(element)}catch(caught){error=caught;}
            if(!(error instanceof Original)||error.name!=='HierarchyRequestError'||error.code!==3)throw new Error('unbranded native exception');
          }finally{globalThis.DOMException=Original;globalThis.String=originalString;WeakMap.prototype.get=get;WeakMap.prototype.set=set;}
          if(element.parentNode!==null||element.childNodes.length!==0||Node.prototype.replaceChild.length!==2)throw new Error('mutation or method shape');
          return true;
        })()"#,
        );
    }
}

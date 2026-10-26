//! Template content lives in a genuine associated inert native document.
use super::*;
#[derive(Default)]
pub(super) struct State {
    pub associated: bool,
    pub enabled: bool,
}
struct Keys {
    content: v8::Global<v8::Private>,
    host: v8::Global<v8::Private>,
    associated_document: v8::Global<v8::Private>,
}
fn capture_keys(scope: &mut v8::PinScope) -> Option<()> {
    if scope.get_slot::<std::rc::Rc<Keys>>().is_some() {
        return Some(());
    }
    if scope.is_execution_terminating() {
        return None;
    }
    fn private(scope: &mut v8::PinScope, name: &str) -> Option<v8::Global<v8::Private>> {
        let name = v8::String::new(scope, name)?;
        let key = v8::Private::for_api(scope, Some(name));
        Some(v8::Global::new(scope, key))
    }
    let content = private(scope, "turbo.template.content")?;
    let host = private(scope, "turbo.template.host")?;
    let associated_document = private(scope, "turbo.template.associated-document")?;
    // Primitive private keys are context-free; this slot never retains a realm.
    scope.set_slot(std::rc::Rc::new(Keys {
        content,
        host,
        associated_document,
    }));
    Some(())
}
fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> Option<v8::Local<'s, v8::Private>> {
    if scope.is_execution_terminating() {
        return None;
    }
    let keys = scope.get_slot::<std::rc::Rc<Keys>>()?.clone();
    let field = match name {
        "turbo.template.content" => &keys.content,
        "turbo.template.host" => &keys.host,
        "turbo.template.associated-document" => &keys.associated_document,
        _ => return None,
    };
    Some(v8::Local::new(scope, field))
}
fn edge<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<v8::Local<'s, v8::Object>> {
    let private = key(scope, name)?;
    object
        .get_private(scope, private)
        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
}
fn put(
    scope: &mut v8::PinScope,
    object: v8::Local<v8::Object>,
    name: &str,
    value: v8::Local<v8::Object>,
) -> Option<()> {
    let private = key(scope, name)?;
    (object.set_private(scope, private, value.into()) == Some(true)).then_some(())
}
fn template(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> bool {
    let _owner = DomScope::for_object(scope, object);
    handle_of(scope, object)
        .and_then(|node| {
            with_tree(|tree| {
                tree.node_type_id(node) == 1
                    && tree.namespace_id(node) == 0
                    && tree.local_name(node) == Some("template")
            })
        })
        .unwrap_or(false)
}
fn document<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let _owner = DomScope::for_object(scope, node);
    let root = with_tree(Tree::root)?;
    Some(wrap(scope, root))
}
fn associated<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    owner: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    let is_associated = {
        let _owner = DomScope::for_object(scope, owner);
        DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.template_contents.associated)
        })
    };
    if is_associated {
        return Some(owner);
    }
    if let Some(document) = edge(scope, owner, "turbo.template.associated-document") {
        return Some(document);
    }
    let context = owner.get_creation_context(scope)?;
    let scope = &mut v8::ContextScope::new(scope, context);
    let mut tree = Tree::parse("<!doctype html>");
    let root = tree.root();
    for child in tree.children(root) {
        tree.remove_child(root, child)
    }
    let document = parse_html_tree(scope, tree, false);
    {
        let _owner = DomScope::for_object(scope, document);
        DOM.with(|dom| {
            let mut dom = dom.borrow_mut();
            let state = &mut dom.as_mut()?.template_contents;
            state.associated = true;
            state.enabled = true;
            Some(())
        });
    }
    dom_implementation::initialize(scope, document, "text/html")?;
    put(scope, owner, "turbo.template.associated-document", document)?;
    Some(document)
}
pub(super) fn ensure<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: v8::Local<'s, v8::Object>,
) -> Option<v8::Local<'s, v8::Object>> {
    if scope.is_execution_terminating() || !template(scope, host) {
        return None;
    }
    if let Some(content) = edge(scope, host, "turbo.template.content") {
        return Some(content);
    }
    let fragment = {
        let _owner = DomScope::for_object(scope, host);
        let node = handle_of(scope, host)?;
        let enabled = DOM.with(|dom| {
            dom.borrow()
                .as_ref()
                .is_some_and(|dom| dom.template_contents.enabled)
        });
        if !enabled {
            return None;
        }
        let fragment = with_tree_mut(|tree| {
            tree.take_template_contents(node)
                .unwrap_or_else(|| tree.create_document_fragment())
        })?;
        wrap(scope, fragment)
    };
    let owner = document(scope, host)?;
    let target = associated(scope, owner)?;
    // Publish both private edges before migration can wrap a nested template.
    put(scope, host, "turbo.template.content", fragment)?;
    put(scope, fragment, "turbo.template.host", host)?;
    document_basics::adopt(scope, target, fragment).ok()?;
    Some(fragment)
}
fn children<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    node: v8::Local<'s, v8::Object>,
) -> Vec<v8::Local<'s, v8::Object>> {
    let _owner = DomScope::for_object(scope, node);
    let Some(handle) = handle_of(scope, node) else {
        return Vec::new();
    };
    with_tree(|tree| tree.children(handle))
        .unwrap_or_default()
        .into_iter()
        .map(|child| wrap(scope, child))
        .collect()
}
pub(super) fn parser_policy_nodes<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: v8::Local<'s, v8::Object>,
) -> Vec<v8::Local<'s, v8::Object>> {
    let mut pending = vec![root];
    let mut ordered = Vec::new();
    while let Some(node) = pending.pop() {
        if scope.is_execution_terminating() {
            break;
        }
        let mut descendants = children(scope, node);
        if let Some(content) = edge(scope, node, "turbo.template.content") {
            descendants.extend(children(scope, content));
        }
        pending.extend(descendants.into_iter().rev());
        ordered.push(node);
    }
    ordered
}
pub(super) fn realize<'s>(scope: &mut v8::PinScope<'s, '_>, root: v8::Local<'s, v8::Object>) {
    let mut pending = vec![root];
    let mut ordered = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = pending.pop() {
        if scope.is_execution_terminating() {
            return;
        }
        let Some(handle) = handle_of(scope, node) else {
            continue;
        };
        if !seen.insert((dom_id_of(scope, node), handle)) {
            continue;
        }
        pending.extend(children(scope, node));
        ordered.push(node);
    }
    // Extract inner reserved fragments first: generic native insertion expands a
    // Fragment, so its private parser edge must never enter ordinary adoption.
    for node in ordered.into_iter().rev() {
        ensure(scope, node);
    }
}
pub(super) fn mark_parsed<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: v8::Local<'s, v8::Object>,
    inert: bool,
) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if scope.is_execution_terminating() {
            return;
        }
        if let Some(content) = edge(scope, node, "turbo.template.content") {
            pending.push(content)
        }
        pending.extend(children(scope, node));
        custom_elements::parsed(scope, node);
        let _owner = DomScope::for_object(scope, node);
        if let Some(handle) = handle_of(scope, node) {
            form_checked_state::refresh(handle);
            if with_tree(|tree| {
                tree.node_type_id(handle) == 1
                    && tree.namespace_id(handle) == 0
                    && tree.local_name(handle) == Some("script")
            })
            .unwrap_or(false)
            {
                script_state::parsed(handle, inert)
            }
        }
    }
    let _owner = DomScope::for_object(scope, root);
    if let Some(handle) = handle_of(scope, root) {
        form_checked_state::inserted(handle);
    }
}

fn get<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    if let Some(content) = ensure(scope, args.this()) {
        realize(scope, content);
        result.set(content.into());
        return;
    }
    if let Some(message) = v8::String::new(scope, "Illegal invocation") {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    if capture_keys(scope).is_none() {
        return;
    }
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__templateContentsNative", get);
    run_js(scope, include_str!("template_contents.js"));
    DOM.with(|dom| {
        if let Some(dom) = dom.borrow_mut().as_mut() {
            dom.template_contents.enabled = true
        }
    });
    let Some(root) = with_tree(Tree::root) else {
        return;
    };
    let document = wrap(scope, root);
    realize(scope, document);
}
pub(super) fn set_html<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: v8::Local<'s, v8::Object>,
    html: &[u16],
) -> bool {
    let Some(content) = ensure(scope, host) else {
        return false;
    };
    let source = {
        let _owner = DomScope::for_object(scope, host);
        // Parse without observable active-document mutations. Its elements must
        // reach the template's inert owner before custom-element upgrade checks.
        let scripting_enabled = node_has_window_owner(scope, host);
        let placeholder = handle_of(scope, host).and_then(|context| {
            with_tree_mut(|tree| {
                tree.create_contextual_fragment_utf16(context, html, scripting_enabled)
            })
        });
        let Some(fragment) = placeholder else {
            return true;
        };
        wrap(scope, fragment)
    };
    realize(scope, source);
    let _target = DomScope::for_object(scope, content);
    let Some(target) = handle_of(scope, content) else {
        return true;
    };
    html_popover::disconnect(scope, target, false);
    range_live::replacing_children(scope, target);
    let existing = with_tree(|tree| tree.children(target)).unwrap_or_default();
    let incoming = children(scope, source)
        .into_iter()
        .map(dom_mutations::Incoming::Node)
        .collect::<Vec<_>>();
    dom_mutations::replace_inputs(scope, target, &incoming, &existing);
    realize(scope, content);
    mark_parsed(scope, content, true);
    true
}
thread_local! {static RELOCATING:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};static RELOCATIONS:RefCell<Vec<v8::Weak<v8::Object>>>=const{RefCell::new(Vec::new())};}
pub(super) fn relocate<'s>(scope: &mut v8::PinScope<'s, '_>, root: v8::Local<'s, v8::Object>) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if scope.is_execution_terminating() {
            return;
        }
        if template(scope, node) {
            RELOCATIONS.with(|queue| queue.borrow_mut().push(v8::Weak::new(scope, node)));
        }
        pending.extend(children(scope, node));
    }
    if RELOCATING.with(|busy| busy.replace(true)) {
        return;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            RELOCATING.with(|busy| busy.set(false));
            RELOCATIONS.with(|queue| queue.borrow_mut().clear());
        }
    }
    let _reset = Reset;
    loop {
        let weak = RELOCATIONS.with(|queue| queue.borrow_mut().pop());
        let Some(weak) = weak else { break };
        let Some(host) = weak.to_local(scope) else {
            continue;
        };
        let Some(content) = edge(scope, host, "turbo.template.content") else {
            continue;
        };
        let Some(owner) = document(scope, host) else {
            continue;
        };
        let Some(target) = associated(scope, owner) else {
            continue;
        };
        if dom_id_of(scope, content) != dom_id_of(scope, target)
            && document_basics::adopt(scope, target, content).is_err()
        {
            return;
        }
    }
}
pub(super) fn copy_contents<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Object>,
    target: v8::Local<'s, v8::Object>,
    deep: bool,
) {
    if !deep {
        return;
    }
    let mut pending = vec![(source, target, false)];
    while let Some((source, target, copy_children)) = pending.pop() {
        if scope.is_execution_terminating() {
            return;
        }
        if let (Some(source_content), Some(target_content)) =
            (ensure(scope, source), ensure(scope, target))
        {
            pending.push((source_content, target_content, true));
        }
        let source_children = children(scope, source);
        if copy_children {
            let Some(owner) = document(scope, target) else {
                return;
            };
            let _target = DomScope::for_object(scope, target);
            let Some(parent) = handle_of(scope, target) else {
                return;
            };
            for child in source_children {
                let Ok(copy) = import_document_node(scope, owner, child, false) else {
                    return;
                };
                let Some(node) = handle_of(scope, copy) else {
                    return;
                };
                with_tree_mut(|tree| tree.append_child(parent, node));
                pending.push((child, copy, true));
            }
        } else {
            for pair in source_children.into_iter().zip(children(scope, target)) {
                pending.push((pair.0, pair.1, false));
            }
        }
    }
}
pub(super) fn host_ancestor<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    parent: v8::Local<'s, v8::Object>,
    incoming: v8::Local<'s, v8::Object>,
) -> bool {
    let identity = (dom_id_of(scope, incoming), handle_of(scope, incoming));
    let mut current = Some(parent);
    let mut seen = std::collections::HashSet::new();
    while let Some(node) = current {
        let pair = (dom_id_of(scope, node), handle_of(scope, node));
        if pair == identity || !seen.insert(pair) {
            return true;
        }
        let parent = {
            let _owner = DomScope::for_object(scope, node);
            handle_of(scope, node)
                .and_then(|handle| {
                    with_tree(|tree| tree.parent(handle).or_else(|| tree.shadow_host(handle)))
                })
                .flatten()
                .map(|parent| wrap(scope, parent))
        };
        current = parent.or_else(|| edge(scope, node, "turbo.template.host"));
    }
    false
}
fn scalar(out: &mut Vec<u16>, text: &str) {
    out.extend(text.encode_utf16())
}
fn escaped(out: &mut Vec<u16>, units: &[u16], attribute: bool) {
    for &unit in units {
        let text = match unit {
            38 => Some("&amp;"),
            160 => Some("&nbsp;"),
            34 if attribute => Some("&quot;"),
            60 if !attribute => Some("&lt;"),
            62 if !attribute => Some("&gt;"),
            _ => None,
        };
        if let Some(text) = text {
            scalar(out, text)
        } else {
            out.push(unit)
        }
    }
}
pub(super) fn serialize<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: v8::Local<'s, v8::Object>,
    outer: bool,
) -> Vec<u16> {
    serialize_with(scope, root, outer, None)
}
/// Serialize with `getHTML` shadow options; hosts whose root is selected emit a
/// declarative `<template>` before their light children.
pub(super) fn serialize_with<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: v8::Local<'s, v8::Object>,
    outer: bool,
    shadows: Option<&shadow_html::Serialization<'s>>,
) -> Vec<u16> {
    enum Task<'s> {
        Node(v8::Local<'s, v8::Object>, bool),
        Close(Vec<u16>),
    }
    let mut out = Vec::new();
    let mut pending = Vec::new();
    // noscript text is literal when scripting is enabled for the node.
    let scripting = node_has_window_owner(scope, root);
    let raw = |tag: &str| raw(tag) || scripting && tag == "noscript";
    if outer {
        pending.push(Task::Node(root, false))
    } else {
        let source = ensure(scope, root).unwrap_or(root);
        let raw = {
            let _owner = DomScope::for_object(scope, root);
            handle_of(scope, root)
                .and_then(|node| with_tree(|tree| tree.local_name(node).map(raw)))
                .flatten()
                .unwrap_or(false)
        };
        for child in children(scope, source).into_iter().rev() {
            pending.push(Task::Node(child, raw));
        }
        if let Some(options) = shadows {
            shadow(scope, root, options, &mut pending, &mut out);
        }
    }
    fn shadow<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        host: v8::Local<'s, v8::Object>,
        options: &shadow_html::Serialization<'s>,
        pending: &mut Vec<Task<'s>>,
        out: &mut Vec<u16>,
    ) {
        if let Some((root, tag)) = shadow_html::serialized_root(scope, host, options) {
            pending.push(Task::Close("template".encode_utf16().collect()));
            for child in children(scope, root).into_iter().rev() {
                pending.push(Task::Node(child, false));
            }
            out.extend(tag);
        }
    }
    while let Some(task) = pending.pop() {
        if scope.is_execution_terminating() {
            return Vec::new();
        }
        let (node, parent_raw) = match task {
            Task::Node(node, raw) => (node, raw),
            Task::Close(tag) => {
                scalar(&mut out, "</");
                out.extend(tag);
                scalar(&mut out, ">");
                continue;
            }
        };
        let _owner = DomScope::for_object(scope, node);
        let Some(handle) = handle_of(scope, node) else {
            continue;
        };
        let Some(kind) = with_tree(|tree| tree.node_type_id(handle)) else {
            continue;
        };
        match kind {
            1 => {
                let Some((name, local, html, attributes)) = with_tree(|tree| {
                    (
                        tree.qualified_name_utf16(handle).unwrap_or_default(),
                        tree.local_name(handle).unwrap_or("").to_owned(),
                        tree.namespace_id(handle) == 0,
                        tree.attributes_units(handle),
                    )
                }) else {
                    continue;
                };
                scalar(&mut out, "<");
                out.extend_from_slice(&name);
                if !attributes
                    .iter()
                    .any(|attribute| attribute.qualified_name == [105, 115])
                {
                    if let Some(value) = custom_elements::is_value(scope, node) {
                        scalar(&mut out, " is=\"");
                        escaped(&mut out, &value, true);
                        scalar(&mut out, "\"");
                    }
                }
                for attribute in attributes {
                    scalar(&mut out, " ");
                    out.extend(attribute.qualified_name);
                    scalar(&mut out, "=\"");
                    escaped(&mut out, &attribute.value, true);
                    scalar(&mut out, "\"");
                }
                scalar(&mut out, ">");
                if html && void(&local) {
                    continue;
                }
                pending.push(Task::Close(name));
                let source = ensure(scope, node).unwrap_or(node);
                for child in children(scope, source).into_iter().rev() {
                    pending.push(Task::Node(child, html && raw(&local)));
                }
                if let Some(options) = shadows {
                    shadow(scope, node, options, &mut pending, &mut out);
                }
            }
            3 => {
                let value = with_tree(|tree| tree.node_value_utf16(handle))
                    .flatten()
                    .unwrap_or_default();
                if parent_raw {
                    out.extend(value)
                } else {
                    escaped(&mut out, &value, false)
                }
            }
            7 => {
                scalar(&mut out, "<?");
                out.extend(
                    with_tree(|tree| tree.processing_instruction_target_utf16(handle))
                        .flatten()
                        .unwrap_or_default(),
                );
                scalar(&mut out, " ");
                out.extend(
                    with_tree(|tree| tree.node_value_utf16(handle))
                        .flatten()
                        .unwrap_or_default(),
                );
                scalar(&mut out, "?>");
            }
            8 => {
                scalar(&mut out, "<!--");
                out.extend(
                    with_tree(|tree| tree.node_value_utf16(handle))
                        .flatten()
                        .unwrap_or_default(),
                );
                scalar(&mut out, "-->");
            }
            10 => {
                scalar(&mut out, "<!DOCTYPE ");
                out.extend(
                    with_tree(|tree| tree.doctype_name_utf16(handle))
                        .flatten()
                        .unwrap_or_default(),
                );
                scalar(&mut out, ">");
            }
            9 | 11 => {
                for child in children(scope, node).into_iter().rev() {
                    pending.push(Task::Node(child, false));
                }
            }
            _ => {}
        }
    }
    out
}
fn raw(tag: &str) -> bool {
    matches!(
        tag,
        "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
    )
}
pub(super) fn void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbo_dom_parser::rtdom::tree::CollectionBudget;

    fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
        v8::tc_scope!(let scope,scope);
        let code = v8::String::new(scope, source).unwrap();
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
    }
    fn collect(scope: &mut v8::PinScope) {
        collect_native_storage(
            scope,
            CollectionBudget {
                max_nodes: 100000,
                max_edges: 1000000,
                max_payload_bytes: 16 * 1024 * 1024,
            },
            32,
        )
        .unwrap();
    }
    #[test]
    fn context_free_template_private_keys_do_not_retain_realms() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let weak = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            assert!(capture_keys(scope).is_some());
            v8::Weak::new(scope, context)
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(weak.is_empty());
    }
    #[test]
    fn interrupted_template_private_lookup_does_not_allocate_or_publish() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        capture_keys(scope).unwrap();
        v8::tc_scope!(let scope,scope);
        let code = v8::String::new(scope, "1").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        scope.terminate_execution();
        let _ = script.run(scope);
        assert!(scope.has_terminated() || scope.is_execution_terminating());
        assert!(key(scope, "turbo.template.content").is_none());
        scope.cancel_terminate_execution();
        scope.reset();
        assert!(key(scope, "turbo.template.content").is_some());
    }
    #[test]
    fn associated_template_document_uses_new_owner_realm_after_foreign_adoption() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let main = v8::Context::new(scope, Default::default());
        let foreign = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, main);
        let host = {
            let scope = &mut v8::ContextScope::new(scope, main);
            install_independent_html(scope,"<!doctype html><body><template id=host><button onclick='return 5'>five</button></template>");
            eval(scope, "document.getElementById('host')")
                .to_object(scope)
                .unwrap()
        };
        let initial = ensure(scope, host).unwrap();
        let foreign_document = {
            let scope = &mut v8::ContextScope::new(scope, foreign);
            install_independent_html(scope, "<!doctype html><body>");
            eval(scope, "document").to_object(scope).unwrap()
        };
        {
            let scope = &mut v8::ContextScope::new(scope, main);
            document_basics::adopt(scope, foreign_document, host).unwrap();
        }
        let owner = document(scope, initial).unwrap();
        assert_eq!(owner.get_creation_context(scope), Some(foreign));
        assert_eq!(ensure(scope, host), Some(initial));
        let host_owner = document(scope, host).unwrap();
        assert_eq!(host_owner, foreign_document);
        {
            let scope = &mut v8::ContextScope::new(scope, main);
            clear_context_capabilities(scope);
        }
        {
            let scope = &mut v8::ContextScope::new(scope, foreign);
            clear_context_capabilities(scope);
        }
        reset();
    }
    #[test]
    fn template_native_graph_retains_connected_and_held_content_then_collects_cycles() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let (context, weak_host, weak_content, weak_child) = {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope,"<!doctype html><body><template id=host><b id=inside>keep</b><i>sibling</i></template>");
            let host = eval(scope, "document.getElementById('host')")
                .to_object(scope)
                .unwrap();
            let content = ensure(scope, host).unwrap();
            let child = children(scope, content)[0];
            (
                v8::Global::new(scope, context),
                v8::Weak::new(scope, host),
                v8::Weak::new(scope, content),
                v8::Weak::new(scope, child),
            )
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(!weak_host.is_empty());
        assert!(!weak_content.is_empty());
        assert!(!weak_child.is_empty());
        {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            collect(scope);
            assert!(eval(
                scope,
                "document.getElementById('host').content.firstChild.textContent==='keep'"
            )
            .is_true());
            eval(scope,"globalThis.held=document.getElementById('host').content.firstChild;document.getElementById('host').remove()");
        }
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(!weak_host.is_empty());
        assert!(!weak_content.is_empty());
        assert!(!weak_child.is_empty());
        {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            collect(scope);
            assert!(eval(scope,"held.parentNode.lastChild.textContent==='sibling'&&held.parentNode.firstChild===held&&held.ownerDocument.defaultView===null").is_true());
            eval(scope, "held=null");
        }
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        assert!(weak_host.is_empty());
        assert!(weak_content.is_empty());
        assert!(weak_child.is_empty());
        {
            v8::scope!(let scope,&mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            collect(scope);
            clear_context_capabilities(scope);
        }
        drop(context);
        clear_interface_records(&mut isolate);
        reset();
    }
    #[test]
    fn native_deep_clone_and_serialization_use_heap_traversals() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let source = format!("{}x{}", "<div>".repeat(8192), "</div>".repeat(8192));
                let tree = Tree::parse(&source);
                // The canonical cross-document seam must not recursively own or drop snapshots.
                let mut owner = tree;
                let root = owner.query_selector("body").unwrap();
                let first = owner.first_child(root).unwrap();
                let copy = clone_subtree(&mut owner, first, true);
                assert_eq!(owner.text_content(copy), "x");
                assert_eq!(
                    turbo_dom_parser::rtdom::serialize::serialize_outer_utf16(&owner, copy),
                    source.encode_utf16().collect::<Vec<_>>()
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn native_template_lifecycle_queues_parsing_and_handler_activation_match_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body>");
        let source = include_str!("template_lifecycle_probe.js");
        let source = source[..source.find("  const scriptRuns=").unwrap()].to_string()
            + "t.remove();button.remove();return {rows,observer,unreadHandler,parse};})()";
        let code = v8::String::new(scope, &format!("JSON.stringify({source})")).unwrap();
        v8::tc_scope!(let scope,scope);
        let result = v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| {
                panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope))
            });
        let actual: serde_json::Value =
            serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("template_lifecycle_chrome155.json")).unwrap();
        for key in ["rows", "observer", "unreadHandler", "parse"] {
            assert_eq!(actual[key], oracle["result"][key], "{key}")
        }
        reset();
    }
    #[test]
    fn native_template_ownership_clone_adoption_and_source_match_independent_chrome() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope,isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<!doctype html><body>");
        let source = format!(
            "JSON.stringify({})",
            include_str!("template_ownership_probe.js")
        );
        let code = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope,scope);
        let result = v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| {
                panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope))
            });
        let actual: serde_json::Value =
            serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("template_ownership_chrome155.json")).unwrap();
        assert_eq!(actual, oracle["result"]);
        reset();
    }
}

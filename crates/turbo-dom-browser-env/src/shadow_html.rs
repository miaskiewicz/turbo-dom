//! Shadow-root init state, declarative shadow parsing, clonable-root copies and
//! shadow-aware HTML serialization (`getHTML` / `setHTMLUnsafe`).
use super::*;

const KEY_NAMES: [&str; 4] = ["clonable", "serializable", "delegates-focus", "declarative"];
struct Keys(Vec<v8::Global<v8::Private>>);
fn key<'s>(scope: &mut v8::PinScope<'s, '_>, name: &str) -> Option<v8::Local<'s, v8::Private>> {
    // Captured before guest execution, like the slot keys: flag reads never
    // allocate strings after the watchdog terminates V8.
    let keys = scope.get_slot::<std::rc::Rc<Keys>>()?.clone();
    let index = KEY_NAMES.iter().position(|key| *key == name)?;
    Some(v8::Local::new(scope, &keys.0[index]))
}
fn flag(scope: &mut v8::PinScope, root: v8::Local<v8::Object>, name: &str) -> bool {
    let Some(private) = key(scope, name) else {
        return false;
    };
    root.get_private(scope, private)
        .is_some_and(|value| value.is_true())
}
fn set_flag(scope: &mut v8::PinScope, root: v8::Local<v8::Object>, name: &str, value: bool) {
    if let Some(private) = key(scope, name) {
        let value = v8::Boolean::new(scope, value);
        root.set_private(scope, private, value.into());
    }
}
#[derive(Clone, Copy, Default)]
struct Init {
    clonable: bool,
    serializable: bool,
    delegates_focus: bool,
    declarative: bool,
}
fn store(scope: &mut v8::PinScope, root: v8::Local<v8::Object>, init: Init) {
    set_flag(scope, root, "clonable", init.clonable);
    set_flag(scope, root, "serializable", init.serializable);
    set_flag(scope, root, "delegates-focus", init.delegates_focus);
    set_flag(scope, root, "declarative", init.declarative);
}
fn load(scope: &mut v8::PinScope, root: v8::Local<v8::Object>) -> Init {
    Init {
        clonable: flag(scope, root, "clonable"),
        serializable: flag(scope, root, "serializable"),
        delegates_focus: flag(scope, root, "delegates-focus"),
        declarative: flag(scope, root, "declarative"),
    }
}
fn mode_of(root: Handle) -> Option<String> {
    DOM.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.shadow_modes.get(&root).cloned())
    })
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
fn top(tree: &Tree, node: Handle) -> Handle {
    let mut cursor = node;
    while let Some(parent) = tree.parent(cursor) {
        cursor = parent;
    }
    cursor
}
fn html(tree: &Tree, node: Handle, name: &str) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some(name)
}

/// The DOM "valid shadow host name" set, independent of any existing root.
pub(super) fn valid_host(tree: &Tree, host: Handle) -> bool {
    let tag = tree.local_name(host).unwrap_or("");
    tree.node_type_id(host) == 1
        && tree.namespace_id(host) == 0
        && (tag.contains('-')
            || matches!(
                tag,
                "article"
                    | "aside"
                    | "blockquote"
                    | "body"
                    | "div"
                    | "footer"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "header"
                    | "main"
                    | "nav"
                    | "p"
                    | "section"
                    | "span"
            ))
}

/// `attachShadow` on an existing host. A declarative root of the same mode is
/// emptied and handed back once; every other case throws NotSupportedError.
pub(super) fn reattach<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    root: Handle,
    mode: &str,
) -> Option<v8::Local<'s, v8::Object>> {
    let object = wrap(scope, root);
    if !flag(scope, object, "declarative") {
        throw_dom_error(
            scope,
            "NotSupportedError",
            "Failed to execute 'attachShadow' on 'Element': Shadow root cannot be created on a host which already hosts a shadow tree.",
        );
        return None;
    }
    if mode_of(root).as_deref() != Some(mode) {
        throw_dom_error(
            scope,
            "NotSupportedError",
            "Failed to execute 'attachShadow' on 'Element': The requested mode does not match the existing declarative shadow root's mode",
        );
        return None;
    }
    let existing = with_tree(|tree| tree.children(root)).unwrap_or_default();
    if !existing.is_empty() && !dom_mutations::replace_inputs(scope, root, &[], &existing) {
        return None;
    }
    set_flag(scope, object, "declarative", false);
    Some(object)
}

/// Record the normalized `ShadowRootInit` booleans of an imperative root.
pub(super) fn configure(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
    options: v8::Local<v8::Object>,
) {
    let read = |name: &str| {
        let Some(key) = v8::String::new(scope, name) else {
            return false;
        };
        options
            .get(scope, key.into())
            .is_some_and(|value| value.boolean_value(scope))
    };
    let init = Init {
        clonable: read("clonable"),
        serializable: read("serializable"),
        delegates_focus: read("delegatesFocus"),
        declarative: false,
    };
    store(scope, root, init);
    reference_target::configure(scope, root, options);
}

/// Attach a native root to `host` in the current document scope.
fn attach<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: Handle,
    mode: &str,
    manual: bool,
    init: Init,
) -> Option<v8::Local<'s, v8::Object>> {
    let root = with_tree_mut(|tree| tree.attach_shadow(host))?;
    DOM.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.shadow_modes.insert(root, mode.to_owned());
        }
    });
    let object = wrap(scope, root);
    shadow_slots::configure(scope, object, manual);
    store(scope, object, init);
    Some(object)
}

/// Clone steps for shadow hosts: every clonable root in the copied subtree is
/// re-attached to its copy, and its children are always copied deeply.
pub(super) fn clone_roots<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Object>,
    clone: v8::Local<'s, v8::Object>,
    deep: bool,
) {
    let mut pending = vec![(source, clone)];
    while let Some((source, clone)) = pending.pop() {
        if scope.is_execution_terminating() {
            return;
        }
        let shadow = {
            let _source = DomScope::for_object(scope, source);
            handle_of(scope, source)
                .and_then(|host| with_tree(|tree| tree.shadow_root(host)).flatten())
                .map(|root| {
                    let object = wrap(scope, root);
                    let manual = shadow_slots::manual(scope, root);
                    (object, mode_of(root).unwrap_or_default(), manual)
                })
        };
        if let Some((root, mode, manual)) = shadow {
            let init = load(scope, root);
            let target = reference_target::object_target(scope, root);
            if init.clonable {
                let copy = {
                    let _clone = DomScope::for_object(scope, clone);
                    let host = handle_of(scope, clone).filter(|host| {
                        with_tree(|tree| tree.shadow_root(*host).is_none()).unwrap_or(false)
                    });
                    host.and_then(|host| {
                        let copy = attach(scope, host, &mode, manual, init)?;
                        reference_target::declare(scope, copy, target.clone());
                        let document = wrap(scope, with_tree(Tree::root)?);
                        Some((copy, document))
                    })
                };
                if let Some((copy, document)) = copy {
                    for child in children(scope, root) {
                        let Ok(node) = import_document_node(scope, document, child, true) else {
                            return;
                        };
                        let _clone = DomScope::for_object(scope, copy);
                        let (Some(parent), Some(node)) =
                            (handle_of(scope, copy), handle_of(scope, node))
                        else {
                            return;
                        };
                        with_tree_mut(|tree| tree.append_child(parent, node));
                    }
                }
            }
        }
        if deep {
            let from = children(scope, source);
            let to = children(scope, clone);
            pending.extend(from.into_iter().zip(to));
        }
    }
}

/// A shadow root the parser attached for a `<template shadowrootmode>`.
pub(super) struct Declared {
    root: Handle,
    mode: String,
    manual: bool,
    init: Init,
    target: Option<Vec<u16>>,
}
/// Convert parser-inserted `<template shadowrootmode>` elements below `parent`
/// (a detached fragment or a freshly parsed document), in tree order, exactly
/// where the parser would attach. Top-level fragment templates have no element
/// parent and stay ordinary templates; template contents are descended into.
/// With `only`, just those templates (the sanitizer's survivors) may attach.
pub(super) fn declarative(
    tree: &mut Tree,
    parent: Handle,
    only: Option<&std::collections::HashSet<Handle>>,
) -> Vec<Declared> {
    let mut attached = Vec::new();
    let mut pending = tree.children(parent);
    pending.reverse();
    while let Some(node) = pending.pop() {
        let mode = html(tree, node, "template")
            .then(|| tree.get_attribute(node, "shadowrootmode"))
            .flatten()
            .map(|mode| mode.to_ascii_lowercase())
            .filter(|mode| mode == "open" || mode == "closed");
        let host = tree
            .parent(node)
            .filter(|host| valid_host(tree, *host) && tree.shadow_root(*host).is_none())
            .filter(|_| only.is_none_or(|only| only.contains(&node)));
        if let (Some(mode), Some(host)) = (mode, host) {
            let init = Init {
                clonable: tree.get_attribute(node, "shadowrootclonable").is_some(),
                serializable: tree.get_attribute(node, "shadowrootserializable").is_some(),
                delegates_focus: tree
                    .get_attribute(node, "shadowrootdelegatesfocus")
                    .is_some(),
                declarative: true,
            };
            let manual = tree
                .get_attribute(node, "shadowrootslotassignment")
                .is_some_and(|value| value.eq_ignore_ascii_case("manual"));
            let target = tree.get_attribute_ns_utf16(node, None, "shadowrootreferencetarget");
            let root = tree.attach_shadow(host);
            if let Some(content) = tree.take_template_contents(node) {
                for child in tree.children(content) {
                    tree.append_child(root, child);
                }
            }
            tree.remove_child(host, node);
            attached.push(Declared {
                root,
                mode,
                manual,
                init,
                target,
            });
            let mut nested = tree.children(root);
            nested.reverse();
            pending.extend(nested);
            continue;
        }
        // Parser template contents are fragment children until realization.
        let mut nested = tree.children(node);
        nested.reverse();
        pending.extend(nested);
    }
    attached
}
/// Give each converted root its mode, slot assignment and init flags in the
/// current document scope, returning the root wrappers in conversion order.
pub(super) fn register<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    declared: Vec<Declared>,
) -> Vec<v8::Local<'s, v8::Object>> {
    ensure_keys(scope);
    shadow_slots::ensure_keys(scope);
    let mut objects = Vec::with_capacity(declared.len());
    for Declared {
        root,
        mode,
        manual,
        init,
        target,
    } in declared
    {
        DOM.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                state.shadow_modes.insert(root, mode);
            }
        });
        let object = wrap(scope, root);
        shadow_slots::configure(scope, object, manual);
        store(scope, object, init);
        reference_target::declare(scope, object, target);
        objects.push(object);
    }
    objects
}

/// `setHTMLUnsafe` / `setHTML`: fragment-parse with declarative shadow roots
/// enabled, optionally sanitize the fragment (and its declarative roots), then
/// replace all children of the element, template contents or shadow root.
pub(super) fn set_html_unsafe<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    receiver: v8::Local<'s, v8::Object>,
    html: &[u16],
    sanitizer: Option<(&sanitizer::Config, bool)>,
) {
    let (context, target) = {
        let _owner = DomScope::for_object(scope, receiver);
        let Some(handle) = handle_of(scope, receiver) else {
            return;
        };
        let host = with_tree(|tree| tree.shadow_host(handle)).flatten();
        (host.unwrap_or(handle), host.is_none())
    };
    let target = if target {
        template_contents::ensure(scope, receiver).unwrap_or(receiver)
    } else {
        receiver
    };
    let context_object = {
        let _owner = DomScope::for_object(scope, receiver);
        wrap(scope, context)
    };
    let scripting_enabled = node_has_window_owner(scope, context_object);
    let (source, roots) = {
        let _owner = DomScope::for_object(scope, receiver);
        let parsed = with_tree_mut(|tree| {
            let fragment = tree.create_contextual_fragment_utf16(context, html, scripting_enabled);
            // The template elements (and their shadowroot* attributes) are
            // sanitized before their declarative shadow roots attach.
            let only =
                sanitizer.map(|(config, safe)| sanitizer::sanitize(tree, fragment, config, safe));
            let roots = declarative(tree, fragment, only.as_ref());
            (fragment, roots)
        });
        let Some((fragment, roots)) = parsed else {
            return;
        };
        let objects = register(scope, roots);
        (wrap(scope, fragment), objects)
    };
    // Innermost roots first: no raw parser content fragment may enter adoption.
    for root in roots.iter().rev() {
        template_contents::realize(scope, *root);
    }
    template_contents::realize(scope, source);
    let incoming = children(scope, source);
    {
        let _target = DomScope::for_object(scope, target);
        let Some(parent) = handle_of(scope, target) else {
            return;
        };
        html_popover::disconnect(scope, parent, false);
        range_live::replacing_children(scope, parent);
        let existing = with_tree(|tree| tree.children(parent)).unwrap_or_default();
        let nodes = incoming
            .iter()
            .map(|node| dom_mutations::Incoming::Node(*node))
            .collect::<Vec<_>>();
        if !dom_mutations::replace_inputs(scope, parent, &nodes, &existing) {
            return;
        }
        css_policy::invalidate_sheet(parent);
    }
    template_contents::realize(scope, target);
    for node in incoming.into_iter().chain(roots) {
        template_contents::mark_parsed(scope, node, true);
    }
}

/// Serialization options of `getHTML`.
pub(super) struct Serialization<'s> {
    pub serializable: bool,
    pub roots: Vec<v8::Local<'s, v8::Object>>,
}
/// The root and declarative start tag for a host whose root this call serializes.
pub(super) fn serialized_root<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    host: v8::Local<'s, v8::Object>,
    options: &Serialization<'s>,
) -> Option<(v8::Local<'s, v8::Object>, Vec<u16>)> {
    let _owner = DomScope::for_object(scope, host);
    let handle = handle_of(scope, host)?;
    let root = with_tree(|tree| tree.shadow_root(handle)).flatten()?;
    let object = wrap(scope, root);
    let init = load(scope, object);
    if !(options.serializable && init.serializable)
        && !options
            .roots
            .iter()
            .any(|listed| listed.strict_equals(object.into()))
    {
        return None;
    }
    let mut tag = format!("<template shadowrootmode=\"{}\"", mode_of(root)?);
    if init.delegates_focus {
        tag.push_str(" shadowrootdelegatesfocus=\"\"");
    }
    if init.serializable {
        tag.push_str(" shadowrootserializable=\"\"");
    }
    if init.clonable {
        tag.push_str(" shadowrootclonable=\"\"");
    }
    let mut tag: Vec<u16> = tag.encode_utf16().collect();
    if let Some(target) = reference_target::serialized(scope, object) {
        tag.extend(target);
    }
    tag.push(u16::from(b'>'));
    Some((object, tag))
}

/// DOM retargeting of `node` against `target` (both in the current tree).
pub(super) fn retarget(tree: &Tree, node: Handle, target: Handle) -> Handle {
    let mut node = node;
    loop {
        let root = top(tree, node);
        if !tree.is_shadow_root(root) {
            return node;
        }
        let mut cursor = Some(target);
        while let Some(current) = cursor {
            if current == root {
                return node;
            }
            cursor = tree.parent(current).or_else(|| tree.shadow_host(current));
        }
        let Some(host) = tree.shadow_host(root) else {
            return node;
        };
        node = host;
    }
}
/// `DocumentOrShadowRoot` element getters for a shadow root: the retargeted
/// candidate only when it lies in this root's own tree.
fn scoped(tree: &Tree, node: Handle, root: Handle) -> Option<Handle> {
    let candidate = retarget(tree, node, root);
    (top(tree, candidate) == root).then_some(candidate)
}

fn illegal(scope: &mut v8::PinScope) {
    if let Some(message) = v8::String::new(scope, "Illegal invocation") {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
}
fn shadow_root(scope: &mut v8::PinScope, object: v8::Local<v8::Object>) -> Option<Handle> {
    let _owner = DomScope::for_object(scope, object);
    handle_of(scope, object)
        .filter(|root| with_tree(|tree| tree.is_shadow_root(*root)) == Some(true))
}
fn native<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    if scope.is_execution_terminating() {
        return;
    }
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(object) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        illegal(scope);
        return;
    };
    match operation.as_str() {
        "clonable" | "serializable" | "delegatesFocus" => {
            if shadow_root(scope, object).is_none() {
                illegal(scope);
                return;
            }
            let name = if operation == "delegatesFocus" {
                "delegates-focus"
            } else {
                operation.as_str()
            };
            rv.set_bool(flag(scope, object, name));
        }
        "pointerLockElement" => {
            if shadow_root(scope, object).is_none() {
                illegal(scope);
                return;
            }
            let lock = pointer_events::lock_element_in(scope, object);
            let _owner = DomScope::for_object(scope, object);
            let value = wrap_opt(scope, lock);
            rv.set(value);
        }
        "activeElement" | "retarget" => {
            let Some(root) = shadow_root(scope, object) else {
                illegal(scope);
                return;
            };
            let _owner = DomScope::for_object(scope, object);
            let node = if operation == "activeElement" {
                ACTIVE
                    .with(|active| *active.borrow())
                    .filter(|node| with_tree(|tree| is_connected(tree, *node)).unwrap_or(false))
            } else {
                v8::Local::<v8::Object>::try_from(args.get(2))
                    .ok()
                    .filter(|node| dom_id_of(scope, *node) == dom_id_of(scope, object))
                    .and_then(|node| handle_of(scope, node))
            };
            let found = node.and_then(|node| with_tree(|tree| scoped(tree, node, root)).flatten());
            let value = wrap_opt(scope, found);
            rv.set(value);
        }
        "getHTML" => {
            let mut roots = Vec::new();
            if let Ok(list) = v8::Local::<v8::Array>::try_from(args.get(3)) {
                for index in 0..list.length() {
                    if let Some(root) = list
                        .get_index(scope, index)
                        .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
                    {
                        roots.push(root);
                    }
                }
            }
            let options = Serialization {
                serializable: args.get(2).is_true(),
                roots,
            };
            let void = {
                let _owner = DomScope::for_object(scope, object);
                handle_of(scope, object)
                    .and_then(|node| {
                        with_tree(|tree| {
                            tree.node_type_id(node) == 1
                                && tree.namespace_id(node) == 0
                                && template_contents::void(tree.local_name(node).unwrap_or(""))
                        })
                    })
                    .unwrap_or(false)
            };
            let html = if void {
                Vec::new()
            } else {
                template_contents::serialize_with(scope, object, false, Some(&options))
            };
            rv.set(unit_string(scope, &html).into());
        }
        "setHTMLUnsafe" => {
            // The markup already passed its Trusted Types check (sanitizer.js).
            let Some(html) = v8::Local::<v8::String>::try_from(args.get(2))
                .ok()
                .map(|html| string_code_units(scope, html))
            else {
                illegal(scope);
                return;
            };
            // SetHTMLUnsafeOptions.sanitizer, already converted and validated.
            let config = v8::Local::<v8::Object>::try_from(args.get(3))
                .ok()
                .and_then(|config| sanitizer::Config::read(scope, config));
            set_html_unsafe(
                scope,
                object,
                &html,
                config.as_ref().map(|config| (config, false)),
            );
        }
        _ => illegal(scope),
    }
}
/// Capture the root-flag keys (idempotent); parsing may attach roots before install.
pub(super) fn ensure_keys(scope: &mut v8::PinScope) {
    if scope.get_slot::<std::rc::Rc<Keys>>().is_none() {
        let mut keys = Vec::new();
        for name in KEY_NAMES {
            let Some(name) = v8::String::new(scope, &format!("turbo.shadow-root.{name}")) else {
                return;
            };
            let private = v8::Private::for_api(scope, Some(name));
            keys.push(v8::Global::new(scope, private));
        }
        scope.set_slot(std::rc::Rc::new(Keys(keys)));
    }
}
pub(super) fn install(scope: &mut v8::PinScope) {
    ensure_keys(scope);
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__shadowHtmlNative", native);
    run_js(scope, include_str!("shadow_html.js"));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_probe(scope: &mut v8::PinScope, probe: &str) -> serde_json::Value {
        let probe = probe.trim().trim_end_matches(';');
        let source = format!("(async()=>{{try{{globalThis.shadowHtmlResult=JSON.stringify(await({probe}));}}catch(error){{globalThis.shadowHtmlError=String(error&&error.stack||error);}}}})()");
        let code = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        scope.perform_microtask_checkpoint();
        let code = v8::String::new(
            scope,
            "globalThis.shadowHtmlResult || globalThis.shadowHtmlError || 'unsettled'",
        )
        .unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope);
        serde_json::from_str(&value).unwrap_or_else(|error| panic!("{error}: {value}"))
    }
    #[test]
    fn shadow_root_state_html_serialization_and_declarative_parsing_match_chrome156() {
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<!doctype html><html><head></head><body></body></html>",
        );
        let actual = run_probe(scope, include_str!("shadow_html_probe.js"));
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("shadow_html_chrome156.json")).unwrap();
        assert_eq!(oracle["browser"], "Chrome/156.0.8078.47");
        let expected = oracle["result"].as_object().unwrap();
        for (key, value) in expected {
            assert_eq!(&actual[key], value, "{key}");
        }
        assert_eq!(actual.as_object().unwrap().len(), expected.len());
        reset();
    }
    #[test]
    fn unreferenced_closed_root_init_survives_collection_for_clone_and_serialization() {
        super::super::tests::init_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        let context = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            install_with_html(scope, "<!doctype html><body></body>");
            let code = v8::String::new(scope, "(()=>{const host=document.createElement('div');host.id='host';document.body.append(host);host.attachShadow({mode:'closed',clonable:true,serializable:true,delegatesFocus:true}).innerHTML='<b>kept</b>';const declared=document.createElement('div');declared.id='declared';document.body.append(declared);declared.setHTMLUnsafe('<span><template shadowrootmode=open>d</template></span>');})()").unwrap();
            v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap();
            v8::Global::new(scope, context)
        };
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            collect_native_storage(
                scope,
                turbo_dom_parser::rtdom::tree::CollectionBudget {
                    max_nodes: 100000,
                    max_edges: 1000000,
                    max_payload_bytes: 16 * 1024 * 1024,
                },
                32,
            )
            .unwrap();
        }
        for _ in 0..3 {
            isolate.low_memory_notification();
        }
        {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Local::new(scope, &context);
            let scope = &mut v8::ContextScope::new(scope, context);
            let code = v8::String::new(scope, "(()=>{const host=document.getElementById('host');const copy=host.cloneNode();const declared=document.getElementById('declared').firstChild;const again=declared.attachShadow({mode:'open'});return host.getHTML({serializableShadowRoots:true})+'|'+copy.getHTML({serializableShadowRoots:true})+'|'+(again===declared.shadowRoot)+again.innerHTML;})()").unwrap();
            let value = v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
            let template = "<template shadowrootmode=\"closed\" shadowrootdelegatesfocus=\"\" shadowrootserializable=\"\" shadowrootclonable=\"\"><b>kept</b></template>";
            assert_eq!(value, format!("{template}|{template}|true"));
            clear_context_capabilities(scope);
        }
        drop(context);
        clear_interface_records(&mut isolate);
        reset();
    }
}

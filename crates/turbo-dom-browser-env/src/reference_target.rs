//! Shadow-root reference targets: `ShadowRoot.referenceTarget` state (imperative, declarative
//! and cloned roots) and the ID-reference resolution that follows a referenced shadow host
//! through its root's reference target to the inner element.
use super::*;

struct Key(v8::Global<v8::Private>);
fn key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    // Captured before guest execution, like the shadow-root flag keys.
    let key = scope.get_slot::<std::rc::Rc<Key>>()?.clone();
    Some(v8::Local::new(scope, &key.0))
}

/// The reference target of the shadow root `root`, wrapper-held like the other init state.
pub(super) fn target_of(scope: &mut v8::PinScope, root: Handle) -> Option<Vec<u16>> {
    let object = wrap(scope, root);
    object_target(scope, object)
}
/// The reference target held by a shadow-root wrapper of any document.
pub(super) fn object_target(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
) -> Option<Vec<u16>> {
    let private = key(scope)?;
    let value = root.get_private(scope, private)?;
    let value = v8::Local::<v8::String>::try_from(value).ok()?;
    Some(string_code_units(scope, value))
}
fn store(scope: &mut v8::PinScope, root: v8::Local<v8::Object>, value: Option<&[u16]>) {
    let Some(private) = key(scope) else {
        return;
    };
    let value: v8::Local<v8::Value> = match value {
        Some(units) => unit_string(scope, units).into(),
        None => v8::null(scope).into(),
    };
    root.set_private(scope, private, value);
}

/// `ShadowRootInit.referenceTarget`, already converted (`DOMString?`) by the attachShadow
/// dictionary reader in IDL member order.
pub(super) fn configure(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
    options: v8::Local<v8::Object>,
) {
    let Some(name) = v8::String::new(scope, "referenceTarget") else {
        return;
    };
    let value = options
        .get(scope, name.into())
        .and_then(|value| v8::Local::<v8::String>::try_from(value).ok())
        .map(|value| string_code_units(scope, value));
    store(scope, root, value.as_deref());
}
/// A declarative root's `shadowrootreferencetarget`, or a clonable root's copied target.
pub(super) fn declare(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
    value: Option<Vec<u16>>,
) {
    store(scope, root, value.as_deref());
}
/// The declarative serialization attribute, when the root has a reference target.
pub(super) fn serialized(
    scope: &mut v8::PinScope,
    root: v8::Local<v8::Object>,
) -> Option<Vec<u16>> {
    let value = object_target(scope, root)?;
    let mut tag: Vec<u16> = " shadowrootreferencetarget=\"".encode_utf16().collect();
    // Chrome escapes `<` and `>` in serialized attribute values too.
    for unit in value {
        match unit {
            0x26 => tag.extend("&amp;".encode_utf16()),
            0xa0 => tag.extend("&nbsp;".encode_utf16()),
            0x22 => tag.extend("&quot;".encode_utf16()),
            0x3c => tag.extend("&lt;".encode_utf16()),
            0x3e => tag.extend("&gt;".encode_utf16()),
            unit => tag.push(unit),
        }
    }
    tag.push(0x22);
    Some(tag)
}

fn first_with_id(tree: &Tree, root: Handle, id: &[u16]) -> Option<Handle> {
    let mut pending = tree.children(root);
    pending.reverse();
    while let Some(node) = pending.pop() {
        if tree.node_type_id(node) == 1
            && tree.get_attribute_ns_utf16(node, None, "id").as_deref() == Some(id)
        {
            return Some(node);
        }
        let mut children = tree.children(node);
        children.reverse();
        pending.extend(children);
    }
    None
}

/// Resolve an element found by an ID reference (or an explicit element reference): while it
/// hosts a root with a reference target, continue to that root's first element with the
/// target ID. A root without a reference target ends at its host. When the target ID is
/// missing, Chrome's invokers resolve to nothing (`strict`) while label association falls
/// back to the host itself.
pub(super) fn resolve(scope: &mut v8::PinScope, element: Handle, strict: bool) -> Option<Handle> {
    let mut current = element;
    // Every step enters a strictly deeper tree, so the chain is bounded by the tree depth.
    loop {
        let Some(root) = with_tree(|tree| tree.shadow_root(current)).flatten() else {
            return Some(current);
        };
        let Some(id) = target_of(scope, root) else {
            return Some(current);
        };
        match with_tree(|tree| first_with_id(tree, root, &id)).flatten() {
            Some(next) => current = next,
            None if strict => return None,
            None => return Some(current),
        }
    }
}

fn illegal(scope: &mut v8::PinScope) {
    if let Some(message) = v8::String::new(scope, "Illegal invocation") {
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error);
    }
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
        if operation == "resolve" {
            rv.set_null();
        } else {
            illegal(scope);
        }
        return;
    };
    let _owner = DomScope::for_object(scope, object);
    let Some(node) = handle_of(scope, object) else {
        if operation == "resolve" {
            rv.set_null();
        } else {
            illegal(scope);
        }
        return;
    };
    match operation.as_str() {
        "resolve" => {
            let found = resolve(scope, node, true);
            let value = wrap_opt(scope, found);
            rv.set(value);
        }
        "get" | "set" => {
            if with_tree(|tree| tree.is_shadow_root(node)) != Some(true) {
                illegal(scope);
                return;
            }
            if operation == "get" {
                match target_of(scope, node) {
                    Some(units) => rv.set(unit_string(scope, &units).into()),
                    None => rv.set_null(),
                }
                return;
            }
            let value = args.get(2);
            let units = v8::Local::<v8::String>::try_from(value)
                .ok()
                .map(|value| string_code_units(scope, value));
            store(scope, object, units.as_deref());
        }
        _ => illegal(scope),
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    if scope.get_slot::<std::rc::Rc<Key>>().is_none() {
        let Some(name) = v8::String::new(scope, "turbo.shadow-root.reference-target") else {
            return;
        };
        let private = v8::Private::for_api(scope, Some(name));
        let key = v8::Global::new(scope, private);
        scope.set_slot(std::rc::Rc::new(Key(key)));
    }
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__referenceTargetNative", native);
    run_js(scope, include_str!("reference_target.js"));
}

#[cfg(test)]
#[path = "probe_harness_tests.rs"]
pub(super) mod probe_harness;
#[cfg(test)]
#[path = "reference_target_tests.rs"]
mod tests;

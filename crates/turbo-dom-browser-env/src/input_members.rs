//! Native HTMLInputElement state Chromium keeps outside content attributes: the IDL-only
//! indeterminate flag (cleared by checkbox activation, copied by cloning) and `list` lookup.
use super::*;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct State {
    indeterminate: HashSet<Handle>,
}

fn html_input(tree: &Tree, node: Handle) -> bool {
    tree.node_type_id(node) == 1
        && tree.namespace_id(node) == 0
        && tree.local_name(node) == Some("input")
}
fn indeterminate(node: Handle) -> bool {
    DOM.with(|dom| {
        dom.borrow()
            .as_ref()
            .is_some_and(|state| state.input_members.indeterminate.contains(&node))
    })
}
fn set_indeterminate(node: Handle, value: bool) {
    DOM.with(|dom| {
        if let Some(state) = dom.borrow_mut().as_mut() {
            let flags = &mut state.input_members.indeterminate;
            if value {
                flags.insert(node);
            } else {
                flags.remove(&node);
            }
        }
    });
}

/// Checkbox legacy-pre-activation clears indeterminate; the previous flag restores a
/// canceled activation.
pub(super) fn pre_activation(node: Handle) -> bool {
    let previous = indeterminate(node);
    set_indeterminate(node, false);
    previous
}
pub(super) fn cancel_activation(node: Handle, previous: bool) {
    set_indeterminate(node, previous);
}

fn preorder(root: Handle) -> Vec<Handle> {
    with_tree(|tree| {
        let mut result = Vec::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            result.push(node);
            pending.extend(tree.children(node).into_iter().rev());
        }
        result
    })
    .unwrap_or_default()
}

/// Cloning copies indeterminate, including across documents (importNode).
pub(super) fn copy<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: v8::Local<'s, v8::Object>,
    clone: v8::Local<'s, v8::Object>,
    deep: bool,
) {
    let flags = {
        let _source = DomScope::for_object(scope, source);
        let Some(root) = handle_of(scope, source) else {
            return;
        };
        let nodes = if deep { preorder(root) } else { vec![root] };
        nodes.into_iter().map(indeterminate).collect::<Vec<_>>()
    };
    if !flags.contains(&true) {
        return;
    }
    let _clone = DomScope::for_object(scope, clone);
    let Some(root) = handle_of(scope, clone) else {
        return;
    };
    let nodes = if deep { preorder(root) } else { vec![root] };
    for (node, flag) in nodes.into_iter().zip(flags) {
        if flag && with_tree(|tree| html_input(tree, node)).unwrap_or(false) {
            set_indeterminate(node, true);
        }
    }
}

/// The first element in tree order of the input's tree (document, shadow root or detached
/// subtree) whose ID is the `list` attribute.
fn list_reference(tree: &Tree, node: Handle) -> Option<Handle> {
    let id = tree.get_attribute_utf16(node, "list")?;
    if id.is_empty() {
        return None;
    }
    let mut root = node;
    while let Some(parent) = tree.parent(root) {
        root = parent;
    }
    let mut pending = vec![root];
    while let Some(current) = pending.pop() {
        if tree.node_type_id(current) == 1
            && tree.get_attribute_utf16(current, "id").as_ref() == Some(&id)
        {
            return Some(current);
        }
        pending.extend(tree.children(current).into_iter().rev());
    }
    None
}
/// The suggestions source: the referenced element, when it (or, for a shadow host, its
/// reference target) is an HTML datalist. A host is exposed retargeted, as Chrome does.
fn datalist(scope: &mut v8::PinScope, node: Handle) -> Option<Handle> {
    let found = with_tree(|tree| list_reference(tree, node)).flatten()?;
    let actual = reference_target::resolve(scope, found, false)?;
    with_tree(|tree| tree.namespace_id(actual) == 0 && tree.local_name(actual) == Some("datalist"))
        .unwrap_or(false)
        .then_some(found)
}

pub(super) fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
    mut rv: v8::ReturnValue,
) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    let Ok(receiver) = v8::Local::<v8::Object>::try_from(args.get(1)) else {
        return;
    };
    let _dom = DomScope::for_object(scope, receiver);
    let Some(node) = handle_of(scope, receiver) else {
        return;
    };
    if !with_tree(|tree| html_input(tree, node)).unwrap_or(false) {
        return;
    }
    match operation.as_str() {
        "indeterminate" => rv.set_bool(indeterminate(node)),
        "set-indeterminate" => set_indeterminate(node, args.get(2).boolean_value(scope)),
        "list" => match datalist(scope, node) {
            Some(list) => rv.set(wrap(scope, list).into()),
            None => rv.set(v8::null(scope).into()),
        },
        _ => {}
    }
}

pub(super) fn install(scope: &mut v8::PinScope) {
    let global = scope.get_current_context().global(scope);
    bind_method(scope, global, "__inputNative", dispatch);
    run_js(scope, include_str!("input_members.js"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    // Leaf paths where the binding and the saved Chrome control disagree.
    fn differences(
        path: &str,
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        out: &mut Vec<String>,
    ) {
        use serde_json::Value::{Array, Object};
        match (actual, expected) {
            (Array(a), Array(b)) if a.len() == b.len() => {
                for (index, (a, b)) in a.iter().zip(b).enumerate() {
                    differences(&format!("{path}[{index}]"), a, b, out);
                }
            }
            (Object(a), Object(b)) if a.len() == b.len() => {
                for (key, b) in b {
                    match a.get(key) {
                        Some(a) => differences(&format!("{path}.{key}"), a, b, out),
                        None => out.push(format!("{path}.{key}: missing")),
                    }
                }
            }
            _ if actual != expected => out.push(format!("{path}: {actual} != {expected}")),
            _ => {}
        }
    }

    #[test]
    fn native_input_members_match_independent_chrome() {
        let cases = include_bytes!("input_members_cases.json");
        let probe = include_str!("input_members_probe.js");
        let control: serde_json::Value =
            serde_json::from_str(include_str!("input_members_chrome156.json")).unwrap();
        assert_eq!(control["browser"], "Chrome/156.0.8078.47");
        assert_eq!(control["v8"], "15.6.75.19");
        assert_eq!(control["errors"], serde_json::json!([]));
        assert_eq!(
            control["probeSha256"],
            format!("{:x}", Sha256::digest(probe.as_bytes()))
        );
        assert_eq!(
            control["casesSha256"],
            format!("{:x}", Sha256::digest(cases))
        );
        let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
        super::super::tests::init_v8();
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_independent_html(scope, cases[0]["html"].as_str().unwrap());
        v8::tc_scope!(let scope, scope);
        let source = v8::String::new(scope, probe).unwrap();
        let Some(result) = v8::Script::compile(scope, source, None).unwrap().run(scope) else {
            panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
        };
        let actual = v8::json::stringify(scope, result)
            .unwrap()
            .to_rust_string_lossy(scope);
        let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
        let mut out = Vec::new();
        differences("", &actual, &control["result"][0]["result"], &mut out);
        let shown = out.iter().take(60).cloned().collect::<Vec<_>>().join("\n");
        assert!(out.is_empty(), "{} differences:\n{shown}", out.len());
        reset();
    }
}

//! Label associations use the actual light-tree root, never guest selectors.
use super::*;
fn rows(root: Handle) -> Vec<Handle> {
    with_tree(|tree| {
        let mut pending = vec![root];
        let mut result = Vec::new();
        while let Some(node) = pending.pop() {
            result.push(node);
            pending.extend(tree.children(node).into_iter().rev());
        }
        result
    })
    .unwrap_or_default()
}
fn root(mut node: Handle) -> Handle {
    with_tree(|tree| {
        while let Some(parent) = tree.parent(node) {
            node = parent;
        }
        node
    })
    .unwrap_or(node)
}
pub(super) fn hidden(node: Handle) -> bool {
    with_tree(|tree| {
        tree.namespace_id(node) == 0
            && tree.local_name(node) == Some("input")
            && tree
                .get_attribute_ns_borrowed(node, None, "type")
                .is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
    })
    .unwrap_or(false)
}
fn labelable(scope: &mut v8::PinScope, node: Handle) -> bool {
    let kind = with_tree(|tree| {
        if tree.namespace_id(node) != 0 {
            return None;
        }
        let name = tree.local_name(node)?;
        if name == "input"
            && tree
                .get_attribute_ns_borrowed(node, None, "type")
                .is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
        {
            return None;
        }
        Some((
            matches!(
                name,
                "input" | "button" | "textarea" | "select" | "output" | "meter" | "progress"
            ),
            name.contains('-'),
        ))
    })
    .flatten();
    match kind {
        Some((true, _)) => true,
        Some((false, true)) => {
            let object = DOM
                .with(|dom| dom.borrow().as_ref()?.cache.get(&node).cloned())
                .and_then(|object| object.to_local(scope));
            let Some(object) = object else {
                return false;
            };
            let name = v8::String::new(scope, "turbo.forms.associated-record").unwrap();
            let key = v8::Private::for_api(scope, Some(name));
            object
                .get_private(scope, key)
                .is_some_and(|value| !value.is_undefined())
        }
        _ => false,
    }
}
/// The label's labeled control as exposed (retargeted into the label's tree) and the actual
/// element it labels: a `for` reference to a shadow host continues through reference targets.
fn association(scope: &mut v8::PinScope, label: Handle) -> Option<(Handle, Handle)> {
    if !with_tree(|tree| tree.namespace_id(label) == 0 && tree.local_name(label) == Some("label"))
        .unwrap_or(false)
    {
        return None;
    }
    if let Some(id) = with_tree(|tree| tree.get_attribute_ns_utf16(label, None, "for")).flatten() {
        if id.is_empty() {
            return None;
        }
        let target = rows(root(label)).into_iter().find(|node| {
            with_tree(|tree| {
                tree.get_attribute_ns_utf16(*node, None, "id").as_deref() == Some(id.as_slice())
            })
            .unwrap_or(false)
        })?;
        let actual = reference_target::resolve(scope, target, false)?;
        return labelable(scope, actual).then_some((target, actual));
    }
    // A wrapped shadow host labels its reference target.
    rows(label).into_iter().skip(1).find_map(|node| {
        let actual = reference_target::resolve(scope, node, false)?;
        labelable(scope, actual).then_some((node, actual))
    })
}
pub(super) fn control(scope: &mut v8::PinScope, label: Handle) -> Option<Handle> {
    association(scope, label).map(|(shown, _)| shown)
}
pub(super) fn labels(scope: &mut v8::PinScope, host: Handle) -> Vec<Handle> {
    if !labelable(scope, host) {
        return Vec::new();
    }
    // Labels in shadow-including ancestor trees reach `host` only through reference targets,
    // so only the roots of `host`'s host chain are entered, in shadow-including tree order.
    let mut chain = Vec::new();
    let mut outer = root(host);
    while let Some(next) = with_tree(|tree| tree.shadow_host(outer)).flatten() {
        chain.push(next);
        outer = root(next);
    }
    let order = with_tree(|tree| {
        let mut order = Vec::new();
        let mut pending = vec![outer];
        while let Some(node) = pending.pop() {
            order.push(node);
            pending.extend(tree.children(node).into_iter().rev());
            if chain.contains(&node) {
                if let Some(shadow) = tree.shadow_root(node) {
                    pending.extend(tree.children(shadow).into_iter().rev());
                }
            }
        }
        order
    })
    .unwrap_or_default();
    // Like Chrome, the root of a detached tree is never one of its own labels.
    order
        .into_iter()
        .skip(1)
        .filter(|label| association(scope, *label).map(|(_, actual)| actual) == Some(host))
        .collect()
}

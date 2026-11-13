//! Canonical hyperlink default activation delegated to an embedder's navigation policy.
use super::*;

/// A hyperlink default was consumed, including a policy denial or noopener's null result.
/// This callback runs after listeners with a connected HTML a/area that still has href.
/// It must apply the host's navigation, download, CSP and sandbox policy itself.
pub type HyperlinkActivationHandler =
    for<'s, 'i> fn(&mut v8::PinScope<'s, 'i>, v8::Local<'s, v8::Object>) -> bool;

struct Handler(HyperlinkActivationHandler);

/// Register the realm's navigation authority before guest code; None removes it.
/// The native function pointer does not retain a guest context or V8 object.
pub fn set_hyperlink_activation_handler(
    context: v8::Local<v8::Context>,
    handler: Option<HyperlinkActivationHandler>,
) {
    if let Some(handler) = handler {
        context.set_slot(std::rc::Rc::new(Handler(handler)));
    } else {
        context.remove_slot::<Handler>();
    }
}

fn receipt_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Private> {
    let name = v8::String::new(scope, "turbo-dom.hyperlink-activation-handled").unwrap();
    v8::Private::for_api(scope, Some(name))
}

/// Whether the host consumed this event's most recent dispatch. Guest properties cannot
/// forge the receipt; a true result also covers policy denial and noopener-null.
pub fn hyperlink_activation_handled(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
) -> bool {
    let key = receipt_key(scope);
    event
        .get_private(scope, key)
        .is_some_and(|value| value.is_true())
}

pub(super) fn reset(scope: &mut v8::PinScope, event: v8::Local<v8::Object>) {
    let key = receipt_key(scope);
    let no = v8::Boolean::new(scope, false);
    event.set_private(scope, key, no.into());
}

pub(super) fn target(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    activation: Option<Handle>,
) -> Option<Handle> {
    let anchor = activation.filter(|&node| eligible(node))?;
    (html_invokers::primary_activation_target(scope, event, anchor) == Some(anchor))
        .then_some(anchor)
}

fn eligible(node: Handle) -> bool {
    with_tree(|tree| {
        tree.namespace_id(node) == 0
            && matches!(tree.local_name(node), Some("a" | "area"))
            && is_connected(tree, node)
            && tree.has_attribute(node, "href")
    })
    .unwrap_or(false)
}

pub(super) fn dispatch(
    scope: &mut v8::PinScope,
    event: v8::Local<v8::Object>,
    anchor: Option<Handle>,
    not_prevented: bool,
) {
    let Some(anchor) = anchor.filter(|&node| not_prevented && eligible(node)) else {
        return;
    };
    let node = wrap(scope, anchor);
    let Some(context) = node.get_creation_context(scope) else {
        return;
    };
    let Some(handler) = context.get_slot::<Handler>() else {
        return;
    };
    let handler = handler.0;
    let scope = &mut v8::ContextScope::new(scope, context);
    if handler(scope, node) {
        let key = receipt_key(scope);
        let yes = v8::Boolean::new(scope, true);
        event.set_private(scope, key, yes.into());
    }
}

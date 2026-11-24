//! Weak ownership evidence for realm-local canonical effect registries.
use super::{dom_id_of, DomScope};
use std::cell::RefCell;

struct TargetOwner {
    animation: v8::Weak<v8::Object>,
    target: v8::Weak<v8::Object>,
    document: v8::Weak<v8::Object>,
}

thread_local! {
    static TARGETS: RefCell<Vec<TargetOwner>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn update(
    scope: &mut v8::PinScope,
    animation: v8::Local<v8::Object>,
    target: Option<v8::Local<v8::Object>>,
    document: v8::Local<v8::Object>,
) {
    TARGETS.with(|owners| {
        let mut owners = owners.borrow_mut();
        owners.retain(|owner| live_owner(scope, owner));
        owners.retain(|owner| owner.animation.to_local(scope) != Some(animation));
        if let Some(target) = target {
            owners.push(TargetOwner {
                animation: v8::Weak::new(scope, animation),
                target: v8::Weak::new(scope, target),
                document: v8::Weak::new(scope, document),
            });
        }
    });
}

fn live_owner(scope: &mut v8::PinScope, owner: &TargetOwner) -> bool {
    owner.animation.to_local(scope).is_some() && owner.target.to_local(scope).is_some()
}

fn owns_target(
    scope: &mut v8::PinScope,
    owner: &TargetOwner,
    document: v8::Local<v8::Object>,
    document_id: u64,
) -> bool {
    let Some(target) = owner.target.to_local(scope) else {
        return true;
    };
    if dom_id_of(scope, target) != document_id {
        return true;
    }
    owner.document.to_local(scope) == Some(document)
}

/// A foreign engine's effects cannot be certified by this document's private stack.
/// Target ownership is read afresh, so adoption cannot conceal a foreign source.
pub(crate) fn owns_all_targets(
    scope: &mut v8::PinScope,
    document: v8::Local<v8::Object>,
    document_id: u64,
) -> bool {
    let _owner = DomScope::for_object(scope, document);
    TARGETS.with(|owners| {
        let mut owners = owners.borrow_mut();
        owners.retain(|owner| live_owner(scope, owner));
        owners
            .iter()
            .all(|owner| owns_target(scope, owner, document, document_id))
    })
}

pub(crate) fn reset() {
    let _ = TARGETS.try_with(|owners| owners.borrow_mut().clear());
}

//! Weak ownership evidence for realm-local canonical effect registries.
use super::{dom_id_of, DomScope};
use std::{cell::RefCell, rc::Rc};

struct TargetOwner {
    animation: v8::Weak<v8::Object>,
    target: v8::Weak<v8::Object>,
    document: v8::Weak<v8::Object>,
}

#[derive(Default)]
struct TargetOwners(RefCell<Vec<TargetOwner>>);

fn owners(scope: &mut v8::PinScope) -> Rc<TargetOwners> {
    if let Some(owners) = scope.get_slot::<Rc<TargetOwners>>() {
        return owners.clone();
    }
    // Weak handles may only be localized in their creating isolate. The isolate
    // owns this registry, including its teardown, rather than the current thread.
    let owners = Rc::new(TargetOwners::default());
    scope.set_slot(owners.clone());
    owners
}

pub(crate) fn update(
    scope: &mut v8::PinScope,
    animation: v8::Local<v8::Object>,
    target: Option<v8::Local<v8::Object>>,
    document: v8::Local<v8::Object>,
) {
    {
        let registry = owners(scope);
        let mut owners = registry.0.borrow_mut();
        owners.retain(|owner| live_owner(scope, owner));
        owners.retain(|owner| owner.animation.to_local(scope) != Some(animation));
        if let Some(target) = target {
            owners.push(TargetOwner {
                animation: v8::Weak::new(scope, animation),
                target: v8::Weak::new(scope, target),
                document: v8::Weak::new(scope, document),
            });
        }
    }
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
    {
        let registry = owners(scope);
        let mut owners = registry.0.borrow_mut();
        owners.retain(|owner| live_owner(scope, owner));
        owners
            .iter()
            .all(|owner| owns_target(scope, owner, document, document_id))
    }
}

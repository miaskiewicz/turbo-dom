//! Rehydrating the function-holding context slots of a context deserialized from a startup
//! snapshot (turbo-dom#249).
//!
//! A context slot lives in the context's slot annex, a Rust heap pointer that cannot be
//! serialized (the snapshot build clears it). Every capture site therefore also stores the
//! captured function, by its slot type's name, in a JS object held by the context's global
//! object under a V8 private (page script cannot reach it): a V8 value, which a snapshot
//! carries. (Not embedder data: rusty_v8's `get_embedder_data` reads unchecked past the
//! array, so a context that never set the index would read garbage.) `rehydrate_context` walks the registry of slot
//! types and rebuilds each slot from that object. The non-function slots (DOM id, font
//! defaults, records, keys, registries) are rebuilt by their own init code.
use std::rc::Rc;

/// The private holding the captured-function store on the context's global object.
fn store_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    let name = v8::String::new(scope, "turbo-dom.context-rehydrate.store")?;
    Some(v8::Private::for_api(scope, Some(name)))
}

/// One function-holding context slot type: its unique name, a constructor that rebuilds the
/// slot from the stored value, and a reader of the functions it holds (for the tests).
pub(super) struct Entry {
    pub(super) name: &'static str,
    pub(super) set: fn(&mut v8::PinScope, v8::Local<v8::Context>, v8::Local<v8::Value>) -> bool,
    pub(super) get: for<'s, 'i> fn(
        &mut v8::PinScope<'s, 'i>,
        v8::Local<'_, v8::Context>,
    ) -> Vec<v8::Local<'s, v8::Function>>,
}

/// A context slot holding one engine function.
pub(super) trait FnSlot: 'static {
    const NAME: &'static str;
    fn wrap(function: v8::Weak<v8::Function>) -> Self;
    fn weak(&self) -> &v8::Weak<v8::Function>;
}

fn set_one<T: FnSlot>(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
    value: v8::Local<v8::Value>,
) -> bool {
    let Ok(function) = v8::Local::<v8::Function>::try_from(value) else {
        return false;
    };
    context.set_slot(Rc::new(T::wrap(v8::Weak::new(scope, function))));
    true
}

fn get_one<'s, T: FnSlot>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
) -> Vec<v8::Local<'s, v8::Function>> {
    context
        .get_slot::<T>()
        .and_then(|slot| slot.weak().to_local(scope))
        .into_iter()
        .collect()
}

impl Entry {
    pub(super) const fn of<T: FnSlot>() -> Self {
        Entry {
            name: T::NAME,
            set: set_one::<T>,
            get: get_one::<T>,
        }
    }
}

/// Weak functions read from a stored array (multi-function slots).
pub(super) fn functions_of<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'_, v8::Value>,
) -> Option<Vec<v8::Weak<v8::Function>>> {
    let value = v8::Local::new(scope, value);
    let array = v8::Local::<v8::Array>::try_from(value).ok()?;
    (0..array.length())
        .map(|index| {
            let item = array.get_index(scope, index)?;
            let function = v8::Local::<v8::Function>::try_from(item).ok()?;
            Some(v8::Weak::new(scope, function))
        })
        .collect()
}

/// Locals of weak functions (multi-function slots' readers).
pub(super) fn locals<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    weaks: &[&v8::Weak<v8::Function>],
) -> Vec<v8::Local<'s, v8::Function>> {
    weaks
        .iter()
        .filter_map(|weak| weak.to_local(scope))
        .collect()
}

/// Implement [`FnSlot`] for single-function tuple slots, named by module path, and define
/// the module's `SLOTS` registry part (plus any custom entries).
macro_rules! fn_slots {
    ([$($slot:ident),* $(,)?] $(, $extra:expr)* $(,)?) => {
        $(
            impl $crate::browser_env::context_rehydrate::FnSlot for $slot {
                const NAME: &'static str = concat!(module_path!(), "::", stringify!($slot));
                fn wrap(function: v8::Weak<v8::Function>) -> Self {
                    Self(function)
                }
                fn weak(&self) -> &v8::Weak<v8::Function> {
                    &self.0
                }
            }
        )*
        pub(in crate::browser_env) const SLOTS: &[$crate::browser_env::context_rehydrate::Entry] = &[
            $($crate::browser_env::context_rehydrate::Entry::of::<$slot>(),)*
            $($extra,)*
        ];
    };
}

/// Every function-holding context slot type.
const REGISTRY: &[&[Entry]] = &[
    super::SLOTS,
    super::custom_elements::SLOTS,
    super::document_legacy::SLOTS,
    super::editing::SLOTS,
    super::element_internals::SLOTS,
    super::event_listeners::SLOTS,
    super::file_inputs::SLOTS,
    super::focus_group::SLOTS,
    super::form_pseudo::SLOTS,
    super::html_invokers::SLOTS,
    super::html_media::SLOTS,
    super::interest_invokers::SLOTS,
    super::pointer_events::SLOTS,
    super::range_contents::SLOTS,
    super::select_element::SLOTS,
    super::selection::SLOTS,
    super::user_activation_gates::SLOTS,
];

pub(super) fn entries() -> impl Iterator<Item = &'static Entry> {
    REGISTRY.iter().flat_map(|part| part.iter())
}

/// The context's captured-function store, if any capture site ran in it.
pub(super) fn store_object<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
) -> Option<v8::Local<'s, v8::Object>> {
    let context = v8::Local::new(scope, context);
    let key = store_key(scope)?;
    let value = context.global(scope).get_private(scope, key)?;
    v8::Local::<v8::Object>::try_from(value).ok()
}

/// Store a captured slot value (a function, or an array of them) under its slot name.
pub(super) fn store(scope: &mut v8::PinScope, name: &str, value: v8::Local<v8::Value>) {
    let context = scope.get_current_context();
    let object = match store_object(scope, context) {
        Some(object) => object,
        None => {
            let null = v8::null(scope).into();
            let object = v8::Object::with_prototype_and_properties(scope, null, &[], &[]);
            let Some(key) = store_key(scope) else {
                return;
            };
            context.global(scope).set_private(scope, key, object.into());
            object
        }
    };
    let Some(key) = v8::String::new(scope, name) else {
        return;
    };
    object.create_data_property(scope, key.into(), value);
}

/// Retain `function` as an engine helper and set it as the current context's `T` slot,
/// storing it for [`rehydrate_context`]. `None` when the helper could not be retained.
pub(super) fn capture<T: FnSlot>(
    scope: &mut v8::PinScope,
    function: v8::Local<v8::Function>,
) -> Option<()> {
    let weak = super::dom_ownership::helper_function(scope, function)?;
    scope.get_current_context().set_slot(Rc::new(T::wrap(weak)));
    store(scope, T::NAME, function.into());
    Some(())
}

/// Rebuild every function-holding slot of `context` (deserialized from a startup snapshot)
/// from its captured-function store.
pub(crate) fn rehydrate_context(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) {
    let Some(object) = store_object(scope, context) else {
        return;
    };
    for entry in entries() {
        let Some(key) = v8::String::new(scope, entry.name) else {
            continue;
        };
        if let Some(value) = object.get(scope, key.into()) {
            (entry.set)(scope, context, value);
        }
    }
}

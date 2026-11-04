//! Context-independent callback data for templates (turbo-dom#249).
//!
//! V8's snapshot serializer puts every template (and so its callback data) in the
//! isolate-wide startup part of a snapshot. Callback data that is a JS object of a context
//! (a function, an array of them) therefore drags a second copy of that whole context into
//! the startup part, and a context deserialized from the snapshot calls back into the copy.
//! A template the bootstrap builds with object data instead stores the object in its
//! context's table and gets a Smi key as its data, which the callback resolves in its own
//! realm. Only while a snapshot is built: otherwise the data stays the object itself, so a
//! template made at run time never pins its data in the table.

struct Table(v8::Weak<v8::Array>);

/// The private holding the table on the context's global object (it keeps the table, and
/// so every entry, alive; the context slot is a fast weak path to it).
fn table_key<'s>(scope: &mut v8::PinScope<'s, '_>) -> Option<v8::Local<'s, v8::Private>> {
    let name = v8::String::new(scope, "turbo-dom.template-data")?;
    Some(v8::Private::for_api(scope, Some(name)))
}

fn table<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
    create: bool,
) -> Option<v8::Local<'s, v8::Array>> {
    if let Some(table) = context
        .get_slot::<Table>()
        .and_then(|table| table.0.to_local(scope))
    {
        return Some(table);
    }
    let context = v8::Local::new(scope, context);
    let key = table_key(scope)?;
    let global = context.global(scope);
    // A context deserialized from a snapshot has the table but not yet the slot.
    let table = match global
        .get_private(scope, key)
        .and_then(|value| v8::Local::<v8::Array>::try_from(value).ok())
    {
        Some(table) => table,
        None if create => {
            let table = v8::Array::new(scope, 0);
            global.set_private(scope, key, table.into())?;
            table
        }
        None => return None,
    };
    context.set_slot(std::rc::Rc::new(Table(v8::Weak::new(scope, table))));
    Some(table)
}

/// The template data for `value`: while a snapshot is built, its key in the current
/// context's table; otherwise `value` itself.
pub(super) fn key<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'_, v8::Value>,
) -> Option<v8::Local<'s, v8::Value>> {
    let value = v8::Local::new(scope, value);
    if !super::crate_surface::startup_snapshot::building() {
        return Some(value);
    }
    let context = scope.get_current_context();
    let table = table(scope, context, true)?;
    let index = table.length();
    table.set_index(scope, index, value)?;
    Some(v8::Integer::new_from_unsigned(scope, index).into())
}

/// The value template data `data` (a [`key`]) stands for in `context`.
pub(super) fn value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<'_, v8::Context>,
    data: v8::Local<'_, v8::Value>,
) -> Option<v8::Local<'s, v8::Value>> {
    let data = v8::Local::new(scope, data);
    if !data.is_uint32() {
        return Some(data);
    }
    let index = data.uint32_value(scope)?;
    let table = table(scope, context, false)?;
    table.get_index(scope, index)
}

/// [`value`] for a function callback, which runs in its function's realm.
pub(super) fn of_call<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::FunctionCallbackArguments<'_>,
) -> Option<v8::Local<'s, v8::Value>> {
    let context = scope.get_current_context();
    value(scope, context, args.data())
}

/// [`value`] for an interceptor, resolved in its holder's realm (the caller's may differ).
pub(super) fn of_property<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: &v8::PropertyCallbackArguments<'_>,
) -> Option<v8::Local<'s, v8::Value>> {
    let context = args.holder().get_creation_context(scope)?;
    value(scope, context, args.data())
}

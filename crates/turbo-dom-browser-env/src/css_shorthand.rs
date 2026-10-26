//! `__cssShorthandNative`: turbo-dom's CSSOM shorthand engine (`rtdom::css_shorthand`, the
//! Chrome 156 shorthand table, grammars and serializers) for the binding's
//! `CSSStyleDeclaration` (turbo-dom#90). A declaration block crosses as a flat array of
//! `name, value, important, pendingShorthand | null, pendingText | null` quintuples.
use super::*;
use turbo_dom_parser::rtdom::css_shorthand::{
    block_text, canonical, declarations, expand, longhand_alias, longhands, shorthand_value,
    shorthands_of, Declaration,
};

fn string<'s>(scope: &mut v8::PinScope<'s, '_>, text: &str) -> v8::Local<'s, v8::Value> {
    v8::String::new(scope, text).map_or_else(|| v8::null(scope).into(), Into::into)
}
fn optional<'s>(scope: &mut v8::PinScope<'s, '_>, text: Option<&str>) -> v8::Local<'s, v8::Value> {
    match text {
        Some(text) => string(scope, text),
        None => v8::null(scope).into(),
    }
}

/// The block's declarations, read from the flat array at `args[index]`.
fn block(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Vec<(String, String, bool, Option<(String, String)>)> {
    let Ok(array) = v8::Local::<v8::Array>::try_from(value) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let length = array.length();
    let mut index = 0;
    while index + 4 < length {
        let mut field = |offset: u32| {
            array
                .get_index(scope, index + offset)
                .unwrap_or_else(|| v8::undefined(scope).into())
        };
        let name = field(0).to_rust_string_lossy(scope);
        let value = field(1).to_rust_string_lossy(scope);
        let important = field(2).is_true();
        let shorthand = field(3);
        let pending = if shorthand.is_string() {
            let text = field(4).to_rust_string_lossy(scope);
            Some((shorthand.to_rust_string_lossy(scope), text))
        } else {
            None
        };
        out.push((name, value, important, pending));
        index += 5;
    }
    out
}

fn borrowed(entries: &[(String, String, bool, Option<(String, String)>)]) -> Vec<Declaration<'_>> {
    entries
        .iter()
        .map(|(name, value, important, pending)| Declaration {
            name,
            value,
            important: *important,
            pending: pending
                .as_ref()
                .map(|(shorthand, text)| (shorthand.as_str(), text.as_str())),
        })
        .collect()
}

fn native(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let operation = args.get(0).to_rust_string_lossy(scope);
    match operation.as_str() {
        // The shorthand's canonical name, or null for a longhand.
        "shorthand" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let value = optional(scope, canonical(&name));
            rv.set(value);
        }
        // The longhand a longhand alias names (`word-wrap` -> `overflow-wrap`), or null.
        "alias" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let value = optional(scope, longhand_alias(&name));
            rv.set(value);
        }
        "longhands" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let Some(list) = longhands(&name) else {
                rv.set_null();
                return;
            };
            let items: Vec<v8::Local<v8::Value>> =
                list.iter().map(|name| string(scope, name)).collect();
            rv.set(v8::Array::new_with_elements(scope, &items).into());
        }
        // The shorthands a longhand belongs to.
        "shorthands" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let names: Vec<&str> = shorthands_of(&name).collect();
            let items: Vec<v8::Local<v8::Value>> =
                names.iter().map(|name| string(scope, name)).collect();
            rv.set(v8::Array::new_with_elements(scope, &items).into());
        }
        // Flat `[longhand, value, pendingShorthand, pendingText, ...]`, or null if invalid.
        "expand" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let value = args.get(2).to_rust_string_lossy(scope);
            let Some(list) = expand(&name, &value) else {
                rv.set_null();
                return;
            };
            let mut items = Vec::with_capacity(list.len() * 4);
            for longhand in &list {
                items.push(string(scope, longhand.name));
                items.push(string(scope, &longhand.value));
                items.push(optional(
                    scope,
                    longhand.pending.as_ref().map(|pending| pending.shorthand),
                ));
                items.push(optional(
                    scope,
                    longhand
                        .pending
                        .as_ref()
                        .map(|pending| pending.text.as_str()),
                ));
            }
            rv.set(v8::Array::new_with_elements(scope, &items).into());
        }
        "value" => {
            let name = args.get(1).to_rust_string_lossy(scope);
            let entries = block(scope, args.get(2));
            let text = shorthand_value(&name, &borrowed(&entries));
            let value = string(scope, &text);
            rv.set(value);
        }
        "text" => {
            let entries = block(scope, args.get(1));
            let text = block_text(&borrowed(&entries));
            let value = string(scope, &text);
            rv.set(value);
        }
        // A declaration list's declarations as flat `[name, value, important, ...]`.
        "declarations" => {
            let text = args.get(1).to_rust_string_lossy(scope);
            let mut items = Vec::new();
            for (name, value, important) in declarations(&text) {
                items.push(string(scope, &name));
                items.push(string(scope, &value));
                items.push(v8::Boolean::new(scope, important).into());
            }
            rv.set(v8::Array::new_with_elements(scope, &items).into());
        }
        _ => rv.set_undefined(),
    }
}

pub(super) fn register(scope: &mut v8::PinScope, global: v8::Local<v8::Object>) {
    bind_method(scope, global, "__cssShorthandNative", native);
}

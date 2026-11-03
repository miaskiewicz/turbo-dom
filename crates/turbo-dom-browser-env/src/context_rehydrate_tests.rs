//! turbo-dom#249 Phase C step 2: a context deserialized from the startup snapshot gets back
//! every function-holding context slot of a freshly bootstrapped one, from the registry.
use super::*;
use crate::browser_env::context_rehydrate::{entries, store_object};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

/// Each present slot type's functions, as (source, arity) per function.
fn slot_functions(
    scope: &mut v8::PinScope,
    context: v8::Local<v8::Context>,
) -> BTreeMap<&'static str, Vec<(String, i64)>> {
    let mut slots = BTreeMap::new();
    for entry in entries() {
        let functions = (entry.get)(scope, context);
        if functions.is_empty() {
            continue;
        }
        let described = functions
            .iter()
            .map(|function| {
                let source = function.to_string(scope).unwrap().to_rust_string_lossy(scope);
                let key = v8::String::new(scope, "length").unwrap();
                let arity = function
                    .get(scope, key.into())
                    .and_then(|value| value.integer_value(scope))
                    .unwrap();
                (source, arity)
            })
            .collect();
        slots.insert(entry.name, described);
    }
    slots
}

/// The slot names the capture sites stored in the context.
fn stored_names(scope: &mut v8::PinScope, context: v8::Local<v8::Context>) -> BTreeSet<String> {
    let object = store_object(scope, context).expect("the bootstrap captured functions");
    let names = object
        .get_own_property_names(scope, Default::default())
        .unwrap();
    (0..names.length())
        .map(|index| {
            names
                .get_index(scope, index)
                .unwrap()
                .to_rust_string_lossy(scope)
        })
        .collect()
}

#[test]
fn rehydrate_rebuilds_every_function_slot_of_a_bootstrapped_context() {
    crate::browser_env::tests::init_v8();
    let names: BTreeSet<&str> = entries().map(|entry| entry.name).collect();
    assert_eq!(names.len(), entries().count(), "registry names are unique");
    let fresh = {
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        bootstrap_engine(scope);
        let stored = stored_names(scope, context);
        let fresh = slot_functions(scope, context);
        reset();
        // A capture site storing a slot type the registry lacks fails here; a slot type
        // the registry knows but whose capture site does not store, below.
        for name in &stored {
            assert!(names.contains(name.as_str()), "no registry entry for {name}");
        }
        let present: BTreeSet<String> = fresh.keys().map(|name| (*name).to_owned()).collect();
        assert_eq!(present, stored, "slots set without storing their function");
        fresh
    };
    assert!(fresh.len() > 25, "{}", fresh.len());

    let blob = startup_snapshot::build().expect("every callback is registered");
    let params = v8::CreateParams::default()
        .snapshot_blob(blob.data)
        .external_references(Cow::Borrowed(blob.references));
    let isolate = &mut v8::Isolate::new(params);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    assert!(slot_functions(scope, context).is_empty());
    rehydrate_context(scope, context);
    assert_eq!(slot_functions(scope, context), fresh);
    // Each rehydrated slot holds the very function the context stored for it.
    let object = store_object(scope, context).unwrap();
    for entry in entries() {
        let functions = (entry.get)(scope, context);
        if functions.is_empty() {
            continue;
        }
        let key = v8::String::new(scope, entry.name).unwrap();
        let stored = object.get(scope, key.into()).unwrap();
        let stored: Vec<v8::Local<v8::Value>> = match v8::Local::<v8::Array>::try_from(stored) {
            Ok(array) => (0..array.length())
                .map(|index| array.get_index(scope, index).unwrap())
                .collect(),
            Err(_) => vec![stored],
        };
        assert_eq!(stored.len(), functions.len(), "{}", entry.name);
        for (stored, function) in stored.iter().zip(&functions) {
            assert!(stored.strict_equals((*function).into()), "{}", entry.name);
        }
    }
}

/// A context with no captured functions (never bootstrapped) rehydrates to no slots.
#[test]
fn rehydrating_a_bare_context_sets_no_slot() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    rehydrate_context(scope, context);
    assert!(slot_functions(scope, context).is_empty());
}

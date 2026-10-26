//! Chrome's own-key order for every Web IDL interface object and prototype, applied once the
//! realm's interfaces are complete. The order is data, captured from Chrome 156 by the
//! key_order_probe.js collector run (key_order_chrome156.json); key_order.js moves keys
//! without changing their descriptors. An embedder that defines further interfaces after the
//! binding (turbo-surf) defers the step and finishes it after its own installs.
use std::collections::HashMap;
use std::sync::OnceLock;

struct Interface {
    name: String,
    interface: (Vec<String>, Vec<String>),
    prototype: (Vec<String>, Vec<String>),
}

/// The embedder finishes the key order itself after its own interfaces exist.
struct Deferred;

fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn table() -> &'static [Interface] {
    static TABLE: OnceLock<Vec<Interface>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("key_order_chrome156.json"))
                .expect("key order fixture is JSON");
        let shape = |entry: &serde_json::Value| (strings(&entry["keys"]), strings(&entry["fixed"]));
        fixture["result"]
            .as_object()
            .map(|interfaces| {
                interfaces
                    .iter()
                    .map(|(name, entry)| Interface {
                        name: name.clone(),
                        interface: shape(&entry["interface"]),
                        prototype: shape(&entry["prototype"]),
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// The positions (into `labels`) to move, in their final order: Chrome's order for the keys
/// present, each followed by the keys Chrome lacks that follow it here. Strings and symbols
/// are ordered separately, as V8 lists them. The longest prefix of the result that already
/// appears in that order stays; a constant Chrome freezes always moves, to be re-added frozen.
pub(super) fn plan(labels: &[(String, bool)], order: &[String], fixed: &[String]) -> Vec<usize> {
    let known: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut moved = Vec::new();
    for symbol in [false, true] {
        let current: Vec<usize> = (0..labels.len())
            .filter(|&index| labels[index].1 == symbol)
            .collect();
        let mut leading = Vec::new();
        let mut attached: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut anchor = None;
        let mut present: Vec<(usize, usize)> = Vec::new();
        for &index in &current {
            match known.get(labels[index].0.as_str()) {
                Some(&rank) => {
                    anchor = Some(rank);
                    present.push((rank, index));
                }
                None => match anchor {
                    Some(rank) => attached.entry(rank).or_default().push(index),
                    None => leading.push(index),
                },
            }
        }
        present.sort_unstable();
        let mut desired = leading;
        for (rank, index) in present {
            desired.push(index);
            desired.extend(attached.remove(&rank).unwrap_or_default());
        }
        let mut kept = 0;
        let mut last = None;
        while kept < desired.len()
            && last.is_none_or(|last| desired[kept] > last)
            && !fixed.contains(&labels[desired[kept]].0)
        {
            last = Some(desired[kept]);
            kept += 1;
        }
        moved.extend_from_slice(&desired[kept..]);
    }
    moved
}

fn label(scope: &mut v8::PinScope, key: v8::Local<v8::Value>) -> (String, bool) {
    match v8::Local::<v8::Symbol>::try_from(key) {
        Ok(symbol) => {
            let description = symbol.description(scope);
            let text = if description.is_undefined() {
                String::new()
            } else {
                description.to_rust_string_lossy(scope)
            };
            (format!("Symbol({text})"), true)
        }
        Err(_) => (key.to_rust_string_lossy(scope), false),
    }
}

/// One mover job for `object`, or `None` when it already is in Chrome's order.
fn job<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    object: v8::Local<'s, v8::Object>,
    (order, fixed): &(Vec<String>, Vec<String>),
) -> Option<v8::Local<'s, v8::Value>> {
    let args = v8::GetPropertyNamesArgs {
        mode: v8::KeyCollectionMode::OwnOnly,
        property_filter: v8::PropertyFilter::ALL_PROPERTIES,
        index_filter: v8::IndexFilter::IncludeIndices,
        key_conversion: v8::KeyConversionMode::KeepNumbers,
    };
    let names = object.get_own_property_names(scope, args)?;
    let keys: Vec<v8::Local<v8::Value>> = (0..names.length())
        .filter_map(|index| names.get_index(scope, index))
        .collect();
    let labels: Vec<(String, bool)> = keys.iter().map(|&key| label(scope, key)).collect();
    let moved = plan(&labels, order, fixed);
    if moved.is_empty() {
        return None;
    }
    let moved: Vec<v8::Local<v8::Value>> = moved.into_iter().map(|index| keys[index]).collect();
    let moved = v8::Array::new_with_elements(scope, &moved);
    let fixed: Vec<v8::Local<v8::Value>> = fixed
        .iter()
        .map(|name| v8::String::new(scope, name).unwrap().into())
        .collect();
    let fixed = v8::Array::new_with_elements(scope, &fixed);
    Some(v8::Array::new_with_elements(scope, &[object.into(), moved.into(), fixed.into()]).into())
}

/// A budget watchdog may terminate execution while the step runs. The termination ends here,
/// where V8 would discard it with the last JS frame, so it is raised again for the guest code
/// that follows instead of being lost.
fn arrange(scope: &mut v8::PinScope) {
    let terminated = {
        v8::tc_scope!(let scope, scope);
        arrange_in(scope);
        scope.has_terminated()
    };
    if terminated {
        scope.terminate_execution();
    }
}

fn arrange_in(scope: &mut v8::PinScope) {
    let source = v8::String::new(scope, include_str!("key_order.js")).unwrap();
    if v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .is_none()
    {
        return;
    }
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__captureKeyOrder").unwrap();
    let mover = global
        .get(scope, key.into())
        .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok());
    global.delete(scope, key.into());
    let Some(mover) = mover else {
        return;
    };
    let prototype_key = v8::String::new(scope, "prototype").unwrap();
    let constructor_key = v8::String::new(scope, "constructor").unwrap();
    let mut jobs = Vec::new();
    for interface in table() {
        let name = v8::String::new(scope, &interface.name).unwrap();
        if !global.has_own_property(scope, name.into()).unwrap_or(false) {
            continue;
        }
        let Some(constructor) = global
            .get(scope, name.into())
            .and_then(|value| v8::Local::<v8::Function>::try_from(value).ok())
        else {
            continue;
        };
        jobs.extend(job(scope, constructor.into(), &interface.interface));
        let Some(prototype) = constructor
            .get(scope, prototype_key.into())
            .and_then(|value| v8::Local::<v8::Object>::try_from(value).ok())
        else {
            continue;
        };
        let own = prototype
            .has_own_property(scope, constructor_key.into())
            .unwrap_or(false);
        let back = prototype.get(scope, constructor_key.into());
        if own && back.is_some_and(|back| back.strict_equals(constructor.into())) {
            jobs.extend(job(scope, prototype, &interface.prototype));
        }
    }
    let jobs = v8::Array::new_with_elements(scope, &jobs);
    let receiver = v8::undefined(scope).into();
    mover.call(scope, receiver, &[jobs.into()]);
}

/// Called at the end of every binding install.
pub(super) fn installed(scope: &mut v8::PinScope) {
    if scope.get_current_context().get_slot::<Deferred>().is_none() {
        arrange(scope);
    }
}

/// Keep the current context's interfaces in definition order until [`finish`].
pub fn defer(scope: &mut v8::PinScope) {
    scope
        .get_current_context()
        .set_slot(std::rc::Rc::new(Deferred));
}

/// Arrange the current context's interfaces in Chrome's order and freeze its constants.
pub fn finish(scope: &mut v8::PinScope) {
    scope.get_current_context().remove_slot::<Deferred>();
    arrange(scope);
}

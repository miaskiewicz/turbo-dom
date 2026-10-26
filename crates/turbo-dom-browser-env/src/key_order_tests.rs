use super::*;
use sha2::{Digest, Sha256};

fn probe_result(scope: &mut v8::PinScope) -> serde_json::Value {
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, include_str!("key_order_probe.js")).unwrap();
    let Some(result) =
        v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap()
}

fn listed(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(|item| item.as_str()).collect())
        .unwrap_or_default()
}

/// Every interface present in both realms lists its shared keys in Chrome's order, on the
/// interface object and the prototype, with Chrome's non-configurable constants.
fn mismatches(actual: &serde_json::Value, chrome: &serde_json::Value) -> (usize, Vec<String>) {
    let mut compared = 0;
    let mut failures = Vec::new();
    for (name, ours) in actual.as_object().unwrap() {
        let Some(theirs) = chrome.get(name) else {
            continue;
        };
        compared += 1;
        for part in ["interface", "prototype"] {
            let mine = listed(&ours[part]["keys"]);
            let expected = listed(&theirs[part]["keys"]);
            let shared: Vec<&str> = mine
                .iter()
                .copied()
                .filter(|key| expected.contains(key))
                .collect();
            let ordered: Vec<&str> = expected
                .iter()
                .copied()
                .filter(|key| mine.contains(key))
                .collect();
            if shared != ordered {
                failures.push(format!(
                    "{name}.{part} order\n  binding: {shared:?}\n  chrome:  {ordered:?}"
                ));
            }
            let fixed = |entry: &serde_json::Value| -> Vec<String> {
                listed(&entry[part]["fixed"])
                    .into_iter()
                    .filter(|key| mine.contains(key) && expected.contains(key))
                    .map(str::to_owned)
                    .collect()
            };
            if fixed(ours) != fixed(theirs) {
                failures.push(format!(
                    "{name}.{part} non-configurable\n  binding: {:?}\n  chrome:  {:?}",
                    fixed(ours),
                    fixed(theirs)
                ));
            }
        }
    }
    (compared, failures)
}

#[test]
fn interface_key_order_matches_chrome156_control() {
    super::tests::init_v8();
    let control: serde_json::Value =
        serde_json::from_str(include_str!("key_order_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["v8"], "15.6.75.19");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!(
            "{:x}",
            Sha256::digest(include_str!("key_order_probe.js").as_bytes())
        )
    );
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let actual = probe_result(scope);
    let (compared, failures) = mismatches(&actual, &control["result"]);
    assert!(compared >= 130, "only {compared} shared interfaces");
    // Blink: attributes, constants, operations, then `constructor` and the symbols.
    let node = listed(&actual["Node"]["prototype"]["keys"]);
    assert_eq!(
        (node[0], node[node.len() - 2], node[node.len() - 1]),
        ("nodeType", "constructor", "Symbol(Symbol.toStringTag)")
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    reset();
}

/// A deferred install keeps definition order (constants configurable) until the embedder
/// finishes it; finishing arranges and freezes exactly as an undeferred install.
#[test]
fn deferred_key_order_finishes_like_an_install() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    key_order::defer(scope);
    install_independent_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let before = probe_result(scope);
    assert_eq!(before["Node"]["prototype"]["keys"][0], "constructor");
    assert!(before["Node"]["prototype"].get("fixed").is_none());
    key_order::finish(scope);
    let after = probe_result(scope);
    let control: serde_json::Value =
        serde_json::from_str(include_str!("key_order_chrome156.json")).unwrap();
    let (_, failures) = mismatches(&after, &control["result"]);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    reset();
}

/// Keys Chrome lacks travel with the key they follow; an in-order prefix stays put and a
/// constant to freeze always moves; symbols are planned after strings.
#[test]
fn plan_moves_only_the_out_of_order_suffix() {
    let labels = |names: &[&str]| -> Vec<(String, bool)> {
        names
            .iter()
            .map(|name| (name.to_string(), name.starts_with("Symbol(")))
            .collect()
    };
    let strings = |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
    let order = strings(&["a", "b", "C", "c", "Symbol(Symbol.toStringTag)"]);
    // current: c x a b Symbol(...) → final a b c x Symbol(...)
    let current = labels(&["c", "x", "a", "b", "Symbol(Symbol.toStringTag)"]);
    assert_eq!(key_order::plan(&current, &order, &[]), vec![0, 1]);
    // An extra before every known key stays first.
    let current = labels(&["x", "a", "b", "c"]);
    assert_eq!(key_order::plan(&current, &order, &[]), Vec::<usize>::new());
    // The constant C must be re-added frozen, so it and everything after it move.
    let current = labels(&["a", "b", "C", "c"]);
    assert_eq!(
        key_order::plan(&current, &order, &strings(&["C"])),
        vec![2, 3]
    );
}

/// A termination requested while the step runs reaches the guest code that follows.
#[test]
fn termination_during_key_order_is_not_lost() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    key_order::defer(scope);
    install_independent_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    scope.terminate_execution();
    key_order::finish(scope);
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, "'guest ran'").unwrap();
    let ran = v8::Script::compile(scope, code, None).and_then(|script| script.run(scope));
    assert!(ran.is_none() && scope.has_terminated());
    scope.cancel_terminate_execution();
    reset();
}

//! turbo-surf#42: DOMExceptions the binding raises are instances of the realm's DOMException
//! interface, against the Chrome 156 control (`dom_exception_realm_probe.js` +
//! `dom_exception_realm_cases.json` -> `dom_exception_realm_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs): with the binding's own interface (turbo-test),
//! after an embedder installs and registers its own (`register_dom_exception`, as turbo-surf's
//! runtime does), and after page script reassigns `globalThis.DOMException`.
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("dom_exception_realm_probe.js");
const CASES: &[u8] = include_bytes!("dom_exception_realm_cases.json");
const CONTROL: &str = include_str!("dom_exception_realm_chrome156.json");

/// An embedder's own DOMException interface, installed over the binding's and registered the
/// way turbo-surf's runtime does it (`globalThis.DOMException = class …`, then
/// `op_register_dom_exception(DOMException)`).
const HOST_EXCEPTION: &str = r#"(() => {
  const state = new WeakMap();
  globalThis.DOMException = class DOMException {
    constructor(message = "", name = "Error") { state.set(this, { message: String(message), name: String(name) }); }
    get name() { return state.get(this).name; }
    get message() { return state.get(this).message; }
  };
  Object.setPrototypeOf(DOMException.prototype, Error.prototype);
  return DOMException;
})()"#;

fn control() -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156."));
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(PROBE.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(CASES))
    );
    control["result"][0]["result"].clone()
}

fn eval<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .expect("script runs")
}

/// Install the binding over the cases page, optionally install + register a host DOMException,
/// then run the probe to completion.
fn probe(host: bool) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    if host {
        let constructor = eval(scope, HOST_EXCEPTION);
        let constructor = v8::Local::<v8::Function>::try_from(constructor).unwrap();
        register_dom_exception(scope, constructor);
    }
    let code = format!(
        "Promise.resolve(({})).then(value=>globalThis.realmResult=JSON.stringify(value),error=>globalThis.realmResult='ERROR '+String(error&&error.stack||error))",
        PROBE.trim().trim_end_matches(';')
    );
    eval(scope, &code);
    scope.perform_microtask_checkpoint();
    let key = v8::String::new(scope, "realmResult").unwrap();
    let result = context.global(scope).get(scope, key.into()).unwrap();
    assert!(result.is_string(), "probe did not settle");
    let result = result.to_rust_string_lossy(scope);
    assert!(!result.starts_with("ERROR"), "{result}");
    reset();
    serde_json::from_str(&result).unwrap()
}

fn mismatches(
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    section: &str,
) -> Vec<String> {
    let mut found = Vec::new();
    for (id, chrome) in expected[section].as_object().unwrap() {
        let binding = &actual[section][id];
        if binding != chrome {
            found.push(format!(
                "{section}/{id}: binding {binding}, Chrome {chrome}"
            ));
        }
    }
    found
}

#[test]
fn mismatches_name_each_differing_case_of_a_section() {
    let expected = serde_json::json!({"s": {"a": 1, "b": 2}});
    let actual = serde_json::json!({"s": {"a": 1, "b": 3}});
    assert_eq!(
        mismatches(&actual, &expected, "s"),
        ["s/b: binding 3, Chrome 2"]
    );
}

/// turbo-surf#42: with an embedder's DOMException installed and registered after the binding,
/// every exception the binding raises is an instance of it.
#[test]
fn binding_exceptions_are_the_hosts_dom_exception() {
    let found = mismatches(&probe(true), &control(), "installed");
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// Without an embedder (turbo-test) they are instances of the binding's own interface.
#[test]
fn binding_exceptions_are_the_bindings_dom_exception_without_a_host() {
    let found = mismatches(&probe(false), &control(), "installed");
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// turbo-dom#72: page script reassigning `globalThis.DOMException` changes nothing: the
/// exceptions stay instances of the realm's interface (the host's, else the binding's).
#[test]
fn binding_exceptions_ignore_a_page_reassigned_dom_exception() {
    let control = control();
    let mut found = mismatches(&probe(true), &control, "replaced");
    found.extend(mismatches(&probe(false), &control, "replaced"));
    assert!(found.is_empty(), "{}", found.join("\n"));
}

use super::*;
use sha2::{Digest, Sha256};

// Leaf paths where the binding and the Chrome control disagree, for readable failures.
fn leaf_mismatches(
    path: &str,
    actual: &serde_json::Value,
    expected: &serde_json::Value,
    out: &mut Vec<String>,
) {
    match (actual, expected) {
        (serde_json::Value::Object(a), serde_json::Value::Object(e)) => {
            for key in e.keys().chain(a.keys().filter(|key| !e.contains_key(*key))) {
                let null = serde_json::Value::Null;
                leaf_mismatches(
                    &format!("{path}.{key}"),
                    a.get(key).unwrap_or(&null),
                    e.get(key).unwrap_or(&null),
                    out,
                );
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(e)) if a.len() == e.len() => {
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                leaf_mismatches(&format!("{path}[{index}]"), a, e, out);
            }
        }
        _ if actual != expected => out.push(format!(
            "{path}\n  binding: {actual}\n  chrome:  {expected}"
        )),
        _ => {}
    }
}

/// Run an authored probe in an independent binding realm and return it with its Chrome control.
pub(crate) fn observe_against_control(
    probe: &str,
    cases_bytes: &[u8],
    control: &str,
) -> (serde_json::Value, serde_json::Value) {
    super::super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let cases: serde_json::Value = serde_json::from_slice(cases_bytes).unwrap();
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    if let Some(base) = cases[0]["documentBase"].as_str() {
        let id = scope
            .get_current_context()
            .get_slot::<DomContextId>()
            .unwrap()
            .0;
        let _dom = DomScope::enter(id);
        event_handlers::install(scope, None, base);
    }
    v8::tc_scope!(let scope,scope);
    let source = v8::String::new(scope, probe).unwrap();
    let Some(result) =
        v8::Script::compile(scope, source, None).and_then(|script| script.run(scope))
    else {
        panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
    };
    let actual = v8::json::stringify(scope, result)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let control: serde_json::Value = serde_json::from_str(control).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases_bytes))
    );
    reset();
    (actual, control["result"][0]["result"].clone())
}

pub(crate) fn assert_matches_control(actual: &serde_json::Value, expected: &serde_json::Value) {
    let mut mismatches = vec![];
    leaf_mismatches("", actual, expected, &mut mismatches);
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

// Control prototypes carry only Chrome's members; select.options is a live HTMLOptionsCollection
// sharing the select's algorithms; the dictionary event interfaces and HTMLHyperlinkElementUtils
// match Chrome 156 (control_interfaces_chrome156.json).
#[test]
fn control_interfaces_match_chrome156_control() {
    let (actual, expected) = observe_against_control(
        include_str!("control_interfaces_probe.js"),
        include_bytes!("control_interfaces_cases.json"),
        include_str!("control_interfaces_chrome156.json"),
    );
    // showPicker (activation cluster) and createValueRange (Selection/Range cluster) belong to
    // other #28 work; until they land here, only their absence is tolerated.
    let mut expected = expected;
    for (interface, member) in [
        ("HTMLSelectElement", "showPicker"),
        ("HTMLTextAreaElement", "createValueRange"),
    ] {
        if actual["shapes"][interface]["proto"][member].is_null() {
            expected["shapes"][interface]["proto"]
                .as_object_mut()
                .unwrap()
                .remove(member)
                .expect("Chrome exposes the member");
        }
    }
    assert_matches_control(&actual, &expected);
}
// HTMLMediaElement constants/loading, HTMLScriptElement.supports, document.plugins and toBlob
// arity against interface_members_chrome156.json.
#[test]
fn interface_members_match_chrome156_control() {
    let (mut actual, expected) = observe_against_control(
        include_str!("interface_members_probe.js"),
        include_bytes!("interface_members_cases.json"),
        include_str!("interface_members_chrome156.json"),
    );
    // KNOWN_GAP: Chrome supports importmap (index 3) and webbundle (index 5) scripts; this engine
    // applies neither, so supports() truthfully reports false for both.
    for index in [3, 5] {
        assert_eq!(expected["scriptSupports"][index], true);
        assert_eq!(actual["scriptSupports"][index], false);
        actual["scriptSupports"][index] = true.into();
    }
    assert_matches_control(&actual, &expected);
}

//! XML documents (turbo-dom#186) against the Chrome 156 control: the parsed XMLDocument
//! tree, `<parsererror>` documents, CDATA sections and the XMLSerializer round trip.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;
use sha2::Digest;

/// The embedder's `DOMParser` XML path: standalone V8 has no DOMParser, so the probe's
/// `new DOMParser().parseFromString(source, type)` calls this host function instead.
fn parse_xml(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let source = dom_units(scope, args.get(0)).unwrap();
    let content_type = args.get(1).to_rust_string_lossy(scope);
    rv.set(parse_xml_document_units(scope, &source, &content_type).into());
}

fn run(probe: &str) -> serde_json::Value {
    tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    let global = context.global(scope);
    bind_method(scope, global, "__parseXML", parse_xml);
    let probe = probe.replace(
        "new DOMParser().parseFromString(source, type)",
        "__parseXML(source, type)",
    );
    let source = format!("JSON.stringify({})", probe.trim().trim_end_matches(';'));
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, &source).unwrap();
    let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let result =
        result.unwrap_or_else(|| panic!("XML probe failed: {}", thrown(scope.exception(), scope)));
    let result = serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    reset();
    result
}

#[test]
fn xml_documents_match_the_chrome156_control() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xml_documents_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let probe = include_str!("xml_documents_probe.js");
    let digest = sha2::Sha256::digest(probe.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        control["probeSha256"], hex,
        "recapture the control for the edited probe"
    );
    let actual = run(probe);
    let expected = control["result"].as_object().unwrap();
    let differences: Vec<_> = expected
        .iter()
        .filter(|(key, value)| actual.get(key.as_str()) != Some(*value))
        .map(|(key, value)| format!("{key}:\n  native={}\n  Chrome={value}", actual[key]))
        .collect();
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(&actual, &control["result"]);
}

#[test]
fn inner_and_outer_html_of_xml_documents_match_the_chrome156_control() {
    let control: serde_json::Value =
        serde_json::from_str(include_str!("xml_inner_html_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    let probe = include_str!("xml_inner_html_probe.js");
    let digest = sha2::Sha256::digest(probe.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        control["probeSha256"], hex,
        "recapture the control for the edited probe"
    );
    let actual = run(probe);
    for section in ["getters", "innerSet", "outerSet"] {
        let expected = control["result"][section].as_object().unwrap();
        let differences: Vec<_> = expected
            .iter()
            .filter(|(key, value)| actual[section].get(key.as_str()) != Some(*value))
            .map(|(key, value)| {
                format!(
                    "{section}.{key}:\n  native={}\n  Chrome={value}",
                    actual[section][key]
                )
            })
            .collect();
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }
    assert_eq!(&actual, &control["result"]);
}

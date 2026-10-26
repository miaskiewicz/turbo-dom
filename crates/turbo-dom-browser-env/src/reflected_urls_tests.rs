//! turbo-dom#73: URL-reflecting `src` / `href` getters parse the attribute against the document
//! base URL (`<base href>`, else the document URL), against the Chrome 156 control
//! (`reflected_urls_probe.js` + `reflected_urls_cases.json` -> `reflected_urls_chrome156.json`,
//! recorded by scripts/select-element-chrome-control.mjs).
use super::*;
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("reflected_urls_probe.js");
const CASES: &[u8] = include_bytes!("reflected_urls_cases.json");
const CONTROL: &str = include_str!("reflected_urls_chrome156.json");

#[test]
#[ignore = "turbo-dom#73: fails until the fix"]
fn reflected_urls_parse_against_the_document_base_like_chrome156() {
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
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, cases[0]["html"].as_str().unwrap());
    {
        let id = context.get_slot::<DomContextId>().unwrap().0;
        let _dom = DomScope::enter(id);
        event_handlers::install(scope, None, "http://127.0.0.1/page/");
    }
    let code = format!("JSON.stringify({})", PROBE.trim().trim_end_matches(';'));
    let code = v8::String::new(scope, &code).unwrap();
    let value = {
        v8::tc_scope!(let scope, scope);
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
            .to_rust_string_lossy(scope)
    };
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    let expected = &control["result"][0]["result"];
    let mismatches: Vec<String> = expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| &actual[key.as_str()] != *value)
        .map(|(key, value)| format!("{key}\n  binding: {}\n  chrome:  {value}", actual[key]))
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    reset();
}

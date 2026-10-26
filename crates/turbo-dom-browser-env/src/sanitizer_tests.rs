use super::*;
use sha2::{Digest, Sha256};

/// Run a committed async probe against the binding with `page` as the parsed document.
fn run_probe(page: &str, probe: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, page);
    let source = format!(
        "Promise.resolve({}).then(JSON.stringify)",
        probe.trim().trim_end_matches(';')
    );
    let source = v8::String::new(scope, &source).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, source, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap_or_else(|| {
            panic!(
                "probe threw: {:?}",
                scope
                    .exception()
                    .map(|error| error.to_rust_string_lossy(scope))
            )
        });
    for _ in 0..1000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

/// Compare every `section.case` of a Chrome control, reporting all differences at once.
fn assert_matches_control(actual: &serde_json::Value, control: &serde_json::Value) {
    let mut differences = Vec::new();
    for (section, cases) in control["result"].as_object().unwrap() {
        match cases.as_object() {
            Some(cases) => {
                for (case, expected) in cases {
                    if &actual[section][case] != expected {
                        differences.push(format!(
                            "{section}.{case}\n  binding: {}\n  chrome:  {expected}",
                            actual[section][case]
                        ));
                    }
                }
                if actual[section].as_object().map(|object| object.len()) != Some(cases.len()) {
                    differences.push(format!("{section}: case set differs"));
                }
            }
            None if &actual[section] != cases => differences.push(format!(
                "{section}\n  binding: {}\n  chrome:  {cases}",
                actual[section]
            )),
            None => {}
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(actual, &control["result"]);
}

fn control(json: &str, probe: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    control
}

/// Captured by running src/sanitizer_probe.js in a fresh headless Chrome 156 profile
/// through scripts/shadow-html-chrome-control.mjs.
#[test]
fn sanitizer_api_matches_chrome_156() {
    let probe = include_str!("sanitizer_probe.js");
    let control = control(include_str!("sanitizer_chrome156.json"), probe);
    let actual = run_probe(
        "<!doctype html><html><head></head><body></body></html>",
        probe,
    );
    assert_matches_control(&actual, &control);
}

/// The recorded policy snapshots are exactly what Chrome 156 reports: the built-in
/// default (`new Sanitizer().get()`) and the remove-unsafe baseline.
#[test]
fn recorded_sanitizer_policies_match_chrome_156_get() {
    let control = control(
        include_str!("sanitizer_chrome156.json"),
        include_str!("sanitizer_probe.js"),
    );
    let default: serde_json::Value =
        serde_json::from_str(include_str!("browser_sanitizer_default.json")).unwrap();
    assert_eq!(control["result"]["defaultGet"], default["configuration"]);
    let baseline: serde_json::Value =
        serde_json::from_str(include_str!("sanitizer_safe_baseline.json")).unwrap();
    let unsafe_get = &control["result"]["methods"]["removeUnsafe"]["value"]["get"];
    assert_eq!(unsafe_get["removeElements"], baseline["removeElements"]);
    assert_eq!(unsafe_get["removeAttributes"], baseline["removeAttributes"]);
}

/// Declarative shadow DOM in the main-document parse and in
/// `Document.parseHTML[Unsafe]`, against src/declarative_shadow_page.html served
/// to Chrome 156 by scripts/shadow-html-chrome-control.mjs --page.
#[test]
fn declarative_shadow_roots_in_document_parsing_match_chrome_156() {
    let probe = include_str!("declarative_shadow_probe.js");
    // Normalize CRLF->LF so the fixture hashes/parses identically regardless of the
    // host git line-ending checkout (committed .html is LF; Windows autocrlf makes it CRLF).
    let page = include_str!("declarative_shadow_page.html").replace("\r\n", "\n");
    let control = control(include_str!("declarative_shadow_chrome156.json"), probe);
    assert_eq!(
        control["pageSha256"],
        format!("{:x}", Sha256::digest(page.as_bytes()))
    );
    let actual = run_probe(&page, probe);
    assert_matches_control(&actual, &control);
}

/// Scripts in declarative shadow roots stay parser scripts at their template's
/// parse position; scripts of ordinary template contents are not listed.
#[test]
fn parser_scripts_follow_parse_order_through_declarative_roots() {
    super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(
        scope,
        "<!doctype html><head><script id=a></script></head><body><div><script id=b></script><template shadowrootmode=open><script id=c></script><p><template shadowrootmode=closed><script id=d></script></template></p></template><script id=e></script></div><template><script id=inert></script></template><svg><script id=f></script></svg><script id=g></script>",
    );
    let document = {
        let code = v8::String::new(scope, "document").unwrap();
        let value = v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        v8::Local::<v8::Object>::try_from(value).unwrap()
    };
    let scripts = parser_scripts(scope, document);
    let ids = scripts
        .iter()
        .map(|script| {
            let key = v8::String::new(scope, "id").unwrap();
            script
                .get(scope, key.into())
                .unwrap()
                .to_rust_string_lossy(scope)
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, ["a", "b", "c", "d", "e", "f", "g"]);
    let state = script_state(scope, scripts[3]).unwrap();
    assert!(state.parser_inserted && !state.already_started);
    reset();
}

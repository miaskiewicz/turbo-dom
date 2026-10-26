use super::*;

/// Host navigation facts for the current realm's document, as an embedder
/// installs them after parsing.
pub(super) fn set_base(scope: &mut v8::PinScope, url: &str) {
    let id = scope
        .get_current_context()
        .get_slot::<DomContextId>()
        .map(|id| id.0);
    let _dom = id.map(DomScope::enter);
    event_handlers::install(scope, None, url);
}
fn document<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Object> {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "document").unwrap();
    global
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap()
}
/// A retired frame keeps its Document but loses its browsing context.
fn retire(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let child = args.data().to_object(scope).unwrap();
    let key = v8::String::new(scope, "document").unwrap();
    let document = child
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    assert!(set_document_resource_active(scope, document, false));
}

/// Run an authored probe in a fresh document with an independent same-isolate
/// child realm standing in for the Chrome control's same-origin iframe; the
/// probe's `frame.remove()` retires that child's browsing context. `setup` runs
/// in each realm (`true` for the child) before the probe.
pub(super) fn run_probe(
    html: &str,
    probe: &str,
    setup: impl Fn(&mut v8::PinScope, bool),
) -> serde_json::Value {
    super::super::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    let value = {
        v8::scope!(let scope, isolate);
        let parent = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, parent);
        install_with_html(scope, html);
        setup(scope, false);
        let child = v8::Context::new(scope, Default::default());
        child.set_security_token(parent.get_security_token(scope));
        {
            let child_scope = &mut v8::ContextScope::new(scope, child);
            install_independent_html(
                child_scope,
                "<!doctype html><html><head></head><body></body></html>",
            );
            setup(child_scope, true);
        }
        let global = parent.global(scope);
        let key = v8::String::new(scope, "__legacyChildRealm").unwrap();
        let child_global = child.global(scope);
        global.create_data_property(scope, key.into(), child_global.into());
        let remove = v8::Function::builder(retire)
            .data(child_global.into())
            .build(scope)
            .unwrap();
        let frame = v8::Object::new(scope);
        for (name, value) in [
            ("contentWindow", child_global.into()),
            ("remove", remove.into()),
        ] {
            let key = v8::String::new(scope, name).unwrap();
            frame.create_data_property(scope, key.into(), value);
        }
        let key = v8::String::new(scope, "__legacyChildFrame").unwrap();
        global.create_data_property(scope, key.into(), frame.into());
        let probe = probe.trim().trim_end_matches(';');
        let source = format!("(async()=>({probe}))().then(value=>globalThis.legacyResult=JSON.stringify(value),error=>globalThis.legacyResult='ERROR '+String(error&&error.stack||error));");
        let code = v8::String::new(scope, &source).unwrap();
        v8::tc_scope!(let scope, scope);
        if v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .is_none()
        {
            panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope));
        }
        scope.perform_microtask_checkpoint();
        let code = v8::String::new(scope, "globalThis.legacyResult||'unsettled'").unwrap();
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    };
    reset();
    clear_interface_records(isolate);
    serde_json::from_str(&value).unwrap_or_else(|_| panic!("{value}"))
}

/// The saved control, after checking it was captured from this exact probe and
/// case list by the pinned Chrome build.
fn control(fixture: &str, probe: &str, cases: &[u8]) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    let control: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.47");
    assert_eq!(control["v8"], "15.6.75.19");
    assert_eq!(control["timeZone"], "UTC");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    assert_eq!(
        control["casesSha256"],
        format!("{:x}", Sha256::digest(cases))
    );
    let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
    assert_eq!(
        control["result"].as_array().unwrap().len(),
        cases.as_array().unwrap().len()
    );
    control
}

#[test]
fn document_all_matches_chrome156_control() {
    let probe = include_str!("document_all_probe.js");
    let cases = include_bytes!("document_all_cases.json");
    let control = control(include_str!("document_all_chrome156.json"), probe, cases);
    let html = serde_json::from_slice::<serde_json::Value>(cases).unwrap()[0]["html"]
        .as_str()
        .unwrap()
        .to_owned();
    let actual = run_probe(&html, probe, |_, _| {});
    let expected = &control["result"][0]["result"];
    assert_eq!(&actual, expected);
}

/// Sloppy/strict assignment and deletion, Object.defineProperty/assign and the
/// Reflect forms reach document.all's and select's interceptors with V8's
/// should-throw mode; each answers exactly as Blink's interceptors do.
#[test]
fn interceptor_modes_match_chrome156_control() {
    let probe = include_str!("interceptor_mode_probe.js");
    let cases = include_bytes!("interceptor_mode_cases.json");
    let control = control(
        include_str!("interceptor_mode_chrome156.json"),
        probe,
        cases,
    );
    let html = serde_json::from_slice::<serde_json::Value>(cases).unwrap()[0]["html"]
        .as_str()
        .unwrap()
        .to_owned();
    let actual = run_probe(&html, probe, |_, _| {});
    assert_eq!(&actual, &control["result"][0]["result"]);
}

/// The navigation facts an embedder supplies for one control case: the
/// document's origin (opaque under CSP sandbox), its Last-Modified instant and
/// the Origin-Agent-Cluster and sandbox flags.
struct Navigation {
    origin: String,
    last_modified: Option<f64>,
    keyed: Option<bool>,
    sandboxed: bool,
}
fn navigation(case: &serde_json::Value) -> Navigation {
    let header = |name: &str| {
        case["headers"].as_array().and_then(|headers| {
            headers
                .iter()
                .find(|header| header[0] == name)
                .map(|header| header[1].as_str().unwrap().to_owned())
        })
    };
    let sandboxed =
        header("Content-Security-Policy").is_some_and(|policy| policy.starts_with("sandbox"));
    let scheme = if case["https"] == true {
        "https"
    } else {
        "http"
    };
    let origin = match case["url"].as_str() {
        Some(url) => url.to_owned(),
        None if sandboxed => "null".to_owned(),
        None => format!(
            "{scheme}://{}",
            case["host"].as_str().unwrap_or("127.0.0.1")
        ),
    };
    // The embedder parses the HTTP date; these are the instants of the fixtures' headers.
    let last_modified = header("Last-Modified").map(|value| match value.as_str() {
        "Wed, 21 Oct 2015 07:28:00 GMT" => 1_445_412_480_000.0,
        "Thu, 29 Feb 2024 23:59:59 GMT" => 1_709_251_199_000.0,
        other => panic!("unparsed fixture date {other}"),
    });
    let keyed = header("Origin-Agent-Cluster").map(|value| value == "?1");
    Navigation {
        origin,
        last_modified,
        keyed,
        sandboxed,
    }
}

#[test]
fn legacy_document_members_match_chrome156_control() {
    let probe = include_str!("document_legacy_probe.js");
    let cases = include_bytes!("document_legacy_cases.json");
    let control = control(include_str!("document_legacy_chrome156.json"), probe, cases);
    let cases: serde_json::Value = serde_json::from_slice(cases).unwrap();
    for (index, case) in cases.as_array().unwrap().iter().enumerate() {
        let facts = navigation(case);
        let html = case["html"]
            .as_str()
            .unwrap_or("<!doctype html><html><head></head><body></body></html>");
        let actual = run_probe(html, probe, |scope, child| {
            set_base(scope, &facts.origin);
            let document = document(scope);
            if let Some(keyed) = facts.keyed {
                assert!(set_document_origin_keyed(scope, document, keyed));
            }
            if !child {
                assert!(set_document_last_modified(
                    scope,
                    document,
                    facts.last_modified
                ));
                assert!(set_document_domain_sandboxed(
                    scope,
                    document,
                    facts.sandboxed
                ));
            }
        });
        let expected = &control["result"][index];
        assert_eq!(expected["id"], case["id"]);
        assert_eq!(actual, expected["result"], "Chrome case {}", case["id"]);
    }
}

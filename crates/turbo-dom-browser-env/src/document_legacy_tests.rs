use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;

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
    let isolate = &mut crate::browser_env::tests::new_isolate();
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
        v8::Script::compile(scope, code, None)
            .and_then(|script| script.run(scope))
            .unwrap_or_else(|| panic!("{}", thrown(scope.exception(), scope)));
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
        other => {
            assert_eq!(
                other, "Thu, 29 Feb 2024 23:59:59 GMT",
                "unparsed fixture date"
            );
            1_709_251_199_000.0
        }
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

/// A page-built `document.domain` value near V8's string limit makes the
/// SecurityError message, which quotes it, too long for a V8 string: throw
/// V8's RangeError instead of panicking the host.
#[test]
fn oversized_domain_refusal_message_throws_range_error() {
    let actual = run_probe(
        "<!doctype html><html><head></head><body></body></html>",
        r#"(() => { try { document.domain = ' '.repeat((1 << 29) - 24 - 4); return 'set'; } catch (error) { return error.name + ': ' + error.message; } })()"#,
        |scope, _| set_base(scope, "https://example.com/"),
    );
    assert_eq!(actual, "RangeError: Invalid string length");
}

/// Refusal messages that quote page-supplied text (a selector, an
/// HTMLAllCollection key) can pass V8's string length limit; like the domain
/// refusal they surface the #183 RangeError instead of aborting the host.
fn oversized_refusal(probe: &str) -> serde_json::Value {
    run_probe(
        "<!doctype html><html><head></head><body><p id=x></p></body></html>",
        &format!(
            "(() => {{ 'use strict'; const huge = '!' + ' '.repeat((1 << 29) - 24 - 4); \
             try {{ {probe}; return 'done'; }} catch (error) {{ return error.name + ': ' + error.message; }} }})()"
        ),
        |_, _| {},
    )
}

#[test]
fn oversized_invalid_selector_message_throws_range_error() {
    assert_eq!(
        oversized_refusal("document.querySelector(huge)"),
        "RangeError: Invalid string length"
    );
}

#[test]
fn oversized_webkit_matches_selector_message_throws_range_error() {
    assert_eq!(
        oversized_refusal("document.getElementById('x').webkitMatchesSelector(huge)"),
        "RangeError: Invalid string length"
    );
}

#[test]
fn oversized_all_collection_named_refusal_throws_range_error() {
    assert_eq!(
        oversized_refusal(
            "const key = huge.slice(1); document.getElementById('x').id = key; delete document.all[key]"
        ),
        "RangeError: Invalid string length"
    );
}

/// How the embedder wires a domain-probe cluster's origins.
#[derive(Clone, Copy, PartialEq)]
enum Sharing {
    /// Single-document embedder: every document keeps its own origin.
    None,
    /// Each child shares the top document's origin.
    Top,
    /// The srcdoc child shares the about:blank child's, which shares the top's.
    Chained,
}

/// One agent cluster as turbo-surf builds it: a top document plus an initial
/// `about:blank` and an `about:srcdoc` child realm whose origins the embedder
/// wires with `share_document_domain` (Blink's inherited `SecurityOrigin` is
/// one object); `setup` configures the top realm. The probe sees the children
/// as `__domainFrames.{blank,srcdoc}`.
fn run_domain_probe(
    probe: &str,
    sharing: Sharing,
    setup: impl Fn(&mut v8::PinScope),
) -> serde_json::Value {
    super::super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    let blank_html = "<!doctype html><html><head></head><body></body></html>";
    let value = {
        v8::scope!(let scope, isolate);
        let top = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, top);
        install_with_html(scope, blank_html);
        setup(scope);
        let mut origin = document(scope);
        let frames = v8::Object::new(scope);
        for (name, url) in [("blank", "about:blank"), ("srcdoc", "about:srcdoc")] {
            let child = v8::Context::new(scope, Default::default());
            child.set_security_token(top.get_security_token(scope));
            {
                let child_scope = &mut v8::ContextScope::new(scope, child);
                install_independent_html(child_scope, blank_html);
                set_base(child_scope, url);
                let child_document = document(child_scope);
                if sharing != Sharing::None {
                    assert!(share_document_domain(child_scope, child_document, origin));
                }
                if sharing == Sharing::Chained {
                    origin = child_document;
                }
            }
            let key = v8::String::new(scope, name).unwrap();
            let global = child.global(scope);
            frames.create_data_property(scope, key.into(), global.into());
        }
        let global = top.global(scope);
        let key = v8::String::new(scope, "__domainFrames").unwrap();
        global.create_data_property(scope, key.into(), frames.into());
        let probe = probe.trim().trim_end_matches(';');
        let source = format!("(async()=>({probe}))().then(value=>globalThis.domainResult=JSON.stringify(value),error=>globalThis.domainResult='ERROR '+String(error&&error.stack||error));");
        let code = v8::String::new(scope, &source).unwrap();
        v8::Script::compile(scope, code, None)
            .unwrap()
            .run(scope)
            .unwrap();
        scope.perform_microtask_checkpoint();
        let code = v8::String::new(scope, "globalThis.domainResult||'unsettled'").unwrap();
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

/// The domain control, after checking it was captured from this exact probe
/// and case list by the frozen chrome-headless-shell 156 oracle.
fn domain_control() -> (serde_json::Value, Vec<serde_json::Value>) {
    use sha2::{Digest, Sha256};
    let probe = include_str!("document_domain_probe.js");
    let cases = include_bytes!("document_domain_cases.json");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("document_domain_chrome156.json")).unwrap();
    assert_eq!(control["browser"], "HeadlessChrome/156.0.8078.47");
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
    let cases: Vec<serde_json::Value> = serde_json::from_slice(cases).unwrap();
    assert_eq!(control["result"].as_array().unwrap().len(), cases.len());
    (control, cases)
}

fn check_domain_control(sharing: Sharing) {
    let (control, cases) = domain_control();
    for (index, case) in cases.iter().enumerate() {
        let facts = navigation(case);
        let https = case["https"] == true;
        let actual = run_domain_probe(include_str!("document_domain_probe.js"), sharing, |scope| {
            set_base(scope, &facts.origin);
            // Chrome honours Origin-Agent-Cluster only in a secure context.
            if let Some(keyed) = facts.keyed.filter(|_| https) {
                let document = document(scope);
                assert!(set_document_origin_keyed(scope, document, keyed));
            }
        });
        let expected = &control["result"][index];
        assert_eq!(expected["id"], case["id"]);
        assert_eq!(actual, expected["result"], "Chrome case {}", case["id"]);
    }
}

/// turbo-dom#284: a document.domain set in any document of the cluster (top,
/// initial about:blank, srcdoc) relaxes all three; refusals, the origin-keyed
/// no-op and another realm's accessor match Chrome 156.
#[test]
fn shared_document_domain_matches_chrome156_control() {
    check_domain_control(Sharing::Top);
}

/// Sharing with a document that already shares another's origin joins that
/// origin, so a chain of frames is still one cluster.
#[test]
fn chained_document_domain_matches_chrome156_control() {
    check_domain_control(Sharing::Chained);
}

/// Without the hook every document keeps its own origin: the about:blank and
/// srcdoc children of a single-document embedder report an opaque origin's
/// empty domain and refuse assignment, and the top document relaxes alone.
#[test]
fn unshared_documents_keep_their_own_domain() {
    let actual = run_domain_probe(
        "(() => { const f = __domainFrames, set = (d, v) => { try { d.domain = v; return d.domain; } catch (e) { return e.name; } }; \
         return [set(document, 'example.com'), f.blank.document.domain, set(f.srcdoc.document, 'example.com'), f.srcdoc.document.domain, document.domain]; })()",
        Sharing::None,
        |scope| set_base(scope, "http://a.example.com/"),
    );
    assert_eq!(
        actual,
        serde_json::json!(["example.com", "", "SecurityError", "", "example.com"])
    );
}

/// The hook refuses anything but two browsing-context documents, and sharing a
/// document with itself keeps (or restores) its own origin.
#[test]
fn share_document_domain_requires_browsing_context_documents() {
    let actual = run_domain_probe(
        "[document.domain, __domainFrames.blank.document.domain]",
        Sharing::None,
        |scope| {
            set_base(scope, "http://a.example.com/");
            let top = document(scope);
            let code =
                v8::String::new(scope, "document.implementation.createHTMLDocument('x')").unwrap();
            let inert = v8::Script::compile(scope, code, None)
                .unwrap()
                .run(scope)
                .unwrap()
                .to_object(scope)
                .unwrap();
            assert!(!share_document_domain(scope, inert, top));
            assert!(!share_document_domain(scope, top, inert));
            let plain = v8::Object::new(scope);
            assert!(!share_document_domain(scope, plain, top));
            assert!(share_document_domain(scope, top, top));
        },
    );
    assert_eq!(actual, serde_json::json!(["a.example.com", ""]));
}

/// The embedder's `retire_context` for the realm the called function belongs to.
#[cfg(feature = "host-internals")]
fn retire_own_context(
    scope: &mut v8::PinScope,
    _: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    crate::browser_env::host_internals::retire_context(scope.get_current_context());
}

/// turbo-test#227: the only font-context input to the cascaded declarations is the html UA
/// table's browsing-context flag, and it changes only when the embedder retires the document
/// (`retire_context`), which moves no tree version. A retired document has no computed style at
/// all, so a transitioning element's reused declarations are never read there: it reads ""
/// like a plain element.
#[cfg(feature = "host-internals")]
#[test]
fn a_retired_document_reads_no_reused_declarations() {
    let value = run_probe(
        "<!doctype html><body>",
        r"(() => {
          const w = __legacyChildRealm, d = w.document;
          const s = d.createElement('style'); s.textContent = '.v { transition: opacity 100s linear; opacity: 1 } .v.dim { opacity: 0.5 }'; d.head.appendChild(s);
          const v = d.createElement('b'), plain = d.createElement('b'); v.className = 'v'; d.body.append(v, plain);
          const rows = [];
          const read = () => { const a = w.getComputedStyle(v), b = w.getComputedStyle(plain); rows.push([a.fontWeight, b.fontWeight, v.getAnimations().length]); };
          read(); v.classList.add('dim'); read(); read();
          w.__retireContext();
          read(); read();
          return rows;
        })()",
        |scope, child| {
            if child {
                let global = scope.get_current_context().global(scope);
                let name = v8::String::new(scope, "__retireContext").unwrap();
                let function = v8::Function::new(scope, retire_own_context).unwrap();
                global.set(scope, name.into(), function.into());
            }
        },
    );
    assert_eq!(
        value,
        serde_json::json!([
            ["700", "700", 0],
            ["700", "700", 1],
            ["700", "700", 1],
            ["", "", 1],
            ["", "", 1]
        ])
    );
}

//! Offline eligibility measurement for a captured {html,sheets} document.
use sha2::{Digest, Sha256};
use std::time::Instant;
use turbo_dom_browser_env::{
    browser_env,
    host_internals::{cached_own_box, interface_record, CachedOwnBox},
};

fn run<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> v8::Local<'s, v8::Value> {
    v8::tc_scope!(let caught, scope);
    let source = v8::String::new(caught, source).unwrap();
    v8::Script::compile(caught, source, None)
        .and_then(|script| script.run(caught))
        .unwrap_or_else(|| {
            panic!(
                "capture evaluation: {}",
                caught.exception().unwrap().to_rust_string_lossy(caught)
            )
        })
}
fn objects<'s>(scope: &mut v8::PinScope<'s, '_>, source: &str) -> Vec<v8::Local<'s, v8::Object>> {
    let array = v8::Local::<v8::Array>::try_from(run(scope, source)).unwrap();
    (0..array.length())
        .map(|i| array.get_index(scope, i).unwrap().to_object(scope).unwrap())
        .collect()
}
fn attach_link(scope: &mut v8::PinScope, owner: v8::Local<v8::Object>, text: &str) {
    let document = run(scope, "document").to_object(scope).unwrap();
    let record = interface_record(scope, document)
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "cssHost").unwrap();
    let host = record
        .get(scope, key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let key = v8::String::new(scope, "attachLinked").unwrap();
    let function = host.get(scope, key.into()).unwrap();
    let function = v8::Local::<v8::Function>::try_from(function).unwrap();
    let text = v8::String::new(scope, text).unwrap();
    let url = v8::String::new(scope, "https://offline.invalid/captured.css").unwrap();
    let clean = v8::Boolean::new(scope, true);
    function
        .call(
            scope,
            host.into(),
            &[owner.into(), text.into(), url.into(), clean.into()],
        )
        .unwrap();
}
fn restore_sheets(scope: &mut v8::PinScope, sheets: &serde_json::Value) {
    let owners=objects(scope,"globalThis.captureOwners=Array.from(document.querySelectorAll('link[rel~=stylesheet],style'))");
    let sheets = sheets.as_array().unwrap();
    assert_eq!(owners.len(), sheets.len(), "owner/sheet mapping");
    for (index, (owner, rules)) in owners.into_iter().zip(sheets).enumerate() {
        let key = v8::String::new(scope, "tagName").unwrap();
        let tag = owner
            .get(scope, key.into())
            .unwrap()
            .to_rust_string_lossy(scope);
        if tag == "LINK" {
            let text = rules
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            attach_link(scope, owner, &text);
        } else {
            run(scope,&format!("(()=>{{const sheet=captureOwners[{index}].sheet;while(sheet.cssRules.length)sheet.deleteRule(0);for(const rule of {rules})sheet.insertRule(rule,sheet.cssRules.length)}})()"));
        }
    }
}
fn measure(scope: &mut v8::PinScope) -> serde_json::Value {
    let styles = objects(
        scope,
        "globalThis.captureStyles=Array.from(document.querySelectorAll('style'))",
    );
    let mut proven = 0;
    let mut cold = 0;
    let mut nanos = 0;
    for (index, style) in styles.iter().enumerate() {
        cold += usize::from(cached_own_box(scope, *style) == CachedOwnBox::DisplayNone);
        run(
            scope,
            &format!("getComputedStyle(captureStyles[{index}]).position"),
        );
        let started = Instant::now();
        let answer = cached_own_box(scope, *style);
        nanos += started.elapsed().as_nanos();
        proven += usize::from(answer == CachedOwnBox::DisplayNone);
    }
    serde_json::json!({"styles":styles.len(),"already_proven_before_own_read":cold,"proven_after_position_read":proven,"unknown":styles.len()-proven,"probe_total_ns":nanos})
}
fn main() {
    let path = std::env::args().nth(1).expect("capture JSON path");
    let bytes = std::fs::read(path).unwrap();
    let capture: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope,isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_html(scope, capture["html"].as_str().unwrap());
    restore_sheets(scope, &capture["sheets"]);
    let result = measure(scope);
    println!(
        "{}",
        serde_json::json!({"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"result":result})
    );
    browser_env::reset();
}

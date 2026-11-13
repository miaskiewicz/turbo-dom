//! Keyframe member admission, independently captured from Chrome 156. This does
//! not claim sampled interpolation or exact math-expression serialization.
use std::sync::Once;
use turbo_dom_browser_env::browser_env;

#[test]
fn keyframe_length_and_math_members_match_chrome156() {
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/keyframe-length-membership-chrome156.json"
    ))
    .unwrap();
    assert_eq!(
        control["browser"],
        "Google Chrome for Testing 156.0.8078.47"
    );
    use sha2::{Digest, Sha256};
    let source = control["probe_source"].as_str().unwrap();
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(source.as_bytes()))
    );
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
        v8::V8::initialize();
    });
    let mut isolate = v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    v8::scope!(let scope, &mut isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    browser_env::install_with_html(scope, "<!doctype html><body></body>");
    let code = v8::String::new(scope, source).unwrap();
    let value = v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let actual: serde_json::Value = serde_json::from_str(&value).unwrap();
    assert_eq!(actual, control["members_present"]);
}

use sha2::{Digest, Sha256};
use turbo_dom_browser_env::browser_env::{install_independent_html, install_with_html, reset};

#[test]
fn touch_event_own_getters_across_realms_matches_chrome156() {
    let probe = include_str!("../scripts/touch-event-realm-probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("../src/touch_event_realm_chrome156.json")).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("156.0.8078."));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe))
    );
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_with_html(scope, "<body>control</body>");
    let foreign = v8::Context::new(scope, Default::default());
    let (foreign_touch, foreign_event) = {
        let scope = &mut v8::ContextScope::new(scope, foreign);
        install_independent_html(scope, "<body>foreign</body>");
        let source = v8::String::new(scope, "globalThis.foreignTouch=new Touch({identifier:9,target:document.body});globalThis.foreignTouchEvent=new TouchEvent('x',{touches:[foreignTouch],targetTouches:[foreignTouch],changedTouches:[foreignTouch],altKey:true,metaKey:false,ctrlKey:true,shiftKey:false});foreignTouch").unwrap();
        let touch = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let key = v8::String::new(scope, "foreignTouchEvent").unwrap();
        let event = foreign.global(scope).get(scope, key.into()).unwrap();
        (touch, event)
    };
    for (name, value) in [
        ("foreignTouch", foreign_touch),
        ("foreignTouchEvent", foreign_event),
    ] {
        let key = v8::String::new(scope, name).unwrap();
        context.global(scope).set(scope, key.into(), value);
    }
    let expression = probe.trim().trim_end_matches(';');
    let source = v8::String::new(scope, &format!("JSON.stringify({expression})")).unwrap();
    let result = v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap();
    let actual: serde_json::Value =
        serde_json::from_str(&result.to_rust_string_lossy(scope)).unwrap();
    assert_eq!(actual, control["result"]);
    reset();
}

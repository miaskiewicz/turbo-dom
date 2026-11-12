use sha2::{Digest, Sha256};
use turbo_dom_browser_env::browser_env::{
    install_with_html, reset, set_computed_style_mode, set_media_mode, ComputedStyleMode, MediaMode,
};
fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, source, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}
#[test]
fn computed_text_decoration_matches_jsdom_without_changing_chrome() {
    assert_eq!(ComputedStyleMode::default(), ComputedStyleMode::Chrome);
    let probe = include_str!("../scripts/text-decoration-jsdom-probe.js");
    let control: serde_json::Value =
        serde_json::from_str(include_str!("../src/text_decoration_jsdom27.json")).unwrap();
    assert!(control["jsdom"].as_str().unwrap().starts_with("27."));
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
    install_with_html(
        scope,
        "<span id=retained style='text-decoration:line-through'></span>",
    );
    let chrome = eval(scope, "globalThis.liveDecoration=getComputedStyle(document.getElementById('retained')); liveDecoration.textDecoration");
    assert!(chrome.contains("rgb(0, 0, 0)"));
    set_computed_style_mode(scope, ComputedStyleMode::Jsdom);
    set_media_mode(scope, MediaMode::Chrome);
    assert_eq!(eval(scope, "liveDecoration.textDecoration"), "line-through");
    assert_eq!(eval(scope, "document.getElementById('retained').style.textDecoration='underline'; liveDecoration.textDecoration"), "underline");
    assert_eq!(eval(scope, "document.getElementById('retained').style.cssText='text-decoration:line-through red'; liveDecoration.getPropertyValue('text-decoration')"), "line-through red");
    assert_eq!(eval(scope, "document.getElementById('retained').style.removeProperty('text-decoration'); liveDecoration.textDecoration"), "");
    eval(
        scope,
        "document.getElementById('retained').style.textDecoration='line-through'",
    );
    let actual: serde_json::Value = serde_json::from_str(&eval(
        scope,
        &format!("JSON.stringify({})", probe.trim().trim_end_matches(';')),
    ))
    .unwrap();
    assert_eq!(actual, control["result"]);
    set_computed_style_mode(scope, ComputedStyleMode::Chrome);
    set_media_mode(scope, MediaMode::Jsdom);
    assert_eq!(eval(scope, "liveDecoration.textDecoration"), chrome);
    reset();
}

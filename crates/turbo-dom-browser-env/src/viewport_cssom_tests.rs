//! turbo-dom#285-#288 (from turbo-test#230) against Chrome 156 controls recorded by
//! `scripts/viewport-chrome-control.mjs` in the frozen headless shell at a 1024x768
//! viewport: custom properties keep their authored colours (`custom_property_colors_*`),
//! `getComputedStyle` reports custom properties (`computed_custom_properties_*`),
//! `matchMedia` is a live `MediaQueryList` (`match_media_*`) and a child iframe window
//! reports its content box (`iframe_viewport_*`).
use super::*;
use sha2::{Digest, Sha256};

/// The committed control's result, after checking it was captured from `probe` by the
/// frozen Chrome 156 shell at the 1024x768 viewport the offline harness installs.
fn control(json: &str, probe: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control["browser"], "HeadlessChrome/156.0.8078.47");
    assert_eq!(control["v8"], "15.6.75.19");
    assert_eq!(
        control["viewport"],
        serde_json::json!({ "width": 1024, "height": 768 })
    );
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    control["result"].clone()
}

/// Run `probe` in the frozen shell with the embedder's 1024x768 viewport. Timers are a
/// FIFO task queue; `__resize` plays the collector: it sets the new viewport, dispatches
/// `resize` on the window and then calls `__resizeDone(id)`.
fn run(probe: &str) -> serde_json::Value {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><html><head></head><body></body></html>",
    );
    run_js(
        scope,
        "globalThis.innerWidth=1024;globalThis.innerHeight=768;\
         globalThis.outerWidth=1024;globalThis.outerHeight=856;\
         globalThis.__tasks=[];globalThis.setTimeout=callback=>{__tasks.push(callback);return __tasks.length};\
         globalThis.__resize=json=>{const {id,width,height}=JSON.parse(json);\
         __tasks.push(()=>{innerWidth=width;innerHeight=height;dispatchEvent(new Event('resize'));__tasks.push(()=>__resizeDone(id));});};",
    );
    let code = format!(
        "Promise.resolve().then(()=>({})).then(JSON.stringify)",
        probe.trim().trim_end_matches(';')
    );
    let code = v8::String::new(scope, &code).unwrap();
    v8::tc_scope!(let scope, scope);
    let promise = v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .and_then(|value| v8::Local::<v8::Promise>::try_from(value).ok())
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)));
    let step = v8::String::new(scope, "__tasks.length&&__tasks.shift()()").unwrap();
    let step = v8::Script::compile(scope, step, None).unwrap();
    for _ in 0..10_000 {
        scope.perform_microtask_checkpoint();
        if promise.state() != v8::PromiseState::Pending {
            break;
        }
        step.run(scope);
    }
    let value = promise.result(scope).to_rust_string_lossy(scope);
    assert_eq!(promise.state(), v8::PromiseState::Fulfilled, "{value}");
    reset();
    serde_json::from_str(&value).unwrap()
}

fn check(json: &str, probe: &str) {
    let expected = control(json, probe);
    assert_eq!(run(probe), expected);
}

/// turbo-dom#285: `--*-color` custom properties keep `#ff0000` in setProperty,
/// getPropertyValue and cssText; real colour properties still serialize as rgb().
#[test]
fn custom_property_colors_match_chrome156_control() {
    check(
        include_str!("custom_property_colors_chrome156.json"),
        include_str!("custom_property_colors_probe.js"),
    );
}

/// turbo-dom#286: getComputedStyle reports inline, stylesheet, style-attribute and
/// inherited custom properties with their specified (trimmed) text.
#[test]
fn computed_custom_properties_match_chrome156_control() {
    check(
        include_str!("computed_custom_properties_chrome156.json"),
        include_str!("computed_custom_properties_probe.js"),
    );
}

/// turbo-dom#287: matchMedia returns a MediaQueryList (an EventTarget) evaluated against
/// the window viewport, serializing its media text like Chrome and firing `change`
/// (legacy listeners, `change` listeners, `onchange`) when a resize flips it.
#[test]
fn match_media_matches_chrome156_control() {
    check(
        include_str!("match_media_chrome156.json"),
        include_str!("match_media_probe.js"),
    );
}

/// turbo-dom#288: a child iframe window's innerWidth / innerHeight are the iframe's live
/// content box (default 300x150, `width` / `height` attributes, CSS sizes, border-box
/// sizing); its outerWidth / outerHeight are the top-level window's.
#[test]
fn iframe_viewport_matches_chrome156_control() {
    check(
        include_str!("iframe_viewport_chrome156.json"),
        include_str!("iframe_viewport_probe.js"),
    );
}

/// turbo-dom#288 follow-up: an iframe's `width` / `height` attributes are presentational
/// hints in the cascade (author CSS wins, the default is 300x150), its window's viewport is
/// that computed content box (rounded like Chrome), and the child window's matchMedia
/// evaluates against it, firing `change` when the iframe resizes.
#[test]
fn iframe_media_matches_chrome156_control() {
    check(
        include_str!("iframe_media_chrome156.json"),
        include_str!("iframe_media_probe.js"),
    );
}

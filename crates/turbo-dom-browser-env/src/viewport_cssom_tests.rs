//! turbo-dom#285-#288 (from turbo-test#230) against Chrome 156 controls recorded by
//! `scripts/viewport-chrome-control.mjs` in the frozen headless shell at a 1024x768
//! viewport: custom properties keep their authored colours (`custom_property_colors_*`),
//! `getComputedStyle` reports custom properties (`computed_custom_properties_*`),
//! `matchMedia` is a live `MediaQueryList` (`match_media_*`) and a child iframe window
//! reports its content box (`iframe_viewport_*`); turbo-dom#299: a child iframe document
//! cascades its own sheets (`iframe_cascade_*`).
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
    run_in(probe, None)
}

/// [`run`] with the embedder's media mode set after install (`None` keeps the default).
fn run_in(probe: &str, mode: Option<MediaMode>) -> serde_json::Value {
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
    if let Some(mode) = mode {
        set_media_mode(scope, mode);
    }
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

/// turbo-dom#299: a child iframe document cascades its own style sheets (`srcdoc` and
/// injected `<style>`s, inline styles, inheritance and custom properties within the child
/// document, @supports / @layer / specificity) with @media evaluated against the iframe's
/// viewport, whichever window's getComputedStyle reads the element; the parent document's
/// rules never select child elements, and a child window reading a parent element uses the
/// parent's viewport.
#[test]
fn iframe_cascade_matches_chrome156_control() {
    check(
        include_str!("iframe_cascade_chrome156.json"),
        include_str!("iframe_cascade_probe.js"),
    );
}

/// The 0.7.1 app regressions: getComputedStyle evaluates @media against the viewport and
/// never asks a page's replacement `window.matchMedia` (a test mock), like Chrome; rules
/// added while the mock is installed are evaluated the same way.
#[test]
fn media_override_matches_chrome156_control() {
    check(
        include_str!("media_override_chrome156.json"),
        include_str!("media_override_probe.js"),
    );
    let explicit = run_in(
        include_str!("media_override_probe.js"),
        Some(MediaMode::Chrome),
    );
    assert_eq!(
        explicit,
        control(
            include_str!("media_override_chrome156.json"),
            include_str!("media_override_probe.js")
        )
    );
}

/// The jsdom 27.4.0 control's result, after checking it was recorded from `probe`.
fn jsdom_control(json: &str, probe: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(control["jsdom"], "27.4.0");
    assert_eq!(
        control["probeSha256"],
        format!("{:x}", Sha256::digest(probe.as_bytes()))
    );
    control["result"].clone()
}

/// MediaMode::Jsdom (turbo-test's default, for vitest parity): no matchMedia /
/// MediaQueryList, and a top-level @media rule applies only when its media list has an item
/// that is exactly `screen`, as jsdom 27.4.0's getComputedStyle does.
#[test]
fn jsdom_media_mode_matches_jsdom27_control() {
    let probe = include_str!("jsdom_media_probe.js");
    let expected = jsdom_control(include_str!("jsdom_media_jsdom27.json"), probe);
    assert_eq!(run_in(probe, Some(MediaMode::Jsdom)), expected);
    // Chrome's answers differ (the default mode), so the mode is what changed them.
    assert_ne!(run(probe), expected);
}

/// Switching modes on a live window re-cascades: Jsdom removes matchMedia and its
/// interfaces, Chrome restores the binding's own and evaluates @media again.
#[test]
fn media_mode_switches_back_and_forth() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(
        scope,
        "<!doctype html><html><head><style>@media (min-width: 1px) { #t { color: rgb(0, 0, 2) } }</style></head><body><div id=t></div></body></html>",
    );
    run_js_string(
        scope,
        "globalThis.innerWidth=1024;globalThis.innerHeight=768",
    );
    let read = "[typeof matchMedia, typeof MediaQueryList, typeof MediaQueryListEvent, getComputedStyle(document.getElementById('t')).color].join()";
    let chrome = run_js_string(scope, read);
    set_media_mode(scope, MediaMode::Jsdom);
    let jsdom = run_js_string(scope, read);
    set_media_mode(scope, MediaMode::Jsdom);
    let again = run_js_string(scope, read);
    set_media_mode(scope, MediaMode::Chrome);
    let back = run_js_string(scope, read);
    let list = run_js_string(
        scope,
        "String(matchMedia('(min-width: 1px)') instanceof MediaQueryList)",
    );
    reset();
    assert_eq!(chrome, "function,function,function,rgb(0, 0, 2)");
    assert_eq!(jsdom, "undefined,undefined,undefined,rgb(0, 0, 0)");
    assert_eq!(again, jsdom);
    assert_eq!(back, chrome);
    assert_eq!(list, "true");
    assert_eq!(MediaMode::default(), MediaMode::Chrome);
}

/// The CSS transitions and CSS animations switches, both off, hide jsdom's missing Web
/// Animations surface and restore the very same bindings when either is back on; either one
/// alone hides nothing (turbo-dom#313), and setting the current value again changes nothing.
/// They and the media mode are independent: none changes what the others control.
#[test]
fn css_transitions_switch_is_independent_of_media_mode() {
    super::tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    install_independent_html(scope, "<!doctype html><html><body></body></html>");
    run_js(scope, "globalThis.__animate = Element.prototype.animate");
    let read = "[typeof Animation, typeof AnimationEvent, typeof CSSTransition, typeof Element.prototype.animate, 'getAnimations' in ShadowRoot.prototype, 'timeline' in document, typeof matchMedia].join()";
    let on = "function,function,function,function,true,true,function";
    let off = "undefined,undefined,undefined,undefined,false,false,function";
    assert_eq!(run_js_string(scope, read), on);
    set_css_transitions(scope, false);
    assert_eq!(run_js_string(scope, read), on);
    set_css_animations(scope, false);
    assert_eq!(run_js_string(scope, read), off);
    set_css_transitions(scope, false);
    set_css_animations(scope, false);
    assert_eq!(run_js_string(scope, read), off);
    set_media_mode(scope, MediaMode::Jsdom);
    assert_eq!(
        run_js_string(scope, read),
        "undefined,undefined,undefined,undefined,false,false,undefined"
    );
    set_css_transitions(scope, true);
    let jsdom_media = run_js_string(scope, read);
    set_media_mode(scope, MediaMode::Chrome);
    let back = run_js_string(scope, read);
    set_css_transitions(scope, true);
    let again = run_js_string(scope, read);
    set_css_transitions(scope, false);
    let both_off_again = run_js_string(scope, read);
    set_css_animations(scope, true);
    let transitions_only_off = run_js_string(scope, read);
    let same = run_js_string(scope, "String(Element.prototype.animate === __animate)");
    reset();
    assert_eq!(
        jsdom_media,
        "function,function,function,function,true,true,undefined"
    );
    assert_eq!(back, on);
    assert_eq!(again, on);
    assert_eq!(both_off_again, off);
    assert_eq!(transitions_only_off, on);
    assert_eq!(same, "true");
}

fn run_js_string(scope: &mut v8::PinScope, source: &str) -> String {
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .unwrap()
        .run(scope)
        .unwrap()
        .to_rust_string_lossy(scope)
}

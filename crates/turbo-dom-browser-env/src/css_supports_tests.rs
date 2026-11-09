//! turbo-dom#322: `CSS.supports()` answered by the engine's own declaration grammar (the one
//! its cascade, `@supports` and `el.style` use), against the Chrome 156 control
//! `css_supports_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs). The binding installs no CSS namespace; an
//! embedder's `CSS.supports` calls the engine namespace's `cssSupports(text)` /
//! `cssSupportsProperty(property, value)`, as the stand-in below does.

/// The embedder's CSS namespace, as turbo-surf installs it over the engine namespace.
const CSS_NAMESPACE: &str = "globalThis.CSS = { supports(...args) { return args.length > 1 ? __tsNs.cssSupportsProperty(args[0], args[1]) : __tsNs.cssSupports(args[0]); } };";

/// turbo-dom#370: `CSS.supports` / its property form coerce their string arguments through an
/// internal primordial, so a page that replaces `globalThis.String` cannot break the engine.
/// Chrome 156 (and every browser) also uses a private coercion here.
#[test]
fn css_supports_uses_internal_string_primordial() {
    let probe = r#"(() => {
        const out = [];
        out.push(CSS.supports('color', 'red'));
        out.push(CSS.supports('color: red'));
        out.push(CSS.supports('(color: red)'));
        out.push(CSS.supports('bogus-prop', 'x'));
        return out;
    })()"#;
    let cases = br#"[{"html":"<!doctype html><html><head></head><body></body></html>","documentBase":"about:blank"}]"#;
    let setup = format!(
        "{CSS_NAMESPACE}\n\
         globalThis.String = function(){{ throw new Error('page replaced String'); }};\n\
         globalThis.String.prototype = null;"
    );
    let actual = super::interface_parity_tests::run_probe_with(probe, cases, &setup);
    assert_eq!(actual, serde_json::json!([true, true, true, false]));
}

#[test]
fn css_supports_matches_chrome156_control() {
    let probe = include_str!("css_supports_probe.js");
    let cases = include_bytes!("css_supports_cases.json");
    let expected = super::interface_parity_tests::control(
        probe,
        cases,
        include_str!("css_supports_chrome156.json"),
    );
    let actual = super::interface_parity_tests::run_probe_with(probe, cases, CSS_NAMESPACE);
    let found = super::interface_parity_tests::mismatches("", &actual, &expected);
    assert!(
        found.is_empty(),
        "mismatches vs Chrome 156:\n{}",
        found.join("\n")
    );
}

/// turbo-dom#390: four inputs that the engine's `cssSupports` / `cssSupportsProperty` once
/// answered `true` for, but Chrome 156 rejects (oracle headless-shell 156.0.8078.47):
/// `-moz-appearance: none` (vendor-prefixed property), and three `selector(...)` arguments
/// that the forgiving selector grammar must still reject — a top-level list and a forgiving
/// `:is()` / `:where()` wrapping an unknown pseudo.
///
/// turbo-dom#391: the `offset` shorthand must reject `/ <offset-anchor>` without a leading
/// position or path; Chrome 156 rejects `offset: / center` and `offset: / auto`.
#[test]
fn css_supports_rejects_chrome156_rejections() {
    let probe = r#"(() => {
        const out = [];
        // #390
        out.push(CSS.supports('-moz-appearance', 'none'));
        out.push(CSS.supports('-moz-appearance: none'));
        // R2 layout M2: CSS 2.2 §4.1.1 says property names are ASCII
        // case-insensitive. Chrome rejects every `-moz-*` spelling.
        out.push(CSS.supports('-MOZ-appearance', 'none'));
        out.push(CSS.supports('-Moz-APPEARANCE: none'));
        out.push(CSS.supports('selector(div, span)'));
        out.push(CSS.supports('selector(:is(:bogus))'));
        out.push(CSS.supports('selector(:where(:bogus, p))'));
        // #391
        out.push(CSS.supports('offset', '/ center'));
        out.push(CSS.supports('offset', '/ auto'));
        out.push(CSS.supports('offset: / center'));
        out.push(CSS.supports('offset: / auto'));
        return out;
    })()"#;
    let cases = br#"[{"html":"<!doctype html><html><head></head><body></body></html>","documentBase":"about:blank"}]"#;
    let actual = super::interface_parity_tests::run_probe_with(probe, cases, CSS_NAMESPACE);
    assert_eq!(
        actual,
        serde_json::json!([
            false, false, false, false, false, false, false, false, false, false, false,
        ]),
        "Chrome 156 rejects each of these; the engine must too"
    );
}

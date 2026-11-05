//! turbo-dom#322: `CSS.supports()` answered by the engine's own declaration grammar (the one
//! its cascade, `@supports` and `el.style` use), against the Chrome 156 control
//! `css_supports_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs). The binding installs no CSS namespace; an
//! embedder's `CSS.supports` calls the engine namespace's `cssSupports(text)` /
//! `cssSupportsProperty(property, value)`, as the stand-in below does.

/// The embedder's CSS namespace, as turbo-surf installs it over the engine namespace.
const CSS_NAMESPACE: &str = "globalThis.CSS = { supports(...args) { return args.length > 1 ? __tsNs.cssSupportsProperty(args[0], args[1]) : __tsNs.cssSupports(args[0]); } };";

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

//! turbo-dom#340: `Animation.prototype.rangeStart` / `rangeEnd` (the animation attachment
//! range) against the Chrome 156 control `animation_range_{probe.js,cases.json,chrome156.json}`
//! (recorded by scripts/select-element-chrome-control.mjs): the accessors, the setters' string
//! and `TimelineRangeOffset` conversions with Chrome's errors, `KeyframeAnimationOptions`
//! ranges, and a `CSSAnimation`'s `animation-range`.

/// The embedder's CSS namespace: turbo-surf installs the engine's `CSS.px()`-style factories.
const CSS_NAMESPACE: &str = "globalThis.CSS = {}; const factories = __tsNs.typedOmFactories(); for (let i = 0; i < factories.length; i++) CSS[factories[i][0]] = factories[i][1];";

#[test]
fn animation_range_matches_chrome156_control() {
    let probe = include_str!("animation_range_probe.js");
    let cases = include_bytes!("animation_range_cases.json");
    let expected = super::interface_parity_tests::control(
        probe,
        cases,
        include_str!("animation_range_chrome156.json"),
    );
    let actual = super::interface_parity_tests::run_probe_with(probe, cases, CSS_NAMESPACE);
    let found = super::interface_parity_tests::mismatches("", &actual, &expected);
    assert!(
        found.is_empty(),
        "{} mismatches vs Chrome 156:\n{}",
        found.len(),
        found.join("\n")
    );
}

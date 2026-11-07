//! turbo-dom#338: the tree-counting functions `sibling-index()` / `sibling-count()` in math
//! expressions and the `shape()` basic shape, against the Chrome 156 control
//! `css_tree_counting_shape_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs): `CSS.supports`, the specified value an inline
//! style and a style rule keep, and the computed value of each of four siblings.

/// The embedder's CSS namespace, as turbo-surf installs it over the engine namespace.
const CSS_NAMESPACE: &str = "globalThis.CSS = { supports(...args) { return args.length > 1 ? __tsNs.cssSupportsProperty(args[0], args[1]) : __tsNs.cssSupports(args[0]); } };";

#[test]
fn tree_counting_and_shape_match_chrome156_control() {
    let probe = include_str!("css_tree_counting_shape_probe.js");
    let cases = include_bytes!("css_tree_counting_shape_cases.json");
    let expected = super::interface_parity_tests::control(
        probe,
        cases,
        include_str!("css_tree_counting_shape_chrome156.json"),
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

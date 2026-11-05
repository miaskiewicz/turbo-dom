//! turbo-dom#358: CSS Nesting. `CSSStyleRule.prototype.cssRules` / `insertRule()` /
//! `deleteRule()`, Chrome 156's cssText for a nested style rule, `CSSNestedDeclarations`, and
//! the cascade of nested rules (with `&` resolution and the implicit `&` of a nested selector
//! that starts with a class, attribute, pseudo, or `>` / `+` / `~`). Chrome 156 control:
//! `css_nesting_{probe.js,cases.json,chrome156.json}` (recorded by
//! scripts/select-element-chrome-control.mjs).

#[test]
fn css_nesting_matches_chrome156_control() {
    let probe = include_str!("css_nesting_probe.js");
    let cases = include_bytes!("css_nesting_cases.json");
    let expected = super::interface_parity_tests::control(
        probe,
        cases,
        include_str!("css_nesting_chrome156.json"),
    );
    let actual = super::interface_parity_tests::run_probe_with(probe, cases, "");
    let found = super::interface_parity_tests::mismatches("", &actual, &expected);
    assert!(
        found.is_empty(),
        "{} mismatches vs Chrome 156:\n{}",
        found.len(),
        found.join("\n")
    );
}

//! turbo-dom#87 (from miaskiewicz/turbo-test#79 item 4): `SVGElement` and `MathMLElement`
//! include the `ElementCSSInlineStyle` (`style`), `HTMLOrSVGElement` (`dataset`, `nonce`,
//! `autofocus`, `tabIndex`, `focus()`, `blur()`) and `GlobalEventHandlers` (`on*`) mixins, and
//! MathML-namespace elements are `MathMLElement`s, so `svg.style.flexGrow = '1'` works.
//! Checked against the Chrome 156 control (`svg_mathml_element_probe.js` +
//! `svg_mathml_element_cases.json` -> `svg_mathml_element_chrome156.json`, recorded by
//! scripts/select-element-chrome-control.mjs).
use super::reference_target::probe_harness::{differences, run};
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("svg_mathml_element_probe.js");
const CASES: &[u8] = include_bytes!("svg_mathml_element_cases.json");
const CONTROL: &str = include_str!("svg_mathml_element_chrome156.json");

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
#[ignore = "turbo-dom#87: fails until the fix"]
fn svg_and_mathml_elements_include_the_html_or_svg_mixins_like_chrome() {
    let control: serde_json::Value = serde_json::from_str(CONTROL).unwrap();
    assert_eq!(control["browser"], "Chrome/156.0.8078.4");
    assert_eq!(control["errors"], serde_json::json!([]));
    assert_eq!(control["probeSha256"], sha(PROBE.as_bytes()));
    assert_eq!(control["casesSha256"], sha(CASES));
    let cases: serde_json::Value = serde_json::from_slice(CASES).unwrap();
    assert_eq!(
        cases[0]["html"],
        "<!doctype html><html><head></head><body></body></html>"
    );
    let expected = &control["result"][0]["result"];
    let actual = run(PROBE);
    let mut out = Vec::new();
    differences("result", &actual, expected, &mut out);
    assert!(out.is_empty(), "{}", out.join("\n"));
}

//! turbo-dom#441: `-webkit-*` / `-moz-*` property acceptance parity vs Chrome 156.
//!
//! The Chrome 156 control (`scripts/css-break-webkit-chrome-control.mjs`) records, per
//! prefixed declaration, `CSS.supports(prop, value)` plus the serialized value the
//! declaration keeps. [`cssom_parser::is_supported_property`] must agree with Chrome
//! at the property level: a property Chrome rejects for every value (e.g.
//! `-webkit-backdrop-filter`, every `-moz-*`) must return `false` here.

use super::cssom_parser::is_supported_property;

#[derive(serde::Deserialize)]
struct Control {
    prefixed: Vec<(String, String, bool, String)>,
}

#[test]
fn is_supported_property_matches_chrome156_prefixed() {
    let raw = include_str!("css_webkit_prefix_chrome156.json");
    let control: Control = serde_json::from_str(raw).expect("fixture parse");
    let mut mismatches = Vec::new();
    for (prop, value, chrome_supports, _held) in &control.prefixed {
        let ours = is_supported_property(prop);
        if ours != *chrome_supports {
            mismatches.push(format!(
                "{prop}: ours={ours} chrome={chrome_supports} (value={value})"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "is_supported_property disagrees with Chrome 156 on {} prefixed properties:\n  {}",
        mismatches.len(),
        mismatches.join("\n  "),
    );
}

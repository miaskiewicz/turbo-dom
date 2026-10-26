//! Every `test_support` constant is the exact bytes of the file it names, and
//! every JSON constant parses (lone UTF-16 surrogates in Chrome controls aside).
#![cfg(feature = "test-support")]

use turbo_dom_browser_env::test_support::{self, FILES};

#[test]
fn test_support_constants_are_the_shipped_files() {
    assert_eq!(FILES.len(), 70);
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (name, bytes) in FILES {
        assert_eq!(*bytes, std::fs::read(src.join(name)).unwrap(), "{name}");
        if name.ends_with(".json") {
            check_json(name, bytes);
        }
    }
    assert!(test_support::ANIMATIONS_FRAMES_CASES_JSON.starts_with(b"["));
    assert!(test_support::KEY_ORDER_PROBE_JS.contains("Object"));
    // FILES is sorted and unique, so a lookup by name is unambiguous.
    assert!(FILES.windows(2).all(|pair| pair[0].0 < pair[1].0));
    // The frame-read source is exported for embedders' agreement tests.
    let _: fn(&mut v8::PinScope<'_, '_>, &v8::PropertyCallbackArguments<'_>) -> bool =
        test_support::frame_read_should_throw;
}

/// The probe/control pairs turbo-surf's `tests/custom_elements.rs` contract runs:
/// each control was recorded by Chrome 155 for exactly that probe's bytes.
#[test]
fn custom_element_lifecycle_pairs_are_exported() {
    use sha2::{Digest, Sha256};
    let pairs = [
        (
            test_support::CUSTOM_ELEMENTS_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_EXTENDED_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_EXTENDED_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_REACTIONS_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_REACTIONS_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_IDL_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_IDL_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_MOVES_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_MOVES_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_CALLBACKS_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_CALLBACKS_CHROME155_JSON,
        ),
        (
            test_support::CUSTOM_ELEMENTS_RETIRED_PROBE_JS,
            test_support::CUSTOM_ELEMENTS_RETIRED_CHROME155_JSON,
        ),
        (
            test_support::DOM_PARSER_OBSERVER_LIFECYCLE_PROBE_JS,
            test_support::DOM_PARSER_OBSERVER_LIFECYCLE_CHROME155_JSON,
        ),
        (
            test_support::CSS_ADOPTION_LIFECYCLE_PROBE_JS,
            test_support::CSS_ADOPTION_LIFECYCLE_CHROME155_JSON,
        ),
    ];
    for (probe, control) in pairs {
        assert!(string_field(control, "browser").starts_with("Chrome/155."));
        assert_eq!(
            string_field(control, "probeSha256"),
            format!("{:x}", Sha256::digest(probe.as_bytes()))
        );
    }
}

/// Every JSON file parses, except that Chrome controls may record lone UTF-16
/// surrogates (`"\ud800"`), which `serde_json` rejects (`JSON.parse` in V8, where
/// consumers compare them, accepts them); such a file must fail for that reason only.
fn check_json(name: &str, bytes: &[u8]) {
    let Err(error) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return;
    };
    let text = std::str::from_utf8(bytes).expect(name).to_ascii_lowercase();
    let lone = (0xd800..0xe000_u32).any(|unit| text.contains(&format!("\\u{unit:x}")));
    assert!(
        lone && error.to_string().contains("escape"),
        "{name}: {error}"
    );
}

/// The string value of the first `"key": "value"` member of a control file.
fn string_field<'a>(json: &'a str, key: &str) -> &'a str {
    let start = json.find(&format!("\"{key}\": \"")).expect(key) + key.len() + 5;
    let len = json[start..].find('"').unwrap();
    &json[start..start + len]
}

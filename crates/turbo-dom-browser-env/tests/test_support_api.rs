//! Every `test_support` constant is the exact bytes of the file it names, and
//! every JSON constant parses (lone UTF-16 surrogates in Chrome controls aside).
#![cfg(feature = "test-support")]

use turbo_dom_browser_env::test_support::{self, FILES};

#[test]
fn test_support_constants_are_the_shipped_files() {
    assert_eq!(FILES.len(), 88);
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

/// The custom element reaction controls ship in the crate (no file outside it is read):
/// the DOM adoption / reactions control the npm runtime shares and the fragment-parser /
/// qualified-name one (miaskiewicz/turbo-dom P5b-X6).
#[test]
fn dom_reaction_controls_are_exported() {
    for name in [
        "dom_adoption_reactions_cases.json",
        "dom_adoption_reactions_chrome156.json",
        "dom_adoption_reactions_probe.js",
        "dom_reactions_names_cases.json",
        "dom_reactions_names_chrome156.json",
        "dom_reactions_names_probe.js",
    ] {
        assert!(
            FILES.iter().any(|(file, _)| *file == name),
            "{name} is not in test_support::FILES"
        );
    }
}

/// The crate's tests are self-contained: no `include_str!` / `include_bytes!` reaches
/// outside the crate, so they run from the published package too.
#[test]
fn tests_read_no_file_outside_the_crate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut outside = Vec::new();
    for dir in ["src", "tests"] {
        for entry in std::fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                for macro_name in ["include_str!(\"", "include_bytes!(\""] {
                    for (at, _) in text.match_indices(macro_name) {
                        let rest = &text[at + macro_name.len()..];
                        let file = &rest[..rest.find('"').unwrap()];
                        let target = path.parent().unwrap().join(file);
                        let resolved = std::fs::canonicalize(&target).unwrap_or(target);
                        if !resolved.starts_with(std::fs::canonicalize(root).unwrap()) {
                            outside.push(format!("{}: {file}", path.display()));
                        }
                    }
                }
            }
        }
    }
    assert!(outside.is_empty(), "{outside:#?}");
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

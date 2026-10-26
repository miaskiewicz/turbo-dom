//! Every `test_support` constant is the exact bytes of the file it names, and
//! every JSON constant parses.
#![cfg(feature = "test-support")]

use turbo_dom_browser_env::test_support::{self, FILES};

#[test]
fn test_support_constants_are_the_shipped_files() {
    assert_eq!(FILES.len(), 49);
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (name, bytes) in FILES {
        assert_eq!(*bytes, std::fs::read(src.join(name)).unwrap(), "{name}");
        if name.ends_with(".json") {
            serde_json::from_slice::<serde_json::Value>(bytes).expect(name);
        }
    }
    assert!(test_support::ANIMATIONS_FRAMES_CASES_JSON.starts_with(b"["));
    assert!(test_support::KEY_ORDER_PROBE_JS.contains("Object"));
}

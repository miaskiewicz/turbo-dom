#![cfg(feature = "test-support")]

use sha2::{Digest, Sha256};
use turbo_dom_browser_env::test_support::*;

#[test]
fn independent_popup_oracle_records_real_targets_for_the_exact_probe() {
    let control: serde_json::Value =
        serde_json::from_str(AUXILIARY_CONTEXTS_CHROME156_JSON).unwrap();
    let cases: serde_json::Value = serde_json::from_slice(AUXILIARY_CONTEXTS_CASES_JSON).unwrap();
    assert_eq!(control["browser"], "HeadlessChrome/156.0.8078.47");
    assert_eq!(control["errors"], serde_json::json!([]));
    for (field, bytes) in [
        ("casesSha256", AUXILIARY_CONTEXTS_CASES_JSON),
        ("probeSha256", AUXILIARY_CONTEXTS_PROBE_JS.as_bytes()),
        (
            "collectorSha256",
            include_bytes!("../scripts/auxiliary-contexts-chrome-control.mjs").as_slice(),
        ),
    ] {
        assert_eq!(control[field], format!("{:x}", Sha256::digest(bytes)));
    }
    let rows = control["result"].as_array().unwrap();
    assert_eq!(rows.len(), cases.as_array().unwrap().len());
    assert_eq!(rows.len(), 22);
    for case in cases.as_array().unwrap() {
        assert_eq!(rows.iter().filter(|row| row["id"] == case["id"]).count(), 1);
    }
    let row = |id: &str| rows.iter().find(|row| row["id"] == id).unwrap();
    assert_eq!(
        row("noopener-named-new")["host"]["live"][0]["name"],
        "Named"
    );
    assert_eq!(row("noopener-named-existing")["host"]["created"], 1);
    assert_eq!(
        row("noopener-named-existing")["result"]["returnedNull"],
        true
    );
    assert_eq!(
        row("noopener-named-existing")["host"]["live"][0]["openerNull"],
        false
    );
    for id in [
        "existing-target-keywords",
        "existing-target-keywords-noopener",
    ] {
        assert_eq!(row(id)["result"]["self"], true);
        assert_eq!(row(id)["result"]["parent"], true);
        assert_eq!(row(id)["result"]["top"], true);
        assert_eq!(row(id)["result"]["openerIsSelf"], false);
    }
    for id in [
        "noopener-bare",
        "noopener-spaced-yes",
        "noopener-true",
        "noopener-negative-integer",
        "noreferrer-implies-noopener",
    ] {
        let row = row(id);
        assert_eq!(row["result"]["returnedNull"], true);
        // Null alone is not evidence of an admitted popup. The independent
        // browser-target stream proves a real owned window was created.
        assert_eq!(row["host"]["created"], 1);
        assert_eq!(row["host"]["live"][0]["openerNull"], true);
    }
    assert_eq!(row("sandbox-popup-denied")["host"]["created"], 0);
    assert_eq!(row("sandbox-popup-admitted")["host"]["created"], 1);
    let noreferrer = row("noreferrer-implies-noopener");
    assert!(noreferrer["host"]["network"]
        .as_array()
        .unwrap()
        .iter()
        .all(|request| request["referrerEmpty"] == true));
    assert!(!noreferrer["host"]["network"].as_array().unwrap().is_empty());
}

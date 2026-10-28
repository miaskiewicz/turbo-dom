//! Native DOM members against the Chrome 156 control `native_guards_chrome156.json`
//! (`native_guards_probe.js`, recorded by `scripts/trusted-input-chrome-control.mjs`):
//! receiver brand checks, argument conversion errors and the edge results of the native
//! attribute, character-data, tree, collection and shadow-root operations.
use crate::browser_env::reference_target::probe_harness::{differences, run};
use sha2::{Digest, Sha256};

const PROBE: &str = include_str!("native_guards_probe.js");

/// A committed control's result, after checking it was captured from this probe in Chrome 156.
pub(super) fn control(json: &str, probe: &str) -> serde_json::Value {
    let control: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(control["browser"]
        .as_str()
        .unwrap()
        .starts_with("Chrome/156.0.8078."));
    let digest = Sha256::digest(probe.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        control["probeSha256"], hex,
        "control was captured from a different probe"
    );
    control["result"].clone()
}

/// KNOWN_GAP rows `(path, binding value)`: leaves where the binding still differs from
/// Chrome 156. Each must still differ exactly so; a fix removes its row.
const KNOWN_GAPS: &[(&str, &str)] = &[];

#[test]
#[ignore = "miaskiewicz/turbo-dom#114: fails until the fix"]
fn native_guards_match_chrome156() {
    let expected = control(include_str!("native_guards_chrome156.json"), PROBE);
    let actual = run(PROBE);
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    let mut gaps = Vec::new();
    out.retain(|entry| {
        let (path, rest) = entry.split_once('\n').unwrap_or((entry, ""));
        let binding = rest
            .lines()
            .find_map(|line| line.trim().strip_prefix("binding: "))
            .unwrap_or("");
        let known = KNOWN_GAPS.contains(&(path, binding));
        if known {
            gaps.push(path.to_string());
        }
        !known
    });
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
    let missing: Vec<_> = KNOWN_GAPS
        .iter()
        .filter(|(path, _)| !gaps.iter().any(|gap| gap == path))
        .collect();
    assert!(
        missing.is_empty(),
        "known gaps now match Chrome (remove them): {missing:?}"
    );
}

/// turbo-dom#114 hostile and boundary inputs against their Chrome 156 control
/// (`native_guards_hostile_probe.js`): proxies, revoked proxies, foreign-document nodes,
/// throwing conversions and dictionary getters reach the guards without aborting.
#[test]
#[ignore = "miaskiewicz/turbo-dom#114: fails until the fix"]
fn native_guard_hostile_inputs_match_chrome156() {
    let probe = include_str!("native_guards_hostile_probe.js");
    let expected = control(include_str!("native_guards_hostile_chrome156.json"), probe);
    let actual = run(probe);
    let mut out = Vec::new();
    differences("result", &actual, &expected, &mut out);
    assert!(
        out.is_empty(),
        "{} differences:\n{}",
        out.len(),
        out.join("\n")
    );
}

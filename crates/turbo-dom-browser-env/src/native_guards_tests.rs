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

/// KNOWN_GAP (miaskiewicz/turbo-dom#114): leaves where the binding still differs from
/// Chrome 156, with the binding's current value. Each must still differ exactly so; a fix
/// removes its row.
const KNOWN_GAPS: &[(&str, &str)] = &[
    ("result.arguments.closestNone", "null"),
    ("result.arguments.containsNone", "false"),
    ("result.arguments.containsPlain", "false"),
    ("result.arguments.getAttributeNone", "null"),
    ("result.arguments.hasAttributeNone", "false"),
    ("result.arguments.matchesNone", "false"),
    ("result.arguments.querySelectorNone", "null"),
    (
        "result.arguments.removeAttributeNodeElement[0]",
        "\"DOMException\"",
    ),
    (
        "result.arguments.removeAttributeNodeElement[1]",
        "\"NotFoundError\"",
    ),
    ("result.arguments.removeAttributeNone", "\"undefined\""),
    ("result.arguments.setAttribute1", "\"undefined\""),
    ("result.arguments.splitTextNone", "{}"),
    ("result.arguments.toggleAttributeBad", "true"),
    ("result.arguments.toggleAttributeNone", "true"),
    ("result.attrs.setAttributeBadName", "\"undefined\""),
    ("result.attrs.setAttributeEmpty", "\"undefined\""),
    ("result.doctype[5]", "\"\""),
    ("result.noops[0]", "\"undefined\""),
    ("result.noops[1]", "\"undefined\""),
    ("result.noops[2]", "\"undefined\""),
    ("result.receivers.Element.after[1]", "\"undefined\""),
    ("result.receivers.Element.before[1]", "\"undefined\""),
    ("result.receivers.Element.remove[1]", "\"undefined\""),
    ("result.receivers.Element.replaceWith[1]", "\"undefined\""),
    (
        "result.receivers.Element.scrollBy[0]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    (
        "result.receivers.Element.scrollBy[1]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    (
        "result.receivers.Element.scrollIntoView[0]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    (
        "result.receivers.Element.scrollIntoView[1]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    (
        "result.receivers.Element.scrollTo[0]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    (
        "result.receivers.Element.scrollTo[1]",
        "[\"TypeError\",\"TypeError\"]",
    ),
    // miaskiewicz/turbo-dom#114: document.createComment() without its argument.
    ("result.conversions.valid[11]", "{}"),
];

#[test]
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

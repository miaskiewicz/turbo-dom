//! turbo-dom#425: `sweep_collected`'s stale-set lookup must stay O(1), not O(n*m). The
//! function builds two HashSets (`stale_set`, `other_dom_ids`) and reads from them on each
//! inner-loop pass. This test is a static source-shape guard: it fails if the HashSets
//! regress to a Vec/slice `.contains`, which `sweep_collected` would reach back for if a
//! future refactor unrolled the collections in place.

#[test]
fn sweep_collected_uses_hashset_lookups() {
    let src = include_str!("dom_ownership.rs");
    let fn_start = src
        .find("pub(super) fn sweep_collected")
        .expect("sweep_collected signature");
    // The function body ends at the next `fn ` or two newlines followed by `pub ` at col 0.
    // Take enough to cover it comfortably.
    let fn_src = &src[fn_start..];
    let end = fn_src
        .find("\npub(super) fn ")
        .or_else(|| fn_src.find("\npub fn "))
        .or_else(|| fn_src.find("\nfn "))
        .unwrap_or(fn_src.len().min(4096));
    let body = &fn_src[..end];
    assert!(
        body.contains("HashSet"),
        "sweep_collected lost its HashSet lookup — stale-set membership is O(n*m) again:\n{body}",
    );
    // Guard against an accidental `Vec`-based `contains` on the hot-path id check.
    assert!(
        !body.contains(".iter().any("),
        "sweep_collected introduced a linear `.iter().any(…)` scan on the stale/other-dom ids path:\n{body}",
    );
}

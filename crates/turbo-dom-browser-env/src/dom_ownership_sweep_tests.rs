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
    // Both lookup-side collections (`stale_set` and `other_dom_ids`) must
    // be declared as `HashSet<_>`. A mutant that converts even one to
    // `Vec<_>` re-introduces O(n) membership on the inner loop; a mutant
    // that converts BOTH must still fail. Match the full `let <name>: … HashSet`
    // prefix so the check cannot be satisfied by an unrelated `HashSet`
    // token elsewhere in the body (e.g. in a comment or a different binding).
    for binding in ["stale_set", "other_dom_ids"] {
        let needle_hashset = format!("let {binding}: ");
        let pos = body.find(&needle_hashset).unwrap_or_else(|| {
            panic!("sweep_collected lost the `let {binding}: ...` binding:\n{body}")
        });
        // The type annotation ends at `=`; scan from there back to ensure
        // `HashSet` appears in the TYPE, not merely somewhere later in body.
        let tail = &body[pos..];
        let eq = tail
            .find('=')
            .unwrap_or_else(|| panic!("binding {binding} missing `=` in its let:\n{body}"));
        let type_annotation = &tail[..eq];
        assert!(
            type_annotation.contains("HashSet"),
            "sweep_collected: `{binding}` is no longer a HashSet (annotation=`{type_annotation}`). \
             Stale-set membership is O(n*m) again:\n{body}",
        );
        assert!(
            !type_annotation.contains("Vec<"),
            "sweep_collected: `{binding}` regressed to a Vec<_> (annotation=`{type_annotation}`). \
             Inner-loop membership is now O(n):\n{body}",
        );
    }
    // Guard against an accidental `Vec`-based `contains` on the hot-path id check.
    assert!(
        !body.contains(".iter().any("),
        "sweep_collected introduced a linear `.iter().any(…)` scan on the stale/other-dom ids path:\n{body}",
    );
}

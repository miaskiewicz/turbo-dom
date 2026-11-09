//! turbo-dom#425: the second-pass id lookup in `sweep_collected` must stay O(1), so
//! sweeping N retired contexts over M registered documents stays O(N + M), not
//! O(N * M). The source-shape grep the first R-round added defended the current
//! `HashSet<_>` binding spelling, not the actual lookup complexity: a mutant that
//! kept the `let stale_set:` shape but swapped the container for a `Vec<_>` with
//! `.contains` would pass the grep and silently regress complexity (R2 adversarial
//! finding M1). This file replaces the grep with a BEHAVIOURAL test on the
//! extracted `compute_stale_ids` helper, using a key type whose `PartialEq`/`Hash`
//! implementations count every call. HashSet-backed lookup touches each key in
//! O(1) expected, so N inputs produce O(N) equality calls; a Vec-backed lookup
//! would scan the whole set on every empty-context id and produce Θ(N²) calls.
//!
//! Mutant check (performed during authoring): locally replacing the two
//! `HashSet<K>` bindings in `compute_stale_ids` with `Vec<K>` and the two
//! `.contains(&id)` calls with `.iter().any(|k| *k == id)` drives the equality
//! count above the `O(N²)` floor this test rejects, so the test fails fast
//! under that regression. (Reverted before committing.)
//!
//! The old source-shape grep is kept as a cheap SECONDARY guard — a readable
//! hint when the helper itself is edited — but the complexity is enforced by
//! the behavioural assertion below.

use std::cell::Cell;
use std::hash::{Hash, Hasher};

thread_local! {
    static EQ_CALLS: Cell<usize> = const { Cell::new(0) };
    static HASH_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Copy, Clone, Debug)]
struct Counting(u64);

impl PartialEq for Counting {
    fn eq(&self, other: &Self) -> bool {
        EQ_CALLS.with(|c| c.set(c.get() + 1));
        self.0 == other.0
    }
}
impl Eq for Counting {}
impl Hash for Counting {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASH_CALLS.with(|c| c.set(c.get() + 1));
        self.0.hash(state);
    }
}

fn reset_counters() {
    EQ_CALLS.with(|c| c.set(0));
    HASH_CALLS.with(|c| c.set(0));
}

/// Behavioural guard: with N = 1000 OTHER_DOMS entries (none graph-stale) and
/// N DOM_CONTEXTS empty-weak ids that all exist in OTHER_DOMS, the second
/// pass runs N iterations and each must do O(1) lookup work on both sets.
/// A HashSet-backed implementation performs roughly one `PartialEq` call per
/// lookup (expected ~O(1) on each hash hit). A `Vec::contains` regression
/// would scan the whole stale/other set for each id, producing Θ(N²).
#[test]
fn compute_stale_ids_lookup_is_sublinear_per_id() {
    const N: u64 = 1000;
    reset_counters();
    let other_dom_rows: Vec<(Counting, bool)> = (0..N).map(|i| (Counting(i), false)).collect();
    let empty_context_ids: Vec<Counting> = (0..N).map(Counting).collect();
    let stale = super::dom_ownership::compute_stale_ids(other_dom_rows, empty_context_ids);
    assert_eq!(stale.len() as u64, N, "every empty-weak id is pruned");

    let eq_calls = EQ_CALLS.with(Cell::get);
    let hash_calls = HASH_CALLS.with(Cell::get);
    // Vec-backed `contains` would be Θ(N²) ≈ 2 * N * N equality calls (two
    // `.contains` sites per iteration, each scanning N entries).
    // HashSet-backed is ~O(N) hashes plus a small constant per collision.
    // We pick a threshold well below Θ(N²) but well above the HashSet case:
    // a HashSet with random u64 keys usually sees ≤ 2 * N equality calls;
    // Vec regression would be ≥ N * N ≈ 10^6. 100 * N = 10^5 cleanly splits.
    let quadratic_floor = (N as usize) * (N as usize);
    let regression_threshold = 100 * (N as usize);
    assert!(
        eq_calls < regression_threshold,
        "compute_stale_ids made {eq_calls} equality calls for N={N}; \
         HashSet-backed stays well below {regression_threshold}. A Vec `.contains` \
         regression would be around {quadratic_floor}.",
    );
    assert!(
        hash_calls <= 4 * (N as usize),
        "compute_stale_ids made {hash_calls} hash calls for N={N}; the HashSet \
         path stays ~O(N) hashes.",
    );
}

/// Secondary soft check: the old source-shape grep. Kept to flag a surprise
/// edit to `compute_stale_ids`' body (the behavioural test above is the real
/// complexity gate; this one catches a reviewer-visible regression early).
#[test]
fn compute_stale_ids_still_uses_hashset_bindings() {
    let src = include_str!("dom_ownership.rs");
    let fn_start = src
        .find("pub(super) fn compute_stale_ids")
        .expect("compute_stale_ids signature");
    let fn_src = &src[fn_start..];
    let end = fn_src
        .find("\npub(super) fn ")
        .or_else(|| fn_src.find("\npub fn "))
        .or_else(|| fn_src.find("\nfn "))
        .unwrap_or(fn_src.len().min(4096));
    let body = &fn_src[..end];
    for binding in ["other_dom_set", "stale_set"] {
        let pos = body
            .find(&format!("let {binding}: "))
            .or_else(|| body.find(&format!("let mut {binding}: ")))
            .unwrap_or_else(|| {
                panic!("compute_stale_ids lost the `let {binding}: ...` binding:\n{body}")
            });
        let tail = &body[pos..];
        let eq = tail
            .find('=')
            .unwrap_or_else(|| panic!("binding {binding} missing `=` in its let:\n{body}"));
        let type_annotation = &tail[..eq];
        assert!(
            type_annotation.contains("HashSet"),
            "compute_stale_ids: `{binding}` is no longer a HashSet (annotation=`{type_annotation}`); \
             the behavioural test is the complexity gate, but this is the readable hint:\n{body}",
        );
    }
    assert!(
        !body.contains(".iter().any("),
        "compute_stale_ids introduced a linear `.iter().any(…)` scan on the stale/other-dom ids path:\n{body}",
    );
}

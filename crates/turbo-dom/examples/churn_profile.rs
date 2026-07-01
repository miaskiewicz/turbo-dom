//! CPU-time profiling harness for the rtdom mutation-churn hot path.
//!
//! This reproduces, byte-for-byte, the `mutation churn (200 nodes)` closure from
//! `bench.rs::large_tree_report` (the row the dhat heap profile targeted), and
//! loops it for a fixed large iteration count so a sampling profiler (`sample`,
//! `samply`, Instruments) collects thousands of stacks at steady state.
//!
//! WHAT'S IN THE LOOP: the closure calls `Tree::parse("<main id=root></main>")`
//! once per iteration — a tiny fresh document, NOT the depth=6/width=8 `big_fixture`
//! (that fixture is the *base* document the OTHER report rows query; the churn row
//! builds its own fresh tiny tree). The per-iter parse is therefore a workload
//! artifact (the dhat profile attributed ~61 allocs/op to it). We profile the
//! closure AS WRITTEN (parse inside) so the CPU ranking lines up 1:1 with the dhat
//! alloc ranking; `parse`/markup5ever frames are bucketed separately in the report
//! so the mutation-only cost is still readable.
//!
//! Usage:
//!   cargo build --release --example `churn_profile` -p turbo-dom
//!   ./`target/release/examples/churn_profile` [iterations]
//!   # in another shell: sample <pid> 20 -file /tmp/churn.sample.txt

use std::hint::black_box;
use turbo_dom::Tree;

/// One iteration = the exact `mutation churn (200 nodes)` body from bench.rs.
#[inline(never)]
fn churn_once() -> u64 {
    let mut t = Tree::parse("<main id=root></main>");
    let root = t.query_selector("#root").unwrap();
    let mut made = Vec::with_capacity(200);
    for i in 0..100 {
        let li = t.create_element("li");
        t.set_attribute(li, "class", "item");
        t.set_attribute(li, "data-i", &i.to_string());
        let txt = t.create_text_node("x");
        t.append_child(li, txt);
        t.append_child(root, li);
        made.push(li);
    }
    // remove half, re-append a fresh batch (overlay churn)
    for (i, &li) in made.iter().enumerate() {
        if i % 2 == 0 {
            t.remove_child(root, li);
        }
    }
    for _ in 0..50 {
        let span = t.create_element("span");
        t.append_child(root, span);
        t.remove_child(root, span);
    }
    t.version
}

fn main() {
    // Default chosen for ~20-40s of steady-state runtime on arm64 release.
    let iters: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3_000_000);

    eprintln!("churn_profile: pid={} iters={}", std::process::id(), iters);

    let mut acc: u64 = 0;
    let start = std::time::Instant::now();
    for _ in 0..iters {
        acc = acc.wrapping_add(black_box(churn_once()));
    }
    let elapsed = start.elapsed();

    // black_box the accumulator so the whole loop can't be optimized away.
    black_box(acc);
    eprintln!(
        "churn_profile: done {} iters in {:.2}s ({:.0} ops/s, acc={})",
        iters,
        elapsed.as_secs_f64(),
        iters as f64 / elapsed.as_secs_f64(),
        acc
    );
}

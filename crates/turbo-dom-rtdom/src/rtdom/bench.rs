//! rtdom hotspot harness. Times each core operation on a realistic fixture so we
//! can see what dominates a render+query+assert workload and where to optimize.
//! Run: `cargo test --release --lib rtdom::bench::hotspot_report -- --ignored --nocapture`.
#![cfg(test)]

use super::cascade;
use super::events::{Dom, Event};
use super::serialize;
use super::tree::Tree;
use std::time::Instant;

/// Realistic-ish page: a styled card grid (classes, ids, data-*, nested text).
fn fixture(cards: usize) -> String {
    let mut s = String::from(
        "<!doctype html><html><head><style>\
.card{color:blue;padding:8px;margin:4px}\
.card .title{font-weight:bold;font-size:14px}\
#app{background:white}\
.btn{color:green}\
</style></head><body><main id=app class=grid>",
    );
    for i in 0..cards {
        s.push_str(&format!(
            "<div class=\"card sx-{}\" data-testid=\"card-{}\" id=\"c{}\">\
<h2 class=title>Title {}</h2><p class=body>Body text for card {}.</p>\
<button class=btn type=button>Action</button></div>",
            i % 7, i, i, i, i
        ));
    }
    s.push_str("</main></body></html>");
    s
}

/// best-of-N ops/s for `f` (warmup + timed windows). `f` returns an observable sink.
fn bench<F: FnMut() -> u64>(mut f: F, ms: u128) -> f64 {
    let mut best = 0.0f64;
    let mut sink = 0u64;
    for _ in 0..6 {
        for _ in 0..50 {
            sink = sink.wrapping_add(f());
        }
        let start = Instant::now();
        let mut iters = 0u64;
        while start.elapsed().as_millis() < ms {
            sink = sink.wrapping_add(f());
            iters += 1;
        }
        let ops = iters as f64 / start.elapsed().as_secs_f64();
        if ops > best {
            best = ops;
        }
    }
    std::hint::black_box(sink);
    best
}

/// Multi-dimension numbers for the cross-DOM comparison (bench/compare-all.mjs reads
/// the `RTDOM_JSON` line). Run:
///   cargo test --release --lib --features rust-runtime rtdom::bench::compare_all -- --ignored --nocapture
#[test]
#[ignore]
fn compare_all() {
    use super::tree::Tree;
    let html = fixture(300);

    // D1 parse: unique HTML each iter (rtdom has no parse cache; keep it honest anyway)
    let mut i = 0u64;
    let parse = bench(|| { i += 1; Tree::parse(&format!("{html}<!--{i}-->")).node_count() as u64 }, 400);

    // D2 construct + light query
    let mut j = 0u64;
    let construct = bench(|| {
        j += 1;
        let t = Tree::parse(&format!("{html}<!--{j}-->"));
        t.query_selector_all("div.card").len() as u64
    }, 400);

    // D3 per-node chatty access over a fixed element list
    let tree = Tree::parse(&html);
    let cards = tree.query_selector_all("div.card");
    let per_node = bench(|| {
        let mut s = 0u64;
        for &el in cards.iter() {
            if let Some(c) = tree.get_attribute(el, "class") { s += c.len() as u64; }
            if let Some(t) = tree.get_attribute(el, "data-testid") { s += t.len() as u64; }
            if let Some(t) = tree.tag_name(el) { s += t.len() as u64; }
            let mut p = tree.parent(el);
            while let Some(x) = p { s += 1; p = tree.parent(x); }
        }
        s
    }, 400);

    // D4 repeated query on an unchanged tree (version-cached)
    let repeated = bench(|| tree.query_selector_all("div.card").len() as u64, 400);

    println!(
        "RTDOM_JSON {{\"parse\":{parse:.0},\"construct\":{construct:.0},\"per_node\":{per_node:.0},\"repeated_query\":{repeated:.0}}}"
    );
}

#[test]
#[ignore]
fn hotspot_report() {
    let n = 300;
    let html = fixture(n);
    let base = Tree::parse(&html);
    let cards = base.query_selector_all("div.card");
    let mut rows: Vec<(String, f64)> = Vec::new();

    // 1. parse (cold tree build per call)
    rows.push(("parse(300 cards)".into(), bench(|| Tree::parse(&html).node_count() as u64, 300)));

    // 2. querySelectorAll cached (repeated query, unchanged tree = cache hit — RTL pattern)
    rows.push(("qsa div.card (cached)".into(), bench(|| base.query_selector_all("div.card").len() as u64, 300)));

    // 3. querySelectorAll cold (fresh tree each time — full matcher walk, no cache benefit)
    rows.push(("qsa div.card (cold tree)".into(), bench(|| {
        let t = Tree::parse(&html);
        t.query_selector_all(".card .title").len() as u64
    }, 300)));

    // 4. getElementById (uncached full-tree walk)
    rows.push(("getElementById".into(), bench(|| base.get_element_by_id("c250").map_or(0, |h| h as u64), 300)));

    // 5. getAttribute over all cards (per-node buffer scan)
    rows.push(("getAttribute x3/card".into(), bench(|| {
        let mut s = 0u64;
        for &c in cards.iter() {
            s += base.get_attribute(c, "class").map_or(0, |v| v.len() as u64);
            s += base.get_attribute(c, "data-testid").map_or(0, |v| v.len() as u64);
            s += base.get_attribute(c, "id").map_or(0, |v| v.len() as u64);
        }
        s
    }, 300)));

    // 6. computed_style per card (cascade — rebuilds the rule index each call?)
    rows.push(("getComputedStyle/card".into(), bench(|| {
        let mut s = 0u64;
        for &c in cards.iter() {
            s += cascade::computed_style(&base, c).len() as u64;
        }
        s
    }, 400)));

    // 7. serialize the whole grid
    let root = base.root();
    rows.push(("serialize_inner(root)".into(), bench(|| serialize::serialize_inner(&base, root).len() as u64, 300)));

    // 8. dispatch listener-less (React fires thousands)
    {
        let mut dom = Dom::parse(&html);
        let btn = dom.tree.query_selector("button").unwrap();
        rows.push(("dispatch listener-less".into(), bench(|| {
            let mut e = Event::new("click", true, true);
            dom.dispatch_event(btn, &mut e) as u64
        }, 300)));
    }

    // 9. mutation: append + setAttribute + remove (no observer)
    rows.push(("mutate append+attr+remove".into(), bench(|| {
        let mut t = Tree::parse("<div id=root></div>");
        let root = t.query_selector("#root").unwrap();
        let el = t.create_element("span");
        t.set_attribute(el, "class", "x");
        t.append_child(root, el);
        t.remove_child(root, el);
        t.version
    }, 300)));

    rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    println!("\n=== rtdom hotspot report ({} cards, best-of-6 ops/s, slowest first) ===", n);
    for (name, ops) in &rows {
        let ns = if *ops > 0.0 { 1e9 / ops } else { 0.0 };
        println!("  {:<28} {:>12.0} ops/s   {:>12.0} ns/op", name, ops, ns);
    }
    println!();
}

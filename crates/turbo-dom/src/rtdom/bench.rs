//! rtdom hotspot harness. Times each core operation on a realistic fixture so we
//! can see what dominates a render+query+assert workload and where to optimize.
//! Run: `cargo test --release --lib rtdom::bench::hotspot_report -- --ignored --nocapture`.

use super::cascade;
use super::events::{Dom, Event};
use super::serialize;
use super::tree::Tree;
use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

// --- counting global allocator (test build only) ----------------------------
//
// A `#[global_allocator]` is process-wide for the *test binary*: it sees every
// heap allocation the test process makes, not just the closure under measure.
// That's fine for our harness because we `reset()` immediately before a single-
// threaded fixed-N pass and `snapshot()` right after, so the delta is dominated
// by the closure under test. The counters are plain relaxed `AtomicUsize`s — a
// couple of atomic adds per alloc/dealloc, far cheaper than the allocation
// itself, so the instrumentation doesn't meaningfully perturb the numbers it
// reports.

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOC_COUNT.fetch_add(1, Relaxed);
            ALLOC_BYTES.fetch_add(layout.size(), Relaxed);
            let live = LIVE_BYTES.fetch_add(layout.size(), Relaxed) + layout.size();
            // relaxed max-update: a race can only under-report peak, never over.
            PEAK_BYTES.fetch_max(live, Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        LIVE_BYTES.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            // Count a realloc as one allocation event; track the byte delta.
            ALLOC_COUNT.fetch_add(1, Relaxed);
            if new_size >= layout.size() {
                let grow = new_size - layout.size();
                ALLOC_BYTES.fetch_add(grow, Relaxed);
                let live = LIVE_BYTES.fetch_add(grow, Relaxed) + grow;
                PEAK_BYTES.fetch_max(live, Relaxed);
            } else {
                LIVE_BYTES.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Zero the cumulative counters (allocs/bytes) and seed `peak` at the current
/// live bytes. Call immediately before a measured pass.
fn reset() {
    ALLOC_COUNT.store(0, Relaxed);
    ALLOC_BYTES.store(0, Relaxed);
    PEAK_BYTES.store(LIVE_BYTES.load(Relaxed), Relaxed);
}

/// `(allocs, total_bytes, peak_live_bytes)` since the last `reset()`.
fn snapshot() -> (usize, usize, usize) {
    (
        ALLOC_COUNT.load(Relaxed),
        ALLOC_BYTES.load(Relaxed),
        PEAK_BYTES.load(Relaxed),
    )
}

/// One benchmarked row: throughput (best-of-6) plus a separate fixed-N
/// allocation pass. Timing and alloc-counting are intentionally *different*
/// loops — timing wants many noisy iterations, alloc accounting wants a small
/// deterministic count divided out cleanly.
struct Measure {
    ops_per_s: f64,
    allocs_per_op: f64,
    bytes_per_op: f64,
}

/// best-of-N ops/s for `f` (warmup + timed windows). `f` returns an observable
/// sink so the optimizer can't elide the work.
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

/// Allocations + bytes attributable to one call of `f`, averaged over `n`
/// iterations. Warms up once (lazy statics, caches) so steady-state allocation
/// — not first-touch — is what we divide out.
fn alloc_per_op<F: FnMut() -> u64>(mut f: F, n: usize) -> (f64, f64) {
    let mut sink = 0u64;
    sink = sink.wrapping_add(f()); // warmup
    reset();
    for _ in 0..n {
        sink = sink.wrapping_add(f());
    }
    let (allocs, bytes, _peak) = snapshot();
    std::hint::black_box(sink);
    (allocs as f64 / n as f64, bytes as f64 / n as f64)
}

/// Time `f` (best-of-6) and, in a separate fixed-N pass, count its allocations.
fn measure<F: FnMut() -> u64>(mut f: F, ms: u128, alloc_n: usize) -> Measure {
    let ops_per_s = bench(&mut f, ms);
    let (allocs_per_op, bytes_per_op) = alloc_per_op(&mut f, alloc_n);
    Measure {
        ops_per_s,
        allocs_per_op,
        bytes_per_op,
    }
}

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
        let _ = write!(
            s,
            "<div class=\"card sx-{}\" data-testid=\"card-{}\" id=\"c{}\">\
<h2 class=title>Title {}</h2><p class=body>Body text for card {}.</p>\
<button class=btn type=button>Action</button></div>",
            i % 7, i, i, i, i
        );
    }
    s.push_str("</main></body></html>");
    s
}

/// Multi-dimension numbers for the cross-DOM comparison (bench/compare-all.mjs reads
/// the `RTDOM_JSON` line). Run:
///   cargo test --release --lib `rtdom::bench::compare_all` -- --ignored --nocapture
#[test]
#[ignore = "perf bench — run explicitly with --ignored"]
fn compare_all() {
    use super::tree::Tree;
    let html = fixture(300);

    // D1 parse: unique HTML each iter (rtdom has no parse cache; keep it honest anyway)
    let mut i = 0u64;
    let parse = bench(|| { i += 1; u64::from(Tree::parse(&format!("{html}<!--{i}-->")).node_count()) }, 400);

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
#[ignore = "perf bench — run explicitly with --ignored"]
fn hotspot_report() {
    let n = 300;
    let html = fixture(n);
    let base = Tree::parse(&html);
    let cards = base.query_selector_all("div.card");
    let mut rows: Vec<(String, Measure)> = Vec::new();

    // 1. parse (cold tree build per call)
    rows.push(("parse(300 cards)".into(), measure(|| u64::from(Tree::parse(&html).node_count()), 300, 200)));

    // 2. querySelectorAll cached (repeated query, unchanged tree = cache hit — RTL pattern)
    rows.push(("qsa div.card (cached)".into(), measure(|| base.query_selector_all("div.card").len() as u64, 300, 5000)));

    // 3. querySelectorAll cold (fresh tree each time — full matcher walk, no cache benefit)
    rows.push(("qsa div.card (cold tree)".into(), measure(|| {
        let t = Tree::parse(&html);
        t.query_selector_all(".card .title").len() as u64
    }, 300, 200)));

    // 4. getElementById (uncached full-tree walk)
    rows.push(("getElementById".into(), measure(|| base.get_element_by_id("c250").map_or(0, |h| u64::from(h.0)), 300, 5000)));

    // 5. getAttribute over all cards (per-node buffer scan)
    rows.push(("getAttribute x3/card".into(), measure(|| {
        let mut s = 0u64;
        for &c in cards.iter() {
            s += base.get_attribute(c, "class").map_or(0, |v| v.len() as u64);
            s += base.get_attribute(c, "data-testid").map_or(0, |v| v.len() as u64);
            s += base.get_attribute(c, "id").map_or(0, |v| v.len() as u64);
        }
        s
    }, 300, 2000)));

    // 6. computed_style per card (cascade — rebuilds the rule index each call?)
    rows.push(("getComputedStyle/card".into(), measure(|| {
        let mut s = 0u64;
        for &c in cards.iter() {
            s += cascade::computed_style(&base, c).len() as u64;
        }
        s
    }, 400, 200)));

    // 7. serialize the whole grid
    let root = base.root();
    rows.push(("serialize_inner(root)".into(), measure(|| serialize::serialize_inner(&base, root).len() as u64, 300, 1000)));

    // 8. dispatch listener-less (React fires thousands)
    {
        let mut dom = Dom::parse(&html);
        let btn = dom.tree.query_selector("button").unwrap();
        rows.push(("dispatch listener-less".into(), measure(|| {
            let mut e = Event::new("click", true, true);
            u64::from(dom.dispatch_event(btn, &mut e))
        }, 300, 5000)));
    }

    // 9. mutation: append + setAttribute + remove (no observer)
    rows.push(("mutate append+attr+remove".into(), measure(|| {
        let mut t = Tree::parse("<div id=root></div>");
        let root = t.query_selector("#root").unwrap();
        let el = t.create_element("span");
        t.set_attribute(el, "class", "x");
        t.append_child(root, el);
        t.remove_child(root, el);
        t.version
    }, 300, 2000)));

    rows.sort_by(|a, b| a.1.ops_per_s.partial_cmp(&b.1.ops_per_s).unwrap());
    print_report(&format!("rtdom hotspot report ({n} cards, best-of-6 ops/s, slowest first)"), &rows);
}

/// Shared table printer: name | ops/s | ns/op | allocs/op | bytes/op.
fn print_report(title: &str, rows: &[(String, Measure)]) {
    println!("\n=== {title} ===");
    println!(
        "  {:<30} {:>12} {:>12} {:>11} {:>12}",
        "name", "ops/s", "ns/op", "allocs/op", "bytes/op"
    );
    for (name, m) in rows {
        let ns = if m.ops_per_s > 0.0 { 1e9 / m.ops_per_s } else { 0.0 };
        println!(
            "  {:<30} {:>12.0} {:>12.0} {:>11.1} {:>12.0}",
            name, m.ops_per_s, ns, m.allocs_per_op, m.bytes_per_op
        );
    }
    println!();
}

/// A deep + wide tree for the crawler/extractor audience. Each level emits a
/// *row* of `width` sibling `<section>` wrappers (width), and exactly ONE of
/// them recurses one level deeper (depth grows linearly, not exponentially) —
/// so the tree is ~`depth * width` sections, each holding a row of `width`
/// cards, giving real descendant chains for `.a .b .c` and `:has(.x)`.
/// `depth=6, width=8` is a few thousand element nodes; tune via the args.
/// Classes are deliberately layered `wrap` > `row` > `card` >
/// {`title`,`body`,`btn`}, with sparse `flag`/`hot` markers so `:has(.flag)`
/// matches some-but-not-all subtrees.
fn big_fixture(depth: usize, width: usize) -> String {
    fn emit(s: &mut String, level: usize, depth: usize, width: usize, counter: &mut usize) {
        if level >= depth {
            return;
        }
        for w in 0..width {
            *counter += 1;
            let id = *counter;
            let hot = if id.is_multiple_of(5) { " hot" } else { "" };
            let _ = write!(
                s,
                "<section class=\"wrap lvl-{level} w-{w}{hot}\" data-depth=\"{level}\" id=\"s{id}\">"
            );
            // a row of cards at this level
            for _c in 0..width {
                *counter += 1;
                let cid = *counter;
                let flag = if cid.is_multiple_of(9) {
                    "<span class=flag>!</span>"
                } else {
                    ""
                };
                let _ = write!(
                    s,
                    "<div class=\"row\"><article class=\"card sx-{}\" data-testid=\"card-{cid}\" id=\"c{cid}\">\
<h3 class=title>Title {cid}</h3>\
<p class=body>Body text for card {cid} at level {level}.</p>\
<button class=btn type=button>Action</button>{flag}\
</article></div>",
                    cid % 7
                );
            }
            // recurse for depth in EXACTLY ONE sibling so node count stays
            // ~depth*width (a single sibling carries the deep chain).
            if w == 0 {
                emit(s, level + 1, depth, width, counter);
            }
            s.push_str("</section>");
        }
    }

    let mut s = String::from(
        "<!doctype html><html><head><style>\
.card{color:blue;padding:8px;margin:4px}\
.card .title{font-weight:bold}\
.wrap .row .card{border:1px solid}\
.flag{color:red}\
</style></head><body><main id=app class=grid>",
    );
    let mut counter = 0usize;
    emit(&mut s, 0, depth, width, &mut counter);
    s.push_str("</main></body></html>");
    s
}

/// Large-tree workload: the paths the perf work targets (mutation churn,
/// UNCACHED matching/querying with a varied selector set, and a `:has`-heavy
/// pass) on a multi-thousand-node fixture. The 300-card hotspot fixture is
/// parse-dominated and hides everything downstream; this is where allocs bite.
#[test]
#[ignore = "perf bench — run explicitly with --ignored"]
fn large_tree_report() {
    let depth = 6;
    let width = 8;
    let html = big_fixture(depth, width);
    let base = Tree::parse(&html);
    let nodes = base.node_count();
    let elements = base.query_selector_all("*");
    let cards = base.query_selector_all("article.card");
    let mut rows: Vec<(String, Measure)> = Vec::new();

    // A varied selector set. `matches()` is genuinely uncached, but
    // `query_selector_all` is version-cached *by selector string*: on an
    // unchanged tree, cycling a fixed handful would hit the cache after one
    // lap. To stay honest we append a unique no-match comma arm (`, #__uN`) per
    // iteration, forcing a distinct cache key and a full document walk every
    // call while leaving the match set identical — the crawler's worst case.
    let varied: &[&str] = &[
        ".card",                  // simple class
        ".wrap .card",            // descendant
        ".row > .card",           // child
        ".wrap .row .card",       // .a .b .c
        "[data-testid^=card]",    // attribute op (prefix)
        ".row:nth-child(1)",      // structural pseudo
        ".card:has(.flag)",       // relational
    ];

    // 1. UNCACHED query_selector_all: unique selector string each call (no-match
    //    comma arm) so the version cache always misses and a full walk runs.
    {
        let mut k = 0usize;
        rows.push(("qsa varied (UNCACHED)".into(), measure(|| {
            let sel = format!("{}, #__u{k}", varied[k % varied.len()]);
            k += 1;
            base.query_selector_all(&sel).len() as u64
        }, 400, 300)));
    }

    // 2. UNCACHED matches(h, sel) across every element, cycling selectors.
    {
        let mut k = 0usize;
        rows.push(("matches varied/elem (UNCACHED)".into(), measure(|| {
            let sel = varied[k % varied.len()];
            k += 1;
            let mut hits = 0u64;
            for &el in elements.iter() {
                if base.matches(el, sel) {
                    hits += 1;
                }
            }
            hits
        }, 400, 30)));
    }

    // 3. :has-heavy querySelectorAll — exercises the descendant search inside
    //    the relational matcher. Varied :has() arms, each call uncached.
    {
        let has_sels: &[&str] = &[
            ".wrap:has(.flag)",
            ".wrap:has(.card .title)",
            ".row:has(> .card)",
            "section:has(.hot)",
            ".card:has(.btn)",
        ];
        let mut k = 0usize;
        rows.push((":has qsa (UNCACHED)".into(), measure(|| {
            let sel = format!("{}, #__h{k}", has_sels[k % has_sels.len()]);
            k += 1;
            base.query_selector_all(&sel).len() as u64
        }, 400, 100)));
    }

    // 4. mutation churn: build a subtree of many nodes, append, then tear it
    //    down — exercises the create/children/parent overlays end to end.
    rows.push((
        "mutation churn (200 nodes)".into(),
        measure(churn_workload, 300, 500),
    ));

    rows.sort_by(|a, b| a.1.ops_per_s.partial_cmp(&b.1.ops_per_s).unwrap());
    print_report(
        &format!(
            "rtdom large-tree report (depth={depth} width={width} = {nodes} nodes, {} elements, {} cards; slowest first)",
            elements.len(),
            cards.len()
        ),
        &rows,
    );
}

/// The mutation-churn workload: build a 100-element subtree (each `li` with two
/// attributes + a text child), tear half down, then 50 append/remove spans —
/// exercising the create/children/parent/attr overlays end to end. Returns
/// `t.version` as an observable sink. Shared by `large_tree_report` (timing) and
/// `churn_alloc_gate` (the CI regression gate) so both measure the same path.
fn churn_workload() -> u64 {
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

/// CI regression gate on the mutation-churn allocation count. allocs/op is
/// code-path-determined — independent of CPU load AND of opt-level (debug and
/// release both measure 363.0 here) — which makes it a stable CI signal where a
/// wall-clock gate would flake on a shared runner. It locks the SSO / lazy-
/// `MutationRecord` / hybrid-overlay / `SmallVec` wins: a change that re-introduces
/// a per-node allocation on the create/append/remove path moves this by hundreds.
///
/// `#[ignore]`d so it never runs inside the *parallel* test suite — `CountingAlloc`
/// is a process-global `#[global_allocator]`, so a concurrently-running test would
/// inflate the count between `reset()` and `snapshot()`. The CI perf-gate step runs
/// it ALONE (`cargo test churn_alloc_gate -- --ignored`), where the bracket is honest.
#[test]
#[ignore = "perf gate — CI runs it in isolation via the alloc-gate step"]
fn churn_alloc_gate() {
    // Measured 363.0 allocs/op, 81510 bytes/op (darwin-arm64; debug == release).
    // The small cushion absorbs platform/toolchain drift; a real regression dwarfs it.
    const MAX_ALLOCS_PER_OP: f64 = 380.0;
    const MAX_BYTES_PER_OP: f64 = 85_000.0;
    let (allocs, bytes) = alloc_per_op(churn_workload, 500);
    println!(
        "churn alloc gate: {allocs:.1} allocs/op, {bytes:.0} bytes/op \
         (ceilings {MAX_ALLOCS_PER_OP} / {MAX_BYTES_PER_OP})"
    );
    assert!(
        allocs <= MAX_ALLOCS_PER_OP,
        "churn allocs/op regressed: {allocs:.1} > {MAX_ALLOCS_PER_OP} — a per-node \
         allocation likely crept back onto the create/append/remove path"
    );
    assert!(
        bytes <= MAX_BYTES_PER_OP,
        "churn bytes/op regressed: {bytes:.0} > {MAX_BYTES_PER_OP}"
    );
}

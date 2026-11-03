//! turbo-dom#112: DOM ownership bookkeeping must stay linear in the work a script does.
//! Every mutating native call used to end in a full `reconcile_all` (a walk of every live
//! wrapper plus the collected-listener/document sweeps), so building n nodes cost O(n^2).
//! These tests count that work (`dom_ownership::WORK`), never wall-clock time, and pin the
//! GC contract the amortised path must keep: a wrapper that only the tree references keeps its
//! identity and expandos while attached, and becomes collectable once detached.
use super::*;

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, source).unwrap();
    v8::Script::compile(scope, code, None)
        .and_then(|script| script.run(scope))
        .unwrap_or_else(|| panic!("{}", scope.exception().unwrap().to_rust_string_lossy(scope)))
        .to_rust_string_lossy(scope)
}

/// `__gc()`: a full GC from inside a running script, while ownership work may be pending.
fn gc(scope: &mut v8::PinScope, _: v8::FunctionCallbackArguments, _: v8::ReturnValue) {
    scope.low_memory_notification();
}

/// What a host does when control returns to its event loop: settle pending ownership work.
fn host_turn(scope: &mut v8::PinScope) {
    flush_ownership(scope);
}

fn reset_work() {
    dom_ownership::WORK.with(|work| {
        work.wrappers_visited.set(0);
        work.sweeps.set(0);
        work.placements.set(0);
    });
}
fn wrappers_visited() -> usize {
    dom_ownership::WORK.with(|work| work.wrappers_visited.get())
}
fn placements() -> usize {
    dom_ownership::WORK.with(|work| work.placements.get())
}
fn sweeps() -> usize {
    dom_ownership::WORK.with(|work| work.sweeps.get())
}

/// Build `rows` rows of 3 nodes (8 mutating calls per row), remove every other row, and
/// return how many wrappers the ownership reconciles walked meanwhile.
fn visits_building(rows: usize) -> usize {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    let visited;
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        reset_work();
        let left = eval(
            scope,
            &format!(
                "const host = document.createElement('div');
                 document.body.appendChild(host);
                 for (let i = 0; i < {rows}; i++) {{
                   const row = document.createElement('div');
                   row.setAttribute('data-i', String(i));
                   const a = document.createElement('span');
                   a.textContent = 'a' + i;
                   const b = document.createElement('span');
                   b.className = 'b';
                   row.append(a, b);
                   host.appendChild(row);
                 }}
                 for (let i = {rows} - 2; i >= 0; i -= 2) host.children[i].remove();
                 host.children.length + ':' + host.querySelectorAll('span').length"
            ),
        );
        assert_eq!(left, format!("{}:{}", rows / 2, rows));
        host_turn(scope);
        visited = wrappers_visited();
        reset();
    }
    clear_interface_records(isolate);
    visited
}

#[test]
fn building_a_large_tree_walks_linearly_many_wrappers() {
    let small = visits_building(1000);
    let large = visits_building(2000);
    // 2000 rows: 6000 nodes, ~17 000 mutating calls. Linear bookkeeping doubles with the
    // tree (a constant number of wrapper visits per call); the per-call full walk quadruples
    // (~50 million visits at 2000 rows).
    assert!(
        large * 10 < small * 25,
        "doubling the tree must not quadruple the ownership work: {small} -> {large}"
    );
    assert!(
        large < 250 * 6000,
        "ownership work must be O(nodes): {large} wrapper visits for 6000 nodes"
    );
}

fn with_page(f: impl FnOnce(&mut v8::PinScope)) {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install(scope);
        let global = context.global(scope);
        bind_method(scope, global, "__gc", gc);
        f(scope);
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn attached_wrappers_keep_expandos_across_gc_while_ownership_work_is_pending() {
    with_page(|scope| {
        // Nothing but the tree references the 1500 paragraphs once each loop iteration ends;
        // the GCs run mid-script, before any host turn.
        let kept = eval(
            scope,
            "const host = document.createElement('div');
             host.id = 'host';
             document.body.appendChild(host);
             for (let i = 0; i < 1500; i++) {
               const p = document.createElement('p');
               p.marker = i;
               host.appendChild(p);
               if (i % 500 === 499) __gc();
             }
             __gc();
             let kept = 0;
             for (let i = 0; i < 1500; i++) if (host.children[i].marker === i) kept++;
             kept",
        );
        assert_eq!(kept, "1500");
        host_turn(scope);
        scope.low_memory_notification();
        let kept = eval(
            scope,
            "const again = document.getElementById('host');
             let n = 0;
             for (let i = 0; i < 1500; i++) if (again.children[i].marker === i) n++;
             n",
        );
        assert_eq!(kept, "1500");
    });
}

#[test]
fn a_detached_subtree_is_collectable_after_the_host_turn() {
    with_page(|scope| {
        eval(
            scope,
            // An IIFE: a top-level `const gone` would be a global binding that keeps it alive.
            "(() => {
               const host = document.createElement('div');
               document.body.appendChild(host);
               for (let i = 0; i < 1500; i++) host.appendChild(document.createElement('p'));
               const gone = document.createElement('section');
               gone.id = 'gone';
               gone.appendChild(document.createElement('i'));
               host.appendChild(gone);
               return 'built';
             })()",
        );
        let weak = {
            v8::scope!(let inner, scope);
            let code = v8::String::new(inner, "document.getElementById('gone')").unwrap();
            let gone = v8::Script::compile(inner, code, None)
                .unwrap()
                .run(inner)
                .unwrap();
            let gone = v8::Local::<v8::Object>::try_from(gone).unwrap();
            v8::Weak::new(inner, gone)
        };
        host_turn(scope);
        scope.low_memory_notification();
        assert!(
            !weak.is_empty(),
            "an attached wrapper is owned by its document"
        );
        eval(scope, "document.getElementById('gone').remove(); 'removed'");
        host_turn(scope);
        for _ in 0..3 {
            scope.low_memory_notification();
        }
        assert!(
            weak.is_empty(),
            "a detached, unreferenced wrapper is collectable"
        );
        assert_eq!(
            eval(scope, "String(document.getElementById('gone'))"),
            "null"
        );
    });
}

#[test]
fn collected_listener_sweeps_wait_for_a_gc() {
    with_page(|scope| {
        reset_work();
        let epoch = || dom_ownership::GC_EPOCH.with(std::cell::Cell::get);
        let start = epoch();
        eval(
            scope,
            "const host = document.createElement('div');
             document.body.appendChild(host);
             for (let i = 0; i < 1000; i++) host.appendChild(document.createElement('p'));
             'built'",
        );
        // ~2000 mutating calls; weak listener/document handles only empty during a GC, so a
        // sweep per call is wasted work. A young-generation GC may still trigger one: at most
        // one sweep per GC. (An isolate fresh from a startup snapshot has not grown its young
        // generation yet, so it scavenges more often than one an engine bootstrap warmed.)
        let quiet = sweeps();
        let gcs = (epoch() - start) as usize;
        assert!(quiet <= gcs + 1, "{quiet} sweeps for {gcs} GCs");
        assert!(quiet <= 10.max(gcs + 1), "{quiet} sweeps without a GC");
        eval(
            scope,
            "__gc(); host.appendChild(document.createElement('p')); 'swept'",
        );
        host_turn(scope);
        assert!(sweeps() > quiet, "the first reconcile after a GC sweeps");
    });
}

/// A reconcile re-places only the wrappers whose ownership changed: small mutations in a large
/// attached tree (React re-rendering one row) must not re-place every live wrapper each time.
#[test]
fn a_reconcile_places_only_the_wrappers_whose_ownership_changed() {
    with_page(|scope| {
        eval(
            scope,
            "globalThis.rows = [];
             const host = document.createElement('div');
             document.body.appendChild(host);
             for (let i = 0; i < 3000; i++) rows.push(host.appendChild(document.createElement('p')));
             'built'",
        );
        host_turn(scope);
        reset_work();
        for i in 0..50 {
            eval(scope, &format!("rows[{i}].setAttribute('data-x', '{i}'); rows[{i}].appendChild(document.createElement('b')); 'ok'"));
            host_turn(scope);
        }
        let placed = placements();
        assert!(placed <= 50 * 8, "{placed} placements for 50 small mutations over 3000 wrappers");
    });
}

/// Moves between detached subtrees and the document keep the ownership contract: a held node
/// keeps its whole detached subtree (ancestors and siblings, with their expandos) alive, an
/// attached node is owned by its document, and a subtree nothing holds any more is
/// collectable, whichever components its nodes passed through.
#[test]
fn moves_between_subtrees_keep_held_components_whole_and_free_the_rest() {
    with_page(|scope| {
        eval(
            scope,
            "(() => {
               const make = (id) => { const root = document.createElement('section'); root.id = id; root.marker = id;
                 for (let i = 0; i < 20; i++) { const c = document.createElement('i'); c.marker = id + i; root.appendChild(c); } return root; };
               const a = make('a'), b = make('b'), c = make('c');
               document.body.appendChild(a);
               // b, c detached; a node of each moves around
               b.appendChild(a.children[0]);
               a.appendChild(c.children[0]);
               document.body.appendChild(b);
               b.remove();
               globalThis.heldLeaf = b.children[3];
               globalThis.cWeak = c;
               return 'moved';
             })()",
        );
        host_turn(scope);
        let weaks = {
            v8::scope!(let inner, scope);
            let mut out = Vec::new();
            for src in ["globalThis.cWeak", "document.getElementById('a').lastChild"] {
                let code = v8::String::new(inner, src).unwrap();
                let value = v8::Script::compile(inner, code, None).unwrap().run(inner).unwrap();
                out.push(v8::Weak::new(inner, v8::Local::<v8::Object>::try_from(value).unwrap()));
            }
            out
        };
        eval(scope, "delete globalThis.cWeak; 'dropped'");
        host_turn(scope);
        for _ in 0..3 {
            scope.low_memory_notification();
        }
        assert!(weaks[0].is_empty(), "the detached subtree c, held by nothing, is collectable");
        assert!(!weaks[1].is_empty(), "c's old child, now attached under a, is owned by the document");
        assert_eq!(
            eval(scope, "[heldLeaf.parentNode.marker, heldLeaf.parentNode.lastChild.marker, heldLeaf.parentNode.children.length, document.getElementById('a').lastChild.marker].join()"),
            "b,a0,21,c0",
            "the held leaf keeps its detached subtree whole, expandos included"
        );
        eval(scope, "heldLeaf.remove(); delete globalThis.heldLeaf; 'released'");
        host_turn(scope);
        for _ in 0..3 {
            scope.low_memory_notification();
        }
        assert_eq!(eval(scope, "String(document.getElementById('b'))"), "null");
    });
}

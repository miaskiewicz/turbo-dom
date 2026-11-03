//! Deterministic perf gates for the turbo-dom 0.7.0 browser-env perf items (operation
//! counters, never wall-clock time), each next to the correctness it must keep.
//!
//! - item 4: `selector_argument` parses each selector string once and skips the node-dependent
//!   rewrites for a selector without a `:`; rejected selectors keep Chrome's exact SyntaxError.
//! - item 5: wrapper owner tags are small integers (Smi), not BigInts; stale wrappers stay
//!   refused (`stale_nodes_tests`).
//! - item 6: `mutation_observers::flush` builds no record objects when nothing consumes them.
use super::*;
use crate::browser_env::crate_surface::harness_tests::thrown;

fn run(scope: &mut v8::PinScope, code: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let source = v8::String::new(scope, code).unwrap();
    let value = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
    let value = value.unwrap_or_else(|| panic!("threw {}", thrown(scope.exception(), scope)));
    value.to_rust_string_lossy(scope)
}

fn selector_work() -> (usize, usize) {
    SELECTOR_WORK.with(|work| (work.grammar_parses.get(), work.rewrites.get()))
}

#[test]
fn a_repeated_selector_is_parsed_once_and_plain_selectors_skip_the_rewrites() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<body><form><input class=field name=a><input class=field name=b required></form>",
        );
        let (parses, rewrites) = selector_work();
        let found = run(
            scope,
            "let n = 0; for (let i = 0; i < 200; i++) { n += document.querySelectorAll('form input.field').length; \
             n += document.querySelector('input[name=b]') ? 1 : 0; } String(n)",
        );
        assert_eq!(found, "600");
        let (after_parses, after_rewrites) = selector_work();
        assert_eq!(
            after_parses - parses,
            2,
            "one grammar parse per distinct selector"
        );
        assert_eq!(
            after_rewrites - rewrites,
            0,
            "no rewrite pass without a ':'"
        );
        // A pseudo-class still reaches the node-dependent rewrites, on every call.
        let found = run(
            scope,
            "let m = 0; for (let i = 0; i < 10; i++) m += document.querySelectorAll('input:required').length; String(m)",
        );
        assert_eq!(found, "10");
        let (final_parses, final_rewrites) = selector_work();
        assert_eq!(final_parses - after_parses, 1);
        assert_eq!(final_rewrites - after_rewrites, 10);
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn memoized_invalid_selectors_still_throw_chromes_exact_syntax_error() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=a></div>");
        // Each selector twice, so the second call answers from the memo; the message names
        // the operation and the receiver's interface, which the memo must not fix.
        let errors = run(
            scope,
            r"
              const out = [];
              const div = document.getElementById('a');
              const frag = document.createDocumentFragment();
              for (let i = 0; i < 2; i++) for (const [target, op, sel] of [
                [document, 'querySelector', 'a..b'], [div, 'matches', 'a..b'], [frag, 'querySelectorAll', 'a..b'],
                [div, 'closest', ''], [document, 'querySelectorAll', ''], [div, 'querySelector', ':nope'],
              ]) { try { target[op](sel); out.push('no throw'); } catch (e) { out.push(e.name + ': ' + e.message); } }
              out.join('\n')
            ",
        );
        let expected = [
            "SyntaxError: Failed to execute 'querySelector' on 'Document': 'a..b' is not a valid selector.",
            "SyntaxError: Failed to execute 'matches' on 'Element': 'a..b' is not a valid selector.",
            "SyntaxError: Failed to execute 'querySelectorAll' on 'DocumentFragment': 'a..b' is not a valid selector.",
            "SyntaxError: Failed to execute 'closest' on 'Element': The provided selector is empty.",
            "SyntaxError: Failed to execute 'querySelectorAll' on 'Document': The provided selector is empty.",
            "SyntaxError: Failed to execute 'querySelector' on 'Element': ':nope' is not a valid selector.",
        ]
        .join("\n");
        assert_eq!(errors, format!("{expected}\n{expected}"));
        reset();
    }
    clear_interface_records(isolate);
}

// The memo holds only short selectors (round-1 review #14): a selector over the size bound is
// parsed on every call, so a page minting huge selectors cannot pin megabytes in the memo, and
// its results and Chrome's exact SyntaxError are unchanged.
#[test]
fn oversized_selectors_are_not_memoized_and_keep_their_results_and_errors() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=a class=x></div>");
        let (parses, _) = selector_work();
        // 2000 units: `div.x` followed by many `, div.x` alternatives (valid), and the same
        // with a trailing `..` (invalid).
        let found = run(
            scope,
            "globalThis.big = 'div.x' + ', div.x'.repeat(285); globalThis.bad = big + '..'; \
             let n = 0; for (let i = 0; i < 3; i++) n += document.querySelectorAll(big).length; \
             String([big.length, n])",
        );
        assert_eq!(found, "2000,3");
        let (after, _) = selector_work();
        assert_eq!(
            after - parses,
            3,
            "an oversized selector is parsed per call"
        );
        let big = format!("div.x{}", ", div.x".repeat(285));
        assert!(!selector_memo_contains(
            &big.encode_utf16().collect::<Vec<_>>()
        ));
        let errors = run(
            scope,
            "const out = []; for (let i = 0; i < 2; i++) try { document.getElementById('a').matches(bad) } \
             catch (e) { out.push(e.message === `Failed to execute 'matches' on 'Element': '${bad}' is not a valid selector.` && e.name) } \
             String(out)",
        );
        assert_eq!(errors, "SyntaxError,SyntaxError");
        assert_eq!(selector_work().0 - after, 2);
        // A short selector is still memoized.
        run(
            scope,
            "document.querySelector('div.x'); document.querySelector('div.x')",
        );
        assert_eq!(selector_work().0 - after, 3);
        assert!(selector_memo_contains(
            &"div.x".encode_utf16().collect::<Vec<_>>()
        ));
        reset();
    }
    clear_interface_records(isolate);
}

// The memo is per page: a new install on the same thread starts it empty (round-1 review #14).
#[test]
fn the_selector_memo_is_cleared_by_each_install() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><p class=memo-a></p>");
        let units: Vec<u16> = "p.memo-a".encode_utf16().collect();
        assert_eq!(
            run(
                scope,
                "String(document.querySelectorAll('p.memo-a').length)"
            ),
            "1"
        );
        assert!(selector_memo_contains(&units));
        install_with_html(scope, "<body><p class=memo-a></p><p class=memo-a></p>");
        assert!(!selector_memo_contains(&units), "install clears the memo");
        let (parses, _) = selector_work();
        assert_eq!(
            run(
                scope,
                "String(document.querySelectorAll('p.memo-a').length)"
            ),
            "2"
        );
        assert_eq!(selector_work().0 - parses, 1);
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn wrapper_owner_tags_are_small_integers_that_still_name_their_live_tree() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><p>x</p>");
        let id = DOM_ID.with(std::cell::Cell::get);
        let source = v8::String::new(scope, "document.querySelector('p')").unwrap();
        let p = v8::Script::compile(scope, source, None)
            .unwrap()
            .run(scope)
            .unwrap();
        let p = v8::Local::<v8::Object>::try_from(p).unwrap();
        let tag = p.get_internal_field(scope, 1).unwrap();
        let tag = v8::Local::<v8::Value>::try_from(tag).unwrap();
        assert!(tag.is_int32() && !tag.is_big_int(), "owner tag is a Smi");
        assert_eq!(tagged_owner(scope, p), Some(Some(id)));
        assert!(handle_of(scope, p).is_some());
        // The next tree under the same id refuses the old wrapper.
        reset();
        install_with_html(scope, "<body><p>y</p>");
        assert_eq!(tagged_owner(scope, p), Some(None));
        assert!(handle_of(scope, p).is_none());
        reset();
    }
    clear_interface_records(isolate);
}

#[test]
fn flush_builds_no_records_when_nothing_observes() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        // A parser-inserted script keeps the document recording (script preparation), as
        // on every React page.
        install_with_html(
            scope,
            "<head><script>1</script></head><body><div id=root></div>",
        );
        assert!(mutation_observers::recording());
        let before = mutation_observers::payloads_built();
        let out = run(
            scope,
            "const root = document.getElementById('root'); for (let i = 0; i < 100; i++) { \
             const d = document.createElement('div'); d.setAttribute('class', 'c' + i); root.appendChild(d); } \
             String(root.children.length)",
        );
        assert_eq!(out, "100");
        assert_eq!(mutation_observers::payloads_built() - before, 0);
        // An observer still receives every record.
        let out = run(
            scope,
            "globalThis.records = []; new MutationObserver(r => records.push(...r)).observe(root, {childList: true}); \
             root.appendChild(document.createElement('span')); root.firstChild.setAttribute('id', 'x'); 'ok'",
        );
        assert_eq!(out, "ok");
        // A microtask checkpoint delivers them to the callback.
        scope.perform_microtask_checkpoint();
        assert_eq!(run(scope, "records.map(r => r.type).join()"), "childList");
        assert_eq!(mutation_observers::payloads_built() - before, 2);
        reset();
    }
    clear_interface_records(isolate);
}

/// A [CEReactions] frame allocates its reaction queue (a V8 array behind a weak global
/// handle) only when a reaction can be queued: with no custom element definition, the DOM
/// calls a React page makes create none (they were a V8 global-handle create and release per
/// call). With a definition, reactions are still queued and delivered in order.
#[test]
fn reaction_frames_allocate_no_queue_without_custom_element_definitions() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=root></div>");
        let before = custom_elements::reaction_queues_built();
        let out = run(
            scope,
            "const root = document.getElementById('root'); for (let i = 0; i < 100; i++) { \
             const d = document.createElement('div'); d.setAttribute('class', 'c' + i); root.appendChild(d); d.remove(); } \
             String(root.children.length)",
        );
        assert_eq!(out, "0");
        assert_eq!(custom_elements::reaction_queues_built() - before, 0);
        let out = run(
            scope,
            "globalThis.log = []; customElements.define('x-r', class extends HTMLElement { \
             connectedCallback() { log.push('c:' + this.id); } disconnectedCallback() { log.push('d:' + this.id); } \
             attributeChangedCallback(n, o, v) { log.push('a:' + v); } static get observedAttributes() { return ['data-v']; } }); \
             const x = document.createElement('x-r'); x.id = 'one'; root.appendChild(x); x.setAttribute('data-v', '1'); x.remove(); \
             root.innerHTML = '<x-r id=two></x-r>'; log.join()",
        );
        assert_eq!(out, "c:one,a:1,d:one,c:two");
        assert!(custom_elements::reaction_queues_built() - before > 0);
        reset();
    }
    clear_interface_records(isolate);
}

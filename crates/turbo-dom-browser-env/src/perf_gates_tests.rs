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

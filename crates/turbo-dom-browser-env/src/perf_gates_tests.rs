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

/// `labels` and a label's `control` read a per-tree-version index (each root's elements by id,
/// its label elements in tree order) instead of walking the whole tree on every call:
/// dom-testing-library reads `labels` of every element it matches by role or label text.
/// The index follows every change (ids, `for`, structure).
#[test]
fn label_lookups_walk_the_tree_once_per_version() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><form id=f></form>");
        run(
            scope,
            "const f = document.getElementById('f'); for (let i = 0; i < 200; i++) { \
             const l = document.createElement('label'); l.htmlFor = 'i' + i; l.textContent = 'L' + i; \
             const input = document.createElement('input'); input.id = 'i' + i; f.append(l, input); } 'ok'",
        );
        let before = control_labels::walk_steps();
        let out = run(
            scope,
            "let n = 0; for (const input of document.querySelectorAll('input')) n += input.labels.length; \
             for (const l of document.querySelectorAll('label')) n += l.control ? 1 : 0; String(n)",
        );
        assert_eq!(out, "400");
        let steps = control_labels::walk_steps() - before;
        assert!(
            steps <= 4 * 410,
            "{steps} walk steps for 400 lookups over 402 nodes"
        );
        let out = run(
            scope,
            "const a = document.getElementById('i0'), b = document.getElementById('i1'); \
             a.id = 'x'; const moved = [a.labels.length, document.querySelector('label').control === null]; \
             b.id = 'i0'; moved.push(b.labels.length, document.querySelector('label').control === b); \
             document.querySelector('label').htmlFor = 'x'; moved.push(a.labels.length, b.labels.length); \
             a.remove(); moved.push(a.labels.length, document.querySelector('label').control === null); JSON.stringify(moved)",
        );
        assert_eq!(out, "[0,true,1,true,1,0,0,true]");
        reset();
    }
    clear_interface_records(isolate);
}

/// A selector with a `:` none of the state-dependent transforms (popover, dialog, interest,
/// form, view transition) applies to skips them: whether each applies is a property of the
/// selector text, kept per selector, so `matches(':focus-visible')` or `closest('li:hover')`
/// on every pointer event parses it once, not five times per call. A transform that applies
/// still runs on every call (its answer reads live state).
#[test]
fn transforms_that_cannot_apply_are_skipped_per_selector() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<body><ul><li id=a>a</li><li id=b><input id=i disabled></li></ul>",
        );
        let before = SELECTOR_WORK.with(|work| work.transforms.get());
        let out = run(
            scope,
            "const a = document.getElementById('a'), i = document.getElementById('i'); let n = 0; \
             for (let k = 0; k < 100; k++) { n += a.matches('li:first-child') ? 1 : 0; n += a.matches(':focus-visible') ? 1 : 0; } String(n)",
        );
        assert_eq!(out, "100");
        let skipped = SELECTOR_WORK.with(|work| work.transforms.get()) - before;
        assert!(
            skipped <= 10,
            "{skipped} transform runs for 200 calls of two transform-free selectors"
        );
        let before = SELECTOR_WORK.with(|work| work.transforms.get());
        let out = run(
            scope,
            "let m = 0; for (let k = 0; k < 10; k++) { m += i.matches(':disabled') ? 1 : 0; i.disabled = k % 2 === 0; } String(m)",
        );
        assert_eq!(out, "6");
        assert!(
            SELECTOR_WORK.with(|work| work.transforms.get()) - before >= 10,
            "a transform that applies runs every call"
        );
        reset();
    }
    clear_interface_records(isolate);
}

/// A node getter decodes its holder's owner tag once: the tag routes the call to the
/// holder's DOM and proves its handle live (stale wrappers stay refused, turbo-dom#224).
#[test]
fn node_getters_decode_the_owner_tag_once_per_call() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=a><b>x</b></div>");
        run(scope, "globalThis.a = document.getElementById('a'); 'ok'");
        let before = TAG_READS.with(std::cell::Cell::get);
        let out = run(
            scope,
            "const body = document.body; let n = 0; for (let i = 0; i < 1000; i++) { n += a.parentNode === body ? 1 : 0; } String(n)",
        );
        assert_eq!(out, "1000");
        let reads = TAG_READS.with(std::cell::Cell::get) - before;
        assert!(
            reads <= 2100,
            "{reads} owner-tag decodes for 1000 parentNode reads"
        );
        reset();
    }
    clear_interface_records(isolate);
}

/// A projected DOM accessor (every public node getter and setter goes through
/// `dom_accessor_dispatch`) reads the native template accessor through a reused raw view of
/// its DOM, not a new template instance per call; reentrant calls take their own view. The
/// values, `this` checks and stale-wrapper refusal are unchanged.
#[test]
fn dom_accessor_dispatch_reuses_raw_views() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><div id=a><b>x</b><i>y</i></div>");
        run(scope, "globalThis.a = document.getElementById('a'); 'ok'");
        let before = RAW_VIEWS_BUILT.with(std::cell::Cell::get);
        let out = run(
            scope,
            "let n = 0; for (let i = 0; i < 1000; i++) { n += a.parentNode === document.body ? 1 : 0; \
             n += a.firstChild.nextSibling.nodeName === 'I' ? 1 : 0; a.firstChild.textContent = 'x' + (i % 3); } \
             String(n) + ':' + a.textContent",
        );
        assert_eq!(out, "2000:x0y");
        let built = RAW_VIEWS_BUILT.with(std::cell::Cell::get) - before;
        assert!(
            built <= 8,
            "{built} raw views built for ~16000 projected accessor calls"
        );
        let thrown = run(
            scope,
            "const get = Object.getOwnPropertyDescriptor(Node.prototype, 'parentNode').get; \
             try { get.call({}); 'no' } catch (e) { e.constructor.name + ':' + e.message }",
        );
        assert_eq!(thrown, "TypeError:Illegal invocation");
        reset();
    }
    clear_interface_records(isolate);
}

fn crossings() -> usize {
    NATIVE_CROSSINGS.with(std::cell::Cell::get)
}

/// 0.7.1 item 1: the engine-JS getters dom-testing-library reads hardest (`document.body`,
/// `children`, `labels`) cost about one native crossing per read, not a JS walk of projected
/// accessor round trips. Identity, liveness and brand checks are unchanged.
#[test]
fn hot_engine_getters_cost_one_native_crossing() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(
            scope,
            "<body><div id=d>t<b></b>u<i></i><!--c--><s></s></div><label for=x>L</label><input id=x>",
        );
        run(
            scope,
            "globalThis.d = document.getElementById('d'); globalThis.x = document.getElementById('x'); 'ok'",
        );
        let before = crossings();
        let out = run(
            scope,
            "const b = document.body; let n = 0; for (let i = 0; i < 1000; i++) n += document.body === b ? 1 : 0; String(n)",
        );
        assert_eq!(out, "1000");
        let used = crossings() - before;
        assert!(used <= 1100, "{used} crossings for 1000 document.body reads");

        // A control round (the same mutations, no `children`) prices the mutations alone.
        let before = crossings();
        run(
            scope,
            "for (let i = 0; i < 300; i++) d.append(document.createElement('p')); 'ok'",
        );
        let control = crossings() - before;
        run(scope, "d.querySelectorAll('p').forEach(p => p.remove()); 'ok'");
        let before = crossings();
        let out = run(
            scope,
            "let k = 0; for (let i = 0; i < 300; i++) { d.append(document.createElement('p')); \
             const c = d.children; k += c === d.children && c.length === 4 + i ? 1 : 0; } String(k)",
        );
        assert_eq!(out, "300");
        let used = crossings() - before - control;
        // two getter reads + one length read per round, the list re-read once per mutation.
        assert!(used <= 300 * 5, "{used} crossings for 300 mutate+children rounds over the control");
        let out = run(
            scope,
            "const cc = d.children; const q = [cc[0].localName, cc[2].localName, cc.length]; \
             d.firstElementChild.remove(); q.push(cc[0].localName, cc.length, [...cc].length); \
             d.querySelectorAll('p').forEach(p => p.remove()); q.push(cc.length); JSON.stringify(q)",
        );
        assert_eq!(out, r#"["b","s",303,"i",302,302,2]"#);

        let before = crossings();
        let out = run(
            scope,
            "const l = x.labels; let m = 0; for (let i = 0; i < 1000; i++) m += x.labels === l ? 1 : 0; String(m) + ':' + l.length",
        );
        assert_eq!(out, "1000:1");
        let used = crossings() - before;
        assert!(used <= 1100, "{used} crossings for 1000 labels reads");

        let out = run(
            scope,
            "\
             const tries = [() => Object.getOwnPropertyDescriptor(Document.prototype, 'body').get.call(d), \
             () => Object.getOwnPropertyDescriptor(Element.prototype, 'children').get.call(document), \
             () => Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'labels').get.call(d), \
             () => Object.getOwnPropertyDescriptor(Document.prototype, 'body').get.call({})]; \
             tries.map(f => { try { f(); return 'no'; } catch (e) { return e.constructor.name + ':' + e.message; } }).join('|')",
        );
        assert_eq!(
            out,
            "TypeError:Illegal invocation|TypeError:Illegal invocation|TypeError:Illegal invocation|TypeError:Illegal invocation"
        );
        let out = run(
            scope,
            "const h = document.implementation.createHTMLDocument('t'); const f = h.createElement('frameset'); \
             h.documentElement.replaceChild(f, h.body); const r = [h.body === f]; h.documentElement.remove(); r.push(h.body); JSON.stringify(r)",
        );
        assert_eq!(out, "[true,null]");
        reset();
    }
    clear_interface_records(isolate);
}

/// The `{__proto__: null, ...}` literals left in `source`'s hot function at `marker`: the body
/// from the first `{` after the marker to its matching `}` (`whole_line`: the marker's line).
fn null_proto_literals(source: &str, marker: &str, whole_line: bool) -> usize {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("hot path {marker:?} not found"));
    let rest = &source[start..];
    let body = if whole_line {
        rest.lines().next().unwrap_or("")
    } else {
        let open = rest.find('{').expect("function body");
        let mut depth = 0usize;
        let mut end = rest.len();
        for (index, ch) in rest[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + index + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        &rest[..end]
    };
    body.replace(' ', "").matches("__proto__:null").count()
}

/// 0.7.1 item 3: the per-call computed-style and collection paths build no
/// `{__proto__: null, ...}` literal per call (V8 makes each a slow dictionary-mode object
/// and the profile's CreateShallowObjectLiteral / dictionary lookups are their cost): static
/// tables are hoisted, records come from fixed-shape constructors, and row descriptors are
/// one reused null-prototype descriptor.
#[test]
fn hot_computed_style_and_collection_paths_build_no_null_proto_literals() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/");
    let read = |name: &str| std::fs::read_to_string(format!("{dir}{name}")).unwrap();
    let browser_env = read("browser_env.js");
    let document_basics = read("document_basics.js");
    let form_collections = read("form_collections.js");
    let typed_om = read("typed_om.js");
    let sites = [
        (&browser_env, "var add=function(list,value){", false),
        (&browser_env, "function computedFonts(", false),
        (&browser_env, "function groupContext(", false),
        (&browser_env, "function appliedGroupRules(", false),
        (&browser_env, "function appliedSheetEntries(", false),
        (&browser_env, "function appliedEntryMatch(", false),
        (&document_basics, "const collectionOwn=", true),
        (&form_collections, "const row = (array, item) =>", false),
        (&typed_om, "const push = (array, value) =>", true),
        (&typed_om, "const toArray = (array) =>", true),
    ];
    let left: Vec<String> = sites
        .iter()
        .filter_map(|(source, marker, line)| {
            let count = null_proto_literals(source, marker, *line);
            (count > 0).then(|| format!("{marker} x{count}"))
        })
        .collect();
    assert!(left.is_empty(), "per-call null-prototype literals left: {left:?}");
}

/// 0.7.1 item 2: a cached live collection (`childNodes`, `children`, `getElementsBy*`)
/// learns of mutations from an epoch the engine pushes on every tree write, so an indexed,
/// `length` or iterator read between mutations makes no native crossing (it used to ask
/// for the tree stamp on every read). Liveness and identity are unchanged.
#[test]
fn live_collection_reads_between_mutations_cross_nothing() {
    tests::init_v8();
    let isolate = &mut crate::browser_env::tests::new_isolate();
    {
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, "<body><ul id=u><li>a</li><li>b</li><li class=k>c</li></ul>");
        run(
            scope,
            "globalThis.u = document.getElementById('u'); globalThis.nodes = u.childNodes; \
             globalThis.kids = u.children; globalThis.ks = document.getElementsByClassName('k'); \
             nodes.length; kids.length; ks.length; 'ok'",
        );
        let before = crossings();
        let out = run(
            scope,
            "let n = 0; for (let i = 0; i < 1000; i++) { n += nodes.length + kids.length + ks.length; \
             n += nodes[i % 3] === kids[i % 3] ? 1 : 0; for (const k of kids) n += k === ks[0] ? 1 : 0; } String(n)",
        );
        assert_eq!(out, "9000");
        let used = crossings() - before;
        assert!(used <= 10, "{used} crossings for 1000 rounds of reads with no mutation");
        let out = run(
            scope,
            "const r = [nodes === u.childNodes, kids === u.children]; const li = document.createElement('li'); \
             u.append(li); r.push(nodes.length, kids[3] === li); li.className = 'k'; r.push(ks.length, ks[1] === li); \
             u.firstChild.remove(); r.push(nodes.length, kids.length, kids[0].textContent); \
             u.textContent = ''; r.push(nodes.length, kids.length, ks.length, nodes[0], [...kids].length); \
             const other = document.implementation.createHTMLDocument('o'); other.body.append(document.adoptNode(u)); \
             u.append(other.createElement('i')); r.push(nodes.length, kids[0].localName); JSON.stringify(r)",
        );
        assert_eq!(out, r#"[true,true,4,true,2,true,3,3,"b",0,0,0,null,0,1,"i"]"#);
        reset();
    }
    clear_interface_records(isolate);
}

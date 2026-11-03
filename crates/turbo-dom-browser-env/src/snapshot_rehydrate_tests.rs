//! turbo-dom#249 Phase C steps 3-4: a context deserialized from the startup snapshot,
//! rehydrated (context slots, DOM state, isolate state) and given a document behaves like a
//! normally installed one.
use super::*;
use std::borrow::Cow;

const PAGE: &str = "<!doctype html><title>t</title><style>#a{color:rgb(1, 2, 3)} .x{display:inline-block}</style><body><div id=a class=x>hi</div><form><input id=req required></form><template id=t><b>c</b></template>";

/// Sync DOM, events, custom elements, form validity, computed style; then the
/// MutationObserver records once microtasks ran.
const SMOKE: &str = r"
globalThis.__log = [];
const a = document.getElementById('a');
const p = document.createElement('p');
p.textContent = 'new';
a.appendChild(p);
__log.push(document.querySelectorAll('#a > p').length, a.innerHTML, document.title);
let seen = [];
a.addEventListener('click', e => seen.push('a:' + e.target.localName + ':' + e.eventPhase), true);
p.addEventListener('click', e => seen.push('p:' + e.eventPhase));
document.body.addEventListener('click', e => seen.push('body:' + e.eventPhase));
p.dispatchEvent(new MouseEvent('click', {bubbles: true}));
__log.push(seen.join());
class XEl extends HTMLElement {
  connectedCallback() { this.dataset.up = 'yes'; }
  get kind() { return 'x-el'; }
}
const early = document.createElement('x-el');
document.body.appendChild(early);
__log.push(early.kind === undefined, document.querySelectorAll(':defined').length > 0);
customElements.define('x-el', XEl);
__log.push(early.kind, early.dataset.up, early instanceof XEl, early.matches(':defined'));
const req = document.getElementById('req');
__log.push(req.matches(':invalid'), document.querySelector('form').checkValidity());
req.value = 'v';
__log.push(req.matches(':valid'), req.validity.valid);
const cs = getComputedStyle(a);
__log.push(cs.color, cs.display);
globalThis.__records = [];
new MutationObserver(rs => { for (const r of rs) __records.push(r.type + ':' + r.addedNodes.length + ':' + (r.attributeName || '')); })
  .observe(document.body, {childList: true, subtree: true, attributes: true});
a.setAttribute('title', 'z');
document.body.appendChild(document.createComment('c'));
__log.push(__records.length);
// Every engine function belongs to this realm (a snapshot that duplicated the context
// would hand out functions of the copy, whose Function.prototype differs).
let foreign = 0;
const isProto = Object.prototype.isPrototypeOf, F = Function.prototype;
const fn = f => { if (typeof f === 'function' && !Reflect.apply(isProto, F, [f])) foreign++; };
const walk = o => { for (const k of Reflect.ownKeys(o)) { let d; try { d = Object.getOwnPropertyDescriptor(o, k); } catch { continue; } if (!d) continue; fn(d.value); fn(d.get); fn(d.set); } };
walk(globalThis); walk(document); walk(a); walk(p);
for (const k of Reflect.ownKeys(globalThis)) { let v; try { v = globalThis[k]; } catch { continue; } if (typeof v === 'function' && v.prototype && typeof v.prototype === 'object') walk(v.prototype); }
__log.push(foreign);
const t = document.getElementById('t');
__log.push(t.content.firstChild.localName, t.content.ownerDocument !== document, t.content.ownerDocument.createElement('i').ownerDocument === t.content.ownerDocument);
const host = document.createElement('div');
host.innerHTML = '<span slot=s>slotted</span>';
document.body.appendChild(host);
const root = host.attachShadow({mode: 'open'});
root.innerHTML = '<slot name=s></slot>';
__log.push(root.querySelector('slot').assignedNodes()[0].textContent, host.shadowRoot === root);
const range = document.createRange();
range.selectNodeContents(a);
getSelection().addRange(range);
__log.push(String(getSelection()), range.toString(), a.classList.contains('x'));
JSON.stringify(__log)
";

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    v8::tc_scope!(let scope, scope);
    let code = v8::String::new(scope, source).unwrap();
    match v8::Script::compile(scope, code, None).and_then(|script| script.run(scope)) {
        Some(value) => value.to_rust_string_lossy(scope),
        None => {
            let exception = scope.exception().unwrap();
            let line = scope.message().map(|m| m.get_line_number(scope));
            panic!(
                "threw {} at {line:?}",
                exception.to_rust_string_lossy(scope)
            );
        }
    }
}

fn smoke(scope: &mut v8::PinScope) -> (String, String) {
    let log = eval(scope, SMOKE);
    scope.perform_microtask_checkpoint();
    (log, eval(scope, "JSON.stringify(__records)"))
}

#[test]
fn a_rehydrated_snapshot_context_behaves_like_an_installed_one() {
    crate::browser_env::tests::init_v8();
    let expected = {
        let isolate = &mut v8::Isolate::new(Default::default());
        v8::scope!(let scope, isolate);
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        install_with_html(scope, PAGE);
        let result = smoke(scope);
        reset();
        result
    };
    assert_eq!(
        expected.0,
        r#"[1,"hi<p>new</p>","t","a:p:1,p:2,body:3",true,true,"x-el","yes",true,true,true,false,true,true,"rgb(1, 2, 3)","inline-block",0,0,"b",true,true,"slotted",true,"hi\nnew\n\n","hinew",true]"#
    );
    assert_eq!(
        expected.1,
        r#"["attributes:0:title","childList:1:","childList:1:"]"#
    );

    let blob = startup_snapshot::build().expect("every callback is registered");
    let params = v8::CreateParams::default()
        .snapshot_blob(blob.data)
        .external_references(Cow::Borrowed(blob.references));
    let isolate = &mut v8::Isolate::new(params);
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    let before = ENGINE_SCRIPTS_RUN.with(std::cell::Cell::get);
    startup_snapshot::rehydrate(scope);
    attach_document(scope, Tree::parse_with_processing_instructions(PAGE));
    assert_eq!(
        ENGINE_SCRIPTS_RUN.with(std::cell::Cell::get),
        before,
        "rehydrate + attach ran an engine script"
    );
    assert_eq!(smoke(scope), expected);
    reset();
}

/// Rehydrating needs a context made from the snapshot: a bare context has no window
/// document to rehydrate, and gets no DOM.
#[test]
fn rehydrating_a_bare_context_installs_no_dom() {
    crate::browser_env::tests::init_v8();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    assert!(!startup_snapshot::rehydrate(scope));
    assert!(with_tree(|tree| tree.root()).is_none());
    reset();
}

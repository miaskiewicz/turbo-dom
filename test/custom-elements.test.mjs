// src/runtime/custom-elements.mjs: the custom element state behind `:defined` (in lockstep with
// crates/turbo-dom/tests/custom_element_state.rs), the registry's upgrades and constructions,
// and the lifecycle callbacks. The Chrome 156 control is test/custom-elements-chrome.test.mjs.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment, DOMParser, Document } from '../src/runtime/index.mjs';
import {
  customElementState, setCustomElementState, copyCustomElementState, isDefinedElement, isValue, isCustomName,
  makeCustomElements,
} from '../src/runtime/custom-elements.mjs';

const SHEET = '<style>:not(:defined) { order: 2 } :defined { order: 1 }</style>';
const HTML_NS = 'http://www.w3.org/1999/xhtml';
const env = (body) => createEnvironment(`<!doctype html><html><head>${SHEET}</head><body>${body}</body></html>`);
const defined = (el) => {
  const dom = el.matches(':defined');
  assert.equal(el.matches(':not(:defined)'), !dom);
  assert.equal(isDefinedElement(el), dom);
  return dom;
};
const errors = (window) => { const seen = []; window.reportError = (e) => seen.push(e.message); return seen; };

test('creation-time state without definitions (the Rust engine\'s rule)', () => {
  const { document } = env('<x-a id=a></x-a><button id=b is=x-b></button><button id=e is></button><div id=d></div><svg><x-a id=s></x-a></svg><font-face id=r></font-face><template id=t><x-a id=ta></x-a></template>');
  const el = (id) => document.getElementById(id);
  for (const [id, state] of [['a', 'undefined'], ['b', 'undefined'], ['e', 'undefined'], ['d', 'uncustomized'], ['s', 'uncustomized'], ['r', 'uncustomized']]) {
    assert.equal(customElementState(el(id)), state, id);
    assert.equal(defined(el(id)), state === 'uncustomized', id);
  }
  assert.equal(customElementState(el('t').content.firstElementChild), 'undefined');
  assert.equal(isValue(el('b')), 'x-b');
  // the `is` value is read once, at creation: later edits never change the state
  el('b').removeAttribute('is');
  assert.ok(!defined(el('b')));
  assert.equal(isValue(el('b')), 'x-b');
  el('d').setAttribute('is', 'x-d');
  assert.ok(defined(el('d')));
  const created = document.createElement('button');
  created.setAttribute('is', 'x-b');
  assert.equal(customElementState(created), 'uncustomized');
  // fragment parsing reads `is` at creation like the initial parse
  el('d').innerHTML = '<button is=x-b></button><p is=q></p><x-c></x-c><svg is=x></svg><i title=t></i>';
  assert.deepEqual([...el('d').childNodes].map(customElementState), ['undefined', 'undefined', 'undefined', 'uncustomized', 'uncustomized']);
  // non-elements and foreign elements are uncustomized
  assert.equal(customElementState(document.createTextNode('x')), 'uncustomized');
  assert.equal(customElementState(null), 'uncustomized');
  assert.equal(customElementState(document.createElementNS('http://www.w3.org/2000/svg', 'x-a')), 'uncustomized');
  assert.ok(isCustomName('x-a') && !isCustomName('font-face') && !isCustomName('X-a') && !isCustomName('xa'));
  // createElement(name, {is}) without a definition is undefined; a string option is ignored
  assert.equal(customElementState(document.createElement('button', { is: 'x-b' })), 'undefined');
  assert.equal(customElementState(document.createElement('button', 'x-b')), 'uncustomized');
  assert.equal(customElementState(document.createElement('button', null)), 'uncustomized');
  assert.equal(customElementState(document.createElementNS(HTML_NS, 'button', { is: 'x-b' })), 'undefined');
  assert.equal(document.createElement('button', { is: 'x-b' }).getAttribute('is'), null);
});

test('embedder reports override the state and invalidate caches', () => {
  const { window, document } = env('<x-a id=a></x-a><x-a id=f></x-a><button id=b is=x-b></button>');
  const [a, f, b] = ['a', 'f', 'b'].map((id) => document.getElementById(id));
  assert.deepEqual([...document.querySelectorAll('body :defined')], []);
  assert.equal(window.getComputedStyle(a).order, '2');
  const version = document.__version;
  assert.equal(setCustomElementState(a, 'precustomized'), true);
  assert.ok(!defined(a), 'the constructor is still running');
  assert.equal(setCustomElementState(a, 'custom'), true);
  assert.ok(document.__version > version, 'a state change bumps the version');
  assert.equal(window.getComputedStyle(a).order, '1', 'getComputedStyle sees it');
  assert.equal(setCustomElementState(f, 'failed'), true);
  assert.equal(setCustomElementState(b, 'custom'), true);
  assert.deepEqual([...document.querySelectorAll('body :defined')], [a, b]);
  assert.deepEqual([...document.querySelectorAll('body :not(:defined)')], [f]);
  const unchanged = document.__version;
  assert.equal(setCustomElementState(a, 'custom'), false);
  assert.equal(document.__version, unchanged);
  assert.equal(setCustomElementState(document.createTextNode('t'), 'custom'), false);
  assert.throws(() => setCustomElementState(a, 'bogus'), TypeError);
  // copies start undefined unless the source is uncustomized; the `is` value travels
  const copy = document.createElement('button');
  copyCustomElementState(b, copy);
  assert.deepEqual([customElementState(copy), isValue(copy)], ['undefined', 'x-b']);
  const plain = document.createElement('p');
  copyCustomElementState(document.body, plain);
  assert.equal(customElementState(plain), 'uncustomized');
  const detached = document.createElement('x-z');
  detached.ownerDocument = null;
  assert.equal(setCustomElementState(detached, 'custom'), true);
});

test('define() upgrades connected candidates in shadow-including tree order', async () => {
  const { window, document } = env('<x-a id=one></x-a><div id=host></div><x-a id=three></x-a>');
  const host = document.getElementById('host');
  const shadow = host.attachShadow({ mode: 'closed' });
  shadow.innerHTML = '<x-a id=two></x-a>';
  const detached = document.createElement('x-a');
  const order = [];
  const ready = window.customElements.whenDefined('x-a');
  class XA extends window.HTMLElement {
    constructor() { super(); order.push([this.id || 'new', this.matches(':defined'), this instanceof XA]); }
    hello() { return 'hi'; }
  }
  window.customElements.define('x-a', XA);
  assert.deepEqual(order, [['one', false, true], ['two', false, true], ['three', false, true]]);
  assert.equal(await ready, XA);
  assert.equal(await window.customElements.whenDefined('x-a'), XA);
  assert.equal(document.getElementById('one').hello(), 'hi');
  assert.ok(!defined(detached));
  window.customElements.upgrade(detached);
  assert.ok(defined(detached) && detached instanceof XA);
  window.customElements.upgrade(detached); // already custom: no second construction
  assert.equal(order.length, 4);
  window.customElements.upgrade(null);
  window.customElements.upgrade({});
  // the stub-era surface stays
  assert.equal(window.customElements.get('x-a'), XA);
  assert.equal(window.customElements.get('x-none'), undefined);
  assert.equal(window.customElements.getName(XA), 'x-a');
  assert.throws(() => window.customElements.define('x-a', class extends window.HTMLElement {}), /already defined/);
  assert.throws(() => window.customElements.define('x-q', 5), TypeError);
  // an element of another document is never upgraded by this registry
  const foreign = new DOMParser().parseFromString('<x-a></x-a>', 'text/html').body.firstChild;
  window.customElements.upgrade(foreign);
  assert.ok(!defined(foreign));
});

test('constructors: new C(), createElement, customized built-ins, misuse', () => {
  const { window, document } = env('<button id=b is=x-btn></button>');
  const seen = errors(window);
  class XA extends window.HTMLElement { constructor() { super(); this.made = this.matches(':defined'); } }
  window.customElements.define('x-a', XA);
  const constructed = new XA();
  assert.ok(constructed instanceof XA && constructed.made && defined(constructed));
  assert.deepEqual([constructed.localName, constructed.ownerDocument === document, constructed.parentNode], ['x-a', true, null]);
  const created = document.createElement('X-A');
  assert.ok(created instanceof XA && created.made);
  assert.ok(document.createElementNS(HTML_NS, 'x-a') instanceof XA);
  class XBtn extends window.HTMLButtonElement { constructor() { super(); this.kind = 'btn'; } }
  window.customElements.define('x-btn', XBtn, { extends: 'button' });
  assert.equal(document.getElementById('b').kind, 'btn');
  const button = document.createElement('button', { is: 'x-btn' });
  assert.ok(button instanceof XBtn && defined(button) && button.kind === 'btn');
  assert.equal(document.createElement('x-btn').kind, undefined, 'the customized definition is not autonomous');
  const direct = new XBtn();
  assert.deepEqual([direct.localName, isValue(direct), defined(direct)], ['button', 'x-btn', true]);
  // the tag interfaces stay illegal constructors without a definition
  assert.throws(() => new window.HTMLButtonElement(), /Illegal constructor/);
  class Loose extends window.HTMLButtonElement {}
  assert.throws(() => new Loose(), /Illegal constructor/);
  assert.throws(() => window.HTMLButtonElement(), /Illegal constructor/);
  // a constructor that throws leaves a failed element (reported, not thrown)
  window.customElements.define('x-fail', class extends window.HTMLElement { constructor() { super(); throw new Error('nope'); } });
  const failed = document.createElement('x-fail');
  assert.deepEqual([failed.localName, customElementState(failed)], ['x-fail', 'failed']);
  // a constructor whose result is not a fresh element of the name fails the same way
  window.customElements.define('x-attr', class extends window.HTMLElement { constructor() { super(); this.setAttribute('a', '1'); } });
  window.customElements.define('x-kid', class extends window.HTMLElement { constructor() { super(); this.append('t'); } });
  window.customElements.define('x-other', class extends window.HTMLElement { constructor() { super(); return document.createElement('p'); } });
  window.customElements.define('x-plain', class { });
  for (const name of ['x-attr', 'x-kid', 'x-other', 'x-plain']) assert.equal(customElementState(document.createElement(name)), 'failed', name);
  // an upgrade whose constructor returns another object, or constructs twice, fails
  const { window: w2, document: d2 } = env('<x-other id=o></x-other><x-twice id=t></x-twice><x-throw id=x></x-throw>');
  const seen2 = errors(w2);
  w2.customElements.define('x-other', class extends w2.HTMLElement { constructor() { super(); return d2.createElement('p'); } });
  class Twice extends w2.HTMLElement { constructor() { super(); new Twice(); } }
  w2.customElements.define('x-twice', Twice);
  w2.customElements.define('x-throw', class extends w2.HTMLElement { constructor() { throw new RangeError('early'); } });
  assert.deepEqual(['o', 't', 'x'].map((id) => customElementState(d2.getElementById(id))), ['failed', 'failed', 'failed']);
  assert.deepEqual(seen2, [
    'Custom element constructors must call super() first and must not return a different object',
    "Failed to construct 'HTMLElement': This instance is already constructed",
    'early',
  ]);
  assert.equal(seen.length, 5);
  // a registry is per document generation: after reset() an old constructor is not defined
  const fresh = createEnvironment('<p></p>');
  class Old extends fresh.window.HTMLElement {}
  fresh.window.customElements.define('x-old', Old);
  fresh.reset();
  assert.notEqual(new Old().localName, 'x-old');
  assert.equal(fresh.window.customElements.get('x-old'), undefined);
  // reportError missing: errors are dropped silently
  const { window: w3, document: d3 } = env('');
  w3.reportError = undefined;
  w3.customElements.define('x-fail', class extends w3.HTMLElement { constructor() { super(); throw new Error('quiet'); } });
  assert.equal(customElementState(d3.createElement('x-fail')), 'failed');
  // a registry without a document (and createElement with a registry but no definitions)
  const bare = makeCustomElements(null);
  bare.define('x-bare', class {});
  assert.equal(bare.get('x-bare') !== undefined, true);
  const { window: w4, document: d4 } = env('');
  assert.equal(w4.customElements.get('x-a'), undefined);
  assert.equal(customElementState(d4.createElement('x-a')), 'undefined');
});

test('insertion, removal, innerHTML, clones and adoption', () => {
  const { window, document } = env('<div id=root></div><button id=pb is=x-btn></button>');
  const root = document.getElementById('root');
  const log = [];
  class XA extends window.HTMLElement {
    static get observedAttributes() { return ['title']; }
    constructor() { super(); log.push(['construct', this.id]); }
    connectedCallback() { log.push(['connected', this.id]); }
    disconnectedCallback() { log.push(['disconnected', this.id]); }
    adoptedCallback(from, to) { log.push(['adopted', this.id, from !== to]); }
    attributeChangedCallback(name, old, value, ns) { log.push(['attr', this.id, name, old, value, ns]); }
  }
  const early = document.createElement('x-a');
  early.id = 'early';
  early.setAttribute('title', 'pre');
  window.customElements.define('x-a', XA);
  assert.deepEqual(log, []);
  root.append(early); // insertion upgrades: constructor, observed attributes, connected
  assert.deepEqual(log.splice(0), [['construct', 'early'], ['attr', 'early', 'title', null, 'pre', null], ['connected', 'early']]);
  early.setAttribute('title', 'post');
  early.setAttribute('lang', 'en'); // not observed
  early.removeAttribute('title');
  assert.deepEqual(log.splice(0), [['attr', 'early', 'title', 'pre', 'post', null], ['attr', 'early', 'title', 'post', null, null]]);
  root.removeChild(early);
  assert.deepEqual(log.splice(0), [['disconnected', 'early']]);
  const held = document.createElement('div');
  held.append(early); // into a disconnected tree: nothing
  root.append(held);
  root.removeChild(held);
  assert.deepEqual(log.splice(0), [['connected', 'early'], ['disconnected', 'early']]);
  // innerHTML upgrades (connected or not); replacing children disconnects them
  root.innerHTML = '<x-a id=p1></x-a><p><x-a id=p2></x-a></p>';
  assert.deepEqual(log.splice(0), [['construct', 'p1'], ['connected', 'p1'], ['construct', 'p2'], ['connected', 'p2']]);
  root.textContent = '';
  assert.deepEqual(log.splice(0), [['disconnected', 'p1'], ['disconnected', 'p2']]);
  root.innerHTML = '';
  const off = document.createElement('div');
  off.innerHTML = '<x-a id=off></x-a>';
  assert.deepEqual(log.splice(0), [['construct', 'off']]);
  root.replaceChildren(off);
  root.replaceChildren();
  assert.deepEqual(log.splice(0), [['connected', 'off'], ['disconnected', 'off']]);
  // shadow roots: innerHTML and insertion through a connected host
  const shadow = root.attachShadow({ mode: 'open' });
  shadow.innerHTML = '<x-a id=s1></x-a>';
  assert.deepEqual(log.splice(0), [['construct', 's1'], ['connected', 's1']]);
  shadow.innerHTML = '';
  assert.deepEqual(log.splice(0), [['disconnected', 's1']]);
  // clones: upgraded copies (deep too), the parser's `is` survives
  const clone = early.cloneNode(true);
  assert.ok(clone instanceof XA && defined(clone));
  const tree = document.createElement('div');
  tree.innerHTML = '<span><x-a id=deep></x-a></span>';
  log.splice(0);
  const deep = tree.cloneNode(true).querySelector('x-a');
  assert.ok(deep instanceof XA);
  assert.ok(document.importNode(early, false) instanceof XA);
  log.splice(0);
  class XBtn extends window.HTMLButtonElement {}
  const pbClone = document.getElementById('pb').cloneNode();
  assert.deepEqual([customElementState(pbClone), isValue(pbClone)], ['undefined', 'x-btn']);
  window.customElements.define('x-btn', XBtn, { extends: 'button' });
  assert.ok(document.getElementById('pb') instanceof XBtn);
  assert.ok(document.getElementById('pb').cloneNode() instanceof XBtn);
  assert.equal(customElementState(document.createElement('p').cloneNode()), 'uncustomized');
  // adoption keeps the parser's `is` value; custom elements get adoptedCallback
  const other = new DOMParser().parseFromString('<button id=ob is=x-btn></button><x-a id=oa></x-a>', 'text/html');
  const ob = document.adoptNode(other.getElementById('ob'));
  assert.deepEqual([customElementState(ob), isValue(ob)], ['undefined', 'x-btn']);
  root.append(ob);
  assert.ok(ob instanceof XBtn);
  const back = other.adoptNode(early);
  assert.equal(back, early);
  assert.deepEqual(log.splice(0), [['adopted', 'early', true]]);
  document.adoptNode(document.createElement('p')); // same document: nothing
  other.adoptNode(other.createElement('p'));
  const plain = document.createElement('p');
  root.append(plain);
  plain.setAttribute('title', 'x'); // not a custom element: no callback
  assert.deepEqual(log, []);
});

test('callbacks that throw are reported; definitions without callbacks are quiet', () => {
  const { window, document } = env('<div id=root></div>');
  const seen = errors(window);
  class Loud extends window.HTMLElement {
    static observedAttributes = ['a'];
    connectedCallback() { throw new Error('connected'); }
    attributeChangedCallback() { throw new Error('attr'); }
  }
  window.customElements.define('x-loud', Loud);
  const loud = document.createElement('x-loud');
  document.getElementById('root').append(loud);
  loud.setAttribute('a', '1');
  assert.deepEqual(seen, ['connected', 'attr']);
  class NoList extends window.HTMLElement { attributeChangedCallback() { throw new Error('never'); } }
  window.customElements.define('x-nolist', NoList);
  const quiet = document.createElement('x-nolist');
  quiet.setAttribute('a', '1');
  window.customElements.define('x-bare', class extends window.HTMLElement {});
  const bare = document.createElement('x-bare');
  document.getElementById('root').append(bare);
  bare.setAttribute('a', '1');
  bare.remove();
  document.adoptNode(bare);
  new DOMParser().parseFromString('', 'text/html').adoptNode(bare);
  assert.deepEqual(seen, ['connected', 'attr']);
});

test('edge paths: nested upgrades, registries without definitions, fragments, raw documents', () => {
  const { window, document } = env('<x-n id=first></x-n><x-n id=second></x-n><div id=root><p id=kid></p></div>');
  const order = [];
  class XN extends window.HTMLElement {
    constructor() {
      super();
      order.push(this.id);
      // upgrading a later candidate from a constructor: define()'s own pass skips it
      if (this.id === 'first') window.customElements.upgrade(document.getElementById('second'));
    }
  }
  window.customElements.define('x-n', XN);
  assert.deepEqual(order, ['first', 'second']);
  // a registry with no definitions leaves every mutation path alone
  const { window: w2, document: d2 } = env('<div id=root><p id=kid></p><x-z></x-z></div>');
  assert.equal(w2.customElements.get('x-z'), undefined);
  const root = d2.getElementById('root');
  root.removeChild(d2.getElementById('kid'));
  root.innerHTML = '<x-z></x-z>';
  root.innerHTML = '';
  root.append(d2.createElement('x-z'));
  d2.body.textContent = '';
  // fragments: ParentNode element traversal
  const f = d2.createDocumentFragment();
  assert.deepEqual([f.firstElementChild, f.lastElementChild, f.childElementCount], [null, null, 0]);
  f.append('t', d2.createElement('a'), d2.createElement('b'), 'u');
  assert.deepEqual([f.firstElementChild.localName, f.lastElementChild.localName, f.childElementCount], ['a', 'b', 2]);
  // a document that never loaded a buffer (no version yet) still records a state
  const raw = new Document();
  const el = raw.createElement('x-a');
  assert.equal(raw.__version, undefined);
  assert.equal(setCustomElementState(el, 'custom'), true);
  assert.ok(raw.__version > 0); // a fresh value of the process-wide version counter
});

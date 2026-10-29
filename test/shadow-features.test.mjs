// Shadow DOM advanced features: <slot> projection, relatedTarget retargeting,
// :host/::slotted + boundary inheritance in the cascade, and declarative
// <template shadowrootmode> promotion. Asserted to WHATWG spec.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const fresh = (html) => createEnvironment(html ?? '<!doctype html><html><head></head><body><div id="host"></div></body></html>');

// ---------------------------------------------- relatedTarget retargeting ----
test('relatedTarget is retargeted across the shadow boundary', () => {
  const { document, window } = fresh();
  const host = document.getElementById('host');
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<button id="a">a</button><button id="b">b</button>';
  const a = root.querySelector('#a'), b = root.querySelector('#b');

  const seen = {};
  document.addEventListener('focusin', (e) => { seen.docTarget = e.target; seen.docRelated = e.relatedTarget; });
  a.addEventListener('focusin', (e) => { seen.inTarget = e.target; seen.inRelated = e.relatedTarget; });

  a.dispatchEvent(new window.FocusEvent('focusin', { bubbles: true, composed: true, relatedTarget: b }));

  // inside the shadow tree: real nodes
  assert.equal(seen.inTarget, a);
  assert.equal(seen.inRelated, b);
  // at the document: both retargeted to the host
  assert.equal(seen.docTarget, host);
  assert.equal(seen.docRelated, host);
});

test('null relatedTarget dispatches cleanly (no retarget work)', () => {
  const { document, window } = fresh();
  const root = document.getElementById('host').attachShadow({ mode: 'open' });
  root.innerHTML = '<button id="a">a</button>';
  const a = root.querySelector('#a');
  let related = 'unset';
  document.addEventListener('focusout', (e) => { related = e.relatedTarget; });
  a.dispatchEvent(new window.FocusEvent('focusout', { bubbles: true, composed: true }));
  assert.equal(related, null);
});

// ------------------------------------------------------- <slot> projection ----
test('assignedNodes / assignedElements route light children by slot name', () => {
  const { document } = fresh();
  const host = document.getElementById('host');
  host.innerHTML = '<span slot="head">H</span><p>body1</p><b slot="head">H2</b><p>body2</p>';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<slot name="head"></slot><slot></slot>';
  const [named, dflt] = root.querySelectorAll('slot');

  assert.equal(named.assignedElements().length, 2);                  // span + b (slot="head")
  assert.deepEqual(dflt.assignedElements().map((e) => e.localName), ['p', 'p']);
  assert.ok(named.assignedNodes().every((n) => n.getAttribute('slot') === 'head'));
});

test('default slot collects unnamed text + elements; text never goes to a named slot', () => {
  const { document } = fresh();
  const host = document.getElementById('host');
  host.innerHTML = 'loose text<span slot="x">named</span><i>i</i>';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<slot name="x"></slot><slot></slot>';
  const [named, dflt] = root.querySelectorAll('slot');
  // default slot: the text node + <i>; named slot: just the span
  assert.equal(dflt.assignedNodes().some((n) => n.nodeType === 3), true);
  assert.equal(dflt.assignedElements().map((e) => e.localName).join(), 'i');
  assert.equal(named.assignedElements().length, 1);
  assert.equal(named.assignedNodes().some((n) => n.nodeType === 3), false);
});

test('assignedNodes({flatten}) falls back to the slot default content when empty', () => {
  const { document } = fresh();
  const host = document.getElementById('host'); // no light children
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<slot>fallback <em>content</em></slot>';
  const slot = root.querySelector('slot');
  assert.equal(slot.assignedNodes().length, 0);                  // nothing assigned
  const flat = slot.assignedNodes({ flatten: true });
  assert.ok(flat.length >= 1);                                   // default content
  assert.equal(slot.assignedElements({ flatten: true }).map((e) => e.localName).join(), 'em');
});

test('assignedSlot links a light child to its slot; null when unmatched / not a host', () => {
  const { document } = fresh();
  const host = document.getElementById('host');
  host.innerHTML = '<span slot="x">a</span><b>b</b>';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<slot name="x"></slot><slot></slot>';
  const span = host.querySelector('span'), bold = host.querySelector('b');
  assert.equal(span.assignedSlot.getAttribute('name'), 'x');
  assert.equal(bold.assignedSlot.getAttribute('name'), null); // default slot
  // a child whose slot name has no matching <slot> → null
  host.innerHTML = '<span slot="nope">a</span>';
  assert.equal(host.querySelector('span').assignedSlot, null);
  // an element not parented by a shadow host → null
  assert.equal(document.createElement('div').assignedSlot, null);
});

test('assignedNodes on a non-slot / slot outside a shadow tree is empty', () => {
  const { document } = fresh();
  assert.deepEqual(document.createElement('div').assignedNodes(), []);
  // a detached <slot> (not inside a shadow root)
  const slot = document.createElement('slot');
  document.body.appendChild(slot);
  assert.deepEqual(slot.assignedNodes(), []);
});

// ------------------------------------------------- :host / ::slotted / inherit ----
test(':host and :host(sel) rules apply to the host element only', () => {
  const { window, document } = fresh();
  const host = document.getElementById('host');
  host.className = 'active';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>:host{color:red;padding:8px} :host(.active){font-weight:bold} :host(.nope){color:lime}</style><span id="s">x</span>';
  const gcs = window.getComputedStyle;
  assert.equal(gcs(host).color, 'rgb(255, 0, 0)');         // bare :host
  assert.equal(gcs(host).fontWeight, 'bold');   // :host(.active) matches
  assert.notEqual(gcs(host).color, 'lime');     // :host(.nope) does not match
  // :host does not directly MATCH shadow children: a non-inherited property
  // (padding) set on :host must not appear on the child…
  assert.equal(gcs(root.querySelector('#s')).padding, '');
  // …though an INHERITED property (color) does cross the boundary by inheritance.
  assert.equal(gcs(root.querySelector('#s')).color, 'rgb(255, 0, 0)');
});

test('::slotted(sel) styles slotted light nodes', () => {
  const { window, document } = fresh();
  const host = document.getElementById('host');
  host.innerHTML = '<span class="hl" id="lit">x</span>';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>::slotted(.hl){color:green} ::slotted(b){color:red}</style><slot></slot>';
  const lit = host.querySelector('#lit');
  assert.equal(window.getComputedStyle(lit).color, 'rgb(0, 128, 0)'); // matches .hl
});

// Chrome 156 (getComputedStyle): the slot must match the selector's originating compound
// and its combinators, and a sub-pseudo-element (::slotted(S)::before, #195) styles
// generated content, never the slotted node.
test('::slotted() rules match the slot against the rest of the selector', () => {
  const { window, document } = fresh();
  const host = document.getElementById('host');
  host.innerHTML = '<span id="a">a</span><i id="b" slot="s">b</i><b id="c" slot="t">c</b>';
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>::slotted(span)::before{color:red} .nope::slotted(i){color:red} '
    + 'slot[name=t]::slotted(b){color:green} div ::slotted(span){color:blue}</style>'
    + '<slot></slot><slot name="s"></slot><div><slot name="t"></slot></div>';
  const color = (id) => window.getComputedStyle(host.querySelector(`#${id}`)).color;
  assert.equal(color('a'), ''); // neither ::slotted(span)::before nor `div ::slotted(span)`
  assert.equal(color('b'), ''); // the slot has no class "nope"
  assert.equal(color('c'), 'rgb(0, 128, 0)');
});

test('inheritable props cross the boundary INTO shadow content; light DOM stays honest', () => {
  const { window, document } = fresh();
  const host = document.getElementById('host');
  const root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>:host{color:purple} .inner{font-weight:bold}</style><div class="inner"><span id="deep">x</span></div>';
  const deep = root.querySelector('#deep');
  // color inherits host → .inner → span (none set it explicitly)
  assert.equal(window.getComputedStyle(deep).color, 'rgb(128, 0, 128)');
  // a sibling light-DOM element inherits nothing here — no ancestor sets color
  // (the shadow :host rule is encapsulated) → honest empty
  document.body.insertAdjacentHTML('beforeend', '<section><span id="l">y</span></section>');
  assert.equal(window.getComputedStyle(document.getElementById('l')).color, '');
});

test('normal shadow rules still resolve with :host present', () => {
  const { window, document } = fresh();
  const root = document.getElementById('host').attachShadow({ mode: 'open' });
  root.innerHTML = '<style>:host{color:red} .x{color:blue}</style><i class="x" id="i">x</i>';
  assert.equal(window.getComputedStyle(root.querySelector('#i')).color, 'rgb(0, 0, 255)');
});

// ------------------------------------------ declarative <template shadowrootmode> ----
test('declarative open shadow root is promoted from <template shadowrootmode>', () => {
  const { document } = fresh(
    '<!doctype html><body><div id="h"><template shadowrootmode="open"><p>shadow</p></template><span>light</span></div></body>'
  );
  const h = document.getElementById('h');
  assert.ok(h.shadowRoot);
  assert.equal(h.shadowRoot.querySelector('p').textContent, 'shadow');
  assert.equal(h.querySelector('template'), null);     // template consumed
  assert.equal(h.querySelector('span').textContent, 'light'); // light child kept
  assert.equal(document.__hasShadow, true);
});

test('declarative closed shadow root + delegatesFocus', () => {
  const { document } = fresh(
    '<!doctype html><body><div id="h"><template shadowrootmode="closed" shadowrootdelegatesfocus><b>x</b></template></div></body>'
  );
  const h = document.getElementById('h');
  assert.equal(h.shadowRoot, null);          // closed → not exposed
  assert.equal(h.__shadow.mode, 'closed');
  assert.equal(h.__shadow.delegatesFocus, true);
  assert.equal(h.__shadow.querySelector('b').textContent, 'x');
});

test('a plain <template> (no shadowrootmode) is left untouched', () => {
  const { document } = fresh(
    '<!doctype html><body><div id="h"><template><p>t</p></template></div></body>'
  );
  const h = document.getElementById('h');
  assert.equal(h.shadowRoot, null);
  assert.ok(h.querySelector('template'));
  assert.equal(h.querySelector('template').content.querySelector('p').textContent, 't');
});

test('declarative shadow promotion runs on reset() too', () => {
  const env = fresh();
  env.reset('<!doctype html><body><div id="h"><template shadowrootmode="open"><i>r</i></template></div></body>');
  const h = env.document.getElementById('h');
  assert.ok(h.shadowRoot);
  assert.equal(h.shadowRoot.querySelector('i').textContent, 'r');
});

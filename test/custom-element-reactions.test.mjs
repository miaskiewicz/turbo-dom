// Custom element reactions ([CEReactions], miaskiewicz/turbo-dom#29 item 4; the Chrome 156
// order is gated by test/dom-adoption-reactions-chrome.test.mjs): every DOM call that can
// queue reactions runs them when it returns, element by element; callbacks run with no scope
// open, so the DOM calls they make run their own reactions first.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

function setup() {
  const { window, document } = createEnvironment('<!doctype html><html><body><div id="host"></div></body></html>');
  const log = [];
  const host = document.getElementById('host');
  window.customElements.define('x-l', class extends window.HTMLElement {
    connectedCallback() { log.push(`conn ${this.id} ${this.parentNode.childNodes.length}`); }
    disconnectedCallback() { log.push(`disc ${this.id} ${host.childNodes.length}`); }
  });
  const make = (id) => { const el = document.createElement('x-l'); el.id = id; return el; };
  return { window, document, host, log, make };
}

test('ChildNode / ParentNode calls run their reactions once the whole call is done', () => {
  const { document, host, log, make } = setup();
  host.prepend(make('a'), make('b'));
  assert.deepEqual(log.splice(0), ['conn a 2', 'conn b 2']);
  const a = document.getElementById('a');
  a.before(make('c'), 'text');
  assert.deepEqual(log.splice(0), ['conn c 4']);
  a.after(make('d'), make('e'));
  assert.deepEqual(log.splice(0), ['conn d 6', 'conn e 6']);
  const t = host.childNodes[1];                   // the text node
  t.before(make('f'));
  t.after(make('g'));
  t.replaceWith(make('h'));
  assert.deepEqual(log.splice(0), ['conn f 7', 'conn g 8', 'conn h 8']);
  // a detached node's ChildNode calls do nothing
  make('z').before(make('y'));
  make('z').after(make('y'));
  make('z').replaceWith(make('y'));
  assert.deepEqual(log, []);
  // fragments: append queues nothing until the fragment is inserted
  const frag = document.createDocumentFragment();
  frag.append(make('i'), make('j'));
  assert.deepEqual(log, []);
  host.append(frag);
  assert.deepEqual(log.splice(0), ['conn i 10', 'conn j 10']);
});

test('Range calls and insertAdjacentHTML into a fragment are one scope each', () => {
  const { document, host, log, make } = setup();
  host.append(make('a'), make('b'), make('c'));
  log.length = 0;
  const range = document.createRange();
  range.setStart(host, 0);
  range.setEnd(host, 2);
  const extracted = range.extractContents();
  assert.deepEqual(log.splice(0), ['disc a 1', 'disc b 1']);
  host.append(extracted);
  log.length = 0;
  range.setStart(host, 1);
  range.setEnd(host, 3);
  range.deleteContents();
  assert.deepEqual(log.splice(0), ['disc a 1', 'disc b 1']);
  range.setStart(host, 0);
  range.setEnd(host, 1);
  const wrap = document.createElement('section');
  range.surroundContents(wrap);
  assert.deepEqual(log.splice(0), ['disc c 1', 'conn c 1']);
  // the context of a fragment parent is <body>
  const frag = document.createDocumentFragment();
  const child = document.createElement('p');
  frag.append(child);
  child.insertAdjacentHTML('afterend', '<x-l id="f"></x-l><td>cell</td>');
  assert.deepEqual(Array.from(frag.childNodes, (n) => n.nodeName), ['P', 'X-L', '#text']);
});

test('a reaction queued with no scope open runs right away; errors are reported', () => {
  const { window, document } = createEnvironment('<!doctype html><html><body></body></html>');
  const log = [];
  const reported = [];
  window.reportError = (e) => reported.push(e.message);
  window.customElements.define('x-a', class extends window.HTMLElement {
    static get observedAttributes() { return ['v']; }
    attributeChangedCallback(name, oldValue, value) {
      log.push(`${name} ${oldValue} ${value}`);
      if (value === 'bad') throw new Error('bad value');
    }
  });
  const el = document.createElement('x-a');
  const mo = new window.MutationObserver(() => {});
  mo.observe(el, { attributes: true });
  el.setAttribute('v', '1');
  assert.deepEqual(log.splice(0), ['v null 1']);
  assert.equal(mo.takeRecords().length, 1); // the record was queued before the callback ran
  el.setAttribute('v', 'bad');
  assert.deepEqual(reported, ['bad value']);
  el.toggleAttribute('v');
  assert.deepEqual(log, ['v 1 bad', 'v bad null']);
});

test('define() and upgrade() queue the upgrades and run them when they return', () => {
  const { window, document } = createEnvironment('<!doctype html><html><body><x-u id="one"></x-u><x-u id="two"></x-u></body></html>');
  const seen = [];
  window.customElements.define('x-u', class extends window.HTMLElement {
    constructor() {
      super();
      // the second candidate is still undefined while the first one constructs
      seen.push([this.id, document.getElementById('two').matches(':defined')]);
    }
  });
  assert.deepEqual(seen, [['one', false], ['two', false]]);
  const detached = document.createElement('div');
  detached.innerHTML = '<x-v></x-v>';
  window.customElements.define('x-v', class extends window.HTMLElement {});
  assert.equal(detached.firstChild.matches(':defined'), false);
  window.customElements.upgrade(detached);
  assert.equal(detached.firstChild.matches(':defined'), true);
});

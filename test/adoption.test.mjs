// Insertion across documents (miaskiewicz/turbo-dom#29 item 2; the Chrome 156 behaviour is
// gated by test/dom-adoption-reactions-chrome.test.mjs): the node is adopted — its buffer
// link severed, the old document's version bumped — and custom elements keep their
// callbacks in a document without a registry.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment, DOMParser } from '../src/runtime/index.mjs';

const parse = (html) => new DOMParser().parseFromString(html, 'text/html');

test('an adopted subtree reads its own attributes and children, not the new buffer', () => {
  const { document } = createEnvironment('<!doctype html><html><body><main id="m" class="x"><i>a</i></main></body></html>');
  const other = parse('<section id="s" data-k="v"><p id="p"><b>deep</b> text</p></section>');
  const section = other.getElementById('s');
  assert.equal(section.__kids, null); // children not inflated yet
  document.getElementById('m').appendChild(section);
  assert.equal(section.ownerDocument, document);
  assert.equal(section.outerHTML, '<section id="s" data-k="v"><p id="p"><b>deep</b> text</p></section>');
  assert.equal(document.getElementById('p').ownerDocument, document);
  assert.equal(other.getElementById('s'), null);
  assert.equal(other.body.childNodes.length, 0);
});

test('versions are process-wide: an adopted node cannot hit a stale cache entry', () => {
  const { document } = createEnvironment('<!doctype html><html><body></body></html>');
  const other = parse('<div id="d"><p></p><p></p></div>');
  const div = other.getElementById('d');
  assert.equal(div.querySelectorAll('p').length, 2); // cached at other's version
  const cachedAt = other.__version;
  // with per-document counters the two bumps below (insert + remove) would land on
  // `cachedAt` and return the stale two-element list
  document.__version = cachedAt - 2;
  document.body.append(div);
  div.lastChild.remove();
  assert.notEqual(document.__version, cachedAt);
  assert.equal(div.querySelectorAll('p').length, 1);
  assert.ok(other.__version > cachedAt, 'the old document version moved on');
});

test('adoptNode within the same document only removes; fragments adopt their children', () => {
  const { document } = createEnvironment('<!doctype html><html><body><p id="p"></p></body></html>');
  const p = document.getElementById('p');
  assert.equal(document.adoptNode(p), p);
  assert.equal(p.parentNode, null);
  assert.equal(p.ownerDocument, document);
  const other = parse('');
  const frag = other.createDocumentFragment();
  frag.append(other.createElement('i'), other.createTextNode('t'));
  document.body.append(frag);
  assert.deepEqual(Array.from(document.body.childNodes, (n) => n.ownerDocument === document), [true, true]);
  assert.equal(frag.ownerDocument, other);
});

test('moving between two documents without registries arms no custom element hooks', () => {
  const a = parse('<x-a id="x"></x-a>');
  const b = parse('');
  b.body.append(a.getElementById('x'));
  assert.equal(a.__ceRegistry, undefined);
  assert.equal(b.__ceRegistry, undefined);
  assert.equal(b.querySelector('x-a').matches(':defined'), false);
});

test('a custom element keeps its callbacks in a document without a registry; errors go to its window', () => {
  const { window, document } = createEnvironment('<!doctype html><html><body></body></html>');
  const log = [], reported = [];
  window.reportError = (e) => reported.push(e.message);
  window.customElements.define('x-m', class extends window.HTMLElement {
    connectedCallback() { log.push(['conn', this.ownerDocument === document]); }
    disconnectedCallback() { log.push(['disc', this.ownerDocument === document]); throw new Error('boom'); }
  });
  const other = parse('');
  const el = document.createElement('x-m');
  other.body.append(el);
  assert.equal(other.__ceRegistry, null); // armed, no registry
  el.remove();
  assert.deepEqual(log, [['conn', false], ['disc', false]]);
  assert.deepEqual(reported, ['boom']);
  // the registry-less document never upgrades
  other.body.innerHTML = '<x-m></x-m>';
  assert.equal(other.body.firstChild.matches(':defined'), false);
});

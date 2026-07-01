// Regression net for the features added across recent releases: lazy attrs,
// memoized live views/collections, the selector engine, mutation methods,
// template content, Range/TreeWalker, the serializer, and the legacy event
// surface. Goal: lock the behavior so perf refactors can't silently break it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const fresh = (html) => createEnvironment(html ?? '<!doctype html><html><head></head><body></body></html>');

// ----------------------------------------------------- mutation methods ----
test('insertBefore with a DocumentFragment splices its children', () => {
  const { document } = fresh();
  const host = document.body;
  const frag = document.createDocumentFragment();
  frag.appendChild(document.createElement('a'));
  frag.appendChild(document.createElement('b'));
  host.appendChild(frag);
  assert.equal(host.children.length, 2);
  assert.equal(frag.children.length, 0); // moved out
});

test('replaceChild swaps the node and returns the old one', () => {
  const { document } = fresh();
  const a = document.createElement('a'), b = document.createElement('b');
  document.body.append(a);
  const ret = document.body.replaceChild(b, a);
  assert.equal(ret, a);
  assert.equal(document.body.firstChild, b);
});

test('insertBefore throws NotFoundError for a non-child ref', () => {
  const { document } = fresh();
  const stray = document.createElement('x');
  assert.throws(() => document.body.insertBefore(document.createElement('y'), stray), /NotFoundError/);
});

test('removeChild throws for a non-child', () => {
  const { document } = fresh();
  assert.throws(() => document.body.removeChild(document.createElement('z')), /NotFoundError/);
});

test('compareDocumentPosition: contains, contained, preceding, following', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="p"><span id="c1"></span><span id="c2"></span></div>';
  const p = document.getElementById('p'), c1 = document.getElementById('c1'), c2 = document.getElementById('c2');
  assert.equal(p.compareDocumentPosition(p), 0);
  assert.equal(p.compareDocumentPosition(c1) & 16, 16); // CONTAINED_BY
  assert.equal(c1.compareDocumentPosition(p) & 8, 8);   // CONTAINS
  assert.equal(c1.compareDocumentPosition(c2) & 4, 4);  // FOLLOWING
  assert.equal(c2.compareDocumentPosition(c1) & 2, 2);  // PRECEDING
  const detached = document.createElement('q');
  assert.equal(c1.compareDocumentPosition(detached), 1); // DISCONNECTED-ish
});

// ----------------------------------------------------------- attributes ----
test('toggleAttribute add/remove/force', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  assert.equal(el.toggleAttribute('hidden'), true);
  assert.equal(el.hasAttribute('hidden'), true);
  assert.equal(el.toggleAttribute('hidden'), false);
  assert.equal(el.hasAttribute('hidden'), false);
  assert.equal(el.toggleAttribute('hidden', true), true);
  assert.equal(el.toggleAttribute('hidden', true), true); // force keeps it
  assert.equal(el.toggleAttribute('hidden', false), false);
});

test('lazy attrs: getAttribute on buffer-backed element never builds the array prematurely', () => {
  const { document } = fresh('<!doctype html><body><div id="x" data-a="1" class="c">hi</div></body>');
  const el = document.getElementById('x');
  assert.equal(el.getAttribute('data-a'), '1');
  assert.equal(el.hasAttribute('class'), true);
  assert.deepEqual(el.getAttributeNames().sort(), ['class', 'data-a', 'id']);
  assert.equal(el.attributes.length, 3);
});

test('dataset reads + reflects through attributes', () => {
  const { document } = fresh('<!doctype html><body><div id="x" data-foo-bar="v"></div></body>');
  const el = document.getElementById('x');
  assert.equal(el.dataset.fooBar, 'v');
  el.dataset.fooBar = 'w';
  assert.equal(el.getAttribute('data-foo-bar'), 'w');
});

// ---------------------------------------------------- form-control value ----
test('option/input/textarea value getters', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<select><option>txt</option><option value="v">x</option></select>' +
    '<input value="iv"><textarea>tv</textarea>';
  const [o1, o2] = document.getElementsByTagName('option');
  assert.equal(o1.value, 'txt');   // no value attr → textContent
  assert.equal(o2.value, 'v');
  assert.equal(document.querySelector('input').value, 'iv');
  const ta = document.querySelector('textarea');
  assert.equal(ta.value, 'tv');    // raw value defaults to child text content
  ta.value = 'edited';
  assert.equal(ta.value, 'edited');
});

test('live collection is a spec object (typeof + Object.keys)', () => {
  const { document } = fresh();
  document.body.innerHTML = '<i></i><i></i>';
  const c = document.getElementsByTagName('i');
  assert.equal(typeof c, 'object');                 // not 'function'
  assert.deepEqual(Object.keys(c), ['0', '1']);     // enumerable indices only
  assert.deepEqual(Object.getOwnPropertyNames(c).sort(), ['0', '1', 'length']);
});

test('input setRangeText splices the value', () => {
  const { document } = fresh();
  const i = document.createElement('input');
  i.value = 'hello';
  i.setRangeText('XX', 1, 3);
  assert.equal(i.value, 'hXXlo');
});

test('label.control resolves via for= and implicit nesting', () => {
  const { document } = fresh();
  document.body.innerHTML = '<label for="t">L</label><input id="t">' +
    '<label id="imp">wrap <input id="t2"></label>';
  assert.equal(document.querySelector('label').control, document.getElementById('t'));
  assert.equal(document.getElementById('imp').control, document.getElementById('t2'));
});

// ------------------------------------------------------------- selectors ----
test('selector engine: combinators, attribute + pseudo selectors', () => {
  const { document } = fresh();
  document.body.innerHTML =
    '<ul class="l"><li class="a">1</li><li class="a b" data-k="v">2</li><li>3</li></ul>' +
    '<section><p>x</p><p>y</p></section>';
  assert.equal(document.querySelectorAll('ul > li').length, 3);
  assert.equal(document.querySelectorAll('ul li.a').length, 2);
  assert.equal(document.querySelectorAll('li.a + li').length, 2);
  assert.equal(document.querySelectorAll('li.a ~ li').length, 2);
  assert.equal(document.querySelectorAll('[data-k="v"]').length, 1);
  assert.equal(document.querySelectorAll('[data-k]').length, 1);
  assert.equal(document.querySelectorAll('li:first-child').length, 1);
  assert.equal(document.querySelectorAll('li:last-child').length, 1);
  assert.equal(document.querySelectorAll('p:nth-child(2)').length, 1);
  assert.equal(document.querySelectorAll('li:not(.a)').length, 1);
  assert.equal(document.querySelectorAll('section p, ul li.b').length, 3);
  assert.ok(document.querySelector('li.b').matches('[data-k="v"]'));
});

test('attribute operators ^= $= *= |=', () => {
  const { document } = fresh();
  document.body.innerHTML = '<a href="https://x.com/p.pdf" hreflang="en-US" rel="a b">L</a>';
  const a = document.querySelector('a');
  assert.ok(a.matches('[href^="https"]'));
  assert.ok(a.matches('[href$=".pdf"]'));
  assert.ok(a.matches('[href*="x.com"]'));
  assert.ok(a.matches('[hreflang|="en"]'));
  assert.ok(a.matches('[rel~="b"]'));
});

test('getElementsByClassName matches a class in the middle of the list', () => {
  const { document } = fresh('<!doctype html><body><div class="x mid y"></div><div class="mid"></div></body>');
  assert.equal(document.getElementsByClassName('mid').length, 2);
});

// --------------------------------------------------- memoized live views ----
test('childNodes / children are stable per node and live', () => {
  const { document } = fresh();
  const d = document.createElement('div');
  assert.equal(d.childNodes, d.childNodes);  // identity stable
  assert.equal(d.children, d.children);
  d.appendChild(document.createElement('span'));
  assert.equal(d.childNodes.length, 1);      // reads live
  assert.equal(d.children.length, 1);
});

test('getElementsByTagName collection is memoized + live', () => {
  const { document } = fresh();
  const c = document.getElementsByTagName('p');
  assert.equal(c, document.getElementsByTagName('p')); // same Proxy
  document.body.appendChild(document.createElement('p'));
  assert.equal(c.length, 1);                          // live
  assert.equal(c[0].localName, 'p');
});

test('live HTMLCollection protocol: index, item, length, iterate, has, keys', () => {
  const { document } = fresh();
  document.body.innerHTML = '<i></i><i></i>';
  const c = document.getElementsByTagName('i');
  assert.equal(c.length, 2);
  assert.equal(c.item(0), c[0]);
  assert.equal(c.item(9), null);
  assert.equal('length' in c, true);
  assert.equal(0 in c, true);
  assert.equal(9 in c, false);
  assert.deepEqual([...c].length, 2);          // Symbol.iterator
  assert.deepEqual(Object.keys(c), ['0', '1']); // ownKeys
});

// --------------------------------------------------------- template ----
test('<template>.content holds parsed markup, excluded from children', () => {
  const { document } = fresh('<!doctype html><body><template><b>hi</b></template></body>');
  const t = document.querySelector('template');
  assert.equal(t.children.length, 0);
  assert.equal(t.content.querySelector('b').textContent, 'hi');
  assert.match(t.innerHTML, /<b>hi<\/b>/);
});

test('template content survives innerHTML parse', () => {
  const { document } = fresh();
  document.body.innerHTML = '<template><i>x</i></template>';
  assert.equal(document.querySelector('template').content.querySelector('i').textContent, 'x');
});

// --------------------------------------------------- namespaces / clone ----
test('createElementNS keeps namespace + case', () => {
  const { document } = fresh();
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  const path = document.createElementNS('http://www.w3.org/2000/svg', 'svg:path');
  assert.equal(svg.namespaceURI, 'http://www.w3.org/2000/svg');
  assert.equal(path.localName, 'path');
});

test('cloneNode deep copies attributes + subtree', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="o" class="c"><span>t</span></div>';
  const clone = document.getElementById('o').cloneNode(true);
  assert.equal(clone.getAttribute('class'), 'c');
  assert.equal(clone.querySelector('span').textContent, 't');
  const shallow = document.getElementById('o').cloneNode(false);
  assert.equal(shallow.children.length, 0);
});

// ----------------------------------------------------- Range / TreeWalker ----
test('Range setStart/setEnd bookkeeping', () => {
  const { document } = fresh();
  document.body.innerHTML = '<p>abc</p>';
  const r = document.createRange();
  const p = document.querySelector('p');
  r.setStart(p.firstChild, 1);
  r.setEnd(p.firstChild, 2);
  assert.equal(r.startOffset, 1);
  assert.equal(r.endOffset, 2);
});

test('TreeWalker honors whatToShow + filter', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div><span>a</span><!--c--><b>d</b></div>';
  const root = document.querySelector('div');
  const w = document.createTreeWalker(root, 1, { acceptNode: (n) => n.localName === 'b' ? 1 : 3 });
  const seen = [];
  let n; while ((n = w.nextNode())) seen.push(n.localName);
  assert.deepEqual(seen, ['b']);
});

// --------------------------------------------------------- DOMParser ----
test('DOMParser parses html + xml-ish strings', () => {
  const { window } = fresh();
  const p = new window.DOMParser();
  const d1 = p.parseFromString('<div id="z">x</div>', 'text/html');
  assert.equal(d1.getElementById('z').textContent, 'x');
  const d2 = p.parseFromString('<note>hi</note>', 'application/xml');
  assert.ok(d2.querySelector('note'));
});

test('implementation.createHTMLDocument + createDocumentType', () => {
  const { document } = fresh();
  const d = document.implementation.createHTMLDocument('T');
  assert.equal(d.title, 'T');
  const dt = document.implementation.createDocumentType('html', '', '');
  assert.equal(dt.name, 'html');
  assert.equal(document.implementation.hasFeature(), true);
});

// ------------------------------------------------- MutationObserver subtree ----
test('MutationObserver fires for subtree childList + attributes', async () => {
  const { window, document } = fresh();
  document.body.innerHTML = '<div id="r"><span id="s"></span></div>';
  const records = [];
  const mo = new window.MutationObserver((recs) => records.push(...recs));
  mo.observe(document.getElementById('r'), { childList: true, attributes: true, subtree: true, attributeOldValue: true });
  document.getElementById('s').setAttribute('data-x', '1');     // attribute in subtree
  document.getElementById('s').appendChild(document.createElement('em')); // childList in subtree
  await Promise.resolve();
  mo.disconnect();
  assert.ok(records.some((r) => r.type === 'attributes' && r.attributeName === 'data-x'));
  assert.ok(records.some((r) => r.type === 'childList'));
});

// -------------------------------------------------------------- serializer ----
test('serializer handles void/comment/doctype/raw-text', () => {
  const { document } = fresh('<!doctype html><html><head></head><body><br><img src="x"><!--c--><style>a>b{}</style></body></html>');
  const html = document.documentElement.outerHTML;
  assert.match(html, /<br>/);
  assert.match(html, /<img src="x">/);
  assert.match(html, /<!--c-->/);
  assert.match(html, /<style>a>b\{\}<\/style>/); // raw text not escaped
});

// ------------------------------------------------------- legacy event API ----
test('CustomEvent + initCustomEvent', () => {
  const { window } = fresh();
  const e = new window.CustomEvent('x', { detail: 42 });
  assert.equal(e.detail, 42);
  const e2 = new window.CustomEvent('y');
  e2.initCustomEvent('z', true, true, 'd');
  assert.equal(e2.type, 'z');
  assert.equal(e2.bubbles, true);
  assert.equal(e2.detail, 'd');
});

test('legacy createEvent + initMouseEvent', () => {
  const { document } = fresh();
  const e = document.createEvent('MouseEvent');
  e.initMouseEvent('click', true, true, document.defaultView);
  assert.equal(e.type, 'click');
  assert.equal(e.bubbles, true);
  assert.equal(e.view, document.defaultView);
});

test('addEventListener dedupes on (callback, capture)', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  let n = 0;
  const fn = () => { n++; };
  el.addEventListener('click', fn);
  el.addEventListener('click', fn);  // ignored (dup)
  el.click();
  assert.equal(n, 1);
});

test('__eventPath walks target → document → window', () => {
  const { document } = fresh();
  document.body.innerHTML = '<div id="d"></div>';
  const path = document.getElementById('d').__eventPath();
  assert.ok(path.includes(document.getElementById('d')));
  assert.ok(path.includes(document));
});

// --------------------------------------------------------------- cascade ----
test('cascade resolves single-token background shorthand to background-color', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>.bg{background:tomato}</style>';
  document.body.innerHTML = '<div class="bg" id="d"></div>';
  assert.equal(window.getComputedStyle(document.getElementById('d')).backgroundColor, 'rgb(255, 99, 71)');
});

// ----------------- parity with rtdom: after()/replaceWith() viable-next-sibling ----
test('after() with an existing following sibling as an argument reorders (no throw)', () => {
  const { document } = fresh();
  document.body.innerHTML = '<i id="a"></i><i id="b"></i>';
  const a = document.getElementById('a'), b = document.getElementById('b');
  const c = document.createElement('i'); c.id = 'c';
  a.after(b, c); // b is a's current nextSibling AND an argument
  assert.deepEqual([...document.body.children].map(n => n.id), ['a', 'b', 'c']);
});

test('replaceWith() with an existing following sibling as an argument keeps order', () => {
  const { document } = fresh();
  document.body.innerHTML = '<i id="a"></i><i id="b"></i>';
  const a = document.getElementById('a'), b = document.getElementById('b');
  const c = document.createElement('i'); c.id = 'c';
  a.replaceWith(b, c);
  assert.deepEqual([...document.body.children].map(n => n.id), ['b', 'c']);
});

test('CharacterData.after() viable-next-sibling handles an argument that is a sibling', () => {
  const { document } = fresh();
  document.body.innerHTML = 'x<i id="b"></i>';
  const t = document.body.firstChild, b = document.getElementById('b');
  const c = document.createElement('i'); c.id = 'c';
  t.after(b, c);
  assert.deepEqual([...document.body.childNodes].map(n => n.id ?? n.data), ['x', 'b', 'c']);
});

// ----------------- parity with rtdom: statement at-rule doesn't swallow next rule ----
test('cascade: @import does not swallow the following rule', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>@import url("x.css"); #d{color:green}</style>';
  document.body.innerHTML = '<div id="d"></div>';
  assert.equal(window.getComputedStyle(document.getElementById('d')).color, 'rgb(0, 128, 0)');
});

test('cascade: @charset then a real rule both handled', () => {
  const { window, document } = fresh();
  document.head.innerHTML = '<style>@charset "utf-8"; .c{color:blue}</style>';
  document.body.innerHTML = '<div class="c" id="d"></div>';
  assert.equal(window.getComputedStyle(document.getElementById('d')).color, 'rgb(0, 0, 255)');
});

// <template> contents (miaskiewicz/turbo-dom#29 item 3, #36; the Chrome 156 behaviour is
// gated by test/dom-adoption-reactions-chrome.test.mjs): an inert owner document per
// document, lazily created contents, innerHTML / cloneNode / importNode, and buffer-backed
// contents moving to and from the document without being materialized.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

test('one inert document per document; nested templates stay in it', () => {
  const { document } = createEnvironment('<!doctype html><html><body><template id="t"><template id="n"><x-a></x-a></template></template></body></html>');
  const inert = document.getElementById('t').content.ownerDocument;
  assert.notEqual(inert, document);
  assert.equal(inert.__templateDocument(), inert);
  assert.equal(document.createElement('template').content.ownerDocument, inert);
  const nested = document.getElementById('t').content.firstChild;
  assert.equal(nested.ownerDocument, inert);
  assert.equal(nested.content.ownerDocument, inert);
  assert.equal(nested.content.firstChild.localName, 'x-a');
  assert.equal(inert.defaultView, null);
  assert.equal(inert.documentElement, null);
});

test('reset gives the document a new inert document', () => {
  const env = createEnvironment('<!doctype html><html><body><template id="t"><i></i></template></body></html>');
  const before = env.document.getElementById('t').content.ownerDocument;
  env.reset();
  const after = env.document.getElementById('t').content.ownerDocument;
  assert.notEqual(after, before);
  assert.equal(env.document.getElementById('t').content.firstChild.localName, 'i');
});

test('buffer-backed contents move between the inert document and the document lazily', () => {
  const { document } = createEnvironment('<!doctype html><html><body><template id="t"><section id="s" class="c"><p>one<b>two</b></p></section></template></body></html>');
  const t = document.getElementById('t');
  const section = t.content.firstChild;
  assert.equal(section.__kids, null); // not inflated
  document.body.append(section);
  assert.equal(section.__kids, null); // still lazy: same buffer, owner switched only
  assert.equal(section.ownerDocument, document);
  assert.equal(section.outerHTML, '<section id="s" class="c"><p>one<b>two</b></p></section>');
  assert.equal(section.querySelector('b').ownerDocument, document);
  // and back into the contents, with an attached shadow root
  const root = section.attachShadow({ mode: 'open' });
  root.innerHTML = '<i></i>';
  t.content.append(section);
  assert.equal(section.ownerDocument, t.content.ownerDocument);
  assert.equal(root.ownerDocument, t.content.ownerDocument);
  assert.equal(root.firstChild.ownerDocument, t.content.ownerDocument);
});

test('innerHTML-created templates own their contents in the inert document', () => {
  const { document } = createEnvironment('<!doctype html><html><body><div id="d"></div></body></html>');
  const d = document.getElementById('d');
  d.innerHTML = '<template><p>x</p></template>';
  const t = d.firstChild;
  assert.equal(t.childNodes.length, 0);
  assert.equal(t.content.ownerDocument, document.createElement('template').content.ownerDocument);
  assert.equal(t.content.firstChild.ownerDocument, t.content.ownerDocument);
  assert.equal(t.innerHTML, '<p>x</p>');
  t.innerHTML = '';
  assert.equal(t.content.childNodes.length, 0);
});

test('content: read-only on templates, reflected on <meta>, an ordinary property elsewhere', () => {
  const { document } = createEnvironment('<!doctype html><html><body></body></html>');
  const t = document.createElement('template');
  const content = t.content;
  t.content = 'nope';
  assert.equal(t.content, content);
  const meta = document.createElementNS('http://www.w3.org/1999/xhtml', 'meta');
  meta.setAttribute('content', 'a');
  assert.equal(meta.content, 'a');
  const svgTemplate = document.createElementNS('http://www.w3.org/2000/svg', 'template');
  assert.equal(svgTemplate.content, undefined);
  svgTemplate.content = 1;
  assert.equal(svgTemplate.content, 1);
});

test('importNode clones into the importing document; cloneNode stays in the source one', () => {
  const { document } = createEnvironment('<!doctype html><html><body><template id="t"><p id="p">x</p><!--c-->t</template></body></html>');
  const content = document.getElementById('t').content;
  const imported = document.importNode(content, true);
  assert.deepEqual(Array.from(imported.childNodes, (n) => n.ownerDocument === document), [true, true, true]);
  const cloned = content.cloneNode(true);
  assert.deepEqual(Array.from(cloned.childNodes, (n) => n.ownerDocument === content.ownerDocument), [true, true, true]);
  assert.equal(document.importNode(content.firstChild, false).childNodes.length, 0);
  const doctype = document.importNode(document.doctype);
  assert.deepEqual([doctype.name, doctype.ownerDocument === document], ['html', true]);
});

test('a document clone is a new document owning cloned children', () => {
  const { document } = createEnvironment('<!doctype html><html><head><title>t</title></head><body><p>x</p></body></html>');
  const copy = document.cloneNode(true);
  assert.notEqual(copy, document);
  assert.equal(copy.documentElement.ownerDocument, copy);
  assert.equal(copy.body.innerHTML, '<p>x</p>');
  assert.equal(copy.doctype.name, 'html');
  assert.equal(document.cloneNode().childNodes.length, 0);
});

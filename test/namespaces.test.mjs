// createElementNS / createElement edge cases around miaskiewicz/turbo-dom#29 item 1 (the
// Chrome 156 behaviour itself is gated by test/dom-adoption-reactions-chrome.test.mjs):
// namespaces other than HTML/SVG/MathML, prefixes, qualified-name validation, tag-name
// lookup by qualified name, serialization.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const SVG = 'http://www.w3.org/2000/svg';
const HTML = 'http://www.w3.org/1999/xhtml';
const fresh = () => createEnvironment('<!doctype html><html><head></head><body></body></html>').document;
const errorName = (fn) => { try { fn(); return 'ok'; } catch (e) { return e.name; } };

test('createElement validates the local name and lowercases it', () => {
  const document = fresh();
  assert.equal(document.createElement('DiV').localName, 'div');
  assert.equal(document.createElement('x-Foo').localName, 'x-foo');
  for (const bad of ['', 'a b', 'a\tb', 'a\nb', 'a\fb', 'a\rb', 'a\0b', 'a/b', 'a>b', '1a', '-a', '$']) {
    assert.equal(errorName(() => document.createElement(bad)), 'InvalidCharacterError', JSON.stringify(bad));
  }
  // a non-letter start: ':', '_' or non-ASCII, then only name characters
  for (const ok of [':a', '_a-1.b_c:d', 'été', '_é']) assert.equal(errorName(() => document.createElement(ok)), 'ok', ok);
  for (const bad of ['_a b', '_a$', '_a/']) assert.equal(errorName(() => document.createElement(bad)), 'InvalidCharacterError', bad);
  // a letter start allows anything but whitespace, NULL, '/' and '>'
  assert.equal(document.createElement('a$%^').localName, 'a$%^');
});

test('createElementNS validates the prefix and the namespace pairing', () => {
  const document = fresh();
  for (const bad of ['a b:c', 'a\0:c', 'a/:c', 'a>:c', 'a\t:c']) {
    assert.equal(errorName(() => document.createElementNS('urn:x', bad)), 'InvalidCharacterError', JSON.stringify(bad));
  }
  assert.equal(errorName(() => document.createElementNS(undefined, 'p:a')), 'NamespaceError');
  assert.equal(errorName(() => document.createElementNS('http://www.w3.org/2000/xmlns/', 'xmlns:a')), 'ok');
  assert.equal(document.createElementNS(undefined, 'a').namespaceURI, null);
});

test('prefixed elements: tagName, lookup by qualified name, clones', () => {
  const document = fresh();
  const hdiv = document.createElementNS(HTML, 'h:div');
  assert.deepEqual([hdiv.prefix, hdiv.localName, hdiv.tagName, hdiv.namespaceURI], ['h', 'div', 'H:DIV', HTML]);
  const circle = document.createElementNS(SVG, 'svg:circle');
  const plain = document.createElementNS(SVG, 'circle');
  document.body.append(hdiv, circle, plain, document.createElement('div'));
  const count = (name) => [document.getElementsByTagName(name).length, document.body.getElementsByTagName(name).length];
  assert.deepEqual(count('div'), [1, 1]);           // the unprefixed HTML div only
  assert.deepEqual(count('h:div'), [1, 1]);
  assert.deepEqual(count('H:DIV'), [1, 1]);         // HTML elements: lowercased qualified name
  assert.deepEqual(count('circle'), [1, 1]);        // the unprefixed svg circle only
  assert.deepEqual(count('svg:circle'), [1, 1]);
  assert.deepEqual(count('SVG:CIRCLE'), [0, 0]);    // foreign elements: exact qualified name
  assert.deepEqual(count('x:circle'), [0, 0]);
  assert.deepEqual(count('*'), [7, 4]);      // html, head, body + the four
  assert.equal(circle.cloneNode().prefix, 'svg');
  assert.equal(plain.cloneNode().prefix, null);
  assert.equal(document.createElement('p').prefix, null);
});

test('serialization: other namespaces use the qualified name and are never void', () => {
  const document = fresh();
  const br = document.createElementNS('urn:x', 'br');
  br.append(document.createElementNS('urn:x', 'q:Img'));
  const host = document.createElement('div');
  host.append(br, document.createElementNS(SVG, 'svg:image'), document.createElement('br'));
  assert.equal(host.innerHTML, '<br><q:Img></q:Img></br><svg:image></svg:image><br>'); // Chrome 156 keeps the prefix
  const t = document.createElementNS('urn:x', 'template');
  t.append('x');
  assert.equal(t.outerHTML, '<template>x</template>');
  // svg keeps its own fragment context
  const svg = document.createElementNS(SVG, 'svg');
  svg.innerHTML = '<rect/>';
  assert.equal(svg.firstChild.namespaceURI, SVG);
});

test('name errors are plain named Errors where the host has no DOMException', () => {
  const document = fresh();
  const saved = globalThis.DOMException;
  delete globalThis.DOMException;
  try {
    let error;
    try { document.createElement('1a'); } catch (e) { error = e; }
    assert.ok(error instanceof Error);
    assert.equal(error.name, 'InvalidCharacterError');
  } finally {
    globalThis.DOMException = saved;
  }
});

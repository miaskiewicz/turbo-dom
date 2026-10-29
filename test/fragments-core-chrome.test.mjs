// The JS runtime's document and innerHTML fragment parsing against the parse rows of the P5-cov
// Chrome 156 control (crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json, generated
// by scripts/rtdom-core-chrome-control.mjs; asserted for Rust by
// crates/turbo-dom/tests/rtdom_core_chrome.rs): every context x input of the U+0000 matrix
// (miaskiewicz/turbo-dom#92: Chrome drops a literal U+0000 in a textarea / title fragment)
// and the other fragment cases, as html5lib-format dumps.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const fixture = JSON.parse(
  readFileSync(new URL('../crates/turbo-dom/tests/fixtures/rtdom_core_chrome156.json', import.meta.url), 'utf8'),
);

// Recorded Chrome answers the JS runtime does not give yet (none; turbo-dom#92 is fixed).
const OPEN = new Set([]);

const NS = { svg: 'http://www.w3.org/2000/svg', math: 'http://www.w3.org/1998/Math/MathML' };

function dump(root) {
  const lines = [];
  const indent = (depth) => `| ${'  '.repeat(depth)}`;
  const walk = (node, depth) => {
    if (node.nodeType === 1) {
      const ns = node.namespaceURI === NS.svg ? 'svg ' : node.namespaceURI === NS.math ? 'math ' : '';
      lines.push(`${indent(depth)}<${ns}${node.localName}>`);
      const attrs = [...node.attributes]
        .map((a) => [a.prefix ? `${a.prefix} ${a.localName}` : a.localName, a.value])
        .sort((x, y) => (x[0] < y[0] ? -1 : x[0] > y[0] ? 1 : 0));
      for (const [name, value] of attrs) lines.push(`${indent(depth + 1)}${name}="${value}"`);
      if (ns === '' && node.localName === 'template') {
        lines.push(`${indent(depth + 1)}content`);
        for (const child of node.content.childNodes) walk(child, depth + 2);
      }
      for (const child of node.childNodes) walk(child, depth + 1);
    } else if (node.nodeType === 3) lines.push(`${indent(depth)}"${node.data}"`);
    else if (node.nodeType === 8) lines.push(`${indent(depth)}<!-- ${node.data} -->`);
    else if (node.nodeType === 10) {
      lines.push(node.publicId || node.systemId
        ? `${indent(depth)}<!DOCTYPE ${node.name} "${node.publicId}" "${node.systemId}">`
        : `${indent(depth)}<!DOCTYPE ${node.name}>`);
    } else throw new Error(`unexpected node type ${node.nodeType}`);
  };
  for (const child of root.childNodes) walk(child, 0);
  return lines.join('\n');
}

test('innerHTML fragments match the Chrome 156 control (U+0000 matrix, CDATA, frameset)', () => {
  assert.ok(fixture.chrome.startsWith('156.'));
  const { document } = createEnvironment('<!DOCTYPE html><html><head></head><body></body></html>');
  const failures = [];
  let rows = 0;
  for (const c of fixture.documents) {
    if (!c.context) continue;
    rows++;
    if (OPEN.has(c.id)) continue;
    const [prefix, local] = c.context.includes(' ') ? c.context.split(' ') : [null, c.context];
    const element = prefix ? document.createElementNS(NS[prefix], local) : document.createElement(local);
    element.innerHTML = c.html;
    const got = dump(element);
    if (got !== c.dump) failures.push(`${c.id}:\n--- Chrome\n${c.dump}\n--- JS\n${got}`);
  }
  assert.ok(rows >= 75, `fixture shrank: ${rows} fragment rows`);
  assert.deepEqual(failures, []);
});

// Documents: the control parses them with DOMParser (no declarative shadow roots), the
// runtime's environment like a navigation (which attaches them), so that one case differs
// by design.
const NAVIGATION_ONLY = new Set(['declarative_shadow_root_not_attached']);
// Recorded Chrome answers the runtime does not give yet (none; turbo-dom#103 and #104 are fixed).
const OPEN_DOCUMENTS = new Set([]);

test('documents match the Chrome 156 control (tree dumps)', () => {
  const failures = [];
  let rows = 0;
  for (const c of fixture.documents) {
    if (c.context) continue;
    rows++;
    if (NAVIGATION_ONLY.has(c.id) || OPEN_DOCUMENTS.has(c.id)) continue;
    const { document } = createEnvironment(c.html);
    const got = dump(document);
    if (got !== c.dump) failures.push(`${c.id}:\n--- Chrome\n${c.dump}\n--- JS\n${got}`);
  }
  assert.ok(rows >= 90, `fixture shrank: ${rows} document rows`);
  assert.deepEqual(failures, []);
});

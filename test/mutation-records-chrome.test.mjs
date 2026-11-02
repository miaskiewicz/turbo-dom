// MutationObserver childList records against Chrome 156: one record per DOM operation
// (a multi-node call inserts a fragment), with the removed nodes of innerHTML /
// textContent / replaceChildren and the real previousSibling. The cases live in
// fixtures/mutation-records-cases.js and ran unchanged in headless Chrome 156.0.8078.47;
// EXPECTED is its output, verbatim.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const src = readFileSync(new URL('./fixtures/mutation-records-cases.js', import.meta.url), 'utf8');
const { MCASES, mrun } = new Function(`${src}\nreturn { MCASES, mrun };`)();
const EXPECTED = {
  "before2": "[\"\",[[\"r\",[\"x\",\"y\"],[],\"a\",\"b\"]]]",
  "after2": "[\"\",[[\"r\",[\"x\",\"y\"],[],\"a\",\"b\"]]]",
  "replaceWith2": "[\"\",[[\"r\",[\"x\",\"y\"],[\"a\"],null,\"b\"]]]",
  "replaceWithSelf": "[\"\",[[\"r\",[],[\"a\"],null,\"b\"],[\"r\",[\"x\",\"a\"],[],null,\"b\"]]]",
  "replaceChildren2": "[\"\",[[\"r\",[\"x\",\"y\"],[\"a\",\"b\"],null,null]]]",
  "replaceChildrenEmpty": "[\"\",[[\"r\",[],[\"a\",\"b\"],null,null]]]",
  "adjacentHtml": "[\"\",[[\"r\",[\"x\",\"y\"],[],\"b\",null]]]",
  "adjacentHtmlAfterbegin": "[\"\",[[\"r\",[\"x\",\"y\"],[],null,\"a\"]]]",
  "appendFragment": "[\"\",[[\"r\",[\"x\",\"y\"],[],\"b\",null]]]",
  "appendEmptyFragment": "[\"\",[]]",
  "append2": "[\"\",[[\"r\",[\"x\",\"#text\",\"y\"],[],\"b\",null]]]",
  "prepend2": "[\"\",[[\"r\",[\"x\",\"y\"],[],null,\"a\"]]]",
  "moveWithin": "[\"\",[[\"r\",[],[\"a\"],null,\"b\"],[\"r\",[\"a\"],[],\"b\",null]]]",
  "innerHTML": "[\"\",[[\"r\",[\"x\"],[\"a\",\"b\"],null,null]]]",
  "innerHTMLEmpty": "[\"\",[[\"r\",[],[\"a\",\"b\"],null,null]]]",
  "textContent": "[\"\",[[\"r\",[\"#text\"],[\"a\",\"b\"],null,null]]]",
  "textContentEmpty": "[\"\",[[\"r\",[],[\"a\",\"b\"],null,null]]]",
  "removeB": "[\"\",[[\"r\",[],[\"b\"],\"a\",null]]]",
  "appendX": "[\"\",[[\"r\",[\"x\"],[],\"b\",null]]]",
  "insertBeforeB": "[\"\",[[\"r\",[\"x\"],[],\"a\",\"b\"]]]",
  "normalize": "[\"\",[[\"r\",[\"#text\",\"#text\"],[],\"b\",null],[\"r\",[],[\"#text\"],\"#text\",null]]]",
  "normalizeEmpty": "[\"\",[[\"r\",[\"#text\"],[],\"a\",\"b\"],[\"r\",[],[\"#text\"],\"a\",\"b\"]]]",
  "splitText": "[\"\",[[\"r\",[\"#text\"],[],\"b\",null],[\"r\",[\"#text\"],[],\"#text\",null]]]",
  "outerHTML": "[\"\",[[\"r\",[\"x\",\"y\"],[\"a\"],null,\"b\"]]]",
  "outerHTMLDetached": "[\"\",[]]"
};

test('every fixture case has a Chrome expectation', () => {
  assert.deepEqual(Object.keys(MCASES), Object.keys(EXPECTED));
});
for (const name of Object.keys(EXPECTED)) {
  test(`MutationObserver records: ${name} matches Chrome 156`, () => {
    const { document, window } = createEnvironment('');
    assert.equal(mrun(document, window.MutationObserver, name), EXPECTED[name]);
  });
}

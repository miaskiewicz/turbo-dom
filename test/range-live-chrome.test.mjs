// Live Range boundary updates and Range.createContextualFragment against Chrome 156
// (miaskiewicz/turbo-dom#217). The cases live in fixtures/range-live-chrome-cases.js and ran
// unchanged in headless Chrome 156.0.8078.47; EXPECTED is its output, verbatim.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const src = readFileSync(new URL('./fixtures/range-live-chrome-cases.js', import.meta.url), 'utf8');
const EXPECTED = {
  "issue217": "P|0|1|function",
  "removeBefore": "0|1",
  "removeAncestorOfEnd": "0|true|1",
  "insertShift": "2|3",
  "insertAtBoundary": "1|1",
  "fragmentInsert": "4|5",
  "appendDataNoMove": "1|2",
  "insertDataShift": "0|5",
  "deleteDataClamp": "1|2",
  "replaceDataMid": "2|8",
  "dataSetResets": "0|0",
  "splitTextMoves": "true|1|true|2",
  "splitTextParentOffset": "2|4",
  "normalizeMerges": "1|true|3|true|6",
  "normalizeParentOffset": "true|2|true|2",
  "normalizeEmpty": "1|1",
  "innerHTMLResets": "true|0|0",
  "textContentResets": "true|0|true|0",
  "replaceChildrenResets": "0|0",
  "moveNodeWithin": "0|1",
  "insertNodeOtherRange": "a<u></u>b<b>cd</b>ef|true|1|5",
  "extractOtherRange": "true|1|1",
  "ccfBasic": "11|<b>x</b>yz|<p>ab<b>cd</b>ef</p><i>gh</i>",
  "ccfTable": "<tbody><tr><td>1</td></tr></tbody>",
  "ccfTextStart": "<li>a</li><p>b</p>",
  "ccfDocument": "<p>a</p>x",
  "ccfScript": "<script>window.__ccf=1</script>|SCRIPT",
  "ccfArgs": "TypeError|null",
  "ccfDetachedFrag": "1"
};

for (const name of Object.keys(EXPECTED)) {
  test(`live Range ${name} matches Chrome 156`, () => {
    const { document } = createEnvironment('');
    const CASES = new Function('d0', `${src}\nreturn CASES;`)(document);
    let got;
    try { got = CASES[name](document); } catch (e) { got = `THROW ${e.name}`; }
    assert.equal(got, EXPECTED[name]);
  });
}

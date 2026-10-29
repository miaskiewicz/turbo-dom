// JS-runtime Range against Chrome 156 (miaskiewicz/turbo-dom#207). The cases live in
// fixtures/range-chrome-cases.js and ran unchanged in headless Chrome 156.0.8078.47;
// EXPECTED is its output, verbatim.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEnvironment } from '../src/runtime/index.mjs';

const src = readFileSync(new URL('./fixtures/range-chrome-cases.js', import.meta.url), 'utf8');
const EXPECTED = {
  "extractTextBounds": "<p>af</p><i>gh</i>|b<b>cd</b>e|true|1|true",
  "cloneTextBounds": "<p>ab<b>cd</b>ef</p><i>gh</i>|<p>b<b>cd</b>ef</p><i>g</i>",
  "deleteAcross": "<p>a</p><i>h</i>|true|1|true",
  "extractSameText": "<p>b<b>cd</b>ef</p><i>gh</i>|a|0",
  "toStringAcross": "bcdefg",
  "toStringElement": "abcdefgh",
  "setStartIndexSize": "IndexSizeError",
  "setEndIndexSizeText": "IndexSizeError",
  "setStartDoctype": "InvalidNodeTypeError",
  "setStartAfterEnd": "2|2|true",
  "setEndBeforeStart": "1|1|true",
  "setStartBeforeOrphan": "InvalidNodeTypeError",
  "selectNodeOrphan": "InvalidNodeTypeError",
  "setStartOtherRoot": "true|true",
  "collapseEnd": "2|true",
  "insertNodeContents": "<u></u>ab<b>cd</b>ef|0|4",
  "insertNodeText": "a<u></u>b<b>cd</b>ef|true|1|true|1|b",
  "insertNodeCollapsed": "ab<u></u><b>cd</b>ef|1|2",
  "insertNodeFragment": "<u></u><s></s>ab<b>cd</b>ef|0|2",
  "insertNodeComment": "HierarchyRequestError",
  "surroundPartial": "InvalidStateError|<p>ab<b>cd</b>ef</p><i>gh</i>",
  "surroundText": "<p>a<u>b<b>cd</b>e</u>f</p><i>gh</i>|true|1|2",
  "surroundFragment": "InvalidNodeTypeError",
  "cbp": "-1,1,1,-1|1,1,-1,-1",
  "cbpBadHow": "NotSupportedError",
  "cbpWrongDoc": "no",
  "comparePoint": "-1,-1,0,0,0,true,false,false,true,true,false,true",
  "comparePointErr": "WrongDocumentError,IndexSizeError",
  "constants": "0,1,2,3,3",
  "cloneRangeIndependent": "1|2|false",
  "extractNested": "<p>ab<b>c</b></p><i>h</i>|<p><b>d</b>ef</p><i>g</i>|false|1",
  "instanceConstants": "0,1,2,3",
  "toStringOtherRoot": "|true",
  "commentBounds": "<!--yz-->a|a|<!--yz-->a|<p><!--x-->b<b>cd</b>ef</p><i>gh</i>",
  "deleteSameText": "<p>b<b>cd</b>ef</p><i>gh</i>|true|0",
  "deleteCollapsed": "<p>ab<b>cd</b>ef</p><i>gh</i>",
  "extractCollapsed": "|<p>ab<b>cd</b>ef</p><i>gh</i>",
  "insertNodeSelf": "HierarchyRequestError",
  "insertNodeMoveRef": "ab<b>cd</b>ef|1|3",
  "insertNodeOrphanText": "HierarchyRequestError",
  "insertNodeTextEndInParent": "a<u></u>b<b>cd</b>ef|4|bcd",
  "surroundNonEmpty": "<p>ab<b>cd</b>ef</p><u><i>gh</i></u>|1|2",
  "selectNodeContentsDoctype": "InvalidNodeTypeError",
  "setEndAfterEtc": "1,2,0,1,true",
  "rectsDetach": "0,0,0",
  "intersectsOtherRoot": "false,true,true"
};

for (const name of Object.keys(EXPECTED)) {
  test(`Range ${name} matches Chrome 156`, () => {
    const { document } = createEnvironment('');
    const Range = document.createRange().constructor;
    const CASES = new Function('d0', 'Range', `${src}\nreturn CASES;`)(document, Range);
    let got;
    try { got = CASES[name](document); } catch (e) { got = `THROW ${e.name}`; }
    assert.equal(got, EXPECTED[name]);
  });
}

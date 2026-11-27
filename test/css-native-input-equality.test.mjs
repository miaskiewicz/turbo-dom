import assert from 'node:assert/strict';
import test from 'node:test';
import {sameOwnedSelectorInput} from '../src/runtime/css-native-input-equality.mjs';
test('owned selector input comparison preserves primitive distinctions and complete ordered fields', () => {
  for (const [left, right, expected] of [
    [null, null, true], [null, {}, false], [{}, null, false],
    [1, {}, false], [{}, 1, false], [NaN, NaN, true], [0, -0, false],
    [{a:1}, {a:1}, true], [{a:1}, {b:1}, false], [{a:1}, {a:1,b:2}, false],
    [{a:undefined}, {}, false], [[1,2], [2,1], false],
    [{id:1,state:{checked:false}}, {id:1,state:{checked:true}}, false],
  ]) assert.equal(sameOwnedSelectorInput(left,right),expected);
});
test('owned input comparison never evaluates accessor lookalikes on either side', () => {
  let calls = 0;
  const accessor = Object.defineProperty({},'a',{get(){calls++;throw Error('untrusted accessor');}});
  assert.equal(sameOwnedSelectorInput(accessor,{a:1}),false);
  assert.equal(sameOwnedSelectorInput({a:1},accessor),false);
  assert.equal(calls,0);
});

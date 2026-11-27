import test from 'node:test';
import assert from 'node:assert/strict';
import {collectCopiedCssAttributes} from '../src/runtime/css-copied-attributes.mjs';

const hooks = {
  charge(state, key, size) { state[key] += size; },
  unsupported(reason) { throw new TypeError(reason); },
  units(s) { return s.split('').map(ch => ch.charCodeAt(0)); },
  data(row, key) {
    const field = Object.getOwnPropertyDescriptor(row, key);
    if (!field && key === 'prefix') return undefined;
    if (!field || !Object.hasOwn(field, 'value')) throw new Error('untrusted field');
    return field.value;
  },
};
const state = () => ({maxAttributes:0,maxAttributeUnits:0});

test('copied fields preserve order, namespace, UTF-16 and quota charges',()=>{
  const counters = state();
  const result = collectCopiedCssAttributes([{name:'a',value:'😀',prefix:''},{name:'id',value:'x',prefix:'ns'}], counters, hooks);
  assert.deepEqual(result.map(r=>[r.name,r.valueUnits,r.prefix]),[['a',[55357,56832],''],['id',[120],'ns']]);
  assert.deepEqual(counters,{maxAttributes:2,maxAttributeUnits:8});
  for (const key of ['name','value','prefix'])
    assert.throws(()=>collectCopiedCssAttributes([{name:'a',value:'a',prefix:'',[key]:42}],state(),hooks),/unknown attribute/);
});

test('changed iterator revalidates substituted fields before invoking a getter',()=>{
  const proto=Object.getPrototypeOf([][Symbol.iterator]()), saved=proto.next;
  const original={name:'a',value:'b',prefix:''};
  let replacement={name:'c',value:'d'}, getterCalls=0;
  try {
    proto.next=function(){
      const step=Reflect.apply(saved,this,[]);
      if(step.value===original)step.value=replacement;
      return step;
    };
    assert.equal(collectCopiedCssAttributes([original],state(),hooks)[0].prefix,'');
    replacement.prefix='ns';
    assert.equal(collectCopiedCssAttributes([original],state(),hooks)[0].prefix,'ns');
    Object.defineProperty(replacement,'name',{get(){getterCalls++;}});
    assert.throws(()=>collectCopiedCssAttributes([original],state(),hooks),/untrusted field/);
    assert.equal(getterCalls,0);
  } finally {proto.next=saved;}
});

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
test('descriptor comparison retains proxy order, late mutation, revocation, and reentry', () => {
  function original(left,right) {
    if(Object.is(left,right))return true;
    if(left===null||right===null||typeof left!=='object'||typeof right!=='object')return false;
    const a=Object.getOwnPropertyNames(left),b=Object.getOwnPropertyNames(right);
    if(a.length!==b.length)return false;
    for(let i=0;i<a.length;i++){
      if(a[i]!==b[i])return false;
      const x=Object.getOwnPropertyDescriptor(left,a[i]),y=Object.getOwnPropertyDescriptor(right,b[i]);
      if(!Object.hasOwn(x,'value')||!Object.hasOwn(y,'value')||!original(x.value,y.value))return false;
    }
    return true;
  }
  function scenario(compare,mode){
    const seen=[],sentinel=new Error('descriptor sentinel');
    const childA={z:2},childB={z:2};
    const wrap=(target,side)=>new Proxy(target,{
      ownKeys(object){seen.push(`${side}:keys`);return Reflect.ownKeys(object);},
      getOwnPropertyDescriptor(object,key){
        seen.push(`${side}:${key}`);
        if(side==='b'&&key==='tail'){
          if(mode==='throw')throw sentinel;
          if(mode==='mutate')childA.z=9;
          if(mode==='delete')delete object.tail;
          if(mode==='reenter')seen.push(compare({x:1},{x:1}));
        }
        return Reflect.getOwnPropertyDescriptor(object,key);
      }
    });
    const a=wrap({head:wrap(childA,'ac'),tail:3},'a'),b=wrap({head:wrap(childB,'bc'),tail:3},'b');
    try{return {value:compare(a,b),seen};}catch(error){return {error:error===sentinel?'sentinel':error.name,seen};}
  }
  for(const mode of ['plain','throw','mutate','delete','reenter'])assert.deepEqual(scenario(sameOwnedSelectorInput,mode),scenario(original,mode));
  const revoked=Proxy.revocable({},{});revoked.revoke();
  assert.throws(()=>sameOwnedSelectorInput(revoked.proxy,{}),TypeError);
  assert.equal(sameOwnedSelectorInput(revoked.proxy,revoked.proxy),true);
});

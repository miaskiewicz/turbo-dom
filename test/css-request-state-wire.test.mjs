import test from 'node:test';
import assert from 'node:assert/strict';
import { encodeCssWire, encodeCssRequestState } from '../src/runtime/css-rust-wire.mjs';
function request(contexts) { return {abiVersion:1,targets:contexts.map((context,id)=>({id,context})),limits:{targets:16}}; }
test('request-state full wire remains exact and fresh refs require exact context bytes',()=>{
 const value=request([{color:'red',units:[['vw',10.24]],root:true},{color:'red',units:[['vw',10.24]],root:false}]);
 const first=encodeCssRequestState(value);
 assert.equal(first.full,encodeCssWire(value));assert.equal(first.reusable,false);
 const warm=encodeCssRequestState(structuredClone(value),first.contexts);
 assert.equal(warm.full,first.full);assert.equal(warm.reusable,true);
 assert.deepEqual(JSON.parse(warm.compact).targets.map(row=>row.context),[0,1]);
 value.targets[0].context.color='blue';
 assert.equal(encodeCssRequestState(value,first.contexts).reusable,false);
});
test('request-state reflection order and late aliases match the original encoder',()=>{
 function fixture(log){
  const context={color:'red'}, first={id:1,context};
  const second=new Proxy({id:2,context},{ownKeys(target){log.push('late');context.color='blue';return Reflect.ownKeys(target);},getOwnPropertyDescriptor(target,key){log.push(key);return Object.getOwnPropertyDescriptor(target,key);}});
  return {targets:[first,second],limits:{}};
 }
 const a=[],b=[], expected=encodeCssWire(fixture(a)),actual=encodeCssRequestState(fixture(b));
 assert.equal(actual.full,expected);assert.deepEqual(a,b);
 assert.deepEqual(actual.contexts,['{"color":"red"}','{"color":"blue"}']);
});
test('request-state keeps snapshot deletion, reentry, symbols and original thrown identity',()=>{
 function fixture(log){
  const target={context:{color:'red'},deleted:1};
  return {targets:[new Proxy(target,{ownKeys(value){log.push('keys');return ['deleted','context'];},getOwnPropertyDescriptor(value,key){log.push(key);if(key==='deleted')delete value.context;return Reflect.getOwnPropertyDescriptor(value,key);}})]};
 }
 const a=[],b=[];assert.equal(encodeCssRequestState(fixture(a)).full,encodeCssWire(fixture(b)));assert.deepEqual(a,b);
 const symbol=Symbol('ignored'), ctx={value:-0,[symbol]:1};
 assert.equal(encodeCssRequestState(request([ctx])).full,encodeCssWire(request([ctx])));
 const sentinel=new Error('original'), throwing=new Proxy({},{ownKeys(){throw sentinel;}});
 for(const encode of [encodeCssWire,encodeCssRequestState])assert.throws(()=>encode(request([throwing])),error=>error===sentinel);
 let nested;const reentrant=new Proxy({value:1},{ownKeys(value){nested=encodeCssRequestState(request([{nested:true}]));return Reflect.ownKeys(value);}});
 assert.equal(encodeCssRequestState(request([reentrant])).full,encodeCssWire(request([reentrant])));assert.match(nested.full,/nested/);
});
test('request-state exact wire covers lexical scalars and refusals',()=>{
 const context={'12':1,'2':2,number:-0,fraction:1e-7,exponent:1e25,text:'😺\ud800',empty:null,flag:true};
 assert.equal(encodeCssRequestState(request([context])).full,encodeCssWire(request([context])));
 const cyclic={};cyclic.self=cyclic;
 for(const value of [cyclic,{get x(){throw Error('must not invoke');}},undefined,NaN]){
  assert.throws(()=>encodeCssRequestState(request([value])),TypeError);
 }
 const revoked=Proxy.revocable({},{});revoked.revoke();assert.throws(()=>encodeCssRequestState(request([revoked.proxy])),TypeError);
});
test('request-state preserves unusual root and target shapes before native schema validation',()=>{
 for(const value of [null,7,[],{targets:{}},{targets:[[]]}, {targets:[null,3]}])assert.equal(encodeCssRequestState(value).full,encodeCssWire(value));
 const hidden={id:1,context:{color:'red'}};Object.defineProperty(hidden,'ignored',{get(){throw Error('unused');},enumerable:false});
 assert.equal(encodeCssRequestState({targets:[hidden]}).full,encodeCssWire({targets:[hidden]}));
 const accessor={get context(){throw Error('not invoked');}};
 const cycle={};cycle.targets=[cycle];
 const sparse={targets:Array(2)},extra={targets:[]};extra.targets.extra=1;
 for(const value of [undefined,cycle,{targets:[accessor]},sparse,extra])assert.throws(()=>encodeCssRequestState(value),TypeError);
});
test('request-state lazy full assembly owns immutable encoded fragments after later source writes',()=>{
 const value=request([{color:'red'},{color:'blue'}]);
 const expected=encodeCssWire(value),encoded=encodeCssRequestState(value);
 assert.equal(typeof Object.getOwnPropertyDescriptor(encoded,'full').get,'function');
 value.targets.reverse();value.targets[0].context.color='green';
 assert.equal(encoded.full,expected);
 const changed=encodeCssRequestState(value,encoded.contexts);assert.equal(changed.reusable,false);assert.equal(changed.full,encodeCssWire(value));
 const reordered=encodeCssRequestState(request([{color:'blue'},{color:'red'}]),encoded.contexts);
 assert.equal(reordered.reusable,true);assert.notEqual(reordered.compact,encoded.compact);
});

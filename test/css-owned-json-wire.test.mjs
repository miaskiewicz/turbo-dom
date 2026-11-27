import assert from 'node:assert/strict';
import test from 'node:test';
import {createOwnedJsonCssWire,encodeCssWire} from '../src/runtime/css-rust-wire.mjs';
const provider='cssComputedPreferencesV1';
function preference(text=[97,98]){return{abiVersion:1,preferenceIdentity:[1],initial:{color:[1],fontFamily:[2],fontSizeInput:[3]},userAgentSource:{abiVersion:1,text,documentText:[4]}};}
test('private JSON codec freezes only eligible privately decoded UA bytes and preserves exact wire',()=>{
 const codec=createOwnedJsonCssWire(),data=codec.decode(JSON.stringify(preference()),provider);
 assert.equal(Object.isFrozen(data),false);assert.equal(Object.isFrozen(data.initial.color),false);
 assert.equal(Object.isFrozen(data.userAgentSource.text),true);
 const request={sources:[{id:0,text:data.userAgentSource.text}],use:{disabled:false}};
 for(let i=0;i<3;i++)assert.equal(codec.compile(request),encodeCssWire(request));
 request.use.disabled=true;assert.equal(codec.prepare(request),encodeCssWire(request));
 codec.reset();assert.equal(codec.compile(request),encodeCssWire(request));
 const external=preference();assert.equal(codec.decode(external,provider),external);
 assert.equal(Object.isFrozen(external.userAgentSource.text),false);
 const unknown=codec.decode(JSON.stringify(preference()),'other');assert.equal(Object.isFrozen(unknown.userAgentSource.text),false);
});
test('incomplete and invalid JSON preferences decline optimization without changing their decoded graph',()=>{
 const cases=[null,1,{},preference(),preference(),preference(),preference(),preference(),preference(),preference()];
 delete cases[3].initial.color;cases[4].initial.fontFamily=null;cases[5].userAgentSource.abiVersion=2;
 cases[6].userAgentSource.documentText=[-1];cases[7].userAgentSource.text=[65536];
 cases[8].userAgentSource.text=[1.5];cases[9].userAgentSource.text=['x'];
 for(const value of cases){const codec=createOwnedJsonCssWire(),wire=JSON.stringify(value),data=codec.decode(wire,provider);assert.equal(JSON.stringify(data),wire);if(data?.userAgentSource?.text)assert.equal(Object.isFrozen(data.userAgentSource.text),false);}
 assert.throws(()=>createOwnedJsonCssWire().decode('{',provider),SyntaxError);
 const empty=createOwnedJsonCssWire().decode(JSON.stringify(preference([])),provider);assert.equal(Object.isFrozen(empty.userAgentSource.text),true);
});
test('private JSON fragments preserve fresh request reflection, cycles and original accessor failures',()=>{
 const codec=createOwnedJsonCssWire(),data=codec.decode(JSON.stringify(preference()),provider);let reads=0,getters=0;
 const value=new Proxy({text:data.userAgentSource.text},{ownKeys(target){reads++;return Reflect.ownKeys(target)}});
 assert.equal(codec.compile(value),encodeCssWire(value));assert.equal(codec.compile(value),encodeCssWire(value));assert.equal(reads,4);
 const cyclic={text:data.userAgentSource.text};cyclic.self=cyclic;assert.throws(()=>codec.compile(cyclic),/cyclic/);
 const wrong={};Object.defineProperty(wrong,'field',{enumerable:true,get(){getters++;return 1;}});
 assert.throws(()=>codec.compile(wrong),/own-data/);assert.equal(getters,0);
 const large=codec.decode(JSON.stringify(preference(Array(17000).fill(65535))),provider);
 assert.equal(codec.compile({text:large.userAgentSource.text}),encodeCssWire({text:large.userAgentSource.text}));
 const next=codec.decode(JSON.stringify(preference()),provider);assert.equal(codec.compile(next),encodeCssWire(next));
});

test('object providers, incomplete policies and exposed policy construction permanently decline ownership',()=>{
 const modes=['object','policy','field','prototype','key'];
 for(const mode of modes){
  const codec=createOwnedJsonCssWire();
  if(mode==='object')codec.decode({});
  if(mode==='policy')codec.decode('{}','cssRuntimePolicyV1');
  if(mode==='field')codec.policyField({},'toString');
  if(mode==='prototype')codec.policyField(Object.create(null),'compile');
  if(mode==='key')codec.policyField({},Symbol('compile'));
  codec.reset();const data=codec.decode(JSON.stringify(preference()),provider);
  assert.equal(Object.isFrozen(data.userAgentSource.text),false);
  assert.equal(codec.compile(data),encodeCssWire(data));
 }
});

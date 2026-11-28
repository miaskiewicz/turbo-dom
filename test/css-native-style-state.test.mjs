import test from 'node:test';
import assert from 'node:assert/strict';
import {captureStyleStatePolicy,createStyleStateSession} from '../src/runtime/css-native-style-state.mjs';
function policy(){return {abiVersion:1,adapter:{serialize:{targets:8,inputUnits:4096,retainedUnits:4096,computation:{maxNesting:16,maxOutputUnits:4096,maxTotalUnits:8192}}}}}
function response(id=7){return {abiVersion:1,geometryOnly:false,targets:[{id,values:{color:[114]},custom:{'--x':[49]},usedProperties:[]}]}}
function session(){return createStyleStateSession(captureStyleStatePolicy(JSON.stringify(policy())))}

test('style-state fixed policy owns only exact string-decoded limit fields',()=>{
 const original=policy(),copy=captureStyleStatePolicy(JSON.stringify(original));
 assert.equal(Object.getPrototypeOf(copy),null);assert.ok(Object.isFrozen(copy));assert.ok(Object.isFrozen(copy.computation));
 original.adapter.serialize.targets=0;assert.equal(copy.targets,8);
 assert.equal(captureStyleStatePolicy(original),undefined);assert.equal(Object.isFrozen(original),false);
 for(const wire of ['bad','null','{}'])assert.equal(captureStyleStatePolicy(wire),undefined);
 for(const path of [[],['computation']]){
  const value=policy();let target=value.adapter.serialize;for(const key of path)target=target[key];
  target.extraQuota=1;assert.equal(captureStyleStatePolicy(JSON.stringify(value)),undefined);
 }
 for(const value of [-1,1.5,'8',null]){
  const input=policy();input.adapter.serialize.targets=value;
  assert.equal(captureStyleStatePolicy(JSON.stringify(input)),undefined);
 }
});

test('style-state policy captured primitives ignore later JSON and Number overrides',()=>{
 const wire=JSON.stringify(policy()),oldParse=JSON.parse,oldSafe=Number.isSafeInteger;let calls=0;
 try{
  JSON.parse=()=>{calls++;throw Error('guest parser')};Number.isSafeInteger=()=>{calls++;return false};
  assert.equal(captureStyleStatePolicy(wire).targets,8);assert.equal(calls,0);
 }finally{JSON.parse=oldParse;Number.isSafeInteger=oldSafe}
});

test('style-state limits never become aliases through later policy adapter setter',()=>{
 const wire=JSON.stringify(policy()),copy=captureStyleStatePolicy(wire),mutable=JSON.parse(wire);let callbacks=0;
 Object.defineProperty(Object.prototype,'serialize',{configurable:true,set(fn){callbacks++;fn().targets=0}});
 try{
  const adapter={};adapter.serialize=()=>mutable.adapter.serialize;
  assert.equal(callbacks,1);assert.equal(mutable.adapter.serialize.targets,0);assert.equal(copy.targets,8);
 }finally{delete Object.prototype.serialize}
});

test('explicit native style-state requests bind finalization to exact native-response row',()=>{
 const s=session(),request={},chosen=s.request({request,resultId:7});let calls=0;
 const native={resolveStyleStateV1(wire,token,options){calls++;assert.equal(wire,'wire');assert.equal(token,'token');assert.equal(JSON.parse(options).targetId,7);return JSON.stringify(response())}};
 const result=s.resolve(native,'wire','token',chosen.request),row=result.targets[0];
 const final=s.finalize(row);assert.deepEqual({...final.values},{color:[114]});assert.equal(final.rendererValues,row.rendererValues);assert.equal(final.custom,row.custom);assert.equal(calls,1);
 assert.equal(s.requireMatching({}),undefined);assert.throws(()=>s.requireMatching(request),/retained matching capability/);
 assert.equal(s.finalize({...row}),undefined);assert.equal(s.resolve(native,'wire','token',{}),undefined);assert.equal(calls,1);
 assert.throws(()=>s.resolve({},'wire','token',request),/requires resolveStyleStateV1/);
 const old=final.values.color;row.values.color=[98];assert.deepEqual(old,[114]);
});

test('style-state overrides stay visible and malformed responses never grant projection',()=>{
 const make=()=>{const s=session(),request={};s.request({request,resultId:7});return{s,request}};
 const original=Error('fused override');const first=make();
 assert.throws(()=>first.s.resolve({resolveStyleStateV1(){throw original}},'w','t',first.request),e=>e===original);
 const bad=[null,'bad',JSON.stringify({abiVersion:2}),JSON.stringify({...response(),abiVersion:2})];
 const duplicate=response();duplicate.targets.push({...duplicate.targets[0]});bad.push(JSON.stringify(duplicate));
 const wrong=response();wrong.targets[0].id=8;bad.push(JSON.stringify(wrong));
 const geometry=response();geometry.geometryOnly=true;bad.push(JSON.stringify(geometry));
 for(const key of ['values','usedProperties']){const invalid=response();invalid.targets[0][key]=null;bad.push(JSON.stringify(invalid));}
 const missing=response();missing.targets=[];bad.push(JSON.stringify(missing));
 for(const value of bad){const f=make();assert.throws(()=>f.s.resolve({resolveStyleStateV1(){return value}},'w','t',f.request))}
});

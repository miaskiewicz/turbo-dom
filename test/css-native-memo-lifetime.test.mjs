import assert from 'node:assert/strict';
import test from 'node:test';
import {nativeMemoFixture,oversizedMemoResponse} from './helpers/css-native-memo-fixture.mjs';
const bytes=value=>Buffer.byteLength(value,'utf8');

test('native warm result accounts exact payload and preserves tightened policy and disposal',()=>{
 const f=nativeMemoFixture();
 try{
  const before=f.prepared.retainedBytes();
  const output=f.prepared.resolveMatchingV1(f.wire,f.token);
  const retained=before+bytes(f.wire)+bytes(output);
  assert.equal(f.prepared.retainedBytes(),retained);
  assert.equal(f.prepared.resolveMatchingV1(f.wire,f.token),output);
  assert.equal(f.prepared.retainedBytes(),retained);
  const strict=JSON.parse(f.wire);strict.limits.retainedResultUnits=0;
  assert.throws(()=>f.prepared.resolveMatchingV1(JSON.stringify(strict),f.token),/ResourceLimit/);
  assert.equal(f.prepared.retainedBytes(),retained);
  assert.equal(f.prepared.resolveMatchingV1(f.wire,f.token),output);
  f.prepared.dispose();assert.equal(f.prepared.retainedBytes(),0);
  assert.throws(()=>f.prepared.resolveMatchingV1(f.wire,f.token),/Disposed/);
 }finally{f.dispose()}
});

test('oversized successful request and response evict the prior native memo',()=>{
 for(const kind of ['request','response']){
  const f=nativeMemoFixture();
  try{
   const before=f.prepared.retainedBytes();
   const original=f.prepared.resolveMatchingV1(f.wire,f.token);
   assert.ok(f.prepared.retainedBytes()>before);
   const wire=kind==='request'?' '.repeat(64*1024)+f.wire:oversizedMemoResponse(f.input);
   const output=f.prepared.resolveMatchingV1(wire,f.token);
   assert.ok(bytes(wire)+bytes(output)>64*1024);
   assert.equal(f.prepared.retainedBytes(),before);
   assert.equal(f.prepared.resolveMatchingV1(wire,f.token),output);
   assert.equal(f.prepared.retainedBytes(),before);
   assert.equal(f.prepared.resolveMatchingV1(f.wire,f.token),original);
   assert.equal(f.prepared.retainedBytes(),before+bytes(f.wire)+bytes(original));
  }finally{f.dispose()}
 }
});

test('failed matching replacement preserves warm memo while successful replacement and foreign tokens refuse reuse',()=>{
 const f=nativeMemoFixture(),other=nativeMemoFixture();
 try{
  const output=f.prepared.resolveMatchingV1(f.wire,f.token),retained=f.prepared.retainedBytes();
  const bad=JSON.parse(JSON.stringify(f.facts));bad.selectorAnswers[0].matches=[999];
  assert.throws(()=>f.prepared.storeMatchingV1(JSON.stringify(bad),JSON.stringify(f.input.limits)),/OutsideDomain/);
  assert.equal(f.prepared.retainedBytes(),retained);
  assert.equal(f.prepared.resolveMatchingV1(f.wire,f.token),output);
  assert.throws(()=>other.prepared.resolveMatchingV1(f.wire,f.token),/StaleMatching/);
  const next=f.prepared.storeMatchingV1(JSON.stringify(f.facts),JSON.stringify(f.input.limits));
  assert.notEqual(next,f.token);assert.ok(f.prepared.retainedBytes()<retained);
  assert.throws(()=>f.prepared.resolveMatchingV1(f.wire,f.token),/StaleMatching/);
  assert.equal(f.prepared.resolveMatchingV1(f.wire,next),output);
 }finally{f.dispose();other.dispose()}
});

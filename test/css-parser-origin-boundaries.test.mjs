import assert from 'node:assert/strict';
import test from 'node:test';
import {
  captureParserOrigin,
  normalizedParserOrigin,
  fieldsMatch,
  parserDecoderSafe,
  originalTextCodecDescriptor,
  originalTextCodecValue,
  originalWasmAvailability,
} from '../src/runtime/parser-origin.mjs';
import {unpack} from '../src/runtime/buffer.mjs';
import {loadNativeCss} from './opt-in-helpers/native.mjs';
const native = loadNativeCss();
const packed = () => native.parseBuffer('<!doctype html><p title="é">real parser</p>');
function replace(owner, key, field, read) {
  const original = Object.getOwnPropertyDescriptor(owner, key);
  let result;
  try { Object.defineProperty(owner, key, {configurable:true, ...field}); result = read(); }
  finally { if (original) Object.defineProperty(owner, key, original); else delete owner[key]; }
  return result;
}
test('parser origin rejects absent fields, non-buffer decoder views and accessor payloads without getters', () => {
  for (const value of [null, undefined, 7, 'buffer']) assert.equal(captureParserOrigin(value, () => true), null);
  const source = packed();
  assert.ok(captureParserOrigin(source, () => true));
  for (const key of ['packed', 'strMeta', 'strBlob']) {
    assert.equal(captureParserOrigin({...source, [key]: {}}, () => true), null);
    const missing = {...source}; delete missing[key];
    assert.equal(captureParserOrigin(missing, () => true), null);
  }
  let calls = 0;
  const accessor = {...source};
  Object.defineProperty(accessor, 'packed', {get() { calls++; return source.packed; }});
  assert.equal(captureParserOrigin(accessor, () => true), null);
  assert.equal(calls, 0);
  const view = packed();
  Object.defineProperty(view.packed, 'byteOffset', {value:0, configurable:true});
  assert.equal(captureParserOrigin(view, () => true), null);
});
test('parser ownership revocation is sticky and normalized buffers retain exact backing identities', () => {
  const source = packed(), raw = unpack(source);
  let valid = true;
  const origin = captureParserOrigin(source, () => valid);
  const normalized = normalizedParserOrigin(raw, origin);
  assert.equal(normalized.length, raw.nodeType.length);
  assert.equal(normalized.valid(), true);
  assert.equal(normalized.matches(), true);
  assert.equal(fieldsMatch({...raw, packed:source.packed}, normalized.fields), false);
  const changed = {...raw, nodeType:new Uint32Array(raw.nodeType)};
  assert.equal(fieldsMatch(changed, normalized.fields), false);
  valid = false;
  assert.equal(origin.valid(), false);
  assert.equal(normalized.valid(), false);
  valid = true;
  assert.equal(origin.valid(), false);
  assert.equal(normalized.valid(), false);
  assert.equal(normalizedParserOrigin(raw, origin), null);
});
test('normalization refuses unknown ownership and cannot retain a replaced field after restoration', () => {
  const source = packed(), raw = unpack(source), origin = captureParserOrigin(source, () => true);
  assert.equal(normalizedParserOrigin(raw, null), null);
  assert.equal(normalizedParserOrigin(raw, {valid:() => false}), null);
  assert.equal(normalizedParserOrigin(raw, {valid:() => true, matches:() => false}), null);
  const incomplete = {...raw}; delete incomplete.nodeType;
  assert.equal(normalizedParserOrigin(incomplete, origin), null);
  const normalized = normalizedParserOrigin(raw, origin), original = raw.nodeType;
  raw.nodeType = new Uint32Array(original);
  assert.equal(normalized.matches(), false);
  raw.nodeType = original;
  assert.equal(normalized.matches(), false);
});
test('parser decoder guards distinguish indexed prototype storage and changed constructor descriptors', () => {
  assert.equal(parserDecoderSafe(), true);
  for (const key of ['', '00', '1a']) {
    assert.equal(replace(Object.prototype, key, {value:'harmless'}, () => parserDecoderSafe()), true);
  }
  assert.equal(replace(Object.prototype, '0', {value:'untrusted indexed storage'}, () => parserDecoderSafe()), false);
  assert.equal(replace(Object.prototype, 'packed', {value:'untrusted backing field'}, () => parserDecoderSafe()), false);
  let calls = 0;
  assert.equal(replace(globalThis, 'Uint8Array', {get() { calls++; return Uint8Array; }}, () => parserDecoderSafe()), false);
  assert.equal(calls, 0);
  assert.equal(parserDecoderSafe(), true);
});
test('original codec and WebAssembly descriptor inspection never executes public accessors', () => {
  assert.equal(originalTextCodecValue('missing', TextEncoder), false);
  assert.equal(originalTextCodecValue('TextEncoder', undefined), false);
  assert.equal(originalTextCodecDescriptor('TextEncoder', undefined), false);
  assert.equal(originalTextCodecDescriptor('TextEncoder', {get:() => TextEncoder}), false);
  assert.equal(originalTextCodecDescriptor('TextEncoder', {value:TextEncoder}), true);
  let calls = 0;
  assert.equal(replace(globalThis, 'WebAssembly', {get() { calls++; throw Error('must not run'); }}, originalWasmAvailability), null);
  assert.equal(calls, 0);
  assert.equal(replace(globalThis, 'WebAssembly', {value:undefined}, originalWasmAvailability), false);
  assert.equal(replace(globalThis, 'WebAssembly', {value:{}}, originalWasmAvailability), null);
  assert.equal(originalWasmAvailability(), true);
});

test('origin field and prototype substitutions revoke safely without evaluating getters', () => {
 const source=packed(), origin=captureParserOrigin(source,()=>true);
 const field=Object.getOwnPropertyDescriptor(source,'n');
 delete source.n;
 assert.equal(origin.matches(),false);
 Object.defineProperty(source,'n',field);
 assert.equal(origin.matches(),false);
 const raw=unpack(packed()), rawOrigin=captureParserOrigin(raw,()=>true);
 assert.equal(rawOrigin.decodeSafe(),true);
 let calls=0;
 assert.equal(replace(raw,'nodeType',{get(){calls++;throw Error('unexpected');}},()=>fieldsMatch(raw,rawOrigin.fields)),false);
 assert.equal(calls,0);
 for(const key of ['packed','strMeta','strBlob']) {
  const input=packed();
  Object.setPrototypeOf(input[key],null);
  assert.equal(captureParserOrigin(input,()=>true),null);
 }
 const input=packed();
 Object.defineProperty(input.strBlob,'subarray',{value:()=>{throw Error('unexpected');}});
 assert.equal(captureParserOrigin(input,()=>true),null);
 const old=Object.getPrototypeOf(Array.prototype);
 let safe;
 try{Object.setPrototypeOf(Array.prototype,null);safe=parserDecoderSafe();}
 finally{Object.setPrototypeOf(Array.prototype,old);}
 assert.equal(safe,false);
 assert.equal(replace(Array.prototype,Symbol.iterator,{get(){return undefined;}},()=>parserDecoderSafe()),true);
 assert.equal(replace(String.prototype,'slice',{get(){calls++;return undefined;}},()=>parserDecoderSafe()),false);
 assert.equal(calls,0);
});
test('decoder guarded accessors and WASM constructor identities cannot be substituted',()=>{
 const typedConstructor=Object.getPrototypeOf(Uint8Array);
 const species=Object.getOwnPropertyDescriptor(typedConstructor,Symbol.species);
 assert.equal(replace(typedConstructor,Symbol.species,{get:species.get,set(){}},()=>parserDecoderSafe()),false);
 assert.equal(replace(typedConstructor,Symbol.species,{get(){return Uint8Array;}},()=>parserDecoderSafe()),false);
 assert.equal(replace(WebAssembly,'Module',{value:function foreign(){}},originalWasmAvailability),null);
 assert.equal(replace(WebAssembly,'Instance',{value:function foreign(){}},originalWasmAvailability),null);
 const old=Object.getOwnPropertyDescriptor(globalThis,'WebAssembly');
 let available;
 try{delete globalThis.WebAssembly;available=originalWasmAvailability();}
 finally{Object.defineProperty(globalThis,'WebAssembly',old);}
 assert.equal(available,false);
});

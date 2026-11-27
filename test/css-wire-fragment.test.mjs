import assert from 'node:assert/strict';
import test from 'node:test';
import { encodeCssWire } from '../src/runtime/css-rust-wire.mjs';

test('large repeated arrays preserve exact scalar output and changed prefix, middle and tail', () => {
 const input=Array.from({length:512},(_,index)=>index);
 for(let repeat=0;repeat<2;repeat++)assert.equal(encodeCssWire(input),JSON.stringify(input));
 for(const index of [0,256,511]){input[index]++;assert.equal(encodeCssWire(input),JSON.stringify(input));assert.equal(encodeCssWire(input),JSON.stringify(input));}
});
test('fragment size boundaries and oversize strings remain valid ordinary encodes', () => {
 for(const length of [511,512,16384,16385]){const input=Array(length).fill(1);assert.equal(encodeCssWire(input),JSON.stringify(input));}
 const small=Array(512).fill('ok');assert.equal(encodeCssWire(small),JSON.stringify(small));
 const huge=Array(512).fill('x'.repeat(512));
 for(let index=0;index<2;index++)assert.equal(encodeCssWire(huge),JSON.stringify(huge));
 assert.equal(encodeCssWire(small),JSON.stringify(small));
});
test('large nested values and exact wire-size cap preserve successful output', () => {
 for(const extra of [0,1]){const input=Array(512).fill('');input[0]='x'.repeat(65536-JSON.stringify(input).length+extra);assert.equal(encodeCssWire(input),JSON.stringify(input));assert.equal(encodeCssWire(input),JSON.stringify(input));}
 const nested=Array(512).fill(null);nested[256]={nested:['x'.repeat(70000)]};assert.equal(encodeCssWire(nested),JSON.stringify(nested));
});
test('array guard failures after a hit neither hide errors nor poison later reads', () => {
 const input=Array(512).fill(1);encodeCssWire(input);encodeCssWire(input);
 delete input[200];assert.throws(()=>encodeCssWire(input),/own-data/);input[200]=1;
 Object.defineProperty(input,'200',{configurable:true,get(){throw Error('getter called');}});assert.throws(()=>encodeCssWire(input),/own-data/);
 Object.defineProperty(input,'200',{configurable:true,writable:true,enumerable:true,value:input});assert.throws(()=>encodeCssWire(input),/cyclic/);
 input[200]=2;assert.equal(encodeCssWire(input),JSON.stringify(input));
});
test('reentrant encodes replace the last entry without changing outer captured values', () => {
 const input=Array(512).fill(3),expected=JSON.stringify(input);let calls=0;
 const proxy=new Proxy(input,{getOwnPropertyDescriptor(target,key){if(key==='length'){calls++;assert.equal(encodeCssWire(Array(512).fill(9)),JSON.stringify(Array(512).fill(9)));target[0]=8;}return Reflect.getOwnPropertyDescriptor(target,key);}});
 assert.equal(encodeCssWire(proxy),expected);assert.equal(calls,1);assert.equal(encodeCssWire(input),JSON.stringify(input));
});

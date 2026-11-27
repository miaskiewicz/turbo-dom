import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment, setParserMode} from '../src/runtime/index.mjs';
import {installGlobals} from '../src/environment/install.mjs';
const html='<style>@layer base{@media(min-width:1px){#parent{--tone:rgb(12,34,56);color:var(--tone);width:200px}#child{color:inherit;width:50%}}}</style><div id="parent"><div id="child"></div></div>';
function values(env){const style=env.window.getComputedStyle(env.document.getElementById('child'));return [style.color,style.width];}
test('CSS backend is explicitly opt-in and independent of the DOM backend',()=>{
 assert.deepEqual(values(createEnvironment(html)),['','']);
 assert.deepEqual(values(createEnvironment(html,{cssBackend:'js'})),['','']);
 assert.deepEqual(values(createEnvironment(html,{cssBackend:'rust',domBackend:'js'})),['rgb(12, 34, 56)','100px']);
 assert.throws(()=>createEnvironment(html,{cssBackend:'native'}),/cssBackend/);
 assert.throws(()=>createEnvironment(html,{cssBackend:'rust',domBackend:'rust'}),/domBackend.*requires/);
 const target={}; const installed=installGlobals(target,{html,cssBackend:'rust'});
 try {assert.deepEqual(values(installed.env),['rgb(12, 34, 56)','100px']);} finally {installed.teardown();}
});
test('explicit Rust CSS refuses genuine artifact absence instead of silently selecting JS',()=>{
 setParserMode('wasm');
 const env=createEnvironment('<style>p{color:red}</style><p>x</p>',{cssBackend:'rust'});
 const original=Object.getOwnPropertyDescriptor(globalThis,'WebAssembly');
 try {
  Object.defineProperty(globalThis,'WebAssembly',{configurable:true,writable:true,value:undefined});
  assert.throws(()=>env.window.getComputedStyle(env.document.querySelector('p')),/capability unavailable: Rust CSS artifact/);
 } finally {Object.defineProperty(globalThis,'WebAssembly',original);setParserMode(null);}
});

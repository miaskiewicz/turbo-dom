import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createEnvironment,setParserMode} from './opt-in-helpers/runtime.mjs';
import {installGlobals} from './opt-in-helpers/install.mjs';
import {bindNativeCssInstalledWindow,unbindNativeCssInstalledWindow,createNativeCssCodecGetter} from '../src/runtime/window.mjs';

test('cold default WASM CSS refuses guest codecs but accepts canonical installed getters',()=>{
 setParserMode('wasm');
 const env=createEnvironment('<style>p{color:red}</style><p>x</p>');
 const target=env.document.querySelector('p'),prior={};let callbacks=0;
 for(const name of ['TextEncoder','TextDecoder'])prior[name]=Object.getOwnPropertyDescriptor(globalThis,name);
 try{
  for(const name of ['TextEncoder','TextDecoder'])Object.defineProperty(globalThis,name,{configurable:true,get(){callbacks++;throw Error('guest codec getter');}});
  assert.throws(()=>env.window.getComputedStyle(target),/WASM TextEncoder/);
  assert.equal(callbacks,0);
 }finally{for(const name of ['TextEncoder','TextDecoder'])Object.defineProperty(globalThis,name,prior[name]);}

 const slice=Object.getOwnPropertyDescriptor(String.prototype,'slice');
 let dependencyFailure;
 try{Object.defineProperty(String.prototype,'slice',{configurable:true,value(){throw Error('untrusted slice');}});env.window.getComputedStyle(target);}
 catch(error){dependencyFailure=error;}
 finally{Object.defineProperty(String.prototype,'slice',slice);}
 assert.match(dependencyFailure?.message??'',/WASM codec dependencies/);
 const wasm=globalThis.WebAssembly;
 for(const [owner,name] of [[globalThis,'WebAssembly'],[wasm,'Module'],[wasm,'Instance']]){
  const field=Object.getOwnPropertyDescriptor(owner,name);
  try{Object.defineProperty(owner,name,{configurable:true,get(){callbacks++;throw Error('guest WASM getter');}});assert.throws(()=>env.window.getComputedStyle(target),/WASM bootstrap/);assert.equal(callbacks,0);}
  finally{Object.defineProperty(owner,name,field);}
 }
 let traps=0;const foreign=new Proxy(env.window,{get(){traps++;throw Error('window get');}});
 assert.throws(()=>createNativeCssCodecGetter(env.document,foreign,'TextEncoder'),/foreign codec getter/);assert.equal(traps,0);
 const encoder=Object.getOwnPropertyDescriptor(globalThis,'TextEncoder');
 const forged=()=>{callbacks++;throw Error('forged getter');};
 try{
  Object.defineProperty(globalThis,'TextEncoder',{configurable:true,get:forged});
  bindNativeCssInstalledWindow(env.document,globalThis,{TextEncoder:forged});
  assert.throws(()=>env.window.getComputedStyle(target),/WASM TextEncoder/);assert.equal(callbacks,0);
 }finally{unbindNativeCssInstalledWindow(env.document,globalThis);Object.defineProperty(globalThis,'TextEncoder',encoder);}
 const wasmField=Object.getOwnPropertyDescriptor(globalThis,'WebAssembly');
 try{Object.defineProperty(globalThis,'WebAssembly',{configurable:true,writable:true,value:undefined});assert.throws(()=>env.window.getComputedStyle(target),/capability unavailable: Rust CSS artifact/);}
 finally{Object.defineProperty(globalThis,'WebAssembly',wasmField);setParserMode('auto');setParserMode('wasm');}
 const installed=installGlobals(globalThis,{html:'<style>p{color:red}</style><p>x</p>'});
 try{assert.equal(globalThis.getComputedStyle(installed.env.document.querySelector('p')).color,'rgb(255, 0, 0)');}
 finally{installed.teardown();}
 assert.deepEqual(Object.keys(createRequire(import.meta.url).cache).filter(path=>path.endsWith('.node')),[]);
});

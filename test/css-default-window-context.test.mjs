import test from 'node:test';import assert from 'node:assert/strict';
import{installGlobals}from'./opt-in-helpers/install.mjs';
test('canonical installed own-data viewport changes invalidate media without DOM mutation or guest getter',()=>{
 const{env,teardown}=installGlobals(globalThis);const{document}=env;
 try{
  const style=document.createElement('style');style.textContent='@media(min-width:900px){.x{color:red}}@media(max-width:899px){.x{color:blue}}';document.head.append(style);const node=document.createElement('b');node.className='x';document.body.append(node);
  const before=document.__version;assert.equal(globalThis.getComputedStyle(node).color,'rgb(255, 0, 0)');
  Object.defineProperty(globalThis,'innerWidth',{configurable:true,writable:true,value:800});assert.equal(document.__version,before);assert.equal(globalThis.getComputedStyle(node).color,'rgb(0, 0, 255)');
  let calls=0;Object.defineProperty(globalThis,'innerWidth',{configurable:true,get(){calls++;throw Error('getter must not run');}});assert.throws(()=>globalThis.getComputedStyle(node),/accessor unavailable/);assert.equal(calls,0);
 }finally{teardown();}
});

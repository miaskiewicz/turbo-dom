import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';

test('default CSS retains empty-view API for null and genuine disconnected elements',()=>{
 const {window,document}=createEnvironment('<div id="connected" style="color:red"></div>');
 for(const target of [null,document.createElement('div')]){
  const style=window.getComputedStyle(target);
  assert.equal(style.color,'');assert.equal(style.length,0);
  assert.equal(style.item(0),'');assert.equal(style.getPropertyPriority('color'),'');
  assert.deepEqual([...style],[]);
 }
 assert.equal(window.getComputedStyle(document.getElementById('connected')).color,'rgb(255, 0, 0)');
 let traps=0;const foreign=new Proxy(document.createElement('div'),{get(){traps++;throw Error('get');},getPrototypeOf(){traps++;throw Error('prototype');}});
 assert.throws(()=>window.getComputedStyle(foreign),/foreign element/);assert.equal(traps,0);
});

test('registered custom prototypes admit native CSS without reflecting proxy ancestors',()=>{
 const {window,document}=createEnvironment('<style>x-safe{color:red}</style><x-safe></x-safe>');
 class Safe extends window.HTMLElement{}
 window.customElements.define('x-safe',Safe);
 const element=document.querySelector('x-safe');
 assert.equal(window.getComputedStyle(element).color,'rgb(255, 0, 0)');
 const original=Object.getPrototypeOf(Safe.prototype);let traps=0;
 Object.setPrototypeOf(Safe.prototype,new Proxy(original,{getPrototypeOf(){traps++;throw Error('prototype');},getOwnPropertyDescriptor(){traps++;throw Error('descriptor');}}));
 try{assert.throws(()=>window.getComputedStyle(element),/unknown native prototype/);assert.equal(traps,0);}
 finally{Object.setPrototypeOf(Safe.prototype,original);}
});

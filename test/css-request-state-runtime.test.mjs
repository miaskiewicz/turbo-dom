import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from '../src/runtime/index.mjs';
import {loadNativeCss} from './opt-in-helpers/native.mjs';
const native=loadNativeCss(),prototype=native.PreparedStylesV1.prototype;
function wrap(name,callback){const field=Object.getOwnPropertyDescriptor(prototype,name);Object.defineProperty(prototype,name,{...field,value:callback(field.value)});return()=>Object.defineProperty(prototype,name,field);}
function environment(){return createEnvironment('<style>div{color:red}</style><div id="target">x</div><div id="other">y</div>',{cssBackend:'rust',cssRequestProtocol:'context-template-v1'});}
test('explicit request-state executes full then compact native calls with fresh mutations',()=>{
 let full=0,compact=0;
 const undoFull=wrap('resolveRequestTemplateV1',original=>function(...args){full++;return Reflect.apply(original,this,args);});
 const undoCompact=wrap('resolveTemplateV1',original=>function(...args){compact++;return Reflect.apply(original,this,args);});
 try{
  const{document,window}=environment(),target=document.querySelector('#target');
  assert.equal(window.getComputedStyle(target).color,'rgb(255, 0, 0)');
  assert.equal(window.getComputedStyle(target).color,'rgb(255, 0, 0)');
  assert.equal(full,1);assert.equal(compact,0);
  assert.equal(window.getComputedStyle(document.querySelector('#other')).color,'rgb(255, 0, 0)');
  assert.equal(compact,1);
  target.style.color='blue';
  assert.equal(window.getComputedStyle(target).color,'rgb(0, 0, 255)');
  window.innerWidth=900;
  assert.equal(window.getComputedStyle(target).color,'rgb(0, 0, 255)');
  assert.ok(full>=2);
 }finally{undoCompact();undoFull();}
});
test('explicit request-state invokes current callbacks and preserves their original errors',()=>{
 let calls=0;const sentinel=new Error('current compact callback');
 const undo=wrap('resolveMatchingV1',original=>function(...args){calls++;if(calls===2)throw sentinel;return Reflect.apply(original,this,args);});
 try{const{window,document}=environment(),target=document.querySelector('#target');
  assert.equal(window.getComputedStyle(target).color,'rgb(255, 0, 0)');
  assert.equal(window.getComputedStyle(target).color,'rgb(255, 0, 0)');
  assert.throws(()=>window.getComputedStyle(target),error=>error===sentinel);assert.equal(calls,2);
  assert.equal(window.getComputedStyle(target).color,'rgb(255, 0, 0)');
 }finally{undo();}
});
test('explicit request-state refuses missing native capability without changing ordinary rust mode',()=>{
 const field=Object.getOwnPropertyDescriptor(prototype,'resolveTemplateV1');
 Object.defineProperty(prototype,'resolveTemplateV1',{...field,value:undefined});
 try{const{window,document}=environment();assert.throws(()=>window.getComputedStyle(document.querySelector('#target')),/context-template-v1 capability unavailable/);
  const ordinary=createEnvironment('<style>div{color:red}</style><div>x</div>',{cssBackend:'rust'});assert.equal(ordinary.window.getComputedStyle(ordinary.document.querySelector('div')).color,'rgb(255, 0, 0)');
 }finally{Object.defineProperty(prototype,'resolveTemplateV1',field);}
});
test('selected template requests refuse absent matching without silently using full resolve',()=>{
 const field=Object.getOwnPropertyDescriptor(prototype,'storeMatchingV1');
 Object.defineProperty(prototype,'storeMatchingV1',{...field,value:undefined});
 try{
  const{window,document}=environment();
  assert.throws(()=>window.getComputedStyle(document.querySelector('#target')),/context-template-v1 requires retained matching capability/);
  const ordinary=createEnvironment('<style>div{color:red}</style><div>x</div>',{cssBackend:'rust'});
  assert.equal(ordinary.window.getComputedStyle(ordinary.document.querySelector('div')).color,'rgb(255, 0, 0)');
 }finally{Object.defineProperty(prototype,'storeMatchingV1',field);}
});

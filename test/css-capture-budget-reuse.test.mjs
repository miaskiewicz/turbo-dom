import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {captureCssFactsBudget,createCssFactsNavigatorFromBudget,reuseCssFactsBudget} from '../src/runtime/css-facts-navigator.mjs';
const snapshotLimits={maxNodes:128,maxDepth:64,maxPathUnits:4096,maxTextUnits:4096,maxAttributes:128,maxAttributeUnits:4096};
const limits={maxNodes:128,maxEdges:4096,maxTextUnits:16384,maxAttributeUnits:4096};
const names=['maxNodes','maxEdges','maxAttributeUnits','maxTextUnits'];
function fixture(){const {document}=createEnvironment('<body><div id="x" title="old">text</div>');return {document,capture:createDefaultCssCapture()};}
function read(capture,document,policy=limits){return capture.selectorFacts(capture.capture(document,snapshotLimits),policy);}
test('exact captured inputs reuse only with equal fresh quotas and preserve quota proxy order',()=>{
 const {document,capture}=fixture(),first=read(capture,document),seen=[];
 const proxy=new Proxy(limits,{getOwnPropertyDescriptor(target,key){seen.push(key);return Reflect.getOwnPropertyDescriptor(target,key);}});
 assert.equal(read(capture,document,proxy),first);assert.deepEqual(seen,names);
 seen.length=0;document.getElementById('x').setAttribute('title','new');
 assert.notEqual(read(capture,document,proxy),first);assert.deepEqual(seen,names);
 const larger=read(capture,document,{...limits,maxNodes:129});assert.equal(read(capture,document,{...limits,maxNodes:129}),larger);
 assert.notEqual(read(capture,document,limits),larger);
 for(const name of names)assert.throws(()=>read(capture,document,{...limits,[name]:0}),/limit/);
});
test('quota accessors and proxy exceptions remain observable even on exact navigator reuse',()=>{
 const {document,capture}=fixture();read(capture,document);let called=0;
 const policy={...limits};Object.defineProperty(policy,'maxEdges',{get(){called++;return 4096;}});
 assert.throws(()=>read(capture,document,policy),/accessor maxEdges/);assert.equal(called,0);
 const sentinel=new Error('quota proxy');const seen=[];
 const proxy=new Proxy(limits,{getOwnPropertyDescriptor(target,key){seen.push(key);if(key==='maxAttributeUnits')throw sentinel;return Reflect.getOwnPropertyDescriptor(target,key);}});
 assert.throws(()=>read(capture,document,proxy),error=>error===sentinel);assert.deepEqual(seen,names.slice(0,3));
});
test('fresh raw attribute and text writes invalidate selector reuse without relying on version',()=>{
 const {document,capture}=fixture(),element=document.getElementById('x');element.setAttribute('title','old');
 const first=read(capture,document),version=document.__version;
 element.__attrs.find(row=>row.name==='title').value='raw';assert.equal(document.__version,version);
 const second=read(capture,document);assert.notEqual(second,first);assert.equal(read(capture,document),second);
 element.firstChild._data='other';assert.equal(document.__version,version);assert.notEqual(read(capture,document),second);
});
test('opaque quota receipts reject forgery and can be consumed only once',()=>{
 assert.throws(()=>createCssFactsNavigatorFromBudget({},{}),/unadmitted budget/);
 const token=captureCssFactsBudget(limits);assert.equal(reuseCssFactsBudget(token,{}),false);
 assert.throws(()=>createCssFactsNavigatorFromBudget({},token),/documentId/);
 assert.throws(()=>createCssFactsNavigatorFromBudget({},token),/unadmitted budget/);
});

test('late public WeakMap hooks cannot expose captured input or the navigator reuse receipt',()=>{
 const {document,capture}=fixture(),first=read(capture,document),snapshot=capture.capture(document,snapshotLimits);
 const original=WeakMap.prototype.get;let calls=0,result;
 try{WeakMap.prototype.get=function(key){calls++;return Reflect.apply(original,this,[key]);};result=capture.selectorFacts(snapshot,limits);}
 finally{WeakMap.prototype.get=original;}
 assert.equal(result,first);assert.equal(calls,0);
});

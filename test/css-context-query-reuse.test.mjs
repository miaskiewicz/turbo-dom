import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from '../src/runtime/index.mjs';
import {compileSharedShadowFacts} from '../src/runtime/css-shared-shadow-facts.mjs';
import {createTestPublicCssFacts} from './helpers/css-shared-public-facts.mjs';
import {captureCssPrimitivePublicFacts} from './helpers/css-primitive-public-facts.mjs';
import {createCssFactsNavigator} from '../src/runtime/css-facts-navigator.mjs';

const limits={maxContexts:1,maxNodes:32,maxAncestorDepth:16,maxAnswers:256,maxDomainUnits:1024,routing:{maxRoots:1,maxNodes:32,maxPathUnits:256,maxScopeDepth:8}};
function fixture() {
 const {document}=createEnvironment('<b class="x"></b>');
 const item={index:0,units:[46,120],pseudo:null,implicitScopeAnchor:false,referencesScope:false,seed:{kind:'class',valueUnits:[120]}};
 const metadata={selectors:[{id:0,items:[item]}],groups:[]};
 const program=compileSharedShadowFacts(metadata,true);
 function collect(options=limits, current=program) {
  const snapshot=createTestPublicCssFacts().capture(document,{maxNodes:32,maxDepth:16,maxPathUnits:256,maxTextUnits:256,maxAttributes:32,maxAttributeUnits:256});
  const facts=captureCssPrimitivePublicFacts(document,{idOf:snapshot.nodeId,focusTarget:null});
  const navigator=createCssFactsNavigator(facts,{maxNodes:32,maxEdges:128,maxTextUnits:4096,maxAttributeUnits:4096});
  return current.collect(snapshot,document,options,undefined,navigator,true);
 }
 return {document,item,metadata,collect};
}
test('unscoped contextual query reuse keeps fresh matches, scope domains and limits',()=>{
 const f=fixture();
 const first=f.collect();
 assert.equal(first.contextAnswers.selectorAnswers.length,1);
 assert.equal(first.contextAnswers.selectorAnswers[0].matches.length,1);
 assert.deepEqual(f.collect(),first);
 f.document.querySelector('b').className='y';
 assert.deepEqual(f.collect().contextAnswers.selectorAnswers[0].matches,[]);
 f.item.referencesScope=true;
 const scoped=f.collect();
 assert.ok(scoped.contextAnswers.selectorAnswers.length>1);
 assert.deepEqual(scoped,f.collect(limits,compileSharedShadowFacts(f.metadata,true)));
 f.item.referencesScope=false;
 assert.equal(f.collect().contextAnswers.selectorAnswers.length,1);
 assert.throws(()=>f.collect({...limits,maxAnswers:0}),/limit exceeded/);
});
test('reused contextual rows still read live scope flags and propagate their errors',()=>{
 const f=fixture();let implicit=0,reference=0,fail=false;
 const original=new Error('scope flag failed');
 Object.defineProperty(f.item,'implicitScopeAnchor',{get(){implicit++;return false;}});
 Object.defineProperty(f.item,'referencesScope',{get(){reference++;if(fail)throw original;return false;}});
 f.collect();f.collect();
 assert.equal(implicit,2);assert.equal(reference,2);
 fail=true;assert.throws(()=>f.collect(),error=>error===original);
 assert.equal(implicit,3);assert.equal(reference,3);
});
test('refused dependency callouts cannot corrupt a retained query row',()=>{
 const f=fixture();const push=Array.prototype.push;let captured,poison=true;
 Object.defineProperty(f.item,'referencesScope',{get(){
  if(poison)Array.prototype.push=function(...values){
   if(values[0]?.selectorId===0)captured=values[0];
   return Reflect.apply(push,this,values);
  };
  return false;
 }});
 let failure;
 try{f.collect();}catch(error){failure=error;}finally{Array.prototype.push=push;}
 assert.match(failure.message,/array dependency/);
 assert.ok(captured);
 Reflect.set(captured,'selectorId',999);
 poison=false;
 const answer=f.collect();
 assert.equal(answer.contextAnswers.selectorAnswers[0].selectorId,0);
 assert.equal(answer.contextAnswers.selectorAnswers[0].matches.length,1);
});
test('cached query protection preserves the original collection callback error',()=>{
 const f=fixture();const push=Array.prototype.push,original=new Error('original push failure');let poison=true;
 Object.defineProperty(f.item,'referencesScope',{get(){
  if(poison)Array.prototype.push=function(...values){
   if(values[0]?.selectorId===0)throw original;
   return Reflect.apply(push,this,values);
  };
  return false;
 }});
 try{assert.throws(()=>f.collect(),error=>error===original);}finally{Array.prototype.push=push;}
 poison=false;
 assert.equal(f.collect().contextAnswers.selectorAnswers[0].matches.length,1);
});

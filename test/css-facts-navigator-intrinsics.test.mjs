import test from 'node:test';
import assert from 'node:assert/strict';
import {createCssFactsNavigator as create,readCssFactsNavigator as read,CssFactsUnavailable} from '../src/runtime/css-facts-navigator.mjs';
const limits={maxNodes:10,maxEdges:10,maxAttributeUnits:100,maxTextUnits:100};
const source=()=>({documentId:0,documentElementId:1,nodes:[{id:0,nodeType:9,parentId:null,ownerDocumentId:0,childIds:[1]},{id:1,nodeType:1,parentId:0,ownerDocumentId:0,childIds:[],attributes:[],state:{checked:undefined},localName:'p',namespaceURI:null}]});
test('captured collections and owned-data construction avoid poisoned public callbacks',()=>{
 const saved=[];const poison=()=>{throw new Error('public collection callback');};
 const replace=(obj,key,value)=>{saved.push([obj,key,Object.getOwnPropertyDescriptor(obj,key)]);Object.defineProperty(obj,key,{value,writable:true,configurable:true});};
 const input=source();
 try {
  replace(Array.prototype,Symbol.iterator,poison);replace(Array.prototype,'includes',poison);replace(Array.prototype,'indexOf',poison);
  replace(Map.prototype,'get',poison);replace(Map.prototype,'set',poison);replace(Map.prototype,'has',poison);replace(Map.prototype,'values',poison);
  replace(WeakMap.prototype,'get',poison);replace(WeakMap.prototype,'set',poison);
  const operations=read(create(input,limits));
  if(operations.previous(1)!==null||operations.next(1)!==null||operations.state(1,'checked')!==undefined)throw new Error('wrong owned result');
 }finally{for(let i=saved.length-1;i>=0;i--){const entry=saved[i];Object.defineProperty(entry[0],entry[1],entry[2]);}}
});
test('u32 identity, graph cycles, original reflection errors and opaque tokens',()=>{
 const input=source();input.nodes[1].id=0x100000000;assert.throws(()=>create(input,limits),CssFactsUnavailable);
 const cycle=source();cycle.nodes[0].parentId=1;cycle.nodes[1].childIds=[0];assert.throws(()=>create(cycle,limits),/cyclic/);
 const error=new Error('reflection');assert.throws(()=>create(new Proxy({},{getOwnPropertyDescriptor(){throw error;}}),limits),e=>e===error);
 let traps=0;const foreign=new Proxy({},{get(){traps++;throw error;},getOwnPropertyDescriptor(){traps++;throw error;}});assert.throws(()=>read(foreign),CssFactsUnavailable);assert.equal(traps,0);
 assert.throws(()=>create(source(),{...limits,maxNodes:0}),/limit/);
});

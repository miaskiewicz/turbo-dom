import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {readCssFactsNavigator} from '../src/runtime/css-facts-navigator.mjs';
const snapshotLimits={maxNodes:128,maxDepth:64,maxPathUnits:4096,maxTextUnits:4096,maxAttributes:128,maxAttributeUnits:4096};
const limits={maxNodes:128,maxEdges:4096,maxTextUnits:16384,maxAttributeUnits:4096};
function fixture(){const {document}=createEnvironment('<body><div id="x">text</div>');return {document,capture:createDefaultCssCapture()};}
function hook(name,action){Object.defineProperty(Object.prototype,name,{configurable:true,set(value){delete Object.prototype[name];Object.defineProperty(this,name,{value,writable:true,enumerable:true,configurable:true});action(this);}});}
test('capture preserves transient selector-row setter errors',()=>{
 const {document,capture}=fixture(),sentinel=new Error('row setter');let calls=0;
 try{hook('namespaceURI',()=>{calls++;throw sentinel;});assert.throws(()=>capture.capture(document,snapshotLimits),error=>error===sentinel);}
 finally{delete Object.prototype.namespaceURI;}
 assert.equal(calls,1);
});
test('escaped previous selector rows still invalidate otherwise equal fresh input',()=>{
 const {document,capture}=fixture();let escaped;
 try{hook('namespaceURI',row=>{escaped=row;});const snapshot=capture.capture(document,snapshotLimits);const first=capture.selectorFacts(snapshot,limits);
 assert.ok(escaped);assert.equal(readCssFactsNavigator(first).property(escaped.id,'namespaceURI'),escaped.namespaceURI);
 escaped.namespaceURI='urn:late-mutation';
 const next=capture.selectorFacts(capture.capture(document,snapshotLimits),limits);
 assert.notEqual(next,first);assert.notEqual(readCssFactsNavigator(next).property(escaped.id,'namespaceURI'),escaped.namespaceURI);
 }finally{delete Object.prototype.namespaceURI;}
});
test('late limit read mutates an earlier escaped row before the final equality check',()=>{
 const {document,capture}=fixture();const first=capture.selectorFacts(capture.capture(document,snapshotLimits),limits);let escaped,mutated=false;
 const policy=new Proxy(snapshotLimits,{get(target,key){if(key==='maxDepth'&&escaped){escaped.namespaceURI='urn:late-finalization';mutated=true;}return Reflect.get(target,key);}});
 try{hook('namespaceURI',row=>{escaped=row;});const snapshot=capture.capture(document,policy);const next=capture.selectorFacts(snapshot,limits);
 assert.equal(mutated,true);assert.notEqual(next,first);assert.equal(readCssFactsNavigator(next).property(escaped.id,'namespaceURI'),'urn:late-finalization');
 }finally{delete Object.prototype.namespaceURI;}
});

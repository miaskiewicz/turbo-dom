import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {installGlobals} from './opt-in-helpers/install.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {ensureCssState} from '../src/runtime/css-invalidation.mjs';
import {compileCssFactsProgram,matchesCssFacts} from '../src/runtime/selectors.mjs';
const require=createRequire(import.meta.url);
const limits={maxNodes:128,maxDepth:64,maxPathUnits:4096,maxTextUnits:4096,maxAttributes:128,maxAttributeUnits:4096};
const selectorLimits={maxNodes:128,maxEdges:4096,maxTextUnits:16384,maxAttributeUnits:4096};
test('authentic React controlled input accessor does not replace native CSS checked state',()=>{
 const{env,teardown}=installGlobals(globalThis);const previous=globalThis.IS_REACT_ACT_ENVIRONMENT;globalThis.IS_REACT_ACT_ENVIRONMENT=true;
 const React=require('react'),{render,cleanup}=require('@testing-library/react');
 try{
  render(React.createElement('input',{type:'checkbox',checked:true,onChange(){},'data-testid':'controlled'}));
  const input=env.document.querySelector('input'),property=Object.getOwnPropertyDescriptor(input,'checked');assert.equal(typeof property.get,'function');
  ensureCssState(env.document);const capture=createDefaultCssCapture(),snapshot=capture.capture(env.document,limits),nav=capture.selectorFacts(snapshot,selectorLimits);
  assert.equal(matchesCssFacts(snapshot.nodeId(input),compileCssFactsProgram(':checked'),{},nav),true);
  let calls=0;Object.defineProperty(input,'checked',{configurable:true,get(){calls++;throw Error('author getter must not run');}});
  const next=capture.capture(env.document,limits);assert.equal(matchesCssFacts(next.nodeId(input),compileCssFactsProgram(':checked'),{},capture.selectorFacts(next,selectorLimits)),true);assert.equal(calls,0);Object.defineProperty(input,'checked',property);
 }finally{cleanup();teardown();if(previous===undefined)delete globalThis.IS_REACT_ACT_ENVIRONMENT;else globalThis.IS_REACT_ACT_ENVIRONMENT=previous;}
});

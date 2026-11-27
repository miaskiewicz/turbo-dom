import test from 'node:test';
import {loadRealCssBackend} from './helpers/css-shared-real.mjs';
import {runOriginalControls} from './helpers/css-shared-corpus-controls.mjs';
import {runInheritanceControls} from './helpers/css-shared-inheritance-controls.mjs';
import {configurePrimitiveReal} from './helpers/css-primitive-real-controls.mjs';
test('actual native: primitive facts preserve original18 and published API15', async()=>runOriginalControls(configurePrimitiveReal(await loadRealCssBackend('native'))));
test('actual native: primitive facts preserve computed inheritance and lazy Scene4', async()=>runInheritanceControls(configurePrimitiveReal(await loadRealCssBackend('native'))));
import assert from 'node:assert/strict';
import {createRealCssFixture} from './helpers/css-shared-real.mjs';
import {CssFactsUnavailable} from '../src/runtime/css-facts-navigator.mjs';
test('private selector hook preserves injected errors and refuses unknown Proxy identity without reflection',async()=>{
 const real=configurePrimitiveReal(await loadRealCssBackend('native'));
 const original=new Error('original selector capture failure');
 for(const [callback,expected] of [[()=>{throw original;},error=>error===original],[()=>new Proxy({},{get(){throw Error('navigator trap');}}),error=>error instanceof CssFactsUnavailable],[()=>undefined,error=>error instanceof CssFactsUnavailable]]){
  const fixture=createRealCssFixture(real,'<style>.x{color:red}</style><b class=x></b>',{captureSelectorFacts:callback});
  try{assert.throws(()=>fixture.adapter.get(fixture.doc.querySelector('.x')),expected);}finally{fixture.dispose();}
 }
});
test('actual primitive adapter routes open shadow host and genuine assigned slot colors',async()=>{
 const real=configurePrimitiveReal(await loadRealCssBackend('native'));
 const fixture=createRealCssFixture(real,'<div id="host"><b class="x"></b></div>');
 const host=fixture.doc.querySelector('#host'),light=fixture.doc.querySelector('.x');host.attachShadow({mode:'open'}).innerHTML='<style>:host{color:blue}slot::slotted(.x){background-color:red}</style><slot></slot>';
 try{assert.equal(fixture.adapter.get(host).color,'rgb(0, 0, 255)');assert.equal(fixture.adapter.get(light).backgroundColor,'rgb(255, 0, 0)');assert.equal(fixture.counts().sceneCalls,0);}finally{fixture.dispose();}
});
import {compileSharedShadowFacts} from '../src/runtime/css-shared-shadow-facts.mjs';
function actualMetadata(backend, source) {
 const n=source.length,compiled=new backend.CompiledStylesV1(JSON.stringify({abiVersion:1,sources:[{id:1,text:Array.from(source,value=>value.charCodeAt(0))}],limits:{sources:1,sourceUnits:n,rules:n,sourceRulePreludes:n,selectors:n,declarations:n,properties:n,groupDepth:32,tokenNesting:32}}));
 try{return JSON.parse(compiled.metadata());}finally{compiled.dispose();}
}
test('canonical actual scope metadata distinguishes absent certificate and unsupported composed boundary',async()=>{
 const {backend}=await loadRealCssBackend('native');
 for(const selector of [':host-context(.x)','::part(label)']){
  const metadata=actualMetadata(backend,`@scope (${selector}){b{color:red}}`);
  assert.throws(()=>compileSharedShadowFacts(metadata,true),/unsupported composed scope boundary/);
 }
 const metadata=actualMetadata(backend,'@scope (:scope){b{color:red}}');
 const scope=metadata.groups.find(row=>row.scope).scope;
 delete scope.startItems[0].policy;
 assert.throws(()=>compileSharedShadowFacts(metadata,true),/scope boundary policy unavailable/);
});
import {createTestPublicCssFacts} from './helpers/css-shared-public-facts.mjs';
import {createEnvironment} from '../src/runtime/index.mjs';
import {captureCssPrimitivePublicFacts} from './helpers/css-primitive-public-facts.mjs';
import {createCssFactsNavigator} from '../src/runtime/css-facts-navigator.mjs';
test('contextual primitive fact quotas refuse incomplete answers on genuine public snapshot',async()=>{
 const {backend}=await loadRealCssBackend('native'),program=compileSharedShadowFacts(actualMetadata(backend,'.x{color:red}'),true);
 const {document}=createEnvironment('<b class="x"></b>'),snapshot=createTestPublicCssFacts().capture(document,{maxNodes:32,maxDepth:16,maxPathUnits:256,maxTextUnits:256,maxAttributes:32,maxAttributeUnits:256});
 const facts=captureCssPrimitivePublicFacts(document,{idOf:snapshot.nodeId,focusTarget:null}),inputUnits=JSON.stringify(facts).length;
 const nav=createCssFactsNavigator(facts,{maxNodes:32,maxEdges:128,maxTextUnits:inputUnits,maxAttributeUnits:inputUnits});
 const limits={maxContexts:1,maxNodes:32,maxAncestorDepth:16,maxAnswers:256,maxDomainUnits:256,routing:{maxRoots:1,maxNodes:32,maxPathUnits:256,maxScopeDepth:8}};
 for(const name of ['maxContexts','maxAnswers','maxDomainUnits'])assert.throws(()=>program.collect(snapshot,document,{...limits,[name]:0},undefined,nav),/limit exceeded/);
 assert.throws(()=>program.collect(snapshot,document,{...limits,maxContexts:-1},undefined,nav),TypeError);
});

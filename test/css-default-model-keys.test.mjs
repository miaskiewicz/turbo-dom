import assert from 'node:assert/strict';
import test from 'node:test';
import {createEnvironment, setParser, setParserMode} from './opt-in-helpers/runtime.mjs';
import {loadNativeCss} from './opt-in-helpers/native.mjs';
import {CSSStyleSheet} from '../src/runtime/cssom.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {createDefaultCssProfile} from '../src/runtime/css-default-profile.mjs';
import {createDefaultSharedCssAdapter} from '../src/runtime/css-shared-adapter.mjs';
const native=loadNativeCss();
const red='rgb(255, 0, 0)', blue='rgb(0, 0, 255)';
function install(overrides){setParser({...native,...overrides,cssBackendVersion:'0.8.7'});}
function restore(){setParser(null);setParserMode('native');}

test('private model keys retain live raw sources, media, CSSOM, adoption and viewport inputs',()=>{
 const {document,window}=createEnvironment('<style>p{color:red}@media(max-width:600px){p{color:green}}</style><p>value</p>');
 const target=document.querySelector('p'),style=document.querySelector('style');
 const read=()=>window.getComputedStyle(target).color;
 assert.equal(read(),red);assert.equal(read(),red);
 style.firstChild._data='p{color:blue}@media(max-width:600px){p{color:green}}';assert.equal(read(),blue);
 style.setAttribute('media','print');assert.notEqual(read(),blue);
 style.setAttribute('media','screen');assert.equal(read(),blue);
 window.innerWidth=400;assert.equal(read(),'rgb(0, 128, 0)');
 window.innerWidth=800;assert.equal(read(),blue);
 const sheet=new CSSStyleSheet();sheet.insertRule('p{color:red}');document.adoptedStyleSheets=[sheet];assert.equal(read(),red);
 sheet.cssRules[0].cssText='p{color:blue}';assert.equal(read(),blue);
 document.adoptedStyleSheets=[];assert.equal(read(),blue);
});

test('object-returning policy aliases remain mutable and descriptor refusals do not become cached hits',()=>{
 const {document,window}=createEnvironment('<style>p{color:red}</style><p>value</p>');
 const target=document.querySelector('p'),policy=JSON.parse(native.cssRuntimePolicyV1());
 let reflections=0,getters=0;
 const compile=policy.adapter.compile;
 policy.adapter.compile=new Proxy(compile,{ownKeys(value){reflections++;return Reflect.ownKeys(value);}});
 try{
  install({cssRuntimePolicyV1:()=>policy});
  assert.equal(window.getComputedStyle(target).color,red);const first=reflections;
  assert.equal(window.getComputedStyle(target).color,red);assert.ok(reflections>first);
  assert.equal(Object.isFrozen(compile),false);
  const original=compile.rules;compile.rules=0;
  assert.throws(()=>window.getComputedStyle(target),/CSSBackendCompile:RuleCount/);
  compile.rules=original;
  Object.defineProperty(compile,'rules',{configurable:true,enumerable:true,get(){getters++;throw Error('must not execute');}});
  assert.throws(()=>window.getComputedStyle(target),/own-data/);assert.equal(getters,0);
  Object.defineProperty(compile,'rules',{configurable:true,enumerable:true,writable:true,value:original});
  assert.equal(window.getComputedStyle(target).color,red);
 }finally{restore();}
});

test('object-returning preferences cannot make aliased UA text immutable or stale',()=>{
 const {document,window}=createEnvironment('<style>p{color:blue}</style><p>value</p>');
 const target=document.querySelector('p');let preferences,compiles=0;const requests=[];
 class Styles extends native.CompiledStylesV1{constructor(wire){super(wire);compiles++;requests.push(JSON.parse(wire));}}
 try{
  install({CompiledStylesV1:Styles,cssComputedPreferencesV1(input){preferences=JSON.parse(native.cssComputedPreferencesV1(input));return preferences;}});
  window.getComputedStyle(target).color;assert.equal(compiles,1);
  const units=preferences.userAgentSource.text;
  assert.equal(Object.isFrozen(units),false);
  for(const char of 'p{color:red}')units.push(char.charCodeAt(0));
  assert.equal(window.getComputedStyle(target).color,blue);assert.equal(compiles,2);
  assert.deepEqual(requests[1].sources[0].text.slice(-12),Array.from('p{color:red}',c=>c.charCodeAt(0)));
 }finally{restore();}
});

test('generic default-profile callers still execute compile quota callbacks and preserve their failures',()=>{
 const policy=JSON.parse(native.cssRuntimePolicyV1());let compileCalls=0,failure;
 const adapters=Object.fromEntries(Object.entries(policy.adapter).map(([name,value])=>[name,()=>value]));
 adapters.compile=(input)=>{if(input.length){compileCalls++;if(failure)throw failure;}return policy.adapter.compile;};
 const resourcePolicy={...Object.fromEntries(['preferences','environment','snapshot','selectorFacts','scene','composition'].map(name=>[name,()=>policy[name]])),adapter:adapters,fonts:policy.fonts};
 const options=createDefaultCssProfile({backend:native,capture:createDefaultCssCapture(),resourcePolicy,readWindowContext:()=>({hostPlatform:'other',width:800,height:600,devicePixelRatio:1,mediaType:'screen'})});
 const adapter=createDefaultSharedCssAdapter(options,true,true);
 const {document}=createEnvironment('<style>p{color:red}</style><p>value</p>'),target=document.querySelector('p');
 try{
  assert.equal(adapter.get(target).color,red);assert.equal(adapter.get(target).color,red);assert.equal(compileCalls,2);
  failure=new Error('quota callback original');assert.throws(()=>adapter.get(target),error=>error===failure);assert.equal(compileCalls,3);
  failure=null;assert.equal(adapter.get(target).color,red);assert.equal(compileCalls,4);
 }finally{adapter.dispose();}
});

test('private string providers preserve wrapper calls, exact constructor wire and original environment errors',()=>{
 const {document,window}=createEnvironment('<style>p{color:red}</style><p>value</p>');
 const target=document.querySelector('p'),counts={policy:0,preferences:0,environment:0,compile:0,prepare:0},wires=[];let failure;
 class Styles extends native.CompiledStylesV1{constructor(wire){super(wire);counts.compile++;wires.push(wire);}prepare(wire){counts.prepare++;return super.prepare(wire);}}
 try{
  install({CompiledStylesV1:Styles,cssRuntimePolicyV1(){counts.policy++;return native.cssRuntimePolicyV1();},cssComputedPreferencesV1(input){counts.preferences++;return native.cssComputedPreferencesV1(input);},cssComputedEnvironmentV1(input){counts.environment++;if(failure)throw failure;return native.cssComputedEnvironmentV1(input);}});
  for(let index=0;index<8;index++)assert.equal(window.getComputedStyle(target).color,red);
  assert.deepEqual(counts,{policy:1,preferences:1,environment:1,compile:1,prepare:1});
  const request=JSON.parse(wires[0]);assert.ok(request.sources[0].text.length>1000);assert.ok(request.sources[1].text.length>0);
  window.innerWidth=500;failure=new Error('original environment failure');
  assert.throws(()=>window.getComputedStyle(target),error=>error===failure);assert.throws(()=>window.getComputedStyle(target),error=>error===failure);
  failure=null;assert.equal(window.getComputedStyle(target).color,red);assert.equal(counts.environment,4);assert.equal(counts.compile,1);assert.equal(counts.prepare,2);
  document.querySelector('style').textContent='p{color:blue}';assert.equal(window.getComputedStyle(target).color,blue);assert.equal(counts.compile,2);
 }finally{restore();}
});

test('late object environment return stays mutable and disables reuse without freezing aliases',()=>{
 const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>');const target=document.querySelector('p');let objectMode=false,environment;
 try{
  install({cssComputedEnvironmentV1(input){const wire=native.cssComputedEnvironmentV1(input);if(!objectMode)return wire;environment=JSON.parse(wire);return environment;}});
  assert.equal(window.getComputedStyle(target).color,red);objectMode=true;window.innerWidth=500;
  assert.equal(window.getComputedStyle(target).color,red);assert.equal(Object.isFrozen(environment.features),false);
  environment.features['prefers-reduced-motion']={kind:'ident',value:'reduce'};
  assert.doesNotThrow(()=>window.getComputedStyle(target));assert.equal(Object.isFrozen(environment.features),false);
 }finally{restore();}
});

test('public default-profile preference aliases remain observable without private memo authority',()=>{
 const policy=JSON.parse(native.cssRuntimePolicyV1()),wires=[];
 class Styles extends native.CompiledStylesV1{constructor(wire){super(wire);wires.push(JSON.parse(wire));}}
 const backend={...native,CompiledStylesV1:Styles};
 const resourcePolicy={...Object.fromEntries(['preferences','environment','snapshot','selectorFacts','scene','composition'].map(name=>[name,()=>policy[name]])),adapter:Object.fromEntries(Object.entries(policy.adapter).map(([name,value])=>[name,()=>value])),fonts:policy.fonts};
 const options=createDefaultCssProfile({backend,capture:createDefaultCssCapture(),resourcePolicy,readWindowContext:()=>({hostPlatform:'other',width:800,height:600,devicePixelRatio:1,mediaType:'screen'})}),adapter=createDefaultSharedCssAdapter(options,true,true);
 const {document}=createEnvironment('<style>p{color:blue}</style><p>x</p>'),target=document.querySelector('p');
 try{
  const context=options.readContext(target);assert.equal(adapter.get(target).color,blue);
  const ua=context.preferences.userAgentSource.text;assert.equal(Object.isFrozen(ua),false);for(const char of 'p{color:red}')ua.push(char.charCodeAt(0));
  assert.equal(adapter.get(target).color,blue);assert.equal(wires.length,2);assert.deepEqual(wires[1].sources[0].text.slice(-12),Array.from('p{color:red}',c=>c.charCodeAt(0)));
 }finally{adapter.dispose();}
});

test('independent document callback reentry keeps private model ownership separate',()=>{
 const one=createEnvironment('<style>p{color:red}</style><p>one</p>'),two=createEnvironment('<style>p{color:blue}</style><p>two</p>');let nested=false,compiles=0;
 class Styles extends native.CompiledStylesV1{constructor(wire){super(wire);compiles++;if(!nested){nested=true;assert.equal(two.window.getComputedStyle(two.document.querySelector('p')).color,blue);}}}
 try{
  install({CompiledStylesV1:Styles});assert.equal(one.window.getComputedStyle(one.document.querySelector('p')).color,red);assert.equal(compiles,2);
  assert.equal(one.window.getComputedStyle(one.document.querySelector('p')).color,red);assert.equal(two.window.getComputedStyle(two.document.querySelector('p')).color,blue);assert.equal(compiles,2);
 }finally{restore();}
});

test('oversized successful source wires and repeated native quota failures retain original outcomes',()=>{
 const {document,window}=createEnvironment('<style>p{color:red}/*'+'x'.repeat(70000)+'*/</style><p>x</p>'),target=document.querySelector('p');
 assert.equal(window.getComputedStyle(target).color,red);assert.equal(window.getComputedStyle(target).color,red);
 let calls=0;const sentinel=new Error('original native constructor');class Broken{constructor(){calls++;throw sentinel;}}
 try{install({CompiledStylesV1:Broken});assert.throws(()=>window.getComputedStyle(target),error=>error===sentinel);assert.throws(()=>window.getComputedStyle(target),error=>error===sentinel);assert.equal(calls,2);}finally{restore();}
 assert.equal(window.getComputedStyle(target).color,red);
});

test('source reduction callbacks cannot lend aliased values to the private memo',()=>{
 for(const accessor of [false,true])for(const warm of [false,true]){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>'),target=document.querySelector('p');document.querySelector('style').sheet;
  if(warm)assert.equal(window.getComputedStyle(target).color,red);
  const original=Object.getOwnPropertyDescriptor(Array.prototype,'reduce'),push=Array.prototype.push,apply=Reflect.apply,foreign={marker:1};let calls=0,getters=0,error;
  function reduce(...args){Array.prototype.push=function(...values){if(values[0]?.sourceId!==undefined&&values[0]?.scopeId!==undefined){values[0].disabled=foreign;calls++;Array.prototype.push=push;}return apply(push,this,values);};return apply(original.value,this,args);}
  Object.defineProperty(Array.prototype,'reduce',accessor?{configurable:true,get(){getters++;return reduce;}}:{...original,value:reduce});
  try{window.getComputedStyle(target);}catch(caught){error=caught;}finally{Object.defineProperty(Array.prototype,'reduce',original);Array.prototype.push=push;}
  assert.match(error.message,/CSSBackendTransport:invalid type: map, expected a boolean/);assert.equal(calls,1);assert.equal(getters,accessor?1:0);assert.equal(Object.isFrozen(foreign),false);
  assert.equal(window.getComputedStyle(target).color,red);
 }
});

test('source Number callbacks cannot lend aliases through method or global binding overrides',()=>{
 for(const mode of ['method','method-accessor','global','global-accessor'])for(const warm of [false,true]){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>'),target=document.querySelector('p');document.querySelector('style').sheet;
  if(warm)assert.equal(window.getComputedStyle(target).color,red);
  const number=Number,globalField=Object.getOwnPropertyDescriptor(globalThis,'Number'),method=Object.getOwnPropertyDescriptor(number,'isSafeInteger'),push=Array.prototype.push,apply=Reflect.apply,foreign={marker:1};let calls=0,helperCalls=0,getters=0,error;
  function safeInteger(...args){helperCalls++;Array.prototype.push=function(...values){if(values[0]?.sourceId!==undefined&&values[0]?.scopeId!==undefined){values[0].disabled=foreign;calls++;Array.prototype.push=push;}return apply(push,this,values);};return apply(method.value,number,args);}
  if(mode==='method')Object.defineProperty(number,'isSafeInteger',{...method,value:safeInteger});
  if(mode==='method-accessor')Object.defineProperty(number,'isSafeInteger',{configurable:true,get(){getters++;return safeInteger;}});
  if(mode==='global')Object.defineProperty(globalThis,'Number',{...globalField,value:{isSafeInteger:safeInteger}});
  if(mode==='global-accessor')Object.defineProperty(globalThis,'Number',{configurable:true,get(){getters++;return{isSafeInteger:safeInteger};}});
  try{window.getComputedStyle(target);}catch(caught){error=caught;}finally{Object.defineProperty(globalThis,'Number',globalField);Object.defineProperty(number,'isSafeInteger',method);Array.prototype.push=push;}
  assert.match(error.message,/CSSBackendTransport:invalid type: map, expected a boolean/);assert.equal(calls,1);assert.equal(helperCalls,2);assert.equal(getters,mode.endsWith('accessor')?2:0);assert.equal(Object.isFrozen(foreign),false);
  assert.equal(window.getComputedStyle(target).color,red);
 }
});

test('missing source Number helpers preserve original errors and later recovery',()=>{
 for(const global of [false,true]){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>'),target=document.querySelector('p');window.getComputedStyle(target);
  const number=Number,owner=global?globalThis:number,name=global?'Number':'isSafeInteger',original=Object.getOwnPropertyDescriptor(owner,name);let error;
  delete owner[name];try{window.getComputedStyle(target);}catch(caught){error=caught;}finally{Object.defineProperty(owner,name,original);}
  assert.equal(error.name,global?'ReferenceError':'TypeError');assert.match(error.message,/Number/);assert.equal(window.getComputedStyle(target).color,red);
 }
});

test('routing quota setters cannot lend source rows to the private codec',()=>{
 for(const warm of [false,true]){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>');document.querySelector('style').sheet;const target=document.querySelector('p');if(warm)window.getComputedStyle(target).color;
  const define=Object.defineProperty,own=Object.hasOwn,proto=Object.prototype,old=Object.getOwnPropertyDescriptor(proto,'maxRoots'),zero=Object.getOwnPropertyDescriptor(proto,'0');
  const foreign={marker:1};let routeCalls=0,indexCalls=0,intercepted=0,error;
  function indexed(value){indexCalls++;if(value!==null&&typeof value==='object'&&own(value,'sourceId')&&own(value,'scopeId')){value.disabled=foreign;intercepted++;delete proto[0];}define(this,'0',{value,writable:true,enumerable:true,configurable:true});}
  define(proto,'maxRoots',{configurable:true,set(value){routeCalls++;define(this,'maxRoots',{value,writable:true,enumerable:true,configurable:true});define(proto,'0',{configurable:true,set:indexed});}});
  try{window.getComputedStyle(target).color;}catch(e){error=e;}finally{if(old)define(proto,'maxRoots',old);else delete proto.maxRoots;if(zero)define(proto,'0',zero);else delete proto[0];}
  assert.match(error.message,/expected a boolean/);assert.equal(routeCalls,1);assert.equal(indexCalls,24);assert.equal(intercepted,1);assert.equal(Object.isFrozen(foreign),false);
 }
});

test('inherited fields on malformed string providers retain mutable receiver and original callback errors',()=>{
 for(const mode of ['policy','preferences']){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>'),target=document.querySelector('p');
  const key=mode==='policy'?'adapter':'initial',saved=Object.getOwnPropertyDescriptor(Object.prototype,key),failure=new Error('inherited provider field');let calls=0,frozen;
  const method=mode==='policy'?'cssRuntimePolicyV1':'cssComputedPreferencesV1';
  try{
   install({[method](...args){const data=JSON.parse(Reflect.apply(native[method],native,args));delete data[key];return JSON.stringify(data);}});
   Object.defineProperty(Object.prototype,key,{configurable:true,get(){calls++;frozen=Object.isFrozen(this);this.observed=true;throw failure;}});
   assert.throws(()=>window.getComputedStyle(target),error=>error===failure);
  }finally{if(saved)Object.defineProperty(Object.prototype,key,saved);else delete Object.prototype[key];restore();}
  assert.equal(calls,1);assert.equal(frozen,false);
 }
});

test('inherited adapter setters preserve quota wrapper observations without freezing UA aliases',()=>{
 for(const key of ['compile','prepare']){
  const {document,window}=createEnvironment('<style>p{color:red}</style><p>x</p>'),target=document.querySelector('p');
  const saved=Object.getOwnPropertyDescriptor(Object.prototype,key),define=Object.defineProperty;let setterCalls=0,calls=0,seen;
  define(Object.prototype,key,{configurable:true,set(original){setterCalls++;define(this,key,{configurable:true,enumerable:true,writable:true,value(input){calls++;if(key==='compile'&&input.length)seen=input[0].text;if(key==='prepare')seen=input.ua.text;return original(input);}});}});
  try{assert.equal(window.getComputedStyle(target).color,red);assert.equal(window.getComputedStyle(target).color,red);}finally{if(saved)define(Object.prototype,key,saved);else delete Object.prototype[key];}
  assert.equal(setterCalls,1);assert.ok(calls>=2);assert.ok(seen.length>1000);assert.equal(Object.isFrozen(seen),false);
 }
});

import {loadNativeCss} from './opt-in-helpers/native.mjs';
// Mechanical source capture contracts; Rust remains the only CSS policy owner.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {CSSStyleSheet} from '../src/runtime/cssom.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {CssNativeSourcesUnavailable} from '../src/runtime/css-default-sources.mjs';
const policy=JSON.parse(loadNativeCss().cssRuntimePolicyV1());
function scene(html='<body>'){const env=createEnvironment(html),capture=createDefaultCssCapture();return{...env,capture,read(limits=policy.adapter.compile){return capture.sources(capture.capture(env.document,policy.snapshot),limits);}};}
test('physical STYLE source is exact UTF16 and ignores comments or nested element text',()=>{
 const {document,read}=scene();const style=document.createElement('style');const text='p{--raw:"\uD800";color:red}';style.append(document.createTextNode(text),document.createComment('not CSS'),document.createElement('b'));style.lastChild.textContent='not physical STYLE text';document.head.append(style);
 const result=read({...policy.adapter.compile,sourceUnits:text.length,sources:1});assert.equal(result.sources.length,1);assert.equal(result.sources[0].text,text);assert.equal(result.sources[0].text.charCodeAt(9),0xd800);
 assert.throws(()=>read({...policy.adapter.compile,sourceUnits:text.length-1}),/source units/);assert.throws(()=>read({...policy.adapter.compile,sources:0}),/source count/);
});
test('empty worlds, physical empty STYLE and constructed/adopted empty sheets have distinct source uses',()=>{
 const {document,read}=scene();assert.deepEqual(read({...policy.adapter.compile,sourceUnits:0,sources:0}).sources,[]);
 const style=document.createElement('style');document.head.append(style);assert.equal(read().sources.length,0);void style.sheet;
 let result=read({...policy.adapter.compile,sourceUnits:0,sources:1});assert.deepEqual(result.sources.map(row=>row.text),['']);assert.equal(result.uses[0].adopted,false);
 const sheet=new CSSStyleSheet();document.adoptedStyleSheets=[sheet];result=read({...policy.adapter.compile,sourceUnits:0,sources:2});assert.deepEqual(result.sources.map(row=>row.text),['','']);assert.equal(result.uses[1].adopted,true);
 assert.throws(()=>read({...policy.adapter.compile,sources:1}),/limit|source count/);
});
test('physical text and injected rules appear once with raw owner/media/disabled facts',()=>{
 const {document,read}=scene('<body><style media="screen" type="text/css">p{color:red}</style>');const owner=document.querySelector('style'),sheet=owner.sheet;sheet.insertRule('b{color:blue}');sheet.media='print';sheet.disabled=true;
 const units='p{color:red}'.length+'b{color:blue}'.length+'print'.length;const result=read({...policy.adapter.compile,sourceUnits:units,sources:2});assert.deepEqual(result.sources.map(row=>row.text),['p{color:red}','b{color:blue}']);
 assert.equal(result.uses[0].ownerMedia,'screen');assert.equal(result.uses[0].ownerType,'text/css');assert.equal(result.uses[0].sheetMedia,'print');assert.equal(result.uses[0].disabled,true);
 assert.throws(()=>read({...policy.adapter.compile,sourceUnits:units-1,sources:2}),/limit|source units/);
 sheet.media=['print'];assert.equal(read({...policy.adapter.compile,sourceUnits:units,sources:3}).uses[0].sheetMedia[0],'print');
});
test('genuine owner/adoption accessors produce typed capability refusal with zero getter calls',()=>{
 for(const kind of ['sheet','adoption']){const {document,capture}=scene('<body><style>p{color:red}</style>');const snapshot=capture.capture(document,policy.snapshot);let calls=0;const target=kind==='sheet'?document.querySelector('style'):document;Object.defineProperty(target,kind==='sheet'?'__sheet':'adoptedStyleSheets',{get(){calls++;throw Error('must not invoke getter');},configurable:true});assert.throws(()=>capture.sources(snapshot,policy.adapter.compile),error=>error instanceof CssNativeSourcesUnavailable);assert.equal(calls,0);}
 const {document,capture}=scene('<body><style>p{color:red}</style>');const snapshot=capture.capture(document,policy.snapshot);document.querySelector('style').firstChild._data=undefined;assert.throws(()=>capture.sources(snapshot,policy.adapter.compile),/unknown STYLE text/);
});
test('media accessors and original caller-policy errors remain honest',()=>{
 const {document,read}=scene('<body><style></style>');const sheet=document.querySelector('style').sheet;let calls=0;Object.defineProperty(sheet,'media',{get(){calls++;throw Error('must not invoke media');}});assert.throws(()=>read(),/accessor media/);assert.equal(calls,0);
 const empty=scene(),original=new Error('policy provider');const limits=new Proxy(policy.adapter.compile,{get(target,key){if(key==='sourceUnits')throw original;return Reflect.get(target,key);}});assert.throws(()=>empty.read(limits),error=>error===original);
 for(const limits of [null,{sourceUnits:-1,sources:1},{sourceUnits:1,sources:NaN}])assert.throws(()=>empty.read(limits),TypeError);
});

test('native sheet source capture validates owned fields and never reads caller accessors',()=>{
 for(const [key,value,reason] of [['ownerNode',{},/unknown owner/],['disabled','false',/non-boolean disabled/],['media',[42],/non-string media/],['cssRules',{},/unknown sequence/],['cssRules',null,/unknown sequence/]]){
  const {document,read}=scene('<body><style></style>');
  const sheet=document.querySelector('style').sheet;
  sheet[key]=value;
  assert.throws(read,reason);
 }
 const {document,read}=scene('<body><style></style>'),sheet=document.querySelector('style').sheet;
 sheet.insertRule('p{color:red}');
 const rule=sheet.cssRules[0];
 rule.cssText=7;
 assert.throws(read,/non-string rule/);
 let calls=0;
 Object.defineProperty(rule,'cssText',{configurable:true,get(){calls++;return 'p{color:red}';}});
 assert.throws(read,/missing\/accessor cssText/);
 assert.equal(calls,0);
});
test('adopted native sheet sequences reject nonobjects, foreign sheets and proxies without traps',()=>{
 for(const value of [null,7,{},[{}]]){
  const {document,read}=scene();
  document.adoptedStyleSheets=value;
  assert.throws(read,/unknown sequence|unknown sheet/);
 }
 const {document,read}=scene();
 let calls=0;
 document.adoptedStyleSheets=new Proxy([],{get(){calls++;throw Error('must not run');}});
 assert.throws(read,/unknown sequence proxy/);
 assert.equal(calls,0);
});

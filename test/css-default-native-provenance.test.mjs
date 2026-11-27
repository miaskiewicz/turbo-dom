import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {CSSStyleSheet} from '../src/runtime/cssom.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
function setup(){const env=createEnvironment('<body><style>[data-tone="a"]{color:red}[data-tone="b"]{color:blue} input:checked{color:red} input{color:blue}</style><div data-tone="a">x</div><input type="checkbox">');return{...env,element:env.document.querySelector('div'),input:env.document.querySelector('input')};}
function trap(target={}){let count=0;const error=new Error('untrusted Proxy must not execute');const proxy=new Proxy(target, {get(){count++;throw error;},getPrototypeOf(){count++;throw error;},getOwnPropertyDescriptor(){count++;throw error;},ownKeys(){count++;throw error;}});return{proxy,count:()=>count};}
function refused(action,probe){assert.throws(action,/CSS|native|unknown|foreign|nonprimitive/);assert.equal(probe.count(),0);}
test('public GCS fingerprints admitted raw attribute edits even with unchanged document version',()=>{
 const {document,window,element}=setup();assert.equal(window.getComputedStyle(element).color,'rgb(255, 0, 0)');
 void element.attributes.length;const attribute=element.__attrs.find(row=>row.name==='data-tone');const version=document.__version;
 attribute.value='b';assert.equal(document.__version,version);assert.equal(window.getComputedStyle(element).color,'rgb(0, 0, 255)');
 attribute.value='a';assert.equal(document.__version,version);assert.equal(window.getComputedStyle(element).color,'rgb(255, 0, 0)');
});
test('public GCS refuses foreign proxy receiver, attrs, cache, buffer and dirty state without traps',()=>{
 {const {window,element}=setup(),probe=trap(element);refused(()=>window.getComputedStyle(probe.proxy),probe);}
 for(const field of ['__attrs','__kids']){const {window,element}=setup(),probe=trap();element[field]=probe.proxy;refused(()=>window.getComputedStyle(element),probe);}
 for(const field of ['__cache','__buf']){const {document,window,element}=setup(),probe=trap();document[field]=probe.proxy;refused(()=>window.getComputedStyle(element),probe);}
 {const {window,input}=setup(),probe=trap();input.__checked=probe.proxy;refused(()=>window.getComputedStyle(input),probe);}
});
test('public GCS refuses unknown CSSOM, adopted, rule and media proxies before traps',()=>{
 for(const field of ['cssRules','media']){const {document,window,element}=setup(),sheet=document.querySelector('style').sheet,probe=trap();sheet[field]=probe.proxy;refused(()=>window.getComputedStyle(element),probe);}
 {const {document,window,element}=setup(),probe=trap();document.adoptedStyleSheets=probe.proxy;refused(()=>window.getComputedStyle(element),probe);}
 {const {document,window,element}=setup(),probe=trap();document.adoptedStyleSheets=[probe.proxy];refused(()=>window.getComputedStyle(element),probe);}
 {const {document,window,element}=setup(),probe=trap();const sheet=new CSSStyleSheet();sheet.cssRules=[probe.proxy];document.adoptedStyleSheets=[sheet];refused(()=>window.getComputedStyle(element),probe);}
});
test('adopted native CSSOM stays live without replacing rule or media identity',()=>{
 const {document,window,element}=setup(),sheet=new CSSStyleSheet();sheet.insertRule('div{background-color:red}');document.adoptedStyleSheets=[sheet];
 const rule=sheet.cssRules[0],media=sheet.media;assert.equal(window.getComputedStyle(element).backgroundColor,'rgb(255, 0, 0)');
 rule.cssText='div{background-color:blue}';assert.equal(window.getComputedStyle(element).backgroundColor,'rgb(0, 0, 255)');assert.equal(sheet.cssRules[0],rule);assert.equal(sheet.media,media);
});
test('snapshot provenance rejects forged and proxy snapshots before method reads',()=>{
 const capture=createDefaultCssCapture(),probe=trap();
 refused(()=>capture.sources(probe.proxy,{sourceUnits:10,sources:10}),probe);
 refused(()=>capture.selectorFacts(probe.proxy,{maxNodes:10,maxEdges:10,maxTextUnits:10,maxAttributeUnits:10}),probe);
 assert.throws(()=>capture.sources({},{}),/unknown native source snapshot/);
});
test('connected dirty state refreshes and removed same-owner focus does not poison other geometry',()=>{
 const {document,window,input,element}=setup();assert.equal(window.getComputedStyle(input).color,'rgb(0, 0, 255)');input.checked=true;assert.equal(window.getComputedStyle(input).color,'rgb(255, 0, 0)');
 input.focus();input.remove();assert.equal(document.__active,null);assert.equal(window.getComputedStyle(element).color,'rgb(255, 0, 0)');
 const foreign=setup();document.__setActive(foreign.input);assert.throws(()=>window.getComputedStyle(element),/foreign focus owner/);
});

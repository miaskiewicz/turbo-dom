import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from '../src/runtime/index.mjs';
import {captureCssPrimitivePublicFacts as capture} from './helpers/css-primitive-public-facts.mjs';
import {createCssFactsNavigator as create,readCssFactsNavigator as read,CssFactsUnavailable} from '../src/runtime/css-facts-navigator.mjs';
const limits={maxNodes:100,maxEdges:100,maxAttributeUnits:1000,maxTextUnits:1000};
function fixture(){const {document}=createEnvironment('<body><p id="a">x</p><b title="b"></b>');return capture(document,{focusTarget:null});}
function p(source){return source.nodes.find(row=>row.localName==='p');}
function admit(source,quota=limits){return read(create(source,quota));}
function rejects(change,pattern){const source=fixture();change(source);assert.throws(()=>admit(source),pattern??CssFactsUnavailable);}
test('owned primitive operations retain order, qualified attrs and null boundaries',()=>{
 const source=fixture(), para=p(source), bold=source.nodes.find(row=>row.localName==='b');
 para.attributes.push({name:'href',value:'x',prefix:'xlink',namespaceURI:'http://www.w3.org/1999/xlink'});
 const nav=admit(source);
 assert.equal(nav.attribute(para.id,'id'),'a');assert.equal(nav.attribute(para.id,'xlink:href'),'x');assert.equal(nav.attribute(para.id,'missing'),null);
 assert.equal(nav.attrs(para.id).length,2);assert.equal(nav.next(para.id),bold.id);assert.equal(nav.previous(bold.id),para.id);
 assert.equal(nav.previous(para.id),null);assert.equal(nav.next(bold.id),null);assert.equal(nav.previous(source.documentId),null);
 assert.equal(nav.children(para.id).length,1);assert.equal(nav.focusTarget(),null);
 assert.throws(()=>nav.property(para.id,'uncaptured'),/uncaptured/);assert.throws(()=>nav.property(0xffffffff,'id'),/unknown node/);
 assert.throws(()=>nav.state(para.id,'uncaptured'),/state uncaptured/);
 assert.ok(Object.isFrozen(nav.children(para.id)));assert.ok(Object.isFrozen(nav.attrs(para.id)));
});
test('exact quotas admit; each one-short component refuses original snapshot',()=>{
 const source=fixture();let edges=0,attributes=0,text=0;
 for(const row of source.nodes){edges+=row.childIds.length+(row.attributes?.length??0);for(const attr of row.attributes??[])for(const key of ['name','value','prefix','namespaceURI'])if(typeof attr[key]==='string')attributes+=attr[key].length;
 for(const key of ['localName','namespaceURI','text'])if(typeof row[key]==='string')text+=row[key].length;
 for(const key of Object.keys(row.state??{})){text+=key.length;if(typeof row.state[key]==='string')text+=row.state[key].length;}}
 const exact={maxNodes:source.nodes.length,maxEdges:edges,maxAttributeUnits:attributes,maxTextUnits:text};admit(source,exact);
 for(const key of Object.keys(exact))assert.throws(()=>admit(source,{...exact,[key]:exact[key]-1}),/limit/);
 for(const value of [-1,1.5,NaN,Infinity])assert.throws(()=>admit(source,{...limits,maxNodes:value}),/limit/);
});
test('attribute admission distinguishes absent, null and malformed primitive data',()=>{
 rejects(s=>{p(s).attributes[0].value=3;},/attribute value/);
 rejects(s=>{delete p(s).attributes[0].name;},/attribute name\/value/);
 rejects(s=>{p(s).attributes[0].value=null;},/attribute name\/value/);
 const source=fixture();delete p(source).attributes[0].prefix;assert.throws(()=>admit(source).attribute(p(source).id,'id'),/attribute prefix/);
 const nullPrefix=fixture();p(nullPrefix).attributes[0].prefix=null;assert.equal(admit(nullPrefix).attribute(p(nullPrefix).id,'id'),'a');
 const absent=fixture();delete p(absent).attributes;assert.throws(()=>admit(absent).attrs(p(absent).id),/attributes/);
});
test('missing and malformed primitive state never become captured false',()=>{
 for(const value of [{},()=>{},Symbol('x'),1n])rejects(s=>{p(s).state.checked=value;},/state checked/);
 const source=fixture();p(source).state={nullable:null,count:1,label:'yes',checked:undefined};const nav=admit(source);
 assert.equal(nav.state(p(source).id,'nullable'),null);assert.equal(nav.state(p(source).id,'count'),1);assert.equal(nav.state(p(source).id,'label'),'yes');assert.equal(nav.state(p(source).id,'checked'),undefined);
 const missing=fixture();delete p(missing).state;assert.throws(()=>admit(missing).state(p(missing).id,'checked'),/state checked/);
 rejects(s=>{p(s).text=1;},/text/);rejects(s=>{Object.defineProperty(p(s),'localName',{get(){throw new Error('must not call');}});},/accessor/);
});
test('graph and identity admission rejects duplicate, missing and inconsistent owners',()=>{
 rejects(s=>{s.nodes.push(s.nodes[0]);},/duplicate/);
 rejects(s=>{const row=p(s);s.nodes.find(x=>x.id===row.parentId).childIds.shift();row.parentId=0xffffffff;},/unknown node/);
 rejects(s=>{p(s).parentId=null;},/child\/parent/);
 rejects(s=>{const row=p(s);s.nodes.find(x=>x.id===row.parentId).childIds=[];},/parent\/child/);
 rejects(s=>{s.documentId=0xffffffff;},/unknown node/);rejects(s=>{s.documentElementId=0xffffffff;},/unknown node/);
 rejects(s=>{s.focusTarget=0xffffffff;},/unknown node/);
 for(const value of [-1,1.1,0x100000000])rejects(s=>{p(s).id=value;},/identity/);
 rejects(s=>{delete p(s).childIds;},/childIds/);
 const source=fixture();delete source.focusTarget;assert.throws(()=>admit(source).focusTarget(),/focusTarget/);
 const empty=createEnvironment('');const facts=capture(empty.document);facts.documentElementId=null;assert.equal(admit(facts).documentElementId,null);
});

import {loadNativeCss} from './opt-in-helpers/native.mjs';
// Internal capture admission contracts, separate from public Step4 acceptance.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment,Document,Node} from './opt-in-helpers/runtime.mjs';
import {createDefaultCssCapture,CssDomSnapshotLimitError,nativeCssConnected} from '../src/runtime/css-default-capture.mjs';
const policy=JSON.parse(loadNativeCss().cssRuntimePolicyV1()).snapshot;
function environment(){return createEnvironment('<body><div id="x" title="é">hello<b>world</b></div>');}
function limited(action,name){assert.throws(action,error=>error instanceof CssDomSnapshotLimitError&&error.limit===name);}
test('real snapshot admits measured exact counts and refuses each one-short quota',()=>{
 const {document}=environment(),capture=createDefaultCssCapture(),wide=capture.capture(document,policy);
 const exact={...policy,...wide.used,maxDepth:Math.max(...wide.records.map(row=>row.physicalPath.length-1))};
 const admitted=capture.capture(document,exact);assert.equal(admitted.records.length,wide.records.length);
 for(const name of ['maxNodes','maxPathUnits','maxTextUnits','maxAttributes','maxAttributeUnits','maxDepth'])limited(()=>capture.capture(document,{...exact,[name]:exact[name]-1}),name);
 for(const name of ['maxNodes','maxPathUnits','maxTextUnits','maxAttributes','maxAttributeUnits','maxDepth'])limited(()=>capture.capture(document,{...policy,[name]:0}),name);
 assert.throws(()=>capture.capture(document,{...policy,maxNodes:-1}),TypeError);
});
test('empty genuine no-buffer Document admits zero text and attrs then resets world identity',()=>{
 const document=new Document(),capture=createDefaultCssCapture();assert.equal(document.__buf,null);
 assert.throws(()=>capture.capture(document,policy),/unknown __version/);
 const temporary=document.createTextNode('');document.appendChild(temporary);document.removeChild(temporary);
 const snapshot=capture.capture(document,{...policy,maxNodes:1,maxPathUnits:2,maxDepth:0,maxTextUnits:0,maxAttributes:0,maxAttributeUnits:0});assert.equal(snapshot.records.length,1);assert.equal(snapshot.records[0].kind,'document');
 capture.reset(document);const next=capture.capture(document,policy);assert.notEqual(next.world.identity,snapshot.world.identity);assert.throws(()=>snapshot.nodeId(new Document()),/outside snapshot/);
});
test('real template contents remain inert across materialization and buffer-cache recapture',()=>{
 const {document}=createEnvironment('<body><template><b title="inert">hidden</b></template><p>shown</p>');
 const template=document.querySelector('template');assert.equal(template.content.querySelector('b').textContent,'hidden');
 const body=document.body;body.__kids=null;const capture=createDefaultCssCapture();const snapshot=capture.capture(document,policy);
 assert.equal(snapshot.records.some(row=>row.tag==='b'),false);assert.equal(snapshot.records.some(row=>row.text==='hidden'),false);assert.equal(snapshot.records.some(row=>row.tag==='template'),true);
 const owner=document.__inert;assert.ok(owner instanceof Document);assert.notEqual(owner,document);
 body.__kids=null;const saved=document.__inert;document.__inert=new Document();assert.throws(()=>capture.capture(document,policy),/foreign inert owner/);document.__inert=saved;
 assert.equal(capture.capture(document,policy).records.some(row=>row.tag==='p'),true);
});
test('real namespace creation is preserved while tampered namespace refuses',()=>{
 const {document}=createEnvironment('<body>');const element=document.createElementNS('urn:actual','test');document.body.append(element);const capture=createDefaultCssCapture();assert.equal(capture.capture(document,policy).records.find(row=>row.element===element).namespace,'urn:actual');
 element.__ns='unknown-native-namespace';assert.throws(()=>capture.capture(document,policy),/unknown namespace/);
});
test('owned cache accessors refuse without getter calls and native method failures preserve identity',()=>{
 const {document}=environment();const body=document.body,html=document.documentElement;const original=Object.getOwnPropertyDescriptor(document.__cache,String(body.__idx));let calls=0;
 Object.defineProperty(document.__cache,String(body.__idx),{configurable:true,get(){calls++;throw Error('cache getter');}});html.__kids=null;
 assert.throws(()=>createDefaultCssCapture().capture(document,policy),/unknown cached node/);assert.equal(calls,0);Object.defineProperty(document.__cache,String(body.__idx),original);
 const error=new Error('limits original');const caps=new Proxy(policy,{get(target,key){if(key==='maxNodes')throw error;return Reflect.get(target,key);}});assert.throws(()=>createDefaultCssCapture().capture(document,caps),caught=>caught===error);
});
test('assigned-slot composed paths consume the same aggregate quota including shadow hosts',()=>{
 const {document}=createEnvironment('<body><div><b>x</b></div>'),host=document.querySelector('div');host.attachShadow({mode:'closed'}).innerHTML='<section><slot></slot></section>';
 const capture=createDefaultCssCapture(),wide=capture.capture(document,policy);const slotted=wide.records.find(row=>row.tag==='b');assert.ok(slotted.assignedSlotPath.length>0);
 assert.equal(capture.capture(document,{...policy,maxPathUnits:wide.used.maxPathUnits}).used.maxPathUnits,wide.used.maxPathUnits);
 limited(()=>capture.capture(document,{...policy,maxPathUnits:wide.used.maxPathUnits-1}),'maxPathUnits');
 limited(()=>capture.capture(document,{...policy,maxDepth:slotted.assignedSlotPath.length-2}),'maxDepth');
});
test('same-owner detached focus ancestry is depth-bounded and cycles refuse honestly',()=>{
 const {document}=createEnvironment('<body>');const root=document.createElement('div');let parent=root;
 for(let i=0;i<8;i++){const next=document.createElement('div');parent.appendChild(next);parent=next;}
 const input=document.createElement('input');parent.appendChild(input);input.focus();assert.notEqual(document.__active,input);document.__setActive(input);const capture=createDefaultCssCapture();
 limited(()=>capture.capture(document,{...policy,maxDepth:3}),'maxDepth');
 assert.equal(capture.capture(document,policy).records.some(row=>row.element===input),false);
 input.parentNode=input;assert.throws(()=>capture.capture(document,policy),/cyclic focused ancestry/);
});
test('the same capture tracks a new document buffer after an environment reset',()=>{
 const env=environment(),capture=createDefaultCssCapture();
 const before=capture.capture(env.document,policy);
 env.reset('<body><p id="after">New document content</p></body>');
 const after=capture.capture(env.document,policy);
 assert.equal(after.world.identity,before.world.identity);
 assert.notEqual(after.world.incarnation,before.world.incarnation);
 assert.equal(after.records.some(row=>row.tag==='p'),true);
 assert.equal(after.records.some(row=>row.tag==='b'),false);
});
test('native element connectivity checks stay within the requested depth',()=>{
 const {document}=environment(),element=document.getElementById('x');
 limited(()=>nativeCssConnected(element,document,0),'maxDepth');
 assert.equal(nativeCssConnected(element,document,policy.maxDepth),true);
 const base=new Node(document);
 document.body.appendChild(base);
 assert.throws(()=>createDefaultCssCapture().capture(document,policy),/foreign node/);
});
test('native reads check element kind, ownership, and original parent edges',()=>{
 const {document,window}=environment(),element=document.getElementById('x');
 assert.throws(()=>window.getComputedStyle(document.createTextNode('text')),/requires an Element/);
 const other=environment().document;
 element.ownerDocument=null;
 assert.throws(()=>window.getComputedStyle(element),/foreign owner document/);
 element.ownerDocument=other;
 assert.throws(()=>createDefaultCssCapture().capture(document,policy),/foreign node owner/);
 element.ownerDocument=document;
 element.parentNode=element;
 assert.throws(()=>nativeCssConnected(element,document,policy.maxDepth),/cyclic element ancestry/);
 assert.throws(()=>createDefaultCssCapture().capture(document,policy),/unknown physical parent/);
 element.parentNode={};
 assert.throws(()=>nativeCssConnected(element,document,policy.maxDepth),/foreign element ancestor/);
 element.parentNode=document.body;
 assert.equal(nativeCssConnected(element,document,policy.maxDepth),true);
});
test('native capture checks shadow host edges and does not cross foreign documents',()=>{
 const {document}=environment(),host=document.getElementById('x');
 const root=host.attachShadow({mode:'open'});
 const child=document.createElement('input');
 root.appendChild(child);
 assert.equal(nativeCssConnected(child,document,policy.maxDepth),true);
 assert.equal(nativeCssConnected(child,environment().document,policy.maxDepth),false);
 root.host=document.body;
 assert.throws(()=>createDefaultCssCapture().capture(document,policy),/foreign shadow host/);
 root.host=host;
 assert.equal(createDefaultCssCapture().capture(document,policy).roots.length,2);
});
test('native capture validates materialized attributes and text before using their values',()=>{
 const {document}=environment(),element=document.getElementById('x');
 const capture=createDefaultCssCapture();
 element.setAttribute('title','updated');
 const attrs=element.__attrs, attribute=attrs.find(row=>row.name==='title');
 const index=attrs.indexOf(attribute);
 attrs[index]={name:'title',value:'foreign'};
 assert.throws(()=>capture.capture(document,policy),/foreign original attribute/);
 attrs[index]=attribute;
 attribute.value=42;
 assert.throws(()=>capture.capture(document,policy),/unknown attribute/);
 attribute.value='updated';
 const text=element.firstChild;
 text._data=null;
 assert.throws(()=>capture.capture(document,policy),/unknown text/);
 text._data='hello';
 assert.ok(capture.capture(document,policy).records.some(row=>row.kind==='text'));
});
test('capture detects unavailable native helpers and recovers after restoration',()=>{
 const {document}=environment(),capture=createDefaultCssCapture();
 const expectRefusal=(owner,key,change,message)=>{
   const original=Object.getOwnPropertyDescriptor(owner,key);
   let failure;
   try {
     change();
     capture.capture(document,policy);
   } catch (error) {
     failure=error;
   } finally {
     if(original)Object.defineProperty(owner,key,original);
     else delete owner[key];
   }
   assert.match(failure?.message ?? '',message);
 };
 expectRefusal(Array.prototype,Symbol.iterator,()=>{delete Array.prototype[Symbol.iterator]},/overridden array iterator/);
 expectRefusal(Map.prototype,'has',()=>{delete Map.prototype.has},/overridden collection helper/);
 expectRefusal(WeakMap.prototype,'delete',()=>{Object.defineProperty(WeakMap.prototype,'delete',{configurable:true,get(){return undefined}})},/overridden collection helper/);
 expectRefusal(Array,Symbol.species,()=>{Object.defineProperty(Array,Symbol.species,{value:Array,configurable:true})},/overridden array constructor\/species/);
 expectRefusal(String.prototype,'charCodeAt',()=>{delete String.prototype.charCodeAt},/overridden string helper/);
 expectRefusal(Document.prototype,'__nodeAt',()=>{delete Document.prototype.__nodeAt},/missing __nodeAt/);
 expectRefusal(Object.prototype,'7',()=>{Object.defineProperty(Object.prototype,'7',{value:undefined,writable:true,configurable:true})},/indexed prototype storage/);
 expectRefusal(Document.prototype,'__inert',()=>{Object.defineProperty(Document.prototype,'__inert',{value:undefined,writable:true,configurable:true})},/constructor field __inert/);
 assert.equal(capture.capture(document,policy).records.some(row=>row.tag==='b'),true);
});
test('selector state follows genuine control data and rejects unknown state descriptors',()=>{
 const {document,window}=createEnvironment('<body><style>input{color:blue}input:checked{color:red}</style><input type="checkbox">');
 const control=document.querySelector('input');
 assert.equal(window.getComputedStyle(control).color,'rgb(0, 0, 255)');
 control.checked=true;
 assert.equal(window.getComputedStyle(control).color,'rgb(255, 0, 0)');
 let calls=0;
 Object.defineProperty(control,'__checked',{configurable:true,get(){calls++;return true}});
 assert.throws(()=>window.getComputedStyle(control),/unknown native state __checked/);
 assert.equal(calls,0);
 Object.defineProperty(control,'__checked',{value:false,writable:true,configurable:true});
 assert.equal(window.getComputedStyle(control).color,'rgb(0, 0, 255)');
 Object.defineProperty(control,'nodeType',{value:1,configurable:true});
 assert.equal(window.getComputedStyle(control).color,'rgb(0, 0, 255)');
 Object.defineProperty(control,'nodeType',{value:3,configurable:true});
 assert.throws(()=>window.getComputedStyle(control),/changed native nodeType/);
});
test('native capture treats stale buffered wrappers and invalid node data as unavailable',()=>{
 const {document}=environment(),body=document.body,html=document.documentElement;
 const index=body.__idx,capture=createDefaultCssCapture(),buffer=document.__buf;
 const original=document.__cache[index],type=buffer.soa.nodeType[index];
 html.__kids=null;
 document.__cache[index]={};
 assert.throws(()=>capture.capture(document,policy),/foreign cached wrapper/);
 document.__cache[index]=original;
 buffer.soa.nodeType[index]=0;
 assert.throws(()=>capture.capture(document,policy),/unknown buffered node type/);
 buffer.soa.nodeType[index]=type;
 assert.ok(capture.capture(document,policy).records.some(row=>row.element===body));
});
test('a connected focused element needs a genuine element wrapper and admitted state getter',()=>{
 const {document,window}=createEnvironment('<body><style>input:focus{color:red}</style><input>');
 const input=document.querySelector('input'),element=Object.getPrototypeOf(input);
 input.focus();
 assert.equal(window.getComputedStyle(input).color,'rgb(255, 0, 0)');
 document.__active=document.createTextNode('not an element');
 assert.throws(()=>window.getComputedStyle(input),/foreign focused node/);
 document.__active=input;
 const original=Object.getOwnPropertyDescriptor(element,'required');
 try {
   Object.defineProperty(element,'required',{configurable:true,value:false});
   assert.throws(()=>window.getComputedStyle(input),/changed native required getter/);
 } finally {
   Object.defineProperty(element,'required',original);
 }
 assert.equal(window.getComputedStyle(input).color,'rgb(255, 0, 0)');
});

test('native selector snapshots reject nonprimitive and inherited private state without coercion',()=>{
 const {document}=createEnvironment('<body><input>'),input=document.querySelector('input');
 const capture=createDefaultCssCapture();
 for(const name of ['__checked','__selected','__ce','__ceIs']){
  const own=Object.getOwnPropertyDescriptor(input,name);
  Object.defineProperty(input,name,{value:{},configurable:true,writable:true});
  assert.throws(()=>capture.capture(document,policy),/nonprimitive native state/);
  if(own)Object.defineProperty(input,name,own);else delete input[name];
 }
 const proto=Object.getPrototypeOf(input),old=Object.getOwnPropertyDescriptor(proto,'__selected');
 delete input.__selected;
 try{
  Object.defineProperty(proto,'__selected',{value:false,configurable:true});
  assert.throws(()=>capture.capture(document,policy),/inherited native state/);
 }finally{if(old)Object.defineProperty(proto,'__selected',old);else delete proto.__selected;}
 assert.ok(capture.capture(document,policy));
});
test('genuine owned node arrays still reject changed storage and duplicate node edges',()=>{
 const {document}=environment(),body=document.body,children=body.childNodes;
 assert.ok(children.length);
 const capture=createDefaultCssCapture(),storage=body.__kids;
 const proto=Object.getPrototypeOf(storage);
 try{
  Object.setPrototypeOf(storage,null);
  assert.throws(()=>capture.capture(document,policy),/unknown array storage/);
 }finally{Object.setPrototypeOf(storage,proto);}
 storage.push(storage[0]);
 assert.throws(()=>capture.capture(document,policy),/duplicate\/cyclic node/);
 storage.pop();
 storage.push({});
 assert.throws(()=>capture.capture(document,policy),/foreign node wrapper/);
 storage.pop();
 assert.ok(capture.capture(document,policy));
});
test('materialized element namespace, tag and native kind descriptors remain checked',()=>{
 const {document}=environment(),element=document.getElementById('x'),capture=createDefaultCssCapture();
 const ns=element.__ns,tag=element.localName;
 element.__ns=7;
 assert.throws(()=>capture.capture(document,policy),/unknown namespace/);
 element.__ns='ns:';
 assert.equal(capture.capture(document,policy).records.find(row=>row.element===element).namespace,null);
 element.__ns=ns;element.localName=7;
 assert.throws(()=>capture.capture(document,policy),/unknown tag/);
 element.localName=tag;
 const proto=Object.getPrototypeOf(element),old=Object.getOwnPropertyDescriptor(proto,'nodeType');
 try{
  Object.defineProperty(proto,'nodeType',{value:1,configurable:true});
  assert.throws(()=>capture.capture(document,policy),/changed native nodeType getter/);
 }finally{Object.defineProperty(proto,'nodeType',old);}
 assert.ok(capture.capture(document,policy));
});
test('buffered sibling cycles and out-of-range columns refuse before materialization',()=>{
 for(const corrupt of ['cycle','missing','text']){
  const {document}=createEnvironment(`<body><div>${corrupt}</div>`),body=document.body,buf=document.__buf;
  const capture=createDefaultCssCapture(),index=body.__idx;
  if(corrupt==='cycle'){
   const first=buf.soa.firstChild[index],old=buf.soa.nextSib[first];buf.soa.nextSib[first]=first;
   try{assert.throws(()=>capture.capture(document,policy),/cyclic buffer children/);}finally{buf.soa.nextSib[first]=old;}
  }else if(corrupt==='missing'){
   const old=buf.soa.firstChild[index];buf.soa.firstChild[index]=-2;
   try{assert.throws(()=>capture.capture(document,policy),/unknown input cell/);}finally{buf.soa.firstChild[index]=old;}
  }else{
   const tag=buf.soa.tagId[buf.soa.firstChild[index]],old=buf.soa.tagNames[tag];buf.soa.tagNames[tag]=false;
   try{assert.throws(()=>capture.capture(document,policy),/invalid tagNames text/);}finally{buf.soa.tagNames[tag]=old;}
  }
 }
});

test('detached focus requires same-owner native ancestry and cannot forge connected membership',()=>{
 for(const kind of ['foreign-owner','foreign-parent','missing-member']){
  const {document}=createEnvironment(`<body><p>${kind}</p>`),input=document.createElement('input');
  document.__setActive(input);
  if(kind==='foreign-owner')input.ownerDocument=createEnvironment('<p>other</p>').document;
  if(kind==='foreign-parent')input.parentNode={};
  if(kind==='missing-member')input.parentNode=document.body;
  const expected=kind==='foreign-owner'?/foreign focus owner/:kind==='foreign-parent'?/foreign focus ancestor/:/connected focus missing/;
  assert.throws(()=>createDefaultCssCapture().capture(document,policy),expected);
 }
});

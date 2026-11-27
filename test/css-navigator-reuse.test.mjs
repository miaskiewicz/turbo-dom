import {loadNativeCss} from './opt-in-helpers/native.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {createDefaultCssProfile} from '../src/runtime/css-default-profile.mjs';
import {createDefaultSharedCssAdapter} from '../src/runtime/css-shared-adapter.mjs';
const native=loadNativeCss();
const policy=JSON.parse(native.cssRuntimePolicyV1());
const html='<body><div title="a">hello<b>world</b></div><input type="checkbox"><aside></aside>';
function setup(source=html) {
 const env=createEnvironment(source),capture=createDefaultCssCapture();
 const read=()=>capture.selectorFacts(capture.capture(env.document,policy.snapshot),policy.selectorFacts);
 return {...env,capture,read};
}
test('unchanged complete native captures reuse one admitted selector token',()=>{
 const {read}=setup();const first=read();assert.equal(read(),first);assert.equal(read(),first);
});
test('default adapter installs retained matching once across unchanged fresh captures',()=>{
 let installs=0,resolves=0,compiles=0;
 class MeasuredStyles {
  constructor(request){this.compiled=new native.CompiledStylesV1(request);compiles++;}
  metadata(){return this.compiled.metadata();} dispose(){this.compiled.dispose();}
  prepare(request){const p=this.compiled.prepare(request);return{
   metadata:()=>p.metadata(),dispose:()=>p.dispose(),resolveBatch:r=>p.resolveBatch(r),
   storeMatchingV1(wire,limits){installs++;return p.storeMatchingV1(wire,limits);},
   resolveMatchingV1(request,token){resolves++;return p.resolveMatchingV1(request,token);}
  };}
 }
 const backend={...native,CompiledStylesV1:MeasuredStyles};
 const env=setup('<style>div{color:red}</style><div>hello</div>');
 const options=createDefaultCssProfile({backend,capture:env.capture,
  readWindowContext:()=>({hostPlatform:'other',width:800,height:600,devicePixelRatio:1,mediaType:'screen'}),
  resourcePolicy:{preferences:()=>policy.preferences,environment:()=>policy.environment,snapshot:()=>policy.snapshot,
   selectorFacts:()=>policy.selectorFacts,scene:()=>policy.scene,composition:()=>policy.composition,fonts:policy.fonts,
   adapter:Object.fromEntries(Object.entries(policy.adapter).map(([name,value])=>[name,()=>value]))}});
 const adapter=createDefaultSharedCssAdapter(options,true,true),target=env.document.querySelector('div');
 try {for(let i=0;i<3;i++)assert.equal(adapter.get(target).color,'rgb(255, 0, 0)');
  assert.equal(compiles,1);assert.equal(resolves,3);assert.equal(installs,1);
 } finally {adapter.dispose();}
});
test('raw attrs, states, focus, text, order and membership each invalidate selector token',()=>{
 const {document,read}=setup(),div=document.querySelector('div'),input=document.querySelector('input'),aside=document.querySelector('aside');
 let token=read();
 const change=(action)=>{action();const next=read();assert.notEqual(next,token);assert.equal(read(),next);token=next;};
 change(()=>div.setAttribute('title','b'));
 const attr=div.__attrs.find(row=>row.name==='title'),version=document.__version;
 change(()=>{attr.value='c';});assert.equal(document.__version,version);
 change(()=>{input.checked=true;});change(()=>input.focus());change(()=>input.blur());
 change(()=>{div.firstChild._data='changed';});
 change(()=>document.body.insertBefore(aside,div));
 change(()=>document.body.appendChild(document.createElement('section')));
});
test('private owner and incarnation ignore forged exposed world and reset clears token',()=>{
 const env=setup(),{document,capture,read}=env;const first=read();
 const snapshot=capture.capture(document,policy.snapshot);snapshot.world={identity:{},incarnation:{}};
 assert.equal(capture.selectorFacts(snapshot,policy.selectorFacts),first);
 capture.reset(document);const second=read();assert.notEqual(second,first);
 assert.notEqual(capture.selectorFacts(snapshot,policy.selectorFacts),second);assert.equal(read(),second);
 env.reset(html);assert.notEqual(read(),second);
 const other=setup();assert.notEqual(other.read(),read());
});
test('shadow slot changes and cross-document adoption invalidate exact native inputs',()=>{
 const {document,read}=setup(),div=document.querySelector('div');let token=read();
 const root=div.attachShadow({mode:'open'});root.innerHTML='<slot></slot>';
 let next=read();assert.notEqual(next,token);assert.equal(read(),next);token=next;
 root.querySelector('slot').setAttribute('name','other');next=read();assert.notEqual(next,token);token=next;
 const other=setup();document.body.appendChild(document.adoptNode(other.document.querySelector('aside')));
 assert.notEqual(read(),token);
});
test('fresh navigator validation still rejects tightened limits on otherwise identical input',()=>{
 const {document,capture,read}=setup();const token=read();
 for(const name of ['maxNodes','maxEdges','maxAttributeUnits','maxTextUnits']){
  const snapshot=capture.capture(document,policy.snapshot);
  assert.throws(()=>capture.selectorFacts(snapshot,{...policy.selectorFacts,[name]:0}),/limit/);
 }
 assert.equal(read(),token);
});
test('fresh native capture still refuses accessor and proxy poisoning without invoking hooks',()=>{
 const {document,read}=setup();const token=read(),div=document.querySelector('div');void div.attributes.length;
 const attr=div.__attrs.find(row=>row.name==='title'),original=Object.getOwnPropertyDescriptor(attr,'value');let hooks=0;
 Object.defineProperty(attr,'value',{configurable:true,get(){hooks++;return'a';}});
 assert.throws(read,/unknown|accessor/);assert.equal(hooks,0);Object.defineProperty(attr,'value',original);
 assert.equal(read(),token);
 const attrs=div.__attrs;div.__attrs=new Proxy(attrs,{get(){hooks++;throw Error('proxy');},getOwnPropertyDescriptor(){hooks++;throw Error('proxy');}});
 assert.throws(read,/foreign|unknown/);assert.equal(hooks,0);div.__attrs=attrs;assert.equal(read(),token);
});

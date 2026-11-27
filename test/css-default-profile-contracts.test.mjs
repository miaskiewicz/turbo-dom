import {loadNativeCss} from './opt-in-helpers/native.mjs';
// Internal profile interfaces and provenance, separate from public Step4 proof.
import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {createEnvironment} from './opt-in-helpers/runtime.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {createDefaultCssProfile} from '../src/runtime/css-default-profile.mjs';
import {createSharedCssAdapter} from '../src/runtime/css-shared-adapter.mjs';
const require=createRequire(import.meta.url),native=loadNativeCss(),wasm=require('../pkg-css/turbo_dom_css.js');
function resource(backend){const policy=JSON.parse(backend.cssRuntimePolicyV1());return{preferences:()=>policy.preferences,environment:()=>policy.environment,snapshot:()=>policy.snapshot,selectorFacts:()=>policy.selectorFacts,adapter:Object.fromEntries(Object.entries(policy.adapter).map(([name,value])=>[name,()=>value])),scene:()=>policy.scene,composition:()=>policy.composition,fonts:policy.fonts};}
function options(backend=native,readWindowContext=()=>({hostPlatform:'other',width:800,height:600,devicePixelRatio:1,mediaType:'screen'})){return createDefaultCssProfile({backend,capture:createDefaultCssCapture(),readWindowContext,resourcePolicy:resource(backend)});}
function env(){return createEnvironment('<body><style>p{color:red}</style><p>x</p>');}
test('actual native and WASM profiles refuse backend and snapshot/context owner substitution',()=>{
 for(const backend of [native,wasm]){const current=options(backend),first=env(),second=env(),one=current.readContext(first.document.querySelector('p')),two=current.readContext(second.document.querySelector('p'));
 assert.equal(current.selectBackend(),backend);assert.equal(current.profile.backendIdentity(backend),1);assert.throws(()=>current.profile.backendIdentity(backend===native?wasm:native),/backend changed/);
 assert.throws(()=>options(backend).profile.userAgentSource(backend),/preferences unavailable/);assert.throws(()=>current.profile.userAgentSource(backend===native?wasm:native),/backend changed/);
 assert.throws(()=>current.captureSnapshot(second.document,one),/capture owner mismatch/);const snapshot=current.captureSnapshot(first.document,one);
 assert.throws(()=>current.profile.world(two,snapshot),/Scene context mismatch/);assert.throws(()=>current.profile.targetContext(null,{root:false,snapshot:{}}),/unknown snapshot/);
 assert.ok(current.profile.targetContext(null,{root:false,snapshot}).initialFontFamily.length>0);assert.equal(current.profile.environment(one).features,one.environment.features);
 }
});
test('interface-error fixtures omit actual methods or return invalid ABI without fabricating CSS success',()=>{
 const {document}=env(),element=document.querySelector('p');
 for(const method of ['cssComputedPreferencesV1','cssComputedEnvironmentV1']){
  const missing={...native,[method]:undefined};assert.throws(()=>options(missing).readContext(element),new RegExp(`missing ${method}`));
  const invalid={...native,[method](){return JSON.stringify({abiVersion:2});}};assert.throws(()=>options(invalid).readContext(element),/ABI/);
 }
 assert.throws(()=>createDefaultCssProfile({backend:native,capture:createDefaultCssCapture(),readWindowContext:()=>({})}),/resource policy unavailable/);
 assert.throws(()=>options(native,()=>({hostPlatform:'unknown'})).readContext(element),/host platform unavailable/);
 const original=new Error('host context');assert.throws(()=>options(native,()=>{throw original;}).readContext(element),error=>error===original);
});
test('actual adapter refuses omitted contextual collector and color views require no Scene provider',()=>{
 const {document}=env(),element=document.querySelector('p');const current=options();let solves=0;
 const adapter=createSharedCssAdapter({...current,measureScene(request){solves++;return current.measureScene(request);}});
 assert.equal(adapter.get(element).color,'rgb(255, 0, 0)');assert.equal(solves,0);adapter.dispose();
 const misconfigured=options();delete misconfigured.captureSelectorFacts;const broken=createSharedCssAdapter(misconfigured);assert.throws(()=>broken.get(element),/requires contextual routing/);broken.dispose();
});
test('unknown actual viewport context refuses Scene measurement before native layout',()=>{
 const { document } = env(), element = document.querySelector('p');
 for (const dimensions of [{}, { width: 800 }, { height: 600 }]) {
  const current = options(native, () => ({
   hostPlatform: 'other', mediaType: 'screen', devicePixelRatio: 1, ...dimensions,
  }));
  const context = current.readContext(element);
  const snapshot = current.captureSnapshot(document, context);
  assert.throws(
   () => current.measureScene({ world: current.profile.world(context, snapshot) }),
   /Scene viewport unavailable/,
  );
 }
});
test('genuine bare-host module graph exposes typed optional native proxy-admission absence',()=>{
 const script=`import vm from 'node:vm';import fs from 'node:fs';import assert from 'node:assert/strict';import {pathToFileURL,fileURLToPath} from 'node:url';const context=vm.createContext({WebAssembly:undefined}),modules=new Map();function load(url){if(modules.has(url))return modules.get(url);const module=new vm.SourceTextModule(fs.readFileSync(fileURLToPath(url),'utf8'),{context,identifier:url,initializeImportMeta(meta){meta.url=url;},importModuleDynamically(){throw Error('no Node loader');}});modules.set(url,module);return module;}const entry=load(pathToFileURL(process.cwd()+'/src/runtime/css-host-identity.mjs').href);await entry.link((name,from)=>load(new URL(name,from.identifier).href));await entry.evaluate();assert.throws(()=>entry.namespace.isCssHostProxy({}),error=>error.name==='CssHostIdentityUnavailable');assert.equal(vm.runInContext('typeof WebAssembly',context),'undefined');assert.equal(vm.runInContext('typeof process',context),'undefined');`;
 const result=spawnSync(process.execPath,['--experimental-vm-modules','--input-type=module','-e',script],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);
});
test('actual host loader failure is retained and rethrown by identity',()=>{
 const script=`import assert from 'node:assert/strict';import{createRequire}from'node:module';const Module=createRequire(import.meta.url)('node:module'),load=Module._load,apply=Reflect.apply;const original=new Error('host util unavailable');let calls=0;try{Module._load=function(name,parent){if(name==='node:util'&&parent?.filename?.endsWith('/src/runtime/parser.mjs')){calls++;throw original;}return apply(load,this,arguments);};const api=await import('./src/runtime/css-host-identity.mjs');assert.throws(()=>api.isCssHostProxy({}),error=>error===original);assert.equal(calls,1);}finally{Module._load=load;}assert.equal(Module._load,load);`;
 const result=spawnSync(process.execPath,['--input-type=module','-e',script],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);
});

test('mismatched backend preference identity is refused and never becomes a cached environment', () => {
 const { document } = env(), element = document.querySelector('p');
 let mismatch = true, reads = 0;
 const backend = {
  ...native,
  cssComputedEnvironmentV1(input) {
   reads++;
   const environment = JSON.parse(native.cssComputedEnvironmentV1(input));
   if (mismatch) environment.preferenceIdentity = ['mismatched-backend-identity'];
   return JSON.stringify(environment);
  },
 };
 const current = options(backend);
 assert.throws(() => current.readContext(element), /preference identity changed/);
 assert.equal(reads, 1);
 mismatch = false;
 const first = current.readContext(element);
 const second = current.readContext(element);
 assert.deepEqual(first.environment.preferenceIdentity, first.preferences.preferenceIdentity);
 assert.equal(first.environment, second.environment);
 assert.equal(reads, 2);
});

import {captureStyleStatePolicy} from './css-native-style-state.mjs';
// The explicit Rust CSS opt-in routes computed style through the shared capability.
// Optional internal callers may retain the legacy fallback when it is absent.
import {makeGetComputedStyle as makeLegacyComputedStyle} from './cascade.mjs';
import {getParserSelection,createParserRequire} from './parser.mjs';
import {createCssArtifactResolver} from './css-artifact.mjs';
import {createDefaultSharedCssAdapter} from './css-shared-adapter.mjs';
import {createDefaultCssCapture,nativeDocument,nativeCssConnected} from './css-default-capture.mjs';
import {createDefaultCssProfile} from './css-default-profile.mjs';
import {readNativeCssWindowContext,nativeCssInstalledCodecSafe} from './window.mjs';
import {parserDecoderSafe,originalTextCodecDescriptor,originalWasmAvailability} from './parser-origin.mjs';
import {createComputedStyleProxy} from './css-computed-proxy.mjs';
import {sharedInvoke} from './css-shared-codec.mjs';
import {createOwnedJsonCssWire} from './css-rust-wire.mjs';
const NativeWeakMap=WeakMap, apply=Reflect.apply, mapGet=WeakMap.prototype.get, mapSet=WeakMap.prototype.set;
const descriptor=Object.getOwnPropertyDescriptor, hasOwn=Object.hasOwn;
let artifactResolver;
const documents=new NativeWeakMap();
export class CssDefaultUnavailable extends Error {
 constructor(capability){super(`turbo-dom: default CSS capability unavailable: ${capability}`);this.name='CssDefaultUnavailable';}
}
function beforeWasmLoad(){
 const available=originalWasmAvailability();
 if(available===false)return;
 if(available!==true)throw new CssDefaultUnavailable('WASM bootstrap');
 if(!parserDecoderSafe({cachedTextCodecs:true}))throw new CssDefaultUnavailable('WASM codec dependencies');
 const codecs=['TextEncoder','TextDecoder'];
 for(let index=0;index<codecs.length;index++){
  const name=codecs[index],field=descriptor(globalThis,name);
  if(!originalTextCodecDescriptor(name,field)&&!nativeCssInstalledCodecSafe(name,field))throw new CssDefaultUnavailable(`WASM ${name}`);
 }
}
function runtimePolicy(backend,codec,nativeStateMode){
 if(typeof backend.cssRuntimePolicyV1!=='function')throw new CssDefaultUnavailable('cssRuntimePolicyV1');
 const rawPolicy=sharedInvoke(backend.cssRuntimePolicyV1,backend,[]);
 const statePolicy=nativeStateMode?captureStyleStatePolicy(rawPolicy):undefined;
 const policy=codec.decode(rawPolicy,'cssRuntimePolicyV1');
 if(policy.abiVersion!==1)throw new TypeError('turbo-dom: incompatible default CSS policy ABI');
 const adapter={};
 for(const name of ['compile','prepare','matching','contextualMatching','resolve','serialize','property','providerSnapshot']){
  if(!policy.adapter||!hasOwn(policy.adapter,name))throw new CssDefaultUnavailable(`policy.adapter.${name}`);
  const value=policy.adapter[name];codec.policyField(adapter,name);adapter[name]=()=>value;
 }
 if(nativeStateMode&&statePolicy===undefined)throw new CssDefaultUnavailable('native-state-v1 fixed projection policy');
 return {statePolicy,preferences:()=>policy.preferences,environment:()=>policy.environment,snapshot:()=>policy.snapshot,selectorFacts:()=>policy.selectorFacts,adapter,scene:()=>policy.scene,composition:()=>policy.composition,fonts:policy.fonts};
}
function hostPlatform(){
 const require=createParserRequire();
 if(!require)throw new CssDefaultUnavailable('host platform');
 const process=require('node:process'),field=descriptor(process,'platform');
 if(!field||!hasOwn(field,'value')||typeof field.value!=='string')throw new CssDefaultUnavailable('host platform');
 return field.value==='darwin'?'macos':'other';
}
function readWindowContext(document){return{...readNativeCssWindowContext(document),hostPlatform:hostPlatform(),mediaType:'screen'};}
function documentAdapter(document,backend,nativeStateMode,requestState){
 let entry=apply(mapGet,documents,[document]);
 if(!entry||entry.backend!==backend||entry.nativeStateMode!==nativeStateMode||entry.requestState!==requestState){
  if(entry)entry.adapter.dispose();
  const capture=createDefaultCssCapture(),modelKeys=createOwnedJsonCssWire(),resourcePolicy=runtimePolicy(backend,modelKeys,nativeStateMode);
  const options=createDefaultCssProfile({backend,capture,readWindowContext,resourcePolicy,decodeResult:modelKeys.decode});
  entry={backend,nativeStateMode,requestState,maxDepth:resourcePolicy.snapshot().maxDepth,adapter:createDefaultSharedCssAdapter(options,true,true,modelKeys,resourcePolicy.statePolicy,requestState)};
  apply(mapSet,documents,[document,entry]);
 }
 return entry;
}
export function makeDefaultGetComputedStyle({requireBackend=false,nativeStateMode=false,requestState=false}={}){
 const legacy=makeLegacyComputedStyle();
 return function getComputedStyle(element,pseudo){
  if(element===null)return createComputedStyleProxy({values:{},custom:{}},0);
  artifactResolver??=createCssArtifactResolver({beforeWasmLoad});
  const backend=artifactResolver.resolve(getParserSelection());
  if(backend===null){if(requireBackend)throw new CssDefaultUnavailable('Rust CSS artifact');return legacy(element,pseudo);}
  const document=nativeDocument(element);
  const entry=documentAdapter(document,backend,nativeStateMode,requestState);
  if(!nativeCssConnected(element,document,entry.maxDepth))return createComputedStyleProxy({values:{},custom:{}},0);
  return entry.adapter.get(element,pseudo);
 };
}

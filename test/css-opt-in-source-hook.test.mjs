import {loadNativeCss} from './opt-in-helpers/native.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {createEnvironment} from '../src/runtime/index.mjs';
import {createSharedCssAdapter} from '../src/runtime/css-shared-adapter.mjs';
import {createDefaultCssCapture} from '../src/runtime/css-default-capture.mjs';
import {createDefaultCssProfile} from '../src/runtime/css-default-profile.mjs';
import {readNativeCssWindowContext} from '../src/runtime/window.mjs';
const backend=loadNativeCss();
function setup(){
 const env=createEnvironment('<style>p{color:red}</style><p>x</p>',{cssBackend:'rust'});
 const policy=JSON.parse(backend.cssRuntimePolicyV1());
 const resourcePolicy={adapter:Object.fromEntries(Object.entries(policy.adapter).map(([key,value])=>[key,()=>value])),fonts:policy.fonts};
 for(const key of ['preferences','environment','snapshot','selectorFacts','scene','composition'])resourcePolicy[key]=()=>policy[key];
 const options=createDefaultCssProfile({backend,capture:createDefaultCssCapture(),resourcePolicy,readWindowContext:document=>({...readNativeCssWindowContext(document),hostPlatform:'macos',mediaType:'screen'})});
 return {env,options,target:env.document.querySelector('p')};
}
test('generic adapter retains ordinary source discovery when trusted source hook is absent',()=>{
 const {options,target}=setup(); delete options.captureSourceFacts;
 const adapter=createSharedCssAdapter(options);
 try{assert.equal(adapter.get(target).color,'rgb(255, 0, 0)');}finally{adapter.dispose();}
});
test('trusted source hook validates its type and preserves original thrown error',()=>{
 const {options,target}=setup();
 assert.throws(()=>createSharedCssAdapter({...options,captureSourceFacts:42}),/captureSourceFacts/);
 const error=new TypeError('trusted source hook sentinel');let calls=0;
 const adapter=createSharedCssAdapter({...options,captureSourceFacts(){calls++;throw error;}});
 try{assert.throws(()=>adapter.get(target),caught=>caught===error);assert.equal(calls,1);}finally{adapter.dispose();}
});
test('public opt-in rejects unknown adopted-sheet proxies without invoking traps',()=>{
 const {env,target}=setup();let calls=0;
 env.document.adoptedStyleSheets=[new Proxy({}, {get(){calls++;throw Error('guest trap');},getOwnPropertyDescriptor(){calls++;throw Error('guest trap');},getPrototypeOf(){calls++;throw Error('guest trap');}})];
 assert.throws(()=>env.window.getComputedStyle(target),/native CSS sheet unknown/);assert.equal(calls,0);
});

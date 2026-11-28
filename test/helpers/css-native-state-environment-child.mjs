import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const native=require('../../native-css/turbo-dom-css.darwin-arm64.node');
const calls={resolve:0,fused:0,serialize:0};
function observe(owner,name,key){const original=owner[name];owner[name]=function(...args){calls[key]++;return Reflect.apply(original,this,args)}}
observe(native.PreparedStylesV1.prototype,'resolveMatchingV1','resolve');
observe(native.PreparedStylesV1.prototype,'resolveStyleStateV1','fused');
observe(native,'serializeCssomV1','serialize');
const fault=process.argv[3];
if(fault==='missing-store')delete native.PreparedStylesV1.prototype.storeMatchingV1;
if(fault==='missing-resolve')delete native.PreparedStylesV1.prototype.resolveMatchingV1;
if(fault==='missing')delete native.PreparedStylesV1.prototype.resolveStyleStateV1;
if(fault==='throw-method')native.PreparedStylesV1.prototype.resolveStyleStateV1=function(){throw Error('native-state override sentinel')};
if(['object-policy','extra-quota','malformed-policy'].includes(fault)){
 const original=native.cssRuntimePolicyV1;
 native.cssRuntimePolicyV1=function(){
  if(fault==='malformed-policy')return 'bad';
  const value=JSON.parse(Reflect.apply(original,this,[]));
  if(fault==='object-policy')return value;
  value.adapter.serialize.extraQuota=0;return JSON.stringify(value);
 };
}
const {createEnvironment}=await import('../../src/runtime/index.mjs');
const mode=process.argv[2];
const html='<!doctype html><html><head><style>#p{width:200px;color:rgb(12,34,56)}#c{color:inherit;width:50%}</style></head><body><div id=p><div id=c></div></div></body></html>';
const env=createEnvironment(html,{cssBackend:'rust',cssResultProtocol:mode,parser:'native'});
const child=env.document.getElementById('c');
if(fault){
 let error;try{env.window.getComputedStyle(child)}catch(caught){error=caught}
 assert.ok(error instanceof Error);
 if(fault==='missing-store'||fault==='missing-resolve')assert.match(error.message,/retained matching capability/);
 if(fault==='missing')assert.match(error.message,/requires resolveStyleStateV1/);
 if(fault==='throw-method')assert.equal(error.message,'native-state override sentinel');
 if(fault==='object-policy'||fault==='extra-quota')assert.match(error.message,/fixed projection policy/);
 assert.equal(calls.resolve,0);assert.equal(calls.serialize,0);
 console.log(JSON.stringify({fault,error:error.message,calls}));process.exit(0);
}
const old=env.window.getComputedStyle(child);
assert.equal(old.color,'rgb(12, 34, 56)');assert.equal(old.width,'100px');
const before=Object.keys(require.cache).filter(path=>path.endsWith('.node'));
env.reset('<!doctype html><html><head><style>#c{color:blue;width:25px}</style></head><body><div id=c></div></body></html>');
const fresh=env.window.getComputedStyle(env.document.getElementById('c'));
assert.equal(fresh.color,'rgb(0, 0, 255)');assert.equal(fresh.width,'25px');assert.equal(old.color,'rgb(12, 34, 56)');
const after=Object.keys(require.cache).filter(path=>path.endsWith('.node'));
assert.equal(before.length,1);assert.deepEqual(after,before);assert.match(before[0],/native-css/);
if(mode==='native-state-v1'){assert.ok(calls.fused>=2);assert.equal(calls.serialize,0)}else{assert.equal(calls.fused,0);assert.ok(calls.resolve>=2);assert.ok(calls.serialize>=2)}
console.log(JSON.stringify({mode,calls,modules:after,color:fresh.color,width:fresh.width,oldColor:old.color}));

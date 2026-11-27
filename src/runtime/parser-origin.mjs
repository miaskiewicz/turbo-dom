// The module loader and bootstrap intrinsics are trusted host infrastructure.
// Arbitrary loader tampering cannot be authenticated in portable JavaScript.
// No injected ParserBinding, mode/name/boolean or caller object grants origin.
const descriptor = Object.getOwnPropertyDescriptor;
const create = Object.create;
const names = Object.getOwnPropertyNames;
const prototype = Object.getPrototypeOf;
const freeze = Object.freeze;
const hasOwn = Object.hasOwn;
const apply = Reflect.apply;
const charCode = String.prototype.charCodeAt;
const rawKeys = ['nodeType','ns','tagId','parent','firstChild','nextSib','textId','pubId','sysId','attrStart','attrCount','attrNameId','attrValueId','attrPrefixId','tagNames','attrNames','attrPrefixes','attrValues','strings'];
const packedKeys = ['n','m','packed','strMeta','strBlob'];
const originalTextCodecs={TextEncoder:typeof TextEncoder==='undefined'?undefined:TextEncoder,TextDecoder:typeof TextDecoder==='undefined'?undefined:TextDecoder};
const constructors = ['Array','Object','String','Uint8Array','Uint16Array','Uint32Array','Int32Array','DataView','TextEncoder','TextDecoder'];
const globals = create(null);
const arrayPrototype=Array.prototype, objectPrototype=Object.prototype;
const guarded = [];
function guard(object, property) { guarded.push({object,property,original:descriptor(object,property)}); }
guard(arrayPrototype,'constructor');
for(const key of ['fromCharCode','fromCodePoint']) guard(String,key);
for(const key of ['charCodeAt','slice']) guard(String.prototype,key);
if(typeof TextEncoder!=='undefined') for(const key of ['encode','encodeInto']) guard(TextEncoder.prototype,key);
if(typeof TextDecoder!=='undefined') guard(TextDecoder.prototype,'decode');
for(const key of ['setInt32','getInt32']) guard(DataView.prototype,key);
const byteConstructor=Uint8Array,bytePrototype=byteConstructor.prototype,metaPrototype=Uint32Array.prototype,typedPrototype=prototype(bytePrototype),typedConstructor=prototype(byteConstructor);
const typedBufferGetter=descriptor(typedPrototype,'buffer').get;
for(const key of ['set','subarray']) {guard(bytePrototype,key);guard(typedPrototype,key);}
guard(bytePrototype,'constructor');guard(Uint8Array,Symbol.species);guard(typedConstructor,Symbol.species);
for(const key of ['length','byteLength','byteOffset','buffer']) {guard(typedPrototype,key);guard(bytePrototype,key);}
const wasmBootstrap=descriptor(globalThis,'WebAssembly')?.value;
const wasmModuleField=wasmBootstrap?descriptor(wasmBootstrap,'Module'):undefined,wasmInstanceField=wasmBootstrap?descriptor(wasmBootstrap,'Instance'):undefined;
if(wasmBootstrap) {
 for(const key of ['get','set','grow']) guard(wasmBootstrap.Table.prototype,key);
 guard(wasmBootstrap.Memory.prototype,'buffer');
}
for (let index=0;index<constructors.length;index++) globals[constructors[index]]=descriptor(globalThis,constructors[index]);
const allFields = [...rawKeys, ...packedKeys];
function same(left,right) {
 if(!left||!right)return left===right;
 const data=hasOwn(left,'value');
 if(data!==hasOwn(right,'value'))return false;
 return data ? left.value===right.value : left.get===right.get&&left.set===right.set;
}
function numericKey(key) {
 if(key.length===0)return false;
 for(let index=0;index<key.length;index++){const code=apply(charCode,key,[index]);if(code<48||code>57)return false;}
 return key==='0'||key[0]!=='0';
}
function numericKeys(object) {
 const keys=names(object);
 for(let index=0;index<keys.length;index++) if(numericKey(keys[index])) return true;
 return false;
}
function parserGlobalsSafe(cachedTextCodecs) {
 for(let index=0;index<constructors.length;index++) {const key=constructors[index];if(cachedTextCodecs&&(key==='TextEncoder'||key==='TextDecoder'))continue;if(!same(descriptor(globalThis,key),globals[key]))return false;}
 return true;
}
function parserPrototypeSafe() {
 return prototype(arrayPrototype)===objectPrototype && prototype(objectPrototype)===null && prototype(bytePrototype)===typedPrototype && prototype(byteConstructor)===typedConstructor && !numericKeys(arrayPrototype) && !numericKeys(objectPrototype);
}
export function parserDecoderSafe({cachedTextCodecs=false} = {}) {
 if(!parserPrototypeSafe())return false;
 if(!parserGlobalsSafe(cachedTextCodecs))return false;
 for(let index=0;index<guarded.length;index++){const entry=guarded[index];if(!same(descriptor(entry.object,entry.property),entry.original))return false;}
 for(let index=0;index<allFields.length;index++) if(descriptor(objectPrototype,allFields[index]))return false;
 return true;
}
export function originalWasmAvailability(){
 const field=descriptor(globalThis,'WebAssembly');
 if(!field||(hasOwn(field,'value')&&field.value===undefined))return false;
 if(!hasOwn(field,'value')||!wasmBootstrap||field.value!==wasmBootstrap)return null;
 return same(descriptor(wasmBootstrap,'Module'),wasmModuleField)&&same(descriptor(wasmBootstrap,'Instance'),wasmInstanceField)?true:null;
}
export function originalTextCodecValue(name,value) {
 return value!==undefined&&hasOwn(originalTextCodecs,name)&&originalTextCodecs[name]===value;
}
export function originalTextCodecDescriptor(name,field) {
 return !!field&&hasOwn(field,'value')&&originalTextCodecValue(name,field.value);
}
const packedViewKeys=['buffer','byteOffset'],blobViewKeys=['subarray','constructor'];
function decoderViewSafe(view,expected,keys){
 try{apply(typedBufferGetter,view,[]);}catch{return false;}
 if(prototype(view)!==expected)return false;
 for(let index=0;index<keys.length;index++)if(descriptor(view,keys[index])!==undefined)return false;
 return true;
}
function rawDecodeSafe(fields){
 if(!hasOwn(fields,'packed'))return true;
 return decoderViewSafe(fields.packed,bytePrototype,packedViewKeys)&&decoderViewSafe(fields.strBlob,bytePrototype,blobViewKeys)&&decoderViewSafe(fields.strMeta,metaPrototype,[]);
}
function originalFields(soa) {
 const keys=descriptor(soa,'packed') ? packedKeys : rawKeys;
 const fields=create(null);
 for(let index=0;index<keys.length;index++) {
  const key=keys[index],value=descriptor(soa,key);
  if(!value || !hasOwn(value,'value')) return null;
  fields[key]=value.value;
 }
 return freeze(fields);
}
export function fieldsMatch(soa,fields) {
 if(!hasOwn(fields,'packed')&&descriptor(soa,'packed'))return false;
 const keys=names(fields);
 for(let index=0;index<keys.length;index++) {const key=keys[index],actual=descriptor(soa,key);if(!actual||!hasOwn(actual,'value')||actual.value!==fields[key])return false;}
 return true;
}
export function captureParserOrigin(soa, valid) {
 if(soa===null||typeof soa!=='object')return null;
 const fields=originalFields(soa);
 if(!fields||!rawDecodeSafe(fields))return null;
 let revoked=false;
 // Codec/global dependencies authenticate production, not later reads of the
 // already-decoded stores. Captures still verify original fields and own cells.
 const current=()=>{if(!valid())revoked=true;return !revoked;};
 const matches=()=>{if(!fieldsMatch(soa,fields))revoked=true;return !revoked;};
 return freeze({soa,fields,valid:current,matches,decodeSafe:()=>rawDecodeSafe(fields)});
}
export function normalizedParserOrigin(soa, source) {
 if(!source || !source.valid() || !source.matches())return null;
 const fields=originalFields(soa);
 if(!fields)return null;
 let revoked=false;
 const current=()=>{if(!source.valid())revoked=true;return !revoked;};
 const matches=()=>{if(!source.matches()||!fieldsMatch(soa,fields))revoked=true;return !revoked;};
 return freeze({soa,fields,length:fields.nodeType.length,valid:current,matches});
}

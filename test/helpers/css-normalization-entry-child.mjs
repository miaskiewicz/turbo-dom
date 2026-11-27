import {parseBufferForRuntime,readParserCssOrigin,setParserMode} from '../../src/runtime/parser.mjs';
import {Document} from '../../src/runtime/dom.mjs';
import {createWindow} from '../../src/runtime/window.mjs';
import {Buffer,unpack,readNativeCssBuffer} from '../../src/runtime/buffer.mjs';
import {createEnvironment} from '../opt-in-helpers/runtime.mjs';
setParserMode('native');
const kind=process.argv[2],raw=parseBufferForRuntime('<style>body{color:red}</style><p data-x="y">λ</p>'),source=readParserCssOrigin(raw),originalArray=Array,construct=Reflect.construct,describe=Object.getOwnPropertyDescriptor,apply=Reflect.apply;
if(!raw.packed||!source)throw Error('actual certified native packed transport required');
let phase='load',callbacks=0,privateCalls=0,arrayConstructions=0,restores=[],input=raw,originalError=new Error('public decode original error'),privateEntrySafe;
const note=()=>{callbacks++;if(phase==='capture')privateCalls++;};
function shadow(view,key,get){const previous=describe(view,key);restores.push(()=>previous?Object.defineProperty(view,key,previous):delete view[key]);Object.defineProperty(view,key,{configurable:true,get});}
if(kind==='entry-aba'){
 const packed=describe(raw,'packed');let reads=0;const replacement=new Proxy(originalArray,{construct(target,args){const value=construct(target,args,target);if(++arrayConstructions===5)globalThis.Array=originalArray;return new Proxy(value,{getOwnPropertyDescriptor(target,key){if(phase==='capture')privateCalls++;return describe(target,key);}});}});
 shadow(raw,'packed',()=>{callbacks++;if(++reads===1)return packed.value;Object.defineProperty(raw,'packed',packed);globalThis.Array=replacement;return packed.value;});
}else if(kind==='normalized-aba'){
 input=unpack(raw);shadow(input,'packed',()=>{note();delete input.packed;return undefined;});
}else if(kind==='blob-subarray'){
 const method=raw.strBlob.subarray;shadow(raw.strBlob,'subarray',()=>{note();return method;});
}else if(kind==='packed-buffer'||kind==='packed-offset'){
 const key=kind==='packed-buffer'?'buffer':'byteOffset',value=raw.packed[key];shadow(raw.packed,key,()=>{note();return value;});
}else if(kind==='blob-prototype'){
 const value=raw.strBlob,prototype=Object.getPrototypeOf(value);restores.push(()=>Object.setPrototypeOf(value,prototype));Object.setPrototypeOf(value,new Proxy(prototype,{get(){note();throw originalError;},getOwnPropertyDescriptor(){note();throw originalError;},getPrototypeOf(){note();throw originalError;}}));
}else if(kind==='view-proxy'){
 const value=raw.strBlob;restores.push(()=>{raw.strBlob=value;});raw.strBlob=new Proxy(value,{get(){note();throw originalError;},getOwnPropertyDescriptor(){note();throw originalError;},getPrototypeOf(){note();throw originalError;}});
}else throw Error('unknown case');
// Private entry checks must not reflect into a replaced unknown view/prototype.
if(kind==='blob-prototype'||kind==='view-proxy'){phase='capture';privateEntrySafe=source.matches()&&source.decodeSafe();phase='load';}
let document,loadError,originalErrorPreserved=false;
try{document=new Document();document.__load(input);}catch(error){loadError={name:error.name,message:error.message};originalErrorPreserved=error===originalError;}
finally{globalThis.Array=originalArray;for(let i=restores.length-1;i>=0;i--)restores[i]();}
phase='capture';let admitted=false,admissionError,cssError;
if(document?.__buf){try{readNativeCssBuffer(document.__buf);admitted=true;}catch(error){admissionError={name:error.name,message:error.message};}try{createWindow(document).window.getComputedStyle(document.body).color;}catch(error){cssError={name:error.name,message:error.message};}}
let normalizedStillUnknown=null;if(kind==='normalized-aba'){try{readNativeCssBuffer(new Buffer(input));normalizedStillUnknown=false;}catch{normalizedStillUnknown=true;}}
const future=createEnvironment('<body>fresh clean parser output');let cleanFutureAdmitted=false;try{readNativeCssBuffer(future.document.__buf);cleanFutureAdmitted=true;}catch{}
console.log(JSON.stringify({kind,callbacks,privateCalls,arrayConstructions,privateEntrySafe,admitted,admissionError,cssError,loadError,originalErrorPreserved,normalizedStillUnknown,cleanFutureAdmitted,restored:Array===originalArray}));

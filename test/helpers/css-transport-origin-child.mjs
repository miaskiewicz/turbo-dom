// Active built-in transport dependencies only; no injected parser/backend.
import {createEnvironment} from '../opt-in-helpers/runtime.mjs';
import {getParser,parseBufferForRuntime,readParserCssOrigin} from '../../src/runtime/parser.mjs';
import {parserDecoderSafe} from '../../src/runtime/parser-origin.mjs';
import {readNativeCssBuffer} from '../../src/runtime/buffer.mjs';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),kind=process.argv[2],events=[],apply=Reflect.apply,bytePrototype=Uint8Array.prototype,typedPrototype=Object.getPrototypeOf(bytePrototype),byteCtor=Uint8Array;
getParser();const old=createEnvironment('<body>already decoded');readNativeCssBuffer(old.document.__buf);
let env,creationError,safeWhileChanged,restore;
const callback=()=>events.push(new Error('active typed-array transport callback').stack);
try{
 if(kind==='subarray'){const field=Object.getOwnPropertyDescriptor(typedPrototype,'subarray');restore=()=>Object.defineProperty(typedPrototype,'subarray',field);Object.defineProperty(typedPrototype,'subarray',{...field,value:function(...args){callback();return apply(field.value,this,args);}});}
 else if(kind==='constructor'||kind==='species'){const owner=kind==='constructor'?bytePrototype:byteCtor,key=kind==='constructor'?'constructor':Symbol.species,field=Object.getOwnPropertyDescriptor(owner,key);restore=()=>field?Object.defineProperty(owner,key,field):delete owner[key];Object.defineProperty(owner,key,{configurable:true,get(){callback();return byteCtor;}});}
 else if(kind==='chain-proto'||kind==='chain-ctor'){const owner=kind==='chain-proto'?bytePrototype:byteCtor,original=Object.getPrototypeOf(owner);restore=()=>Object.setPrototypeOf(owner,original);Object.setPrototypeOf(owner,new Proxy(original,{get(){callback();throw Error('proxy trap');},getOwnPropertyDescriptor(){callback();throw Error('proxy trap');},getPrototypeOf(){callback();throw Error('proxy trap');}}));}
 else if(kind==='shadow'){const field=Object.getOwnPropertyDescriptor(bytePrototype,'subarray');restore=()=>field?Object.defineProperty(bytePrototype,'subarray',field):delete bytePrototype.subarray;Object.defineProperty(bytePrototype,'subarray',{configurable:true,get(){callback();throw Error('shadow getter');}});}
 else throw Error('unknown probe');
 safeWhileChanged=parserDecoderSafe({cachedTextCodecs:true});
 if(['subarray','constructor','species'].includes(kind))env=createEnvironment('<body><p>λ-😀-'+kind+'</p>');
}catch(error){creationError={name:error.name,message:error.message};}finally{restore();}
let affectedAdmitted=false,admissionError,oldAdmitted=false,cssError;
try{readNativeCssBuffer(old.document.__buf);oldAdmitted=true;}catch{}
if(env){try{readNativeCssBuffer(env.document.__buf);affectedAdmitted=true;}catch(error){admissionError={name:error.name,message:error.message};}try{env.window.getComputedStyle(env.document.querySelector('p')).color;}catch(error){cssError={name:error.name,message:error.message};}}
const clean=createEnvironment('<body>separate restored clean parse');let cleanAdmitted=false;try{readNativeCssBuffer(clean.document.__buf);cleanAdmitted=true;}catch{}
const raw=parseBufferForRuntime('<body>restored raw input'),origin=readParserCssOrigin(raw);
const hashes={};for(const file of ['src/runtime/parser-origin.mjs','src/runtime/buffer.mjs','src/runtime/parser.mjs','pkg/turbo_dom_parser.js','pkg/turbo_dom_parser_bg.wasm'])hashes[file]=createHash('sha256').update(readFileSync(file)).digest('hex');
console.log(JSON.stringify({kind,mode:process.env.TURBO_DOM_PARSER,callbacks:events.length,events,safeWhileChanged,creationError,parsed:env?.document.querySelector('p')?.textContent,affectedAdmitted,admissionError,cssError,oldAdmitted,cleanAdmitted,cleanRawOrigin:!!origin&&origin.valid(),restoredSafe:parserDecoderSafe({cachedTextCodecs:true}),hashes,modulePaths:Object.keys(require.cache).filter(path=>path.includes('turbo_dom_parser')||path.endsWith('.node'))}));

// Authentic built-in parser initialization; no injected parser or CSS backend.
import {createEnvironment} from '../opt-in-helpers/runtime.mjs';
import {getParser,setParserMode,parseBufferForRuntime,readParserCssOrigin} from '../../src/runtime/parser.mjs';
import {readNativeCssBuffer} from '../../src/runtime/buffer.mjs';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),scenario=process.argv[2],events=[];
const wasm=globalThis.WebAssembly,owner=scenario==='global'||scenario==='native'?globalThis:wasm;
const key=scenario==='global'||scenario==='native'?'WebAssembly':scenario==='instance'?'Instance':'Module';
const original=Object.getOwnPropertyDescriptor(owner,key),value=owner[key];
const before=Object.keys(require.cache);if(before.some(path=>path.includes('turbo_dom_parser.js')||path.endsWith('.node')))throw Error('plain parser unexpectedly warm');
let first,previousAdmitted=null;
if(scenario==='cached'||scenario==='reload'){
 first=createEnvironment('<body>first admitted');readNativeCssBuffer(first.document.__buf);previousAdmitted=true;
}
let env,parsingError;
try{
 if(scenario!=='clean')Object.defineProperty(owner,key,{configurable:true,get(){events.push(new Error('constructor callback').stack);return value;}});
 if(scenario==='reload')delete require.cache[require.resolve('../../pkg/turbo_dom_parser.js')];
 if(scenario==='cached'||scenario==='reload')setParserMode('wasm');
 env=createEnvironment('<body><p>new real parse</p>');
}catch(error){parsingError={name:error.name,message:error.message};}
finally{if(scenario!=='clean')Object.defineProperty(owner,key,original);}
let admitted=false,admissionError,subsequentOrigin=false,cssResult,cssError;
if(env){try{readNativeCssBuffer(env.document.__buf);admitted=true;}catch(error){admissionError={name:error.name,message:error.message};}
 try{cssResult=env.window.getComputedStyle(env.document.querySelector('p')).color;}catch(error){cssError={name:error.name,message:error.message};}
 const raw=parseBufferForRuntime('<body>separate subsequent parse');const origin=readParserCssOrigin(raw);subsequentOrigin=!!origin&&origin.valid();}
let oldOriginStillValid=null;if(first){try{readNativeCssBuffer(first.document.__buf);oldOriginStillValid=true;}catch{oldOriginStillValid=false;}}
console.log(JSON.stringify({scenario,mode:process.env.TURBO_DOM_PARSER,callbacks:events.length,events,parsingError,parsed:env?.document.querySelector('p')?.textContent,admitted,admissionError,cssResult,cssError,subsequentOrigin,previousAdmitted,oldOriginStillValid,restored:Object.getOwnPropertyDescriptor(owner,key).value===original.value,modulePaths:Object.keys(require.cache).filter(path=>path.includes('turbo_dom_parser')||path.endsWith('.node')),bindingKeys:Object.keys(getParser())}));

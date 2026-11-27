import {createRequire, Module} from 'node:module';
import {pathToFileURL} from 'node:url';
const [root,scenario]=process.argv.slice(2),require=createRequire(import.meta.url);
const runtime=await import(pathToFileURL(`${root}/src/runtime/index.mjs`));
const {createEnvironment,getParser,setParser}=runtime;
const html='<style>p{color:red}</style><p>sample</p>';
let selected,first,after,beforeCss,error,originError,facadeKeys,iteratorCalls,exportGetterCalls=0;
const originalLoad=Module._load;
if(scenario.startsWith('export-')) {
 Module._load=function(spec,...args){
  const binding=Reflect.apply(originalLoad,this,[spec,...args]);
  if(!String(spec).includes('native-css'))return binding;
  const malformed={...binding};
  if(scenario==='export-missing')delete malformed.parseRaw;
  if(scenario==='export-value')malformed.parseRaw=0;
  if(scenario==='export-accessor')Object.defineProperty(malformed,'parseRaw',{get(){exportGetterCalls++;return binding.parseRaw;}});
  return malformed;
 };
}
try {
 if(scenario==='iterator'){
  const parserAPI=await import(pathToFileURL(`${root}/src/runtime/parser.mjs`));
  const original=Object.getOwnPropertyDescriptor(Array.prototype,Symbol.iterator);iteratorCalls=0;
  Object.defineProperty(Array.prototype,Symbol.iterator,{configurable:true,writable:true,value(){iteratorCalls++;throw Error('unexpected iterator');}});
  try {parserAPI.parseBufferForRuntime(html,true);} finally {Object.defineProperty(Array.prototype,Symbol.iterator,original);}
 }
 if(scenario==='selected')selected=getParser();
 if(scenario==='injected'){selected=require(`${root}/index.js`);setParser(selected);}
 if(scenario==='global'){selected=require(`${root}/index.js`);globalThis.__TURBO_DOM_PARSER__=selected;}
 const env=createEnvironment(html,scenario==='js'?{}:{cssBackend:'rust'});
 beforeCss=Object.keys(require.cache).filter(path=>path.endsWith('.node'));
 first=env.document.querySelector('p').textContent;
 try {after=env.window.getComputedStyle(env.document.querySelector('p')).color;}
 catch(failure){originError={name:failure.name,message:failure.message};}
 if(scenario==='mixed'){
  const parser=getParser();const js=createEnvironment('<p>second</p>');
  if(getParser()!==parser)throw Error('mixed parser replaced');
  first+='|'+js.document.querySelector('p').textContent;
 }
 if(scenario==='reset'){env.reset('<style>p{color:blue}</style><p>reset</p>');after=env.window.getComputedStyle(env.document.querySelector('p')).color;first=env.document.querySelector('p').textContent;}
 if(scenario==='changed'){
  const parser=getParser(),original=parser.parseBuffer;
  parser.parseBuffer=(...args)=>original(...args);
  const next=createEnvironment('<style>p{color:blue}</style><p>changed</p>',{cssBackend:'rust'});
  try{next.window.getComputedStyle(next.document.querySelector('p'));throw Error('unsafe builtin admitted');}
  catch(failure){originError={name:failure.name,message:failure.message};}
 }
 facadeKeys=Object.keys(getParser()).sort();
} catch(failure){error={name:failure.name,message:failure.message,code:failure.code};}
Module._load=originalLoad;
console.log(JSON.stringify({exportGetterCalls,iteratorCalls,first,after,beforeCss,error,originError,facadeKeys,selectedPreserved:selected?selected===getParser():null,nativeModules:Object.keys(require.cache).filter(path=>path.endsWith('.node'))}));

import {afterAll} from 'vitest';
import {createRequire} from 'node:module';
import {writeFileSync} from 'node:fs';
if(process.env.CONSUMER_ENV==='turbo-dom'){
 const {getParser}=await import('turbo-dom/runtime');const binding=getParser();
 const require=createRequire(import.meta.url);
 afterAll(()=>{const modules=Object.keys(require.cache).filter(path=>path.endsWith('.node'));if(process.env.TURBO_DOM_PARSER==='wasm'&&modules.length)throw new Error('forced WASM loaded native module');writeFileSync(`mode-${process.env.TURBO_DOM_PARSER}-${process.pid}.json`,JSON.stringify({requested:process.env.TURBO_DOM_PARSER,parserKeys:Object.keys(binding),nativeModules:modules,environment:import.meta.resolve('turbo-dom/environment/vitest')}));});
}

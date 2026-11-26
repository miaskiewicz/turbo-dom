import {defineConfig} from 'vitest/config';
import {fileURLToPath} from 'node:url';
import {writeFileSync} from 'node:fs';
const mode=process.env.CONSUMER_ENV??'turbo-dom';
const resolved=import.meta.resolve('turbo-dom/environment/vitest');
writeFileSync(`resolved-${mode}-${process.env.TURBO_DOM_PARSER??'auto'}.json`,JSON.stringify({mode,resolved,parserMode:process.env.TURBO_DOM_PARSER??'auto'}));
export default defineConfig({test:{include:process.env.CSS_ACCEPTANCE?['css-acceptance.test.mjs']:['shared/**/*.test.{js,jsx}'],environment:mode==='turbo-dom'?fileURLToPath(resolved):mode,setupFiles:['shared/setup.js','mode-audit.mjs'],maxWorkers:1,environmentOptions:mode==='turbo-dom'?{turboDom:{url:'http://localhost/'}}:{}}});

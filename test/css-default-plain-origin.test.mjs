import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const helper=fileURLToPath(new URL('./helpers/css-plain-origin-child.mjs',import.meta.url));
function run(scenario,mode='wasm'){
 const result=spawnSync(process.execPath,[helper,scenario],{env:{...process.env,TURBO_DOM_PARSER:mode},encoding:'utf8'});assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);
}
for(const scenario of ['global','module','instance'])test(`actual cold WASM ${scenario} callback never grants sticky private origin`,()=>{
 const result=run(scenario);assert.equal(result.parsingError,undefined);assert.equal(result.parsed,'new real parse');assert.ok(result.callbacks>0);assert.equal(result.restored,true);
 assert.equal(result.admitted,false);assert.match(result.cssError.message,/CSS buffer|input|origin/);assert.match(result.admissionError.message,/CSS buffer|input|origin/);assert.equal(result.subsequentOrigin,false);
 assert.equal(result.modulePaths.filter(path=>path.endsWith('.node')).length,0);
});
test('canonical WASM and native parser preserve admission with no constructor callbacks',()=>{
 for(const [scenario,mode]of [['clean','wasm'],['native','native']]){const result=run(scenario,mode);assert.equal(result.callbacks,0);assert.equal(result.parsingError,undefined);assert.equal(result.admitted,true);assert.equal(result.cssError,undefined);assert.equal(typeof result.cssResult,'string');assert.equal(result.subsequentOrigin,true);assert.equal(result.restored,true);}
});
test('previously admitted cached WASM binding retains origin while inactive constructors differ',()=>{
 const result=run('cached');assert.equal(result.callbacks,0);assert.equal(result.previousAdmitted,true);assert.equal(result.oldOriginStillValid,true);assert.equal(result.admitted,true);assert.equal(result.cssError,undefined);assert.equal(typeof result.cssResult,'string');assert.equal(result.subsequentOrigin,true);assert.equal(result.restored,true);
});
test('deleting real parser module cache makes a changed-constructor reload uncertified',()=>{
 const result=run('reload');assert.ok(result.callbacks>0);assert.equal(result.previousAdmitted,true);assert.equal(result.oldOriginStillValid,true);assert.equal(result.parsingError,undefined);assert.equal(result.parsed,'new real parse');assert.equal(result.admitted,false);assert.match(result.cssError.message,/CSS buffer|input|origin/);assert.equal(result.subsequentOrigin,false);assert.equal(result.restored,true);
 assert.ok(result.modulePaths.some(path=>path.endsWith('/pkg/turbo_dom_parser.js')));
});

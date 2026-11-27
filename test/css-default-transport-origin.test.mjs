import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const helper=fileURLToPath(new URL('./helpers/css-transport-origin-child.mjs',import.meta.url));
function run(kind,mode){const result=spawnSync(process.execPath,[helper,kind],{env:{...process.env,TURBO_DOM_PARSER:mode},encoding:'utf8'});assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);}
for(const mode of ['native','wasm'])for(const kind of ['subarray','constructor','species'])test(`actual ${mode} ${kind} callouts preserve public parsing but refuse affected private origin`,()=>{
 const result=run(kind,mode);assert.equal(result.creationError,undefined);assert.match(result.parsed,/λ-😀/);assert.ok(result.callbacks>0);assert.equal(result.safeWhileChanged,false);assert.equal(result.affectedAdmitted,false);assert.match(result.admissionError.message,/CSS buffer|input|origin/);assert.match(result.cssError.message,/CSS buffer|input|origin/);assert.equal(result.oldAdmitted,true);assert.equal(result.cleanAdmitted,true);assert.equal(result.cleanRawOrigin,true);assert.equal(result.restoredSafe,true);
 if(mode==='wasm')assert.equal(result.modulePaths.filter(path=>path.endsWith('.node')).length,0);
});
for(const kind of ['chain-proto','chain-ctor','shadow'])test(`decoder admission refuses ${kind} without invoking proxy or getter`,()=>{
 const result=run(kind,'wasm');assert.equal(result.creationError,undefined);assert.equal(result.safeWhileChanged,false);assert.equal(result.callbacks,0);assert.equal(result.oldAdmitted,true);assert.equal(result.cleanAdmitted,true);assert.equal(result.restoredSafe,true);
});

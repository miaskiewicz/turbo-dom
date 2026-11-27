import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const helper=fileURLToPath(new URL('./helpers/css-normalization-entry-child.mjs',import.meta.url));
function run(kind){const result=spawnSync(process.execPath,[helper,kind],{env:{...process.env,TURBO_DOM_PARSER:'native'},encoding:'utf8'});assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);}
for(const kind of ['entry-aba','normalized-aba','blob-subarray','packed-buffer','packed-offset'])test(`real native ${kind} cannot acquire private authority after restoration`,()=>{
 const result=run(kind);assert.equal(result.loadError,undefined);assert.ok(result.callbacks>0);assert.equal(result.admitted,false);assert.match(result.admissionError.message,/origin|input/);assert.equal(result.privateCalls,0);assert.equal(result.cleanFutureAdmitted,true);assert.equal(result.restored,true);
 if(kind==='entry-aba')assert.equal(result.arrayConstructions,5);if(kind==='normalized-aba')assert.equal(result.normalizedStillUnknown,true);
});
for(const kind of ['blob-prototype','view-proxy'])test(`unknown ${kind} private admission invokes zero traps and preserves public decode error`,()=>{
 const result=run(kind);assert.equal(result.privateEntrySafe,false);assert.equal(result.privateCalls,0);assert.equal(result.admitted,false);assert.equal(result.originalErrorPreserved,true);assert.ok(result.callbacks>0);assert.equal(result.cleanFutureAdmitted,true);assert.equal(result.restored,true);
});

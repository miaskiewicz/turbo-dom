import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
test('explicit native style-state environment preserves combined parser, used values and old snapshots',()=>{
 const child=fileURLToPath(new URL('./helpers/css-native-state-environment-child.mjs',import.meta.url));
 const results=[];
 for(const mode of ['wire-v1','native-state-v1']){
  const output=spawnSync(process.execPath,[child,mode],{env:{...process.env,TURBO_DOM_PARSER:'native'},encoding:'utf8'});
  assert.equal(output.status,0,output.stdout+output.stderr);results.push(JSON.parse(output.stdout));
 }
 assert.deepEqual(results[0].modules,results[1].modules);
 for(const key of ['color','width','oldColor'])assert.equal(results[0][key],results[1][key]);
});

test('explicit native state protocol refuses missing capabilities and unsuitable policy without wire fallback',()=>{
 const child=fileURLToPath(new URL('./helpers/css-native-state-environment-child.mjs',import.meta.url));
 for(const fault of ['missing','missing-store','missing-resolve','throw-method','object-policy','extra-quota','malformed-policy']){
  const output=spawnSync(process.execPath,[child,'native-state-v1',fault],{env:{...process.env,TURBO_DOM_PARSER:'native'},encoding:'utf8'});
  assert.equal(output.status,0,output.stdout+output.stderr);
  assert.equal(JSON.parse(output.stdout).fault,fault);
 }
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
test('native result memo preserves pre-load callback output and later exception',()=>{
 const child=fileURLToPath(new URL('./helpers/css-native-memo-callback-child.mjs',import.meta.url));
 const result=spawnSync(process.execPath,[child],{env:{...process.env,TURBO_DOM_PARSER:'native'},encoding:'utf8'});
 assert.equal(result.status,0,result.stdout+result.stderr);
 assert.deepEqual(JSON.parse(result.stdout),{first:'rgb(255, 0, 0)',second:'rgb(0, 0, 255)',calls:2});
});

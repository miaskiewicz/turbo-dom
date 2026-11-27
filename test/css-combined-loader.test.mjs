import {nativeCssFilename,nativeParserFilename} from './opt-in-helpers/native.mjs';
import {tmpdir} from 'node:os';
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {cpSync,mkdirSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
const root=process.env.CSS_LOADER_TEST_ROOT||fileURLToPath(new URL('../',import.meta.url));
const helper=fileURLToPath(new URL('./helpers/css-combined-loader-child.mjs',import.meta.url));
function run(scenario,mode='native',packageRoot=root){const child=spawnSync(process.execPath,[helper,packageRoot,scenario],{env:{...process.env,TURBO_DOM_PARSER:mode},encoding:'utf8'});assert.equal(child.status,0,child.stderr);return JSON.parse(child.stdout);}
function healthy(row){assert.equal(row.error,undefined);assert.equal(row.originError,undefined);assert.equal(row.after,'rgb(255, 0, 0)');}
test('explicit native Rust CSS uses one real combined addon with native origin',()=>{
 const row=run('native');healthy(row);assert.equal(row.nativeModules.length,1);assert.match(row.nativeModules[0],/native-css/);
 assert.deepEqual(row.facadeKeys,['parse','parseBuffer','parseFragment','parseRaw']);assert.deepEqual(row.beforeCss,row.nativeModules);
});
test('default JS and auto/WASM parser modes keep their existing loader behavior',()=>{
 const js=run('js');healthy(js);assert.equal(js.nativeModules.length,1);assert.doesNotMatch(js.nativeModules[0],/native-css/);
 const auto=run('auto','auto');healthy(auto);assert.equal(auto.beforeCss.length,1);assert.equal(auto.nativeModules.length,2);
 const wasm=run('wasm','wasm');healthy(wasm);assert.equal(wasm.nativeModules.length,0);
});
test('already selected native parser is never replaced to force single-addon loading',()=>{
 const row=run('selected');healthy(row);assert.equal(row.selectedPreserved,true);assert.equal(row.nativeModules.length,2);
});
test('explicit and global injected parsers never acquire builtin origin',()=>{
 for(const scenario of ['injected','global']){const row=run(scenario);assert.equal(row.error,undefined);assert.equal(row.selectedPreserved,true);assert.match(row.originError.message,/CSS buffer|input|origin/);}
});
test('mixed JS environment and reset preserve parser ownership and current CSS',()=>{
 const mixed=run('mixed');healthy(mixed);assert.equal(mixed.first,'sample|second');assert.equal(mixed.nativeModules.length,1);
 const reset=run('reset');assert.equal(reset.error,undefined);assert.equal(reset.originError,undefined);assert.equal(reset.first,'reset');assert.equal(reset.after,'rgb(0, 0, 255)');assert.equal(reset.nativeModules.length,1);
});
test('replacing an actual builtin parseBuffer still revokes certification',()=>{
 const row=run('changed');assert.equal(row.error,undefined);assert.match(row.originError.message,/CSS buffer|input|origin/);assert.equal(row.nativeModules.length,1);
});
function fixture(action){
 const path=mkdtempSync(join(tmpdir(),'turbo-dom-loader-'));
 try{for(const file of ['src','index.js','package.json',nativeParserFilename])cpSync(join(root,file),join(path,file),{recursive:true});mkdirSync(join(path,'native-css'));return action(path);}finally{rmSync(path,{recursive:true,force:true});}
}
test('known combined artifact absence preserves parsing and later explicit CSS refusal',()=>fixture(path=>{
 const row=run('missing','native',path);assert.equal(row.error,undefined);assert.equal(row.first,'sample');assert.match(row.originError.message,/capability unavailable: Rust CSS artifact/);assert.equal(row.nativeModules.length,1);assert.doesNotMatch(row.nativeModules[0],/native-css/);
}));
test('present malformed combined artifact propagates startup error without parser fallback',()=>fixture(path=>{
 writeFileSync(join(path,'native-css',nativeCssFilename),'not a native addon');
 const row=run('invalid','native',path);assert.equal(row.error.code,'ERR_DLOPEN_FAILED');assert.equal(row.first,undefined);assert.equal(row.nativeModules.length,0);
}));

test('combined builtin facade never invokes an overridden public array iterator',()=>{const row=run('iterator');healthy(row);assert.equal(row.iteratorCalls,0);assert.equal(row.nativeModules.length,1);});

test('malformed builtin parser exports refuse without invoking accessors or falling back',()=>{
 for(const scenario of ['export-missing','export-value','export-accessor']){
  const row=run(scenario);
  assert.equal(row.error.name,'TypeError');
  assert.match(row.error.message,/combined native parser missing parseRaw/);
  assert.equal(row.exportGetterCalls,0);
  assert.equal(row.first,undefined);
  assert.equal(row.nativeModules.length,1);
  assert.match(row.nativeModules[0],/native-css/);
 }
});

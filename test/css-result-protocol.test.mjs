import test from 'node:test';
import assert from 'node:assert/strict';
import {validateCssResultProtocol} from '../src/runtime/css-result-protocol.mjs';
import {createEnvironment} from '../src/runtime/index.mjs';
import {createWindow} from '../src/runtime/window.mjs';
import {installGlobals} from '../src/environment/install.mjs';
test('CSS result protocol validates independently and never selects another backend',()=>{
 assert.equal(validateCssResultProtocol('js'),false);
 assert.equal(validateCssResultProtocol('rust','wire-v1'),false);
 assert.equal(validateCssResultProtocol('rust','native-state-v1'),true);
 for(const value of [null,{},'native-state','rust',0])assert.throws(()=>validateCssResultProtocol('rust',value),/cssResultProtocol/);
 assert.throws(()=>validateCssResultProtocol('js','native-state-v1'),/requires cssBackend/);
 for(const make of [options=>createEnvironment('<p>x</p>',options),options=>createWindow({},options)]){
  assert.throws(()=>make({cssBackend:'rust-style-state-experiment'}),/cssBackend/);
  assert.throws(()=>make({cssResultProtocol:'native-state-v1'}),/requires cssBackend/);
  assert.throws(()=>make({cssResultProtocol:'bad'}),/cssResultProtocol/);
 }
});
test('installed native-state protocol survives reset and preserves old declaration snapshot',()=>{
 const installed=installGlobals({}, {html:'<style>p{color:red}</style><p>x</p>',cssBackend:'rust',cssResultProtocol:'native-state-v1'});
 try{
  const old=installed.env.window.getComputedStyle(installed.env.document.querySelector('p'));
  assert.equal(old.color,'rgb(255, 0, 0)');
  installed.env.reset('<style>p{color:blue}</style><p>y</p>');
  assert.equal(installed.env.window.getComputedStyle(installed.env.document.querySelector('p')).color,'rgb(0, 0, 255)');
  assert.equal(old.color,'rgb(255, 0, 0)');
 }finally{installed.teardown()}
});

import test from 'node:test';
import assert from 'node:assert/strict';
import {validateCssRequestProtocol} from '../src/runtime/css-request-protocol.mjs';
import {createEnvironment} from '../src/runtime/index.mjs';
import {createWindow} from '../src/runtime/window.mjs';
import {installGlobals} from '../src/environment/install.mjs';
test('request transport validates before parsing and keeps one Rust backend',()=>{
 assert.equal(validateCssRequestProtocol('js'),false);
 assert.equal(validateCssRequestProtocol('rust','wire-v1','native-state-v1'),false);
 assert.equal(validateCssRequestProtocol('rust','context-template-v1'),true);
 for(const value of [null,{},'rust-request-state',0])assert.throws(()=>validateCssRequestProtocol('rust',value),/cssRequestProtocol/);
 let coerced=0;const html={toString(){coerced++;throw new Error('must not parse')}};
 for(const make of [options=>createEnvironment(html,options),options=>createWindow({},options)]){
  assert.throws(()=>make({cssRequestProtocol:'context-template-v1'}),/requires cssBackend/);
  assert.throws(()=>make({cssBackend:'rust',cssRequestProtocol:'context-template-v1',cssResultProtocol:'native-state-v1'}),/combination is not supported/);
  assert.throws(()=>make({cssBackend:'rust',cssRequestProtocol:'bad'}),/cssRequestProtocol/);
 }
 assert.equal(coerced,0);
});
test('installed request protocol survives reset without changing old declaration',()=>{
 const installed=installGlobals({}, {html:'<style>p{color:red}</style><p>x</p>',cssBackend:'rust',cssRequestProtocol:'context-template-v1'});
 try{
  const old=installed.env.window.getComputedStyle(installed.env.document.querySelector('p'));
  assert.equal(old.color,'rgb(255, 0, 0)');
  installed.env.reset('<style>p{color:blue}</style><p>y</p>');
  assert.equal(installed.env.window.getComputedStyle(installed.env.document.querySelector('p')).color,'rgb(0, 0, 255)');
  assert.equal(old.color,'rgb(255, 0, 0)');
 }finally{installed.teardown()}
});

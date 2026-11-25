import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

test('CSS store behavior survives missing WeakRef while trusted caching is explicitly unavailable', () => {
  const module = new URL('../src/runtime/cssom.mjs', import.meta.url).href;
  const source = `globalThis.WeakRef=undefined;const {CSSStyleRule,CSSStyleSheet,cssSheetSnapshot,createObservedCssSheet}=await import(${JSON.stringify(module)});let touches=0;const owner={__touch(){touches++}};const sheet=createObservedCssSheet(owner);sheet.insertRule('a{}');if(touches!==1)throw new Error('touch missing');if(cssSheetSnapshot(sheet).rulesTrusted!==false)throw new Error('unsafe capability');sheet.cssRules[0].cssText='b{}';if(sheet.cssRules[0].selectorText!=='b')throw new Error('lost store value');sheet.replaceSync('c{}');if(sheet.cssRules.length!==1||touches!==2)throw new Error('replace failed');`;
  const result = spawnSync(process.execPath, ['--input-type=module', '-e', source], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
});

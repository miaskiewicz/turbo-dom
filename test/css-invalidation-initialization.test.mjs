import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  cssFocusTarget,
  ensureCssState,
  cssStateSnapshot,
  noteCssMutation,
  noteCssStateChange,
} from '../src/runtime/css-invalidation.mjs';

test('no DOM owner initialization means no connected-document cache proof', () => {
  const object = { __version: 1 };
  assert.equal(cssFocusTarget(object), undefined);
  assert.equal(ensureCssState(object), null);
  assert.equal(cssStateSnapshot(object), null);
  noteCssMutation(object, object, 0);
  noteCssStateChange(object);
  assert.equal(cssStateSnapshot(object), null);
});

test('preempting internal arm registration makes DOM initialization fail explicitly', () => {
  const stateModule = new URL('../src/runtime/css-invalidation.mjs', import.meta.url).href;
  const domModule = new URL('../src/runtime/dom.mjs', import.meta.url).href;
  const source = `import { installCssTrackingArm } from ${JSON.stringify(stateModule)};installCssTrackingArm(()=>false);try{await import(${JSON.stringify(domModule)});throw new Error('unsafe initialization');}catch(error){if(!String(error).includes('already initialized'))throw error;}`;
  const result = spawnSync(process.execPath, ['--input-type=module', '-e', source], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
});

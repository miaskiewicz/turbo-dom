import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { loadRealCssBackend } from './helpers/css-shared-real.mjs';
for (const mode of ['native', 'wasm'])
  test('actual resolver forwards operations into production adapter: ' + mode, async () => {
    const real = await loadRealCssBackend(mode === 'native' ? 'native' : 'node-wasm');
    const result = spawnSync(
      process.execPath,
      [new URL('./fixtures/css-optional-ops/actual.mjs', import.meta.url).pathname, mode, real.artifact],
      { encoding: 'utf8' },
    );
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });

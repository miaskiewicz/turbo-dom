// Every package.json `exports` target must ship in the packed tarball, and the
// runtime entry's declaration file must declare every runtime export (turbo-dom#200).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));

function targets(node, out = []) {
  if (typeof node === 'string') out.push(node.replace(/^\.\//, ''));
  else for (const v of Object.values(node)) targets(v, out);
  return out;
}

test('every exports target is in the npm pack file list (or is a build output)', () => {
  const [info] = JSON.parse(execFileSync('npm', ['pack', '--dry-run', '--json', '--ignore-scripts'], { cwd: root, encoding: 'utf8' }));
  const packed = new Set(info.files.map((f) => f.path));
  // pkg-web is a wasm-pack build output (absent in a source checkout); check the rest.
  const missing = targets(pkg.exports).filter((t) => !t.startsWith('pkg-web/') && !packed.has(t));
  assert.deepEqual(missing, []);
});

test('runtime index.d.ts declares every runtime export', async () => {
  const dts = readFileSync(new URL('../src/runtime/index.d.ts', import.meta.url), 'utf8');
  const mod = await import('../src/runtime/index.mjs');
  const undeclared = Object.keys(mod).filter((k) => !new RegExp(`export (declare )?(const|function|class|abstract class) ${k}\\b`).test(dts));
  assert.deepEqual(undeclared, []);
});

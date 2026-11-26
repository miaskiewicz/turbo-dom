import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import { join, relative, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const [oldArg, newArg, outputArg, count = '5'] = process.argv.slice(2);
assert.ok(oldArg && newArg && outputArg, 'Usage: node benchmark-paired.mjs OLD_CONSUMER NEW_CONSUMER OUTPUT [ROUNDS]');
const rounds = Number(count);
assert.ok(Number.isInteger(rounds) && rounds >= 3 && rounds <= 9);
const consumers = { old: resolve(oldArg), new: resolve(newArg) };
const output = resolve(outputArg);
mkdirSync(output, { recursive: true });

const exclude = 'getByLabelText: wrapping <label> labels only its FIRST control';
const namePattern = `^(?!${exclude.replace(/[.*+?${}()|[\]\\]/g, '\\$&')}$).*$`;
const modes = ['native', 'wasm'];
const dependencies = ['vitest', 'react', 'react-dom', '@testing-library/react', '@testing-library/user-event'];
const hash = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');

function manifest(root, directory = root, rows = {}) {
  for (const item of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, item.name);
    if (item.isDirectory()) manifest(root, path, rows);
    else if (item.isFile()) rows[relative(root, path)] = hash(path);
    else assert.fail(`Unexpected package entry: ${path}`);
  }
  return rows;
}

function fixture(consumer) {
  return {
    shared: manifest(join(consumer, 'shared')),
    config: hash(join(consumer, 'vitest.config.mjs')),
    audit: hash(join(consumer, 'mode-audit.mjs')),
    sentinel: hash(join(consumer, 'css-acceptance.test.mjs')),
    dependencies: Object.fromEntries(dependencies.map((name) => [
      name, realpathSync(join(consumer, 'node_modules', name)),
    ])),
  };
}

const originalFixture = fixture(consumers.old);
assert.deepEqual(fixture(consumers.new), originalFixture, 'Both consumers must use identical tests, config and dependencies');
const artifacts = Object.fromEntries(Object.entries(consumers).map(([label, path]) => [
  label, manifest(join(path, 'node_modules', 'turbo-dom')),
]));
assert.notDeepEqual(artifacts.old, artifacts.new, 'Old and new packages must differ');

function run(build, mode, phase, round) {
  const stem = `${phase}-${round}-${mode}-${build}`;
  const report = join(output, `${stem}.json`);
  const env = { ...process.env, CONSUMER_ENV: 'turbo-dom', TURBO_DOM_PARSER: mode };
  delete env.CSS_ACCEPTANCE;
  const args = ['node_modules/vitest/vitest.mjs', 'run', '--config', 'vitest.config.mjs', '--maxWorkers=1'];
  if (phase === 'sentinel') env.CSS_ACCEPTANCE = '1';
  else args.push('--testNamePattern', namePattern);
  args.push('--reporter=json', '--outputFile', report);
  const started = performance.now();
  const child = spawnSync(process.execPath, args, { cwd: consumers[build], env, encoding: 'utf8' });
  const wallMs = performance.now() - started;
  writeFileSync(join(output, `${stem}.log`), (child.stdout ?? '') + (child.stderr ?? ''));
  if (child.error) throw child.error;
  assert.equal(child.status, 0, `${stem} failed`);
  const results = JSON.parse(readFileSync(report, 'utf8')).testResults.flatMap((file) => file.assertionResults);
  const passing = results.filter((item) => item.status === 'passed');
  const excluded = results.filter((item) => item.status !== 'passed');
  assert.equal(passing.length, phase === 'sentinel' ? 1 : 28);
  assert.deepEqual(excluded.map((item) => item.fullName), phase === 'sentinel' ? [] : [exclude]);
  const testMs = passing.reduce((sum, item) => sum + (item.duration ?? 0), 0);
  return { build, mode, phase, round, wallMs, testMs, reportSha256: hash(report), load: os.loadavg() };
}

function median(values) {
  const sorted = values.toSorted((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

const loadBefore = os.loadavg();
const preflight = modes.map((mode) => run('new', mode, 'sentinel', 0));
const warmup = modes.flatMap((mode) => [run('old', mode, 'warmup', 0), run('new', mode, 'warmup', 0)]);
const timed = [];
for (let round = 0; round < rounds; round++) {
  const order = round % 2 ? ['new', 'old'] : ['old', 'new'];
  const sequence = round % 2 ? modes.toReversed() : modes;
  for (const mode of sequence) for (const build of order) timed.push(run(build, mode, 'timed', round));
}
for (const [build, path] of Object.entries(consumers)) {
  assert.deepEqual(fixture(path), originalFixture);
  assert.deepEqual(manifest(join(path, 'node_modules', 'turbo-dom')), artifacts[build]);
}
const summary = modes.map((mode) => {
  const times = timed.filter((item) => item.mode === mode);
  const old = times.filter((item) => item.build === 'old');
  const next = times.filter((item) => item.build === 'new');
  const pairRatios = old.map((item, index) => next[index].wallMs / item.wallMs);
  return {
    mode, oldWallMs: median(old.map((item) => item.wallMs)), newWallMs: median(next.map((item) => item.wallMs)),
    oldTestMs: median(old.map((item) => item.testMs)), newTestMs: median(next.map((item) => item.testMs)),
    medianPairRatio: median(pairRatios), pairRatios,
  };
});
const result = {
  date: new Date().toISOString(), node: process.version, platform: process.platform,
  arch: process.arch, cpu: os.cpus()[0]?.model, rounds, exclude, loadBefore, loadAfter: os.loadavg(),
  fixture: originalFixture, artifacts, preflight, warmup, timed, summary,
};
writeFileSync(join(output, 'RESULT.json'), JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify(summary, null, 2));

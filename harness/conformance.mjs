// html5lib-tests tree-construction conformance gate.
// Runs every fixture through the native parser, serializes to html5lib dump
// format, and compares against the expected #document. Reports pass rate
// overall and per file. The "we inherit Servo's correctness" number.
//
//   node harness/conformance.mjs            # summary
//   node harness/conformance.mjs --verbose  # show first failures with diffs
//   node harness/conformance.mjs --file tests1.dat

import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { parseDatFile } from './dat.mjs';
import { serializeTree } from './serialize.mjs';
import { parse, parseFragment } from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
export const fixturesDir = join(here, '..', 'vendor', 'html5lib-tests');

// Default adapter: our native parser + html5lib serializer.
export function fastDomAdapter(t) {
  const tree = t.fragmentContext != null ? parseFragment(t.data, t.fragmentContext) : parse(t.data);
  return serializeTree(tree);
}

function runOne(t, adapter) {
  // Scripting-enabled fixtures expect behavior we don't model (scripting flag off).
  if (t.scriptMode === 'on') return { status: 'skip', reason: 'script-on' };
  if (t.document == null) return { status: 'skip', reason: 'no-document' };

  let actual;
  try {
    actual = adapter(t);
  } catch (e) {
    return { status: 'error', reason: String(e && e.message || e) };
  }
  return actual === t.document
    ? { status: 'pass' }
    : { status: 'fail', expected: t.document, actual };
}

// Run the gate. Returns aggregate stats + per-file breakdown + sample failures.
export function runConformance({ onlyFile = null, maxShow = 8, adapter = fastDomAdapter } = {}) {
  const files = (onlyFile ? [onlyFile] : readdirSync(fixturesDir).filter((f) => f.endsWith('.dat'))).sort();
  let totalPass = 0, totalFail = 0, totalSkip = 0, totalErr = 0;
  const perFile = [];
  const failures = [];

  for (const file of files) {
    const text = readFileSync(join(fixturesDir, file), 'utf8');
    const tests = parseDatFile(text);
    let pass = 0, fail = 0, skip = 0, err = 0;
    for (const t of tests) {
      const r = runOne(t, adapter);
      if (r.status === 'pass') pass++;
      else if (r.status === 'skip') skip++;
      else if (r.status === 'error') { err++; if (failures.length < maxShow) failures.push({ file, t, r }); }
      else { fail++; if (failures.length < maxShow) failures.push({ file, t, r }); }
    }
    totalPass += pass; totalFail += fail; totalSkip += skip; totalErr += err;
    perFile.push({ file, pass, fail, skip, err, total: tests.length });
  }
  const evaluated = totalPass + totalFail + totalErr;
  const rate = evaluated ? (totalPass / evaluated) * 100 : 0;
  return { totalPass, totalFail, totalSkip, totalErr, evaluated, rate, perFile, failures };
}

// ---- CLI ----
if (import.meta.url === `file://${process.argv[1]}`) cli();

function cli() {
const args = process.argv.slice(2);
const verbose = args.includes('--verbose');
const onlyFile = args.includes('--file') ? args[args.indexOf('--file') + 1] : null;

const { totalPass, totalFail, totalSkip, totalErr, evaluated, perFile, failures } =
  runConformance({ onlyFile });
const rate = evaluated ? ((totalPass / evaluated) * 100).toFixed(2) : '0.00';

// per-file table, worst first
perFile.sort((a, b) => (a.fail + a.err) - (b.fail + b.err) || a.file.localeCompare(b.file));
console.log('\nhtml5lib-tests tree-construction — fast-dom parser\n');
console.log('file'.padEnd(38), 'pass'.padStart(5), 'fail'.padStart(5), 'err'.padStart(4), 'skip'.padStart(5));
console.log('-'.repeat(64));
for (const r of perFile) {
  if (r.fail || r.err) {
    console.log(r.file.padEnd(38), String(r.pass).padStart(5), String(r.fail).padStart(5), String(r.err).padStart(4), String(r.skip).padStart(5));
  }
}
const clean = perFile.filter((r) => !r.fail && !r.err);
console.log(`\n${clean.length}/${perFile.length} files fully passing (clean files hidden above).`);
console.log('-'.repeat(64));
console.log(`PASS ${totalPass}  FAIL ${totalFail}  ERROR ${totalErr}  SKIP ${totalSkip}`);
console.log(`Conformance: ${rate}%  (${totalPass}/${evaluated} evaluated; ${totalSkip} skipped)`);

if (verbose && failures.length) {
  console.log('\n--- first failures ---');
  for (const { file, t, r } of failures) {
    console.log(`\n[${file}] input: ${JSON.stringify(t.data)}${t.fragmentContext ? `  (fragment: ${t.fragmentContext})` : ''}`);
    if (r.status === 'error') { console.log('  ERROR:', r.reason); continue; }
    console.log('  expected:\n' + r.expected.split('\n').map((l) => '    ' + l).join('\n'));
    console.log('  actual:\n' + r.actual.split('\n').map((l) => '    ' + l).join('\n'));
  }
}

// non-zero exit if anything regressed below the committed bar
const BAR = 98.0;
process.exit(parseFloat(rate) >= BAR ? 0 : 1);
}

// Parse-throughput benchmark: fast-dom vs parse5 vs happy-dom vs jsdom.
// Plus the parse-only / parse+marshal isolation that decides whether the SoA
// buffer (Phase 2.5) is worth building.
//
//   node bench/parse.mjs

import { createRequire } from 'node:module';
import { parse, parseRaw } from '../index.js';
import { fixtures } from './fixtures.mjs';

const require = createRequire(import.meta.url);
const parse5 = require('parse5');
const { Window } = require('happy-dom');
const { JSDOM } = require('jsdom');

// reuse runtime instances so we measure parsing, not window construction
const hdWindow = new Window();
const HappyDOMParser = hdWindow.DOMParser;
const jsWindow = new JSDOM('').window;
const JsDOMParser = jsWindow.DOMParser;

const engines = [
  { name: 'fast-dom parse()', fn: (h) => parse(h) },
  { name: 'fast-dom parseRaw()', fn: (h) => parseRaw(h) },
  { name: 'parse5', fn: (h) => parse5.parse(h) },
  { name: 'happy-dom', fn: (h) => new HappyDOMParser().parseFromString(h, 'text/html') },
  { name: 'jsdom', fn: (h) => new JsDOMParser().parseFromString(h, 'text/html') },
];

function bench(fn, html, ms = 800) {
  // warmup
  for (let i = 0; i < 20; i++) fn(html);
  let n = 0;
  const start = process.hrtime.bigint();
  const deadline = start + BigInt(ms) * 1_000_000n;
  while (process.hrtime.bigint() < deadline) {
    fn(html);
    n++;
  }
  const elapsedMs = Number(process.hrtime.bigint() - start) / 1e6;
  return { opsPerSec: (n / elapsedMs) * 1000, msPerOp: elapsedMs / n, n };
}

console.log('\nParse throughput — ops/sec (higher better), median of run\n');

for (const fx of fixtures) {
  console.log(`### ${fx.name}  (${fx.html.length.toLocaleString()} bytes)`);
  const results = engines.map((e) => {
    let r;
    try {
      r = bench(e.fn, fx.html);
    } catch (err) {
      return { name: e.name, err: err.message };
    }
    return { name: e.name, ...r };
  });

  const baseline = results.find((r) => r.name === 'fast-dom parse()');
  console.log('engine'.padEnd(22), 'ops/sec'.padStart(12), 'ms/op'.padStart(10), 'rel'.padStart(8));
  console.log('-'.repeat(54));
  for (const r of results) {
    if (r.err) {
      console.log(r.name.padEnd(22), 'ERROR: ' + r.err);
      continue;
    }
    const rel = baseline && !baseline.err ? (r.opsPerSec / baseline.opsPerSec).toFixed(2) + 'x' : '-';
    console.log(
      r.name.padEnd(22),
      Math.round(r.opsPerSec).toLocaleString().padStart(12),
      r.msPerOp.toFixed(4).padStart(10),
      rel.padStart(8)
    );
  }
  console.log();
}

console.log('rel = speed relative to fast-dom parse() (>1 faster than us, <1 slower).');
console.log('parseRaw() = html5ever parse only, no JS tree built → marshaling cost = parse() vs parseRaw() gap.');

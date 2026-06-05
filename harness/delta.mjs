// Conformance delta: run the SAME html5lib-tests suite against fast-dom,
// happy-dom, and jsdom; report pass rates and the head-to-head divergence.
//
//   node harness/delta.mjs

import { runConformance, fastDomAdapter } from './conformance.mjs';
import { makeHappyDomAdapter, makeJsdomAdapter } from './adapters.mjs';

function safe(name, make) {
  try {
    return make();
  } catch (e) {
    console.error(`! ${name} unavailable: ${e.message}`);
    return null;
  }
}

const engines = [
  { name: 'fast-dom', adapter: fastDomAdapter },
  { name: 'happy-dom', adapter: safe('happy-dom', makeHappyDomAdapter) },
  { name: 'jsdom', adapter: safe('jsdom', makeJsdomAdapter) },
].filter((e) => e.adapter);

const results = engines.map((e) => {
  const s = runConformance({ adapter: e.adapter, maxShow: 0 });
  return { name: e.name, ...s };
});

console.log('\nhtml5lib-tests tree-construction — engine conformance\n');
console.log('engine'.padEnd(12), 'pass'.padStart(6), 'fail'.padStart(6), 'error'.padStart(6), 'skip'.padStart(5), 'rate'.padStart(8));
console.log('-'.repeat(52));
for (const r of results) {
  console.log(
    r.name.padEnd(12),
    String(r.totalPass).padStart(6),
    String(r.totalFail).padStart(6),
    String(r.totalErr).padStart(6),
    String(r.totalSkip).padStart(5),
    (r.rate.toFixed(2) + '%').padStart(8)
  );
}

const ours = results.find((r) => r.name === 'fast-dom');
console.log('\nDelta vs fast-dom (' + ours.rate.toFixed(2) + '%):');
for (const r of results) {
  if (r.name === 'fast-dom') continue;
  const d = (ours.rate - r.rate).toFixed(2);
  const sign = d >= 0 ? '+' : '';
  console.log(`  fast-dom is ${sign}${d} pts vs ${r.name}  (${ours.totalPass} vs ${r.totalPass} passing, ${r.totalErr} ${r.name} crashes)`);
}

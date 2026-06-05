import { test } from 'node:test';
import assert from 'node:assert/strict';
import { runConformance } from '../harness/conformance.mjs';

// The html5lib-tests gate as a regression test. Locks the inherited-from-Servo
// conformance number so a parser/marshaling change can't silently regress it.
const stats = runConformance();

test('zero parser crashes across the whole suite', () => {
  assert.equal(stats.totalErr, 0, 'native parser must never throw on any fixture');
});

test('conformance rate >= 99.5%', () => {
  assert.ok(
    stats.rate >= 99.5,
    `conformance ${stats.rate.toFixed(2)}% < 99.5% (pass ${stats.totalPass}/${stats.evaluated})`
  );
});

test('all remaining failures are the upstream <select> insertion-mode divergence', () => {
  // Every known miss is html5ever 0.27 lagging the WHATWG <select> spec update.
  // If a failure appears whose input does NOT involve <select>, it's likely OUR
  // marshaling/serializer bug and must be investigated — fail loudly.
  const nonSelect = stats.failures.filter(
    (f) => !/<select/i.test(f.t.data) && f.t.fragmentContext !== 'select'
  );
  assert.equal(
    nonSelect.length,
    0,
    'non-<select> failure surfaced (possible marshaling bug):\n' +
      nonSelect.map((f) => `  [${f.file}] ${JSON.stringify(f.t.data)}`).join('\n')
  );
});

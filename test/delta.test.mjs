import { test } from 'node:test';
import assert from 'node:assert/strict';
import { runConformance, fastDomAdapter } from '../harness/conformance.mjs';
import { makeJsdomAdapter, makeHappyDomAdapter } from '../harness/adapters.mjs';

test('generic DOM serializer renders a simple jsdom document', () => {
  const adapter = makeJsdomAdapter();
  const out = adapter({ data: '<!doctype html><div id="a">hi</div>', fragmentContext: null, document: '' });
  assert.ok(out.startsWith('| <!DOCTYPE html>'));
  assert.ok(out.includes('|     <div>'));
  assert.ok(out.includes('|       id="a"'));
  assert.ok(out.includes('|       "hi"'));
});

test('conformance ordering holds: fast-dom >= jsdom >> happy-dom', () => {
  const fast = runConformance({ adapter: fastDomAdapter, maxShow: 0 });
  const jsd = runConformance({ adapter: makeJsdomAdapter(), maxShow: 0 });
  const hd = runConformance({ adapter: makeHappyDomAdapter(), maxShow: 0 });

  // fast-dom (html5ever) must be at least as conformant as jsdom
  assert.ok(
    fast.rate >= jsd.rate,
    `fast-dom ${fast.rate.toFixed(2)}% < jsdom ${jsd.rate.toFixed(2)}%`
  );
  // both spec parsers must crush happy-dom's hand-rolled one by a wide margin
  assert.ok(
    jsd.rate - hd.rate > 30,
    `expected jsdom to beat happy-dom by >30pts, got ${(jsd.rate - hd.rate).toFixed(2)}`
  );
  assert.ok(fast.rate >= 98.0, `fast-dom regressed below 98%: ${fast.rate.toFixed(2)}%`);
});

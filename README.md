# turbo-dom

A faster, more spec-correct DOM for test runners — a drop-in-style alternative to
**jsdom** and **happy-dom** for **vitest** and **jest**.

The HTML parser is native ([html5ever](https://github.com/servo/html5ever), Servo's
WHATWG tree constructor, via Rust/N-API with a WASM fallback). The DOM itself stays in
JavaScript but is **lazy** — nodes inflate from a compact typed-array buffer only when a
test touches them, and `window` globals materialize only on first use.

```bash
npm install -D @miaskiewicz/turbo-dom
```

- ✅ **More compatible than happy-dom** — 99.72% on html5lib-tests vs happy-dom's 37%.
  Runs React Testing Library, `user-event`, downshift, Radix UI, and Headless UI unmodified.
- ⚡ **Fast where suites spend time** — ~23× jsdom / ~10× happy-dom on per-file setup, 18–37× faster HTML parsing, ~7× jsdom on queries. (happy-dom edges raw query throughput by trading correctness; turbo-dom won't.)
- 🎯 **Honest, not lying** — no fake layout numbers; `getBoundingClientRect()` is zeros and
  `getComputedStyle` reflects only what you set. Geometry tests belong in a real browser.

## Quick start

### vitest

```ts
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import { createRequire } from 'node:module';

const envPath = createRequire(import.meta.url).resolve('@miaskiewicz/turbo-dom/environment/vitest');

export default defineConfig({
  test: {
    environment: envPath, // vitest resolves a bare name only for `vitest-environment-*`
                          // packages, so a scoped package is referenced by file path
  },
});
```

Works on vitest 1–4.

### jest

```js
// jest.config.js
module.exports = {
  testEnvironment: '@miaskiewicz/turbo-dom/jest',
};
```

Now `document`, `window`, and friends are global in your tests — write them exactly like
you would against jsdom/happy-dom:

```js
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

test('counter increments', async () => {
  render(<Counter />);
  await userEvent.click(screen.getByRole('button'));
  expect(screen.getByText('count: 1')).toBeInTheDocument();
});
```

### Without a test runner

```js
import { createEnvironment } from '@miaskiewicz/turbo-dom/runtime';

const env = createEnvironment('<!doctype html><body><div id="app"></div></body>');
env.document.querySelector('#app');     // nodes inflate lazily from the parse buffer
env.window.localStorage;                // globals materialize on first touch
env.reset();                            // fast per-file reset (reuses the parse buffer)
```

### Just the parser

```js
const { parse, parseBuffer, parseFragment } = require('@miaskiewicz/turbo-dom');

parse('<div id=a><span>hi</span></div>');         // nested tree
parseBuffer('<div id=a>...</div>');                // compact SoA typed-array buffer
parseFragment('<rect/>', 'svg path');              // fragment in a context element
```

## Compatibility

| | turbo-dom | happy-dom | jsdom |
|---|---|---|---|
| html5lib-tests conformance | **99.72%** | 37.35% | 97.03% |
| @testing-library/dom + user-event | ✅ | ✅ | ✅ |
| React + Radix / Headless UI / downshift | ✅ | partial | ✅ |
| Real layout / `getComputedStyle` cascade | ❌ (honest stub) | partial | partial |

turbo-dom inherits Servo's tree constructor, so the "messy input" cases hand-rolled parsers
get wrong — adoption-agency reparenting (`<a><p></a></p>`), table foster-parenting, optional
end tags, `<template>` content, SVG/MathML — all match the spec. The 5 remaining
conformance misses are bleeding-edge `<select>`-family proposals upstream `html5ever` hasn't
adopted yet.

## Performance

Measured on darwin-arm64, Node 24 (`npm run bench:all`). Higher = faster, except
the suite row (ms/file, lower = faster):

| benchmark | turbo-dom | happy-dom | jsdom |
|---|---:|---:|---:|
| **per-file setup + 1 query** (ops/s) | **5,950** | 611 | 260 |
| **realistic suite**, 200 files (ms/file) | **0.13** | 1.50 | 3.38 |
| **parse 56 KB SSR** (ops/s) | **478** | 43 | 26 |
| **parse 20 KB real page** (ops/s) | **4,203** | 190 | 114 |
| html5lib conformance | **99.72%** | 37.35% | 97.03% |
| raw query throughput (iters/s) | 24k | **635k** | 3k |

**The honest picture:** turbo-dom wins where test suites actually spend time —
**per-file construction (~10× happy-dom, ~23× jsdom)** and **parsing**, while being
far more spec-correct than happy-dom. On a realistic per-file workload (construct +
queries + events, 200 files) it's **~10× happy-dom and ~23× jsdom**, because per-file
setup dominates and turbo-dom builds lazily.

happy-dom is **faster on raw query throughput** (its selector engine is heavily
tuned, trading correctness for speed) — if your test does thousands of
`querySelector` calls against one already-built document, happy-dom wins that
micro-benchmark. But it fails real component libraries (37% conformance), which is
the trade turbo-dom refuses. turbo-dom is ~7× jsdom on queries and allocation-free
on the hot paths (no per-element `classList`/`split`/regex; version-cached
`getElementsBy*`), so RTL queries like `getByLabelText` stay cheap.

## How it works

```
test code (RTL, user-event)
   └─ lazy window (Proxy, self-replacing globals)        ← JS
   └─ lazy copy-on-write node tree (memoized identity)   ← JS
   └─ immutable Structure-of-Arrays parse buffer          ← shared
        └─ Rust: html5ever → flat typed-array buffer       ← native (N-API / WASM)
```

The parser runs in Rust (compute-bound, one boundary crossing per parse). The DOM stays in
JS (chatty, fine-grained) but pays only for what a test touches. Full design notes:
[turbo-dom-spec.md](./turbo-dom-spec.md).

## Limitations (by design)

- **No layout.** `getBoundingClientRect()` returns zeros; `getClientRects()` is empty.
- **`getComputedStyle` is inline-only** — it reflects the `style` attribute and explicitly
  set properties, never an invented cascade. Style/geometry assertions belong in a real
  browser (Playwright/WebDriver).
- Canvas, `<select>` rendering, and similar visual APIs are honest no-op stubs.

## Development

Requires Node ≥ 18 and a Rust toolchain (`rustup`, stable).

```bash
npm install
npm run build          # native addon (.node)
npm test               # JS suite (unit, conformance, differential, gauntlets)
npm run test:rust      # Rust core tests
npm run conformance    # html5lib-tests report
npm run bench:all      # benchmarks
npm run build:wasm     # wasm32 fallback
```

Contributions welcome — issues and PRs at
[github.com/miaskiewicz/turbo-dom](https://github.com/miaskiewicz/turbo-dom).

## License

[MIT](./LICENSE).

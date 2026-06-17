# MessageChannel: make it real + virtualizable (React 19 scheduler)

> **RESOLVED in v0.2.4 (C2/C3).** `window.mjs` ships a real `MessageChannel`/
> `MessagePort` polyfill (no host passthrough) whose delivery routes through the
> LIVE `globalThis.setTimeout` — so React's scheduler hops land in a render
> tier's owned/virtual timer queue and a `MessageChannel` exists in the bare V8
> isolate. Pairs with the clock hook: `setClock(fn)` (exported from
> `@miaskiewicz/turbo-dom/runtime`) drives `performance.now()` + the rAF
> timestamp; `requestAnimationFrame` schedules via live `globalThis.setTimeout`
> at a 16ms frame delay. C1 (virtual-clock drain) is turbo-crawl's. Tests in
> `test/coverage-fill.test.mjs`.

Date: 2026-06-17
Hand-off from turbo-crawl. Pairs with the **clock hook** work (C2) and
turbo-crawl's **virtual-clock drain** (C1). Full context:
`turbo-crawl/docs/render-geometry-loop.md`.

## Why this matters

**React 19's scheduler posts its work loop through a `MessageChannel`** (a
`MessagePort.postMessage` → `port.onmessage` hop), falling back to `setTimeout`
only if MessageChannel is absent. So to render/hydrate React headless under a
**virtual clock** (so time-gated MUI transitions complete instead of looping),
the scheduler's MessageChannel hops must land in the **same owned timer queue**
the virtual clock drains. Today they don't.

## Current state in turbo-dom (verified)

`src/runtime/window.mjs:389`:

```js
MessageChannel: globalThis.MessageChannel, MessagePort: globalThis.MessagePort,
```

It's a **host passthrough**, captured at module load:

- **node:vm host (fast):** `globalThis.MessageChannel` = Node's real one → it
  works, but delivers on the **real event loop**, not through `globalThis.
  setTimeout`. A consumer that owns `setTimeout` (turbo-crawl's virtual pump)
  **cannot** intercept it → React's work loop runs in real time, outside the
  virtual clock → time-gated animations never resolve / infinite ones storm.
- **bare V8 isolate (secure):** host has no `MessageChannel` → passthrough is
  **`undefined`** → MessageChannel is **missing**; `isolate-polyfills.mjs`
  doesn't add one. React falls back to `setTimeout` there (which the isolate
  queues), but that's incidental and other isolate gaps remain.

## The fix: a built-in MessagePort polyfill that posts via `globalThis.setTimeout`

Replace the host passthrough with a real, self-contained `MessageChannel` /
`MessagePort` whose delivery is scheduled through **`globalThis.setTimeout(fn,
0)`** (read live, not captured). Then:

- it exists in the **bare isolate** (no host dependency), and
- whoever owns `setTimeout` (turbo-crawl's virtual-clock queue) **catches every
  port delivery** → React's scheduler runs in **virtual time** and is bounded by
  the drain/deadline.

Sketch:

```js
class MessagePort extends EventTarget {           // or your EventTarget base
  constructor() { super(); this.onmessage = null; this._other = null; this._started = false; }
  start() { this._started = true; }
  close() { this._other = null; }
  postMessage(data) {
    const target = this._other;
    if (!target) return;
    // live lookup — lands in the owned/virtual timer queue, NOT host MessageChannel
    globalThis.setTimeout(() => {
      const ev = { data, type: 'message', target };
      if (typeof target.onmessage === 'function') target.onmessage(ev);
      target.dispatchEvent?.(Object.assign(new Event('message'), { data }));
    }, 0);
  }
}

class MessageChannel {
  constructor() {
    this.port1 = new MessagePort();
    this.port2 = new MessagePort();
    this.port1._other = this.port2;
    this.port2._other = this.port1;
  }
}
```

Notes:
- **Use `globalThis.setTimeout` read at call time** (same principle as the C2
  clock hook / `rAF → globalThis.setTimeout`), so a consumer's owned timers win.
- React only needs `port.postMessage` + `port.onmessage` (+ `start()`); the full
  transferable/`addEventListener` surface is optional but cheap to include.
- **Default safety:** routing via `setTimeout` is deterministic and works under
  jest/vitest too (they advance timers). If you want zero behavior change when no
  custom clock is installed, you may keep the host MessageChannel when
  `globalThis.MessageChannel` exists *and* no clock hook is set — but the
  setTimeout-routed polyfill is simpler and composes everywhere, so prefer it.

## Validate

With turbo-crawl's virtual-clock drain (C1) owning `setTimeout`:
- React's scheduler hops appear in the owned queue (turbo-crawl can count them).
- `node payroll-app-turbocrawl/e2e/turbo/smoke.mjs` against a page with
  client-rendered `data-test-id`s → count goes **0 → >0**, render returns inside
  the deadline, **CPU not pegged**.

## The three coordinated pieces

1. **C1 — turbo-crawl** (owns the pump): virtual clock + own `setTimeout`/rAF/
   **MessageChannel** delivery, drain advancing `vnow`, deadline backstop.
2. **C2 — turbo-dom** (clock hook): `rAF` + `performanceNow` read a settable
   clock / route through `globalThis.setTimeout`.
3. **This doc — turbo-dom** (MessageChannel): real `MessagePort` that posts via
   `globalThis.setTimeout`, so React's scheduler is virtual-clock-drivable and
   exists in the bare isolate.

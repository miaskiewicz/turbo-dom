# React 19 DOM gaps (found via turbo-crawl render tier)

> **RESOLVED in v0.2.2.** Added `Element.removeAttributeNode`/`setAttributeNode`
> (+ NS aliases) to `src/runtime/dom.mjs`. `focus`/`blur` and every "likely
> further gap" (`toggleAttribute`, `isConnected`, `matches`/`closest`,
> `scrollIntoView`, `getRootNode`, …) already existed. `removeEventListener` is
> on the runtime's own `EventTarget` (`events.mjs`), so it works in the bare WASM
> isolate too. Tests in `test/surface-sweep.test.mjs`.

Date: 2026-06-17
Found while running a **Next.js 15 (App Router, Turbopack) app** through
turbo-crawl's JS-execution render tier (turbo-dom as the DOM). After the
turbo-crawl-side fixes landed (`document.currentScript` + raw chunk `src` +
chunk-`load` doorbell), the Turbopack runtime evaluates the entrypoint and
**React 19 begins hydration** — then crashes inside React's commit phase on a
DOM method turbo-dom doesn't implement.

This doc is the hand-off for fixing those gaps **in turbo-dom**. turbo-crawl
needs no further change for this; the remaining work is DOM-method completeness.

## The crash (confirmed)

```
TypeError: instance.removeAttributeNode is not a function
    at releaseSingletonInstance (react-dom)
    at commitDeletionEffectsOnFiber
    at commitMutationEffectsOnFiber
    ...
```

React 19's `releaseSingletonInstance` (run when unmounting/resetting the
singleton host nodes — `<html>`, `<head>`, `<body>`) iterates the node's
attributes and removes each via **`Element.removeAttributeNode(attr)`**, passing
back the objects it read from `node.attributes`. turbo-dom's `Element` has no
`removeAttributeNode`, so the call throws, the commit aborts, and (in the
in-process `node:vm` "fast" host) the error surfaces through Node's `EventTarget`
on `process.nextTick` and crashes the host. (The isolate/"secure" backend would
instead catch it per-timer and return a partial snapshot — but it's still the
same missing method.)

## Missing methods (scanned `src/runtime/dom.mjs`)

| method | status | notes |
|---|---|---|
| `Element.removeAttributeNode(attr)` | **MISSING** | the crash above |
| `Element.setAttributeNode(attr)` | **MISSING** | the symmetric pair; libraries that read/move Attr nodes use it |
| `HTMLElement.blur()` | **MISSING** | `focus()` exists; React focus management calls both |
| `EventTarget.removeEventListener` | **VERIFY** | absent from `dom.mjs`; `Node extends EventTarget` so it may be inherited. Native `EventTarget` (fast/Node host) has it; confirm the **WASM-isolate** EventTarget used by the secure path also exposes it. |

`getAttributeNode(name)` already exists, so the attribute data already round-trips
as a plain object — the node methods just need wiring to it.

## Data-model notes (so the implementation matches what exists)

turbo-dom does **not** have a real `Attr` class or live `NamedNodeMap`. Today:

- `getAttributeNode(name)` → a fresh plain object `{ name, value, ownerElement }`
  (not a live Attr; new object each call).
- `get attributes()` → a fresh **array** of `{ name, localName, value, prefix,
  namespaceURI }` (not a NamedNodeMap).
- `removeAttribute(name)` / `setAttribute(name, value)` mutate the internal
  `this.__attrs` array (entries `{ name, value, prefix }`), rebuilt lazily via
  `__buildAttrs()`.

So the new node methods are thin shims over the existing **name-based** methods.

## Suggested implementations

```js
// on Element
removeAttributeNode(attr) {
  // React passes objects it read from node.attributes (plain { name, value, ... }).
  const removed = this.getAttributeNode(attr && attr.name);
  if (attr && attr.name != null) this.removeAttribute(attr.name);
  return removed; // spec returns the removed Attr
}

setAttributeNode(attr) {
  const prev = this.getAttributeNode(attr.name);
  this.setAttribute(attr.name, attr.value);
  return prev; // spec returns the replaced Attr, or null
}

// on HTMLElement (mirror focus())
blur() {
  const doc = this.ownerDocument;
  if (doc && doc.__active === this) doc.__active = doc.body ?? null;
  // dispatch a blur Event if focus() dispatches focus — keep them symmetric
}
```

Note: React reads a **snapshot** of `node.attributes` then removes each — since
`get attributes()` already returns a detached array copy, iterating it while
`removeAttribute` mutates `__attrs` is safe.

## Likely further gaps (pre-empt the long tail)

React 19 / Next App Router hydration over a non-trivial app will probably also
touch, if not already present: `Element.toggleAttribute`, `Node.isConnected`,
`Element.matches`/`closest`, `Element.scrollIntoView` (can be a no-op),
`HTMLElement.dataset` writes, `style.setProperty`/`cssText`, `classList.toggle`,
and `getRootNode()`. Worth grepping `dom.mjs` for each and stubbing the
no-op-safe ones, so the next render gets further in one pass instead of
crash-by-crash.

## How to validate the fix

1. Rebuild turbo-dom (native + `pkg-web` WASM) so both hosts pick up the methods.
2. In `payroll-app-turbocrawl` (turbo-crawl is symlinked there), with the Next
   dev server up on :3010:
   ```sh
   node e2e/turbo/smoke.mjs     # JS-render tier should no longer crash
   node e2e/turbo/probe4.mjs    # requestAnimationFrame should flip 0 -> >0
   ```
   Win condition: **smoke `data-test-id` count goes 0 → large**, no host crash.
3. A prod build (`npm run build && PORT=3010 npm start`) is the cleaner target —
   the prod Turbopack runtime has no HMR chunk-load gate and evaluates entries
   eagerly.

## Cross-reference

turbo-crawl side (already fixed + released-pending):
`turbo-crawl/FINDINGS-nextjs-render-tier.md` and commits
`fix(render): set document.currentScript …` + `fix(render): run scripts as real
<script> nodes …`.

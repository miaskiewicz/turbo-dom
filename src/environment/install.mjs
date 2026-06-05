// Install a turbodom window/document onto a global object (the shared core of
// the vitest + jest environment adapters). Globals are defined as getters that
// pull from the lazy window Proxy, so laziness + the touch-tracer are preserved.

import { createEnvironment } from '../runtime/index.mjs';

const DEFAULT_HTML = '<!doctype html><html><head></head><body></body></html>';

// Globals that point at the window itself.
const SELF_KEYS = ['window', 'self', 'globalThis', 'parent', 'top', 'frames'];

export function installGlobals(target, { html = DEFAULT_HTML, url } = {}) {
  const env = createEnvironment(html, url ? { url } : {});
  const { window } = env;

  const define = (name, getter) => {
    Object.defineProperty(target, name, { configurable: true, get: getter, set(v) { Object.defineProperty(target, name, { configurable: true, writable: true, value: v }); } });
  };

  // window self-references
  for (const k of SELF_KEYS) define(k, () => window);
  // document is eager + universal
  Object.defineProperty(target, 'document', { configurable: true, writable: true, value: env.document });

  // every other window global → lazy getter (materializes + traces on first read)
  for (const name of env.globalKeys) {
    if (name === 'document' || SELF_KEYS.includes(name)) continue;
    define(name, () => window[name]);
  }

  // handy escape hatches for adapters / per-test reset
  target.__turbodom = env;
  return env;
}

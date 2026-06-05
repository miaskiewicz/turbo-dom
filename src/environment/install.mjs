// Install a turbo-dom window/document onto a global object (the shared core of
// the vitest + jest environment adapters). Globals are defined as getters that
// pull from the lazy window Proxy, so laziness + the touch-tracer are preserved.

import { createEnvironment } from '../runtime/index.mjs';

const DEFAULT_HTML = '<!doctype html><html><head></head><body></body></html>';

// Globals that point at the window itself. Deliberately NOT `globalThis`/`global`
// — redefining those breaks test runners (vitest builds its module-runner vm
// primitives off globalThis; pointing it at our window Proxy hides Symbol et al).
const SELF_KEYS = ['window', 'self', 'parent', 'top', 'frames'];

export function installGlobals(target, { html = DEFAULT_HTML, url } = {}) {
  const env = createEnvironment(html, url ? { url } : {});
  const { window } = env;

  const installed = [];                 // keys we defined
  const originals = new Map();          // prior descriptors to restore on teardown

  const define = (name, descriptor) => {
    const prior = Object.getOwnPropertyDescriptor(target, name);
    if (prior) originals.set(name, prior);
    Object.defineProperty(target, name, descriptor);
    installed.push(name);
  };

  // window self-references
  for (const k of SELF_KEYS) {
    define(k, {
      configurable: true,
      get: () => window,
      set(v) { Object.defineProperty(target, k, { configurable: true, writable: true, value: v }); },
    });
  }
  // document is eager + universal
  define('document', { configurable: true, writable: true, enumerable: true, value: env.document });

  // every other window global → lazy getter (materializes + traces on first read)
  for (const name of env.globalKeys) {
    if (name === 'document' || SELF_KEYS.includes(name)) continue;
    define(name, {
      configurable: true,
      get: () => window[name],
      set(v) { Object.defineProperty(target, name, { configurable: true, writable: true, value: v }); },
    });
  }

  // handy escape hatch
  define('__turboDom', { configurable: true, writable: true, value: env });

  const teardown = () => {
    for (const name of installed) {
      delete target[name];
      const prior = originals.get(name);
      if (prior) Object.defineProperty(target, name, prior);
    }
  };

  return { env, window, teardown };
}

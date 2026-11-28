// Install a turbo-dom window/document onto a global object (the shared core of
// the vitest + jest environment adapters). Globals are defined as getters that
// pull from the lazy window Proxy, so laziness + the touch-tracer are preserved.

import { bindNativeCssInstalledWindow, unbindNativeCssInstalledWindow, createNativeCssCodecGetter } from '../runtime/window.mjs';
import { createEnvironment } from '../runtime/index.mjs';

const DEFAULT_HTML = '<!doctype html><html><head></head><body></body></html>';

// Globals that point at the window itself. Deliberately NOT `globalThis`/`global`
// — redefining those breaks test runners (vitest builds its module-runner vm
// primitives off globalThis; pointing it at our window Proxy hides Symbol et al).
const SELF_KEYS = ['window', 'self', 'parent', 'top', 'frames'];

function installedGetter(document,window,name){
 return name==='TextEncoder'||name==='TextDecoder'?createNativeCssCodecGetter(document,window,name):()=>window[name];
}
function recordCssGetter(records,name,get){
 if(name==='innerWidth'||name==='innerHeight'||name==='devicePixelRatio'||name==='TextEncoder'||name==='TextDecoder')records[name]=get;
}
export function installGlobals(target, { html = DEFAULT_HTML, url, domBackend, cssBackend, cssResultProtocol, cssRequestProtocol } = {}) {
  const env = createEnvironment(html, { url: url || undefined, domBackend, cssBackend, cssResultProtocol, cssRequestProtocol });
  const { window } = env;

  const cssViewportGetters = {};
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
    // window/self/parent/top resolve to the GLOBAL itself (target), like a real
    // browser where window === globalThis. This is what makes vi.stubGlobal('x')
    // visible via window.x — they're the same object/property, not two bindings.
    define(k, {
      configurable: true,
      get: () => target,
      set(v) { Object.defineProperty(target, k, { configurable: true, writable: true, value: v }); },
    });
  }
  // document is eager + universal
  define('document', { configurable: true, writable: true, enumerable: true, value: env.document });

  // every other window global → lazy getter (materializes + traces on first read)
  for (const name of env.globalKeys) {
    if (name === 'document' || SELF_KEYS.includes(name)) continue;
    const get = installedGetter(env.document,window,name);
    recordCssGetter(cssViewportGetters,name,get);
    define(name, {
      configurable: true,
      get,
      set(v) { Object.defineProperty(target, name, { configurable: true, writable: true, value: v }); },
    });
  }

  // window === globalThis (target): keep document.defaultView consistent so
  // `window === document.defaultView` holds and stubs on the global are seen.
  try { env.document.defaultView = target; } catch { /* getter-only in some setups */ }

  bindNativeCssInstalledWindow(env.document,target,cssViewportGetters);

  // handy escape hatch
  define('__turboDom', { configurable: true, writable: true, value: env });

  const teardown = () => {
    unbindNativeCssInstalledWindow(env.document,target);
    for (const name of installed) {
      delete target[name];
      const prior = originals.get(name);
      if (prior) Object.defineProperty(target, name, prior);
    }
  };

  return { env, window, teardown };
}

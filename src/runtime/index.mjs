// fast-dom test runtime — assembles Layers 1–5 into a jsdom-like environment.
//
//   import { createEnvironment } from './src/runtime/index.mjs';
//   const env = createEnvironment('<!doctype html><body><div id=app></div></body>');
//   env.window.document.querySelector('#app');
//   env.reset();                 // Layer 5: cheap per-file reset
//   env.reset('<body>next</body>');

import { createRequire } from 'node:module';
import { Document } from './dom.mjs';
import { createWindow } from './window.mjs';

const require = createRequire(import.meta.url);
const native = require('../../index.js');

export { Document } from './dom.mjs';
export * from './dom.mjs';

export function createEnvironment(html = '<!doctype html><html><head></head><body></body></html>', options = {}) {
  // Layer 1: native parse → immutable SoA buffer (typed arrays, one boundary copy).
  let soa = native.parseBuffer(String(html));

  // Layer 2: Document over the buffer (nodes inflate lazily from the arrays).
  const document = new Document();
  document.__load(soa);

  // Layer 3: lazy window.
  const win = createWindow(document, options);

  return {
    window: win.window,
    document,
    touched: win.touched,

    // Layer 5: arena-style reset. Re-point at the (re)parsed buffer, drop the
    // owned overlay + node cache + materialized globals. Class machinery stays warm.
    reset(nextHtml) {
      if (nextHtml !== undefined) soa = native.parseBuffer(String(nextHtml));
      document.__load(soa);       // drops __cache + __kids overlay, keeps the buffer if reused
      win.resetGlobals();
      document.__active = null;
      document.__cookie = '';
    },
  };
}

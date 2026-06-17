// turbo-dom test runtime — assembles Layers 1–5 into a jsdom-like environment.
//
//   import { createEnvironment } from './src/runtime/index.mjs';
//   const env = createEnvironment('<!doctype html><body><div id=app></div></body>');
//   env.window.document.querySelector('#app');
//   env.reset();                 // Layer 5: cheap per-file reset
//   env.reset('<body>next</body>');

import { Document } from './dom.mjs';
import { createWindow } from './window.mjs';
import { unpack } from './buffer.mjs';
import { getParser, setParserMode } from './parser.mjs';

export { Document } from './dom.mjs';
export * from './dom.mjs';

// Parser-selection seam (public API): inject a ready binding for a node-free host,
// or force 'wasm' | 'native' | 'auto'. See parser.mjs.
export { setParser, setParserMode, getParser } from './parser.mjs';

// Clock seam (public API): install a virtual clock so time-gated rAF loops (MUI
// transitions) terminate under a synchronous timer-drain render tier. See window.mjs.
export { setClock } from './window.mjs';

// Parse cache: the SoA buffer is READ-ONLY (every mutation goes to a Document's
// own __kids/__attrs/__cache overlay, never the buffer), so the same buffer can
// back many Documents. Test suites call setup with the SAME html per file
// (usually the empty default) → parse once, reuse for every file, skipping the
// native parse + boundary marshaling entirely. Bounded (fixtures are few).
const __parseCache = new Map();
const __PARSE_CACHE_MAX = 64;
let __parseCacheMRU; // last (most-recently-used) key — skip the LRU re-insert when unchanged
function parseBufferCached(html) {
  const hit = __parseCache.get(html);
  if (hit !== undefined) {
    // LRU bump: re-insert so this entry is most-recently-used (Map keeps order).
    // Skip the delete+set entirely when this key is ALREADY the MRU — the common
    // case for a suite that reuses one shell HTML across every file.
    if (html !== __parseCacheMRU) { __parseCache.delete(html); __parseCache.set(html, hit); __parseCacheMRU = html; }
    return hit;
  }
  let soa = getParser().parseBuffer(html);
  // Unpack the packed blob into typed-array views ONCE here, not per Document in
  // every Buffer ctor — the views are read-only over the shared immutable buffer,
  // so all Documents backed by this cached soa reuse them (no re-unpack per file).
  if (soa.packed) soa = unpack(soa);
  // Evict the single oldest entry, not the whole cache — a suite with >64 distinct
  // shells should keep its hot fixtures warm, not thrash every one cold on overflow.
  if (__parseCache.size >= __PARSE_CACHE_MAX) __parseCache.delete(__parseCache.keys().next().value);
  __parseCache.set(html, soa);
  __parseCacheMRU = html;
  return soa;
}

// Declarative Shadow DOM: promote every `<template shadowrootmode="open|closed">`
// into a real ShadowRoot on its parent (content moved in, template removed). This
// is a DOM walk, so it's gated by a cheap substring check on the HTML source in
// createEnvironment/reset — a document with no declarative shadow root pays
// nothing (the walk never runs).
function promoteDeclarativeShadowRoots(document) {
  for (const tpl of Array.from(document.getElementsByTagName('template'))) {
    const mode = tpl.getAttribute('shadowrootmode');
    if (mode !== 'open' && mode !== 'closed') continue;
    const host = tpl.parentNode;
    if (!host || host.nodeType !== 1 || host.__shadow) continue;
    const root = host.attachShadow({ mode, delegatesFocus: tpl.hasAttribute('shadowrootdelegatesfocus') });
    if (tpl.content) for (const c of tpl.content.__children().slice()) root.appendChild(c);
    host.removeChild(tpl);
  }
}

export function createEnvironment(html = '<!doctype html><html><head></head><body></body></html>', options = {}) {
  // Optional parser selection: options.parser === 'wasm' | 'native' | 'auto'. Applied
  // before the first parse below; process-global (the parser binding is a singleton),
  // so the last mode set wins. Embedders wanting a fully node-free path inject a
  // binding via setParser()/globalThis.__TURBO_DOM_PARSER__ instead.
  if (options.parser) setParserMode(options.parser);

  // Layer 1: native parse → immutable SoA buffer (typed arrays, one boundary copy).
  let currentHtml = String(html);
  let soa = parseBufferCached(currentHtml);

  // Layer 2: Document over the buffer (nodes inflate lazily from the arrays).
  const document = new Document();
  // declarative shadow roots only when the source even mentions them (cheap gate)
  const maybePromote = () => { if (currentHtml.includes('shadowroot')) promoteDeclarativeShadowRoots(document); };
  document.__load(soa);
  maybePromote();

  // Layer 3: lazy window.
  const win = createWindow(document, options);

  return {
    window: win.window,
    document,
    touched: win.touched,
    globalKeys: win.globalKeys,

    // Layer 5: arena-style reset. Re-point at the (re)parsed buffer, drop the
    // owned overlay + node cache + materialized globals. Class machinery stays warm.
    reset(nextHtml) {
      if (nextHtml !== undefined) { currentHtml = String(nextHtml); soa = parseBufferCached(currentHtml); }
      document.__load(soa);       // drops __cache + __kids overlay, keeps the buffer if reused
      maybePromote();
      win.resetGlobals();
      document.__active = null;
      document.__cookieJar = null;
    },
  };
}

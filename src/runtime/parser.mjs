// Parser binding registry for the whole runtime (dom.mjs, index.mjs).
//
// Resolution order, lazily on the FIRST parse (getParser):
//   1. an explicitly injected binding — setParser(b) or globalThis.__TURBO_DOM_PARSER__.
//      This is the node-free path: an embedder (e.g. a non-Node V8 host) instantiates
//      the wasm itself and injects the {parse,parseBuffer,parseFragment} object, so
//      NONE of the Node loaders below ever run.
//   2. an explicit mode — setParserMode(m), the `parser` option on createEnvironment,
//      TURBO_DOM_PARSER env, or globalThis.__TURBO_DOM_PARSER_MODE__:
//      'wasm' loads pkg/, 'native' loads the prebuilt .node addon.
//   3. auto (default): native addon first, wasm fallback.
//
// node:module is imported via GUARDED top-level await, so this module still LOADS in
// a bare V8 with no Node builtins — provided a parser is injected before the first
// parse, the Node `require` loaders are never reached. (Node hosts get it normally.)

let _createRequire;
try { ({ createRequire: _createRequire } = await import('node:module')); } catch { /* bare V8: parser must be injected */ }

let _parser = null;
let _mode = null; // 'wasm' | 'native' | 'auto' | null (consult env/global)

function req(spec) {
  if (!_createRequire) {
    throw new Error(
      'turbo-dom: no parser available — node:module is missing (bare V8) and nothing was injected. ' +
      'Call setParser(binding) (or set globalThis.__TURBO_DOM_PARSER__) before the first parse.',
    );
  }
  return _createRequire(import.meta.url)(spec);
}
const loadNative = () => req('../../index.js');
const loadWasm = () => req('../../pkg/turbo_dom_parser.js');

// Inject a ready parser binding ({parse, parseBuffer, parseFragment}). Pass null to
// clear it (next getParser re-resolves). The node-free seam for embedders.
export function setParser(binding) { _parser = binding || null; }

// Force a parser mode: 'wasm' | 'native' | 'auto' (or null to consult env/global).
// Resets the resolved parser so the next parse honors the new mode.
export function setParserMode(mode) {
  _mode = mode || null;
  _parser = null;
}

function resolveMode() {
  if (_mode) return _mode;
  if (globalThis.__TURBO_DOM_PARSER_MODE__) return globalThis.__TURBO_DOM_PARSER_MODE__;
  if (typeof process !== 'undefined' && process.env && process.env.TURBO_DOM_PARSER) return process.env.TURBO_DOM_PARSER;
  return 'auto';
}

// Resolve (once) and return the active parser binding. Memoized — every call after
// the first is a single field read, so call sites can use getParser().parseBuffer(…)
// freely without caching it themselves.
export function getParser() {
  if (_parser) return _parser;
  if (globalThis.__TURBO_DOM_PARSER__) return (_parser = globalThis.__TURBO_DOM_PARSER__);
  const mode = resolveMode();
  if (mode === 'wasm') return (_parser = loadWasm());
  if (mode === 'native') return (_parser = loadNative());
  // auto: native addon first, wasm fallback; surface the native error if both fail.
  try { _parser = loadNative(); }
  catch (nativeErr) { try { _parser = loadWasm(); } catch { throw nativeErr; } }
  return _parser;
}

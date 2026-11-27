import { nativeCssSuffix, loadCssModule, detectNativeMusl } from './css-native-loader.mjs';
import { captureParserOrigin, parserDecoderSafe, originalWasmAvailability } from './parser-origin.mjs';
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

// Provenance is recorded only around the unchanged built-in parser call. Injected
// parse-only objects retain their API but cannot certify raw backing stores.
const cssOrigins = new WeakMap(), cssBuiltins = new WeakMap();
const cssApply = Reflect.apply, cssDescriptor = Object.getOwnPropertyDescriptor,cssOwn=Object.hasOwn;
const cssGet = WeakMap.prototype.get, cssSet = WeakMap.prototype.set;
let cssBuiltin = null;
function rememberCssBuiltin(binding, cachedTextCodecs) {
  let record = cssApply(cssGet, cssBuiltins, [binding]);
  if (!record) {
    const field = cssDescriptor(binding, 'parseBuffer');
    if (field && cssOwn(field, 'value') && typeof field.value === 'function') {
      record = {binding, fn:field.value, cachedTextCodecs};
      cssApply(cssSet, cssBuiltins, [binding, record]);
    }
  }
  cssBuiltin = record ?? null;
  return binding;
}
function cssBuiltinCurrent(record) {
  const field=cssDescriptor(record.binding,'parseBuffer');
  return !!field&&cssOwn(field,'value')&&field.value===record.fn;
}
export function readParserCssOrigin(value) { return cssApply(cssGet, cssOrigins, [value]) ?? null; }
function cssProducerTrusted(html,binding,fn,original){
 return typeof html==='string'&&original?.binding===binding&&original.fn===fn&&original.cachedTextCodecs&&cssBuiltinCurrent(original)&&parserDecoderSafe({cachedTextCodecs:true});
}
function selectCombinedNative(preferCombinedNative) {
  if (preferCombinedNative && !_parser && !globalThis.__TURBO_DOM_PARSER__ && resolveMode() === 'native') {
    _parser = loadCombinedNative();
  }
}
export function parseBufferForRuntime(html, preferCombinedNative = false) {
  selectCombinedNative(preferCombinedNative);
  const binding = getParser(), fn = binding.parseBuffer, original = cssBuiltin;
  const trusted = cssProducerTrusted(html,binding,fn,original);
  const result = cssApply(fn, binding, [html]);
  if (trusted && cssBuiltinCurrent(original) && parserDecoderSafe({cachedTextCodecs:true})) {
    const origin = captureParserOrigin(result, () => cssBuiltinCurrent(original));
    if (origin) cssApply(cssSet, cssOrigins, [result, origin]);
  }
  return result;
}
let _parser = null;
let _revision = 0;
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
function loadCssBuiltin(spec,wasm=false) {
  const before=parserDecoderSafe()&&(!wasm||originalWasmAvailability()===true);
  const binding=req(spec);
  return rememberCssBuiltin(binding,before&&parserDecoderSafe()&&(!wasm||originalWasmAvailability()===true));
}
const cssDefine = Object.defineProperty;
function parserFacade(binding) {
  const facade = {};
  const names = ['parse', 'parseRaw', 'parseBuffer', 'parseFragment'];
  for (let index = 0; index < names.length; index++) {
    const name = names[index];
    const field = cssDescriptor(binding, name);
    if (!field || !cssOwn(field, 'value') || typeof field.value !== 'function')
      throw new TypeError(`turbo-dom: combined native parser missing ${name}`);
    cssDefine(facade, name, {__proto__:null, value:field.value, writable:true, enumerable:true, configurable:true});
  }
  return facade;
}
function loadCombinedNative() {
  const before = parserDecoderSafe(), require = createParserRequire();
  if (!require) return null;
  const host = require('node:process');
  const suffix = nativeCssSuffix(host.platform, host.arch, host.platform === 'linux' && detectNativeMusl(require, host));
  if (!suffix) return null;
  const binding = loadCssModule(require, `../../native-css/turbo-dom-css.${suffix}.node`);
  if (binding === null) return null;
  return rememberCssBuiltin(parserFacade(binding), before && parserDecoderSafe());
}
const loadNative = () => loadCssBuiltin('../../index.js');
const loadWasm = () => loadCssBuiltin('../../pkg/turbo_dom_parser.js',true);

// Inject a ready parser binding ({parse, parseBuffer, parseFragment}). Pass null to
// clear it (next getParser re-resolves). The node-free seam for embedders.
export function setParser(binding) { cssBuiltin = null; _parser = binding || null; _revision++; }

// Force a parser mode: 'wasm' | 'native' | 'auto' (or null to consult env/global).
// Resets the resolved parser so the next parse honors the new mode.
export function setParserMode(mode) {
  cssBuiltin = null;
  _revision++;
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
  if (globalThis.__TURBO_DOM_PARSER__) { cssBuiltin = null; return (_parser = globalThis.__TURBO_DOM_PARSER__); }
  const mode = resolveMode();
  if (mode === 'wasm') return (_parser = loadWasm());
  if (mode === 'native') return (_parser = loadNative());
  // auto: native addon first, wasm fallback; surface the native error if both fail.
  try { _parser = loadNative(); }
  catch (nativeErr) { try { _parser = loadWasm(); } catch { throw nativeErr; } }
  return _parser;
}

// Internal CSS selection does not force HTML parsing or artifact initialization.
// Global/environment modes retain the same precedence as getParser().
export function getParserSelection() {
  return { mode: resolveMode(), binding: _parser || globalThis.__TURBO_DOM_PARSER__ || null, revision: _revision };
}

// Internal optional loader capability; preserves the existing guarded Node import.
export function createParserRequire() {
  return _createRequire ? _createRequire(import.meta.url) : null;
}

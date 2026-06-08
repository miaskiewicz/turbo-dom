// Parser-selection seam (#2): mode select ('wasm'|'native'|'auto'), injection
// (setParser / globalThis.__TURBO_DOM_PARSER__ / env), and the node-free wasm recipe
// (pkg-web initSync → setParser). Node isolates each test FILE in its own process, so
// the process-global parser state mutated here never leaks to other suites; within
// this file each test resets it.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
  createEnvironment, setParser, setParserMode, getParser,
} from '../src/runtime/index.mjs';

// The wasm builds are optional artifacts (npm run build:wasm:pkg / :web). Skip the
// wasm-backed subtests when they aren't present so a fresh clone / native-only CI
// stays green; native + injection paths always run.
const HAS_WASM = existsSync(fileURLToPath(new URL('../pkg/turbo_dom_parser.js', import.meta.url)));
const HAS_WASM_WEB = existsSync(fileURLToPath(new URL('../pkg-web/turbo_dom_parser.js', import.meta.url)));

const reset = () => { setParser(null); setParserMode(null); delete globalThis.__TURBO_DOM_PARSER__; delete globalThis.__TURBO_DOM_PARSER_MODE__; delete process.env.TURBO_DOM_PARSER; };

test('auto (default) resolves a working parser', () => {
  reset();
  const p = getParser();
  assert.equal(typeof p.parseBuffer, 'function');
  const e = createEnvironment('<div id=a>auto</div>');
  assert.equal(e.document.querySelector('#a').textContent, 'auto');
  reset();
});

test("setParserMode('wasm') and ('native') both parse", { skip: HAS_WASM ? false : 'wasm pkg not built' }, () => {
  reset();
  setParserMode('wasm');
  let e = createEnvironment('<div id=w>wasm</div>');
  assert.equal(e.document.querySelector('#w').textContent, 'wasm');
  setParserMode('native');
  e = createEnvironment('<div id=n>native</div>');
  assert.equal(e.document.querySelector('#n').textContent, 'native');
  reset();
});

test('createEnvironment({ parser }) option selects the mode', { skip: HAS_WASM ? false : 'wasm pkg not built' }, () => {
  reset();
  const e = createEnvironment('<div id=o>opt</div>', { parser: 'wasm' });
  assert.equal(e.document.querySelector('#o').textContent, 'opt');
  reset();
});

test('setParser injects a binding; getParser returns it without loading', () => {
  reset();
  let calls = 0;
  const real = getParser();
  setParser({
    parse: real.parse,
    parseBuffer: (h) => { calls++; return real.parseBuffer(h); },
    parseFragment: real.parseFragment,
  });
  assert.equal(getParser().parseBuffer === undefined, false);
  const e = createEnvironment('<div id=i>inj</div>');
  assert.equal(e.document.querySelector('#i').textContent, 'inj');
  assert.ok(calls > 0, 'injected parseBuffer was used');
  reset();
});

test('globalThis.__TURBO_DOM_PARSER__ is honored', () => {
  reset();
  const real = getParser();
  reset();
  let used = false;
  globalThis.__TURBO_DOM_PARSER__ = {
    parse: real.parse,
    parseBuffer: (h) => { used = true; return real.parseBuffer(h); },
    parseFragment: real.parseFragment,
  };
  const e = createEnvironment('<div id=g>glob</div>');
  assert.equal(e.document.querySelector('#g').textContent, 'glob');
  assert.ok(used);
  reset();
});

test('globalThis.__TURBO_DOM_PARSER_MODE__ and env TURBO_DOM_PARSER select wasm', { skip: HAS_WASM ? false : 'wasm pkg not built' }, () => {
  reset();
  globalThis.__TURBO_DOM_PARSER_MODE__ = 'wasm';
  let e = createEnvironment('<div id=gm>gmode</div>');
  assert.equal(e.document.querySelector('#gm').textContent, 'gmode');
  reset();
  process.env.TURBO_DOM_PARSER = 'wasm';
  e = createEnvironment('<div id=em>emode</div>');
  assert.equal(e.document.querySelector('#em').textContent, 'emode');
  reset();
});

test('node-free recipe: pkg-web initSync → setParser (no Node builtins in the glue)', { skip: HAS_WASM_WEB ? false : 'pkg-web not built' }, async () => {
  reset();
  // The embedder supplies wasm bytes (here via fs, in a non-Node host via its own
  // loader) and instantiates synchronously, then injects the binding.
  const web = await import('../pkg-web/turbo_dom_parser.js');
  const wasmPath = fileURLToPath(new URL('../pkg-web/turbo_dom_parser_bg.wasm', import.meta.url));
  web.initSync({ module: readFileSync(wasmPath) });
  setParser({ parse: web.parse, parseBuffer: web.parseBuffer, parseFragment: web.parseFragment });
  const e = createEnvironment('<div id=nf class=x>nodefree</div><span>s</span>');
  assert.equal(e.document.querySelector('#nf').textContent, 'nodefree');
  assert.equal(e.document.querySelector('.x').className, 'x');
  // fragment parse path (innerHTML) also routes through the injected binding
  e.document.querySelector('#nf').innerHTML = '<b>bold</b>';
  assert.equal(e.document.querySelector('b').textContent, 'bold');
  reset();
});

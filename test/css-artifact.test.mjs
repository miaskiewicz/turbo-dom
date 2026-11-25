import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createCssArtifactResolver,
  detectNativeMusl,
  loadCssModule,
  nativeCssSuffix,
  normalizeCssArtifact,
} from '../src/runtime/css-artifact.mjs';
import { getParserSelection, setParser, setParserMode } from '../src/runtime/parser.mjs';
const CompiledStylesV1 = class {};
const raw = () => ({
  cssAbiVersion() {
    return 1;
  },
  cssBackendVersion() {
    return '0.8.7';
  },
  CompiledStylesV1,
});
const selection = (mode, binding = null, revision = 0) => ({ mode, binding, revision });
function loader(value, calls) {
  function require(path) {
    calls.push(path);
    if (value instanceof Error) throw value;
    return value;
  }
  require.resolve = (spec) => spec;
  return require;
}
test('normalizes real raw functions and rejects partial/version mismatch without hiding original errors', () => {
  assert.deepEqual(normalizeCssArtifact(raw()), { cssAbiVersion: 1, CompiledStylesV1 });
  assert.equal(normalizeCssArtifact({ parse() {} }), null);
  assert.equal(normalizeCssArtifact(null), null);
  for (const value of [{ cssAbiVersion: 2 }, { ...raw(), cssBackendVersion: '0.9.0' }, { cssAbiVersion: 1 }]) {
    assert.throws(() => normalizeCssArtifact(value), TypeError);
  }
  const error = {};
  assert.throws(
    () =>
      normalizeCssArtifact({
        cssAbiVersion() {
          throw error;
        },
      }),
    (e) => e === error,
  );
});
test('native does not inspect WebAssembly; wasm does not load native; injected capability wins', () => {
  const calls = [];
  const resolver = createCssArtifactResolver({
    require: loader(raw(), calls),
    platform: 'darwin',
    arch: 'arm64',
    hasWebAssembly() {
      throw Error('not native');
    },
  });
  assert.equal(resolver.resolve(selection('native')).CompiledStylesV1, CompiledStylesV1);
  assert.deepEqual(calls, ['../../native-css/turbo-dom-css.darwin-arm64.node']);
  const wasmCalls = [];
  const wasm = createCssArtifactResolver({ require: loader(raw(), wasmCalls) });
  wasm.resolve(selection('wasm'));
  assert.deepEqual(wasmCalls, ['../../pkg-css/turbo_dom_css.js']);
  wasm.resolve(selection('wasm', raw(), 1));
  assert.equal(wasmCalls.length, 1);
});
test('preinitialized web namespace and genuinely capability-absent hosts', () => {
  const wasm = createCssArtifactResolver({
    require: null,
    preinitializedWasm: raw(),
    hasWebAssembly() {
      throw Error('already initialized');
    },
  });
  assert.ok(wasm.resolve(selection('wasm')));
  assert.equal(createCssArtifactResolver({ require: null }).resolve(selection('auto', { parse() {} })), null);
  assert.equal(
    createCssArtifactResolver({ require: loader(raw(), []), hasWebAssembly: () => false }).resolve(selection('wasm')),
    null,
  );
});
test('only exact resolution absence falls back; transitive and initialization errors preserve identity', () => {
  const missing = Object.assign(
    Error("Cannot find module '../../native-css/turbo-dom-css.darwin-arm64.node'\nRequire stack:"),
    { code: 'MODULE_NOT_FOUND' },
  );
  const calls = [];
  const require = loader(raw(), calls);
  require.resolve = (spec) => {
    if (spec.includes('native-css')) throw missing;
    return spec;
  };
  assert.ok(createCssArtifactResolver({ require, platform: 'darwin', arch: 'arm64' }).resolve(selection('auto')));
  assert.equal(calls.length, 1);
  const nested = Object.assign(Error("Cannot find module 'dependency'"), { code: 'MODULE_NOT_FOUND' });
  const broken = loader(nested, []);
  assert.throws(
    () => loadCssModule(broken, 'artifact'),
    (e) => e === nested,
  );
  broken.resolve = () => {
    throw nested;
  };
  assert.throws(
    () => loadCssModule(broken, 'artifact'),
    (e) => e === nested,
  );
});
test('platform suffixes are deterministic', () => {
  for (const [p, a, m, expected] of [
    ['darwin', 'arm64', false, 'darwin-arm64'],
    ['linux', 'x64', true, 'linux-x64-musl'],
    ['linux', 'arm64', false, 'linux-arm64-gnu'],
    ['win32', 'x64', false, 'win32-x64-msvc'],
    ['android', 'arm', false, 'android-arm-eabi'],
    ['freebsd', 'x64', false, 'freebsd-x64'],
    ['other', 'x64', false, null],
  ]) {
    assert.equal(nativeCssSuffix(p, a, m), expected);
  }
});
test('selection snapshot is lazy and preserves explicit/global mode precedence and parser identity', () => {
  const globalMode = globalThis.__TURBO_DOM_PARSER_MODE__;
  const globalParser = globalThis.__TURBO_DOM_PARSER__;
  try {
    setParserMode(null);
    globalThis.__TURBO_DOM_PARSER_MODE__ = 'wasm';
    const parser = { parse() {} };
    globalThis.__TURBO_DOM_PARSER__ = parser;
    assert.equal(getParserSelection().mode, 'wasm');
    assert.equal(getParserSelection().binding, parser);
    const revision = getParserSelection().revision;
    setParserMode('native');
    assert.equal(getParserSelection().mode, 'native');
    assert.ok(getParserSelection().revision > revision);
    setParser(parser);
    assert.equal(getParserSelection().binding, parser);
  } finally {
    setParserMode(null);
    globalThis.__TURBO_DOM_PARSER_MODE__ = globalMode;
    globalThis.__TURBO_DOM_PARSER__ = globalParser;
  }
});
test('memoizes success and absence; reset/revision invalidates without disposal', () => {
  const calls = [];
  const resolver = createCssArtifactResolver({ require: loader(raw(), calls), platform: 'darwin', arch: 'arm64' });
  const first = resolver.resolve(selection('native'));
  assert.equal(resolver.resolve(selection('native')), first);
  assert.equal(calls.length, 1);
  resolver.resolve(selection('native', null, 1));
  assert.equal(calls.length, 2);
  resolver.reset();
  resolver.resolve(selection('native', null, 1));
  assert.equal(calls.length, 3);
  const absent = createCssArtifactResolver({ require: null });
  assert.equal(absent.resolve(selection('auto')), null);
  assert.equal(absent.resolve(selection('auto')), null);
});
test('default Linux detection and unsupported native host remain independent', () => {
  const calls = [];
  const linux = createCssArtifactResolver({
    require: loader(raw(), calls),
    platform: 'linux',
    arch: 'x64',
    isMusl: () => false,
  });
  linux.resolve(selection('native'));
  assert.equal(calls[0], '../../native-css/turbo-dom-css.linux-x64-gnu.node');
  const unsupported = createCssArtifactResolver({ require: loader(raw(), []), platform: 'unknown' });
  assert.equal(unsupported.resolve(selection('native')), null);
  const detected = createCssArtifactResolver({ require: loader(raw(), []), platform: 'linux', arch: 'x64' });
  assert.ok(detected.resolve(selection('native')));
});
test('numeric artifact descriptors preserve their version contract', () => {
  assert.deepEqual(normalizeCssArtifact({ cssAbiVersion: 1, cssBackendVersion: '0.8.7', CompiledStylesV1 }), {
    cssAbiVersion: 1,
    CompiledStylesV1,
  });
});
test('bare host imports without Node or WebAssembly and accepts preinitialized web namespace', async () => {
  const { spawnSync } = await import('node:child_process');
  const script = `
    import vm from 'node:vm';
    import fs from 'node:fs';
    import assert from 'node:assert/strict';
    const context = vm.createContext({ WebAssembly: undefined });
    const backend = new vm.SourceTextModule(fs.readFileSync('src/runtime/css-backend.mjs','utf8'), {context});
    const parser = new vm.SourceTextModule(fs.readFileSync('src/runtime/parser.mjs','utf8'), {context, importModuleDynamically() { throw new Error('no Node modules'); }});
    const artifact = new vm.SourceTextModule(fs.readFileSync('src/runtime/css-artifact.mjs','utf8'), {
      context, importModuleDynamically() { throw new Error('no Node modules'); }
    });
    await artifact.link(spec => spec === './parser.mjs' ? parser : backend);
    await artifact.evaluate();
    const api = artifact.namespace;
    assert.equal(api.createCssArtifactResolver().resolve({mode:'auto',binding:null,revision:0}), null);
    const binding = {cssAbiVersion:()=>1,cssBackendVersion:()=> '0.8.7',CompiledStylesV1:class{}};
    assert.equal(api.createCssArtifactResolver({preinitializedWasm:binding}).resolve({mode:'wasm',binding:null,revision:0}).CompiledStylesV1,binding.CompiledStylesV1);
  `;
  const result = spawnSync(process.execPath, ['--experimental-vm-modules', '--input-type=module', '-e', script], {
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
});
test('default Node require capability resolves an absent optional artifact lazily', () => {
  const resolver = createCssArtifactResolver({ platform: 'unknown' });
  assert.equal(resolver.resolve(selection('native')), null);
});

test('native musl detection exactly preserves generated parser report/ldd fallback', () => {
  const never = () => {
    throw Error('modern report must not load shell/fs');
  };
  assert.equal(
    detectNativeMusl(never, { report: { getReport: () => ({ header: { glibcVersionRuntime: '2.31' } }) } }),
    false,
  );
  assert.equal(detectNativeMusl(never, { report: { getReport: () => ({ header: {} }) } }), true);
  for (const [text, expected] of [
    ['musl libc', true],
    ['GNU libc', false],
  ]) {
    const calls = [];
    const require = (name) =>
      name === 'child_process'
        ? {
            execSync(command) {
              calls.push(command);
              return Buffer.from(' /bin/ldd\n');
            },
          }
        : {
            readFileSync(path, encoding) {
              calls.push([path, encoding]);
              return text;
            },
          };
    assert.equal(detectNativeMusl(require, {}), expected);
    assert.deepEqual(calls, ['which ldd', ['/bin/ldd', 'utf8']]);
  }
  assert.equal(detectNativeMusl(never, { report: {} }), true);
  const error = {};
  assert.throws(
    () =>
      detectNativeMusl(never, {
        report: {
          getReport() {
            throw error;
          },
        },
      }),
    (e) => e === error,
  );
});
test('artifact normalization reads class capability only once', () => {
  let reads = 0;
  const artifact = {
    ...raw(),
    get CompiledStylesV1() {
      reads++;
      return CompiledStylesV1;
    },
  };
  assert.equal(normalizeCssArtifact(artifact).CompiledStylesV1, CompiledStylesV1);
  assert.equal(reads, 1);
});
test('artifact optional scene capability is retained and validated without peer fallback', () => {
  const measureSceneV1 = () => 'scene';
  let reads = 0;
  const artifact = {
    ...raw(),
    get measureSceneV1() {
      reads++;
      return measureSceneV1;
    },
  };
  assert.equal(normalizeCssArtifact(artifact).measureSceneV1, measureSceneV1);
  assert.equal(reads, 1);
  for (const measureSceneV1 of [null, 1, {}])
    assert.throws(() => normalizeCssArtifact({ ...raw(), measureSceneV1 }), TypeError);
  const error = {};
  assert.throws(
    () =>
      normalizeCssArtifact({
        ...raw(),
        get measureSceneV1() {
          throw error;
        },
      }),
    (e) => e === error,
  );
  const resolver = createCssArtifactResolver({
    require: loader({ ...raw(), measureSceneV1 }, []),
    hasWebAssembly: () => true,
  });
  assert.equal(resolver.resolve(selection('wasm')).measureSceneV1, measureSceneV1);
  const cssOnly = createCssArtifactResolver({ require: loader(raw(), []) }).resolve(selection('native'));
  assert.equal(cssOnly.measureSceneV1, undefined);
});

// The runtime in a bare V8 (no Node builtins, no web-platform globals), simulated in a child
// Node process: a preload removes `btoa` / `atob` / `Buffer` / `URL` / `Blob` from the global
// object and makes `import('node:module')` fail inside parser.mjs (the guarded top-level
// await), so the module-load-time fallbacks in window.mjs / parser.mjs are the ones that run.
// The oracle for every fallback answer is the real platform function in THIS process.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const runtime = new URL('../src/runtime/index.mjs', import.meta.url).href;

// `remove`: globals deleted before the runtime loads; `inject`: a parser binding set on
// globalThis.__TURBO_DOM_PARSER__ (the embedder seam) before node:module goes away.
function preload({ remove, inject }) {
  const src = `
    import { registerHooks, createRequire } from 'node:module';
    ${inject ? `globalThis.__TURBO_DOM_PARSER__ = createRequire(${JSON.stringify(root + 'index.js')})(${JSON.stringify(root + 'index.js')});` : ''}
    registerHooks({
      resolve(specifier, context, next) {
        if (specifier === 'node:module' && context.parentURL?.endsWith('/src/runtime/parser.mjs')) {
          throw new Error('bare V8: no node:module');
        }
        return next(specifier, context);
      },
    });
    // Node defines fetch / FormData / … as lazy getters that load undici (which needs
    // Buffer): resolve them all first, so only the runtime sees the missing globals.
    for (const name of Object.getOwnPropertyNames(globalThis)) { try { void globalThis[name]; } catch {} }
    for (const name of ${JSON.stringify(remove)}) delete globalThis[name];
  `;
  return `data:text/javascript,${encodeURIComponent(src)}`;
}

function run(options, body) {
  const script = `
    const { createEnvironment } = await import(${JSON.stringify(runtime)});
    const out = await (async () => { ${body} })();
    process.stdout.write(JSON.stringify(out));
  `;
  const r = spawnSync(process.execPath, ['--import', preload(options), '--input-type=module', '-e', script], {
    encoding: 'utf8',
  });
  assert.equal(r.status, 0, r.stderr);
  return JSON.parse(r.stdout);
}

// Latin-1 strings covering every padding case (length mod 3 = 0, 1, 2), the empty string and
// bytes above 0x7f.
const SAMPLES = ['', 'M', 'Ma', 'Man', 'hello world', 'ÿ\u0080\u0000a', 'turbo-dom!?'];
const B64S = SAMPLES.map((s) => btoa(s));

const CODEC = `
  const { window } = createEnvironment('<p>x</p>');
  const samples = ${JSON.stringify(SAMPLES)};
  return {
    has: [typeof globalThis.btoa, typeof globalThis.Buffer, typeof globalThis.URL, typeof globalThis.Blob],
    encoded: samples.map((s) => window.btoa(s)),
    decoded: ${JSON.stringify(B64S)}.map((s) => window.atob(s)),
    // atob drops characters outside the base64 alphabet (whitespace, line breaks)
    spaced: window.atob(' TW\\nFu '),
    coerced: [window.btoa(12), window.atob(window.btoa(null))],
  };
`;

test('pure-JS btoa/atob match the platform when neither btoa/atob nor Buffer exist', () => {
  const got = run({ remove: ['btoa', 'atob', 'Buffer', 'URL', 'Blob'], inject: true }, CODEC);
  assert.deepEqual(got, {
    has: ['undefined', 'undefined', 'undefined', 'undefined'],
    encoded: B64S,
    decoded: SAMPLES,
    spaced: 'Man',
    coerced: ['MTI=', 'null'],
  });
});

test('Buffer-backed btoa/atob match the platform when only btoa/atob are missing', () => {
  const got = run({ remove: ['btoa', 'atob'], inject: true }, CODEC);
  assert.deepEqual(got, {
    has: ['undefined', 'function', 'function', 'function'],
    encoded: B64S,
    decoded: SAMPLES,
    spaced: 'Man',
    coerced: ['MTI=', 'null'],
  });
});

test('minimal URL and Blob fallbacks round-trip the fields the runtime uses', () => {
  const got = run({ remove: ['btoa', 'atob', 'Buffer', 'URL', 'Blob'], inject: true }, `
    const { window } = createEnvironment('<p>x</p>');
    const { URL, Blob, File } = window;
    const url = new URL('page.html', 'https://example.test/dir/');
    const bare = new URL('https://example.test/x');
    const blob = new Blob(['a', 'b'], { type: 'text/plain' });
    const empty = new Blob();
    const file = new File(['z'], 'z.txt', { type: 'text/x' });
    const objectUrls = [URL.createObjectURL(blob), URL.createObjectURL(blob)];
    return {
      url: [url.href, String(url), url.origin, bare.href],
      blob: [blob.type, blob.size, await blob.text(), (await blob.arrayBuffer()).byteLength],
      empty: [empty.type, empty.size, await empty.text()],
      slice: [blob.slice() instanceof Blob, await blob.slice().text()],
      file: [file instanceof Blob, file.name, file.type],
      objectUrls: objectUrls.map((u) => u.replace(/[0-9]+$/, 'N')),
      distinct: objectUrls[0] !== objectUrls[1],
      revoke: URL.revokeObjectURL(objectUrls[0]) === undefined,
    };
  `);
  assert.deepEqual(got, {
    url: [
      'https://example.test/dir/page.html',
      'https://example.test/dir/page.html',
      '',
      'https://example.test/x',
    ],
    blob: ['text/plain', 0, 'ab', 0],
    empty: ['', 0, ''],
    slice: [true, ''],
    file: [true, 'z.txt', 'text/x'],
    objectUrls: ['blob:turbo-dom/N', 'blob:turbo-dom/N'],
    distinct: true,
    revoke: true,
  });
});

test('without node:module and without an injected parser the first parse throws the setParser hint', () => {
  const got = run({ remove: [], inject: false }, `
    try { createEnvironment('<p>x</p>'); return 'parsed'; }
    catch (e) { return [e.constructor.name, e.message]; }
  `);
  assert.deepEqual(got, [
    'Error',
    'turbo-dom: no parser available — node:module is missing (bare V8) and nothing was injected. ' +
      'Call setParser(binding) (or set globalThis.__TURBO_DOM_PARSER__) before the first parse.',
  ]);
});

test('without TextDecoder, fetch, URLSearchParams and performance the runtime still parses, times and fails XHR cleanly', () => {
  const got = run({ remove: ['TextDecoder', 'fetch', 'URLSearchParams', 'performance'], inject: true }, `
    const { window, document } = createEnvironment('<p title="é">é\u{1F600}ü &amp; 中</p>');
    const p = document.querySelector('p');
    const xhr = new window.XMLHttpRequest();
    const events = [];
    for (const type of ['readystatechange', 'load', 'error', 'loadend']) xhr.addEventListener(type, () => events.push([type, xhr.readyState, xhr.status]));
    const dropped = () => events.push(['dropped']);
    xhr.addEventListener('error', dropped);
    xhr.removeEventListener('error', dropped);
    xhr.removeEventListener('abort', dropped); // no listener of that type: a no-op
    xhr.open('GET', 'https://example.test/');
    xhr.send();
    const before = Date.now();
    const t = window.performance.now();
    return {
      text: p.textContent,
      title: p.getAttribute('title'),
      fetch: typeof window.fetch,
      search: typeof window.URLSearchParams,
      events,
      // the bare-V8 clock is Date.now()
      clock: t >= before && t <= Date.now(),
    };
  `);
  assert.deepEqual(got, {
    text: 'é\u{1F600}ü & 中',
    title: 'é',
    fetch: 'undefined',
    search: 'undefined',
    events: [['readystatechange', 1, 0], ['error', 4, 0], ['loadend', 4, 0]],
    clock: true,
  });
});

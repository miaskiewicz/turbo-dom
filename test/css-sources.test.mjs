import { ensureCssState } from '../src/runtime/css-invalidation.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import { createEnvironment } from '../src/runtime/index.mjs';
import { CSSStyleSheet } from '../src/runtime/cssom.mjs';
import { captureCssSources } from '../src/runtime/css-sources.mjs';
function scene(html = '<style>.a{color:red}</style><div></div>') {
  const env = createEnvironment(html);
  return { ...env, style: env.document.querySelector('style') };
}
test('physical text and injected rule text captured once, without CSS parsing or textContent suffix', () => {
  const { document, style } = scene('<style>@media (width > 1px){.a{--x:";{}"}}</style>');
  const physical = style.textContent;
  const injected = '.a{--raw:"\\";";color:var(--x)}';
  style.sheet.insertRule(injected);
  assert.notEqual(style.textContent, physical);
  const capture = captureCssSources(document);
  assert.equal(capture.status, 'captured');
  assert.deepEqual(
    capture.sources.map((item) => item.text),
    [physical, injected],
  );
  assert.equal(capture.uses.length, 2);
  assert.equal(capture.uses[0].ownerId, style);
  assert.equal(capture.uses[0].implicitScopeRootId, style.parentNode);
  assert.equal(capture.uses[0].scopeId, document);
});
test('preserves raw UTF16 and independent media/type/disabled facts', () => {
  const { document, style } = scene('<style></style>');
  const raw = '--x:"\ud800\udfff\ud800";';
  style.appendChild(document.createTextNode(`.a{${raw}}`));
  style.setAttribute('media', 'not print');
  style.setAttribute('type', 'text/not-css');
  style.sheet.media = ['screen', '(width: 1px)'];
  style.sheet.disabled = true;
  const capture = captureCssSources(document);
  assert.equal(capture.sources[0].text, `.a{${raw}}`);
  assert.deepEqual(capture.uses[0].sheetMedia, ['screen', '(width: 1px)']);
  assert.equal(capture.uses[0].ownerMedia, 'not print');
  assert.equal(capture.uses[0].ownerType, 'text/not-css');
  assert.equal(capture.uses[0].disabled, true);
  assert.equal(capture.uses[0].ownerNamespace, 'http://www.w3.org/1999/xhtml');
});
test('adopted shared sheet source dedup retains root/use order and implicit scope owner', () => {
  const { document, style } = scene();
  const shadow = document.querySelector('div').attachShadow({ mode: 'closed' });
  shadow.innerHTML = '<style>@scope {.a{color:green}}</style>';
  const sheet = new CSSStyleSheet();
  sheet.insertRule('.adopted{color:blue}');
  document.adoptedStyleSheets = [sheet];
  shadow.adoptedStyleSheets = [sheet];
  const capture = captureCssSources(document);
  assert.equal(capture.status, 'captured');
  assert.equal(capture.sources.filter((item) => item.text === '.adopted{color:blue}').length, 1);
  const adopted = capture.uses.filter((item) => item.adopted);
  assert.equal(adopted.length, 2);
  assert.equal(adopted[0].scopeId, shadow);
  assert.equal(adopted[1].scopeId, document);
  assert.equal(adopted[0].sourceId, adopted[1].sourceId);
  assert.equal(adopted[0].ownerId, null);
  assert.equal(capture.uses[0].ownerId, style);
});
test('CSSOM edits, media array edits, reset and exact source fingerprint have no stale reuse', () => {
  const { document, style, reset } = scene();
  style.sheet.insertRule('.a{color:blue}');
  const before = captureCssSources(document);
  style.sheet.cssRules[0].cssText = '.a{color:green}';
  const after = captureCssSources(document);
  assert.notDeepEqual(after.fingerprint, before.fingerprint);
  style.sheet.media.push('print');
  assert.notDeepEqual(captureCssSources(document).fingerprint, after.fingerprint);
  // Existing snapshots own their source strings/media arrays.
  assert.equal(after.sources[1].text, '.a{color:green}');
  assert.deepEqual(after.uses[0].sheetMedia, []);
  reset('<style>.new{color:black}</style>');
  assert.deepEqual(
    captureCssSources(document).sources.map((item) => item.text),
    ['.new{color:black}'],
  );
});
test('foreign documents refuse and foreign sheet getter errors retain original identity', () => {
  assert.equal(captureCssSources({}).status, 'unsupported');
  const { document } = scene();
  assert.equal(captureCssSources(new Proxy(document, {})).status, 'unsupported');
  const error = {};
  document.adoptedStyleSheets = [
    {
      media: [],
      disabled: false,
      ownerNode: null,
      get cssRules() {
        throw error;
      },
    },
  ];
  assert.throws(
    () => captureCssSources(document),
    (value) => value === error,
  );
  document.adoptedStyleSheets = [{ media: {}, disabled: false, ownerNode: null, cssRules: [] }];
  assert.equal(captureCssSources(document).status, 'unsupported');
});
test('uncertain descriptors and malformed foreign stores refuse instead of silently dropping sources', () => {
  const { document, style } = scene();
  Object.defineProperty(document, 'adoptedStyleSheets', {
    configurable: true,
    get() {
      throw Error('not invoked');
    },
  });
  assert.equal(captureCssSources(document).status, 'unsupported');
  Object.defineProperty(document, 'adoptedStyleSheets', { configurable: true, value: [null] });
  assert.equal(captureCssSources(document).status, 'unsupported');
});
test('malformed rules and media stay unsupported without string coercion', () => {
  const { document } = scene('<style></style>');
  for (const sheet of [
    { media: [], cssRules: {} },
    { media: [], cssRules: [null] },
    {
      media: [],
      cssRules: [
        {
          cssText: {
            toString() {
              throw Error('not coerced');
            },
          },
        },
      ],
    },
    { media: [1], cssRules: [] },
    { media: new Array(1), cssRules: [] },
  ]) {
    document.adoptedStyleSheets = [sheet];
    assert.equal(captureCssSources(document).status, 'unsupported');
  }
  document.adoptedStyleSheets = [
    { media: 'screen', cssRules: [{ cssText: '.x{}' }], disabled: false, ownerNode: null },
  ];
  assert.equal(captureCssSources(document).uses[0].sheetMedia, 'screen');
});
test('descriptor uncertainty, corrupted text and cycles refuse bounded capture', () => {
  const { document, style } = scene();
  const child = style.firstChild;
  child._data = 1;
  assert.equal(captureCssSources(document).status, 'unsupported');
  child._data = '.x{}';
  style.appendChild(document.createComment('ignored comment'));
  assert.equal(captureCssSources(document).sources[0].text, '.x{}');
  Object.defineProperty(style, '__sheet', {
    configurable: true,
    get() {
      throw Error('not invoked');
    },
  });
  assert.equal(captureCssSources(document).status, 'unsupported');
  Object.defineProperty(style, '__sheet', { configurable: true, writable: true, value: undefined });
  Object.defineProperty(style, '__shadow', {
    configurable: true,
    get() {
      throw Error('not invoked');
    },
  });
  assert.equal(captureCssSources(document).status, 'unsupported');
  Object.defineProperty(style, '__shadow', { configurable: true, writable: true, value: document });
  assert.equal(captureCssSources(document).status, 'unsupported');
  style.__shadow = undefined;
  style.__kids.push(style);
  assert.equal(captureCssSources(document).status, 'unsupported');
});
test('foreign sheet ownership remains captured uncached and explicit callbacks map identities', () => {
  const { document, style } = scene();
  const other = new CSSStyleSheet();
  other.insertRule('.foreign{}');
  style.__sheet = other;
  const nodes = new Map();
  const id = (node) => {
    if (!nodes.has(node)) nodes.set(node, nodes.size);
    return nodes.get(node);
  };
  const capture = captureCssSources(document, { nodeId: id, scopeId: id });
  assert.equal(capture.cacheable, false);
  assert.equal(capture.status, 'captured');
  assert.equal(capture.uses[0].ownerId, nodes.get(style));
});
test('exposes exact source-store tokens separately from scan-required document provenance', () => {
  const { document, style } = scene();
  ensureCssState(document);
  style.sheet.insertRule('.a{}');
  const before = captureCssSources(document);
  const store = before.provenance.stores[0];
  assert.equal(before.provenance.documentCertified, false);
  assert.equal(before.provenance.requiresSourceScan, true);
  assert.equal(store.sheet, style.sheet);
  assert.equal(store.snapshot.metadataTrusted, true);
  assert.equal(store.snapshot.rulesTrusted, true);
  assert.equal(store.snapshot.ownerStable, true);
  assert.equal(store.snapshot.mediaNeedsFingerprint, true);
  style.sheet.media.push('print');
  const after = captureCssSources(document);
  assert.equal(store.snapshot, after.provenance.stores[0].snapshot);
  assert.deepEqual(store.mediaFingerprint, []);
  assert.deepEqual(after.provenance.stores[0].mediaFingerprint, ['print']);
  document.adoptedStyleSheets = [{ media: [], disabled: false, ownerNode: null, cssRules: [] }];
  assert.equal(captureCssSources(document).provenance.stores[1].snapshot, null);
});

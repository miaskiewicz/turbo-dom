import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';
import { ensureCssState, cssStateSnapshot } from '../src/runtime/css-invalidation.mjs';

const environment = (html = '<div id=target class=old>old</div>') => createEnvironment(`<body>${html}</body>`);
const snapshot = (document) => cssStateSnapshot(document, document.querySelector('#target'));

function changed(document, mutate) {
  const before = snapshot(document);
  assert.notEqual(before, null);
  mutate();
  assert.notEqual(snapshot(document), before);
}

test('tracking remains absent until explicitly enabled; detached construction preserves connected token', () => {
  const { document } = environment();
  assert.equal(snapshot(document), null);
  const target = document.querySelector('#target');
  ensureCssState(document);
  const token = snapshot(document);
  const broadVersion = document.__version;
  for (let index = 0; index < 10; index++) {
    const container = document.createElement('div');
    const input = document.createElement('input');
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    container.setAttribute('class', 'next');
    container.textContent = 'detached';
    input.defaultChecked = true;
    input.defaultChecked = false;
    container.appendChild(input);
    container.appendChild(svg);
    container.removeAttribute('class');
    container.appendChild(document.createTextNode('payload'));
    assert.equal(container.isConnected, false);
    assert.equal(snapshot(document), token);
  }
  assert.ok(document.__version > broadVersion);
  assert.equal(target.className, 'old');
  assert.equal(ensureCssState(document), token);
});

test('connected scalar and structural changes invalidate without weakening broad versions or observers', () => {
  const { document, window } = environment();
  const target = document.querySelector('#target');
  const observer = new window.MutationObserver(() => {});
  observer.observe(document.body, {
    subtree: true,
    attributes: true,
    childList: true,
    characterData: true,
  });
  ensureCssState(document);
  const version = document.__version;
  changed(document, () => target.setAttribute('class', 'next'));
  changed(document, () => {
    target.firstChild.data = 'new';
  });
  changed(document, () => {
    target.textContent = 'replaced';
  });
  changed(document, () => {
    target.innerHTML = '<b>markup</b>';
  });
  const detached = document.createElement('span');
  detached.textContent = 'built';
  changed(document, () => target.appendChild(detached));
  changed(document, () => target.removeChild(detached));
  changed(document, () => target.style.setProperty('color', 'red'));
  assert.ok(document.__version > version);
  const records = observer.takeRecords();
  assert.ok(records.some((record) => record.type === 'attributes'));
  assert.ok(records.some((record) => record.type === 'childList'));
  assert.ok(records.some((record) => record.type === 'characterData'));
});

test('unaccounted broad changes cannot be erased by a later detached mutation', () => {
  const { document } = environment();
  ensureCssState(document);
  let token = snapshot(document);
  document.__version += 1;
  assert.notEqual(snapshot(document), token);
  token = snapshot(document);
  document.__version += 1;
  document.createElement('div').setAttribute('class', 'detached');
  assert.notEqual(snapshot(document), token);
});

test('dirty form state, focus, click cancellation and reset invalidate even without broad-version writes', () => {
  const { document } = environment(
    '<div id=target></div><form><input id=check type=checkbox><input id=text><select><option>A</option><option>B</option></select></form>',
  );
  ensureCssState(document);
  const check = document.querySelector('#check');
  const input = document.querySelector('#text');
  const options = document.querySelectorAll('option');
  let version = document.__version;
  changed(document, () => {
    check.checked = true;
  });
  assert.equal(document.__version, version);
  changed(document, () => {
    input.value = 'typed';
  });
  changed(document, () => {
    options[1].selected = true;
  });
  changed(document, () => input.focus());
  changed(document, () => input.blur());
  let duringClick;
  check.addEventListener('click', (event) => {
    duringClick = snapshot(document);
    event.preventDefault();
  });
  const beforeClick = snapshot(document);
  check.click();
  assert.notEqual(duringClick, beforeClick);
  assert.notEqual(snapshot(document), duringClick);
  changed(document, () => document.querySelector('form').reset());
  assert.equal(document.__version, version);
});

test('radio activation and reversal invalidate all affected checked-state reads', () => {
  const { document } = environment(
    '<div id=target></div><input id=a type=radio name=g checked><input id=b type=radio name=g>',
  );
  ensureCssState(document);
  const first = document.querySelector('#a');
  const second = document.querySelector('#b');
  let active;
  second.addEventListener('click', (event) => {
    active = snapshot(document);
    assert.equal(first.checked, false);
    assert.equal(second.checked, true);
    event.preventDefault();
  });
  const before = snapshot(document);
  second.click();
  assert.notEqual(active, before);
  assert.notEqual(snapshot(document), active);
  assert.equal(first.checked, true);
});

test('shadow composition and its connected descendants invalidate while detached scalar work stays isolated', () => {
  const { document } = environment();
  ensureCssState(document);
  const target = document.querySelector('#target');
  let root;
  changed(document, () => {
    root = target.attachShadow({ mode: 'open' });
  });
  changed(document, () => {
    root.innerHTML = '<style>:host{color:red}</style><span>shadow</span>';
  });
  const child = root.querySelector('span');
  changed(document, () => {
    child.textContent = 'changed';
  });
  assert.notEqual(cssStateSnapshot(document, child), null);
  const detached = document.createElement('div');
  const detachedRoot = detached.attachShadow({ mode: 'closed' });
  const before = snapshot(document);
  detachedRoot.innerHTML = '<span>outside</span>';
  assert.equal(snapshot(document), before);
});

test('document reset, stale nodes, raw unlinked buffer handles and cross-document adoption refuse stale proof', () => {
  const env = environment();
  const { document } = env;
  ensureCssState(document);
  const old = document.querySelector('#target');
  const before = snapshot(document);
  env.reset('<body><div id=target>replacement</div></body>');
  assert.notEqual(snapshot(document), before);
  assert.equal(cssStateSnapshot(document, old), null);
  const second = environment();
  ensureCssState(second.document);
  const priorFirst = snapshot(document);
  const priorSecond = snapshot(second.document);
  second.document.adoptNode(old);
  assert.notEqual(snapshot(document), priorFirst);
  assert.notEqual(snapshot(second.document), priorSecond);
  assert.equal(cssStateSnapshot(second.document, old), null);
  const plain = environment();
  ensureCssState(plain.document);
  const unlinked = plain.document.__nodeAt(3);
  assert.equal(cssStateSnapshot(plain.document, unlinked), null);
});

test('detached and inert-document targets never receive a connected-document token', () => {
  const { document } = environment('<div id=target></div><template><span>inert</span></template>');
  ensureCssState(document);
  assert.equal(cssStateSnapshot(document, document.createElement('span')), null);
  const content = document.querySelector('template').content;
  assert.equal(cssStateSnapshot(document, content.firstChild), null);
});

test('proof does not execute overridden connection/parent/host getters and cycles refuse reuse', () => {
  const { document } = environment();
  const target = document.querySelector('#target');
  ensureCssState(document);
  Object.defineProperty(target, 'isConnected', {
    get() {
      assert.fail('public isConnected was read');
    },
  });
  assert.notEqual(snapshot(document), null);
  const parent = target.parentNode;
  Object.defineProperty(target, 'parentNode', {
    configurable: true,
    get() {
      assert.fail('parent getter was read');
    },
  });
  assert.equal(cssStateSnapshot(document, target), null);
  Object.defineProperty(target, 'parentNode', {
    configurable: true,
    writable: true,
    value: target,
  });
  assert.equal(cssStateSnapshot(document, target), null);
  Object.defineProperty(target, 'parentNode', {
    configurable: true,
    writable: true,
    value: parent,
  });
  const root = target.attachShadow({ mode: 'closed' });
  Object.defineProperty(root, 'host', {
    configurable: true,
    get() {
      assert.fail('host getter was read');
    },
  });
  assert.equal(cssStateSnapshot(document, root), null);
});

test('untracked mutation adds no ownership walk or per-document tracking field', () => {
  const { document } = environment();
  const target = document.querySelector('#target');
  const parent = target.parentNode;
  const keys = Reflect.ownKeys(document);
  Object.defineProperty(target, 'parentNode', {
    configurable: true,
    get() {
      assert.fail('untracked mutation walked ownership');
    },
  });
  target.setAttribute('class', 'next');
  assert.equal(cssStateSnapshot(document, target), null);
  assert.deepEqual(Reflect.ownKeys(document), keys);
  Object.defineProperty(target, 'parentNode', {
    configurable: true,
    writable: true,
    value: parent,
  });
});

test('uncertain storage descriptors and exotic reflection refuse proof instead of guessing ownership', () => {
  const { document } = environment();
  const target = document.querySelector('#target');
  ensureCssState(document);
  const index = target.__idx;
  target.__idx = 'not an index';
  assert.equal(cssStateSnapshot(document, target), null);
  target.__idx = index;
  const parent = target.parentNode;
  const children = parent.__kids;
  parent.__kids = [];
  assert.equal(cssStateSnapshot(document, target), null);
  const exoticChildren = [];
  Object.defineProperty(exoticChildren, 0, {
    get() {
      assert.fail('child storage getter was read');
    },
  });
  parent.__kids = exoticChildren;
  assert.equal(cssStateSnapshot(document, target), null);
  parent.__kids = new Proxy([], {
    getOwnPropertyDescriptor() {
      throw new Error('exotic length');
    },
  });
  assert.equal(cssStateSnapshot(document, target), null);
  parent.__kids = children;
  const detached = document.createElement('div');
  Object.defineProperty(detached, '__isShadowRoot', {
    get() {
      assert.fail('shadow flag getter was read');
    },
  });
  assert.equal(cssStateSnapshot(document, detached), null);
  let traps = 0;
  const proxy = new Proxy(target, {
    getOwnPropertyDescriptor() {
      traps++;
      throw new Error('exotic descriptor');
    },
  });
  assert.equal(cssStateSnapshot(document, proxy), null);
  assert.ok(traps > 0);
  const first = document.createElement('div');
  const second = document.createElement('div');
  first.parentNode = second;
  second.parentNode = first;
  first.__kids = [second];
  second.__kids = [first];
  assert.equal(cssStateSnapshot(document, first), null);
});

test('known CSSOM methods invalidate; dirty detached state remains conservative', () => {
  const { document } = environment('<style>.old{color:red}</style><div id=target></div>');
  ensureCssState(document);
  const sheet = document.querySelector('style').sheet;
  changed(document, () => sheet.insertRule('#target{color:blue}'));
  changed(document, () => sheet.deleteRule(0));
  changed(document, () => sheet.replaceSync('#target{color:green}'));
  const version = document.__version;
  changed(document, () => {
    document.createElement('input').checked = true;
  });
  assert.equal(document.__version, version);
});

test('private activation cannot be disabled with caller fields and foreign owners add no new exceptions', () => {
  const { document } = environment();
  const ownKeys = Reflect.ownKeys(document);
  ensureCssState(document);
  assert.deepEqual(Reflect.ownKeys(document), ownKeys);
  Object.defineProperty(document, '__cssTracked', {
    value: false,
    writable: true,
  });
  const detached = document.createElement('input');
  changed(document, () => {
    detached.checked = true;
  });
  let proxyReads = 0;
  const proxy = new Proxy(document, {
    get() {
      proxyReads++;
      throw new Error('unexpected owner read');
    },
  });
  for (const owner of [{}, null, 17, proxy]) {
    detached.ownerDocument = owner;
    detached.checked = false;
    assert.equal(detached.checked, false);
  }
  assert.equal(proxyReads, 0);
});

test('fresh documents remain untracked after another document enabled tracking', () => {
  const first = environment();
  ensureCssState(first.document);
  const second = environment();
  const target = second.document.querySelector('#target');
  target.setAttribute('class', 'new');
  target.textContent = 'new';
  target.checked = true;
  assert.equal(cssStateSnapshot(second.document, target), null);
  assert.notEqual(snapshot(first.document), null);
});

test('private activation works without requiring new public fields on sealed documents', () => {
  const { document } = environment();
  const target = document.querySelector('#target');
  Object.seal(document);
  ensureCssState(document);
  const before = cssStateSnapshot(document, target);
  assert.notEqual(before, null);
  target.setAttribute('class', 'changed');
  assert.notEqual(cssStateSnapshot(document, target), before);
});

test('later WeakMap prototype changes cannot forge or disable a genuine checkpoint', () => {
  const { document } = environment();
  const target = document.querySelector('#target');
  ensureCssState(document);
  const before = cssStateSnapshot(document, target);
  const original = { get: WeakMap.prototype.get, has: WeakMap.prototype.has, set: WeakMap.prototype.set };
  try {
    WeakMap.prototype.get = () => null;
    WeakMap.prototype.has = () => false;
    WeakMap.prototype.set = () => {
      throw new Error('unexpected dynamic registry method');
    };
    assert.equal(ensureCssState(document), before);
    target.setAttribute('class', 'new');
    assert.notEqual(cssStateSnapshot(document, target), before);
  } finally {
    Object.assign(WeakMap.prototype, original);
  }
});

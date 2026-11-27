import assert from 'node:assert/strict';
import test from 'node:test';
import { installGlobals } from '../src/environment/install.mjs';

test('installed location and history share the current URL, including when location is used first', async () => {
  const global = {};
  const installed = installGlobals(global, { url: 'http://localhost/' });
  try {
    const { location, document } = global;
    const events = [];
    global.window.addEventListener('hashchange', e => events.push([e.oldURL, e.newURL]));
    location.hash = '#a';
    assert.equal(location.href, 'http://localhost/#a');
    location.hash = 'a';
    assert.equal(global.history.length, 2);
    global.history.replaceState(null, '', '/?keep=1');
    assert.equal(location.search, '?keep=1');
    global.history.pushState({ page: 1 }, '', '/x?y=1#z');
    assert.deepEqual([location.pathname, location.search, location.hash], ['/x', '?y=1', '#z']);
    assert.equal(document.URL, 'http://localhost/x?y=1#z');
    assert.deepEqual(events, []);
    await new Promise(resolve => setTimeout(resolve, 5));
    assert.deepEqual(events, [['http://localhost/', 'http://localhost/#a']]);
  } finally { installed.teardown(); }
});

test('URL component writes and traversals use the current entry; invalid history URLs are atomic', async () => {
  const global = {};
  const installed = installGlobals(global, { url: 'https://host.test/base/start' });
  const { history, location } = global;
  const events = [];
  global.onhashchange = event => events.push(event.newURL);
  try {
    location.pathname = '/space here/';
    location.search = 'x=1';
    location.href = '#first#x';
    location.replace('#first#y');
    location.assign('#');
    location.hash = '';
    assert.equal(history.length, 6);
    assert.equal(location.href, 'https://host.test/space%20here/?x=1');
    history.replaceState(1, '');
    history.pushState(2, '', '../next?q=%23#hash');
    history.pushState(3, '', '');
    assert.equal(history.state, 3);
    assert.equal(location.href, 'https://host.test/next?q=%23#hash');
    assert.throws(() => history.replaceState(99, '', 'https://other.test/x'), { name: 'SecurityError' });
    assert.throws(() => history.pushState(99, '', 'http://['), { name: 'SecurityError' });
    assert.equal(history.length, 8);
    assert.equal(history.state, 3);
    history.go(-2);
    assert.equal(history.state, 1);
    assert.equal(location.search, '?x=1');
    history.forward();
    assert.equal(location.hash, '#hash');
    history.back();
    history.pushState(4, '', 'new');
    history.forward();
    assert.equal(history.state, 4);
    await new Promise(resolve => setTimeout(resolve, 5));
    assert.deepEqual(events.slice(0, 4), [
      'https://host.test/space%20here/?x=1#first#x',
      'https://host.test/space%20here/?x=1#first#y',
      'https://host.test/space%20here/?x=1#',
      'https://host.test/space%20here/?x=1',
    ]);
  } finally { installed.teardown(); }
});

test('history rewrite rules also apply to opaque documents and file URLs', () => {
  for (const url of ['about:blank', 'file:///tmp/doc']) {
    const target = {};
    const { teardown } = installGlobals(target, { url });
    try {
      target.history.replaceState(null, '', '#frag');
      assert.equal(target.location.href, url + '#frag');
      assert.throws(() => target.history.pushState(null, '', url + '/elsewhere'), { name: 'SecurityError' });
    } finally { teardown(); }
  }
});

test('disallowed history rewrite keeps its error name on hosts without DOMException', () => {
  const global = {};
  const { teardown } = installGlobals(global);
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'DOMException');
  try {
    Object.defineProperty(globalThis, 'DOMException', { configurable: true, value: undefined });
    assert.throws(() => global.history.pushState(null, '', 'https://other.test/'), { name: 'SecurityError' });
  } finally {
    Object.defineProperty(globalThis, 'DOMException', descriptor);
    teardown();
  }
});

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const HTML = `<!doctype html><html><head><title>t</title></head><body>
  <div id="app" class="card big" data-role="main">
    <h1>Title</h1>
    <ul class="list"><li>a</li><li class="sel">b</li><li>c</li></ul>
    <input type="text" value="hi">
  </div>
</body></html>`;

const fresh = () => createEnvironment(HTML);

// ---------------------------------------------------------- Layer 2 ----
test('identity memoization: same node === across accesses', () => {
  const { document } = fresh();
  const a = document.getElementById('app');
  const b = document.querySelector('#app');
  assert.equal(a, b);
  assert.equal(document.getElementById('app'), a);
});

test('WeakMap keying survives mutations around the node', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  const wm = new WeakMap();
  wm.set(app, 'tag');
  app.appendChild(document.createElement('span'));
  app.setAttribute('x', '1');
  assert.equal(wm.get(document.getElementById('app')), 'tag');
});

test('live childNodes / children reflect mutation immediately', () => {
  const { document } = fresh();
  const ul = document.querySelector('ul');
  const kids = ul.children;
  assert.equal(kids.length, 3);
  ul.appendChild(document.createElement('li'));
  assert.equal(kids.length, 4); // same live collection sees the new child
  ul.removeChild(ul.firstElementChild);
  assert.equal(kids.length, 3);
});

test('live getElementsByClassName updates on class change', () => {
  const { document } = fresh();
  const sel = document.getElementsByClassName('sel');
  assert.equal(sel.length, 1);
  document.querySelector('h1').classList.add('sel');
  assert.equal(sel.length, 2);
});

test('COW: reading buffer-backed then mutating is consistent', () => {
  const { document } = fresh();
  const li = document.querySelectorAll('li')[1];
  assert.equal(li.textContent, 'b');
  li.textContent = 'B!';
  assert.equal(document.querySelectorAll('li')[1].textContent, 'B!');
});

test('textContent get/set', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  assert.ok(app.textContent.includes('Title'));
  app.textContent = 'gone';
  assert.equal(app.textContent, 'gone');
  assert.equal(app.children.length, 0);
});

test('innerHTML get + set (reparse)', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  app.innerHTML = '<p class="x">hi</p><p>yo</p>';
  assert.equal(app.children.length, 2);
  assert.equal(app.querySelector('.x').textContent, 'hi');
  assert.equal(app.innerHTML, '<p class="x">hi</p><p>yo</p>');
});

test('innerHTML get on unmutated buffer-backed subtree (lazy __attrs)', () => {
  const { document } = fresh();
  const app = document.getElementById('app'); // never mutated → children keep lazy __attrs
  const html = app.innerHTML;                  // serializes buffer-backed els with __attrs undefined
  assert.ok(html.includes('<li class="sel">b</li>'));
  assert.ok(html.includes('<input type="text" value="hi">'));
});

test('insertBefore / replaceChild / cloneNode(deep)', () => {
  const { document } = fresh();
  const ul = document.querySelector('ul');
  const first = ul.firstElementChild;
  const n = document.createElement('li'); n.textContent = 'z';
  ul.insertBefore(n, first);
  assert.equal(ul.firstElementChild.textContent, 'z');
  const clone = ul.cloneNode(true);
  assert.equal(clone.children.length, ul.children.length);
  assert.notEqual(clone, ul);
});

test('classList + attribute reflection', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  assert.ok(app.classList.contains('card'));
  app.classList.toggle('card');
  assert.ok(!app.classList.contains('card'));
  app.id = 'changed';
  assert.equal(app.getAttribute('id'), 'changed');
});

test('inline style CSSOM (honest, inline-only)', () => {
  const { document } = fresh();
  const app = document.getElementById('app');
  app.style.color = 'red';
  assert.equal(app.style.color, 'red');
  assert.ok(app.getAttribute('style').includes('color'));
});

// ---------------------------------------------------------- Events ----
test('event capture → target → bubble order', () => {
  const { document, window } = fresh();
  const app = document.getElementById('app');
  const h1 = document.querySelector('h1');
  const order = [];
  document.body.addEventListener('x', () => order.push('body-capture'), true);
  app.addEventListener('x', () => order.push('app-capture'), true);
  h1.addEventListener('x', () => order.push('target'));
  app.addEventListener('x', () => order.push('app-bubble'));
  document.body.addEventListener('x', () => order.push('body-bubble'));
  h1.dispatchEvent(new window.Event('x', { bubbles: true }));
  assert.deepEqual(order, ['body-capture', 'app-capture', 'target', 'app-bubble', 'body-bubble']);
});

test('stopPropagation / stopImmediatePropagation / preventDefault', () => {
  const { document, window } = fresh();
  const app = document.getElementById('app');
  const h1 = document.querySelector('h1');
  let bubbledToApp = false;
  app.addEventListener('y', () => { bubbledToApp = true; });
  h1.addEventListener('y', (e) => e.stopPropagation());
  h1.dispatchEvent(new window.Event('y', { bubbles: true }));
  assert.equal(bubbledToApp, false);

  let count = 0;
  h1.addEventListener('z', (e) => { count++; e.stopImmediatePropagation(); });
  h1.addEventListener('z', () => { count++; });
  h1.dispatchEvent(new window.Event('z'));
  assert.equal(count, 1);

  const e = new window.Event('w', { cancelable: true });
  h1.addEventListener('w', (ev) => ev.preventDefault());
  const ret = h1.dispatchEvent(e);
  assert.equal(ret, false);
  assert.equal(e.defaultPrevented, true);
});

test('composedPath + once', () => {
  const { document, window } = fresh();
  const h1 = document.querySelector('h1');
  let path;
  h1.addEventListener('p', (e) => { path = e.composedPath(); });
  h1.dispatchEvent(new window.Event('p', { bubbles: true }));
  assert.equal(path[0], h1);
  assert.ok(path.includes(document.getElementById('app')));

  let n = 0;
  h1.addEventListener('o', () => n++, { once: true });
  h1.dispatchEvent(new window.Event('o'));
  h1.dispatchEvent(new window.Event('o'));
  assert.equal(n, 1);
});

// -------------------------------------------------------- Selectors ----
test('selector engine: combinators, attr, :not, :first-child', () => {
  const { document } = fresh();
  assert.equal(document.querySelectorAll('ul > li').length, 3);
  assert.equal(document.querySelector('li.sel').textContent, 'b');
  assert.equal(document.querySelectorAll('[data-role="main"]').length, 1);
  assert.equal(document.querySelector('li:first-child').textContent, 'a');
  assert.equal(document.querySelectorAll('li:not(.sel)').length, 2);
  assert.equal(document.querySelector('li + li.sel').textContent, 'b');
  assert.ok(document.getElementById('app').matches('div.card[data-role]'));
  assert.equal(document.querySelector('h1').closest('#app').id, 'app');
});

// --------------------------------------------------- Layer 3 window ----
test('lazy window: nothing materialized until touched', () => {
  const env = fresh();
  assert.deepEqual(env.touched(), []);
  void env.document; // eager, not lazy
  assert.deepEqual(env.touched(), []);
  env.window.matchMedia('(min-width: 1px)');
  void env.window.localStorage;
  assert.deepEqual(env.touched().sort(), ['localStorage', 'matchMedia']);
});

test('window self-references + storage isolation', () => {
  const env = fresh();
  assert.equal(env.window.window, env.window);
  assert.equal(env.window.self, env.window);
  env.window.localStorage.setItem('a', '1');
  assert.equal(env.window.localStorage.getItem('a'), '1');
  assert.equal(env.window.sessionStorage.getItem('a'), null);
});

test('history co-materializes with location and shares it', () => {
  const env = fresh();
  env.window.history.pushState({ n: 1 }, '', '/next');
  assert.equal(env.window.history.state.n, 1);
  assert.ok(env.touched().includes('location'));
});

// ---------------------------------------------------- Layer 4 honest ----
test('honest stubs: no plausible lies', () => {
  const env = fresh();
  const app = env.document.getElementById('app');
  assert.deepEqual(app.getBoundingClientRect().width, 0);
  assert.equal(env.window.getComputedStyle(app).color, ''); // honest empty, not "rgb(0,0,0)"
  assert.equal(env.window.matchMedia('(x)').matches, false);
});

// ----------------------------------------------------- Layer 5 reset ----
test('reset drops overlay + cache, keeps working', () => {
  const env = fresh();
  const app1 = env.document.getElementById('app');
  app1.textContent = 'mutated';
  env.window.localStorage.setItem('k', 'v');
  env.reset();
  // buffer reused → pristine structure back, overlay gone
  const app2 = env.document.getElementById('app');
  assert.ok(app2.textContent.includes('Title'));
  assert.notEqual(app1, app2);             // cache cleared → fresh handles
  assert.deepEqual(env.touched(), []);     // globals dropped
});

test('reset with new html', () => {
  const env = fresh();
  env.reset('<!doctype html><body><main>new page</main></body>');
  assert.equal(env.document.querySelector('main').textContent, 'new page');
  assert.equal(env.document.getElementById('app'), null);
});

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const fresh = () => createEnvironment('<!doctype html><html><head><title>T</title></head><body><div id=root></div></body></html>');

test('DOMParser parses text/html into a Document', () => {
  const { window } = fresh();
  const doc = new window.DOMParser().parseFromString('<!doctype html><body><p class=x>hi</p></body>', 'text/html');
  assert.equal(doc.nodeType, 9);
  assert.equal(doc.querySelector('.x').textContent, 'hi');
});

test('XMLSerializer round-trips an element', () => {
  const { document, window } = fresh();
  const el = document.createElement('div'); el.setAttribute('id', 'a'); el.innerHTML = '<span>hi</span>';
  assert.equal(new window.XMLSerializer().serializeToString(el), '<div id="a"><span>hi</span></div>');
});

test('createTreeWalker walks elements', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  root.innerHTML = '<a></a><b><i></i></b>';
  const tw = document.createTreeWalker(root, 1 /* elements */);
  const names = [];
  let n; while ((n = tw.nextNode())) names.push(n.localName);
  assert.deepEqual(names, ['a', 'b', 'i']);
});

test('Text splitText + wholeText + CharacterData edits', () => {
  const { document } = fresh();
  const t = document.createTextNode('hello world');
  document.getElementById('root').appendChild(t);
  const w = t.splitText(5);
  assert.equal(t.data, 'hello');
  assert.equal(w.data, ' world');
  assert.equal(t.wholeText, 'hello world');
  t.appendData('!');
  assert.equal(t.data, 'hello!');
});

test('Element geometry is honest zero + isConnected + getRootNode', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  assert.equal(root.offsetWidth, 0);
  assert.equal(root.clientHeight, 0);
  assert.equal(root.scrollTop, 0);
  assert.equal(root.isConnected, true);
  const detached = document.createElement('div');
  assert.equal(detached.isConnected, false);
  assert.equal(root.getRootNode(), document);
});

test('insertAdjacentElement / insertAdjacentText', () => {
  const { document } = fresh();
  const root = document.getElementById('root');
  const a = document.createElement('a'); root.appendChild(a);
  const b = document.createElement('b'); a.insertAdjacentElement('afterend', b);
  assert.equal(a.nextElementSibling, b);
  a.insertAdjacentText('beforebegin', 'x');
  assert.equal(root.firstChild.data, 'x');
});

test('reflected props: tabIndex, title, hidden, dir, contentEditable, NS attrs', () => {
  const { document } = fresh();
  const el = document.createElement('div');
  el.tabIndex = 3; assert.equal(el.getAttribute('tabindex'), '3');
  el.title = 'tip'; assert.equal(el.title, 'tip');
  el.hidden = true; assert.equal(el.hasAttribute('hidden'), true);
  el.contentEditable = 'true'; assert.equal(el.isContentEditable, true);
  el.setAttributeNS('http://x', 'data-y', '1'); assert.equal(el.getAttributeNS('http://x', 'data-y'), '1');
});

test('document.title reflects <title>, getElementsByName, normalize', () => {
  const { document } = fresh();
  assert.equal(document.title, 'T');
  document.title = 'New';
  assert.equal(document.querySelector('title').textContent, 'New');

  const root = document.getElementById('root');
  const i1 = document.createElement('input'); i1.setAttribute('name', 'q'); root.appendChild(i1);
  assert.equal(document.getElementsByName('q').length, 1);

  root.appendChild(document.createTextNode('a'));
  root.appendChild(document.createTextNode('b'));
  root.normalize();
  const texts = Array.from(root.childNodes).filter((n) => n.nodeType === 3);
  assert.equal(texts[texts.length - 1].data, 'ab');
});

test('window web-platform globals exist and work', () => {
  const { window } = fresh();
  assert.equal(typeof window.fetch, 'function');
  assert.equal(typeof window.Headers, 'function');
  assert.equal(typeof window.crypto.getRandomValues, 'function');
  assert.equal(window.btoa('hi'), Buffer.from('hi').toString('base64'));
  assert.equal(window.atob(window.btoa('hi')), 'hi');
  assert.equal(window.CSS.supports('color', 'red'), true);
  assert.equal(typeof window.requestIdleCallback, 'function');
  assert.equal(typeof window.XMLHttpRequest, 'function');
  assert.equal(window.screen.width, 1024);
  assert.equal(typeof window.navigator.clipboard.writeText, 'function');
  assert.equal(window.navigator.cookieEnabled, true);
  const img = new window.Image(10, 20);
  assert.equal(img.localName, 'img');
  assert.equal(img.getAttribute('width'), '10');
});

test('XMLHttpRequest constructs + has the expected shape', () => {
  const { window } = fresh();
  const xhr = new window.XMLHttpRequest();
  assert.equal(xhr.readyState, 0);
  let opened = false;
  xhr.addEventListener('readystatechange', () => { opened = true; });
  xhr.open('GET', 'http://localhost/x');
  assert.equal(xhr.readyState, 1);
  assert.equal(opened, true);
});

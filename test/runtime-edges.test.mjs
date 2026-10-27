// Runtime edges the rest of the suite does not reach, each asserted against jsdom (the
// differential oracle) where jsdom implements it, else against the spec / a recorded Chrome
// 156 answer named in the test.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { JSDOM } from 'jsdom';
import { createEnvironment } from '../src/runtime/index.mjs';

const both = (html) => [createEnvironment(html), new JSDOM(html)];

test('matchMedia: a square viewport is portrait (Chrome 156, Media Queries 4; turbo-dom#138)', { todo: 'turbo-dom#138' }, () => {
  const { window } = createEnvironment('<p>x</p>');
  const orientation = (w, h) => {
    window.innerWidth = w;
    window.innerHeight = h;
    return [window.matchMedia('(orientation: portrait)').matches, window.matchMedia('(orientation: landscape)').matches];
  };
  // Chrome 156 headless at 800x800, 800x457 and 600x657 viewports
  assert.deepEqual(orientation(800, 800), [true, false]);
  assert.deepEqual(orientation(800, 457), [false, true]);
  assert.deepEqual(orientation(600, 657), [true, false]);
});

test('CharacterData textContent setter replaces the data and queues a characterData record', async () => {
  for (const { window } of both('<p>old</p>')) {
    const { document } = window;
    const text = document.querySelector('p').firstChild;
    const records = [];
    new window.MutationObserver((list) => records.push(...list)).observe(text, { characterData: true, characterDataOldValue: true });
    text.textContent = 'new';
    const comment = document.createComment('c');
    comment.textContent = 42;
    assert.deepEqual([text.data, text.nodeValue, comment.data, document.querySelector('p').textContent], ['new', 'new', '42', 'new']);
    await new Promise((resolve) => setTimeout(resolve, 0));
    assert.deepEqual(records.map((r) => [r.type, r.target === text, r.oldValue]), [['characterData', true, 'old']]);
  }
});

test('DocumentFragment.children is a live element-only collection', () => {
  for (const { window } of both('')) {
    const { document } = window;
    const frag = document.createDocumentFragment();
    const kids = frag.children;
    assert.equal(kids.length, 0);
    frag.append('t', document.createElement('a'), document.createComment('c'), document.createElement('b'));
    assert.deepEqual([kids.length, kids[0].localName, kids[1].localName, kids[2]], [2, 'a', 'b', undefined]);
    assert.equal(frag.children, kids);
    frag.firstElementChild.remove();
    assert.deepEqual([...kids].map((e) => e.localName), ['b']);
  }
});

test('Range.getBoundingClientRect() is an all-zero rect whose toJSON is the rect', () => {
  // jsdom has no Range geometry; Chrome 156 gives a collapsed range in an empty document an
  // all-zero DOMRect (its toJSON has the same eight keys).
  const { document } = createEnvironment('<p>x</p>');
  const range = document.createRange();
  const rect = range.getBoundingClientRect();
  assert.equal(rect.toJSON(), rect);
  assert.equal(JSON.stringify(rect), '{"x":0,"y":0,"top":0,"left":0,"right":0,"bottom":0,"width":0,"height":0}');
  assert.notEqual(range.getBoundingClientRect(), rect, 'a new rect per call');
});

test('screen.orientation and visualViewport accept and drop listeners', () => {
  const { window } = createEnvironment('<p>x</p>');
  const listener = () => assert.fail('never fires');
  for (const target of [window.screen.orientation, window.visualViewport]) {
    assert.equal(target.addEventListener('change', listener), undefined);
    assert.equal(target.removeEventListener('change', listener), undefined);
  }
  assert.deepEqual([window.screen.orientation.type, window.screen.orientation.angle], ['landscape-primary', 0]);
  assert.deepEqual([window.visualViewport.width, window.visualViewport.height, window.visualViewport.scale], [1024, 768, 1]);
});

test('NamedNodeMap getNamedItem / getNamedItemNS find an attribute or return null', () => {
  for (const { window } of both('<p id=a data-x=y></p>')) {
    const { attributes } = window.document.querySelector('p');
    assert.deepEqual(
      [attributes.getNamedItem('data-x').value, attributes.getNamedItem('nope'), attributes.getNamedItemNS(null, 'id').value, attributes.getNamedItemNS(null, 'nope')],
      ['y', null, 'a', null],
    );
  }
});

test('CSSStyleSheet.insertRule without an index inserts at 0', () => {
  // (an empty <style>: the npm CSSOM does not parse authored rules into cssRules, by design)
  for (const { window } of both('<style></style>')) {
    const sheet = window.document.querySelector('style').sheet;
    assert.equal(sheet.insertRule('b{color:red}'), 0);
    assert.equal(sheet.insertRule('i{color:blue}'), 0);
    assert.equal(sheet.insertRule('u{color:green}', 2), 2);
    assert.deepEqual([...sheet.cssRules].map((r) => r.selectorText), ['i', 'b', 'u']);
    assert.throws(() => sheet.insertRule('s{color:red}', 4)); // past the end (jsdom: IndexSizeError)
  }
});

test('declarative shadow roots: a bogus mode or a second template on one host stays a template', () => {
  // HTML parsing (declarative shadow root steps): only "open" / "closed" attach, and a host
  // that already has a shadow root keeps a later declarative template as a plain element.
  const { document } = createEnvironment(
    '<div id=bad><template shadowrootmode=bogus><i>b</i></template></div>' +
      '<div id=two><template shadowrootmode=open><i>1</i></template><template shadowrootmode=open><i>2</i></template></div>',
  );
  const bad = document.getElementById('bad');
  assert.equal(bad.shadowRoot, null);
  assert.deepEqual([...bad.children].map((e) => e.localName), ['template']);
  assert.equal(bad.firstElementChild.content.textContent, 'b');
  const two = document.getElementById('two');
  assert.equal(two.shadowRoot.textContent, '1');
  assert.deepEqual([...two.children].map((e) => e.localName), ['template']);
  assert.equal(two.firstElementChild.content.textContent, '2');
});

test('window add/removeEventListener: a removed listener no longer sees events bubbling from the document', () => {
  for (const { window } of both('<p>x</p>')) {
    const seen = [];
    const listener = (e) => seen.push(e.type);
    window.addEventListener('ping', listener);
    window.document.dispatchEvent(new window.Event('ping', { bubbles: true }));
    window.removeEventListener('ping', listener);
    window.document.dispatchEvent(new window.Event('ping', { bubbles: true }));
    assert.deepEqual(seen, ['ping']);
  }
});

test('the Worker stub constructs and its methods are inert', () => {
  const { window } = createEnvironment('<p>x</p>');
  const worker = new window.Worker('worker.js');
  const listener = () => assert.fail('never fires');
  assert.deepEqual(
    [worker.addEventListener('message', listener), worker.postMessage({ a: 1 }), worker.removeEventListener('message', listener), worker.terminate()],
    [undefined, undefined, undefined, undefined],
  );
});

test('math functions in <number> / <integer> longhands: Chrome 156 answers', () => {
  // recorded with Chrome for Testing 156 (el.style.setProperty + getPropertyValue)
  const chrome = [
    ['flex-grow', 'calc(1px)', ''], ['flex-grow', 'calc(2)', 'calc(2)'], ['flex-grow', 'calc(2 * 3)', 'calc(6)'],
    ['z-index', 'calc(1px)', ''], ['z-index', 'calc(2)', 'calc(2)'],
    ['column-count', 'calc(1px + 1px)', ''], ['column-count', 'calc(3)', 'calc(3)'],
    ['line-height', 'calc(1.5)', 'calc(1.5)'], ['line-height', 'calc(2px)', 'calc(2px)'], ['font-weight', 'calc(1px)', ''],
  ];
  const { document } = createEnvironment('<p>x</p>');
  const ours = chrome.map(([property, value]) => {
    const div = document.createElement('div');
    div.style.setProperty(property, value);
    return [property, value, div.style.getPropertyValue(property)];
  });
  assert.deepEqual(ours, chrome);
});

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const prototypeNames = ['__proto__', 'constructor', 'toString', 'valueOf', 'hasOwnProperty', 'isPrototypeOf'];

test('legacy event fallback does not dispatch through Object.prototype names', () => {
  const { document, window } = createEnvironment('<div id=target></div>');
  const target = document.getElementById('target');
  let calls = 0;
  target.addEventListener('legacy', () => calls++);
  // Preserve this runtime's generic Event fallback for unlisted interface names.
  for (const name of ['Event', 'UnlistedInterface', ...prototypeNames]) {
    const event = document.createEvent(name);
    assert.ok(event instanceof window.Event, name);
    event.initEvent('legacy', false, false);
    assert.equal(target.dispatchEvent(event), true, name);
    assert.equal(event.target, target, name);
  }
  assert.equal(calls, prototypeNames.length + 2);
});

test('style method dispatch leaves prototype member names on the CSS lookup path', () => {
  const { document } = createEnvironment('<div style="color:red"></div>');
  const style = document.querySelector('div').style;
  for (const name of prototypeNames) assert.equal(style[name], '', name);
  for (const name of ['getPropertyValue', 'getPropertyPriority', 'setProperty', 'removeProperty']) {
    assert.equal(style[name], style[name], name);
  }
  assert.equal(style.color, 'red');
  style.setProperty('width', '12px');
  assert.equal(style.getPropertyValue('width'), '12px');
  assert.equal(style.removeProperty('width'), '12px');
});

test('selector pseudo dispatch ignores inherited callbacks for valid pseudo names', () => {
  const { document, window } = createEnvironment('<style>#target:hover { color:red } #target:lang(en) { width:12px }</style><div id=target lang=en></div>');
  const target = document.getElementById('target');
  const cases = [
    ['hover', ':hover', false],
    ['focus', ':focus', false],
    ['lang', ':lang(en)', true],
  ];
  for (const [name, selector, expected] of cases) {
    const previous = Object.getOwnPropertyDescriptor(Object.prototype, name);
    let reads = 0;
    Object.defineProperty(Object.prototype, name, {
      configurable: true,
      get() { reads++; return () => !expected; },
    });
    try {
      assert.equal(target.matches(selector), expected, selector);
      assert.equal(document.querySelector(`#target${selector}`), expected ? target : null, selector);
      target.setAttribute('data-probe', name);
      const style = window.getComputedStyle(target);
      assert.equal(style.color, '');
      assert.equal(style.width, '12px');
      assert.equal(reads, 0, `inherited ${name} getter is not a dispatch operation`);
    } finally {
      if (previous) Object.defineProperty(Object.prototype, name, previous);
      else delete Object.prototype[name];
    }
  }
});

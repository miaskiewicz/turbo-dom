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

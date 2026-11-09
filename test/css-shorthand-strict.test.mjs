// turbo-dom#390/#391: strict grammar rejections that the big Chrome 156 shorthand control
// does not cover: the `offset` shorthand's bare-slash anchor forms (`offset: / center`,
// `offset: / auto`). Oracle: Chrome 156.0.8078.47 headless-shell,
// `CSS.supports('offset', '/ center') === false` and `CSS.supports('offset', '/ auto') === false`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

test('offset shorthand rejects a bare-slash anchor (#391)', () => {
  const { document } = createEnvironment('<!doctype html><div></div>');
  const el = document.querySelector('div');
  for (const value of ['/ center', '/ auto', '/ left top', '/ 10px']) {
    el.removeAttribute('style');
    el.style.setProperty('offset', value);
    assert.equal(el.style.getPropertyValue('offset'), '',
      `offset: ${value} must not be accepted (Chrome 156 rejects)`);
    assert.equal(el.style.getPropertyValue('offset-anchor'), '',
      `offset-anchor must stay empty for 'offset: ${value}'`);
  }
});

// Attribute values escape < and > when serialized, as in Chrome 156 (headless
// 156.0.8078.47: setAttribute('t', 'a<b>&"') -> t="a&lt;b&gt;&amp;&quot;").
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

test('outerHTML escapes < and > in attribute values like Chrome 156', () => {
  const { document } = createEnvironment('');
  const i = document.createElement('i');
  i.setAttribute('t', 'a<b>&" \'');
  assert.equal(i.outerHTML, '<i t="a&lt;b&gt;&amp;&quot;&nbsp;\'"></i>');
  const div = document.createElement('div');
  div.innerHTML = '<a title="x<y>z">q</a>';
  assert.equal(div.innerHTML, '<a title="x&lt;y&gt;z">q</a>');
});

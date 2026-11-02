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

// createElementNS prefixes survive serialization (Chrome 156 headless 156.0.8078.47):
// <h:br> stays void, <h:script> escapes its text, <h:P> keeps its case.
test('prefixed elements serialize their qualified name like Chrome 156', () => {
  const { document: d } = createEnvironment('');
  const H = 'http://www.w3.org/1999/xhtml';
  const s = d.createElement('div');
  const tp = d.createElementNS(H, 'h:template');
  tp.appendChild(d.createTextNode('t'));
  const sc = d.createElementNS(H, 'h:script');
  sc.appendChild(d.createTextNode('<'));
  s.append(d.createElementNS(H, 'h:br'), tp, sc, d.createElementNS('http://www.w3.org/1998/Math/MathML', 'm:mi'),
    d.createElementNS('http://www.w3.org/2000/svg', 'svg:g'), d.createElementNS(H, 'h:P'));
  assert.equal(s.innerHTML, '<h:br><h:template></h:template><h:script>&lt;</h:script><m:mi></m:mi><svg:g></svg:g><h:P></h:P>');
});

// SVG DOM-property wrappers (#1): SVGAnimated{String,Length,Rect}. Component libs read
// el.className.baseVal / rect.width.baseVal.value / svg.viewBox.baseVal — these must exist
// on SVG elements and stay absent on HTML ones.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEnvironment } from '../src/runtime/index.mjs';

const HTML = '<!doctype html><html><body>'
  + '<svg viewBox="0 0 24 24" class="icon big">'
  + '<rect x="2" y="3" width="10" height="20"/><circle cx="5" cy="6" r="7"/><text>t</text></svg>'
  + '<div id="h" class="plain">x</div></body></html>';
const fresh = () => createEnvironment(HTML);

test('SVG elements are SVGElement instances; HTML elements are not', () => {
  const { window, document } = fresh();
  assert.ok(document.querySelector('svg') instanceof window.SVGElement);
  assert.ok(document.querySelector('rect') instanceof window.SVGElement);
  assert.ok(!(document.querySelector('#h') instanceof window.SVGElement));
});

test('className is an SVGAnimatedString on SVG, a plain string on HTML', () => {
  const { document } = fresh();
  const svg = document.querySelector('svg');
  assert.equal(svg.className.baseVal, 'icon big');
  assert.equal(svg.className.animVal, 'icon big');
  svg.className.baseVal = 'changed';
  assert.equal(svg.getAttribute('class'), 'changed');
  // assigning a string to className writes the attribute (read still returns wrapper)
  svg.className = 'viastr';
  assert.equal(svg.className.baseVal, 'viastr');
  assert.equal(typeof document.querySelector('#h').className, 'string');
});

test('length attributes are SVGAnimatedLength with numeric baseVal', () => {
  const { document } = fresh();
  const rect = document.querySelector('rect');
  const circle = document.querySelector('circle');
  assert.equal(rect.width.baseVal.value, 10);
  assert.equal(rect.height.baseVal.value, 20);
  assert.equal(rect.x.baseVal.value, 2);
  assert.equal(rect.y.baseVal.value, 3);
  assert.equal(circle.cx.baseVal.value, 5);
  assert.equal(circle.r.baseVal.value, 7);
  // valueOf → usable as a number directly
  assert.equal(+rect.width.baseVal, 10);
  assert.equal(rect.width.baseVal + 5, 15);
  // animVal tracks baseVal
  assert.equal(rect.width.animVal.value, 10);
  // misc SVGLength surface
  assert.equal(rect.width.baseVal.unitType, 1);
  assert.equal(rect.width.baseVal.valueInSpecifiedUnits, 10);
  assert.equal(rect.width.baseVal.valueAsString, '10');
  assert.equal(String(rect.width.baseVal), '10');
});

test('missing length attribute reads 0 (honest)', () => {
  const { document } = fresh();
  const text = document.querySelector('text');
  assert.equal(text.x.baseVal.value, 0);
  assert.equal(text.x.baseVal.valueAsString, '0');
});

test('writing baseVal.value / valueAsString updates the attribute', () => {
  const { document } = fresh();
  const rect = document.querySelector('rect');
  rect.width.baseVal.value = 99;
  assert.equal(rect.getAttribute('width'), '99');
  rect.height.baseVal.valueAsString = '50';
  assert.equal(rect.getAttribute('height'), '50');
  // assigning the animated wrapper property directly (number) writes the attr
  rect.x = 12;
  assert.equal(rect.getAttribute('x'), '12');
});

test('viewBox is an SVGAnimatedRect parsed from the attribute', () => {
  const { document } = fresh();
  const b = document.querySelector('svg').viewBox.baseVal;
  assert.deepEqual([b.x, b.y, b.width, b.height], [0, 0, 24, 24]);
  // animVal === baseVal
  assert.equal(document.querySelector('svg').viewBox.animVal.width, 24);
});

test('honest geometry stubs: getBBox zeros, getCTM/ownerSVGElement', () => {
  const { document } = fresh();
  const svg = document.querySelector('svg');
  const rect = document.querySelector('rect');
  assert.deepEqual(svg.getBBox(), { x: 0, y: 0, width: 0, height: 0 });
  assert.equal(svg.getCTM(), null);
  assert.equal(svg.getScreenCTM(), null);
  assert.equal(rect.ownerSVGElement, svg);
  assert.equal(svg.ownerSVGElement, null);
});

test('createElementNS produces an SVGElement; cloneNode keeps SVG wrappers', () => {
  const { window, document } = fresh();
  const made = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
  assert.ok(made instanceof window.SVGElement);
  made.setAttribute('width', '8');
  assert.equal(made.width.baseVal.value, 8);
  const clone = document.querySelector('svg').cloneNode(true);
  assert.ok(clone instanceof window.SVGElement);
  assert.equal(clone.className.baseVal, 'icon big');
});

test('SVG built via innerHTML also gets wrappers', () => {
  const { window, document } = fresh();
  const host = document.querySelector('#h');
  host.innerHTML = '<svg><rect width="4" height="5"/></svg>';
  const rect = host.querySelector('rect');
  assert.ok(rect instanceof window.SVGElement);
  assert.equal(rect.width.baseVal.value, 4);
});

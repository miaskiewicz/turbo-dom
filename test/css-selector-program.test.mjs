import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createEnvironment, setParser } from '../src/runtime/index.mjs';
import { compileCssSelectorProgram, matchesCssProgram } from '../src/runtime/selectors.mjs';

if (process.env.TURBO_DOM_TEST_PARSER) setParser(createRequire(import.meta.url)(process.env.TURBO_DOM_TEST_PARSER));
const html = '<div class=outside><div class=outsideSibling></div><section class="root &" id=r data-mark=":scope&"><div class=nested><button class=target id=t>T</button></div></section><div class=outsideSibling></div></div>';
function setup() {
  const env = createEnvironment(html);
  return { ...env, root: env.document.querySelector('.root'), target: env.document.querySelector('.target') };
}
const program = (source, implicitScopeAnchor = false) => compileCssSelectorProgram(source, { implicitScopeAnchor, allowScopeReference: true });

const caseSource = readFileSync(new URL('./fixtures/css-selector-program-cases.json', import.meta.url), 'utf8');
const probeSource = readFileSync(new URL('./fixtures/css-selector-program-probe.mjs', import.meta.url), 'utf8');
const cases = JSON.parse(caseSource);
const control = JSON.parse(readFileSync(new URL('./fixtures/css_selector_program_chrome159.json', import.meta.url), 'utf8'));

// The Rust caller owns root/limit eligibility, layer order and proximity. This
// source-hashed oracle establishes the mechanical matches for its scoped facts.
test('CSS scope programs match the actual frozen Chrome159 selector control', () => {
  assert.equal(control.browser, '159.0.8154.6');
  const hash = (source) => createHash('sha256').update(source).digest('hex');
  assert.equal(control.probeSha256, hash(probeSource));
  assert.equal(control.casesSha256, hash(caseSource));
  const { root, target } = setup();
  assert.equal(cases.scope.length, control.result.scope.length);
  for (let i = 0; i < cases.scope.length; i++) {
    const row = cases.scope[i];
    const expected = control.result.scope[i];
    assert.equal(expected.selector, row.selector);
    const subject = row.target === 'root' ? root : target;
    assert.equal(matchesCssProgram(subject, program(row.selector, row.implicitScopeAnchor), { scopeRoot: root }), expected.matches, row.selector);
  }
});

test('CSS programs never mutate DOM cached pseudo subjects or repair stylesheet EOF', () => {
  const { root, target } = setup();
  assert.equal(target.matches('.target::before'), false);
  const before = compileCssSelectorProgram('.target::before', { pseudo: '::before' });
  assert.equal(matchesCssProgram(target, before), true);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.outside .target::before', { pseudo: ':before' })), true);
  assert.equal(target.matches('.target::before'), false);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.target::after', { pseudo: 'before' })), false);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.target', { pseudo: 'before' })), false);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.target::before')), false);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.target::before::marker', { pseudo: 'marker' })), false);
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram('.target::before::marker', { pseudo: 'before' })), false);
  assert.equal(target.matches(':is(.target'), true);
  assert.throws(() => compileCssSelectorProgram(':is(.target'), SyntaxError);
  assert.throws(() => compileCssSelectorProgram('& .target'), SyntaxError);
  assert.throws(() => compileCssSelectorProgram('& .target', { allowScopeReference: false }), SyntaxError);
  assert.throws(() => matchesCssProgram(target, program('.target', true)), /scope root is required/);
  assert.equal(matchesCssProgram(target, program('.target', true), { scopeRoot: root }), true);
  const foreign = createEnvironment('<div></div>').document.documentElement;
  assert.equal(matchesCssProgram(target, program('.target', true), { scopeRoot: foreign }), false);
  assert.throws(() => matchesCssProgram(target, {}), TypeError);
});

test('raw quoted units and repaired identifier units remain distinct in CSS programs', () => {
  const { document } = createEnvironment('<div id=raw></div><div id=repaired></div><div id=marker></div>');
  const records = [['raw', '\ud800'], ['repaired', '\ufffd'], ['marker', '\u{f0000}\u{f0800}']];
  for (const [id, value] of records) { const el = document.getElementById(id); el.setAttribute('data-x', value); el.className = 'x' + value; }
  for (const row of control.result.units) {
    const source = row.selector;
    const raw = Uint16Array.from({ length: source.length }, (_, i) => source.charCodeAt(i));
    const compiled = compileCssSelectorProgram(raw);
    assert.deepEqual(records.filter(([id]) => matchesCssProgram(document.getElementById(id), compiled)).map(([id]) => id), row.matches, source);
  }
  assert.equal(document.getElementById('repaired').matches('.x\ud800'), true);
  assert.equal(document.getElementById('raw').matches('.x\ud800'), false);
  assert.equal(document.getElementById('raw').matches('[data-x="\ud800"]'), true);
  assert.throws(() => compileCssSelectorProgram([65536]), TypeError);
  assert.throws(() => compileCssSelectorProgram([-1]), TypeError);
  assert.throws(() => compileCssSelectorProgram([1.5]), TypeError);
  assert.throws(() => compileCssSelectorProgram({}), TypeError);
});

test('CSS focus uses canonical runtime facts without changing DOM resting matching', () => {
  const { document, target } = setup();
  const liveState = { focusTarget: (doc) => doc.__active };
  const focus = program(':focus');
  const within = program(':focus-within');
  assert.equal(matchesCssProgram(target, focus, { liveState }), false);
  assert.equal(matchesCssProgram(document.body, within, { liveState }), false);
  target.focus();
  assert.equal(document.__active, target);
  assert.equal(matchesCssProgram(target, focus, { liveState }), true);
  assert.equal(matchesCssProgram(document.body, within, { liveState }), true);
  assert.equal(matchesCssProgram(document.body, focus, { liveState }), false);
  assert.equal(target.matches(':focus'), false);
  assert.equal(document.querySelector(':focus'), null);
  const reentrant = { focusTarget: (doc) => { assert.equal(target.matches(':focus'), false); assert.equal(doc.querySelector(':focus'), null); return doc.__active; } };
  assert.equal(matchesCssProgram(target, focus, { liveState: reentrant }), true);
  assert.throws(() => matchesCssProgram(target, focus), /focus state is unavailable/);
  const error = new Error('original state provider error');
  assert.throws(() => matchesCssProgram(target, focus, { liveState: { focusTarget: () => { throw error; } } }), (actual) => actual === error);
  assert.equal(target.matches(':focus'), false);
  assert.throws(() => matchesCssProgram(target, focus, { liveState: { focusTarget: () => ({}) } }), /invalid CSS focus target/);
  const foreign = createEnvironment('<button></button>').document.querySelector('button');
  assert.throws(() => matchesCssProgram(target, focus, { liveState: { focusTarget: () => foreign } }), /invalid CSS focus target/);
});

test('focus-within follows composed slots while focus retargets only shadow boundaries', () => {
  const { document } = createEnvironment('<main><div id=host><button id=slotted>light</button></div></main>');
  const host = document.getElementById('host');
  const shadow = host.attachShadow({ mode: 'open' });
  shadow.innerHTML = '<section><slot></slot><button id=inner>shadow</button></section>';
  const inner = shadow.getElementById('inner');
  const liveState = { focusTarget: (doc) => doc.__active };
  inner.focus();
  assert.equal(matchesCssProgram(inner, program(':focus'), { liveState }), true);
  assert.equal(matchesCssProgram(host, program(':focus'), { liveState }), true);
  assert.equal(matchesCssProgram(document.querySelector('main'), program(':focus'), { liveState }), false);
  assert.equal(matchesCssProgram(document.body, program(':focus-within'), { liveState }), true);
  const slotted = document.getElementById('slotted');
  slotted.focus();
  assert.equal(matchesCssProgram(host, program(':focus'), { liveState }), false);
  assert.equal(matchesCssProgram(shadow.querySelector('slot'), program(':focus-within'), { liveState }), true);
});

test('synthetic pointer events do not invent trusted hover or active and modality stays explicit', () => {
  const { window, target } = setup();
  const liveState = { focusTarget: (doc) => doc.__active };
  for (const type of ['mouseover', 'mousedown', 'pointerdown']) target.dispatchEvent(new window.Event(type, { bubbles: true }));
  assert.equal(matchesCssProgram(target, program(':hover'), { liveState }), false);
  assert.equal(matchesCssProgram(target, program(':active'), { liveState }), false);
  assert.equal(matchesCssProgram(target, program(':focus-visible'), { liveState }), false);
  assert.equal(matchesCssProgram(target, program(':focus-visible'), { liveState: { ...liveState, matchesPseudo: () => { throw new Error('unneeded modality lookup'); } } }), false);
  target.focus();
  assert.throws(() => matchesCssProgram(target, program(':focus-visible'), { liveState }), /modality is unavailable/);
  assert.equal(matchesCssProgram(target, program(':focus-visible'), { liveState: { ...liveState, matchesPseudo: () => true } }), true);
  assert.throws(() => matchesCssProgram(target, program(':hover'), { liveState: { matchesPseudo: () => undefined } }), /state is unavailable/);
});

test('CSS unit-array programs retain supplementary names across transport chunks', () => {
  const { target } = setup();
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram([46,116,97,114,103,101,116])), true);
  const name = 'a'.repeat(1022) + '\u{f0000}';
  target.className = name;
  const source = '.' + name;
  const raw = Uint16Array.from({ length: source.length }, (_, i) => source.charCodeAt(i));
  assert.equal(matchesCssProgram(target, compileCssSelectorProgram(raw)), true);
});

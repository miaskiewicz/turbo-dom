// CSSOM shorthand properties as Chrome 156 implements them (miaskiewicz/turbo-dom#90): the
// shorthand table, setting a shorthand (each longhand set — omitted ones to their initial value,
// or pending when the value holds var()), serializing a shorthand from its longhands
// (getPropertyValue) and a declaration block (cssText).
//
// The npm runtime's lockstep port of turbo-dom's Rust `rtdom::css_shorthand` (crates/turbo-dom/
// src/rtdom/css_shorthand/): same table, grammars and serializers, checked against the same
// Chrome 156 control (crates/turbo-dom/tests/fixtures/css_shorthand_chrome156.json, by
// test/css-shorthand-chrome.test.mjs). Change both together.
//
// PERF: only element.style reads/writes reach this module (test/app time).
import { canonicalizeColor } from './color.mjs';
import { SHORTHAND_TABLE, LONGHAND_ALIASES } from './css-shorthand-table.mjs';

// ---------------------------------------------------------------- numbers ----
const f32 = Math.fround;
/** The largest finite f32. */
const F32_MAX = 3.4028234663852886e38;
/** Blink's CSS number serialization: six significant digits (printf `%g`). */
export function cssNumber(value) {
  const v = f32(value);
  if (v === 0 || !Number.isFinite(v)) return '0';
  const exponent = Math.floor(Math.log10(Math.abs(v)));
  const trim = (text) => (text.includes('.') ? text.replace(/0+$/, '').replace(/\.$/, '') : text);
  if (exponent < -5 || exponent >= 6) {
    const [mantissa, power] = v.toExponential(5).split('e');
    const p = Number(power);
    return `${trim(mantissa)}e${p < 0 ? '-' : '+'}${String(Math.abs(p)).padStart(2, '0')}`;
  }
  return trim(v.toFixed(5 - exponent));
}

// --------------------------------------------------------------- tokenizer ----
// The CSS Syntax tokens the grammars read: ident, function, number / percentage / dimension,
// hash, string, url, delim, comma, (, ), [, ], whitespace. `null` for bad strings / urls.
const isNameStart = (c) => /[A-Za-z_\u0080-￿]/.test(c);
const isName = (c) => /[A-Za-z0-9_\-\u0080-￿]/.test(c);
// CSS Syntax "consume an escaped code point": zero, a surrogate or past U+10FFFF is U+FFFD.
const escapedCodePoint = (hex) => {
  const code = parseInt(hex, 16);
  return String.fromCodePoint(code === 0 || code > 0x10ffff || (code >= 0xd800 && code <= 0xdfff) ? 0xfffd : code);
};
const tokStartsNumber = (text, at) => {
  const c = text[at], d = text[at + 1], e = text[at + 2];
  if (c === '+' || c === '-') return /[0-9]/.test(d || '') || (d === '.' && /[0-9]/.test(e || ''));
  if (c === '.') return /[0-9]/.test(d || '');
  return /[0-9]/.test(c || '');
};
const tokStartsIdent = (text, at) => {
  const c = text[at], d = text[at + 1];
  const escape = (index) => text[index] === '\\' && text[index + 1] !== '\n';
  if (c === '-') return isNameStart(d || '') || d === '-' || escape(at + 1);
  if (c === '\\') return escape(at);
  return isNameStart(c || '');
};
function tokReadName(text, state) {
  const n = text.length;
  let name = '';
  while (state.i < n) {
    const c = text[state.i];
    if (isName(c)) { name += c; state.i++; }
    else if (c === '\\' && state.i + 1 >= n) { name += '\ufffd'; state.i++; }
    else if (c === '\\' && text[state.i + 1] !== '\n') {
      state.i++;
      const hex = /^[0-9a-fA-F]{1,6}/.exec(text.slice(state.i));
      if (hex) { name += escapedCodePoint(hex[0]); state.i += hex[0].length; if (/\s/.test(text[state.i] || '')) state.i++; }
      else { name += text[state.i]; state.i++; }
    } else break;
  }
  return name;
}
function tokReadString(text, state, quote) {
  const n = text.length;
  let value = ''; state.i++;
  while (state.i < n) {
    const d = text[state.i];
    if (d === quote) { state.i++; return { type: 'string', value }; }
    if (d === '\n') return null;
    if (d === '\\') {
      if (state.i + 1 >= n) { state.i++; continue; }
      const e = text[state.i + 1];
      if (e === '\n') { state.i += 2; continue; }
      state.i++;
      const hex = /^[0-9a-fA-F]{1,6}/.exec(text.slice(state.i));
      if (hex) { value += escapedCodePoint(hex[0]); state.i += hex[0].length; if (/\s/.test(text[state.i] || '')) state.i++; }
      else { value += e; state.i++; }
      continue;
    }
    value += d; state.i++;
  }
  return { type: 'string', value };
}
function tokReadNumeric(text, state) {
  const m = /^[+-]?(\d*\.\d+|\d+)([eE][+-]?\d+)?/.exec(text.slice(state.i));
  const raw = m[0];
  state.i += raw.length;
  const value = Number(raw);
  const integer = !/[.eE]/.test(raw);
  if (text[state.i] === '%') { state.i++; return { type: 'percentage', value, raw }; }
  if (tokStartsIdent(text, state.i)) return { type: 'dimension', value, unit: tokReadName(text, state) };
  return { type: 'number', value, integer };
}
function tokReadUrlEscape(text, state) {
  const n = text.length;
  if (text[state.i + 1] === '\n') return null;
  state.i++;
  const hex = /^[0-9a-fA-F]{1,6}/.exec(text.slice(state.i));
  if (hex) { const value = escapedCodePoint(hex[0]); state.i += hex[0].length; if (/\s/.test(text[state.i] || '')) state.i++; return value; }
  if (state.i < n) { const value = text[state.i]; state.i++; return value; }
  return '\ufffd';
}
function tokReadUrlBody(text, state) {
  const n = text.length;
  let value = '';
  while (state.i < n && text[state.i] !== ')') {
    const d = text[state.i];
    if (/\s/.test(d)) { while (state.i < n && /\s/.test(text[state.i])) state.i++; if (state.i < n && text[state.i] !== ')') return null; break; }
    if (d === '"' || d === "'" || d === '(') return null;
    if (d === '\\') { const escaped = tokReadUrlEscape(text, state); if (escaped === null) return null; value += escaped; continue; }
    value += d; state.i++;
  }
  if (state.i < n) state.i++;
  return { type: 'url', value };
}
function tokReadIdentOrFunc(text, state) {
  const n = text.length;
  const name = tokReadName(text, state);
  if (text[state.i] !== '(') return { type: 'ident', value: name };
  state.i++;
  if (name.toLowerCase() === 'url') {
    let j = state.i;
    while (j < n && /\s/.test(text[j])) j++;
    if (text[j] !== '"' && text[j] !== "'") {
      state.i = j;
      return tokReadUrlBody(text, state);
    }
  }
  return { type: 'function', name };
}
const PUNCT_TOKENS = { ',': { type: 'comma' }, '(': { type: 'open', value: '(' }, '[': { type: 'open', value: '[' }, '{': { type: 'open', value: '{' }, ')': { type: 'close', value: ')' }, ']': { type: 'close', value: ']' }, '}': { type: 'close', value: '}' }, ';': { type: 'semicolon' } };
function tokPunctuation(c) {
  return PUNCT_TOKENS[c] ?? { type: 'delim', value: c };
}
function tokenizeComment(text, state, n) {
  if (text[state.i] !== '/' || text[state.i + 1] !== '*') return false;
  const end = text.indexOf('*/', state.i + 2);
  state.i = end < 0 ? n : end + 2;
  return true;
}
function tokenizeWhitespace(text, state, n, out) {
  if (!/\s/.test(text[state.i])) return false;
  while (state.i < n && /\s/.test(text[state.i])) state.i++;
  out.push({ type: 'ws' });
  return true;
}
function tokenizeHash(text, state, n, out) {
  if (text[state.i] !== '#' || state.i + 1 >= n || !(isName(text[state.i + 1]) || text[state.i + 1] === '\\')) return false;
  state.i++;
  out.push({ type: 'hash', value: tokReadName(text, state) });
  return true;
}
function tokenizeStep(text, state, n, out) {
  if (tokenizeComment(text, state, n)) return true;
  if (tokenizeWhitespace(text, state, n, out)) return true;
  const c = text[state.i];
  if (c === '"' || c === "'") { const token = tokReadString(text, state, c); if (!token) return false; out.push(token); return true; }
  if (tokStartsNumber(text, state.i)) { out.push(tokReadNumeric(text, state)); return true; }
  if (tokStartsIdent(text, state.i)) { const token = tokReadIdentOrFunc(text, state); if (!token) return false; out.push(token); return true; }
  if (tokenizeHash(text, state, n, out)) return true;
  state.i++;
  out.push(tokPunctuation(c));
  return true;
}
function tokenize(text) {
  const out = [];
  const state = { i: 0 };
  const n = text.length;
  while (state.i < n) {
    if (!tokenizeStep(text, state, n, out)) return null;
  }
  return out;
}

// ------------------------------------------------------------- components ----
const LENGTH_UNITS = new Set(['px', 'em', 'rem', 'ex', 'rex', 'ch', 'rch', 'ic', 'ric', 'cap', 'rcap', 'lh', 'rlh', 'vw', 'vh', 'vi', 'vb', 'vmin', 'vmax', 'svw', 'svh', 'svi', 'svb', 'svmin', 'svmax', 'lvw', 'lvh', 'lvi', 'lvb', 'lvmin', 'lvmax', 'dvw', 'dvh', 'dvi', 'dvb', 'dvmin', 'dvmax', 'cqw', 'cqh', 'cqi', 'cqb', 'cqmin', 'cqmax', 'cm', 'mm', 'q', 'in', 'pt', 'pc']);
const MATH_FUNCTIONS = new Set(['calc', 'min', 'max', 'clamp', 'round', 'mod', 'rem', 'abs', 'sign', 'sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'atan2', 'pow', 'sqrt', 'hypot', 'log', 'exp', 'calc-size', 'sibling-index', 'sibling-count']);
const IMAGE_FUNCTIONS = new Set(['linear-gradient', 'radial-gradient', 'conic-gradient', 'repeating-linear-gradient', 'repeating-radial-gradient', 'repeating-conic-gradient', '-webkit-linear-gradient', '-webkit-radial-gradient', '-webkit-repeating-linear-gradient', '-webkit-repeating-radial-gradient', '-webkit-gradient', 'image-set', '-webkit-image-set', 'cross-fade', '-webkit-cross-fade', 'paint', 'image']);
const COLOR_FUNCTIONS = new Set(['rgb', 'rgba', 'hsl', 'hsla', 'hwb', 'lab', 'lch', 'oklab', 'oklch', 'color', 'color-mix', 'light-dark', 'contrast-color', '-webkit-device-cmyk']);
const SYSTEM_COLORS = new Set(['currentcolor', 'transparent', 'accentcolor', 'accentcolortext', 'activetext', 'buttonborder', 'buttonface', 'buttontext', 'canvas', 'canvastext', 'field', 'fieldtext', 'graytext', 'highlight', 'highlighttext', 'linktext', 'mark', 'marktext', 'selecteditem', 'selecteditemtext', 'visitedtext', 'activeborder', 'activecaption', 'appworkspace', 'background', 'buttonhighlight', 'buttonshadow', 'captiontext', 'inactiveborder', 'inactivecaption', 'inactivecaptiontext', 'infobackground', 'infotext', 'menu', 'menutext', 'scrollbar', 'threeddarkshadow', 'threedface', 'threedhighlight', 'threedlightshadow', 'threedshadow', 'window', 'windowframe', 'windowtext', '-webkit-link', '-webkit-focus-ring-color', '-internal-quirk-inherit']);
const CSS_WIDE = new Set(['initial', 'inherit', 'unset', 'revert', 'revert-layer']);

export const quoted = (text) => {
  let out = '"';
  for (const c of text) out += c === '"' ? '\\"' : c === '\\' ? '\\\\' : c === '\n' ? '\\a ' : c;
  return `${out}"`;
};

/** Join components with single spaces, commas as `, `. */
function join(comps) {
  let out = '';
  comps.forEach((comp, index) => {
    if (comp.kind === 'comma') { out += ','; return; }
    if (index > 0) out += ' ';
    out += comp.text;
  });
  return out;
}

/**
 * The like-unit terms of a sum of products (operands: literals, the constants, nested
 * parentheses / `calc()`, one-unit `min()` / `max()` / `clamp()` already simplified to
 * `calc(...)`), absolute lengths combined in px; `null` when Chrome keeps it unsimplified.
 */
function sumOperand(comp) {
  const terms = literalTerms(comp);
  if (terms) return terms;
  if (comp.kind === 'function' && comp.name === '(') { const nested = components(comp.text.slice(1, -1)); return nested && sumTerms(nested); }
  // A nested math function Chrome simplified already spells `calc(...)`.
  if (isMath(comp) && comp.text.startsWith('calc(')) {
    const nested = components(comp.text.slice(5, -1));
    return nested && sumTerms(nested);
  }
  return null;
}
const sumScalar = (terms) => (terms.length === 1 && terms[0][0] === '' ? terms[0][1] : null);
const sumScale = (terms, factor) => terms.map(([unit, value]) => [unit, value * factor]);
function sumApplyOperator(op, current, next) {
  if (op.kind === 'delim' && op.value === '*') {
    const b = sumScalar(next);
    return { current: b !== null ? sumScale(current, b) : sumScale(next, sumScalar(current)) };
  }
  if (op.kind === 'slash' || (op.kind === 'delim' && op.value === '/')) {
    const b = sumScalar(next);
    return b === null ? null : { current: sumScale(current, 1 / b) };
  }
  // The remaining case is +/- by precondition: every caller of sumTerms (the only caller of
  // sumApplyOperator) runs after functionValid / checkedSum, which rejects any other operator
  // at an odd position. Chrome 156 fixture-exercised in test/css-shorthand-chrome.test.mjs.
  return { current: next, flush: op.value === '+' ? 1 : -1 };
}
function sumTerms(inner) {
  // Operands alternate with operators; products bind first.
  if (inner.length % 2 === 0) return null;
  const terms = [];
  const push = (product, sign) => {
    for (const [unit, value] of product) { const existing = terms.find((t) => t[0] === unit); if (existing) existing[1] += sign * value; else terms.push([unit, sign * value]); }
  };
  let sign = 1;
  let current = sumOperand(inner[0]);
  if (!current) return null;
  for (let index = 1; index < inner.length; index += 2) {
    const next = sumOperand(inner[index + 1]);
    if (!next) return null;
    const applied = sumApplyOperator(inner[index], current, next);
    if (!applied) return null;
    if (applied.flush !== undefined) { push(current, sign); sign = applied.flush; }
    current = applied.current;
  }
  push(current, sign);
  return canonicalTerms(terms);
}
const CONSTANTS = { e: Math.E, pi: Math.PI, infinity: Infinity, '-infinity': -Infinity, nan: NaN };
/** A literal `calc()` operand's one term (a number, a percentage, a dimension, a constant), else null. */
function literalTerms(comp) {
  if (comp.kind === 'number') return [['', comp.value]];
  if (comp.kind === 'percentage') return [['%', comp.value]];
  if (comp.kind === 'dimension') return [[comp.unit, comp.value]];
  if (comp.kind === 'ident') { const name = comp.value.toLowerCase(); return name in CONSTANTS ? [['', CONSTANTS[name]]] : null; }
  return null;
}
/** Like-unit terms with absolute lengths in px and times, angles and frequencies in their canonical unit. */
function canonicalTerms(terms) {
  // Absolute lengths combine in px.
  const ABSOLUTE = { in: 96, cm: 96 / 2.54, mm: 96 / 25.4, q: 96 / 101.6, pt: 96 / 72, pc: 16 };
  if (terms.some(([unit]) => unit in ABSOLUTE)) {
    let px = 0;
    const rest = [];
    for (const [unit, value] of terms) {
      if (unit in ABSOLUTE) px += value * ABSOLUTE[unit];
      else if (unit === 'px') px += value;
      else rest.push([unit, value]);
    }
    rest.push(['px', px]);
    terms.length = 0;
    terms.push(...rest);
  }
  // Times, angles and frequencies resolve to their canonical unit (`calc(1ms)` is `calc(0.001s)`).
  for (const [canonical, factors] of [['s', { s: 1, ms: 0.001 }], ['deg', { deg: 1, grad: 0.9, rad: 180 / Math.PI, turn: 360 }], ['hz', { hz: 1, khz: 1000 }]]) {
    if (!terms.some(([unit]) => unit in factors)) continue;
    let total = 0;
    for (let index = terms.length - 1; index >= 0; index--) if (terms[index][0] in factors) { total += terms[index][1] * factors[terms[index][0]]; terms.splice(index, 1); }
    terms.push([canonical, total]);
  }
  return terms;
}
function calcSum(inner) {
  if (hasTreeCounting(inner)) return treeCalcText(inner);
  const terms = sumTerms(inner);
  return terms && calcText(terms);
}
const unitCategory = (unit) => (LENGTH_UNITS.has(unit) ? 0 : unit === 's' || unit === 'ms' ? 1 : ['deg', 'grad', 'rad', 'turn'].includes(unit) ? 2 : unit === 'hz' || unit === 'khz' ? 3 : ['dpi', 'dpcm', 'dppx', 'x'].includes(unit) ? 4 : null);
/**
 * The type of a `calc()` sum (`number`, `percent`, a unit category, `unknown` for a nested
 * math function), `null` when Chrome rejects it: a sum mixes numbers and dimensions or two
 * dimension categories, a product multiplies two dimensions, or a divisor has a unit.
 */
function calcTypeOperand(comp) {
  if (comp.kind === 'number') return 'number';
  if (comp.kind === 'percentage') return 'percent';
  if (comp.kind === 'dimension') { const category = unitCategory(comp.unit); return category === null ? null : `unit${category}`; }
  if (comp.kind === 'function' && comp.name === '(') { const nested = components(comp.text.slice(1, -1)); return nested && calcType(nested); }
  if (isMath(comp)) return mathType(comp);
  if (comp.kind === 'ident' && ['e', 'pi', 'infinity', '-infinity', 'nan'].includes(comp.value.toLowerCase())) return 'number';
  return null;
}
function calcTypeMultiply(product, next, total) {
  if (unitType(product) && unitType(next)) return null;
  return { product: (product === 'number' || product === 'unknown') ? next : product, total };
}
function calcTypeAdd(product, next, total) {
  const newTotal = total === null ? product : sumType(total, product);
  return newTotal === null ? null : { product: next, total: newTotal };
}
function calcTypeStep(op, product, next, total) {
  if (op.kind === 'delim' && op.value === '*') return calcTypeMultiply(product, next, total);
  if (op.kind === 'slash' || (op.kind === 'delim' && op.value === '/')) return unitType(next) ? null : { product, total };
  if (op.kind === 'delim' && (op.value === '+' || op.value === '-')) return calcTypeAdd(product, next, total);
  return null;
}
function calcType(inner) {
  if (!inner.length) return null;
  let total = null;
  let product = calcTypeOperand(inner[0]);
  if (product === null) return null;
  for (let index = 1; index < inner.length; index += 2) {
    const next = inner[index + 1] && calcTypeOperand(inner[index + 1]);
    if (!next) return null;
    const stepped = calcTypeStep(inner[index], product, next, total);
    if (!stepped) return null;
    product = stepped.product; total = stepped.total;
  }
  return total === null ? product : sumType(total, product);
}
const unitType = (type) => type === 'percent' || type.startsWith('unit');
/** The type of a sum of two types (`null`: Chrome rejects adding them). */
function sumType(a, b) {
  if (a === 'unknown') return b;
  if (b === 'unknown') return a;
  if (a === 'number' && b === 'number') return 'number';
  if (a === 'percent' && unitType(b)) return b;
  if (b === 'percent' && unitType(a)) return a;
  return a.startsWith('unit') && a === b ? a : null;
}
const isMath = (c) => c.kind === 'function' && MATH_FUNCTIONS.has(c.name);
/**
 * A math function's type: a simplified one's `calc()` sum, `min()` / `max()` / `clamp()` /
 * `abs()` their arguments' common type, `sign()` a number, the others unknown.
 */
function mathType(comp) {
  const inner = innerOf(comp);
  if (comp.text.startsWith('calc(')) return calcType(inner);
  if (comp.name === 'sign' || comp.name === 'sibling-index' || comp.name === 'sibling-count') return 'number';
  return ['min', 'max', 'clamp', 'abs'].includes(comp.name) ? argumentsType(inner) : 'unknown';
}
/** A function's comma-separated arguments. */
const argumentsOf = (inner) => split(inner, isComma);
/**
 * A `calc()` sum's type when Chrome accepts it: operands alternate with operators (a literal
 * product has at most one unit) and the types add up.
 */
function checkedSum(inner) {
  if (!inner.length || inner.length % 2 === 0) return null;
  const unit = (c) => c.kind === 'dimension' || c.kind === 'percentage';
  const alternates = inner.every((comp, index) => {
    if (index % 2 === 1) {
      const product = comp.kind === 'delim' && comp.value === '*';
      const operator = product || (comp.kind === 'delim' && ['+', '-', '/'].includes(comp.value)) || comp.kind === 'slash';
      return operator && !(product && unit(inner[index - 1]) && unit(inner[index + 1]));
    }
    return comp.kind !== 'comma' && comp.kind !== 'slash' && comp.kind !== 'delim';
  });
  const type = calcType(inner);
  return alternates ? type : null;
}
const isNone = (argument) => argument.length === 1 && isIdent(argument[0], ['none']);
/**
 * The common type of a math function's comma-separated `calc()` sums (`clamp()`'s `none`
 * bounds left out); `null` when an argument is not a sum or the types do not add up.
 */
function argumentsType(inner) {
  let total = 'unknown';
  for (const argument of argumentsOf(inner)) {
    if (isNone(argument)) continue;
    const type = checkedSum(argument);
    total = type === null ? null : sumType(total, type);
    if (total === null) return null;
  }
  return total;
}
function rgbHslValid(name, inner) {
  const commasCount = inner.filter((c) => c.kind === 'comma').length;
  // Relative color syntax: `from <color>`, three channels, then `/ alpha`.
  if (inner.length && isIdent(inner[0], ['from'])) return commasCount === 0 && (inner.length === 5 || (inner.length === 7 && inner[5].kind === 'slash'));
  // A hue: a number, an angle, `none` (modern syntax) or a math function of those.
  const hue = (c) => c.kind === 'number' || (c.kind === 'dimension' && unitCategory(c.unit) === 2) || (commasCount === 0 && isIdent(c, ['none'])) || (isMath(c) && ['number', 'unit2', 'unknown'].includes(mathType(c)));
  if (name.startsWith('hsl') && inner.length && !hue(inner[0])) return false;
  if (commasCount) return (commasCount === 2 || commasCount === 3) && inner.length === 2 * commasCount + 1 && inner.every((c, index) => (c.kind === 'comma') === (index % 2 === 1));
  const slash = inner.findIndex((c) => c.kind === 'slash');
  return slash < 0 ? inner.length === 3 : slash === 3 && inner.length === 5;
}
function linearValid(inner) {
  // Each stop: a number output and at most two percentage positions.
  return inner.some((c) => c.kind === 'comma') && argumentsOf(inner).every((stop) => stop.length >= 1 && stop.length <= 3 && stop[0].kind === 'number' && stop.slice(1).every((c) => c.kind === 'percentage'));
}
function stepsValid(inner) {
  const count = (c, minimum) => (c.kind === 'number' ? c.integer && c.value >= minimum : c.kind === 'function' && MATH_FUNCTIONS.has(c.name));
  if (inner.length === 1) return count(inner[0], 1);
  if (inner.length === 3 && inner[1].kind === 'comma') {
    const position = keyword(inner[2], ['start', 'end', 'jump-start', 'jump-end', 'jump-none', 'jump-both']);
    return position !== null && count(inner[0], position === 'jump-none' ? 2 : 1);
  }
  return false;
}
function cubicBezierValid(inner) {
  const args = argumentsOf(inner);
  return args.length === 4 && args.every((arg, index) => arg.length === 1 && arg[0].kind === 'number' && (index % 2 === 1 || (arg[0].value >= 0 && arg[0].value <= 1)));
}
const COLOR_FN_NAMES = new Set(['rgb', 'rgba', 'hsl', 'hsla']);
/**
 * Structural checks Chrome makes before keeping a function.
 */
function functionValid(name, inner) {
  if (name === 'calc') return checkedSum(inner) !== null;
  if (name === 'min' || name === 'max') return !argumentsOf(inner).some(isNone) && argumentsType(inner) !== null;
  if (name === 'clamp') { const args = argumentsOf(inner); return args.length === 3 && !isNone(args[1]) && argumentsType(inner) !== null; }
  if (name === 'abs' || name === 'sign') return !inner.some(isComma) && checkedSum(inner) !== null;
  if (COLOR_FN_NAMES.has(name)) return rgbHslValid(name, inner);
  if (name === 'linear') return linearValid(inner);
  if (name === 'steps') return stepsValid(inner);
  if (name === 'cubic-bezier') return cubicBezierValid(inner);
  return treeArgumentsValid(name, inner);
}
/** The tree-counting functions take no arguments. */
const treeArgumentsValid = (name, inner) => !TREE_COUNTING.has(name) || inner.length === 0;
const TREE_COUNTING = new Set(['sibling-index', 'sibling-count']);
/** `calc()` of simplified terms: numbers, then percentages, then units in ASCII order. */
function calcText(terms) {
  if (terms.length === 1 && !Number.isFinite(terms[0][1])) {
    // `calc(1px / 0)` is `calc(infinity * 1px)`.
    const [unit, value] = terms[0];
    const infinity = Number.isNaN(value) ? 'NaN' : value < 0 ? '-infinity' : 'infinity';
    return `calc(${infinity}${unit ? ` * 1${unit}` : ''})`;
  }
  const rank = (unit) => (unit === '' ? 0 : unit === '%' ? 1 : 2);
  terms.sort((a, b) => rank(a[0]) - rank(b[0]) || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  let out = 'calc(';
  terms.forEach(([unit, value], index) => {
    if (index === 0) out += cssNumber(value);
    else if (value < 0) out += ` - ${cssNumber(-value)}`;
    else out += ` + ${cssNumber(value)}`;
    out += unit;
  });
  return `${out})`;
}
/**
 * Units whose values Chrome resolves while parsing (absolute lengths, times, angles and
 * frequencies after `sumTerms` canonicalized them): `abs()` / `sign()` / `min()` / `max()` /
 * `clamp()` of them simplify; relative lengths and percentages stay as written.
 */
const RESOLVED_UNITS = ['', 'px', 's', 'deg', 'hz'];
/** Math functions whose arguments are `calc()` sums. */
const SUM_FUNCTIONS = ['min', 'max', 'clamp', 'abs', 'sign'];
/** A sum's single term when Chrome resolves it while parsing, else null. */
const resolved = (terms) => (terms && terms.length === 1 && RESOLVED_UNITS.includes(terms[0][0]) ? terms[0] : null);
/**
 * `min()` / `max()` / `clamp()` Chrome simplifies: one argument (`clamp()`'s `none` bounds
 * left out) is its `calc()` sum, arguments that all resolve in one unit their value.
 */
function minMax(name, inner) {
  const args = argumentsOf(inner);
  const present = args.filter((argument) => !isNone(argument));
  if (present.length === 1) { const terms = sumTerms(present[0]); return terms && calcText(terms); }
  let unit = null;
  const values = [];
  for (const argument of args) {
    if (isNone(argument)) { values.push(null); continue; }
    // functionValid type-checked the arguments: resolved ones share the unit.
    const term = resolved(sumTerms(argument));
    if (!term) return null;
    unit = term[0];
    values.push(term[1]);
  }
  const given = values.filter((value) => value !== null);
  const value = name === 'min' ? Math.min(...given) : name === 'max' ? Math.max(...given) : Math.max(values[0] ?? -Infinity, Math.min(values[1], values[2] ?? Infinity));
  return calcText([[unit, value]]);
}
/** `abs()` / `sign()` of a sum Chrome resolves: its `calc()` value (`sign()` a number). */
function absSign(name, inner) {
  const term = resolved(sumTerms(inner));
  if (!term) return null;
  // `sign()` keeps a (signed) zero and NaN.
  return name === 'abs' ? calcText([[term[0], Math.abs(term[1])]]) : calcText([['', term[1] === 0 || Number.isNaN(term[1]) ? term[1] : Math.sign(term[1])]]);
}
// ------------------------------------------------------ shape() (#338) ----
// Lockstep port of rtdom/css_shorthand/shape.rs: `shape(<fill-rule>? from <position>,
// <shape-command>#)` with Chrome 156's spelling (the default `nonzero` rule, `from origin` /
// `from start` anchors, `ccw` / `small` / `rotate 0deg` arc flags left out).
function shapePositionText(comps) {
  if (comps.length === 3) return null;
  const point = position(comps);
  return point && `${point[0]} ${point[1]}`;
}
function coordinatePair(comps) {
  if (comps.length !== 2) return null;
  const [x, y] = comps.map((c) => lengthPercentage(c, true));
  return x === null || y === null ? null : `${x} ${y}`;
}
/** A control point: a position (`to` commands) or a pair anchored `from start | end | origin`. */
function controlPoint(comps, absolute) {
  if (comps.length === 4) {
    const anchor = isIdent(comps[2], ['from']) ? keyword(comps[3], ['start', 'end', 'origin']) : null;
    const pair = anchor && coordinatePair(comps.slice(0, 2));
    if (!pair) return null;
    return anchor === (absolute ? 'origin' : 'start') ? pair : `${pair} from ${anchor}`;
  }
  return absolute ? shapePositionText(comps) : coordinatePair(comps);
}
/** `to <position>` or `by <coordinate-pair>`: [text, absolute], or null. */
function endPoint(comps) {
  const way = comps.length ? keyword(comps[0], ['to', 'by']) : null;
  const text = way === 'to' ? shapePositionText(comps.slice(1)) : way === 'by' ? coordinatePair(comps.slice(1)) : null;
  return text === null ? null : [`${way} ${text}`, way === 'to'];
}
/** `with <control-point> [/ <control-point>]?`. */
function shapeControls(comps, absolute, two) {
  // splitEnd cut the end point at the `with`.
  if (!comps.length) return null;
  const points = split(comps.slice(1), isSlash);
  if (points.length > (two ? 2 : 1)) return null;
  const texts = points.map((point) => controlPoint(point, absolute));
  return texts.includes(null) ? null : `with ${texts.join(' / ')}`;
}
/** The end point at the front of `comps` (up to a `stop` keyword) and the rest: [text, absolute, rest]. */
function splitEnd(comps, stop) {
  const at = comps.findIndex((comp, index) => index > 0 && isIdent(comp, stop));
  const end = endPoint(at < 0 ? comps : comps.slice(0, at));
  return end && [...end, at < 0 ? [] : comps.slice(at)];
}
function axisLine(comps, sides) {
  if (comps.length !== 2) return null;
  const way = keyword(comps[0], ['to', 'by']);
  const value = way === 'to' ? keyword(comps[1], sides) ?? lengthPercentage(comps[1], true) : way === 'by' ? lengthPercentage(comps[1], true) : null;
  return value === null ? null : `${way} ${value}`;
}
/** `of <length-percentage>{1,2}` at the front of `rest`: the components it used, or 0. */
function arcRadii(rest, parts) {
  const values = [];
  for (const comp of rest.slice(1, 3)) { const value = lengthPercentage(comp, true); if (value === null) break; values.push(value); }
  parts.radii = values.join(' ');
  return values.length && values.length + 1;
}
/** One arc part at the front of `rest` into `parts`: the components it used, or 0. */
function arcPart(rest, parts) {
  const word = ident(rest[0]);
  if (word === 'of' && parts.radii === undefined) return arcRadii(rest, parts);
  const flag = (word === 'cw' || word === 'ccw') ? 'sweep' : (word === 'large' || word === 'small') ? 'size' : null;
  if (flag && parts[flag] === undefined) { parts[flag] = word; return 1; }
  if (word === 'rotate' && parts.rotate === undefined && rest[1] && angleStrict(rest[1]) !== null) { parts.rotate = rest[1]; return 2; }
  return 0;
}
/** `arc <end> [of <lp>{1,2}] && [cw | ccw]? && [large | small]? && [rotate <angle>]?`. */
function arcCommand(comps) {
  const end = splitEnd(comps, ['of', 'cw', 'ccw', 'large', 'small', 'rotate']);
  if (!end) return null;
  const parts = {};
  for (let rest = end[2]; rest.length;) { const used = arcPart(rest, parts); if (!used) return null; rest = rest.slice(used); }
  if (!parts.radii) return null;
  let out = `${end[0]} of ${parts.radii}`;
  if (parts.sweep === 'cw') out += ' cw';
  if (parts.size === 'large') out += ' large';
  const rotate = parts.rotate;
  return rotate && !(rotate.kind === 'dimension' && rotate.value === 0) ? `${out} rotate ${rotate.text}` : out;
}
/** `curve` / `smooth`: the end point, then the control point(s) (`smooth`'s optional). */
function curveCommand(rest, curve) {
  const end = splitEnd(rest, ['with']);
  if (!end) return null;
  if (!curve && !end[2].length) return end[0];
  const controls = shapeControls(end[2], end[1], curve);
  return controls === null ? null : `${end[0]} ${controls}`;
}
const SHAPE_COMMANDS = {
  close: (rest) => (rest.length ? null : ''),
  move: (rest) => { const end = endPoint(rest); return end && end[0]; },
  hline: (rest) => axisLine(rest, ['left', 'center', 'right', 'x-start', 'x-end']),
  vline: (rest) => axisLine(rest, ['top', 'center', 'bottom', 'y-start', 'y-end']),
  curve: (rest) => curveCommand(rest, true),
  smooth: (rest) => curveCommand(rest, false),
  arc: arcCommand,
};
SHAPE_COMMANDS.line = SHAPE_COMMANDS.move;
/** One `<shape-command>`. */
function shapeCommand(comps) {
  const name = comps.length ? ident(comps[0]) : null;
  const parse = name !== null && Object.hasOwn(SHAPE_COMMANDS, name) ? SHAPE_COMMANDS[name] : null;
  const text = parse && parse(comps.slice(1));
  if (text === null || text === undefined) return null;
  return text ? `${name} ${text}` : name;
}
/** A `shape()` function in Chrome's spelling, or null when Chrome rejects it. */
function shapeText(comp) {
  const items = split(innerOf(comp), isComma);
  // One trailing comma is dropped.
  if (items.length > 2 && !items[items.length - 1].length) items.pop();
  let head = items[0];
  const rule = head.length && isIdent(head[0], ['nonzero', 'evenodd']) ? ident(head[0]) : null;
  if (rule) head = head.slice(1);
  const from = head.length && isIdent(head[0], ['from']) ? shapePositionText(head.slice(1)) : null;
  if (from === null || items.length < 2) return null;
  const parts = [`from ${from}`];
  for (const item of items.slice(1)) { const command = shapeCommand(item); if (command === null) return null; parts.push(command); }
  return `shape(${rule === 'evenodd' ? 'evenodd ' : ''}${parts.join(', ')})`;
}
/** `shape() || <box>` (`fallback` left out), or null when `comps` hold no valid shape(). */
function shapeWithBox(comps, boxes, fallback) {
  const isShape = (c) => functionNamed(c, ['shape']) !== null;
  let shape = null, box = null;
  if (comps.length === 1) [shape] = comps;
  else if (comps.length === 2) [shape, box] = isShape(comps[0]) ? comps : [comps[1], comps[0]];
  if (!shape || !isShape(shape)) return null;
  const text = shapeText(shape);
  const name = box && keyword(box, boxes);
  if (text === null || (box && name === null)) return null;
  return name && name !== fallback ? `${text} ${name}` : text;
}
// ----------------------------------------------- tree-counting math (#338) ----
// Math expressions Chrome keeps unresolved at parse time because an operand is only known at
// computed-value time: the tree-counting functions `sibling-index()` / `sibling-count()`.
// Lockstep port of rtdom/css_shorthand/math.rs: literal factors of a product multiply into one
// leading coefficient, literal terms of a sum add up in front, nested sums and products
// flatten, a product in a sum (or a sum in a product) is parenthesized.
/** Whether `comps` hold a tree-counting function, nested ones included. */
const hasTreeCounting = (comps) => comps.some((comp) => comp.kind === 'function' && (comp.text.includes('sibling-index()') || comp.text.includes('sibling-count()')));
/** Add like-unit `terms` into `into`, negated when `negative`. */
function addTerms(into, terms, negative) {
  for (const [unit, value] of terms) {
    const existing = into.find((term) => term[0] === unit);
    if (existing) existing[1] += negative ? -value : value;
    else into.push([unit, negative ? -value : value]);
  }
}
function treeOperand(comp) {
  const terms = literalTerms(comp);
  if (terms) return { kind: 'literal', terms: canonicalTerms(terms) };
  if (comp.kind === 'function' && comp.name === '(') return treeSum(components(comp.text.slice(1, -1)));
  // A nested math function Chrome simplified already spells `calc(...)`.
  if (comp.text.startsWith('calc(')) return treeSum(components(comp.text.slice(5, -1)));
  return { kind: 'function', text: comp.text };
}
/** A checked `calc()` sum: operands at even positions, operators between them. */
function treeSum(comps) {
  const literals = [], rest = [];
  let start = 0, negative = false;
  for (let index = 1; index <= comps.length; index += 2) {
    const next = comps[index];
    if (next && !(next.kind === 'delim' && (next.value === '+' || next.value === '-'))) continue;
    const expr = treeProduct(comps.slice(start, index));
    if (expr.kind === 'literal') addTerms(literals, expr.terms, negative);
    else if (expr.kind === 'sum') { addTerms(literals, expr.literals, negative); for (const [minus, term] of expr.rest) rest.push([minus !== negative, term]); }
    else rest.push([negative, expr]);
    negative = !!next && next.value === '-';
    start = index + 1;
  }
  if (!rest.length) return { kind: 'literal', terms: literals };
  if (!literals.length && rest.length === 1 && !rest[0][0]) return rest[0][1];
  return { kind: 'sum', literals, rest };
}
/** Fold one product operand into the coefficient and factors. */
function treeFactor(coefficient, factors, expr, divisor) {
  if (expr.kind === 'literal' && expr.terms.length === 1) {
    const [unit, value] = expr.terms[0];
    if (divisor) coefficient[1] /= value;
    else { coefficient[1] *= value; if (unit) coefficient[0] = unit; }
  } else if (expr.kind === 'product' && !divisor) {
    coefficient[1] *= expr.coefficient[1];
    if (expr.coefficient[0]) coefficient[0] = expr.coefficient[0];
    factors.push(...expr.factors);
  } else factors.push([divisor, expr]);
}
/** A checked product: operands at even positions, `*` / `/` between them. */
function treeProduct(comps) {
  const coefficient = ['', 1], factors = [];
  for (let index = 0; index < comps.length; index += 2) {
    const divisor = index > 0 && (comps[index - 1].kind === 'slash' || comps[index - 1].value === '/');
    treeFactor(coefficient, factors, treeOperand(comps[index]), divisor);
  }
  if (!factors.length) return { kind: 'literal', terms: [coefficient] };
  if (coefficient[0] === '' && coefficient[1] === 1 && factors.length === 1 && !factors[0][0]) return factors[0][1];
  return { kind: 'product', coefficient, factors };
}
const literalText = (terms) => calcText(terms.map((term) => [...term])).slice(5, -1);
/** `calc()`'s text without the wrapper; `(...)` around a nested sum or product. */
function treeText(expr, grouped) {
  if (expr.kind === 'function') return expr.text;
  let body;
  if (expr.kind === 'literal') body = literalText(expr.terms);
  else if (expr.kind === 'sum') {
    body = expr.literals.length ? literalText(expr.literals) : '';
    // The first term is never subtracted.
    for (const [minus, term] of expr.rest) body += (body ? (minus ? ' - ' : ' + ') : '') + treeText(term, true);
  } else {
    const [unit, value] = expr.coefficient;
    // A unitless 1 is left out before a multiplied factor.
    body = unit === '' && value === 1 && !expr.factors[0][0] ? '' : literalText([[unit, value]]);
    for (const [divisor, factor] of expr.factors) body += (body ? (divisor ? ' / ' : ' * ') : '') + treeText(factor, true);
  }
  return grouped ? `(${body})` : body;
}
/** `calc(<sum>)` holding a tree-counting function: a lone function without the wrapper. */
function treeCalcText(inner) {
  const expr = treeSum(inner);
  return expr.kind === 'function' ? expr.text : `calc(${treeText(expr, false)})`;
}
const treeArgumentText = (argument) => treeText(treeSum(argument), false);
/**
 * The arguments of a math function Chrome keeps: each a simplified sum without the `calc()`
 * wrapper (`min(1px, calc(1px + 1em))` is `min(1px, 1em + 1px)`), or as written.
 */
function sumArguments(inner) {
  return argumentsOf(inner).map((argument) => {
    const terms = isNone(argument) ? null : sumTerms(argument);
    if (!terms && hasTreeCounting(argument)) return treeArgumentText(argument);
    return terms ? calcText(terms).slice(5, -1) : join(argument);
  }).join(', ');
}
/** Blink's alpha spelling: the shortest of two or three decimals naming the same 8-bit alpha. */
function alphaText(alpha) {
  const a = Math.min(1, Math.max(0, alpha));
  const byte = Math.round(a * 255);
  const two = Math.round(a * 100) / 100;
  return cssNumber(Math.round(two * 255) === byte ? two : Math.round(a * 1000) / 1000);
}
function rgbText(channels, alpha) {
  // Channels computed from hsl() land a hair off a half: round at 1e-9 first.
  const [r, g, b] = channels.map((c) => Math.min(255, Math.max(0, Math.round(Math.round(c * 1e9) / 1e9))));
  return alpha >= 1 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${alphaText(alpha)})`;
}
/** `#rgb[a]` / `#rrggbb[aa]` as Chrome serializes them. */
function hexColor(hex) {
  if (!/^[0-9a-fA-F]+$/.test(hex)) return null;
  const digits = [...hex].map((c) => parseInt(c, 16));
  let pairs;
  if (digits.length === 3 || digits.length === 4) pairs = digits.map((d) => d * 17);
  else if (digits.length === 6 || digits.length === 8) { pairs = []; for (let i = 0; i < digits.length; i += 2) pairs.push(digits[i] * 16 + digits[i + 1]); }
  else return null;
  return rgbText(pairs.slice(0, 3), pairs.length > 3 ? pairs[3] / 255 : 1);
}
/** `rgb()` / `hsl()` in the legacy (commas) or modern (spaces, `/ alpha`) syntax. */
function colorFunction(name, inner) {
  // functionValid admitted three channels and an optional alpha, or the relative syntax (kept as written).
  const values = inner.filter((c) => c.kind !== 'comma' && c.kind !== 'slash');
  if (values.length > 4) return null;
  const parts = values.slice(0, 3), alphaPart = values[3] ?? null;
  // A channel: a literal, `none` (zero) or a math function that computes to one.
  const computed = (c, unit) => {
    if (isIdent(c, ['none'])) return 0;
    const term = mathValue(c);
    return term && term[0] === unit ? term[1] : null;
  };
  const num = (c) => (c.kind === 'number' ? c.value : computed(c, ''));
  const pct = (c) => (c.kind === 'percentage' ? c.value : computed(c, '%'));
  let alpha = 1;
  if (alphaPart !== null) { const n = num(alphaPart); const p = pct(alphaPart); if (n !== null) alpha = n; else if (p !== null) alpha = p / 100; else return null; }
  let channels;
  if (name.startsWith('rgb')) {
    channels = parts.map((c) => { const n = num(c); if (n !== null) return n; const p = pct(c); return p === null ? null : p * 2.55; });
    if (channels.some((c) => c === null)) return null;
  } else {
    const h = parts[0];
    let hue;
    const turns = { deg: 1, grad: 0.9, rad: 180 / Math.PI, turn: 360 };
    // functionValid admitted a number, an angle, `none` or a math function hue.
    if (h.kind === 'number') hue = h.value; else if (h.kind === 'dimension') hue = h.value * turns[h.unit]; else hue = computed(h, '') ?? computed(h, 'deg');
    if (hue === null) return null;
    const sat = pct(parts[1]), light = pct(parts[2]);
    if (sat === null || light === null) return null;
    const S = sat / 100, L = light / 100, H = (((hue % 360) + 360) % 360) / 360;
    const q = L < 0.5 ? L * (1 + S) : L + S - L * S, p = 2 * L - q;
    const channel = (t) => { t = ((t % 1) + 1) % 1; return 255 * (t < 1 / 6 ? p + (q - p) * 6 * t : t < 0.5 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p); };
    channels = [channel(H + 1 / 3), channel(H), channel(H - 1 / 3)];
  }
  return rgbText(channels, alpha);
}
/** `lab()` / `lch()` / `oklab()` / `oklch()`: a percentage lightness spelled as a number. */
function labFunction(name, inner) {
  return `${name}(${inner.map((c, index) => (index === 0 && c.kind === 'percentage' ? cssNumber(c.value / (name.startsWith('ok') ? 100 : 1)) : c.text)).join(' ')})`;
}
/** `steps(n, end | jump-end)` is `steps(n)`; the position keyword lowercase. */
function stepsText(inner) {
  if (inner.length !== 3 || inner[1].kind !== 'comma' || ident(inner[2]) === null) return null;
  const position = ident(inner[2]);
  return position === 'end' || position === 'jump-end' ? `steps(${inner[0].text})` : `steps(${inner[0].text}, ${position})`;
}
/** `linear()` with every stop's input position filled in. */
function linearText(inner) {
  const stops = [];
  for (const item of split(inner, isComma)) {
    // functionValid admitted a number and at most two percentages per stop.
    const positions = item.slice(1).map((c) => c.value);
    if (!positions.length) stops.push([item[0].text, [null]]);
    // A stop with two input positions is two stops.
    else for (const position of positions) stops.push([item[0].text, [position]]);
  }
  const flat = stops.flatMap(([, positions]) => positions);
  if (flat[0] === null) flat[0] = 0;
  if (flat[flat.length - 1] === null) flat[flat.length - 1] = 100;
  // A position below the largest before it is that largest.
  let largest = -Infinity;
  for (let at = 0; at < flat.length; at++) if (flat[at] !== null) { largest = Math.max(largest, flat[at]); flat[at] = largest; }
  for (let at = 0; at < flat.length; at++) {
    if (flat[at] !== null) continue;
    let end = at;
    while (flat[end] === null) end++;
    const low = flat[at - 1], high = flat[end], steps = end - at + 1;
    for (let index = at; index < end; index++) flat[index] = low + ((high - low) * (index - at + 1)) / steps;
    at = end;
  }
  let index = 0;
  return `linear(${stops.map(([output, positions]) => `${output} ${positions.map(() => `${cssNumber(flat[index++])}%`).join(' ')}`).join(', ')})`;
}

/** Functions whose identifier arguments are names (kept as written). */
const NAMING_ARGUMENTS = new Set(['stylistic', 'styleset', 'character-variant', 'swash', 'ornaments', 'annotation', 'local', 'format', 'attr', 'counter', 'counters', 'symbols', 'src', 'var', 'env']);
/** A math function Chrome simplified to one term (its text spells `calc(...)`): `[unit, value]`, else null. */
function mathValue(comp) {
  if (!isMath(comp) || !comp.text.startsWith('calc(')) return null;
  const terms = sumTerms(components(comp.text.slice(5, -1)));
  return terms && terms.length === 1 ? terms[0] : null;
}
function readSimpleComponent(token) {
  switch (token.type) {
    case 'ident': return { kind: 'ident', value: token.value, text: token.value };
    case 'number': return { kind: 'number', value: f32(token.value), integer: token.integer, text: cssNumber(token.value) };
    case 'percentage': { const value = f32(Number(token.raw)); return { kind: 'percentage', value, text: `${cssNumber(Number(token.raw))}%` }; }
    case 'dimension': {
      // A length past f32's range (`1e39px`) is the largest float, as Chrome clamps it
      // (`3.40282e+38px`), never infinite (#262; the Rust port's tokenizer likewise).
      const number = Math.max(-F32_MAX, Math.min(F32_MAX, token.value));
      const unit = token.unit.toLowerCase();
      return { kind: 'dimension', value: f32(number), unit, text: `${cssNumber(number)}${unit}` };
    }
    case 'hash': return { kind: 'hash', value: token.value, text: `#${token.value}` };
    case 'string': return { kind: 'string', value: token.value, text: quoted(token.value) };
    case 'url': return { kind: 'url', value: token.value, text: `url(${quoted(token.value)})` };
    case 'comma': return { kind: 'comma', text: ',' };
    default: return null;
  }
}
function readDelim(token) {
  if (token.value === '!') return null;
  return token.value === '/' ? { kind: 'slash', text: '/' } : { kind: 'delim', value: token.value, text: token.value };
}
function simplifyMathLiterals(name, inner) {
  // Keyword arguments are case-insensitive: Chrome writes them lowercase.
  if (!NAMING_ARGUMENTS.has(name)) for (const comp of inner) if (comp.kind === 'ident' && !comp.value.startsWith('--')) comp.text = comp.value.toLowerCase();
  if (name !== 'cubic-bezier' && name !== 'linear') return;
  inner.forEach((comp, index) => {
    const term = mathValue(comp);
    if (term && term[0] === '') inner[index] = { kind: 'number', value: f32(term[1]), integer: false, text: cssNumber(term[1]) };
    else if (term && term[0] === '%' && name === 'linear') inner[index] = { kind: 'percentage', value: f32(term[1]), text: `${cssNumber(term[1])}%` };
  });
}
// Null prototype: a function named after an Object.prototype member (`constructor(…)`,
// `__proto__(…)`) is an ordinary unknown function, never a handler.
const SPECIAL_FUNCTION_HANDLERS = {
  __proto__: null,
  calc: (name, inner) => calcSum(inner),
  min: (name, inner) => minMax(name, inner),
  max: (name, inner) => minMax(name, inner),
  clamp: (name, inner) => minMax(name, inner),
  abs: (name, inner) => absSign(name, inner),
  sign: (name, inner) => absSign(name, inner),
  rgb: colorFunction,
  rgba: colorFunction,
  hsl: colorFunction,
  hsla: colorFunction,
  lab: labFunction,
  lch: labFunction,
  oklab: labFunction,
  oklch: labFunction,
  steps: (_n, inner) => stepsText(inner),
  linear: (_n, inner) => linearText(inner),
};
function specialFunctionText(name, inner) {
  const handler = SPECIAL_FUNCTION_HANDLERS[name];
  return handler ? handler(name, inner) : null;
}
function readFunctionComponent(name, inner) {
  if (name === 'url') {
    if (inner.length !== 1 || inner[0].kind !== 'string') return null;
    return { kind: 'url', value: inner[0].value, text: `url(${quoted(inner[0].value)})` };
  }
  simplifyMathLiterals(name, inner);
  if (!functionValid(name, inner)) return null;
  const special = specialFunctionText(name, inner);
  const text = special ?? `${name}(${SUM_FUNCTIONS.includes(name) ? sumArguments(inner) : join(inner)})`;
  return { kind: 'function', name, text };
}

/** Tokens -> components (a function / block is one component); `null` on bad input. */
function read(tokens, state, close) {
  const out = [];
  while (state.at < tokens.length) {
    const token = tokens[state.at++];
    if (token.type === 'ws') continue;
    if (token.type === 'delim') { const comp = readDelim(token); if (!comp) return null; out.push(comp); continue; }
    if (token.type === 'function') {
      const name = token.name.toLowerCase();
      const inner = read(tokens, state, ')');
      if (inner === null) return null;
      const comp = readFunctionComponent(name, inner);
      if (!comp) return null;
      out.push(comp);
      continue;
    }
    if (token.type === 'open') {
      if (token.value === '{') return null;
      const closer = token.value === '(' ? ')' : ']';
      const inner = read(tokens, state, closer);
      if (inner === null) return null;
      out.push({ kind: 'function', name: token.value, text: `${token.value}${join(inner)}${closer}` });
      continue;
    }
    if (token.type === 'close') return token.value === close ? out : null;
    const simple = readSimpleComponent(token);
    if (!simple) return null;
    out.push(simple);
  }
  return out;
}

/** Tokenize a declaration value into components; `null` for unbalanced or bad input. */
function components(value) {
  const tokens = tokenize(value);
  if (tokens === null) return null;
  return read(tokens, { at: 0 }, null);
}

// Component predicates: the canonical text when the component is of the type, else null.
const ident = (c) => (c.kind === 'ident' ? c.value.toLowerCase() : null);
const isIdent = (c, names) => { const name = ident(c); return name !== null && names.includes(name); };
const keyword = (c, names) => { const name = ident(c); return name !== null && names.includes(name) ? name : null; };
const isComma = (c) => c.kind === 'comma';
const isSlash = (c) => c.kind === 'slash';
const innerOf = (c) => components(c.text.slice(c.text.search(/[([]/) + 1, -1));
function visit(comps, callback) {
  for (const comp of comps) {
    callback(comp);
    if (comp.kind === 'function') { const inner = innerOf(comp); if (inner) visit(inner, callback); }
  }
}
function mathOf(c, accepts, percent, number) {
  if (c.kind !== 'function' || !MATH_FUNCTIONS.has(c.name)) return false;
  const inner = innerOf(c);
  if (!inner) return false;
  let found = false, bad = false;
  visit(inner, (comp) => {
    if (comp.kind === 'dimension') { if (accepts(comp.unit)) found = true; else bad = true; }
    else if (comp.kind === 'percentage') { if (percent) found = true; else bad = true; }
  });
  return !bad && (found || number);
}
const isLengthUnit = (unit) => LENGTH_UNITS.has(unit);
function length(c, negative) {
  if (c.kind === 'dimension' && LENGTH_UNITS.has(c.unit)) return negative || c.value >= 0 ? c.text : null;
  if (c.kind === 'number' && c.value === 0) return '0px';
  return mathOf(c, isLengthUnit, false, false) ? c.text : null;
}
const percentage = (c, negative) => (c.kind === 'percentage' && (negative || c.value >= 0) ? c.text : null);
function lengthPercentage(c, negative) {
  if (mathOf(c, isLengthUnit, true, false)) return c.text;
  return length(c, negative) ?? percentage(c, negative);
}
function number(c, negative) {
  if (c.kind === 'number') return negative || c.value >= 0 ? c.text : null;
  return mathOf(c, () => false, false, true) ? c.text : null;
}
const integer = (c, minimum) => (c.kind === 'number' ? (c.integer && c.value >= minimum ? c.text : null) : mathOf(c, () => false, false, true) ? c.text : null);
function time(c, negative) {
  if (c.kind === 'dimension' && (c.unit === 's' || c.unit === 'ms')) return negative || c.value >= 0 ? c.text : null;
  return mathOf(c, (unit) => unit === 's' || unit === 'ms', false, false) ? c.text : null;
}
const ANGLES = ['deg', 'grad', 'rad', 'turn'];
function angle(c) {
  if (c.kind === 'dimension' && ANGLES.includes(c.unit)) return c.text;
  if (c.kind === 'number' && c.value === 0) return '0deg';
  return mathOf(c, (unit) => ANGLES.includes(unit), false, false) ? c.text : null;
}
/** `<angle>` without the unitless zero (`offset-rotate`). */
const angleStrict = (c) => (c.kind === 'number' ? null : angle(c));
function color(c) {
  if (c.kind === 'hash') return hexColor(c.value);
  if (c.kind === 'ident') {
    const name = c.value.toLowerCase();
    return SYSTEM_COLORS.has(name) || canonicalizeColor(name, true) !== null ? name : null;
  }
  if (c.kind === 'function' && COLOR_FUNCTIONS.has(c.name)) return c.text;
  return null;
}
const url = (c) => (c.kind === 'url' ? c.text : null);
const image = (c) => url(c) ?? (c.kind === 'function' && IMAGE_FUNCTIONS.has(c.name) ? c.text : null);
const string = (c) => (c.kind === 'string' ? c.text : null);
function customIdent(c, excluded) {
  if (c.kind !== 'ident') return null;
  const lower = c.value.toLowerCase();
  return !CSS_WIDE.has(lower) && lower !== 'default' && !excluded.includes(lower) ? c.value : null;
}
const dashedIdent = (c) => (c.kind === 'ident' && c.value.startsWith('--') && c.value.length > 2 ? c.value : null);
const functionNamed = (c, names) => (c.kind === 'function' && names.includes(c.name) ? c.text : null);

/** Whether the raw value holds a var() / env() / attr() / if() / inherit() reference. */
function hasSubstitution(value) {
  const tokens = tokenize(value);
  if (!tokens) return false;
  return tokens.some((t) => t.type === 'function' && ['var', 'env', 'attr', 'if', 'inherit'].includes(t.name.toLowerCase()));
}
const substitutionText = (value) => value.trim().replace(/\s+/g, ' ');

// ---------------------------------------------------------------- grammars ----
const INITIAL = 'initial';
const marginV = (c) => lengthPercentage(c, true) ?? keyword(c, ['auto']);
const paddingV = (c) => lengthPercentage(c, false);
const scrollMarginV = (c) => length(c, true);
const scrollPaddingV = (c) => lengthPercentage(c, false) ?? keyword(c, ['auto']);
const lineWidth = (c) => length(c, false) ?? keyword(c, ['thin', 'medium', 'thick']);
const LINE_STYLES = ['none', 'hidden', 'dotted', 'dashed', 'solid', 'double', 'groove', 'ridge', 'inset', 'outset'];
const lineStyle = (c) => keyword(c, LINE_STYLES);
const cornerShape = (c) => keyword(c, ['round', 'scoop', 'bevel', 'notch', 'square', 'squircle']) ?? functionNamed(c, ['superellipse']);
const radius = (c) => lengthPercentage(c, false);
const gapV = (c) => lengthPercentage(c, false) ?? keyword(c, ['normal']);
const overflowV = (c) => keyword(c, ['visible', 'hidden', 'clip', 'scroll', 'auto', 'overlay']);
const overscrollV = (c) => keyword(c, ['auto', 'contain', 'none', 'chain']);
const spacingV = (c) => length(c, false);
const delayV = (c) => keyword(c, ['normal']) ?? time(c, false);
const ruleInsetV = (c) => lengthPercentage(c, true) ?? keyword(c, ['overlap-join']);
const ruleBreakV = (c) => keyword(c, ['none', 'normal', 'intersection']);
const ruleVisibilityV = (c) => keyword(c, ['all', 'normal', 'around', 'between']);
const markerV = (c) => keyword(c, ['none']) ?? url(c);

const single = (comps, ty) => (comps.length === 1 ? ty(comps[0]) : null);
function list(comps, ty, max) {
  if (comps.length === 0 || comps.length > max) return null;
  const out = [];
  for (const comp of comps) { const value = ty(comp); if (value === null) return null; out.push(value); }
  return out;
}
function box4(comps, ty) {
  const v = list(comps, ty, 4);
  if (!v) return null;
  return v.length === 1 ? [v[0], v[0], v[0], v[0]] : v.length === 2 ? [v[0], v[1], v[0], v[1]] : v.length === 3 ? [v[0], v[1], v[2], v[1]] : v;
}
function pair(comps, ty) {
  const v = list(comps, ty, 2);
  return v ? [v[0], v[1] ?? v[0]] : null;
}
const named = (names, values) => (values ? names.map((name, index) => [name, values[index]]) : null);
const same = (names, value) => (value === null ? null : names.map((name) => [name, value]));
const one = (ty) => (comps) => { if (!comps.length) return null; const value = ty(comps[0]); return value === null ? null : [1, value]; };
/** `a || b || ...`: each slot at most once, in any order; earlier slots win a component. */
function anyOrder(comps, slots) {
  const out = slots.map(() => null);
  let at = 0;
  while (at < comps.length) {
    let matched = false;
    for (let index = 0; index < slots.length; index++) {
      if (out[index] !== null) continue;
      const result = slots[index](comps.slice(at));
      if (result) { out[index] = result[1]; at += result[0]; matched = true; break; }
    }
    if (!matched) return null;
  }
  return out;
}
function split(comps, test) {
  const items = [[]];
  for (const comp of comps) { if (test(comp)) items.push([]); else items[items.length - 1].push(comp); }
  return items;
}
const commas = (comps) => { const items = split(comps, isComma); return items.every((item) => item.length) ? items : null; };
const slashes = (comps) => split(comps, isSlash);
const namesOrInitial = (names, parsed) => names.map((name, index) => [name, parsed[index] ?? INITIAL]);
const BORDER_IMAGE_INITIAL = ['none', '100%', '1', '0', 'stretch'];

const outlineStyle = (c) => keyword(c, ['auto']) ?? lineStyle(c);
function line(comps, defaults) {
  const parsed = anyOrder(comps, [one(lineWidth), one(lineStyle), one(color)]);
  return parsed && parsed.map((value, index) => value ?? defaults[index]);
}
function side(comps, name, defaults) {
  const parts = line(comps, defaults);
  return parts && [[`${name}-width`, parts[0]], [`${name}-style`, parts[1]], [`${name}-color`, parts[2]]];
}

function borderRadius(comps, legacy) {
  const parts = slashes(comps);
  let horizontal, vertical;
  if (parts.length === 1 && legacy && parts[0].length === 2) {
    const a = radius(parts[0][0]), b = radius(parts[0][1]);
    if (a === null || b === null) return null;
    horizontal = [a, a, a, a]; vertical = [b, b, b, b];
  } else if (parts.length === 1) {
    horizontal = box4(parts[0], radius); vertical = horizontal;
  } else if (parts.length === 2) {
    horizontal = box4(parts[0], radius); vertical = box4(parts[1], radius);
  } else return null;
  if (!horizontal || !vertical) return null;
  return ['border-top-left-radius', 'border-top-right-radius', 'border-bottom-right-radius', 'border-bottom-left-radius']
    .map((name, index) => [name, horizontal[index] === vertical[index] ? horizontal[index] : `${horizontal[index]} ${vertical[index]}`]);
}

function ruleInset(comps, kind) {
  const parts = slashes(comps);
  let cap, junction;
  if (parts.length === 1) { cap = pair(parts[0], ruleInsetV); junction = cap; }
  else if (parts.length === 2) { cap = pair(parts[0], ruleInsetV); junction = pair(parts[1], ruleInsetV); }
  else return null;
  if (!cap || !junction) return null;
  return [[`${kind}-rule-inset-cap-start`, cap[0]], [`${kind}-rule-inset-cap-end`, cap[1]], [`${kind}-rule-inset-junction-start`, junction[0]], [`${kind}-rule-inset-junction-end`, junction[1]]];
}

function containIntrinsicSize(comps) {
  const axis = (comps) => {
    if (comps.length >= 2 && isIdent(comps[0], ['auto'])) {
      const value = length(comps[1], false) ?? keyword(comps[1], ['none']);
      return value === null ? null : [2, `auto ${value}`];
    }
    if (!comps.length) return null;
    const value = length(comps[0], false) ?? keyword(comps[0], ['none']);
    return value === null ? null : [1, value];
  };
  const first = axis(comps);
  if (!first) return null;
  let height = first[1];
  const rest = comps.slice(first[0]);
  // A trailing lone `auto` after `auto <length>` repeats the width (Chrome).
  if (rest.length && !(rest.length === 1 && isIdent(rest[0], ['auto']) && first[1].startsWith('auto '))) {
    const second = axis(rest);
    if (!second || first[0] + second[0] !== comps.length) return null;
    height = second[1];
  }
  return [['contain-intrinsic-width', first[1]], ['contain-intrinsic-height', height]];
}

function baseline(comps) {
  if (comps.length >= 2 && isIdent(comps[0], ['first']) && isIdent(comps[1], ['baseline'])) return [2, 'baseline'];
  if (comps.length >= 2 && isIdent(comps[0], ['last']) && isIdent(comps[1], ['baseline'])) return [2, 'last baseline'];
  if (comps.length >= 1 && isIdent(comps[0], ['baseline'])) return [1, 'baseline'];
  return null;
}
function overflowPosition(comps, positions) {
  if (comps.length >= 2 && isIdent(comps[0], ['safe', 'unsafe'])) {
    const position = keyword(comps[1], positions);
    return position === null ? null : [2, `${ident(comps[0])} ${position}`];
  }
  if (!comps.length) return null;
  const value = keyword(comps[0], positions);
  return value === null ? null : [1, value];
}
const CONTENT_POSITIONS = ['center', 'start', 'end', 'flex-start', 'flex-end'];
const SELF_POSITIONS = ['center', 'start', 'end', 'self-start', 'self-end', 'flex-start', 'flex-end'];
const DISTRIBUTIONS = ['space-between', 'space-around', 'space-evenly', 'stretch'];
function firstKeyword(comps, names) { if (!comps.length) return null; const value = keyword(comps[0], names); return value === null ? null : [1, value]; }

function placeContent(comps) {
  const align = (comps) => baseline(comps) ?? firstKeyword(comps, ['normal', ...DISTRIBUTIONS]) ?? overflowPosition(comps, CONTENT_POSITIONS);
  const justify = (comps) => firstKeyword(comps, ['normal', 'left', 'right', ...DISTRIBUTIONS]) ?? overflowPosition(comps, ['center', 'start', 'end', 'flex-start', 'flex-end', 'left', 'right']);
  const a = align(comps);
  if (!a) return null;
  // `[first] baseline` takes a justify value (`start` when there is none); Chrome 156 rejects
  // `last baseline` here.
  if (a[1] === 'last baseline') return null;
  let j;
  if (a[0] === comps.length) j = a[1] === 'baseline' ? 'start' : a[1];
  else {
    const rest = justify(comps.slice(a[0]));
    if (!rest || a[0] + rest[0] !== comps.length) return null;
    j = rest[1];
  }
  return [['align-content', a[1]], ['justify-content', j]];
}
const selfAlignment = (comps, extra) => baseline(comps) ?? firstKeyword(comps, extra) ?? overflowPosition(comps, SELF_POSITIONS);
/** A `justify-items` value from the front of `comps`: `[used, text]`. */
function justifyItems(comps) {
  let legacy = null;
  if (comps.length === 1 && isIdent(comps[0], ['legacy'])) legacy = [1, 'legacy'];
  else if (comps.length === 2 && isIdent(comps[0], ['legacy'])) { const v = keyword(comps[1], ['left', 'right', 'center']); legacy = v === null ? null : [2, `legacy ${v}`]; }
  else if (comps.length === 2 && isIdent(comps[1], ['legacy'])) { const v = keyword(comps[0], ['left', 'right', 'center']); legacy = v === null ? null : [2, `legacy ${v}`]; }
  return legacy ?? selfAlignment(comps, ['normal', 'stretch', 'left', 'right', 'anchor-center']) ?? overflowPosition(comps, ['left', 'right']);
}
function placeItems(comps) {
  const a = selfAlignment(comps, ['normal', 'stretch']);
  if (!a) return null;
  let justify = a[1];
  if (a[0] !== comps.length) {
    const rest = comps.slice(a[0]);
    const result = justifyItems(rest);
    if (!result || a[0] + result[0] !== comps.length) return null;
    justify = result[1];
  }
  return [['align-items', a[1]], ['justify-items', justify]];
}
function placeSelf(comps) {
  const a = selfAlignment(comps, ['auto', 'normal', 'stretch', 'anchor-center']);
  if (!a) return null;
  let justify = a[1];
  if (a[0] !== comps.length) {
    const rest = comps.slice(a[0]);
    const result = selfAlignment(rest, ['auto', 'normal', 'stretch', 'anchor-center', 'left', 'right']) ?? overflowPosition(rest, ['left', 'right']);
    if (!result || a[0] + result[0] !== comps.length) return null;
    justify = result[1];
  }
  return [['align-self', a[1]], ['justify-self', justify]];
}

const POS_LP = (c) => lengthPercentage(c, true);
const POS_HORIZ = (c) => keyword(c, ['left', 'right']);
const POS_VERT = (c) => keyword(c, ['top', 'bottom']);
const POS_CENTER = (c) => keyword(c, ['center']);
function positionOne(a) {
  const v = POS_VERT(a);
  if (v !== null) return ['center', v];
  const x = POS_HORIZ(a) ?? POS_CENTER(a) ?? POS_LP(a);
  return x === null ? null : [x, 'center'];
}
function positionTwo(a, b) {
  const ax = POS_HORIZ(a) ?? POS_CENTER(a) ?? POS_LP(a), by = POS_VERT(b) ?? POS_CENTER(b) ?? POS_LP(b);
  if (ax !== null && by !== null) return [ax, by];
  const ay = POS_VERT(a) ?? POS_CENTER(a), bx = POS_HORIZ(b) ?? POS_CENTER(b);
  return ay !== null && bx !== null ? [bx, ay] : null;
}
function positionManyAssign(slots, kw, text) {
  if (kw === 'left' || kw === 'right') { if (slots.x !== null) return false; slots.x = text; return true; }
  if (kw === 'top' || kw === 'bottom') { if (slots.y !== null) return false; slots.y = text; return true; }
  if (slots.middle !== null) return false;
  slots.middle = text;
  return true;
}
function positionManyResolve(slots) {
  const { x, y, middle } = slots;
  if (x !== null && y !== null && middle === null) return [x, y];
  if (x !== null && y === null && middle !== null) return [x, middle];
  if (x === null && y !== null && middle !== null) return [middle, y];
  return null;
}
function positionMany(comps) {
  // Keyword + offset pairs; a lone keyword takes no offset; `center` takes the axis the
  // other keyword leaves.
  const slots = { x: null, y: null, middle: null };
  let at = 0;
  while (at < comps.length) {
    const kw = keyword(comps[at], ['left', 'right', 'top', 'bottom', 'center']);
    if (kw === null) return null;
    const offset = comps[at + 1] ? POS_LP(comps[at + 1]) : null;
    const used = offset !== null && kw !== 'center' ? 2 : 1;
    const text = used === 2 ? `${kw} ${offset}` : kw;
    if (!positionManyAssign(slots, kw, text)) return null;
    at += used;
  }
  return positionManyResolve(slots);
}
/** One `<bg-position>`: its x and y specified values. */
function position(comps) {
  if (comps.length === 1) return positionOne(comps[0]);
  if (comps.length === 2) return positionTwo(comps[0], comps[1]);
  if (comps.length === 3 || comps.length === 4) return positionMany(comps);
  return null;
}
/** A comma list of positions; `three`: three-value positions too (background only). */
function positions(comps, three) {
  const layers = commas(comps);
  if (!layers) return null;
  const xs = [], ys = [];
  for (const layer of layers) { if (!three && layer.length === 3) return null; const p = position(layer); if (!p) return null; xs.push(p[0]); ys.push(p[1]); }
  return [xs.join(', '), ys.join(', ')];
}

const RANGE_NAMES = ['cover', 'contain', 'entry', 'exit', 'entry-crossing', 'exit-crossing', 'scroll'];
function rangeValue(comps, initial, end) {
  if (!comps.length) return null;
  const first = comps[0];
  const kw = keyword(first, [initial]);
  if (kw !== null) return [1, kw, null];
  const name = keyword(first, RANGE_NAMES);
  if (name !== null) {
    const offset = comps[1] ? lengthPercentage(comps[1], true) : null;
    if (offset !== null) return [2, offset === (end ? '100%' : '0%') ? name : `${name} ${offset}`, name];
    return [1, name, name];
  }
  const value = lengthPercentage(first, true);
  return value === null ? null : [1, value, null];
}
function rangePair(comps, start, end, initial) {
  const first = rangeValue(comps, initial === 'auto' ? 'normal' : initial, false) ?? rangeValue(comps, initial, false);
  if (!first) return null;
  let endValue;
  if (first[0] === comps.length) endValue = first[2] ?? initial;
  else {
    const second = rangeValue(comps.slice(first[0]), initial, true);
    if (!second || first[0] + second[0] !== comps.length) return null;
    endValue = second[1];
  }
  return [[start, first[1]], [end, endValue]];
}

function textDecoration(comps) {
  const order = ['underline', 'overline', 'line-through', 'blink', 'spelling-error', 'grammar-error'];
  const lines = (comps) => {
    if (!comps.length) return null;
    if (isIdent(comps[0], ['none'])) return [1, 'none'];
    const seen = [];
    for (const comp of comps) { const name = keyword(comp, order); if (name === null || seen.includes(name)) break; seen.push(name); }
    if (!seen.length) return null;
    return [seen.length, order.filter((name) => seen.includes(name)).join(' ')];
  };
  const thickness = (c) => keyword(c, ['auto', 'from-font']) ?? lengthPercentage(c, true);
  const style = (c) => keyword(c, ['solid', 'double', 'dotted', 'dashed', 'wavy']);
  const parsed = anyOrder(comps, [lines, one(thickness), one(style), one(color)]);
  return parsed && namesOrInitial(['text-decoration-line', 'text-decoration-thickness', 'text-decoration-style', 'text-decoration-color'], parsed);
}

function textEmphasis(comps) {
  const style = (comps) => {
    if (!comps.length) return null;
    const first = comps[0];
    const text = string(first);
    if (text !== null) return [1, text];
    if (isIdent(first, ['none'])) return [1, 'none'];
    let fill = null, shape = null, used = 0;
    for (const comp of comps.slice(0, 2)) {
      if (fill === null) { const v = keyword(comp, ['filled', 'open']); if (v !== null) { fill = v; used++; continue; } }
      if (shape === null) { const v = keyword(comp, ['dot', 'circle', 'double-circle', 'triangle', 'sesame']); if (v !== null) { shape = v; used++; continue; } }
      break;
    }
    if (!used) return null;
    return [used, [fill, shape].filter((v) => v !== null).join(' ')];
  };
  const parsed = anyOrder(comps, [style, one(color)]);
  return parsed && namesOrInitial(['text-emphasis-style', 'text-emphasis-color'], parsed);
}

function listStyle(comps) {
  const pos = (c) => keyword(c, ['inside', 'outside']);
  const kind = (c) => string(c) ?? (() => { const v = customIdent(c, ['none']); return v === null ? null : v.toLowerCase(); })();
  let nones = 0;
  const rest = comps.filter((c) => { if (isIdent(c, ['none'])) { nones++; return false; } return true; });
  const parsed = anyOrder(rest, [one(pos), one(image), one(kind)]);
  if (!parsed) return null;
  if (nones === 1 && parsed[2] === null) parsed[2] = 'none';
  else if (nones === 1 && parsed[1] === null) parsed[1] = 'none';
  else if (nones === 2 && parsed[1] === null && parsed[2] === null) { parsed[1] = 'none'; parsed[2] = 'none'; }
  else if (nones !== 0) return null;
  return namesOrInitial(['list-style-position', 'list-style-image', 'list-style-type'], parsed);
}

function columnsMain(main) {
  if (!main.length || main.length > 2) return null;
  let width = null, count = null;
  for (const comp of main) {
    if (isIdent(comp, ['auto'])) continue;
    const w = width === null ? length(comp, false) : null;
    if (w !== null) { width = w; continue; }
    const n = count === null ? integer(comp, 1) : null;
    if (n !== null) { count = n; continue; }
    return null;
  }
  return { width, count };
}
function columns(comps) {
  const parts = slashes(comps);
  let main, height = null;
  if (parts.length === 1) main = parts[0];
  else if (parts.length === 2 && parts[1].length === 1) {
    main = parts[0];
    height = length(parts[1][0], false) ?? keyword(parts[1][0], ['auto']);
    if (height === null) return null;
  } else return null;
  const parsed = columnsMain(main);
  if (!parsed) return null;
  return [['column-width', parsed.width ?? 'auto'], ['column-count', parsed.count ?? 'auto'], ['column-height', height ?? 'auto'], ['column-wrap', 'auto']];
}

function container(comps) {
  const parts = slashes(comps);
  let names, kind = null;
  if (parts.length === 1) names = parts[0];
  else if (parts.length === 2 && parts[1].length) { names = parts[0]; kind = parts[1]; }
  else return null;
  let name;
  if (names.length === 1) {
    if (isIdent(names[0], ['none'])) name = 'none';
    else { name = customIdent(names[0], ['none', 'and', 'or', 'not']); if (name === null) return null; }
  } else {
    const list = names.map((c) => customIdent(c, ['none', 'and', 'or', 'not']));
    if (list.some((v) => v === null)) return null;
    name = list.join(' ');
  }
  if (!name) return null;
  let type;
  if (kind === null) type = 'normal';
  else if (kind.length === 1) { type = keyword(kind[0], ['normal', 'size', 'inline-size', 'scroll-state', 'anchored']); if (type === null) return null; }
  else {
    const list = kind.map((c) => keyword(c, ['size', 'inline-size', 'scroll-state', 'anchored']));
    // Each keyword once, and at most one of size / inline-size.
    if (list.some((v, index) => v === null || list.indexOf(v) !== index) || list.filter((v) => v.endsWith('size')).length > 1) return null;
    type = list.join(' ');
  }
  return [['container-name', name], ['container-type', type]];
}

function whiteSpace(comps) {
  if (comps.length === 1) {
    const mapped = { normal: ['collapse', 'wrap'], pre: ['preserve', 'nowrap'], nowrap: ['collapse', 'nowrap'], 'pre-wrap': ['preserve', 'wrap'], 'pre-line': ['preserve-breaks', 'wrap'], 'break-spaces': ['break-spaces', 'wrap'] }[ident(comps[0])];
    if (mapped) return [['white-space-collapse', mapped[0]], ['text-wrap-mode', mapped[1]]];
  }
  const parsed = anyOrder(comps, [one((c) => keyword(c, ['collapse', 'preserve', 'preserve-breaks', 'break-spaces'])), one((c) => keyword(c, ['wrap', 'nowrap']))]);
  return parsed && namesOrInitial(['white-space-collapse', 'text-wrap-mode'], parsed);
}

function fontSynthesis(comps) {
  const names = ['font-synthesis-weight', 'font-synthesis-style', 'font-synthesis-small-caps'];
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return same(names, 'none');
  const parsed = anyOrder(comps, [one((c) => keyword(c, ['weight'])), one((c) => keyword(c, ['style'])), one((c) => keyword(c, ['small-caps']))]);
  return parsed && names.map((name, index) => [name, parsed[index] !== null ? 'auto' : 'none']);
}

function textBox(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['normal'])) return [['text-box-trim', 'none'], ['text-box-edge', 'auto']];
  const trim = (c) => keyword(c, ['none', 'trim-start', 'trim-end', 'trim-both']);
  const edge = (comps) => {
    if (!comps.length) return null;
    const first = comps[0];
    if (isIdent(first, ['auto'])) return [1, 'auto'];
    const a = keyword(first, ['text', 'cap', 'ex', 'ideographic', 'ideographic-ink']);
    if (a === null) return null;
    const b = comps[1] ? keyword(comps[1], ['text', 'alphabetic', 'ideographic', 'ideographic-ink']) : null;
    if (b !== null) return [2, a === b ? a : `${a} ${b}`];
    return ['text', 'ideographic', 'ideographic-ink'].includes(a) ? [1, a] : null;
  };
  const parsed = anyOrder(comps, [one(trim), edge]);
  return parsed && [['text-box-trim', parsed[0] ?? 'trim-both'], ['text-box-edge', parsed[1] ?? 'auto']];
}

const TRY_TACTICS = ['flip-block', 'flip-inline', 'flip-start', 'flip-x', 'flip-y'];
function tryFallbacks(comps) {
  const items = commas(comps);
  if (!items) return null;
  const out = [];
  for (const item of items) {
    let name = null;
    const tactics = [];
    for (const comp of item) {
      const dashed = name === null ? dashedIdent(comp) : null;
      if (dashed !== null) { name = dashed; continue; }
      const tactic = keyword(comp, TRY_TACTICS);
      if (tactic === null || tactics.includes(tactic)) return null;
      tactics.push(tactic);
    }
    out.push([...(name === null ? [] : [name]), ...tactics].join(' '));
  }
  return out.join(', ');
}
function positionTry(comps) {
  const orders = ['normal', 'most-width', 'most-height', 'most-block-size', 'most-inline-size'];
  let order = 'normal', rest = comps;
  if (comps.length > 1 && isIdent(comps[0], orders)) { order = ident(comps[0]); rest = comps.slice(1); }
  const fallbacks = rest.length === 1 && isIdent(rest[0], ['none']) ? 'none' : tryFallbacks(rest);
  return fallbacks === null ? null : [['position-try-order', order], ['position-try-fallbacks', fallbacks]];
}

function scrollTimeline(comps) {
  const layers = commas(comps);
  if (!layers) return null;
  const nameList = [], axisList = [];
  for (const layer of layers) {
    if (layer.length > 2) return null;
    const name = keyword(layer[0], ['none']) ?? dashedIdent(layer[0]);
    if (name === null) return null;
    nameList.push(name);
    if (layer[1]) { const axis = keyword(layer[1], ['block', 'inline', 'x', 'y']); if (axis === null) return null; axisList.push(axis); } else axisList.push('block');
  }
  return [['scroll-timeline-name', nameList.join(', ')], ['scroll-timeline-axis', axisList.join(', ')]];
}
function viewTimeline(comps) {
  const layers = commas(comps);
  if (!layers) return null;
  const nameList = [], axisList = [], insetList = [];
  for (const layer of layers) {
    const name = keyword(layer[0], ['none']) ?? dashedIdent(layer[0]);
    if (name === null) return null;
    nameList.push(name);
    const inset = (comps) => {
      const value = (c) => keyword(c, ['auto']) ?? lengthPercentage(c, true);
      if (!comps.length) return null;
      const a = value(comps[0]);
      if (a === null) return null;
      const b = comps[1] ? value(comps[1]) : null;
      return b !== null ? [2, a === b ? a : `${a} ${b}`] : [1, a];
    };
    const parsed = anyOrder(layer.slice(1), [one((c) => keyword(c, ['block', 'inline', 'x', 'y'])), inset]);
    if (!parsed) return null;
    axisList.push(parsed[0] ?? 'block');
    insetList.push(parsed[1] ?? 'auto');
  }
  return [['view-timeline-name', nameList.join(', ')], ['view-timeline-axis', axisList.join(', ')], ['view-timeline-inset', insetList.join(', ')]];
}

const LIGATURES = [['common-ligatures', 'no-common-ligatures'], ['discretionary-ligatures', 'no-discretionary-ligatures'], ['historical-ligatures', 'no-historical-ligatures'], ['contextual', 'no-contextual']];
const NUMERIC = [['lining-nums', 'oldstyle-nums'], ['proportional-nums', 'tabular-nums'], ['diagonal-fractions', 'stacked-fractions'], ['ordinal'], ['slashed-zero']];
const EAST_ASIAN = [['jis78', 'jis83', 'jis90', 'jis04', 'simplified', 'traditional'], ['full-width', 'proportional-width'], ['ruby']];
const CAPS = ['small-caps', 'all-small-caps', 'petite-caps', 'all-petite-caps', 'unicase', 'titling-caps'];
function groupSet(comps, groups) {
  const picked = groups.map(() => null);
  let used = 0;
  for (const comp of comps) {
    const name = ident(comp);
    if (name === null) break;
    const group = groups.findIndex((g) => g.includes(name));
    if (group < 0) break;
    if (picked[group] !== null) return null;
    picked[group] = name;
    used++;
  }
  return used ? [used, picked.filter((v) => v !== null).join(' ')] : null;
}
const ALTERNATES_ORDER = ['stylistic', 'historical-forms', 'styleset', 'character-variant', 'swash', 'ornaments', 'annotation'];
const isAlternate = (c) => isIdent(c, ['historical-forms']) || (c.kind === 'function' && ALTERNATES_ORDER.includes(c.name) && c.name !== 'historical-forms');
/**
 * `font-variant-alternates` values: `historical-forms` and the feature functions, each at most
 * once, in Chrome's order; `[used, text]` from the front of `comps`.
 */
function alternates(comps) {
  const found = [];
  for (const comp of comps) {
    if (!isAlternate(comp)) break;
    const rank = ALTERNATES_ORDER.indexOf(comp.kind === 'ident' ? 'historical-forms' : comp.name);
    if (found.some(([seen]) => seen === rank)) return null;
    found.push([rank, comp.kind === 'ident' ? 'historical-forms' : comp.text]);
  }
  if (!found.length) return null;
  const used = found.length;
  found.sort((a, b) => a[0] - b[0]);
  return [used, found.map(([, text]) => text).join(' ')];
}
function fontVariant(comps) {
  const order = ['font-variant-ligatures', 'font-variant-caps', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-alternates', 'font-variant-position', 'font-variant-emoji'];
  if (comps.length === 1 && isIdent(comps[0], ['normal'])) return same(order, 'normal');
  if (comps.length === 1 && isIdent(comps[0], ['none'])) { const out = same(order, 'normal'); out[0][1] = 'none'; return out; }
  // The alternates values may sit anywhere among the others.
  const alternateComps = comps.filter(isAlternate), others = comps.filter((c) => !isAlternate(c));
  let alternatesValue = null;
  if (alternateComps.length) { const a = alternates(alternateComps); if (!a || a[0] !== alternateComps.length) return null; alternatesValue = a[1]; }
  const parsed = others.length ? anyOrder(others, [(c) => groupSet(c, LIGATURES), (c) => groupSet(c, NUMERIC), (c) => groupSet(c, EAST_ASIAN), one((c) => keyword(c, CAPS)), () => null, one((c) => keyword(c, ['sub', 'super'])), one((c) => keyword(c, ['text', 'emoji', 'unicode']))]) : order.map(() => null);
  if (!parsed) return null;
  parsed[4] = alternatesValue;
  const names = ['font-variant-ligatures', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-caps', 'font-variant-alternates', 'font-variant-position', 'font-variant-emoji'];
  return names.map((name, index) => [name, parsed[index] ?? 'normal']);
}

const FLEX_BASIS = (c) => keyword(c, ['auto', 'content', 'min-content', 'max-content', 'fit-content']) ?? lengthPercentage(c, false);
function flexStep(state, comps, at) {
  const comp = comps[at];
  if (state.grow === null) {
    const n = number(comp, false);
    if (n !== null) {
      state.grow = n;
      const next = comps[at + 1] ? number(comps[at + 1], false) : null;
      if (next !== null) { state.shrink = next; return at + 2; }
      return at + 1;
    }
  }
  if (state.basisValue === null) {
    const b = FLEX_BASIS(comp);
    if (b !== null) { state.basisValue = b; return at + 1; }
  }
  return -1;
}
function flex(comps) {
  const names = ['flex-grow', 'flex-shrink', 'flex-basis'];
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return named(names, ['0', '0', 'auto']);
  if (comps.length === 1 && isIdent(comps[0], ['auto'])) return named(names, ['1', '1', 'auto']);
  const state = { grow: null, shrink: null, basisValue: null };
  let at = 0;
  while (at < comps.length) {
    const next = flexStep(state, comps, at);
    if (next === -1) return null;
    at = next;
  }
  if (state.grow === null && state.basisValue === null) return null;
  return named(names, [state.grow ?? '1', state.shrink ?? '1', state.basisValue ?? '0%']);
}

const SYSTEM_FONTS = ['caption', 'icon', 'menu', 'message-box', 'small-caption', 'status-bar'];
const FONT_SIZES = ['xx-small', 'x-small', 'small', 'medium', 'large', 'x-large', 'xx-large', 'xxx-large', 'larger', 'smaller', '-webkit-xxx-large'];
const STRETCHES = ['ultra-condensed', 'extra-condensed', 'condensed', 'semi-condensed', 'semi-expanded', 'expanded', 'extra-expanded', 'ultra-expanded'];
const GENERIC_FAMILIES = ['serif', 'sans-serif', 'cursive', 'fantasy', 'monospace', 'system-ui', 'math', 'emoji', 'fangsong', 'ui-serif', 'ui-sans-serif', 'ui-monospace', 'ui-rounded', '-webkit-body', '-webkit-pictograph'];
const reserved = (name, extra) => { const lower = name.toLowerCase(); return CSS_WIDE.has(lower) || lower === 'default' || extra.includes(lower); };
/**
 * A family name as Chrome serializes it: unquoted when it is one identifier (an optional `-`,
 * a name-start character, name characters; no escapes) Chrome would not read as a keyword, a
 * string otherwise.
 */
const familyText = (name) => (/^-?[A-Za-z_\u0080-\uffff][A-Za-z0-9_\-\u0080-\uffff]*$/.test(name) && !reserved(name, GENERIC_FAMILIES) ? name : quoted(name));
const familyString = (comp) => familyText(comp.value);
/** CSSOM "serialize an identifier". */
function serializeIdentifier(text) {
  const chars = [...text];
  let out = '';
  chars.forEach((c, index) => {
    const code = c.codePointAt(0);
    if (code === 0) out += '\ufffd';
    else if ((code >= 1 && code <= 0x1f) || code === 0x7f || (index === 0 && /[0-9]/.test(c)) || (index === 1 && /[0-9]/.test(c) && chars[0] === '-')) out += `\\${code.toString(16)} `;
    else if (index === 0 && c === '-' && chars.length === 1) out += '\\-';
    else if (code >= 0x80 || /[-_0-9A-Za-z]/.test(c)) out += c;
    else out += `\\${c}`;
  });
  return out;
}
/** A quoted animation name: an identifier unless it is empty (invalid) or reads as `none`, a CSS-wide keyword or `default` (kept quoted). */
const animationNameString = (c) => (c.value === '' ? null : reserved(c.value, ['none']) ? c.text : serializeIdentifier(c.value));
function fontFamily(comps) {
  const items = commas(comps);
  if (!items) return null;
  const families = [];
  for (const family of items) {
    if (family.length === 1 && string(family[0]) !== null) { families.push(familyString(family[0])); continue; }
    if (family.some((c) => c.kind !== 'ident')) return null;
    const words = family.map((c) => c.value);
    if (words.length === 1 && CSS_WIDE.has(words[0].toLowerCase())) return null;
    // Chrome quotes a family name of several identifiers.
    const name = words.join(' ');
    const lower = name.toLowerCase();
    // A generic family is a keyword: lowercase.
    families.push(words.length > 1 ? quoted(name) : GENERIC_FAMILIES.includes(lower) ? lower : familyText(name));
  }
  return families.join(', ');
}
/** `oblique <angle>`: Chrome checks a literal angle's number, in its own unit, against [-90, 90]; a zero angle is `normal`. */
function oblique(c) {
  // A unitless zero is not an angle here.
  if (c.kind === 'number') return null;
  const text = angle(c);
  if (text === null) return null;
  if (c.kind === 'dimension' && (c.value < -90 || c.value > 90)) return null;
  if ((c.kind === 'dimension' || c.kind === 'number') && c.value === 0) return 'normal';
  return `oblique ${text}`;
}
function fontStyleSlot(comps, at) {
  const comp = comps[at];
  const value = keyword(comp, ['italic', 'oblique']);
  if (value === null) return null;
  if (value === 'oblique' && comps[at + 1] && comps[at + 1].kind === 'dimension' && angle(comps[at + 1]) !== null) {
    const o = oblique(comps[at + 1]);
    return o === null ? 'bad' : { value: o, used: 2 };
  }
  return { value, used: 1 };
}
function fontWeightSlot(comp) {
  const value = keyword(comp, ['bold', 'bolder', 'lighter']) ?? (comp.kind === 'number' && comp.value >= 1 && comp.value <= 1000 ? number(comp, false) : null);
  return value === null ? null : { value, used: 1 };
}
function fontLeading(comps) {
  let style = null, caps = null, weight = null, stretch = null, at = 0;
  while (at < comps.length && at < 4) {
    const comp = comps[at];
    if (isIdent(comp, ['normal'])) { at++; continue; }
    if (style === null) {
      const s = fontStyleSlot(comps, at);
      if (s === 'bad') return null;
      if (s) { style = s.value; at += s.used; continue; }
    }
    if (caps === null) { const value = keyword(comp, ['small-caps']); if (value !== null) { caps = value; at++; continue; } }
    if (weight === null) {
      const w = fontWeightSlot(comp);
      if (w) { weight = w.value; at++; continue; }
    }
    if (stretch === null) { const value = keyword(comp, STRETCHES); if (value !== null) { stretch = value; at++; continue; } }
    break;
  }
  return { style, caps, weight, stretch, at };
}
function fontSizeAndLineHeight(comps, at) {
  const sizeComp = comps[at];
  if (!sizeComp) return null;
  const size = keyword(sizeComp, FONT_SIZES) ?? lengthPercentage(sizeComp, false);
  if (size === null) return null;
  at++;
  let lineHeight = 'normal';
  if (comps[at] && isSlash(comps[at])) {
    const value = comps[at + 1];
    if (!value) return null;
    lineHeight = keyword(value, ['normal']) ?? number(value, false) ?? lengthPercentage(value, false);
    if (lineHeight === null) return null;
    at += 2;
  }
  return { size, lineHeight, at };
}
function font(comps) {
  if (comps.length === 1) { const name = keyword(comps[0], SYSTEM_FONTS); if (name !== null) return { pending: name }; }
  const leading = fontLeading(comps);
  if (!leading) return null;
  const sized = fontSizeAndLineHeight(comps, leading.at);
  if (!sized) return null;
  const rest = comps.slice(sized.at);
  if (!rest.length) return null;
  const family = fontFamily(rest);
  if (family === null) return null;
  return [['font-style', leading.style ?? 'normal'], ['font-variant-caps', leading.caps ?? 'normal'], ['font-variant-ligatures', 'normal'], ['font-variant-numeric', 'normal'], ['font-variant-east-asian', 'normal'], ['font-variant-alternates', 'normal'], ['font-size-adjust', 'none'], ['font-language-override', 'normal'], ['font-kerning', 'auto'], ['font-optical-sizing', 'auto'], ['font-feature-settings', 'normal'], ['font-variation-settings', 'normal'], ['font-variant-position', 'normal'], ['font-variant-emoji', 'normal'], ['font-weight', leading.weight ?? 'normal'], ['font-stretch', leading.stretch ?? 'normal'], ['font-size', sized.size], ['line-height', sized.lineHeight], ['font-family', family]];
}

/** `<grid-line>` with Chrome's canonical part order (`span`, integer, name). */
function gridLinePart(state, comp) {
  if (isIdent(comp, ['span']) && !state.span) { state.span = true; return true; }
  const n = state.int === null ? integer(comp, -Infinity) : null;
  if (n !== null) { if (n === '0') return false; state.int = n; return true; }
  const c = state.name === null ? customIdent(comp, ['auto', 'span']) : null;
  if (c !== null) { state.name = c; return true; }
  return false;
}
function gridLine(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['auto'])) return 'auto';
  if (!comps.length) return null;
  const state = { span: false, int: null, name: null };
  for (const comp of comps) if (!gridLinePart(state, comp)) return null;
  if (state.int === null && state.name === null) return null;
  if (state.span && state.int !== null && state.int.startsWith('-')) return null;
  return [state.span ? 'span' : null, state.int, state.name].filter((v) => v !== null).join(' ');
}
/** A line's default end / omitted value: a lone custom ident repeats, else `auto`. */
function lineDefault(start) {
  const lone = start !== '' && !start.includes(' ') && start !== 'auto' && (start.startsWith('--') || !/^[0-9+-]/.test(start));
  return lone ? start : 'auto';
}
function gridLines(comps, start, end) {
  const parts = slashes(comps);
  if (parts.length === 1) { const a = gridLine(parts[0]); return a === null ? null : [[start, a], [end, lineDefault(a)]]; }
  if (parts.length === 2) { const a = gridLine(parts[0]), b = gridLine(parts[1]); return a === null || b === null ? null : [[start, a], [end, b]]; }
  return null;
}
function gridArea(comps) {
  const parts = slashes(comps);
  if (!parts.length || parts.length > 4) return null;
  const lines = parts.map(gridLine);
  if (lines.some((v) => v === null)) return null;
  const rowStart = lines[0];
  const columnStart = lines[1] ?? lineDefault(rowStart);
  const rowEnd = lines[2] ?? lineDefault(rowStart);
  const columnEnd = lines[3] ?? lineDefault(columnStart);
  return [['grid-row-start', rowStart], ['grid-column-start', columnStart], ['grid-row-end', rowEnd], ['grid-column-end', columnEnd]];
}
/** A track breadth: a non-negative `<length-percentage>`, a sizing keyword, or (where `flex`) a non-negative `fr`. */
function trackBreadth(c, flex) {
  if (c.kind === 'dimension' && c.unit === 'fr') return flex && c.value >= 0;
  if (c.kind === 'ident') return isIdent(c, ['auto', 'min-content', 'max-content']);
  if (c.kind === 'function' && c.name === '[') return false;
  return lengthPercentage(c, false) !== null;
}
/** At least one track, and no two line-name blocks side by side. */
function tracksValid(comps) {
  const names = (c) => c.kind === 'function' && c.name === '[';
  return comps.some((c) => !names(c)) && !comps.some((c, index) => index > 0 && names(c) && names(comps[index - 1]));
}
/** An auto-repeated track: line names or a fixed size (a `<length-percentage>`, or a `minmax()` with a fixed bound). */
function trackFixed(c) {
  const fixed = (x) => x.kind !== 'ident' && lengthPercentage(x, false) !== null;
  if (c.kind === 'function' && c.name === '[') return true;
  if (c.kind === 'function' && c.name === 'minmax') { const inner = components(c.text.slice(7, -1)); return !!inner && inner.length === 3 && (fixed(inner[0]) || fixed(inner[2])); }
  if (c.kind === 'function' && c.name === 'fit-content') return false;
  return fixed(c);
}
/** A track component spelled like Chrome: a unitless zero track size as `0px`, keywords lowercase, inside minmax() / fit-content() / repeat() too. */
function trackText(c, repeatCount) {
  if (c.kind === 'number' && !repeatCount) return '0px';
  if (c.kind === 'ident') return c.value.toLowerCase();
  if (c.kind === 'function' && ['minmax', 'fit-content', 'repeat'].includes(c.name)) {
    const inner = components(c.text.slice(c.name.length + 1, -1));
    if (!inner) return c.text;
    let seenComma = false;
    const spelled = inner.map((part) => { const count = c.name === 'repeat' && !seenComma; if (isComma(part)) seenComma = true; return { ...part, text: trackText(part, count) }; });
    return `${c.name}(${join(spelled)})`;
  }
  return c.text;
}
/** One track-list component: line names, a track size, or (at the top level) `repeat()`. */
function trackOkRepeat(inner, repeatAllowed) {
  if (!repeatAllowed || !inner) return false;
  const comma = inner.findIndex(isComma);
  if (comma < 0) return false;
  const head = inner.slice(0, comma), tracks = inner.slice(comma + 1);
  const auto = head.length === 1 && isIdent(head[0], ['auto-fill', 'auto-fit']);
  const count = head.length === 1 && (head[0].kind === 'number' ? head[0].integer && head[0].value >= 1 : auto);
  return count && tracksValid(tracks) && tracks.every((track) => trackOk(track, false) && (!auto || trackFixed(track)));
}
function trackOk(c, repeatAllowed) {
  if (c.kind !== 'function') return trackBreadth(c, true);
  if (c.name === '[') return true;
  const inner = components(c.text.slice(c.name.length + 1, -1));
  if (c.name === 'minmax') return !!inner && inner.length === 3 && isComma(inner[1]) && trackBreadth(inner[0], false) && trackBreadth(inner[2], true);
  if (c.name === 'fit-content') return !!inner && inner.length === 1 && lengthPercentage(inner[0], false) !== null;
  if (c.name === 'repeat') return trackOkRepeat(inner, repeatAllowed);
  return lengthPercentage(c, false) !== null;
}
function trackList(comps) {
  if (!comps.length) return null;
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return 'none';
  const subgrid = isIdent(comps[0], ['subgrid']);
  for (let index = 0; index < comps.length; index++) {
    const comp = comps[index];
    const ok = comp.kind === 'ident' ? (index === 0 && subgrid) || isIdent(comp, ['auto', 'min-content', 'max-content', 'masonry']) : trackOk(comp, true);
    if (!ok) return null;
  }
  if (!subgrid && !tracksValid(comps)) return null;
  return join(comps.map((c) => ({ ...c, text: trackText(c, false) })));
}
/**
 * A `grid-template-areas` string's cell tokens: runs of name code points, a run of `.` one null
 * cell `.`; null for a row without cells or with any other character.
 */
function areaRow(comp) {
  if (comp.kind !== 'string') return null;
  const tokens = comp.value.match(/[A-Za-z0-9_\-\u0080-\uffff]+|\.+|[^ \t\n\r\f]/g) ?? [];
  if (!tokens.length || tokens.some((token) => !/^([A-Za-z0-9_\-\u0080-\uffff]+|\.+)$/.test(token))) return null;
  return tokens.map((token) => (token[0] === '.' ? '.' : token));
}
/**
 * `grid-template-areas` rows Chrome accepts (as many cells each, every named area a filled
 * rectangle) as quoted rows.
 */
function areasText(rows) {
  if (rows.some((row) => row === null || row.length !== rows[0].length)) return null;
  const boxes = new Map();
  rows.forEach((row, y) => row.forEach((cell, x) => {
    if (cell === '.') return;
    const box = boxes.get(cell) ?? { x0: x, x1: x, y0: y, y1: y, count: 0 };
    Object.assign(box, { x0: Math.min(box.x0, x), x1: Math.max(box.x1, x), y0: Math.min(box.y0, y), y1: Math.max(box.y1, y), count: box.count + 1 });
    boxes.set(cell, box);
  }));
  for (const box of boxes.values()) if ((box.x1 - box.x0 + 1) * (box.y1 - box.y0 + 1) !== box.count) return null;
  return rows.map((row) => `"${row.join(' ')}"`).join(' ');
}
const isNamesBlock = (c) => c.kind === 'function' && c.name === '[';
function gridTemplateColumns(parts) {
  if (parts.length === 1) return { rows: parts[0], cols: null };
  if (parts.length === 2 && !parts[1].some((c) => functionNamed(c, ['repeat']) !== null)) {
    const cols = trackList(parts[1]);
    return cols === null ? null : { rows: parts[0], cols };
  }
  return null;
}
function gridTemplateTrackSize(c) {
  if (!c || string(c) !== null || isNamesBlock(c)) return null;
  return lengthPercentage(c, false) ?? keyword(c, ['auto', 'min-content', 'max-content']) ?? functionNamed(c, ['minmax', 'fit-content']) ?? (c.kind === 'dimension' && c.unit === 'fr' ? c.text : null);
}
function gridTemplateAreasAndTracks(rows) {
  const areas = [], tracks = [];
  let at = 0;
  const pushNames = () => { if (rows[at] && isNamesBlock(rows[at])) { tracks.push(rows[at].text); at++; } };
  while (at < rows.length) {
    pushNames();
    const area = rows[at] ? areaRow(rows[at]) : null;
    if (area === null) return null;
    areas.push(area);
    at++;
    const size = gridTemplateTrackSize(rows[at]);
    if (size !== null) { tracks.push(size); at++; } else tracks.push('auto');
    pushNames();
  }
  return { areas, tracks };
}
function mergeTrackNames(tracks) {
  const merged = [];
  for (const track of tracks) {
    const last = merged[merged.length - 1];
    if (last !== undefined && track.startsWith('[') && last.startsWith('[')) { merged[merged.length - 1] = `${last.slice(0, -1)} ${track.slice(1)}`; continue; }
    merged.push(track);
  }
  return merged;
}
function gridTemplate(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return ['none', 'none', 'none'];
  const hasStrings = comps.some((c) => string(c) !== null);
  const parts = slashes(comps);
  if (!hasStrings) {
    if (parts.length !== 2) return null;
    const rows = trackList(parts[0]), cols = trackList(parts[1]);
    return rows === null || cols === null ? null : [rows, cols, 'none'];
  }
  const split = gridTemplateColumns(parts);
  if (!split) return null;
  const parsed = gridTemplateAreasAndTracks(split.rows);
  if (!parsed) return null;
  const areasValue = areasText(parsed.areas);
  return areasValue === null ? null : [mergeTrackNames(parsed.tracks).join(' '), split.cols ?? 'none', areasValue];
}
function grid(comps) {
  const parts = slashes(comps);
  const autoFlow = (side) => {
    let flow = false, dense = false;
    const rest = [];
    for (const comp of side) {
      if (isIdent(comp, ['auto-flow']) && !flow) flow = true;
      else if (isIdent(comp, ['dense']) && !dense) dense = true;
      else rest.push(comp);
    }
    return flow ? [dense, rest] : null;
  };
  if (parts.length === 2) {
    const [rows, cols] = parts;
    const r = autoFlow(rows);
    if (r) {
      const auto = r[1].length ? trackList(r[1]) : 'auto';
      const c = trackList(cols);
      if (auto === null || c === null) return null;
      return [['grid-template-columns', c], ['grid-template-rows', 'none'], ['grid-template-areas', 'none'], ['grid-auto-flow', r[0] ? 'dense' : 'row'], ['grid-auto-columns', 'auto'], ['grid-auto-rows', auto]];
    }
    const k = autoFlow(cols);
    if (k) {
      const auto = k[1].length ? trackList(k[1]) : 'auto';
      const t = trackList(rows);
      if (auto === null || t === null) return null;
      return [['grid-template-columns', 'none'], ['grid-template-rows', t], ['grid-template-areas', 'none'], ['grid-auto-flow', k[0] ? 'column dense' : 'column'], ['grid-auto-columns', auto], ['grid-auto-rows', 'auto']];
    }
  }
  if (comps.some((c) => isIdent(c, ['auto-flow', 'dense']))) return null;
  const template = gridTemplate(comps);
  return template && [['grid-template-rows', template[0]], ['grid-template-columns', template[1]], ['grid-template-areas', template[2]], ['grid-auto-flow', 'row'], ['grid-auto-columns', 'auto'], ['grid-auto-rows', 'auto']];
}

/** Collapse a one-to-four value side list the `margin` way. */
function collapse4(values) {
  const full = values.length === 1 ? [values[0], values[0], values[0], values[0]] : values.length === 2 ? [values[0], values[1], values[0], values[1]] : values.length === 3 ? [values[0], values[1], values[2], values[1]] : values.length === 4 ? values : null;
  return full ? boxText(full) : values.join(' ');
}
function borderImageRepeat(comps, at) {
  const kw = (c) => keyword(c, ['stretch', 'repeat', 'round', 'space']);
  const first = kw(comps[at]);
  if (first === null) return null;
  const second = comps[at + 1] ? kw(comps[at + 1]) : null;
  const used = second !== null ? 2 : 1;
  return { value: second !== null && second !== first ? `${first} ${second}` : first, used };
}
function borderImageSliceValues(comps, state) {
  const values = [];
  let fill = false;
  while (comps[state.at]) {
    const c = comps[state.at];
    if (isIdent(c, ['fill']) && !fill) fill = true;
    else if (fill && values.length) break; // `fill` goes before or after the numbers
    else { const value = values.length < 4 ? number(c, false) ?? percentage(c, false) : null; if (value === null) break; values.push(value); }
    state.at++;
  }
  return { values, fill };
}
function borderImageQuad(comps, state, ty) {
  const list = [];
  while (comps[state.at] && list.length < 4) { const value = ty(comps[state.at]); if (value === null) break; list.push(value); state.at++; }
  return list.length ? collapse4(list) : null;
}
function borderImageWidthOutset(comps, state) {
  if (!comps[state.at] || !isSlash(comps[state.at])) return { width: null, outset: null };
  state.at++;
  const w = borderImageQuad(comps, state, (c) => number(c, false) ?? lengthPercentage(c, false) ?? keyword(c, ['auto']));
  let o = null;
  if (comps[state.at] && isSlash(comps[state.at])) {
    state.at++;
    o = borderImageQuad(comps, state, (c) => number(c, false) ?? length(c, false));
    if (o === null) return null;
  } else if (w === null) return null;
  return { width: w, outset: o };
}
function borderImageSlice(comps, state, webkit) {
  const { values, fill } = borderImageSliceValues(comps, state);
  if (!values.length) return null;
  const slice = collapse4(values) + (fill || webkit ? ' fill' : '');
  const wo = borderImageWidthOutset(comps, state);
  return wo && { slice, width: wo.width, outset: wo.outset };
}
function borderImage(comps, webkit) {
  let source = null, slice = null, width = null, outset = null, repeat = null;
  const state = { at: 0 };
  while (state.at < comps.length) {
    const comp = comps[state.at];
    if (source === null) { const value = image(comp) ?? keyword(comp, ['none']); if (value !== null) { source = value; state.at++; continue; } }
    if (repeat === null) {
      const r = borderImageRepeat(comps, state.at);
      if (r) { repeat = r.value; state.at += r.used; continue; }
    }
    if (slice === null) {
      const parsed = borderImageSlice(comps, state, webkit);
      if (!parsed) return null;
      slice = parsed.slice; width = parsed.width; outset = parsed.outset;
      continue;
    }
    return null;
  }
  if ([source, slice, width, outset, repeat].every((v) => v === null)) return null;
  return [source, slice, width, outset, repeat];
}
/** `path()`'s string holds path data: at least one command, only path data characters. */
function pathDataValid(text) {
  const start = text.indexOf('"'), end = text.lastIndexOf('"');
  return end > start && start >= 0 && /^[\sMmLlHhVvCcSsQqTtAaZz0-9.,+eE-]*$/.test(text.slice(start + 1, end)) && text.slice(start + 1, end).trim() !== '';
}
/** `<offset-rotate>` from the front: `[auto | reverse] || <angle>` (an angle has a unit). */
function rotatePart(comps) {
  if (!comps.length) return null;
  const first = comps[0];
  const kw = keyword(first, ['auto', 'reverse']);
  if (kw !== null) { const a = comps[1] ? angleStrict(comps[1]) : null; return a !== null ? [2, `${kw} ${a}`] : [1, kw]; }
  const a = angleStrict(first);
  if (a === null) return null;
  const kw2 = comps[1] ? keyword(comps[1], ['auto', 'reverse']) : null;
  return kw2 !== null ? [2, `${kw2} ${a}`] : [1, a];
}
function pathText(text) {
  // pathDataValid found the quoted data.
  const start = text.indexOf('"');
  const end = text.lastIndexOf('"');
  const inner = text.slice(start + 1, end > start ? end : text.length);
  const tokens = [];
  let current = '';
  for (const c of inner) {
    if (/[A-Za-z]/.test(c) && c !== 'e' && c !== 'E') { if (current) { tokens.push(current); current = ''; } tokens.push(c); }
    else if (/\s/.test(c) || c === ',') { if (current) { tokens.push(current); current = ''; } }
    else if (c === '-' && current && !/[eE]$/.test(current)) { tokens.push(current); current = c; }
    else current += c;
  }
  if (current) tokens.push(current);
  return `${text.slice(0, start)}"${tokens.join(' ')}")`;
}
function offsetAnchor(anchorPart) {
  if (anchorPart === null) return 'auto';
  if (anchorPart.length === 1 && isIdent(anchorPart[0], ['auto'])) return 'auto';
  // Three-value positions are background-only.
  if (anchorPart.length === 3) return null;
  const p = position(anchorPart);
  return p ? `${p[0]} ${p[1]}` : null;
}
const OFFSET_POS_KW = (c) => keyword(c, ['left', 'right', 'top', 'bottom', 'center']) !== null || lengthPercentage(c, true) !== null;
function offsetPositionPart(main) {
  if (!main.length) return { value: 'normal', at: 0 };
  if (isIdent(main[0], ['auto', 'normal'])) return { value: ident(main[0]), at: 1 };
  for (let len = 4; len >= 1; len--) {
    if (main.length < len) continue;
    const slice = main.slice(0, len);
    if (slice.every(OFFSET_POS_KW)) {
      const p = position(slice);
      if (p) { if (len === 3) return null; return { value: `${p[0]} ${p[1]}`, at: len }; }
    }
  }
  return { value: 'normal', at: 0 };
}
function offsetPathPart(main, at) {
  const comp = main[at];
  if (!comp) return { path: 'none', distance: '0px', rotate: 'auto', at };
  const value = isIdent(comp, ['none']) ? 'none' : functionNamed(comp, ['path', 'ray', 'url', 'circle', 'ellipse', 'inset', 'polygon', 'xywh', 'rect', 'shape']) ?? url(comp);
  if (value === null) return { path: 'none', distance: '0px', rotate: 'auto', at };
  if (value.startsWith('path(') && !pathDataValid(value)) return null;
  const path = value.startsWith('path(') ? pathText(value) : value;
  at++;
  const parsed = anyOrder(main.slice(at), [one((c) => lengthPercentage(c, true)), rotatePart]);
  if (!parsed) return null;
  return { path, distance: parsed[0] ?? '0px', rotate: parsed[1] ?? 'auto', at: main.length };
}
function offset(comps) {
  const parts = slashes(comps);
  let main, anchorPart = null;
  if (parts.length === 1) main = parts[0];
  else if (parts.length === 2) { main = parts[0]; anchorPart = parts[1]; }
  else return null;
  const anchor = offsetAnchor(anchorPart);
  if (anchor === null) return null;
  const pos = offsetPositionPart(main);
  if (!pos) return null;
  const path = offsetPathPart(main, pos.at);
  if (!path) return null;
  if (path.at !== main.length || (!main.length && parts.length === 1)) return null;
  return [['offset-position', pos.value], ['offset-path', path.path], ['offset-distance', path.distance], ['offset-rotate', path.rotate], ['offset-anchor', anchor]];
}
function repeatStyle(comps) {
  if (!comps.length) return null;
  const xy = keyword(comps[0], ['repeat-x', 'repeat-y']);
  if (xy !== null) return [1, xy];
  const values = ['repeat', 'space', 'round', 'no-repeat'];
  const a = keyword(comps[0], values);
  if (a === null) return null;
  const b = comps[1] ? keyword(comps[1], values) : null;
  if (b === null) return [1, a];
  const text = a === b ? a : a === 'repeat' && b === 'no-repeat' ? 'repeat-x' : a === 'no-repeat' && b === 'repeat' ? 'repeat-y' : `${a} ${b}`;
  return [2, text];
}
function layerPosition(comps) {
  const part = (c) => keyword(c, ['left', 'right', 'top', 'bottom', 'center']) !== null || lengthPercentage(c, true) !== null;
  let available = 0;
  while (available < comps.length && part(comps[available])) available++;
  available = Math.min(available, 4);
  for (let len = available; len >= 1; len--) { const p = position(comps.slice(0, len)); if (p) return [len, p]; }
  return null;
}
function bgSize(comps) {
  if (!comps.length) return null;
  const cover = keyword(comps[0], ['cover', 'contain']);
  if (cover !== null) return [1, cover];
  const size = (c) => lengthPercentage(c, false) ?? keyword(c, ['auto']);
  const a = size(comps[0]);
  if (a === null) return null;
  const b = comps[1] ? size(comps[1]) : null;
  if (b !== null) return [2, `${a} ${b}`];
  // A single width other than `auto` spells its height `auto`.
  return a === 'auto' ? [1, a] : [1, `${a} auto`];
}
function layerPositionSize(out, comps, at) {
  const p = layerPosition(comps.slice(at));
  if (!p) return null;
  out.position = p[1]; at += p[0];
  if (comps[at] && isSlash(comps[at])) {
    const s = bgSize(comps.slice(at + 1));
    if (!s) return -1;
    out.size = s[1]; at += 1 + s[0];
  }
  return at;
}
function layerBoxSlot(out, value) {
  const clipOnly = value === 'text' || value === 'no-clip';
  if (out.origin === null && !clipOnly) out.origin = value;
  else if (out.clip === null) out.clip = value;
  else return false;
  return true;
}
function layerMaskSlot(out, comp) {
  if (out.composite === null) { const value = keyword(comp, ['add', 'subtract', 'intersect', 'exclude']); if (value !== null) { out.composite = value; return 1; } }
  if (out.mode === null) { const value = keyword(comp, ['alpha', 'luminance', 'match-source']); if (value !== null) { out.mode = value; return 1; } }
  return 0;
}
function layerStepImageOrPos(out, comps, at) {
  const comp = comps[at];
  if (out.image === null) { const value = image(comp) ?? keyword(comp, ['none']); if (value !== null) { out.image = value; return at + 1; } }
  if (out.position === null) {
    const next = layerPositionSize(out, comps, at);
    if (next === -1) return -1;
    if (next !== null) return next;
  }
  return null;
}
function layerStepRepeatAttach(out, comps, at, mask) {
  const comp = comps[at];
  if (out.repeat === null) { const r = repeatStyle(comps.slice(at)); if (r) { out.repeat = r[1]; return at + r[0]; } }
  if (!mask && out.attachment === null) { const value = keyword(comp, ['scroll', 'fixed', 'local']); if (value !== null) { out.attachment = value; return at + 1; } }
  return null;
}
function layerStepTail(out, comps, at, mask, last, boxes) {
  const comp = comps[at];
  if (out.clip === null || out.origin === null) {
    const value = keyword(comp, boxes);
    if (value !== null) return layerBoxSlot(out, value) ? at + 1 : -1;
  }
  if (mask) { const used = layerMaskSlot(out, comp); if (used) return at + used; }
  else if (last && out.color === null) { const value = color(comp); if (value !== null) { out.color = value; return at + 1; } }
  return -1;
}
function layerStep(out, comps, at, mask, last, boxes) {
  const first = layerStepImageOrPos(out, comps, at);
  if (first !== null) return first;
  const second = layerStepRepeatAttach(out, comps, at, mask);
  if (second !== null) return second;
  return layerStepTail(out, comps, at, mask, last, boxes);
}
const LAYER_BOXES_MASK = ['border-box', 'padding-box', 'content-box', 'fill-box', 'stroke-box', 'view-box', 'no-clip'];
const LAYER_BOXES_BG = ['border-box', 'padding-box', 'content-box', 'text', 'border-area'];
function layer(comps, mask, last) {
  const out = { image: null, position: null, size: null, repeat: null, attachment: null, origin: null, clip: null, color: null, composite: null, mode: null };
  const boxes = mask ? LAYER_BOXES_MASK : LAYER_BOXES_BG;
  let at = 0;
  while (at < comps.length) {
    const next = layerStep(out, comps, at, mask, last, boxes);
    if (next === -1) return null;
    at = next;
  }
  if (out.origin !== null && out.clip === null) out.clip = out.origin;
  return out;
}
function background(comps) {
  const items = commas(comps);
  if (!items) return null;
  const layers = items.map((item, index) => layer(item, false, index + 1 === items.length));
  if (layers.some((l) => l === null)) return null;
  const column = (pick) => layers.map((l) => pick(l) ?? INITIAL).join(', ');
  return [['background-image', column((l) => l.image)], ['background-position-x', column((l) => l.position && l.position[0])], ['background-position-y', column((l) => l.position && l.position[1])], ['background-size', column((l) => l.size)], ['background-repeat', column((l) => l.repeat)], ['background-attachment', column((l) => l.attachment)], ['background-origin', column((l) => l.origin)], ['background-clip', column((l) => l.clip)], ['background-color', layers[layers.length - 1].color ?? INITIAL]];
}
function maskShorthand(comps) {
  const items = commas(comps);
  if (!items) return null;
  const layers = items.map((item) => layer(item, true, false));
  if (layers.some((l) => l === null)) return null;
  const column = (pick, initial) => layers.map((l) => pick(l) ?? initial).join(', ');
  return [['mask-image', column((l) => l.image, 'none')], ['-webkit-mask-position-x', column((l) => l.position && l.position[0], '0%')], ['-webkit-mask-position-y', column((l) => l.position && l.position[1], '0%')], ['mask-size', column((l) => l.size, 'auto')], ['mask-repeat', column((l) => l.repeat, 'repeat')], ['mask-origin', column((l) => l.origin, 'border-box')], ['mask-clip', column((l) => l.clip, 'border-box')], ['mask-composite', column((l) => l.composite, 'add')], ['mask-mode', column((l) => l.mode, 'match-source')]];
}
const EASINGS = ['linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out', 'step-start', 'step-end'];
const easing = (c) => keyword(c, EASINGS) ?? functionNamed(c, ['cubic-bezier', 'steps', 'linear']);
function transition(comps) {
  const items = commas(comps);
  if (!items) return null;
  const columns = [[], [], [], [], []];
  for (const item of items) {
    const property = (c) => {
      if (isIdent(c, ['none'])) return 'none';
      if (isIdent(c, ['all'])) return 'all';
      const value = customIdent(c, ['none', 'initial']);
      return value === null ? null : value.startsWith('--') ? value : value.toLowerCase();
    };
    const parsed = anyOrder(item, [one((c) => time(c, false)), one(easing), one((c) => time(c, true)), one((c) => keyword(c, ['normal', 'allow-discrete'])), one(property)]);
    if (!parsed) return null;
    const prop = parsed[4] ?? 'all';
    if (prop === 'none' && items.length > 1) return null;
    columns[0].push(parsed[3] ?? 'normal');
    columns[1].push(parsed[0] ?? '0s');
    columns[2].push(parsed[1] ?? 'ease');
    columns[3].push(parsed[2] ?? '0s');
    columns[4].push(prop);
  }
  const [behavior, duration, timing, delay, property] = columns.map((c) => c.join(', '));
  return [['transition-behavior', behavior], ['transition-duration', duration], ['transition-timing-function', timing], ['transition-delay', delay], ['transition-property', property]];
}
function animation(comps) {
  const items = commas(comps);
  if (!items) return null;
  const columns = [[], [], [], [], [], [], [], []];
  const initials = ['auto', 'ease', '0s', '1', 'normal', 'none', 'running', 'none'];
  for (const item of items) {
    // A quoted name serializes as an identifier when it is one Chrome would not read as a keyword.
    const name = (c) => (isIdent(c, ['none']) ? 'none' : c.kind === 'string' ? animationNameString(c) : customIdent(c, ['none']));
    const parsed = anyOrder(item, [one((c) => time(c, false) ?? keyword(c, ['auto'])), one(easing), one((c) => time(c, true)), one((c) => keyword(c, ['infinite']) ?? number(c, false)), one((c) => keyword(c, ['normal', 'reverse', 'alternate', 'alternate-reverse'])), one((c) => keyword(c, ['none', 'forwards', 'backwards', 'both'])), one((c) => keyword(c, ['running', 'paused'])), one(name)]);
    if (!parsed) return null;
    initials.forEach((initial, index) => columns[index].push(parsed[index] ?? initial));
  }
  const names = ['animation-duration', 'animation-timing-function', 'animation-delay', 'animation-iteration-count', 'animation-direction', 'animation-fill-mode', 'animation-play-state', 'animation-name'];
  return [...names.map((n, index) => [n, columns[index].join(', ')]), ['animation-timeline', 'auto'], ['animation-range-start', 'normal'], ['animation-range-end', 'normal']];
}
function timelineTrigger(comps) {
  const defaults = () => [['timeline-trigger-source', 'auto'], ['timeline-trigger-activation-range-start', 'normal'], ['timeline-trigger-activation-range-end', 'normal'], ['timeline-trigger-active-range-start', 'auto'], ['timeline-trigger-active-range-end', 'auto']];
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return [['timeline-trigger-name', 'none'], ...defaults()];
  // A comma list of triggers: each longhand lists its value per trigger.
  const items = commas(comps);
  if (!items) return null;
  if (items.length > 1) {
    const columns = [];
    for (const item of items) {
      const parsed = timelineTriggerItem(item, defaults);
      if (!parsed) return null;
      parsed.forEach(([name, value], index) => { if (!columns[index]) columns[index] = [name, []]; columns[index][1].push(value); });
    }
    return columns.map(([name, values]) => [name, values.join(', ')]);
  }
  return timelineTriggerItem(comps, defaults);
}
function timelineTriggerItem(comps, defaults) {
  // The name and the source may each be left out (`--t / entry`, `/ entry`).
  let at = 0, name = 'none', source = 'auto';
  if (comps[at] && dashedIdent(comps[at]) !== null) name = dashedIdent(comps[at++]);
  const sourceOf = (c) => functionNamed(c, ['view', 'scroll']) ?? keyword(c, ['auto', 'none']);
  if (comps[at] && sourceOf(comps[at]) !== null) source = sourceOf(comps[at++]);
  const more = defaults().slice(1);
  const rest = comps.slice(at);
  if (rest.length) {
    const parts = slashes(rest);
    // An empty activation range before the slash keeps its default.
    if (!(parts[0].length === 0 && parts.length === 2)) {
      const activation = rangePair(parts[0], 'timeline-trigger-activation-range-start', 'timeline-trigger-activation-range-end', 'normal');
      if (!activation) return null;
      more[0][1] = activation[0][1]; more[1][1] = activation[1][1];
    }
    if (parts[1]) {
      const active = rangePair(parts[1], 'timeline-trigger-active-range-start', 'timeline-trigger-active-range-end', 'auto');
      if (!active) return null;
      more[2][1] = active[0][1]; more[3][1] = active[1][1];
    }
    if (parts.length > 2) return null;
  }
  return [['timeline-trigger-name', name], ['timeline-trigger-source', source], ...more];
}

const BORDER_SIDE = ['medium', 'none', 'currentcolor'];
const GRAMMARS = {
  margin: (c) => named(['margin-top', 'margin-right', 'margin-bottom', 'margin-left'], box4(c, marginV)),
  padding: (c) => named(['padding-top', 'padding-right', 'padding-bottom', 'padding-left'], box4(c, paddingV)),
  inset: (c) => named(['top', 'right', 'bottom', 'left'], box4(c, marginV)),
  'scroll-margin': (c) => named(['scroll-margin-top', 'scroll-margin-right', 'scroll-margin-bottom', 'scroll-margin-left'], box4(c, scrollMarginV)),
  'scroll-padding': (c) => named(['scroll-padding-top', 'scroll-padding-right', 'scroll-padding-bottom', 'scroll-padding-left'], box4(c, scrollPaddingV)),
  'border-width': (c) => named(['border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width'], box4(c, lineWidth)),
  'border-style': (c) => named(['border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style'], box4(c, lineStyle)),
  'border-color': (c) => named(['border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color'], box4(c, color)),
  'corner-shape': (c) => named(['corner-top-left-shape', 'corner-top-right-shape', 'corner-bottom-right-shape', 'corner-bottom-left-shape'], box4(c, cornerShape)),
  'border-radius': (c) => borderRadius(c, false),
  '-webkit-border-radius': (c) => borderRadius(c, true),
  'margin-block': (c) => named(['margin-block-start', 'margin-block-end'], pair(c, marginV)),
  'margin-inline': (c) => named(['margin-inline-start', 'margin-inline-end'], pair(c, marginV)),
  'padding-block': (c) => named(['padding-block-start', 'padding-block-end'], pair(c, paddingV)),
  'padding-inline': (c) => named(['padding-inline-start', 'padding-inline-end'], pair(c, paddingV)),
  'inset-block': (c) => named(['inset-block-start', 'inset-block-end'], pair(c, marginV)),
  'inset-inline': (c) => named(['inset-inline-start', 'inset-inline-end'], pair(c, marginV)),
  'scroll-margin-block': (c) => named(['scroll-margin-block-start', 'scroll-margin-block-end'], pair(c, scrollMarginV)),
  'scroll-margin-inline': (c) => named(['scroll-margin-inline-start', 'scroll-margin-inline-end'], pair(c, scrollMarginV)),
  'scroll-padding-block': (c) => named(['scroll-padding-block-start', 'scroll-padding-block-end'], pair(c, scrollPaddingV)),
  'scroll-padding-inline': (c) => named(['scroll-padding-inline-start', 'scroll-padding-inline-end'], pair(c, scrollPaddingV)),
  'border-block-width': (c) => named(['border-block-start-width', 'border-block-end-width'], pair(c, lineWidth)),
  'border-block-style': (c) => named(['border-block-start-style', 'border-block-end-style'], pair(c, lineStyle)),
  'border-block-color': (c) => named(['border-block-start-color', 'border-block-end-color'], pair(c, color)),
  'border-inline-width': (c) => named(['border-inline-start-width', 'border-inline-end-width'], pair(c, lineWidth)),
  'border-inline-style': (c) => named(['border-inline-start-style', 'border-inline-end-style'], pair(c, lineStyle)),
  'border-inline-color': (c) => named(['border-inline-start-color', 'border-inline-end-color'], pair(c, color)),
  gap: (c) => named(['row-gap', 'column-gap'], pair(c, gapV)),
  overflow: (c) => named(['overflow-x', 'overflow-y'], pair(c, overflowV)),
  'overscroll-behavior': (c) => named(['overscroll-behavior-x', 'overscroll-behavior-y'], pair(c, overscrollV)),
  'border-spacing': (c) => named(['-webkit-border-horizontal-spacing', '-webkit-border-vertical-spacing'], pair(c, spacingV)),
  'interest-delay': (c) => named(['interest-delay-start', 'interest-delay-end'], pair(c, delayV)),
  'corner-top-shape': (c) => named(['corner-top-left-shape', 'corner-top-right-shape'], pair(c, cornerShape)),
  'corner-right-shape': (c) => named(['corner-top-right-shape', 'corner-bottom-right-shape'], pair(c, cornerShape)),
  'corner-bottom-shape': (c) => named(['corner-bottom-left-shape', 'corner-bottom-right-shape'], pair(c, cornerShape)),
  'corner-left-shape': (c) => named(['corner-top-left-shape', 'corner-bottom-left-shape'], pair(c, cornerShape)),
  'corner-block-start-shape': (c) => named(['corner-start-start-shape', 'corner-start-end-shape'], pair(c, cornerShape)),
  'corner-block-end-shape': (c) => named(['corner-end-start-shape', 'corner-end-end-shape'], pair(c, cornerShape)),
  'corner-inline-start-shape': (c) => named(['corner-start-start-shape', 'corner-end-start-shape'], pair(c, cornerShape)),
  'corner-inline-end-shape': (c) => named(['corner-start-end-shape', 'corner-end-end-shape'], pair(c, cornerShape)),
  'column-rule-inset-cap': (c) => named(['column-rule-inset-cap-start', 'column-rule-inset-cap-end'], pair(c, ruleInsetV)),
  'column-rule-inset-junction': (c) => named(['column-rule-inset-junction-start', 'column-rule-inset-junction-end'], pair(c, ruleInsetV)),
  'row-rule-inset-cap': (c) => named(['row-rule-inset-cap-start', 'row-rule-inset-cap-end'], pair(c, ruleInsetV)),
  'row-rule-inset-junction': (c) => named(['row-rule-inset-junction-start', 'row-rule-inset-junction-end'], pair(c, ruleInsetV)),
  'column-rule-inset-start': (c) => same(['column-rule-inset-cap-start', 'column-rule-inset-junction-start'], single(c, ruleInsetV)),
  'column-rule-inset-end': (c) => same(['column-rule-inset-cap-end', 'column-rule-inset-junction-end'], single(c, ruleInsetV)),
  'row-rule-inset-start': (c) => same(['row-rule-inset-cap-start', 'row-rule-inset-junction-start'], single(c, ruleInsetV)),
  'row-rule-inset-end': (c) => same(['row-rule-inset-cap-end', 'row-rule-inset-junction-end'], single(c, ruleInsetV)),
  'column-rule-inset': (c) => ruleInset(c, 'column'),
  'row-rule-inset': (c) => ruleInset(c, 'row'),
  'rule-inset': (c) => { const a = ruleInset(c, 'column'), b = ruleInset(c, 'row'); return a && b && [...a, ...b]; },
  'rule-inset-cap': (c) => { const p = pair(c, ruleInsetV); return p && [['row-rule-inset-cap-start', p[0]], ['row-rule-inset-cap-end', p[1]], ['column-rule-inset-cap-start', p[0]], ['column-rule-inset-cap-end', p[1]]]; },
  'rule-inset-junction': (c) => { const p = pair(c, ruleInsetV); return p && [['row-rule-inset-junction-start', p[0]], ['row-rule-inset-junction-end', p[1]], ['column-rule-inset-junction-start', p[0]], ['column-rule-inset-junction-end', p[1]]]; },
  'rule-inset-start': (c) => same(['column-rule-inset-cap-start', 'column-rule-inset-junction-start', 'row-rule-inset-cap-start', 'row-rule-inset-junction-start'], single(c, ruleInsetV)),
  'rule-inset-end': (c) => same(['column-rule-inset-cap-end', 'column-rule-inset-junction-end', 'row-rule-inset-cap-end', 'row-rule-inset-junction-end'], single(c, ruleInsetV)),
  'rule-break': (c) => same(['column-rule-break', 'row-rule-break'], single(c, ruleBreakV)),
  'rule-color': (c) => same(['column-rule-color', 'row-rule-color'], single(c, color)),
  'rule-style': (c) => same(['column-rule-style', 'row-rule-style'], single(c, lineStyle)),
  'rule-width': (c) => same(['column-rule-width', 'row-rule-width'], single(c, lineWidth)),
  'rule-visibility-items': (c) => same(['column-rule-visibility-items', 'row-rule-visibility-items'], single(c, ruleVisibilityV)),
  marker: (c) => same(['marker-start', 'marker-mid', 'marker-end'], single(c, markerV)),
  'contain-intrinsic-size': containIntrinsicSize,
  'place-content': placeContent,
  'place-items': placeItems,
  'place-self': placeSelf,
  'background-position': (c) => { const p = positions(c, true); return p && [['background-position-x', p[0]], ['background-position-y', p[1]]]; },
  'mask-position': (c) => { const p = positions(c, false); return p && [['-webkit-mask-position-x', p[0]], ['-webkit-mask-position-y', p[1]]]; },
  // `-webkit-mask-position` keeps the background syntax's three-value form.
  '-webkit-mask-position': (c) => { const p = positions(c, true); return p && [['-webkit-mask-position-x', p[0]], ['-webkit-mask-position-y', p[1]]]; },
  'animation-range': (c) => rangePair(c, 'animation-range-start', 'animation-range-end', 'normal'),
  'timeline-trigger-activation-range': (c) => rangePair(c, 'timeline-trigger-activation-range-start', 'timeline-trigger-activation-range-end', 'normal'),
  'timeline-trigger-active-range': (c) => rangePair(c, 'timeline-trigger-active-range-start', 'timeline-trigger-active-range-end', 'auto'),
  'border-top': (c) => side(c, 'border-top', BORDER_SIDE),
  'border-right': (c) => side(c, 'border-right', BORDER_SIDE),
  'border-bottom': (c) => side(c, 'border-bottom', BORDER_SIDE),
  'border-left': (c) => side(c, 'border-left', BORDER_SIDE),
  'border-block-start': (c) => side(c, 'border-block-start', [INITIAL, INITIAL, INITIAL]),
  'border-block-end': (c) => side(c, 'border-block-end', [INITIAL, INITIAL, INITIAL]),
  'border-inline-start': (c) => side(c, 'border-inline-start', [INITIAL, INITIAL, INITIAL]),
  'border-inline-end': (c) => side(c, 'border-inline-end', [INITIAL, INITIAL, INITIAL]),
  'column-rule': (c) => side(c, 'column-rule', BORDER_SIDE),
  'row-rule': (c) => side(c, 'row-rule', BORDER_SIDE),
  'border-block': (c) => bothSides(c, 'border-block'),
  'border-inline': (c) => bothSides(c, 'border-inline'),
  rule: (c) => { const p = line(c, BORDER_SIDE); return p && [['column-rule-width', p[0]], ['column-rule-style', p[1]], ['column-rule-color', p[2]], ['row-rule-width', p[0]], ['row-rule-style', p[1]], ['row-rule-color', p[2]]]; },
  border: (c) => {
    const p = line(c, BORDER_SIDE);
    if (!p) return null;
    const out = [];
    ['width', 'style', 'color'].forEach((part, index) => { for (const s of ['top', 'right', 'bottom', 'left']) out.push([`border-${s}-${part}`, p[index]]); });
    ['border-image-source', 'border-image-slice', 'border-image-width', 'border-image-outset', 'border-image-repeat'].forEach((name, index) => out.push([name, BORDER_IMAGE_INITIAL[index]]));
    return out;
  },
  outline: (c) => { const p = anyOrder(c, [one(color), one(outlineStyle), one(lineWidth)]); return p && namesOrInitial(['outline-color', 'outline-style', 'outline-width'], p); },
  '-webkit-text-stroke': (c) => { const p = anyOrder(c, [one(lineWidth), one(color)]); return p && namesOrInitial(['-webkit-text-stroke-width', '-webkit-text-stroke-color'], p); },
  'text-decoration': textDecoration,
  'text-emphasis': textEmphasis,
  'flex-flow': (c) => { const p = anyOrder(c, [one((x) => keyword(x, ['row', 'row-reverse', 'column', 'column-reverse'])), one((x) => keyword(x, ['nowrap', 'wrap', 'wrap-reverse']))]); return p && [['flex-direction', p[0] ?? 'row'], ['flex-wrap', p[1] ?? 'nowrap']]; },
  'list-style': listStyle,
  columns,
  container,
  'text-wrap': (c) => { const p = anyOrder(c, [one((x) => keyword(x, ['wrap', 'nowrap'])), one((x) => keyword(x, ['auto', 'balance', 'stable', 'pretty', 'avoid-orphans']))]); return p && namesOrInitial(['text-wrap-mode', 'text-wrap-style'], p); },
  'white-space': whiteSpace,
  'font-synthesis': fontSynthesis,
  'text-box': textBox,
  'position-try': positionTry,
  'scroll-timeline': scrollTimeline,
  'view-timeline': viewTimeline,
  'font-variant': fontVariant,
  flex,
  font,
  'grid-area': gridArea,
  'grid-row': (c) => gridLines(c, 'grid-row-start', 'grid-row-end'),
  'grid-column': (c) => gridLines(c, 'grid-column-start', 'grid-column-end'),
  'grid-template': (c) => { const t = gridTemplate(c); return t && [['grid-template-rows', t[0]], ['grid-template-columns', t[1]], ['grid-template-areas', t[2]]]; },
  grid,
  'border-image': (c) => { const p = borderImage(c, false); return p && ['border-image-source', 'border-image-slice', 'border-image-width', 'border-image-outset', 'border-image-repeat'].map((name, index) => [name, p[index] ?? BORDER_IMAGE_INITIAL[index]]); },
  '-webkit-mask-box-image': (c) => { const p = borderImage(c, true); return p && namesOrInitial(['-webkit-mask-box-image-source', '-webkit-mask-box-image-slice', '-webkit-mask-box-image-width', '-webkit-mask-box-image-outset', '-webkit-mask-box-image-repeat'], p); },
  offset,
  background,
  mask: maskShorthand,
  transition,
  animation,
  'timeline-trigger': timelineTrigger,
};
function bothSides(comps, name) {
  const p = line(comps, BORDER_SIDE);
  return p && [[`${name}-start-width`, p[0]], [`${name}-end-width`, p[0]], [`${name}-start-style`, p[1]], [`${name}-end-style`, p[1]], [`${name}-start-color`, p[2]], [`${name}-end-color`, p[2]]];
}

// -------------------------------------------------------------- serializers ----
/** Side values collapsed the `margin` way. */
function boxText([top, right, bottom, left]) {
  if (left !== right) return `${top} ${right} ${bottom} ${left}`;
  if (bottom !== top) return `${top} ${right} ${bottom}`;
  if (right !== top) return `${top} ${right}`;
  return top;
}
const pairText = (a, b) => (a === b ? a : `${a} ${b}`);
const allSame = (values) => (values.every((v) => v === values[0]) ? values[0] : '');
const joined = (parts) => parts.filter((part) => part !== '').join(' ');
/** A comma-separated list's items (commas inside functions do not split). */
function layers(value) {
  const out = [];
  let depth = 0, start = 0;
  for (let index = 0; index < value.length; index++) {
    const c = value[index];
    if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth = Math.max(0, depth - 1);
    else if (c === ',' && depth === 0) { out.push(value.slice(start, index).trim()); start = index + 1; }
  }
  out.push(value.slice(start).trim());
  return out;
}
const omitIf = (value, initial) => (value === INITIAL || value === initial ? '' : value);
function lineText(width, style, colorValue) {
  return joined([omitIf(width, 'medium'), omitIf(style, 'none'), omitIf(colorValue, 'currentcolor')]);
}
function ruleInsetText(v, kind) {
  const [cs, ce, js, je] = ['cap-start', 'cap-end', 'junction-start', 'junction-end'].map((part) => v(`${kind}-rule-inset-${part}`));
  if ([ce, js, je].every((value) => value === cs)) return cs;
  return `${cs} ${ce} / ${js} ${je}`;
}
function rangeText(start, end, initial) {
  const name = start.split(' ')[0];
  const fallback = RANGE_NAMES.includes(name) ? name : initial === 'auto' && start === 'normal' ? 'auto' : initial;
  // An active range's `auto` end is left out too.
  return end === fallback || (initial === 'auto' && end === 'auto') ? start : `${start} ${end}`;
}
/** `rangeText` per item of comma lists (a timeline trigger list). */
function rangesText(start, end, initial) {
  const starts = layers(start), ends = layers(end);
  return starts.length === ends.length ? starts.map((s, index) => rangeText(s, ends[index], initial)).join(', ') : '';
}
/** A trigger range's parts that differ from `initial` (the end only when it is not its start's default). */
function triggerParts(start, end, initial) {
  const name = start.split(' ')[0], parts = [];
  if (start !== initial) parts.push(start);
  if (end !== initial && end !== name) parts.push(end);
  return parts.join(' ');
}
/** One timeline trigger: each part that differs from its initial value; the active range follows a slash when its start is set, even after nothing. */
function triggerText(name, source, activation, active) {
  let text = joined([name === 'none' ? '' : name, source === 'auto' ? '' : source, triggerParts(activation[0], activation[1], 'normal')]);
  if (active[0] && active[0] !== 'auto') text = `${text} / ${triggerParts(active[0], active[1], 'auto')}`;
  else if (active[1] && active[1] !== 'auto') text = joined([text, active[1]]);
  return text || 'none';
}
const STRETCH_KEYWORDS = { '50%': 'ultra-condensed', '62.5%': 'extra-condensed', '75%': 'condensed', '87.5%': 'semi-condensed', '100%': 'normal', '112.5%': 'semi-expanded', '125%': 'expanded', '150%': 'extra-expanded', '200%': 'ultra-expanded' };
function fontText(v) {
  for (const [name, initial] of [['font-variant-ligatures', 'normal'], ['font-variant-numeric', 'normal'], ['font-variant-east-asian', 'normal'], ['font-variant-alternates', 'normal'], ['font-variant-position', 'normal'], ['font-variant-emoji', 'normal'], ['font-size-adjust', 'none'], ['font-kerning', 'auto'], ['font-optical-sizing', 'auto'], ['font-feature-settings', 'normal'], ['font-variation-settings', 'normal'], ['font-language-override', 'normal']]) {
    if (v(name) !== initial) return '';
  }
  const caps = v('font-variant-caps');
  if (caps !== 'normal' && caps !== 'small-caps') return '';
  let stretch = v('font-stretch');
  if (stretch.endsWith('%')) { stretch = STRETCH_KEYWORDS[stretch]; if (!stretch) return ''; }
  const style = v('font-style'), weight = v('font-weight'), lineHeight = v('line-height');
  const size = lineHeight === 'normal' ? v('font-size') : `${v('font-size')} / ${lineHeight}`;
  return joined([style === 'normal' ? '' : style, caps === 'normal' ? '' : caps, weight === 'normal' ? '' : weight, stretch === 'normal' ? '' : stretch, size, v('font-family')]);
}
function tracks(rows) {
  const out = [];
  let current = '', depth = 0;
  for (const c of rows) {
    if (c === '[' || c === '(') { depth++; current += c; }
    else if (c === ']' || c === ')') { depth--; current += c; }
    else if (c === ' ' && depth === 0) { if (current) { out.push(current); current = ''; } }
    else current += c;
  }
  if (current) out.push(current);
  return out;
}
function quotedStrings(text) {
  const out = [];
  let current = null, escaped = false;
  for (const c of text) {
    if (current === null) { if (c === '"') current = '"'; continue; }
    current += c;
    if (escaped) escaped = false;
    else if (c === '\\') escaped = true;
    else if (c === '"') { out.push(current); current = null; }
  }
  return out;
}
function gridTemplateText(rows, cols, areas) {
  if (areas === 'none') return rows === 'none' && cols === 'none' ? 'none' : `${rows} / ${cols}`;
  const strings = quotedStrings(areas);
  const rowTracks = tracks(rows);
  if (rowTracks.filter((t) => !t.startsWith('[')).length !== strings.length) return '';
  const parts = [];
  let index = 0;
  for (const track of rowTracks) {
    if (track.startsWith('[')) parts.push(track);
    else { parts.push(strings[index++]); if (track !== 'auto') parts.push(track); }
  }
  const text = parts.join(' ');
  return cols === 'none' ? text : `${text} / ${cols}`;
}
function backgroundText(v) {
  const names = ['background-image', 'background-position-x', 'background-position-y', 'background-size', 'background-repeat', 'background-attachment', 'background-origin', 'background-clip'];
  const columns = names.map((name) => layers(v(name)));
  // As many layers as the longest list; a shorter list leaves its value out of the later
  // layers, and an empty layer past the images is dropped.
  const count = Math.max(...columns.map((c) => c.length));
  const colorValue = v('background-color');
  const out = [];
  for (let index = 0; index < count; index++) {
    const item = (column) => { const value = columns[column][index]; return value === undefined || value === INITIAL ? '' : value; };
    const x = item(1), y = item(2), size = item(3);
    // Blink writes the position's parts it has; a size needs a position, and with neither
    // part set that is `0% 0%` (` 0% 0%` when it starts the layer).
    let pos = joined([x, y]);
    if (size) { if (!pos) pos = item(0) ? '0% 0%' : ' 0% 0%'; pos += ` / ${size}`; }
    const last = index + 1 === count;
    const text = joined([item(0), pos, item(4), item(5), item(6), item(7), last && colorValue !== INITIAL ? colorValue : '']);
    if (text) out.push(text);
    else if (index < columns[0].length) out.push('none');
  }
  return out.join(', ');
}
function maskText(v) {
  const names = [['mask-image', 'none'], ['-webkit-mask-position-x', '0%'], ['-webkit-mask-position-y', '0%'], ['mask-size', 'auto'], ['mask-repeat', 'repeat'], ['mask-origin', 'border-box'], ['mask-clip', 'border-box'], ['mask-composite', 'add'], ['mask-mode', 'match-source']];
  const columns = names.map(([name]) => layers(v(name)));
  // As many layers as the longest list; a shorter list leaves its value out of the later layers.
  const count = Math.max(...columns.map((c) => c.length));
  const out = [];
  for (let index = 0; index < count; index++) {
    const raw = (column) => columns[column][index] ?? INITIAL;
    const item = (column) => { const value = raw(column); return value === names[column][1] || value === INITIAL ? '' : value; };
    const part = (column, initial) => (raw(column) === INITIAL ? initial : raw(column));
    const size = item(3), x = part(1, '0%'), y = part(2, '0%');
    // Blink's position before a size starts its layer with a space.
    const pos = !item(1) && !item(2) && !size ? '' : !item(1) && !item(2) && !item(0) ? ` ${x} ${y} / ${size}` : size ? `${x} ${y} / ${size}` : `${x} ${y}`;
    const origin = part(5, 'border-box'), clip = part(6, 'border-box');
    // `no-clip` is a clip-only keyword: it reads back without the default origin.
    const boxes = origin === 'border-box' && clip === 'border-box' ? '' : origin === clip ? origin : origin === 'border-box' && clip === 'no-clip' ? clip : `${origin} ${clip}`;
    out.push(joined([item(0), pos, item(4), boxes, item(7), item(8)]) || 'none');
  }
  return out.join(', ');
}
function transitionText(v) {
  const properties = layers(v('transition-property')), durations = layers(v('transition-duration')), timings = layers(v('transition-timing-function')), delays = layers(v('transition-delay')), behaviors = layers(v('transition-behavior'));
  const zero = (t) => t === '0s' || t === '0ms';
  // As many layers as the longest list; a shorter list leaves its value out.
  const count = Math.max(properties.length, durations.length, timings.length, delays.length, behaviors.length);
  return Array.from({ length: count }, (_, index) => {
    const property = properties[index] ?? 'all';
    const duration = durations[index] ?? '0s', timing = timings[index] ?? 'ease', delay = delays[index] ?? '0s', behavior = behaviors[index] ?? 'normal';
    const parts = [];
    if (property !== 'all') parts.push(property);
    if (!zero(duration)) parts.push(duration);
    if (timing !== 'ease') parts.push(timing);
    if (!zero(delay)) parts.push(delay);
    if (behavior !== 'normal') parts.push(behavior);
    return parts.length ? parts.join(' ') : 'all';
  }).join(', ');
}
function animationText(v) {
  if (v('animation-timeline') !== 'auto' || v('animation-range-start') !== 'normal' || v('animation-range-end') !== 'normal') return '';
  const names = ['animation-duration', 'animation-timing-function', 'animation-delay', 'animation-iteration-count', 'animation-direction', 'animation-fill-mode', 'animation-play-state', 'animation-name'];
  const columns = names.map((name) => layers(v(name)));
  // As many layers as the longest list; a shorter list leaves its value out.
  const count = Math.max(...columns.map((c) => c.length));
  const out = [];
  for (let index = 0; index < count; index++) out.push(columns.map((c) => c[index]).filter((value) => value !== undefined).join(' '));
  return out.join(', ');
}

const PAIRS = new Set(['contain-intrinsic-size', 'margin-block', 'margin-inline', 'padding-block', 'padding-inline', 'inset-block', 'inset-inline', 'scroll-margin-block', 'scroll-margin-inline', 'scroll-padding-block', 'scroll-padding-inline', 'border-block-width', 'border-block-style', 'border-block-color', 'border-inline-width', 'border-inline-style', 'border-inline-color', 'gap', 'overflow', 'overscroll-behavior', 'border-spacing', 'interest-delay', 'column-rule-inset-cap', 'column-rule-inset-junction', 'row-rule-inset-cap', 'row-rule-inset-junction', 'corner-top-shape', 'corner-right-shape', 'corner-bottom-shape', 'corner-left-shape', 'corner-block-start-shape', 'corner-block-end-shape', 'corner-inline-start-shape', 'corner-inline-end-shape']);
const SAME = new Set(['marker', 'rule-break', 'rule-color', 'rule-style', 'rule-width', 'rule-visibility-items', 'column-rule-inset-start', 'column-rule-inset-end', 'row-rule-inset-start', 'row-rule-inset-end', 'rule-inset-start', 'rule-inset-end']);
const BOXES = {
  margin: ['margin-top', 'margin-right', 'margin-bottom', 'margin-left'],
  padding: ['padding-top', 'padding-right', 'padding-bottom', 'padding-left'],
  inset: ['top', 'right', 'bottom', 'left'],
  'scroll-margin': ['scroll-margin-top', 'scroll-margin-right', 'scroll-margin-bottom', 'scroll-margin-left'],
  'scroll-padding': ['scroll-padding-top', 'scroll-padding-right', 'scroll-padding-bottom', 'scroll-padding-left'],
  'border-width': ['border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width'],
  'border-style': ['border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style'],
  'border-color': ['border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color'],
  'corner-shape': ['corner-top-left-shape', 'corner-top-right-shape', 'corner-bottom-right-shape', 'corner-bottom-left-shape'],
};

// Per-shorthand serializers (lookup table below). Each takes (v, texts, grammar) and returns the
// shorthand's spelling, '' when the values have no shorthand form.
function borderRadiusText(v) {
  const corners = ['border-top-left-radius', 'border-top-right-radius', 'border-bottom-right-radius', 'border-bottom-left-radius'].map(v);
  const horizontal = [], vertical = [];
  for (const corner of corners) { const space = corner.indexOf(' '); const h = space < 0 ? corner : corner.slice(0, space); horizontal.push(h); vertical.push(space < 0 ? h : corner.slice(space + 1)); }
  const h = boxText(horizontal);
  return horizontal.every((value, index) => value === vertical[index]) ? h : `${h} / ${boxText(vertical)}`;
}
function ruleInsetBoth(v) {
  const column = ruleInsetText(v, 'column');
  return column === ruleInsetText(v, 'row') ? column : '';
}
function ruleInsetPartText(v, part) {
  const row = pairText(v(`row-rule-inset-${part}-start`), v(`row-rule-inset-${part}-end`));
  return row === pairText(v(`column-rule-inset-${part}-start`), v(`column-rule-inset-${part}-end`)) ? row : '';
}
function positionShorthandText(texts) {
  const xs = layers(texts[0]), ys = layers(texts[1]);
  if (xs.length !== ys.length) {
    const part = (list, index) => (list[index] === undefined || list[index] === INITIAL ? '' : list[index]);
    return Array.from({ length: Math.max(xs.length, ys.length) }, (_, index) => joined([part(xs, index), part(ys, index)])).filter((layer) => layer).join(', ');
  }
  if ([...xs, ...ys].includes(INITIAL)) return '';
  return xs.map((x, index) => `${x} ${ys[index]}`).join(', ');
}
function borderSideShort(v, _texts, grammar) {
  return lineText(v(`${grammar}-width`), v(`${grammar}-style`), v(`${grammar}-color`));
}
function borderLogicalSideText(v, _texts, grammar) {
  return joined(['width', 'style', 'color'].map((part) => omitIf(v(`${grammar}-${part}`), null)));
}
function borderBothSidesText(v, _texts, grammar) {
  const [start, end] = grammar === 'rule' ? ['column-rule', 'row-rule'] : [`${grammar}-start`, `${grammar}-end`];
  const a = ['width', 'style', 'color'].map((part) => v(`${start}-${part}`));
  const b = ['width', 'style', 'color'].map((part) => v(`${end}-${part}`));
  return a.every((value, index) => value === b[index]) ? lineText(...a) : '';
}
function borderShort(v) {
  const image = ['border-image-source', 'border-image-slice', 'border-image-width', 'border-image-outset', 'border-image-repeat'].map(v);
  if (image.some((value, index) => value !== BORDER_IMAGE_INITIAL[index])) return '';
  const parts = [];
  for (const part of ['width', 'style', 'color']) {
    const sides = ['top', 'right', 'bottom', 'left'].map((s) => v(`border-${s}-${part}`));
    if (sides.some((value) => value !== sides[0])) return '';
    parts.push(sides[0]);
  }
  return lineText(...parts);
}
function textDecorationShort(v) {
  const lineValue = v('text-decoration-line');
  const text = joined([omitIf(lineValue, 'none'), omitIf(v('text-decoration-thickness'), 'auto'), omitIf(v('text-decoration-style'), 'solid'), omitIf(v('text-decoration-color'), 'currentcolor')]);
  return text || (lineValue === INITIAL ? 'none' : lineValue);
}
function flexFlowText(v) {
  const direction = v('flex-direction'), wrap = v('flex-wrap');
  return wrap === 'nowrap' ? direction : direction === 'row' ? wrap : `${direction} ${wrap}`;
}
function columnsText(v) {
  const width = v('column-width'), count = v('column-count'), height = v('column-height');
  const main = width === 'auto' && count === 'auto' ? 'auto' : width === 'auto' ? count : count === 'auto' ? width : `${width} ${count}`;
  return height === 'auto' ? main : `${main} / ${height}`;
}
function textWrapText(v) {
  const mode = v('text-wrap-mode') === INITIAL ? 'wrap' : v('text-wrap-mode');
  const style = v('text-wrap-style') === INITIAL ? 'auto' : v('text-wrap-style');
  return style === 'auto' ? mode : mode === 'wrap' ? style : `${mode} ${style}`;
}
const WHITE_SPACE_MAP = { 'collapse wrap': 'normal', 'preserve nowrap': 'pre', 'collapse nowrap': 'nowrap', 'preserve wrap': 'pre-wrap', 'preserve-breaks wrap': 'pre-line', 'break-spaces wrap': 'break-spaces' };
function whiteSpaceText(v) {
  const collapse = v('white-space-collapse') === INITIAL ? 'collapse' : v('white-space-collapse');
  const mode = v('text-wrap-mode') === INITIAL ? 'wrap' : v('text-wrap-mode');
  return WHITE_SPACE_MAP[`${collapse} ${mode}`] ?? `${collapse} ${mode}`;
}
function fontSynthesisText(v) {
  const parts = [];
  const values = ['font-synthesis-weight', 'font-synthesis-style', 'font-synthesis-small-caps'].map(v);
  for (const [index, name] of ['weight', 'style', 'small-caps'].entries()) {
    if (values[index] === 'auto') parts.push(name);
    else if (values[index] !== 'none') return '';
  }
  return parts.length ? parts.join(' ') : 'none';
}
function textBoxText(v) {
  const trim = v('text-box-trim'), edge = v('text-box-edge');
  if (trim === 'none' && edge === 'auto') return 'normal';
  if (trim === 'trim-both') return edge === 'auto' ? 'trim-both' : edge;
  return edge === 'auto' ? trim : `${trim} ${edge}`;
}
function scrollTimelineText(v) {
  const names = layers(v('scroll-timeline-name')), axes = layers(v('scroll-timeline-axis'));
  if (names.length !== axes.length) return '';
  return names.map((name, index) => (axes[index] === 'block' ? name : `${name} ${axes[index]}`)).join(', ');
}
function viewTimelineText(v) {
  const names = layers(v('view-timeline-name')), axes = layers(v('view-timeline-axis')), insets = layers(v('view-timeline-inset'));
  if (names.length !== axes.length || names.length !== insets.length) return '';
  return names.map((name, index) => joined([name, axes[index] === 'block' ? '' : axes[index], insets[index] === 'auto' ? '' : insets[index]])).join(', ');
}
function fontVariantText(v) {
  const values = ['font-variant-ligatures', 'font-variant-caps', 'font-variant-alternates', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-position', 'font-variant-emoji'].map(v);
  if (values.every((value) => value === 'normal')) return 'normal';
  if (values[0] === 'none') return values.slice(1).every((value) => value === 'normal') ? 'none' : '';
  return joined(values.map((value) => (value === 'normal' ? '' : value)));
}
function gridAreaText(v) {
  const [rs, cs, re, ce] = ['grid-row-start', 'grid-column-start', 'grid-row-end', 'grid-column-end'].map(v);
  const parts = [rs, cs, re, ce];
  if (ce === lineDefault(cs)) { parts.pop(); if (re === lineDefault(rs)) { parts.pop(); if (cs === lineDefault(rs)) parts.pop(); } }
  return parts.join(' / ');
}
function gridFlowText(rows, cols, flow, autoCols, autoRows) {
  const dense = flow.endsWith('dense');
  if ((flow.startsWith('row') || flow === 'dense') && rows === 'none' && autoCols === 'auto') return `${joined(['auto-flow', dense ? 'dense' : '', autoRows === 'auto' ? '' : autoRows])} / ${cols}`;
  if (flow.startsWith('column') && cols === 'none' && autoRows === 'auto') return `${rows} / ${joined(['auto-flow', dense ? 'dense' : '', autoCols === 'auto' ? '' : autoCols])}`;
  return '';
}
function gridText(v) {
  const rows = v('grid-template-rows'), cols = v('grid-template-columns'), areas = v('grid-template-areas'), flow = v('grid-auto-flow'), autoCols = v('grid-auto-columns'), autoRows = v('grid-auto-rows');
  if (flow === 'row' && autoRows === 'auto' && autoCols === 'auto') return gridTemplateText(rows, cols, areas);
  if (areas !== 'none') return '';
  return gridFlowText(rows, cols, flow, autoCols, autoRows);
}
function borderImageText(v) {
  const [source, slice, width, outsetValue, repeat] = ['border-image-source', 'border-image-slice', 'border-image-width', 'border-image-outset', 'border-image-repeat'].map(v);
  if (slice === '100%' && width === '1' && outsetValue === '0' && repeat === 'stretch') return source;
  return `${source} ${slice} / ${width} / ${outsetValue} ${repeat}`;
}
function offsetText(v) {
  const [pos, path, distance, rotate, anchor] = ['offset-position', 'offset-path', 'offset-distance', 'offset-rotate', 'offset-anchor'].map(v);
  const moved = distance !== '0px' || rotate !== 'auto';
  const main = joined([pos === 'normal' ? '' : pos, path === 'none' && !moved ? '' : path, distance === '0px' ? '' : distance, rotate === 'auto' ? '' : rotate]) || 'normal';
  return anchor === 'auto' ? main : `${main} / ${anchor}`;
}
function timelineTriggerText(v) {
  const columns = ['timeline-trigger-name', 'timeline-trigger-source', 'timeline-trigger-activation-range-start', 'timeline-trigger-activation-range-end', 'timeline-trigger-active-range-start', 'timeline-trigger-active-range-end'].map((name) => layers(v(name)));
  const count = columns[0].length;
  if (columns.some((column) => column.length !== count)) return '';
  return Array.from({ length: count }, (_, index) => {
    const [name, source, as, ae, bs, be] = columns.map((column) => column[index]);
    return triggerText(name, source, [as, ae], [bs, be]);
  }).join(', ');
}

const SERIALIZERS = {
  'border-radius': (v) => borderRadiusText(v),
  '-webkit-border-radius': (v) => borderRadiusText(v),
  'column-rule-inset': (v) => ruleInsetText(v, 'column'),
  'row-rule-inset': (v) => ruleInsetText(v, 'row'),
  'rule-inset': ruleInsetBoth,
  'rule-inset-cap': (v) => ruleInsetPartText(v, 'cap'),
  'rule-inset-junction': (v) => ruleInsetPartText(v, 'junction'),
  'place-items': (v) => pairText(v('align-items'), v('justify-items')),
  'place-self': (v) => pairText(v('align-self'), v('justify-self')),
  'place-content': (v) => pairText(v('align-content'), v('justify-content')),
  'background-position': (_v, texts) => positionShorthandText(texts),
  'mask-position': (_v, texts) => positionShorthandText(texts),
  '-webkit-mask-position': (_v, texts) => positionShorthandText(texts),
  'animation-range': (v) => rangeText(v('animation-range-start'), v('animation-range-end'), 'normal'),
  'timeline-trigger-activation-range': (v) => rangesText(v('timeline-trigger-activation-range-start'), v('timeline-trigger-activation-range-end'), 'normal'),
  'timeline-trigger-active-range': (v) => rangesText(v('timeline-trigger-active-range-start'), v('timeline-trigger-active-range-end'), 'auto'),
  'border-top': borderSideShort,
  'border-right': borderSideShort,
  'border-bottom': borderSideShort,
  'border-left': borderSideShort,
  'column-rule': borderSideShort,
  'row-rule': borderSideShort,
  'border-block-start': borderLogicalSideText,
  'border-block-end': borderLogicalSideText,
  'border-inline-start': borderLogicalSideText,
  'border-inline-end': borderLogicalSideText,
  'border-block': borderBothSidesText,
  'border-inline': borderBothSidesText,
  rule: borderBothSidesText,
  border: borderShort,
  outline: (v) => joined(['outline-color', 'outline-style', 'outline-width'].map((name) => omitIf(v(name), null))),
  '-webkit-text-stroke': (v) => joined(['-webkit-text-stroke-width', '-webkit-text-stroke-color'].map((name) => omitIf(v(name), null))),
  'text-decoration': textDecorationShort,
  'text-emphasis': (v) => joined([omitIf(v('text-emphasis-style'), 'none'), omitIf(v('text-emphasis-color'), 'currentcolor')]) || 'none',
  'flex-flow': flexFlowText,
  'list-style': (v) => joined(['list-style-position', 'list-style-image', 'list-style-type'].map((name) => omitIf(v(name), null))),
  columns: columnsText,
  container: (v) => { const kind = v('container-type'); return kind === 'normal' ? v('container-name') : `${v('container-name')} / ${kind}`; },
  'text-wrap': textWrapText,
  'white-space': whiteSpaceText,
  'font-synthesis': fontSynthesisText,
  'text-box': textBoxText,
  'position-try': (v) => (v('position-try-order') === 'normal' ? v('position-try-fallbacks') : `${v('position-try-order')} ${v('position-try-fallbacks')}`),
  'scroll-timeline': scrollTimelineText,
  'view-timeline': viewTimelineText,
  'font-variant': fontVariantText,
  flex: (v) => `${v('flex-grow')} ${v('flex-shrink')} ${v('flex-basis')}`,
  font: (v) => fontText(v),
  'grid-area': gridAreaText,
  'grid-row': (_v, texts) => (texts[1] === lineDefault(texts[0]) ? texts[0] : `${texts[0]} / ${texts[1]}`),
  'grid-column': (_v, texts) => (texts[1] === lineDefault(texts[0]) ? texts[0] : `${texts[0]} / ${texts[1]}`),
  'grid-template': (v) => gridTemplateText(v('grid-template-rows'), v('grid-template-columns'), v('grid-template-areas')),
  grid: gridText,
  'border-image': borderImageText,
  offset: offsetText,
  background: (v) => backgroundText(v),
  mask: (v) => maskText(v),
  transition: (v) => transitionText(v),
  animation: (v) => animationText(v),
  'timeline-trigger': timelineTriggerText,
};

/** Serialize `grammar` from its longhands' values (`texts` in table order). */
function serializeValue(grammar, entry, texts) {
  const v = (name) => { const index = entry.longhands.indexOf(name); return index < 0 ? '' : texts[index]; };
  if (BOXES[grammar]) return boxText(BOXES[grammar].map(v));
  if (PAIRS.has(grammar)) return pairText(texts[0], texts[1]);
  if (SAME.has(grammar)) return allSame(texts);
  const serializer = SERIALIZERS[grammar];
  return serializer ? serializer(v, texts, grammar) : '';
}

// ------------------------------------------------------------------- table ----
const ENTRIES = new Map(SHORTHAND_TABLE.map(([name, canonical, grammar, allowInitial, longhands]) => [name, { name, canonical, grammar, allowInitial, longhands }]));
let preferred = null;
function preferredEntries() {
  if (!preferred) {
    preferred = [...ENTRIES.values()].filter((entry) => entry.name === entry.canonical)
      .sort((a, b) => b.longhands.length - a.longhands.length || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  }
  return preferred;
}
const ALIASES = new Map(LONGHAND_ALIASES);

/** Every shorthand (and shorthand alias) name, sorted. */
export const shorthandNames = () => [...ENTRIES.keys()];
/** The shorthand's canonical name (`-webkit-flex` -> `flex`), or null for a non-shorthand. */
export const canonical = (name) => ENTRIES.get(name)?.canonical ?? null;
/** The longhands `name` sets, in Chrome's order; null for a non-shorthand. */
export const longhands = (name) => ENTRIES.get(name)?.longhands ?? null;
/** The longhand a longhand alias names (`word-wrap` -> `overflow-wrap`), or null. */
export const longhandAlias = (name) => ALIASES.get(name) ?? null;
export { LONGHAND_ALIASES };

// --------------------------------------------------------------------- api ----
/**
 * Set shorthand `name` to `value`: `[{ name, value, pending }]` in the order Chrome adds them
 * (`pending` = [shorthand, text] for a var() value, whose longhand values are ''), or null when
 * the value is invalid for the shorthand.
 */
export function expand(name, value) {
  const entry = ENTRIES.get(name);
  if (!entry) return null;
  const trimmed = String(value).trim();
  if (!trimmed) return null;
  const lower = trimmed.toLowerCase();
  if (CSS_WIDE.has(lower)) return entry.longhands.map((longhand) => ({ name: longhand, value: lower, pending: null }));
  if (hasSubstitution(trimmed)) {
    if (components(trimmed) === null) return null;
    const text = substitutionText(trimmed);
    return entry.longhands.map((longhand) => ({ name: longhand, value: '', pending: [entry.canonical, text] }));
  }
  const comps = components(trimmed);
  if (!comps || !comps.length) return null;
  const grammar = GRAMMARS[entry.grammar];
  const parsed = grammar ? grammar(comps) : null;
  if (!parsed) return null;
  if (parsed.pending) return entry.longhands.map((longhand) => ({ name: longhand, value: '', pending: [entry.canonical, parsed.pending] }));
  return parsed.map(([longhand, text]) => ({ name: longhand, value: text, pending: null }));
}

/**
 * getPropertyValue(shorthand) from `declarations` ([{ name, value, important, pending }]):
 * '' when a longhand is missing, importance is mixed, or the values have no shorthand spelling.
 */
function collectLonghandValues(longhands, declarations) {
  const values = [];
  for (const longhand of longhands) {
    let found = null;
    for (let index = declarations.length - 1; index >= 0; index--) if (declarations[index].name === longhand) { found = declarations[index]; break; }
    if (!found) return null;
    values.push(found);
  }
  return values;
}
const samePending = (a, b) => (a === null || a === undefined ? b === null || b === undefined : !!b && a[0] === b[0] && a[1] === b[1]);
function pendingShorthandText(first, entry) {
  // A system font keyword leaves the font-variant longhands at `normal`.
  if (first.pending[0] === 'font' && entry.canonical === 'font-variant' && SYSTEM_FONTS.includes(first.pending[1])) return 'normal';
  return first.pending[0] === entry.canonical ? first.pending[1] : '';
}
function shorthandUniformText(values, entry) {
  const first = values[0];
  if (!first.pending && !CSS_WIDE.has(first.value)) return null;
  if (!values.every((v) => v.value === first.value && samePending(first.pending, v.pending))) return null;
  return first.pending ? pendingShorthandText(first, entry) : first.value;
}
export function shorthandValue(name, declarations) {
  const entry = ENTRIES.get(name);
  if (!entry) return '';
  const values = collectLonghandValues(entry.longhands, declarations);
  if (!values) return '';
  if (values.some((v) => v.important) && values.some((v) => !v.important)) return '';
  const uniform = shorthandUniformText(values, entry);
  if (uniform !== null) return uniform;
  for (const v of values) {
    if (v.pending || (CSS_WIDE.has(v.value) && (v.value !== 'initial' || !entry.allowInitial))) return '';
  }
  return serializeValue(entry.grammar, ENTRIES.get(entry.grammar) ?? entry, values.map((v) => v.value));
}

/** cssText: the CSSOM "serialize a CSS declaration block" algorithm with Chrome's shorthand preference. */
export function blockText(declarations) {
  const done = declarations.map(() => false);
  const indexOf = new Map();
  declarations.forEach((d, index) => { if (!indexOf.has(d.name)) indexOf.set(d.name, index); });
  const parts = [];
  const push = (name, value, important) => parts.push(`${name}: ${value}${important ? ' !important' : ''};`);
  declarations.forEach((declaration, index) => {
    if (done[index]) return;
    for (const entry of preferredEntries()) {
      if (!entry.longhands.includes(declaration.name)) continue;
      const positions = entry.longhands.map((longhand) => indexOf.get(longhand));
      if (positions.some((p) => p === undefined || done[p])) continue;
      const value = shorthandValue(entry.canonical, declarations);
      if (!value) continue;
      push(entry.canonical, value, declaration.important);
      for (const p of positions) done[p] = true;
      return;
    }
    push(declaration.name, declaration.value, declaration.important);
    done[index] = true;
  });
  return parts.join(' ');
}

/** A declaration list's `[name, value, important]` declarations, in order. */
function declarationsScanInQuote(source, state, c) {
  if (c === '\\') state.index++;
  else if (c === state.quote) state.quote = '';
}
function declarationsScanBracket(state, c) {
  if (c === '(' || c === '[' || c === '{') { state.depth++; return true; }
  if (c === ')' || c === ']' || c === '}') { state.depth = Math.max(0, state.depth - 1); return true; }
  return false;
}
function declarationsScan(source, state) {
  const c = source[state.index];
  if (state.quote) { declarationsScanInQuote(source, state, c); return 'skip'; }
  if (c === '"' || c === "'") { state.quote = c; return 'skip'; }
  if (c === '/' && source[state.index + 1] === '*') { const end = source.indexOf('*/', state.index + 2); state.index = end < 0 ? source.length : end + 1; return 'skip'; }
  if (declarationsScanBracket(state, c)) return 'skip';
  if (c !== ';' || state.depth > 0) return 'skip';
  return 'split';
}
function parseDeclarationPart(part) {
  const colon = part.indexOf(':');
  if (colon < 0) return null;
  const name = part.slice(0, colon).trim();
  if (!/^-?[A-Za-z_-][A-Za-z0-9_-]*$/.test(name)) return null;
  let value = part.slice(colon + 1).trim();
  const important = /!\s*important\s*$/i.exec(value);
  if (important) value = value.slice(0, important.index).trim();
  else if (value.includes('!')) return null;
  return [name, value, !!important];
}
export function declarations(text) {
  const tokens = tokenize(String(text));
  const out = [];
  if (!tokens) return out;
  const source = String(text);
  const state = { depth: 0, quote: '', index: 0 };
  let start = 0;
  for (state.index = 0; state.index <= source.length; state.index++) {
    if (state.index < source.length && declarationsScan(source, state) === 'skip') continue;
    const part = source.slice(start, state.index).replace(/\/\*[\s\S]*?\*\//g, '');
    start = state.index + 1;
    const parsed = parseDeclarationPart(part);
    if (parsed) out.push(parsed);
  }
  return out;
}

// ---------------------------------------------------------------- longhands ----
// Longhand specified values the way Chrome 156 spells them (miaskiewicz/turbo-dom#97; the port
// of rtdom::css_shorthand::longhand_value): the longhands of the shorthands (and a few common
// ones) go through their value grammar ({ valid, text } or { valid: false }); any other
// longhand gets the context-free spelling ({ valid: null, text }).
const listed = (comps, item) => { const items = commas(comps); if (!items) return null; const out = items.map(item); return out.some((v) => v === null) ? null : out.join(', '); };
const keywords = (comps, names) => (comps.length === 1 ? keyword(comps[0], names) : null);
const oneOf = (ty) => (comps) => single(comps, ty);
function radiusPair(comps) {
  if (comps.length === 1) return radius(comps[0]);
  if (comps.length !== 2) return null;
  const a = radius(comps[0]), b = radius(comps[1]);
  return a === null || b === null ? null : a === b ? a : `${a} ${b}`;
}
const flexBasis = (comps) => single(comps, (c) => keyword(c, ['auto', 'content', 'min-content', 'max-content', 'fit-content']) ?? lengthPercentage(c, false));
const fontSize = (comps) => single(comps, (c) => keyword(c, FONT_SIZES) ?? lengthPercentage(c, false));
const fontWeight = (comps) => single(comps, (c) => keyword(c, ['normal', 'bold', 'bolder', 'lighter']) ?? (c.kind !== 'number' || (c.value >= 1 && c.value <= 1000) ? number(c, false) : null));
// (A literal weight is in [1, 1000]; a math function is checked when computed.)
function fontStyle(comps) {
  if (comps.length === 1) return keyword(comps[0], ['normal', 'italic', 'oblique']);
  if (comps.length === 2 && isIdent(comps[0], ['oblique'])) return oblique(comps[1]);
  return null;
}
const lineHeight = (comps) => single(comps, (c) => keyword(c, ['normal']) ?? number(c, false) ?? lengthPercentage(c, false));
const fontStretch = (comps) => single(comps, (c) => keyword(c, ['normal', ...STRETCHES]) ?? percentage(c, false));
function variantGroups(comps, groups, extra) {
  if (comps.length === 1) { const value = keyword(comps[0], extra); if (value !== null) return value; }
  const result = groupSet(comps, groups);
  return result && result[0] === comps.length ? result[1] : null;
}
const zIndex = (comps) => single(comps, (c) => keyword(c, ['auto']) ?? integer(c, -Infinity));
const SIZE_FUNCTIONS = ['anchor-size', 'calc-size'];
const sizeV = (comps) => single(comps, (c) => keyword(c, ['auto', 'min-content', 'max-content', 'fit-content', 'stretch', '-webkit-fill-available']) ?? lengthPercentage(c, false) ?? functionNamed(c, SIZE_FUNCTIONS));
const maxSizeV = (comps) => single(comps, (c) => keyword(c, ['none', 'min-content', 'max-content', 'fit-content', 'stretch', '-webkit-fill-available']) ?? lengthPercentage(c, false) ?? functionNamed(c, SIZE_FUNCTIONS));
const imageList = (comps) => listed(comps, (item) => single(item, (c) => image(c) ?? keyword(c, ['none'])));
const repeatList = (comps) => listed(comps, (item) => { const r = repeatStyle(item); return r && r[0] === item.length ? r[1] : null; });
const sizeList = (comps) => listed(comps, (item) => { const r = bgSize(item); return r && r[0] === item.length ? r[1] : null; });
function axisPosition(sides, center) {
  return (comps) => listed(comps, (item) => {
    if (item.length === 1) return keyword(item[0], [sides[0], center, sides[1]]) ?? lengthPercentage(item[0], true);
    if (item.length === 2 && isIdent(item[0], sides)) { const offset = lengthPercentage(item[1], true); return offset === null ? null : `${ident(item[0])} ${offset}`; }
    return null;
  });
}
const timeList = (comps) => listed(comps, (item) => single(item, (c) => time(c, true)));
const durationList = (comps) => listed(comps, (item) => single(item, (c) => time(c, false)));
const easingList = (comps) => listed(comps, (item) => single(item, easing));
// `none` only stands alone.
const transitionPropertyV = (comps) => (comps.length > 1 && comps.some((c) => isIdent(c, ['none'])) ? null : transitionPropertyList(comps));
const transitionPropertyList = (comps) => listed(comps, (item) => single(item, (c) => keyword(c, ['none', 'all']) ?? (() => { const v = customIdent(c, ['none', 'initial']); return v === null ? null : v.startsWith('--') ? v : v.toLowerCase(); })()));
const rangeOf = (end) => (comps) => { const r = rangeValue(comps, 'normal', end); return r && r[0] === comps.length ? r[1] : null; };
function textDecorationLine(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return 'none';
  const order = ['underline', 'overline', 'line-through', 'blink', 'spelling-error', 'grammar-error'];
  const seen = [];
  for (const comp of comps) { const name = keyword(comp, order); if (name === null || seen.includes(name)) return null; seen.push(name); }
  return seen.length ? order.filter((n) => seen.includes(n)).join(' ') : null;
}
function gridAreas(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return 'none';
  return areasText(comps.map(areaRow));
}
const COORD_BOXES = ['content-box', 'padding-box', 'border-box', 'fill-box', 'stroke-box', 'view-box'];
/** `clip-path`'s `shape() || <geometry-box>` and `path()`; the embedder's grammar decides the rest. */
const clipPath = (comps) => shapeWithBox(comps, [...COORD_BOXES, 'margin-box'], 'border-box') ?? single(comps, (c) => {
  const value = functionNamed(c, ['path']);
  return value !== null && pathDataValid(value) ? pathText(value) : null;
});
/** `shape-outside`'s `shape() || <shape-box>`; the embedder's grammar decides the rest. */
const shapeOutside = (comps) => shapeWithBox(comps, ['margin-box', 'border-box', 'padding-box', 'content-box'], 'margin-box');
const offsetPath = (comps) => (comps.some((c) => functionNamed(c, ['shape']) !== null) ? shapeWithBox(comps, COORD_BOXES, 'border-box') : single(comps, (c) => {
  if (isIdent(c, ['none'])) return 'none';
  const value = functionNamed(c, ['path', 'ray', 'circle', 'ellipse', 'inset', 'polygon', 'xywh', 'rect']) ?? url(c);
  if (value === null || (value.startsWith('path(') && !pathDataValid(value))) return null;
  return value.startsWith('path(') ? pathText(value) : value;
}));
// A percentage calc() stays as written.
const opacityV = (comps) => single(comps, (c) => number(c, true) ?? (c.kind === 'percentage' ? cssNumber(c.value / 100) : mathOf(c, isLengthUnit, true, false) && c.text.includes('%') ? c.text : null));
const DISPLAY_KEYWORDS = ['none', 'contents', 'block', 'inline', 'inline-block', 'flow-root', 'flex', 'inline-flex', 'grid', 'inline-grid', 'table', 'inline-table', 'table-row-group', 'table-header-group', 'table-footer-group', 'table-row', 'table-cell', 'table-column-group', 'table-column', 'table-caption', 'list-item', 'ruby', 'ruby-text', 'math', '-webkit-box', '-webkit-inline-box'];
const DISPLAY_PAIRS = { 'block flow': 'block', 'inline flow': 'inline', 'inline flow-root': 'inline-block', 'block flow-root': 'flow-root', 'block flex': 'flex', 'inline flex': 'inline-flex', 'block grid': 'grid', 'inline grid': 'inline-grid', 'block table': 'table', 'inline table': 'inline-table', 'block ruby': 'block ruby', 'inline ruby': 'ruby' };
function display(comps) {
  if (comps.length === 1) return keyword(comps[0], DISPLAY_KEYWORDS);
  if (comps.length !== 2) return null;
  const a = ident(comps[0]), b = ident(comps[1]);
  if (a === null || b === null) return null;
  return DISPLAY_PAIRS[`${a} ${b}`] ?? DISPLAY_PAIRS[`${b} ${a}`] ?? null;
}
const animationNames = (comps) => listed(comps, (item) => single(item, (c) => {
  if (isIdent(c, ['none'])) return 'none';
  if (c.kind === 'string') return animationNameString(c);
  return customIdent(c, ['none']);
}));
function shadowItemStep(state, comp, box) {
  if (box && isIdent(comp, ['inset']) && !state.inset) { state.inset = true; return true; }
  // The blur radius (the third length) is not negative.
  const negative = comp.kind === 'dimension' && comp.value < 0;
  const l = state.lengths.length < (box ? 4 : 3) && !(state.lengths.length === 2 && negative) ? length(comp, true) : null;
  if (l !== null) { state.lengths.push(l); return true; }
  const c = state.shadowColor === null ? color(comp) : null;
  if (c === null) return false;
  state.shadowColor = c;
  return true;
}
function shadowItem(item, box) {
  const state = { shadowColor: null, inset: false, lengths: [] };
  for (const comp of item) if (!shadowItemStep(state, comp, box)) return null;
  if (state.lengths.length < 2) return null;
  return [...(state.shadowColor === null ? [] : [state.shadowColor]), ...state.lengths, ...(state.inset ? ['inset'] : [])].join(' ');
}
/** `box-shadow` / `text-shadow` (`box`: a spread length and `inset`): each shadow as Chrome orders it (color, lengths, `inset`). */
function shadow(comps, box) {
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return 'none';
  return listed(comps, (item) => shadowItem(item, box));
}
const SIDES4 = ['top', 'right', 'bottom', 'left'];
const LOGICAL = ['block-start', 'block-end', 'inline-start', 'inline-end'];
const LONGHAND_GRAMMARS = new Map();
/** A position longhand: one, two or (`four`) four values, and for `transform-origin` (`z`) an optional depth length after two. */
function origin(comps, four, z) {
  const pairOf = (list) => { const p = position(list); return p ? `${p[0]} ${p[1]}` : null; };
  if (comps.length === 1 || comps.length === 2) return pairOf(comps);
  if (comps.length === 3 && z) { const p = pairOf(comps.slice(0, 2)), depth = length(comps[2], true); return p === null || depth === null ? null : `${p} ${depth}`; }
  if (comps.length === 4 && four) return pairOf(comps);
  return null;
}
/** One to four values, collapsed the way Chrome writes a box. */
function quad(comps, ty) {
  if (!comps.length || comps.length > 4) return null;
  const values = comps.map(ty);
  return values.includes(null) ? null : collapse4(values);
}
/** `border-image-slice`: up to four numbers / percentages, `fill` (before or after) last. */
function imageSlice(comps) {
  let values = comps, filled = false;
  if (comps.length && isIdent(comps[0], ['fill'])) { values = comps.slice(1); filled = true; }
  else if (comps.length && isIdent(comps[comps.length - 1], ['fill'])) { values = comps.slice(0, -1); filled = true; }
  if (!values.length || values.length > 4) return null;
  const texts = values.map((c) => number(c, false) ?? percentage(c, false));
  if (texts.includes(null)) return null;
  return filled ? `${collapse4(texts)} fill` : collapse4(texts);
}
const add = (names, parse, strict = true) => { for (const name of names) LONGHAND_GRAMMARS.set(name, [parse, strict]); };
add([...SIDES4.map((s) => `margin-${s}`), ...LOGICAL.map((s) => `margin-${s}`), ...SIDES4, ...LOGICAL.map((s) => `inset-${s}`)], oneOf(marginV));
add([...SIDES4, ...LOGICAL].map((s) => `padding-${s}`), oneOf(paddingV));
add([...SIDES4, ...LOGICAL].map((s) => `scroll-margin-${s}`), oneOf(scrollMarginV));
add([...SIDES4, ...LOGICAL].map((s) => `scroll-padding-${s}`), oneOf(scrollPaddingV));
add([...[...SIDES4, ...LOGICAL].map((s) => `border-${s}-width`), 'outline-width', 'column-rule-width', 'row-rule-width', '-webkit-text-stroke-width'], oneOf(lineWidth));
add([...[...SIDES4, ...LOGICAL].map((s) => `border-${s}-style`), 'column-rule-style', 'row-rule-style'], oneOf(lineStyle));
add(['outline-style'], oneOf(outlineStyle));
add([...[...SIDES4, ...LOGICAL].map((s) => `border-${s}-color`), 'outline-color', 'column-rule-color', 'row-rule-color', 'text-decoration-color', 'text-emphasis-color', '-webkit-text-stroke-color', 'background-color', 'color'], oneOf(color));
add(['row-gap', 'column-gap'], oneOf(gapV));
add(['overflow-x', 'overflow-y'], oneOf(overflowV));
add(['overscroll-behavior-x', 'overscroll-behavior-y'], oneOf(overscrollV));
add(['-webkit-border-horizontal-spacing', '-webkit-border-vertical-spacing'], oneOf(spacingV));
add(['interest-delay-start', 'interest-delay-end'], oneOf(delayV));
add(['column', 'row'].flatMap((k) => ['cap-start', 'cap-end', 'junction-start', 'junction-end'].map((p) => `${k}-rule-inset-${p}`)), oneOf(ruleInsetV));
add(['column-rule-break', 'row-rule-break'], oneOf(ruleBreakV));
add(['column-rule-visibility-items', 'row-rule-visibility-items'], oneOf(ruleVisibilityV));
add(['marker-start', 'marker-mid', 'marker-end'], oneOf(markerV));
add(['top-left', 'top-right', 'bottom-right', 'bottom-left', 'start-start', 'start-end', 'end-start', 'end-end'].map((c) => `corner-${c}-shape`), oneOf(cornerShape));
add(['top-left', 'top-right', 'bottom-right', 'bottom-left'].map((c) => `border-${c}-radius`), radiusPair);
add(['flex-grow', 'flex-shrink'], (c) => single(c, (x) => number(x, false)));
add(['flex-basis'], flexBasis);
add(['flex-direction'], (c) => keywords(c, ['row', 'row-reverse', 'column', 'column-reverse']));
add(['flex-wrap'], (c) => keywords(c, ['nowrap', 'wrap', 'wrap-reverse']));
add(['font-size'], fontSize);
add(['font-weight'], fontWeight);
add(['font-style'], fontStyle);
add(['font-family'], fontFamily);
add(['line-height'], lineHeight);
add(['font-stretch'], fontStretch);
add(['font-variant-caps'], (c) => keywords(c, ['normal', ...CAPS]));
add(['font-variant-position'], (c) => keywords(c, ['normal', 'sub', 'super']));
add(['font-variant-emoji'], (c) => keywords(c, ['normal', 'text', 'emoji', 'unicode']));
add(['font-variant-ligatures'], (c) => variantGroups(c, LIGATURES, ['normal', 'none']));
add(['font-variant-numeric'], (c) => variantGroups(c, NUMERIC, ['normal']));
add(['font-variant-east-asian'], (c) => variantGroups(c, EAST_ASIAN, ['normal']));
add(['font-kerning'], (c) => keywords(c, ['auto', 'normal', 'none']));
add(['font-optical-sizing'], (c) => keywords(c, ['auto', 'none']));
add(['font-synthesis-weight', 'font-synthesis-style', 'font-synthesis-small-caps'], (c) => keywords(c, ['auto', 'none']));
add(['position-try-fallbacks'], (c) => (c.length === 1 && isIdent(c[0], ['none']) ? 'none' : tryFallbacks(c)));
add(['position-try-order'], (c) => keywords(c, ['normal', 'most-width', 'most-height', 'most-block-size', 'most-inline-size']));
add(['text-decoration-line'], textDecorationLine);
add(['text-decoration-style'], (c) => keywords(c, ['solid', 'double', 'dotted', 'dashed', 'wavy']));
add(['list-style-position'], (c) => keywords(c, ['inside', 'outside']));
add(['list-style-image'], (c) => single(c, (x) => image(x) ?? keyword(x, ['none'])), false);
add(['z-index'], zIndex);
add(['order'], (comps) => single(comps, (c) => integer(c, -Infinity)));
add(['width', 'height', 'min-width', 'min-height', 'block-size', 'inline-size', 'min-block-size', 'min-inline-size'], sizeV);
add(['max-width', 'max-height', 'max-block-size', 'max-inline-size'], maxSizeV);
add(['column-count'], (c) => single(c, (x) => keyword(x, ['auto']) ?? integer(x, 1)));
add(['column-width', 'column-height'], (c) => single(c, (x) => keyword(x, ['auto']) ?? length(x, false)));
add(['column-wrap'], (c) => keywords(c, ['auto', 'nowrap', 'wrap']));
add(['column-span'], (c) => keywords(c, ['none', 'all']));
add(['column-fill'], (c) => keywords(c, ['auto', 'balance']));
add(['break-before', 'break-after'], (c) => keywords(c, ['auto', 'avoid', 'avoid-page', 'page', 'left', 'right', 'recto', 'verso', 'avoid-column', 'column']));
add(['break-inside'], (c) => keywords(c, ['auto', 'avoid', 'avoid-page', 'avoid-column']));
add(['grid-row-start', 'grid-row-end', 'grid-column-start', 'grid-column-end'], gridLine);
add(['grid-template-columns', 'grid-template-rows'], trackList);
add(['transition-duration'], durationList);
add(['animation-duration'], (c) => listed(c, (item) => single(item, (x) => time(x, false) ?? keyword(x, ['auto']))));
add(['font-variant-alternates'], (c) => { if (c.length === 1 && isIdent(c[0], ['normal'])) return 'normal'; const a = alternates(c); return a && a[0] === c.length ? a[1] : null; });
add(['text-align'], (c) => keywords(c, ['start', 'end', 'left', 'right', 'center', 'justify', 'match-parent', '-webkit-left', '-webkit-right', '-webkit-center', '-webkit-auto']));
add(['transition-delay', 'animation-delay'], timeList);
add(['transition-timing-function', 'animation-timing-function'], easingList, false);
add(['transition-property'], transitionPropertyV);
add(['background-image', 'mask-image'], imageList, false);
add(['background-repeat', 'mask-repeat'], repeatList);
add(['background-size', 'mask-size'], sizeList);
add(['background-attachment'], (c) => listed(c, (item) => keywords(item, ['scroll', 'fixed', 'local'])));
add(['background-origin'], (c) => listed(c, (item) => keywords(item, ['border-box', 'padding-box', 'content-box'])));
add(['background-clip'], (c) => listed(c, (item) => keywords(item, ['border-box', 'padding-box', 'content-box', 'text', 'border-area'])));
add(['background-position-x', '-webkit-mask-position-x'], axisPosition(['left', 'right'], 'center'));
add(['background-position-y', '-webkit-mask-position-y'], axisPosition(['top', 'bottom'], 'center'));
add(['animation-range-start'], rangeOf(false));
add(['animation-range-end'], rangeOf(true));
add(['offset-distance'], (c) => single(c, (x) => lengthPercentage(x, true)));
add(['offset-anchor'], (c) => (c.length === 1 && isIdent(c[0], ['auto']) ? 'auto' : origin(c, true, false)));
add(['offset-position'], (c) => (c.length === 1 && isIdent(c[0], ['auto', 'normal']) ? ident(c[0]) : origin(c, true, false)));
add(['object-position', 'perspective-origin'], (c) => origin(c, true, false));
add(['transform-origin'], (c) => origin(c, false, true));
add(['border-image-slice'], imageSlice);
add(['border-image-width'], (c) => quad(c, (x) => number(x, false) ?? lengthPercentage(x, false) ?? keyword(x, ['auto'])));
add(['border-image-outset'], (c) => quad(c, (x) => number(x, false) ?? length(x, false)));
add(['justify-items'], (c) => { const j = justifyItems(c); return j && j[0] === c.length ? j[1] : null; });
// `none`, or a trigger name and one or two actions per item.
add(['animation-trigger'], (c) => (c.length === 1 && isIdent(c[0], ['none']) ? 'none' : listed(c, (item) => {
  const ACTIONS = ['none', 'play', 'play-once', 'play-forwards', 'play-backwards', 'pause', 'reset', 'replay'];
  if (item.length < 2 || item.length > 3 || dashedIdent(item[0]) === null) return null;
  const actions = item.slice(1).map((x) => keyword(x, ACTIONS));
  return actions.includes(null) ? null : [dashedIdent(item[0]), ...actions].join(' ');
})));
add(['white-space-collapse'], (c) => keywords(c, ['collapse', 'preserve', 'preserve-breaks', 'break-spaces']));
add(['text-wrap-mode'], (c) => keywords(c, ['wrap', 'nowrap']));
add(['text-wrap-style'], (c) => keywords(c, ['auto', 'balance', 'stable', 'pretty', 'avoid-orphans']));
add(['overflow-wrap'], (c) => keywords(c, ['normal', 'break-word', 'anywhere']));
add(['grid-template-areas'], gridAreas);
add(['offset-path'], offsetPath);
add(['clip-path'], clipPath, false);
add(['shape-outside'], shapeOutside, false);
add(['opacity'], opacityV);
add(['display'], display, false);
add(['animation-name'], animationNames);
add(['box-shadow'], (c) => shadow(c, true));
add(['text-shadow'], (c) => shadow(c, false));
add(['offset-rotate'], (c) => { const r = rotatePart(c); return r && r[0] === c.length ? r[1] : null; });

// Properties whose identifiers are names (kept as written) rather than keywords.
const NAMED = ['animation-name', 'font-family', 'container-name', 'counter-reset', 'counter-increment', 'counter-set', 'grid-area', 'grid-row-start', 'grid-row-end', 'grid-column-start', 'grid-column-end', 'grid-template-areas', 'view-transition-name', 'view-transition-class', 'anchor-name', 'position-anchor', 'anchor-scope', 'timeline-scope', 'font-palette', 'will-change', 'page', 'quotes', 'content'];
const NAME_KEYWORDS = ['none', 'auto', 'span', 'normal', 'all', 'match-element'];
const ZERO_LENGTH_PARTS = ['margin', 'padding', 'inset', 'width', 'height', 'size', 'gap', 'radius', 'spacing', 'indent', 'offset', 'flex-basis', 'font-size', 'column-width', 'column-height', 'perspective', 'shape-margin', 'position-x', 'position-y', 'thickness'];
const zeroIsLength = (name) => SIDES4.includes(name) || ZERO_LENGTH_PARTS.some((part) => name.includes(part));
const NAMING_FUNCTIONS = ['stylistic', 'styleset', 'character-variant', 'swash', 'ornaments', 'annotation', 'local', 'format', 'attr', 'counter', 'counters', 'symbols', 'url', 'src', '['];
const LENGTH_FUNCTIONS = ['translate', 'translatex', 'translatey', 'translatez', 'translate3d', 'perspective'];
/** A function's spelling: keyword arguments lowercase (recursively), a length function's `0` as `0px`. */
function functionText(fn, comp) {
  if (NAMING_FUNCTIONS.includes(fn)) return comp.text;
  const open = comp.text.search(/[([]/);
  if (open < 0) return comp.text;
  const inner = components(comp.text.slice(open + 1, -1));
  if (!inner) return comp.text;
  const lengths = LENGTH_FUNCTIONS.includes(fn);
  const spelled = inner.map((part) => ({ ...part, text: part.kind === 'ident' && !part.value.startsWith('--') ? part.value.toLowerCase() : part.kind === 'number' && lengths && part.value === 0 ? '0px' : part.kind === 'function' ? functionText(part.name, part) : part.text }));
  return `${comp.text.slice(0, open + 1)}${join(spelled)}${comp.text.slice(-1)}`;
}
/** The context-free spelling: identifiers lowercase unless they name something, a lone `0` as `0px` where a length is meant. */
function genericSpelling(name, comps) {
  const isNamed = NAMED.includes(name) || name.startsWith('--');
  return join(comps.map((comp) => {
    let text = comp.text;
    if (comp.kind === 'ident' && !comp.value.startsWith('--')) { const lower = comp.value.toLowerCase(); text = isNamed && !NAME_KEYWORDS.includes(lower) ? comp.value : lower; }
    else if (comp.kind === 'hash') text = color(comp) ?? comp.text;
    else if (comp.kind === 'function' && !isNamed) text = functionText(comp.name, comp);
    else if (comp.kind === 'number' && comp.value === 0 && comps.length === 1 && zeroIsLength(name)) text = '0px';
    return { ...comp, text };
  }));
}
/**
 * The specified value of longhand `name` set to `value`, as Chrome 156 spells it:
 * { valid: true, text } / { valid: false } from the longhand's grammar, or
 * { valid: null, text } (the context-free spelling) for a longhand without one.
 */
export function longhandValue(name, value) {
  const trimmed = String(value).trim();
  if (!trimmed) return { valid: false };
  const lower = trimmed.toLowerCase();
  if (CSS_WIDE.has(lower)) return { valid: true, text: lower };
  if (hasSubstitution(trimmed)) return { valid: null, text: substitutionText(trimmed) };
  const comps = components(trimmed);
  if (!comps || !comps.length) return { valid: false };
  const grammar = LONGHAND_GRAMMARS.get(name);
  if (!grammar) return { valid: null, text: genericSpelling(name, comps) };
  const text = grammar[0](comps);
  if (text !== null) return { valid: true, text };
  // A lenient grammar's rejection of a shape() is final: no embedder grammar knows shape() (#338).
  return grammar[1] || comps.some((c) => functionNamed(c, ['shape']) !== null) ? { valid: false } : { valid: null, text: genericSpelling(name, comps) };
}

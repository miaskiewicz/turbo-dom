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
function tokenize(text) {
  const out = [];
  let i = 0;
  const n = text.length;
  const startsNumber = (at) => {
    const c = text[at], d = text[at + 1], e = text[at + 2];
    if (c === '+' || c === '-') return /[0-9]/.test(d || '') || (d === '.' && /[0-9]/.test(e || ''));
    if (c === '.') return /[0-9]/.test(d || '');
    return /[0-9]/.test(c || '');
  };
  const startsIdent = (at) => {
    const c = text[at], d = text[at + 1];
    if (c === '-') return isNameStart(d || '') || d === '-' || (d === '\\');
    if (c === '\\') return true;
    return isNameStart(c || '');
  };
  const readName = () => {
    let name = '';
    while (i < n) {
      const c = text[i];
      if (isName(c)) { name += c; i++; }
      else if (c === '\\' && i + 1 < n && text[i + 1] !== '\n') {
        i++;
        const hex = /^[0-9a-fA-F]{1,6}/.exec(text.slice(i));
        if (hex) { name += String.fromCodePoint(parseInt(hex[0], 16) || 0xfffd); i += hex[0].length; if (/\s/.test(text[i] || '')) i++; }
        else { name += text[i]; i++; }
      } else break;
    }
    return name;
  };
  while (i < n) {
    const c = text[i];
    if (c === '/' && text[i + 1] === '*') { const end = text.indexOf('*/', i + 2); i = end < 0 ? n : end + 2; continue; }
    if (/\s/.test(c)) { while (i < n && /\s/.test(text[i])) i++; out.push({ type: 'ws' }); continue; }
    if (c === '"' || c === "'") {
      let value = ''; i++;
      let closed = false;
      while (i < n) {
        const d = text[i];
        if (d === c) { i++; closed = true; break; }
        if (d === '\n') return null;
        if (d === '\\') {
          if (i + 1 >= n) { i++; continue; }
          const e = text[i + 1];
          if (e === '\n') { i += 2; continue; }
          i++;
          const hex = /^[0-9a-fA-F]{1,6}/.exec(text.slice(i));
          if (hex) { value += String.fromCodePoint(parseInt(hex[0], 16) || 0xfffd); i += hex[0].length; if (/\s/.test(text[i] || '')) i++; }
          else { value += e; i++; }
          continue;
        }
        value += d; i++;
      }
      void closed;
      out.push({ type: 'string', value });
      continue;
    }
    if (startsNumber(i)) {
      const m = /^[+-]?(\d*\.\d+|\d+)([eE][+-]?\d+)?/.exec(text.slice(i));
      const raw = m[0];
      i += raw.length;
      const value = Number(raw);
      const integer = !/[.eE]/.test(raw);
      if (text[i] === '%') { i++; out.push({ type: 'percentage', value, raw }); continue; }
      if (startsIdent(i)) { out.push({ type: 'dimension', value, unit: readName() }); continue; }
      out.push({ type: 'number', value, integer });
      continue;
    }
    if (startsIdent(i)) {
      const name = readName();
      if (text[i] === '(') {
        i++;
        if (name.toLowerCase() === 'url') {
          let j = i;
          while (j < n && /\s/.test(text[j])) j++;
          if (text[j] !== '"' && text[j] !== "'") {
            let value = '';
            i = j;
            while (i < n && text[i] !== ')') {
              const d = text[i];
              if (/\s/.test(d)) { while (i < n && /\s/.test(text[i])) i++; if (text[i] !== ')') return null; break; }
              if (d === '"' || d === "'" || d === '(') return null;
              if (d === '\\') { value += text[i + 1] || ''; i += 2; continue; }
              value += d; i++;
            }
            if (text[i] !== ')') return null;
            i++;
            out.push({ type: 'url', value });
            continue;
          }
        }
        out.push({ type: 'function', name });
        continue;
      }
      out.push({ type: 'ident', value: name });
      continue;
    }
    if (c === '#' && i + 1 < n && (isName(text[i + 1]) || text[i + 1] === '\\')) { i++; out.push({ type: 'hash', value: readName() }); continue; }
    i++;
    if (c === ',') out.push({ type: 'comma' });
    else if (c === '(' || c === '[' || c === '{') out.push({ type: 'open', value: c });
    else if (c === ')' || c === ']' || c === '}') out.push({ type: 'close', value: c });
    else if (c === ';') out.push({ type: 'semicolon' });
    else out.push({ type: 'delim', value: c });
  }
  return out;
}

// ------------------------------------------------------------- components ----
const LENGTH_UNITS = new Set(['px', 'em', 'rem', 'ex', 'rex', 'ch', 'rch', 'ic', 'ric', 'cap', 'rcap', 'lh', 'rlh', 'vw', 'vh', 'vi', 'vb', 'vmin', 'vmax', 'svw', 'svh', 'svi', 'svb', 'svmin', 'svmax', 'lvw', 'lvh', 'lvi', 'lvb', 'lvmin', 'lvmax', 'dvw', 'dvh', 'dvi', 'dvb', 'dvmin', 'dvmax', 'cqw', 'cqh', 'cqi', 'cqb', 'cqmin', 'cqmax', 'cm', 'mm', 'q', 'in', 'pt', 'pc']);
const MATH_FUNCTIONS = new Set(['calc', 'min', 'max', 'clamp', 'round', 'mod', 'rem', 'abs', 'sign', 'sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'atan2', 'pow', 'sqrt', 'hypot', 'log', 'exp', 'calc-size']);
const IMAGE_FUNCTIONS = new Set(['linear-gradient', 'radial-gradient', 'conic-gradient', 'repeating-linear-gradient', 'repeating-radial-gradient', 'repeating-conic-gradient', '-webkit-linear-gradient', '-webkit-radial-gradient', '-webkit-repeating-linear-gradient', '-webkit-repeating-radial-gradient', '-webkit-gradient', 'image-set', '-webkit-image-set', 'cross-fade', '-webkit-cross-fade', 'paint', 'image']);
const COLOR_FUNCTIONS = new Set(['rgb', 'rgba', 'hsl', 'hsla', 'hwb', 'lab', 'lch', 'oklab', 'oklch', 'color', 'color-mix', 'light-dark', 'contrast-color', '-webkit-device-cmyk']);
const SYSTEM_COLORS = new Set(['currentcolor', 'transparent', 'accentcolor', 'accentcolortext', 'activetext', 'buttonborder', 'buttonface', 'buttontext', 'canvas', 'canvastext', 'field', 'fieldtext', 'graytext', 'highlight', 'highlighttext', 'linktext', 'mark', 'marktext', 'selecteditem', 'selecteditemtext', 'visitedtext', 'activeborder', 'activecaption', 'appworkspace', 'background', 'buttonhighlight', 'buttonshadow', 'captiontext', 'inactiveborder', 'inactivecaption', 'inactivecaptiontext', 'infobackground', 'infotext', 'menu', 'menutext', 'scrollbar', 'threeddarkshadow', 'threedface', 'threedhighlight', 'threedlightshadow', 'threedshadow', 'window', 'windowframe', 'windowtext', '-webkit-link', '-webkit-focus-ring-color', '-internal-quirk-inherit']);
const CSS_WIDE = new Set(['initial', 'inherit', 'unset', 'revert', 'revert-layer']);

const quoted = (text) => {
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

const operand = (comp) => (comp.kind === 'number' ? ['', comp.value] : comp.kind === 'percentage' ? ['%', comp.value] : comp.kind === 'dimension' ? [comp.unit, comp.value] : null);
function calcSum(inner) {
  // Operands alternate with operators; products bind first.
  if (inner.length % 2 === 0) return null;
  const terms = [];
  const push = ([unit, value], sign) => { const existing = terms.find((t) => t[0] === unit); if (existing) existing[1] += sign * value; else terms.push([unit, sign * value]); };
  let sign = 1;
  let current = operand(inner[0]);
  if (!current) return null;
  for (let index = 1; index < inner.length; index += 2) {
    const next = operand(inner[index + 1]);
    if (!next) return null;
    const op = inner[index];
    if (op.kind === 'delim' && op.value === '*') {
      if (next[0] === '') current = [current[0], current[1] * next[1]];
      else if (current[0] === '') current = [next[0], current[1] * next[1]];
      else return null;
    } else if ((op.kind === 'slash' || (op.kind === 'delim' && op.value === '/')) && next[0] === '') {
      current = [current[0], current[1] / next[1]];
    } else if (op.kind === 'delim' && (op.value === '+' || op.value === '-')) {
      push(current, sign);
      sign = op.value === '+' ? 1 : -1;
      current = next;
    } else return null;
  }
  push(current, sign);
  return calcText(terms);
}
/**
 * Structural checks Chrome makes before keeping a function: a `calc()` sum alternates operands
 * and operators (a literal product has at most one unit), `rgb()` / `hsl()` use commas
 * everywhere or nowhere, and `linear()` has at least two stops.
 */
function functionValid(name, inner) {
  if (name === 'calc') {
    if (!inner.length || inner.length % 2 === 0) return false;
    const unit = (c) => c.kind === 'dimension' || c.kind === 'percentage';
    return inner.every((comp, index) => {
      if (index % 2 === 1) {
        const product = comp.kind === 'delim' && comp.value === '*';
        const operator = product || (comp.kind === 'delim' && ['+', '-', '/'].includes(comp.value)) || comp.kind === 'slash';
        return operator && !(product && unit(inner[index - 1]) && unit(inner[index + 1]));
      }
      return comp.kind !== 'comma' && comp.kind !== 'slash' && comp.kind !== 'delim';
    });
  }
  if (['rgb', 'rgba', 'hsl', 'hsla'].includes(name)) {
    return !inner.some((c) => c.kind === 'comma') || inner.every((c, index) => (c.kind === 'comma') === (index % 2 === 1));
  }
  if (name === 'linear') return inner.some((c) => c.kind === 'comma');
  return true;
}
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
/** `min()` / `max()` / `clamp()` of literals in one unit: their `calc()` value. */
function minMax(name, inner) {
  let unit = null;
  const values = [];
  for (let index = 0; index < inner.length; index++) {
    const comp = inner[index];
    if (index % 2 === 1) { if (comp.kind !== 'comma') return null; continue; }
    const o = operand(comp);
    if (!o) return null;
    if (unit === null) unit = o[0]; else if (unit !== o[0]) return null;
    values.push(o[1]);
  }
  let value;
  if (name === 'min' && values.length) value = Math.min(...values);
  else if (name === 'max' && values.length) value = Math.max(...values);
  else if (name === 'clamp' && values.length === 3) value = Math.max(values[0], Math.min(values[1], values[2]));
  else return null;
  return calcText([[unit, value]]);
}
/** Blink's alpha spelling: the shortest of two or three decimals naming the same 8-bit alpha. */
function alphaText(alpha) {
  const a = Math.min(1, Math.max(0, alpha));
  const byte = Math.round(a * 255);
  const two = Math.round(a * 100) / 100;
  return cssNumber(Math.round(two * 255) === byte ? two : Math.round(a * 1000) / 1000);
}
function rgbText(channels, alpha) {
  const [r, g, b] = channels.map((c) => Math.min(255, Math.max(0, Math.round(c))));
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
  const parts = [];
  let alphaPart = null, slash = false;
  const commasUsed = inner.some((c) => c.kind === 'comma');
  for (let index = 0; index < inner.length; index++) {
    const comp = inner[index];
    if (comp.kind === 'comma' && commasUsed && index % 2 === 1) continue;
    if (comp.kind === 'slash' && !commasUsed && !slash) { slash = true; continue; }
    if (commasUsed && index % 2 === 1) return null;
    if (slash) { if (alphaPart !== null) return null; alphaPart = comp; continue; }
    parts.push(comp);
  }
  if (commasUsed && parts.length === 4) alphaPart = parts.pop();
  if (parts.length !== 3 || (slash && alphaPart === null)) return null;
  const num = (c) => (c.kind === 'number' ? c.value : null);
  const pct = (c) => (c.kind === 'percentage' ? c.value : null);
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
    if (h.kind === 'number') hue = h.value; else if (h.kind === 'dimension' && h.unit in turns) hue = h.value * turns[h.unit]; else return null;
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
/** `steps(n, end | jump-end)` is `steps(n)`. */
function stepsText(inner) {
  return inner.length === 3 && inner[1].kind === 'comma' && isIdent(inner[2], ['end', 'jump-end']) ? `steps(${inner[0].text})` : null;
}
/** `linear()` with every stop's input position filled in. */
function linearText(inner) {
  const stops = [];
  for (const item of split(inner, isComma)) {
    if (!item.length || item[0].kind !== 'number' || item.length > 3) return null;
    const positions = item.slice(1).map((c) => (c.kind === 'percentage' ? c.value : undefined));
    if (positions.some((p) => p === undefined)) return null;
    if (!positions.length) stops.push([item[0].text, [null]]);
    // A stop with two input positions is two stops.
    else for (const position of positions) stops.push([item[0].text, [position]]);
  }
  if (stops.length < 2) return null;
  const flat = stops.flatMap(([, positions]) => positions);
  if (flat[0] === null) flat[0] = 0;
  if (flat[flat.length - 1] === null) flat[flat.length - 1] = 100;
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

/** Tokens -> components (a function / block is one component); `null` on bad input. */
function read(tokens, state, close) {
  const out = [];
  while (state.at < tokens.length) {
    const token = tokens[state.at++];
    switch (token.type) {
      case 'ws': break;
      case 'ident': out.push({ kind: 'ident', value: token.value, text: token.value }); break;
      case 'number': out.push({ kind: 'number', value: f32(token.value), integer: token.integer, text: cssNumber(token.value) }); break;
      case 'percentage': {
        const value = f32(Number(token.raw));
        out.push({ kind: 'percentage', value, text: `${cssNumber(Number(token.raw))}%` });
        break;
      }
      case 'dimension': out.push({ kind: 'dimension', value: f32(token.value), unit: token.unit.toLowerCase(), text: `${cssNumber(token.value)}${token.unit.toLowerCase()}` }); break;
      case 'hash': out.push({ kind: 'hash', value: token.value, text: `#${token.value}` }); break;
      case 'string': out.push({ kind: 'string', value: token.value, text: quoted(token.value) }); break;
      case 'url': out.push({ kind: 'url', value: token.value, text: `url(${quoted(token.value)})` }); break;
      case 'comma': out.push({ kind: 'comma', text: ',' }); break;
      case 'delim':
        if (token.value === '!') return null;
        out.push(token.value === '/' ? { kind: 'slash', text: '/' } : { kind: 'delim', value: token.value, text: token.value });
        break;
      case 'function': {
        const name = token.name.toLowerCase();
        const inner = read(tokens, state, ')');
        if (inner === null) return null;
        if (name === 'url') {
          if (inner.length !== 1 || inner[0].kind !== 'string') return null;
          out.push({ kind: 'url', value: inner[0].value, text: `url(${quoted(inner[0].value)})` });
        } else {
          if (!functionValid(name, inner)) return null;
          const special = name === 'calc' ? calcSum(inner) : name === 'min' || name === 'max' || name === 'clamp' ? minMax(name, inner) : name === 'rgb' || name === 'rgba' || name === 'hsl' || name === 'hsla' ? colorFunction(name, inner) : name === 'lab' || name === 'lch' || name === 'oklab' || name === 'oklch' ? labFunction(name, inner) : name === 'steps' ? stepsText(inner) : name === 'linear' ? linearText(inner) : null;
          const text = special ?? `${name}(${join(inner)})`;
          out.push({ kind: 'function', name, text });
        }
        break;
      }
      case 'open': {
        if (token.value === '{') return null;
        const closer = token.value === '(' ? ')' : ']';
        const inner = read(tokens, state, closer);
        if (inner === null) return null;
        out.push({ kind: 'function', name: token.value, text: `${token.value}${join(inner)}${closer}` });
        break;
      }
      case 'close':
        if (token.value === close) return out;
        return null;
      default: return null;
    }
  }
  return close ? out : out;
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
const integer = (c, minimum) => (c.kind === 'number' && c.integer && c.value >= minimum ? c.text : null);
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
  const isBaseline = a[1].endsWith('baseline');
  let j;
  if (a[0] === comps.length) j = isBaseline ? 'start' : a[1];
  else {
    if (isBaseline) return null;
    const rest = justify(comps.slice(a[0]));
    if (!rest || a[0] + rest[0] !== comps.length) return null;
    j = rest[1];
  }
  return [['align-content', a[1]], ['justify-content', j]];
}
const selfAlignment = (comps, extra) => baseline(comps) ?? firstKeyword(comps, extra) ?? overflowPosition(comps, SELF_POSITIONS);
function placeItems(comps) {
  const a = selfAlignment(comps, ['normal', 'stretch']);
  if (!a) return null;
  let justify = a[1];
  if (a[0] !== comps.length) {
    const rest = comps.slice(a[0]);
    let legacy = null;
    if (rest.length === 1 && isIdent(rest[0], ['legacy'])) legacy = [1, 'legacy'];
    else if (rest.length === 2 && isIdent(rest[0], ['legacy'])) { const v = keyword(rest[1], ['left', 'right', 'center']); legacy = v === null ? null : [2, `legacy ${v}`]; }
    else if (rest.length === 2 && isIdent(rest[1], ['legacy'])) { const v = keyword(rest[0], ['left', 'right', 'center']); legacy = v === null ? null : [2, `legacy ${v}`]; }
    const result = legacy ?? selfAlignment(rest, ['normal', 'stretch', 'left', 'right', 'anchor-center']) ?? overflowPosition(rest, ['left', 'right']);
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

/** One `<bg-position>`: its x and y specified values. */
function position(comps) {
  const lp = (c) => lengthPercentage(c, true);
  const horizontal = (c) => keyword(c, ['left', 'right']);
  const vertical = (c) => keyword(c, ['top', 'bottom']);
  const center = (c) => keyword(c, ['center']);
  if (comps.length === 1) {
    const [a] = comps;
    const v = vertical(a);
    if (v !== null) return ['center', v];
    const x = horizontal(a) ?? center(a) ?? lp(a);
    return x === null ? null : [x, 'center'];
  }
  if (comps.length === 2) {
    const [a, b] = comps;
    const ax = horizontal(a) ?? center(a) ?? lp(a), by = vertical(b) ?? center(b) ?? lp(b);
    if (ax !== null && by !== null) return [ax, by];
    const ay = vertical(a) ?? center(a), bx = horizontal(b) ?? center(b);
    return ay !== null && bx !== null ? [bx, ay] : null;
  }
  if (comps.length === 3 || comps.length === 4) {
    let at = 0, x = null, y = null;
    while (at < comps.length) {
      const kw = keyword(comps[at], ['left', 'right', 'top', 'bottom', 'center']);
      if (kw === null) return null;
      const offset = comps[at + 1] ? lp(comps[at + 1]) : null;
      const used = offset !== null && kw !== 'center' ? 2 : 1;
      const text = used === 2 ? `${kw} ${offset}` : kw;
      const isX = kw === 'left' || kw === 'right' ? true : kw === 'top' || kw === 'bottom' ? false : x === null && (y !== null || at === 0);
      if (isX) { if (x !== null) return null; x = text; } else { if (y !== null) return null; y = text; }
      at += used;
    }
    return x !== null && y !== null ? [x, y] : null;
  }
  return null;
}
function positions(comps) {
  const layers = commas(comps);
  if (!layers) return null;
  const xs = [], ys = [];
  for (const layer of layers) { const p = position(layer); if (!p) return null; xs.push(p[0]); ys.push(p[1]); }
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

function columns(comps) {
  const parts = slashes(comps);
  let main, height = null;
  if (parts.length === 1) main = parts[0];
  else if (parts.length === 2 && parts[1].length === 1) {
    main = parts[0];
    height = length(parts[1][0], false) ?? keyword(parts[1][0], ['auto']);
    if (height === null) return null;
  } else return null;
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
  return [['column-width', width ?? 'auto'], ['column-count', count ?? 'auto'], ['column-height', height ?? 'auto'], ['column-wrap', 'auto']];
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
  const parsed = anyOrder(comps, [one((c) => keyword(c, ['collapse', 'preserve', 'preserve-breaks', 'preserve-spaces', 'break-spaces'])), one((c) => keyword(c, ['wrap', 'nowrap']))]);
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
function fontVariant(comps) {
  const order = ['font-variant-ligatures', 'font-variant-caps', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-alternates', 'font-variant-position', 'font-variant-emoji'];
  if (comps.length === 1 && isIdent(comps[0], ['normal'])) return same(order, 'normal');
  if (comps.length === 1 && isIdent(comps[0], ['none'])) { const out = same(order, 'normal'); out[0][1] = 'none'; return out; }
  const alternates = (comps) => {
    if (!comps.length) return null;
    const value = keyword(comps[0], ['historical-forms']) ?? functionNamed(comps[0], ['stylistic', 'styleset', 'character-variant', 'swash', 'ornaments', 'annotation']);
    return value === null ? null : [1, value];
  };
  const parsed = anyOrder(comps, [(c) => groupSet(c, LIGATURES), (c) => groupSet(c, NUMERIC), (c) => groupSet(c, EAST_ASIAN), one((c) => keyword(c, CAPS)), alternates, one((c) => keyword(c, ['sub', 'super'])), one((c) => keyword(c, ['text', 'emoji', 'unicode']))]);
  if (!parsed) return null;
  const names = ['font-variant-ligatures', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-caps', 'font-variant-alternates', 'font-variant-position', 'font-variant-emoji'];
  return names.map((name, index) => [name, parsed[index] ?? 'normal']);
}

function flex(comps) {
  const names = ['flex-grow', 'flex-shrink', 'flex-basis'];
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return named(names, ['0', '0', 'auto']);
  if (comps.length === 1 && isIdent(comps[0], ['auto'])) return named(names, ['1', '1', 'auto']);
  const basis = (c) => keyword(c, ['auto', 'content', 'min-content', 'max-content', 'fit-content']) ?? lengthPercentage(c, false);
  let grow = null, shrink = null, basisValue = null, at = 0;
  while (at < comps.length) {
    const comp = comps[at];
    if (grow === null) {
      const n = number(comp, false);
      if (n !== null) {
        grow = n;
        const next = comps[at + 1] ? number(comps[at + 1], false) : null;
        if (next !== null) { shrink = next; at++; }
        at++;
        continue;
      }
    }
    if (basisValue === null) {
      const b = basis(comp);
      if (b !== null) { basisValue = b; at++; continue; }
    }
    return null;
  }
  if (grow === null && basisValue === null) return null;
  return named(names, [grow ?? '1', shrink ?? '1', basisValue ?? '0%']);
}

const SYSTEM_FONTS = ['caption', 'icon', 'menu', 'message-box', 'small-caption', 'status-bar'];
const FONT_SIZES = ['xx-small', 'x-small', 'small', 'medium', 'large', 'x-large', 'xx-large', 'xxx-large', 'larger', 'smaller', '-webkit-xxx-large'];
const STRETCHES = ['ultra-condensed', 'extra-condensed', 'condensed', 'semi-condensed', 'semi-expanded', 'expanded', 'extra-expanded', 'ultra-expanded'];
const GENERIC_FAMILIES = ['serif', 'sans-serif', 'cursive', 'fantasy', 'monospace', 'system-ui', 'math', 'emoji', 'fangsong', 'ui-serif', 'ui-sans-serif', 'ui-monospace', 'ui-rounded', '-webkit-body', '-webkit-pictograph'];
/** Whether `text` tokenizes as exactly one identifier (no escapes needed). */
const isIdentifier = (text) => /^(-(-|[A-Za-z_\u0080-\uffff])|[A-Za-z_\u0080-\uffff])[A-Za-z0-9_\-\u0080-\uffff]*$/.test(text);
const reserved = (name, extra) => { const lower = name.toLowerCase(); return CSS_WIDE.has(lower) || lower === 'default' || extra.includes(lower); };
/** A quoted family name: unquoted when it is one valid identifier Chrome would not read as a keyword. */
const familyString = (comp) => (isIdentifier(comp.value) && !reserved(comp.value, GENERIC_FAMILIES) ? comp.value : comp.text);
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
    families.push(words.length > 1 ? `"${name}"` : name);
  }
  return families.join(', ');
}
function font(comps) {
  if (comps.length === 1) { const name = keyword(comps[0], SYSTEM_FONTS); if (name !== null) return { pending: name }; }
  let style = null, caps = null, weight = null, stretch = null, at = 0;
  while (at < comps.length && at < 4) {
    const comp = comps[at];
    if (isIdent(comp, ['normal'])) { at++; continue; }
    if (style === null) {
      const value = keyword(comp, ['italic', 'oblique']);
      if (value !== null) {
        if (value === 'oblique' && comps[at + 1] && comps[at + 1].kind === 'dimension' && angle(comps[at + 1]) !== null) {
          // `oblique 0deg` is `normal`, the default 14deg angle plain `oblique`.
          const a = angle(comps[at + 1]);
          style = a === '0deg' ? 'normal' : a === '14deg' ? 'oblique' : `oblique ${a}`;
          at += 2;
          continue;
        }
        style = value; at++; continue;
      }
    }
    if (caps === null) { const value = keyword(comp, ['small-caps']); if (value !== null) { caps = value; at++; continue; } }
    if (weight === null) {
      const value = keyword(comp, ['bold', 'bolder', 'lighter']) ?? (comp.kind === 'number' && comp.value >= 1 && comp.value <= 1000 ? number(comp, false) : null);
      if (value !== null) { weight = value; at++; continue; }
    }
    if (stretch === null) { const value = keyword(comp, STRETCHES); if (value !== null) { stretch = value; at++; continue; } }
    break;
  }
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
  const rest = comps.slice(at);
  if (!rest.length) return null;
  const family = fontFamily(rest);
  if (family === null) return null;
  return [['font-style', style ?? 'normal'], ['font-variant-caps', caps ?? 'normal'], ['font-variant-ligatures', 'normal'], ['font-variant-numeric', 'normal'], ['font-variant-east-asian', 'normal'], ['font-variant-alternates', 'normal'], ['font-size-adjust', 'none'], ['font-language-override', 'normal'], ['font-kerning', 'auto'], ['font-optical-sizing', 'auto'], ['font-feature-settings', 'normal'], ['font-variation-settings', 'normal'], ['font-variant-position', 'normal'], ['font-variant-emoji', 'normal'], ['font-weight', weight ?? 'normal'], ['font-stretch', stretch ?? 'normal'], ['font-size', size], ['line-height', lineHeight], ['font-family', family]];
}

/** `<grid-line>` with Chrome's canonical part order (`span`, integer, name). */
function gridLine(comps) {
  if (comps.length === 1 && isIdent(comps[0], ['auto'])) return 'auto';
  if (!comps.length) return null;
  let span = false, int = null, name = null;
  for (const comp of comps) {
    if (isIdent(comp, ['span']) && !span) { span = true; continue; }
    const n = int === null ? integer(comp, -Infinity) : null;
    if (n !== null) { if (n === '0') return null; int = n; continue; }
    const c = name === null ? customIdent(comp, ['auto', 'span']) : null;
    if (c !== null) { name = c; continue; }
    return null;
  }
  if (int === null && name === null) return null;
  if (span && int !== null && int.startsWith('-')) return null;
  return [span ? 'span' : null, int, name].filter((v) => v !== null).join(' ');
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
function trackList(comps) {
  if (!comps.length) return null;
  if (comps.length === 1 && isIdent(comps[0], ['none'])) return 'none';
  const subgrid = isIdent(comps[0], ['subgrid']);
  for (let index = 0; index < comps.length; index++) {
    const comp = comps[index];
    let ok;
    if (comp.kind === 'function') ok = comp.name === '[' || ['repeat', 'minmax', 'fit-content'].includes(comp.name) || lengthPercentage(comp, false) !== null;
    else if (comp.kind === 'dimension') ok = comp.unit === 'fr' || length(comp, false) !== null;
    else if (comp.kind === 'percentage') ok = comp.value >= 0;
    else if (comp.kind === 'number') ok = comp.value === 0;
    else if (comp.kind === 'ident') ok = (index === 0 && subgrid) || isIdent(comp, ['auto', 'min-content', 'max-content', 'masonry']);
    else ok = false;
    if (!ok) return null;
  }
  return join(comps.map((c) => (c.kind === 'number' ? { ...c, text: '0px' } : c)));
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
  let rows, cols = null;
  if (parts.length === 1) rows = parts[0];
  // The explicit columns of the areas form take no `repeat()`.
  else if (parts.length === 2 && !parts[1].some((c) => functionNamed(c, ['repeat']) !== null)) { rows = parts[0]; cols = trackList(parts[1]); if (cols === null) return null; }
  else return null;
  const areas = [], tracks = [];
  let at = 0;
  const isNames = (c) => c.kind === 'function' && c.name === '[';
  const names = () => { if (rows[at] && isNames(rows[at])) { tracks.push(rows[at].text); at++; } };
  while (at < rows.length) {
    names();
    const area = rows[at] ? string(rows[at]) : null;
    if (area === null) return null;
    areas.push(area);
    at++;
    const c = rows[at];
    let size = null;
    if (c && string(c) === null && !isNames(c)) size = lengthPercentage(c, false) ?? keyword(c, ['auto', 'min-content', 'max-content']) ?? functionNamed(c, ['minmax', 'fit-content']) ?? (c.kind === 'dimension' && c.unit === 'fr' ? c.text : null);
    if (size !== null) { tracks.push(size); at++; } else tracks.push('auto');
    names();
  }
  const merged = [];
  for (const track of tracks) {
    const last = merged[merged.length - 1];
    if (last !== undefined && track.startsWith('[') && last.startsWith('[')) { merged[merged.length - 1] = `${last.slice(0, -1)} ${track.slice(1)}`; continue; }
    merged.push(track);
  }
  return [merged.join(' '), cols ?? 'none', areas.join(' ')];
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
function borderImage(comps, webkit) {
  let source = null, slice = null, width = null, outset = null, repeat = null, at = 0;
  while (at < comps.length) {
    const comp = comps[at];
    if (source === null) { const value = image(comp) ?? keyword(comp, ['none']); if (value !== null) { source = value; at++; continue; } }
    if (repeat === null) {
      const kw = (c) => keyword(c, ['stretch', 'repeat', 'round', 'space']);
      const first = kw(comp);
      if (first !== null) {
        const second = comps[at + 1] ? kw(comps[at + 1]) : null;
        at += second !== null ? 2 : 1;
        repeat = second !== null && second !== first ? `${first} ${second}` : first;
        continue;
      }
    }
    if (slice === null) {
      const values = [];
      let fill = false;
      while (comps[at]) {
        const c = comps[at];
        if (isIdent(c, ['fill']) && !fill) fill = true;
        else if (fill && values.length) break; // `fill` goes before or after the numbers
        else { const value = values.length < 4 ? number(c, false) ?? percentage(c, false) : null; if (value === null) break; values.push(value); }
        at++;
      }
      if (!values.length) return null;
      slice = collapse4(values) + (fill || webkit ? ' fill' : '');
      const quad = (ty) => {
        const list = [];
        while (comps[at] && list.length < 4) { const value = ty(comps[at]); if (value === null) break; list.push(value); at++; }
        return list.length ? collapse4(list) : null;
      };
      if (comps[at] && isSlash(comps[at])) {
        at++;
        const w = quad((c) => number(c, false) ?? lengthPercentage(c, false) ?? keyword(c, ['auto']));
        let o = null;
        if (comps[at] && isSlash(comps[at])) {
          at++;
          o = quad((c) => number(c, false) ?? length(c, false));
          if (o === null) return null;
        } else if (w === null) return null;
        width = w; outset = o;
      }
      continue;
    }
    return null;
  }
  if ([source, slice, width, outset, repeat].every((v) => v === null)) return null;
  return [source, slice, width, outset, repeat];
}
function pathText(text) {
  const start = text.indexOf('"');
  if (start < 0) return text;
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
function offset(comps) {
  const parts = slashes(comps);
  let main, anchorPart = null;
  if (parts.length === 1) main = parts[0];
  else if (parts.length === 2) { main = parts[0]; anchorPart = parts[1]; }
  else return null;
  let anchor;
  if (anchorPart === null) anchor = 'auto';
  else if (anchorPart.length === 1 && isIdent(anchorPart[0], ['auto'])) anchor = 'auto';
  else { const p = position(anchorPart); if (!p) return null; anchor = `${p[0]} ${p[1]}`; }
  let at = 0, positionValue = 'normal';
  if (main.length) {
    if (isIdent(main[0], ['auto', 'normal'])) { positionValue = ident(main[0]); at = 1; }
    else {
      for (let len = 4; len >= 1; len--) {
        if (main.length < len) continue;
        const slice = main.slice(0, len);
        if (slice.every((c) => keyword(c, ['left', 'right', 'top', 'bottom', 'center']) !== null || lengthPercentage(c, true) !== null)) {
          const p = position(slice);
          if (p) { positionValue = `${p[0]} ${p[1]}`; at = len; break; }
        }
      }
    }
  }
  let path = 'none', distance = '0px', rotate = 'auto';
  const comp = main[at];
  if (comp) {
    {
      const value = isIdent(comp, ['none']) ? 'none' : functionNamed(comp, ['path', 'ray', 'url', 'circle', 'ellipse', 'inset', 'polygon', 'xywh', 'rect', 'shape']) ?? url(comp);
      if (value !== null) {
        path = value.startsWith('path(') ? pathText(value) : value;
        at++;
        const rotatePart = (comps) => {
          if (!comps.length) return null;
          const first = comps[0];
          const kw = keyword(first, ['auto', 'reverse']);
          if (kw !== null) { const a = comps[1] ? angle(comps[1]) : null; return a !== null ? [2, `${kw} ${a}`] : [1, kw]; }
          const a = angle(first);
          if (a === null) return null;
          const kw2 = comps[1] ? keyword(comps[1], ['auto', 'reverse']) : null;
          return kw2 !== null ? [2, `${kw2} ${a}`] : [1, a];
        };
        const parsed = anyOrder(main.slice(at), [one((c) => lengthPercentage(c, true)), rotatePart]);
        if (!parsed) return null;
        at = main.length;
        if (parsed[0] !== null) distance = parsed[0];
        if (parsed[1] !== null) rotate = parsed[1];
      }
    }
  }
  if (at !== main.length || (!main.length && parts.length === 1)) return null;
  return [['offset-position', positionValue], ['offset-path', path], ['offset-distance', distance], ['offset-rotate', rotate], ['offset-anchor', anchor]];
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
function layer(comps, mask, last) {
  const out = { image: null, position: null, size: null, repeat: null, attachment: null, origin: null, clip: null, color: null, composite: null, mode: null };
  const boxes = mask ? ['border-box', 'padding-box', 'content-box', 'fill-box', 'stroke-box', 'view-box', 'no-clip'] : ['border-box', 'padding-box', 'content-box', 'text', 'border-area'];
  let at = 0;
  while (at < comps.length) {
    const rest = comps.slice(at), comp = comps[at];
    if (out.image === null) { const value = image(comp) ?? keyword(comp, ['none']); if (value !== null) { out.image = value; at++; continue; } }
    if (out.position === null) {
      const p = layerPosition(rest);
      if (p) {
        out.position = p[1]; at += p[0];
        if (comps[at] && isSlash(comps[at])) { const s = bgSize(comps.slice(at + 1)); if (!s) return null; out.size = s[1]; at += 1 + s[0]; }
        continue;
      }
    }
    if (out.repeat === null) { const r = repeatStyle(rest); if (r) { out.repeat = r[1]; at += r[0]; continue; } }
    if (!mask && out.attachment === null) { const value = keyword(comp, ['scroll', 'fixed', 'local']); if (value !== null) { out.attachment = value; at++; continue; } }
    if (out.clip === null || out.origin === null) {
      const value = keyword(comp, boxes);
      if (value !== null) {
        const clipOnly = value === 'text' || value === 'no-clip';
        if (out.origin === null && !clipOnly) out.origin = value;
        else if (out.clip === null) out.clip = value;
        else return null;
        at++;
        continue;
      }
    }
    if (mask) {
      if (out.composite === null) { const value = keyword(comp, ['add', 'subtract', 'intersect', 'exclude']); if (value !== null) { out.composite = value; at++; continue; } }
      if (out.mode === null) { const value = keyword(comp, ['alpha', 'luminance', 'match-source']); if (value !== null) { out.mode = value; at++; continue; } }
    } else if (last && out.color === null) { const value = color(comp); if (value !== null) { out.color = value; at++; continue; } }
    return null;
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
const ANIMATION_KEYWORDS = ['none', 'auto', 'infinite', 'normal', 'reverse', 'alternate', 'alternate-reverse', 'forwards', 'backwards', 'both', 'running', 'paused', 'linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out', 'step-start', 'step-end'];
function animation(comps) {
  const items = commas(comps);
  if (!items) return null;
  const columns = [[], [], [], [], [], [], [], []];
  const initials = ['auto', 'ease', '0s', '1', 'normal', 'none', 'running', 'none'];
  for (const item of items) {
    // A quoted name serializes as an identifier when it is one Chrome would not read as a keyword.
    const name = (c) => (isIdent(c, ['none']) ? 'none' : c.kind === 'string' ? (isIdentifier(c.value) && !reserved(c.value, ANIMATION_KEYWORDS) ? c.value : c.text) : customIdent(c, ['none']));
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
  if (comps.length < 2) return null;
  const name = dashedIdent(comps[0]);
  if (name === null) return null;
  const source = functionNamed(comps[1], ['view', 'scroll']) ?? keyword(comps[1], ['auto', 'none']);
  if (source === null) return null;
  const more = defaults().slice(1);
  const rest = comps.slice(2);
  if (rest.length) {
    const parts = slashes(rest);
    const activation = rangePair(parts[0], 'timeline-trigger-activation-range-start', 'timeline-trigger-activation-range-end', 'normal');
    if (!activation) return null;
    more[0][1] = activation[0][1]; more[1][1] = activation[1][1];
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
  'background-position': (c) => { const p = positions(c); return p && [['background-position-x', p[0]], ['background-position-y', p[1]]]; },
  'mask-position': (c) => { const p = positions(c); return p && [['-webkit-mask-position-x', p[0]], ['-webkit-mask-position-y', p[1]]]; },
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
  return end === fallback ? start : `${start} ${end}`;
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
  const count = columns[0].length;
  if (columns.some((c) => c.length !== count)) return '';
  const colorValue = v('background-color');
  const out = [];
  for (let index = 0; index < count; index++) {
    const item = (column) => (columns[column][index] === INITIAL ? '' : columns[column][index]);
    const x = item(1), y = item(2), size = item(3);
    let pos = '';
    if (x || y || size) { pos = `${x || '0%'} ${y || '0%'}`; if (size) pos += ` / ${size}`; }
    const last = index + 1 === count;
    const text = joined([item(0), pos, item(4), item(5), item(6), item(7), last && colorValue !== INITIAL ? colorValue : '']);
    out.push(text || 'none');
  }
  return out.join(', ');
}
function maskText(v) {
  const names = [['mask-image', 'none'], ['-webkit-mask-position-x', '0%'], ['-webkit-mask-position-y', '0%'], ['mask-size', 'auto'], ['mask-repeat', 'repeat'], ['mask-origin', 'border-box'], ['mask-clip', 'border-box'], ['mask-composite', 'add'], ['mask-mode', 'match-source']];
  const columns = names.map(([name]) => layers(v(name)));
  const count = columns[0].length;
  if (columns.some((c) => c.length !== count)) return '';
  const out = [];
  for (let index = 0; index < count; index++) {
    const item = (column) => { const value = columns[column][index]; return value === names[column][1] || value === INITIAL ? '' : value; };
    const size = item(3), x = columns[1][index], y = columns[2][index];
    const pos = !item(1) && !item(2) && !size ? '' : size ? `${x} ${y} / ${size}` : `${x} ${y}`;
    const origin = columns[5][index], clip = columns[6][index];
    // `no-clip` is a clip-only keyword: it reads back without the default origin.
    const boxes = origin === 'border-box' && clip === 'border-box' ? '' : origin === clip ? origin : origin === 'border-box' && clip === 'no-clip' ? clip : `${origin} ${clip}`;
    out.push(joined([item(0), pos, item(4), boxes, item(7), item(8)]) || 'none');
  }
  return out.join(', ');
}
function transitionText(v) {
  const properties = layers(v('transition-property')), durations = layers(v('transition-duration')), timings = layers(v('transition-timing-function')), delays = layers(v('transition-delay')), behaviors = layers(v('transition-behavior'));
  const zero = (t) => t === '0s' || t === '0ms';
  return properties.map((property, index) => {
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
  const count = columns[7].length;
  if (columns.some((c) => c.length !== count)) return '';
  const out = [];
  for (let index = 0; index < count; index++) out.push(columns.map((c) => c[index]).join(' '));
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

/** Serialize `grammar` from its longhands' values (`texts` in table order). */
function serializeValue(grammar, entry, texts) {
  const v = (name) => { const index = entry.longhands.indexOf(name); return index < 0 ? '' : texts[index]; };
  if (BOXES[grammar]) return boxText(BOXES[grammar].map(v));
  if (PAIRS.has(grammar)) return pairText(texts[0], texts[1]);
  if (SAME.has(grammar)) return allSame(texts);
  switch (grammar) {
    case 'border-radius':
    case '-webkit-border-radius': {
      const corners = ['border-top-left-radius', 'border-top-right-radius', 'border-bottom-right-radius', 'border-bottom-left-radius'].map(v);
      const horizontal = [], vertical = [];
      for (const corner of corners) { const space = corner.indexOf(' '); const h = space < 0 ? corner : corner.slice(0, space); horizontal.push(h); vertical.push(space < 0 ? h : corner.slice(space + 1)); }
      const h = boxText(horizontal);
      return horizontal.every((value, index) => value === vertical[index]) ? h : `${h} / ${boxText(vertical)}`;
    }
    case 'column-rule-inset': return ruleInsetText(v, 'column');
    case 'row-rule-inset': return ruleInsetText(v, 'row');
    case 'rule-inset': { const column = ruleInsetText(v, 'column'); return column === ruleInsetText(v, 'row') ? column : ''; }
    case 'rule-inset-cap':
    case 'rule-inset-junction': {
      const part = grammar === 'rule-inset-cap' ? 'cap' : 'junction';
      const row = pairText(v(`row-rule-inset-${part}-start`), v(`row-rule-inset-${part}-end`));
      return row === pairText(v(`column-rule-inset-${part}-start`), v(`column-rule-inset-${part}-end`)) ? row : '';
    }
    case 'place-items': return pairText(v('align-items'), v('justify-items'));
    case 'place-self': return pairText(v('align-self'), v('justify-self'));
    case 'place-content': return pairText(v('align-content'), v('justify-content'));
    case 'background-position':
    case 'mask-position': {
      const xs = layers(texts[0]), ys = layers(texts[1]);
      if (xs.length !== ys.length || [...xs, ...ys].includes(INITIAL)) return '';
      return xs.map((x, index) => `${x} ${ys[index]}`).join(', ');
    }
    case 'animation-range': return rangeText(v('animation-range-start'), v('animation-range-end'), 'normal');
    case 'timeline-trigger-activation-range': return rangeText(v('timeline-trigger-activation-range-start'), v('timeline-trigger-activation-range-end'), 'normal');
    case 'timeline-trigger-active-range': return rangeText(v('timeline-trigger-active-range-start'), v('timeline-trigger-active-range-end'), 'auto');
    case 'border-top': case 'border-right': case 'border-bottom': case 'border-left': case 'column-rule': case 'row-rule':
      return lineText(v(`${grammar}-width`), v(`${grammar}-style`), v(`${grammar}-color`));
    // The logical sides leave omitted parts `initial` and keep every other value.
    case 'border-block-start': case 'border-block-end': case 'border-inline-start': case 'border-inline-end':
      return joined(['width', 'style', 'color'].map((part) => omitIf(v(`${grammar}-${part}`), null)));
    case 'border-block': case 'border-inline': case 'rule': {
      const [start, end] = grammar === 'rule' ? ['column-rule', 'row-rule'] : [`${grammar}-start`, `${grammar}-end`];
      const a = ['width', 'style', 'color'].map((part) => v(`${start}-${part}`)), b = ['width', 'style', 'color'].map((part) => v(`${end}-${part}`));
      return a.every((value, index) => value === b[index]) ? lineText(...a) : '';
    }
    case 'border': {
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
    case 'outline': return joined(['outline-color', 'outline-style', 'outline-width'].map((name) => omitIf(v(name), null)));
    case '-webkit-text-stroke': return joined(['-webkit-text-stroke-width', '-webkit-text-stroke-color'].map((name) => omitIf(v(name), null)));
    case 'text-decoration': {
      const lineValue = v('text-decoration-line');
      const text = joined([omitIf(lineValue, 'none'), omitIf(v('text-decoration-thickness'), 'auto'), omitIf(v('text-decoration-style'), 'solid'), omitIf(v('text-decoration-color'), 'currentcolor')]);
      return text || (lineValue === INITIAL ? 'none' : lineValue);
    }
    case 'text-emphasis': return joined([omitIf(v('text-emphasis-style'), 'none'), omitIf(v('text-emphasis-color'), 'currentcolor')]) || 'none';
    case 'flex-flow': {
      const direction = v('flex-direction'), wrap = v('flex-wrap');
      return wrap === 'nowrap' ? direction : direction === 'row' ? wrap : `${direction} ${wrap}`;
    }
    case 'list-style': return joined(['list-style-position', 'list-style-image', 'list-style-type'].map((name) => omitIf(v(name), null)));
    case 'columns': {
      const width = v('column-width'), count = v('column-count'), height = v('column-height');
      if (v('column-wrap') !== 'auto') return '';
      const main = width === 'auto' && count === 'auto' ? 'auto' : width === 'auto' ? count : count === 'auto' ? width : `${width} ${count}`;
      return height === 'auto' ? main : `${main} / ${height}`;
    }
    case 'container': { const kind = v('container-type'); return kind === 'normal' ? v('container-name') : `${v('container-name')} / ${kind}`; }
    case 'text-wrap': {
      const mode = v('text-wrap-mode') === INITIAL ? 'wrap' : v('text-wrap-mode');
      const style = v('text-wrap-style') === INITIAL ? 'auto' : v('text-wrap-style');
      return style === 'auto' ? mode : mode === 'wrap' ? style : `${mode} ${style}`;
    }
    case 'white-space': {
      const collapse = v('white-space-collapse') === INITIAL ? 'collapse' : v('white-space-collapse');
      const mode = v('text-wrap-mode') === INITIAL ? 'wrap' : v('text-wrap-mode');
      return { 'collapse wrap': 'normal', 'preserve nowrap': 'pre', 'collapse nowrap': 'nowrap', 'preserve wrap': 'pre-wrap', 'preserve-breaks wrap': 'pre-line', 'break-spaces wrap': 'break-spaces' }[`${collapse} ${mode}`] ?? `${collapse} ${mode}`;
    }
    case 'font-synthesis': {
      const parts = [];
      const values = ['font-synthesis-weight', 'font-synthesis-style', 'font-synthesis-small-caps'].map(v);
      for (const [index, name] of ['weight', 'style', 'small-caps'].entries()) {
        if (values[index] === 'auto') parts.push(name);
        else if (values[index] !== 'none') return '';
      }
      return parts.length ? parts.join(' ') : 'none';
    }
    case 'text-box': {
      const trim = v('text-box-trim'), edge = v('text-box-edge');
      if (trim === 'none' && edge === 'auto') return 'normal';
      if (trim === 'trim-both') return edge === 'auto' ? 'trim-both' : edge;
      return edge === 'auto' ? trim : `${trim} ${edge}`;
    }
    case 'position-try': return v('position-try-order') === 'normal' ? v('position-try-fallbacks') : `${v('position-try-order')} ${v('position-try-fallbacks')}`;
    case 'scroll-timeline': {
      const names = layers(v('scroll-timeline-name')), axes = layers(v('scroll-timeline-axis'));
      if (names.length !== axes.length) return '';
      return names.map((name, index) => (axes[index] === 'block' ? name : `${name} ${axes[index]}`)).join(', ');
    }
    case 'view-timeline': {
      const names = layers(v('view-timeline-name')), axes = layers(v('view-timeline-axis')), insets = layers(v('view-timeline-inset'));
      if (names.length !== axes.length || names.length !== insets.length) return '';
      return names.map((name, index) => joined([name, axes[index] === 'block' ? '' : axes[index], insets[index] === 'auto' ? '' : insets[index]])).join(', ');
    }
    case 'font-variant': {
      const values = ['font-variant-ligatures', 'font-variant-caps', 'font-variant-alternates', 'font-variant-numeric', 'font-variant-east-asian', 'font-variant-position', 'font-variant-emoji'].map(v);
      if (values.every((value) => value === 'normal')) return 'normal';
      if (values[0] === 'none') return values.slice(1).every((value) => value === 'normal') ? 'none' : '';
      return joined(values.map((value) => (value === 'normal' ? '' : value)));
    }
    case 'flex': return `${v('flex-grow')} ${v('flex-shrink')} ${v('flex-basis')}`;
    case 'font': return fontText(v);
    case 'grid-area': {
      const [rs, cs, re, ce] = ['grid-row-start', 'grid-column-start', 'grid-row-end', 'grid-column-end'].map(v);
      const parts = [rs, cs, re, ce];
      if (ce === lineDefault(cs)) { parts.pop(); if (re === lineDefault(rs)) { parts.pop(); if (cs === lineDefault(rs)) parts.pop(); } }
      return parts.join(' / ');
    }
    case 'grid-row':
    case 'grid-column': return texts[1] === lineDefault(texts[0]) ? texts[0] : `${texts[0]} / ${texts[1]}`;
    case 'grid-template': return gridTemplateText(v('grid-template-rows'), v('grid-template-columns'), v('grid-template-areas'));
    case 'grid': {
      const rows = v('grid-template-rows'), cols = v('grid-template-columns'), areas = v('grid-template-areas'), flow = v('grid-auto-flow'), autoCols = v('grid-auto-columns'), autoRows = v('grid-auto-rows');
      if (flow === 'row' && autoRows === 'auto' && autoCols === 'auto') return gridTemplateText(rows, cols, areas);
      if (areas !== 'none') return '';
      const dense = flow.endsWith('dense');
      if ((flow.startsWith('row') || flow === 'dense') && rows === 'none' && autoCols === 'auto') return `${joined(['auto-flow', dense ? 'dense' : '', autoRows === 'auto' ? '' : autoRows])} / ${cols}`;
      if (flow.startsWith('column') && cols === 'none' && autoRows === 'auto') return `${rows} / ${joined(['auto-flow', dense ? 'dense' : '', autoCols === 'auto' ? '' : autoCols])}`;
      return '';
    }
    case 'border-image': {
      const [source, slice, width, outsetValue, repeat] = ['border-image-source', 'border-image-slice', 'border-image-width', 'border-image-outset', 'border-image-repeat'].map(v);
      if (slice === '100%' && width === '1' && outsetValue === '0' && repeat === 'stretch') return source;
      return `${source} ${slice} / ${width} / ${outsetValue} ${repeat}`;
    }
    case 'offset': {
      const [pos, path, distance, rotate, anchor] = ['offset-position', 'offset-path', 'offset-distance', 'offset-rotate', 'offset-anchor'].map(v);
      const moved = distance !== '0px' || rotate !== 'auto';
      const main = joined([pos === 'normal' ? '' : pos, path === 'none' && !moved ? '' : path, distance === '0px' ? '' : distance, rotate === 'auto' ? '' : rotate]) || 'normal';
      return anchor === 'auto' ? main : `${main} / ${anchor}`;
    }
    case 'background': return backgroundText(v);
    case 'mask': return maskText(v);
    case 'transition': return transitionText(v);
    case 'animation': return animationText(v);
    case 'timeline-trigger': {
      const [name, source, as, ae, bs, be] = ['timeline-trigger-name', 'timeline-trigger-source', 'timeline-trigger-activation-range-start', 'timeline-trigger-activation-range-end', 'timeline-trigger-active-range-start', 'timeline-trigger-active-range-end'].map(v);
      const activation = rangeText(as, ae, 'normal'), active = rangeText(bs, be, 'auto');
      if (name === 'none' && source === 'auto' && activation === 'normal' && active === 'auto') return 'none';
      let text = joined([name, source === 'auto' ? '' : source, activation === 'normal' && active === 'auto' ? '' : activation]);
      if (active !== 'auto') text = `${text} / ${active}`;
      return text;
    }
    default: return '';
  }
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
export function shorthandValue(name, declarations) {
  const entry = ENTRIES.get(name);
  if (!entry) return '';
  const values = [];
  for (const longhand of entry.longhands) {
    let found = null;
    for (let index = declarations.length - 1; index >= 0; index--) if (declarations[index].name === longhand) { found = declarations[index]; break; }
    if (!found) return '';
    values.push(found);
  }
  if (values.some((v) => v.important) && values.some((v) => !v.important)) return '';
  const first = values[0];
  const samePending = (a, b) => (a === null || a === undefined ? b === null || b === undefined : !!b && a[0] === b[0] && a[1] === b[1]);
  if (first.pending || CSS_WIDE.has(first.value)) {
    if (values.every((v) => v.value === first.value && samePending(first.pending, v.pending))) {
      if (first.pending) {
        // A system font keyword leaves the font-variant longhands at `normal`.
        if (first.pending[0] === 'font' && entry.canonical === 'font-variant' && SYSTEM_FONTS.includes(first.pending[1])) return 'normal';
        return first.pending[0] === entry.canonical ? first.pending[1] : '';
      }
      return first.value;
    }
  }
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
export function declarations(text) {
  const tokens = tokenize(String(text));
  const out = [];
  if (!tokens) return out;
  // Split at top-level semicolons, keeping each declaration's source span.
  const source = String(text);
  let depth = 0, start = 0, quote = '';
  for (let index = 0; index <= source.length; index++) {
    const c = source[index];
    if (index < source.length) {
      if (quote) { if (c === '\\') index++; else if (c === quote) quote = ''; continue; }
      if (c === '"' || c === "'") { quote = c; continue; }
      if (c === '/' && source[index + 1] === '*') { const end = source.indexOf('*/', index + 2); index = end < 0 ? source.length : end + 1; continue; }
      if (c === '(' || c === '[' || c === '{') { depth++; continue; }
      if (c === ')' || c === ']' || c === '}') { depth = Math.max(0, depth - 1); continue; }
      if (c !== ';' || depth > 0) continue;
    }
    const part = source.slice(start, index).replace(/\/\*[\s\S]*?\*\//g, '');
    start = index + 1;
    const colon = part.indexOf(':');
    if (colon < 0) continue;
    const name = part.slice(0, colon).trim();
    if (!/^-?[A-Za-z_-][A-Za-z0-9_-]*$/.test(name)) continue;
    let value = part.slice(colon + 1).trim();
    const important = /!\s*important\s*$/i.exec(value);
    if (important) value = value.slice(0, important.index).trim();
    else if (value.includes('!')) continue;
    out.push([name, value, !!important]);
  }
  return out;
}

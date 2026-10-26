// CSS selector engine for the JS runtime. Correctness-first, right-to-left matching,
// kept in lockstep with the Rust engine (crates/turbo-dom/src/rtdom/query.rs +
// query_utf16.rs + query_cascade.rs): both answer like Chrome 156, checked against the
// same control (crates/turbo-dom/tests/fixtures/cascade_selectors_chrome156.json, asserted
// here by test/selectors-chrome.test.mjs).
//
// Grammar: comma lists; descendant (' '), child ('>'), adjacent ('+') and general-sibling
// ('~') combinators; type / '*' selectors with the '*|' / '|' namespace prefixes; '#id',
// '.class' and '[attr]' / '[attr op value i]' (ops = ~= |= ^= $= *=, '*|' / '|' attribute
// prefixes); CSS identifiers with escapes and non-ASCII code points; pseudo-classes
// (structural, :nth-*(An+B [of S]), :is/:where/:not/:has, form state, :link, :lang(),
// :dir(), :defined, :root, :scope, :empty, :state(), plus the static-resting-state ones
// that are valid and never match: :hover, :focus, …); pseudo-elements (valid, never match
// an element). Every selector Chrome rejects throws a SyntaxError (empty items, dangling
// combinators, a type selector after another simple selector, unknown pseudos, invalid
// An+B, nested :has(), a simple selector after a pseudo-element, …).
//
// Hot paths stay allocation-free: the parse is cached by selector string, compounds are
// matched with index loops over node.__children(), classes with a whole-word scan.

import { isDefinedElement } from './custom-elements.mjs';

const SVG_NS = 'http://www.w3.org/2000/svg';

// ── character classes ───────────────────────────────────────────────────────
function isWs(c) {
  return c === ' ' || c === '\t' || c === '\n' || c === '\r' || c === '\f';
}
function isLetter(c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
function isDigit(c) {
  return c >= '0' && c <= '9';
}
function isHex(c) {
  return isDigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}
function isNameStart(c) {
  return isLetter(c) || c === '_' || c.charCodeAt(0) >= 0x80;
}
function isNameChar(c) {
  return isNameStart(c) || isDigit(c) || c === '-';
}
// A `\` at `i` is a valid escape unless a newline follows (a trailing `\` is U+FFFD).
function validEscape(src, i) {
  const n = src[i + 1];
  return src[i] === '\\' && n !== '\n' && n !== '\r' && n !== '\f';
}
// CSS Syntax "would start an identifier" at `i`.
function startsIdent(src, i) {
  const c = src[i];
  if (c === undefined) return false;
  if (isNameStart(c) || (c === '\\' && validEscape(src, i))) return true;
  if (c !== '-') return false;
  const d = src[i + 1];
  return d !== undefined && (d === '-' || isNameStart(d) || (d === '\\' && validEscape(src, i + 1)));
}
// Consume the escape at `src[i]` (a valid `\`): { ch, end }.
function consumeEscape(src, i) {
  let j = i + 1;
  if (j >= src.length) return { ch: '\uFFFD', end: j };
  if (!isHex(src[j])) {
    const cp = src.codePointAt(j);
    return { ch: String.fromCodePoint(cp), end: j + (cp > 0xffff ? 2 : 1) };
  }
  let hex = '';
  while (j < src.length && hex.length < 6 && isHex(src[j])) hex += src[j++];
  if (src[j] === '\r' && src[j + 1] === '\n') j += 2;
  else if (j < src.length && isWs(src[j])) j++;
  const cp = parseInt(hex, 16);
  const bad = cp === 0 || cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff);
  return { ch: bad ? '\uFFFD' : String.fromCodePoint(cp), end: j };
}
// Scan an identifier starting at `i` (caller checked startsIdent): { value, end }.
function scanIdent(src, i) {
  let value = '';
  while (i < src.length) {
    const c = src[i];
    if (c === '\\') {
      if (!validEscape(src, i)) break;
      const e = consumeEscape(src, i);
      value += e.ch;
      i = e.end;
    } else if (isNameChar(c)) {
      value += c;
      i++;
    } else break;
  }
  return { value, end: i };
}
// Is `s` (trimmed by the caller) exactly one identifier? Returns its decoded value or null.
function identValue(s) {
  if (!startsIdent(s, 0)) return null;
  const r = scanIdent(s, 0);
  return r.end === s.length ? r.value : null;
}
function trimWs(s) {
  let a = 0, b = s.length;
  while (a < b && isWs(s[a])) a++;
  while (b > a && isWs(s[b - 1])) b--;
  return s.slice(a, b);
}
function syntax(src, why) {
  return new SyntaxError(`'${src}' is not a valid selector: ${why}`);
}

// Scan from `start` (just past an opening `open`) to its matching `close`, honoring quotes,
// escapes and nesting: { inner, end } with `end` the index of the closer (or src.length).
function scanBalanced(src, start, open, close) {
  const n = src.length;
  let depth = 1, i = start, quote = null;
  for (; i < n; i++) {
    const d = src[i];
    if (d === '\\') { i++; continue; }
    if (quote) { if (d === quote) quote = null; continue; }
    if (d === '"' || d === "'") quote = d;
    else if (d === open) depth++;
    else if (d === close) { depth--; if (depth === 0) break; }
  }
  return { inner: src.slice(start, i), end: i };
}
// Split at top-level commas (outside quotes, escapes, () and []).
function splitTopLevel(src) {
  const out = [];
  let depth = 0, quote = null, last = 0;
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    if (c === '\\') { i++; continue; }
    if (quote) { if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'") quote = c;
    else if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth--;
    else if (c === ',' && depth === 0) { out.push(src.slice(last, i)); last = i + 1; }
  }
  out.push(src.slice(last));
  return out;
}

// ── attribute selectors ─────────────────────────────────────────────────────
const ATTR_OP = { '~': 'includes', '|': 'dash', '^': 'prefix', $: 'suffix', '*': 'substr' };
// Replace `/* … */` outside quotes with a space (comments are allowed inside `[…]`).
function stripAttrComments(s) {
  if (s.indexOf('/*') === -1) return s;
  let out = '', quote = null;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === '\\') { out += s.slice(i, i + 2); i++; continue; }
    if (quote) { if (c === quote) quote = null; out += c; continue; }
    if (c === '"' || c === "'") { quote = c; out += c; continue; }
    if (c === '/' && s[i + 1] === '*') {
      const end = s.indexOf('*/', i + 2);
      if (end === -1) throw new SyntaxError('unterminated comment in attribute selector');
      out += ' ';
      i = end + 1;
      continue;
    }
    out += c;
  }
  return out;
}
// A quoted string starting at `i` (src[i] is the quote): { value, end } (end past the
// closing quote; an unterminated string ends with the input, as in CSS — the attribute
// scanner has already rejected unbalanced quotes).
function scanString(src, i) {
  const q = src[i];
  let value = '';
  for (let j = i + 1; ; j++) {
    const c = src[j];
    if (c === q || j >= src.length) return { value, end: j + 1 };
    if (c === '\\') {
      const n = src[j + 1];
      if (n === '\n' || n === '\f') { j++; continue; }
      if (n === '\r') { j += src[j + 2] === '\n' ? 2 : 1; continue; }
      const e = consumeEscape(src, j);
      value += e.ch;
      j = e.end - 1;
      continue;
    }
    value += c;
  }
}
// Parse a `[ … ]` interior: { name, match, ns?, ci? } — `match` = { op: 'present' } or
// { op, value }; `ns: 'any'` for `[*|a]`; `ci: true` for the `i` modifier. Throws on what
// Chrome 156 rejects, including the `s` modifier (so does the Rust engine since
// turbo-dom#24).
function parseAttr(raw, src) {
  const s = stripAttrComments(raw);
  let i = 0;
  const skip = () => { while (i < s.length && isWs(s[i])) i++; };
  skip();
  let ns = null;
  if (s[i] === '*' && s[i + 1] === '|') { ns = 'any'; i += 2; }
  else if (s[i] === '|' && s[i + 1] !== '=') { i += 1; }
  if (!startsIdent(s, i)) throw syntax(src, 'attribute name');
  const nameTok = scanIdent(s, i);
  i = nameTok.end;
  if (s[i] === '|' && s[i + 1] !== '=') throw syntax(src, 'undeclared namespace prefix');
  const attr = { name: nameTok.value, match: { op: 'present' } };
  if (ns) attr.ns = ns;
  // the HTML-element lookup name, folded once (non-enumerable: the AST stays `{ name, match }`)
  Object.defineProperty(attr, 'lname', { value: nameTok.value.toLowerCase() });
  skip();
  if (i === s.length) return attr;
  let op = 'equals';
  if (ATTR_OP[s[i]] !== undefined && s[i + 1] === '=') { op = ATTR_OP[s[i]]; i += 2; }
  else if (s[i] === '=') i += 1;
  else throw syntax(src, 'attribute operator');
  skip();
  let value;
  if (s[i] === '"' || s[i] === "'") {
    const str = scanString(s, i);
    value = str.value;
    i = str.end;
  } else if (startsIdent(s, i)) {
    const v = scanIdent(s, i);
    value = v.value;
    i = v.end;
  } else throw syntax(src, 'attribute value');
  attr.match = { op, value };
  skip();
  if (i < s.length) {
    const m = s[i];
    if (m === 'i' || m === 'I') attr.ci = true;
    else throw syntax(src, 'attribute modifier');
    i++;
    skip();
    if (i < s.length) throw syntax(src, 'attribute selector');
  }
  return attr;
}

// ── pseudo-classes and pseudo-elements Chrome 156 accepts ───────────────────
// Implemented, argument-less.
const PLAIN = new Set([
  'first-child', 'last-child', 'only-child', 'first-of-type', 'last-of-type', 'only-of-type',
  'empty', 'root', 'scope', 'checked', 'disabled', 'enabled', 'required', 'optional',
  'read-only', 'read-write', 'link', 'any-link', 'defined',
]);
// Valid and never matching (a static document's resting state).
const NEVER = new Set([
  'active', 'autofill', '-webkit-autofill', 'buffering', 'current', 'default', 'focus',
  'focus-visible', 'focus-within', 'fullscreen', '-webkit-full-screen', 'future', 'host',
  'hover', 'in-range', 'indeterminate', 'invalid', 'modal', 'muted', 'open', 'out-of-range',
  'past', 'paused', 'picture-in-picture', 'placeholder-shown', 'playing', 'popover-open',
  'seeking', 'stalled', 'target', 'user-invalid', 'user-valid', 'valid', 'visited',
  'volume-locked', 'xr-overlay',
]);
const NEVER_FUNCTIONAL = new Set(['host', 'host-context']);
const PSEUDO_ELEMENTS = new Set([
  'after', 'backdrop', 'before', 'checkmark', 'column', 'cue', 'details-content',
  'file-selector-button', 'first-letter', 'first-line', 'grammar-error', 'marker',
  'picker-icon', 'placeholder', 'scroll-marker', 'scroll-marker-group', 'search-text',
  'selection', 'spelling-error', 'target-text', 'view-transition',
]);
const FUNCTIONAL_PSEUDO_ELEMENTS = new Set([
  'cue', 'highlight', 'part', 'picker', 'scroll-button', 'slotted', 'view-transition-group',
  'view-transition-image-pair', 'view-transition-new', 'view-transition-old',
]);
const LEGACY_PSEUDO_ELEMENTS = new Set(['after', 'before', 'first-letter', 'first-line']);
const AFTER_SCROLLBAR = new Set([
  'active', 'corner-present', 'decrement', 'disabled', 'double-button', 'enabled', 'end',
  'focus', 'focus-visible', 'focus-within', 'horizontal', 'hover', 'increment', 'no-button',
  'single-button', 'start', 'vertical', 'window-inactive',
]);
const USER_ACTION = new Set(['active', 'focus', 'focus-visible', 'focus-within', 'hover']);
const ELEMENT_BACKED = new Set([
  'details-content', 'file-selector-button', 'picker', 'scroll-button', 'scroll-marker',
  'scroll-marker-group',
]);
// The implemented non-structural pseudo-classes `::part()` may be followed by.
const PART_STATE = new Set(['checked', 'disabled', 'enabled', 'required', 'optional', 'read-only', 'read-write', 'link', 'any-link']);
// May pseudo-class `p` follow pseudo-element `pe`? (Chrome 156; mirrors the Rust
// allowed_after_pseudo_element.)
function allowedAfter(pe, p) {
  if (pe.startsWith('-webkit-')) return p.arg === null && AFTER_SCROLLBAR.has(p.name);
  if (pe === 'selection') return p.arg === null && p.name === 'window-inactive';
  if (pe === 'part') return p.arg === null ? NEVER.has(p.name) || PART_STATE.has(p.name) : p.name === 'state';
  if (ELEMENT_BACKED.has(pe)) return p.arg === null && USER_ACTION.has(p.name);
  return false;
}

// ── An+B ────────────────────────────────────────────────────────────────────
// `<an+b>` (CSS Syntax 3 §6) → { a, b } or null. Mirrors the Rust valid_anb + parse_nth.
function parseAnB(arg) {
  const s = trimWs(arg).toLowerCase();
  if (s === 'odd') return { a: 2, b: 1 };
  if (s === 'even') return { a: 2, b: 0 };
  let i = 0;
  let sign = 1;
  if (s[0] === '+' || s[0] === '-') { sign = s[0] === '-' ? -1 : 1; i = 1; }
  const ds = i;
  while (i < s.length && isDigit(s[i])) i++;
  const digits = s.slice(ds, i);
  if (s[i] !== 'n') {
    if (!digits || i !== s.length) return null;
    return { a: 0, b: sign * parseInt(digits, 10) };
  }
  const a = sign * (digits ? parseInt(digits, 10) : 1);
  i++;
  while (i < s.length && isWs(s[i])) i++;
  if (i === s.length) return { a, b: 0 };
  if (s[i] !== '+' && s[i] !== '-') return null;
  const bs = s[i] === '-' ? -1 : 1;
  i++;
  while (i < s.length && isWs(s[i])) i++;
  const bd = i;
  while (i < s.length && isDigit(s[i])) i++;
  if (i === bd) return null;
  const b = bs * parseInt(s.slice(bd, i), 10);
  while (i < s.length && isWs(s[i])) i++;
  return i === s.length ? { a, b } : null;
}
// The `of` keyword of `:nth-child(An+B of S)`: lowercase, whitespace before it, a non-empty
// An+B part, and no name code point right after it. Index or -1.
function nthOfPosition(arg) {
  for (let i = 1; i < arg.length - 1; i++) {
    if (arg[i] !== 'o' || arg[i + 1] !== 'f' || !isWs(arg[i - 1])) continue;
    if (!trimWs(arg.slice(0, i))) continue;
    const after = arg[i + 2];
    if (after === undefined || !(isNameChar(after) || after === '\\')) return i;
  }
  return -1;
}
function nthMatch(a, b, index) {
  if (a === 0) return index === b;
  const d = index - b;
  return d % a === 0 && d / a >= 0;
}

// ── lexer ───────────────────────────────────────────────────────────────────
// Token kinds: ws | comma | comb (v) | type (ns, name) | id (v) | class (v) | attr |
// pseudo (name, arg) | pelem (name, arg).
function tokenize(src) {
  const tokens = [];
  const n = src.length;
  let i = 0;
  while (i < n) {
    const c = src[i];
    if (isWs(c)) {
      while (i < n && isWs(src[i])) i++;
      tokens.push({ k: 'ws' });
    } else if (c === ',') { tokens.push({ k: 'comma' }); i++; }
    else if (c === '>' || c === '+' || c === '~') { tokens.push({ k: 'comb', v: c }); i++; }
    else if (c === '*' || c === '|' || startsIdent(src, i)) {
      // type / universal selector with an optional `*|` / `|` namespace prefix
      let ns = null, name = null;
      if (c === '*') i++;
      else if (c !== '|') { const t = scanIdent(src, i); name = t.value; i = t.end; }
      if (src[i] === '|' && src[i + 1] !== '=') {
        if (name !== null) throw syntax(src, 'undeclared namespace prefix');
        ns = c === '*' ? 'any' : 'none';
        i++;
        if (src[i] === '*') i++;
        else if (startsIdent(src, i)) { const t = scanIdent(src, i); name = t.value; i = t.end; }
        else throw syntax(src, 'type selector after a namespace prefix');
      } else if (c === '|') throw syntax(src, `unexpected '|'`);
      tokens.push({ k: 'type', ns, name: name === null ? null : name.toLowerCase() });
    } else if (c === '#' || c === '.') {
      if (!startsIdent(src, i + 1)) throw syntax(src, `unexpected '${c}'`);
      const t = scanIdent(src, i + 1);
      tokens.push({ k: c === '#' ? 'id' : 'class', v: t.value });
      i = t.end;
    } else if (c === '[') {
      const { inner, end } = scanBalanced(src, i + 1, '[', ']');
      if (end >= n) throw new SyntaxError(`unterminated attribute selector: ${src.slice(i)}`);
      tokens.push({ k: 'attr', attr: parseAttr(inner, src) });
      i = end + 1;
    } else if (c === ':') {
      i++;
      const element = src[i] === ':';
      if (element) i++;
      if (!startsIdent(src, i)) throw syntax(src, `unexpected ':'`);
      const t = scanIdent(src, i);
      const name = t.value.toLowerCase();
      i = t.end;
      let arg = null;
      if (src[i] === '(') {
        const { inner, end } = scanBalanced(src, i + 1, '(', ')');
        arg = inner;
        i = end < n ? end + 1 : n; // an unterminated argument closes at the end (CSS)
      }
      const legacy = !element && LEGACY_PSEUDO_ELEMENTS.has(name);
      tokens.push({ k: element || legacy ? 'pelem' : 'pseudo', name, arg, legacy });
    } else {
      throw new SyntaxError(`unexpected '${c}' in selector`);
    }
  }
  return tokens;
}

// ── parser ──────────────────────────────────────────────────────────────────
// A Complex is `{ first, rest: [{ combinator, compound }] }`; a compound is
// `{ tag, ns, id, classes, attrs, pseudos, pe }`: `tag` null = no type selector, '*' =
// universal, else the lowercase name; `ns` the type's namespace prefix; `pe` the pseudo-element; pseudos are `{ name, arg }` with the
// parsed argument kept in a non-enumerable `parsed`.

// Parse one pseudo-class (throws on what Chrome rejects). `nested`: inside a pseudo
// argument; `inHas`: inside `:has()` (which cannot nest).
function parsePseudo(tok, src, inHas) {
  const { name, arg } = tok;
  const p = { name, arg };
  let parsed = null;
  const functional = arg !== null;
  if (PLAIN.has(name)) {
    if (functional) throw syntax(src, `:${name}() takes no argument`);
  } else if (NEVER.has(name) && !functional) {
    parsed = { never: true };
  } else if (NEVER_FUNCTIONAL.has(name) && functional) {
    parsed = { never: true };
  } else if (!functional) {
    throw syntax(src, `unknown pseudo-class :${name}`);
  } else if (name === 'not') {
    parsed = { list: parseList(arg, { nested: true, forgiving: false, inHas }) };
  } else if (name === 'is' || name === 'where' || name === '-webkit-any') {
    parsed = { list: parseList(arg, { nested: true, forgiving: true, inHas }) };
  } else if (name === 'has') {
    if (inHas) throw syntax(src, ':has() inside :has()');
    parsed = { rel: parseRelative(arg, src) };
  } else if (name === 'nth-child' || name === 'nth-last-child' || name === 'nth-of-type' || name === 'nth-last-of-type') {
    const at = name.endsWith('child') ? nthOfPosition(arg) : -1;
    const anb = parseAnB(at === -1 ? arg : arg.slice(0, at));
    if (anb === null) throw syntax(src, `:${name}() argument`);
    parsed = { a: anb.a, b: anb.b, of: at === -1 ? null : parseList(arg.slice(at + 2), { nested: true, forgiving: false, inHas }) };
  } else if (name === 'lang' || name === 'dir') {
    const v = identValue(trimWs(arg));
    if (v === null) throw syntax(src, `:${name}() takes one identifier`);
    parsed = { v: v.toLowerCase() };
  } else if (name === 'state') {
    const v = identValue(trimWs(arg));
    if (v === null) throw syntax(src, ':state() takes one identifier');
    parsed = { v };
  } else {
    throw syntax(src, `unknown pseudo-class :${name}()`);
  }
  if (parsed) Object.defineProperty(p, 'parsed', { value: parsed });
  return p;
}

// Parse one pseudo-element (throws on unknown ones): { name, arg, slotted? }.
function parsePseudoElement(tok, src) {
  const { name, arg } = tok;
  if (tok.legacy && arg !== null) throw syntax(src, `:${name}() takes no argument`);
  const known = arg === null ? PSEUDO_ELEMENTS.has(name) : FUNCTIONAL_PSEUDO_ELEMENTS.has(name);
  if (!known && !name.startsWith('-webkit-')) throw syntax(src, `unknown pseudo-element ::${name}`);
  const pe = { name, arg };
  if (name === 'slotted' && arg !== null) {
    const list = parseList(arg, { nested: true, forgiving: false, inHas: false });
    if (list.length !== 1) throw syntax(src, '::slotted() takes one compound selector');
    // kept for specificity (non-enumerable, like a pseudo-class's `parsed`)
    Object.defineProperty(pe, 'parsed', { value: list });
  }
  return pe;
}

// Parse tokens[lo..hi) (one complex selector, commas already split out). Throws on an empty
// item, a dangling combinator, a type selector after another simple selector, and a
// pseudo-element anywhere but at the end of the last compound.
function parseComplexTokens(tokens, lo, hi, src, opts) {
  let start = lo, end = hi;
  while (start < end && tokens[start].k === 'ws') start++;
  while (end > start && tokens[end - 1].k === 'ws') end--;
  if (start >= end) throw syntax(src, 'empty selector');
  let first = null;
  const rest = [];
  let combinator = null;
  let pos = start;
  let sawPe = false;
  while (pos < end) {
    const t = tokens[pos];
    if (t.k === 'ws') { pos++; if (combinator === null && first !== null) combinator = ' '; continue; }
    if (t.k === 'comb') {
      if (first === null || (combinator !== null && combinator !== ' ')) throw syntax(src, 'dangling combinator');
      combinator = t.v;
      pos++;
      continue;
    }
    if (sawPe) throw syntax(src, 'combinator after a pseudo-element');
    // a compound
    // one object shape for every compound (monomorphic matching): `ns` is null / 'any' /
    // 'none', `pe` the pseudo-element or null
    const compound = { tag: null, ns: null, id: null, classes: [], attrs: [], pseudos: [], pe: null };
    let parts = 0;
    let pe = null;
    for (; pos < end; pos++) {
      const p = tokens[pos];
      if (p.k === 'ws' || p.k === 'comb') break;
      if (pe !== null && p.k !== 'pseudo') throw syntax(src, 'simple selector after a pseudo-element');
      if (p.k === 'type') {
        if (parts) throw syntax(src, 'type selector after another simple selector');
        compound.tag = p.name === null ? '*' : p.name;
        compound.ns = p.ns;
      } else if (p.k === 'id') compound.id = p.v;
      else if (p.k === 'class') compound.classes.push(p.v);
      else if (p.k === 'attr') compound.attrs.push(p.attr);
      else if (p.k === 'pelem') {
        if (opts.nested) throw syntax(src, 'pseudo-element inside a pseudo-class argument');
        if (pe !== null) throw syntax(src, 'two pseudo-elements');
        pe = parsePseudoElement(p, src);
      } else if (pe !== null) {
        // after a pseudo-element only the pseudo-classes Chrome allows there (the selector
        // never matches an element either way)
        if (!allowedAfter(pe.name, p)) throw syntax(src, `:${p.name} after ::${pe.name}`);
        compound.pseudos.push({ name: p.name, arg: p.arg });
      } else {
        compound.pseudos.push(parsePseudo(p, src, opts.inHas));
      }
      parts++;
    }
    if (pe !== null) { compound.pe = pe; sawPe = true; }
    if (first === null) first = compound;
    else rest.push({ combinator: combinator === null ? ' ' : combinator, compound });
    combinator = null;
  }
  if (combinator !== null && combinator !== ' ') throw syntax(src, 'dangling combinator');
  return { first, rest };
}

// A selector-list argument / list: `forgiving` (`:is()` / `:where()`) drops invalid items.
function parseList(src, opts) {
  const out = [];
  for (const item of splitTopLevel(src)) {
    try {
      const tokens = tokenize(item);
      out.push(parseComplexTokens(tokens, 0, tokens.length, item, opts));
    } catch (e) {
      if (!opts.forgiving) throw e;
    }
  }
  return out;
}

// `:has()` relative selector list (unforgiving): [{ combinator, complex }].
function parseRelative(arg, src) {
  const out = [];
  for (const item of splitTopLevel(arg)) {
    const tokens = tokenize(item);
    let lo = 0;
    while (lo < tokens.length && tokens[lo].k === 'ws') lo++;
    let combinator = ' ';
    if (lo < tokens.length && tokens[lo].k === 'comb') { combinator = tokens[lo].v; lo++; }
    out.push({ combinator, complex: parseComplexTokens(tokens, lo, tokens.length, src, { nested: true, forgiving: false, inHas: true }) });
  }
  return out;
}

// Parse one complex selector STRING (no top-level comma). Exposed via `_internal`.
function parseComplex(src) {
  const tokens = tokenize(src);
  return parseComplexTokens(tokens, 0, tokens.length, src, { nested: false, forgiving: false, inHas: false });
}

// parsed-selector cache: querySelector(All)/matches re-run the same selector strings
// constantly; parsing once and reusing is a large win on query-heavy suites. Bounded so a
// pathological generator can't grow it without limit. Invalid selectors are not cached
// (they throw every time, like the browser).
const __selectorCache = new Map();
export function parseSelectorList(selector) {
  const hit = __selectorCache.get(selector);
  if (hit !== undefined) return hit;
  // CSS preprocessing: NUL becomes U+FFFD (a name code point)
  const src = String(selector).replace(/\0/g, '\uFFFD');
  const parsed = parseList(src, { nested: false, forgiving: false, inHas: false });
  if (__selectorCache.size > 10000) __selectorCache.clear();
  __selectorCache.set(selector, parsed);
  return parsed;
}

// The items of a style rule's selector list (top-level comma split), or [] when the list is
// invalid — a browser drops the whole rule (cascade.mjs; mirrors rtdom::cascade).
export function ruleSelectors(prelude) {
  try { parseSelectorList(prelude); } catch { return []; }
  return splitTopLevel(prelude).map(trimWs);
}

// Selectors 4 specificity `[ids, classes + attributes + pseudo-classes, types +
// pseudo-elements]` of one complex selector of a (valid) rule, compared lexicographically by
// compareSpecificity — mirrors the Rust query_cascade specificity (miaskiewicz/turbo-dom#24):
// `:where()` is 0; `:is()`, `:not()` and `:has()` take their most specific argument;
// `:nth-child(An+B of S)` is a pseudo-class plus S; `*` counts nothing; a pseudo-element
// counts as a type (`::slotted(S)` plus S); everything else is one pseudo-class.
function addSpecificity(a, b) {
  a[0] += b[0]; a[1] += b[1]; a[2] += b[2];
  return a;
}
export function compareSpecificity(a, b) {
  return a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
}
function maxSpecificity(list) {
  let best = [0, 0, 0];
  for (const cx of list) {
    const s = complexSpecificity(cx);
    if (compareSpecificity(s, best) > 0) best = s;
  }
  return best;
}
function complexSpecificity(cx) {
  const s = compoundSpecificity(cx.first);
  for (const r of cx.rest) addSpecificity(s, compoundSpecificity(r.compound));
  return s;
}
function compoundSpecificity(c) {
  const s = [c.id !== null ? 1 : 0, c.classes.length + c.attrs.length, c.tag !== null && c.tag !== '*' ? 1 : 0];
  for (const p of c.pseudos) addSpecificity(s, pseudoSpecificity(p));
  if (c.pe !== null) {
    s[2]++;
    if (c.pe.parsed) addSpecificity(s, maxSpecificity(c.pe.parsed));
  }
  return s;
}
function pseudoSpecificity(p) {
  const parsed = p.parsed;
  if (p.name === 'where') return [0, 0, 0];
  if (parsed !== undefined && parsed.list !== undefined && p.name !== '-webkit-any') return maxSpecificity(parsed.list);
  if (parsed !== undefined && parsed.rel !== undefined) return maxSpecificity(parsed.rel.map((r) => r.complex));
  if (parsed !== undefined && parsed.of) return addSpecificity([0, 1, 0], maxSpecificity(parsed.of));
  return [0, 1, 0];
}
// The specificity of one item of ruleSelectors() (a valid complex selector).
export function specificity(item) {
  return complexSpecificity(parseSelectorList(item)[0]);
}

// ── element state helpers ───────────────────────────────────────────────────
// HTML elements carry no short namespace (`__ns` is '' — 'svg' / 'math' otherwise).
function isHtml(el) {
  return !el.__ns;
}
function attrList(el) {
  return el.__attrs ?? (el.__buildAttrs ? (el.__attrs = el.__buildAttrs()) : []);
}
// The no-namespace attribute `name` (foreign elements: skip prefixed attributes such as
// xlink:href), or null.
function plainAttr(el, name) {
  if (!el.__ns) return el.getAttribute(name);
  const at = attrList(el);
  for (let i = 0; i < at.length; i++) if (at[i].name === name && !at[i].prefix) return at[i].value;
  return null;
}
function rawChildren(node) {
  return typeof node.__children === 'function' ? node.__children() : Array.from(node.childNodes || []);
}
function previousElement(el) {
  let n = el.previousSibling;
  while (n && n.nodeType !== 1) n = n.previousSibling;
  return n || null;
}
function nextElement(el) {
  let n = el.nextSibling;
  while (n && n.nodeType !== 1) n = n.nextSibling;
  return n || null;
}
// (1-based index, count) among the element siblings (`ofType`: of the same local name). A
// parentless element is its own only sibling (as the Rust engine).
function position(el, ofType) {
  const p = el.parentNode;
  if (!p) return [1, 1];
  const kids = rawChildren(p);
  let index = 0, count = 0;
  for (let i = 0; i < kids.length; i++) {
    const c = kids[i];
    if (c.nodeType !== 1 || (ofType && c.localName !== el.localName)) continue;
    count++;
    if (c === el) index = count;
  }
  return [index, count];
}
function positionOf(el, list, last) {
  const p = el.parentNode;
  const kids = p ? rawChildren(p) : [el];
  let index = 0, count = 0;
  for (let i = 0; i < kids.length; i++) {
    const c = kids[i];
    if (c.nodeType !== 1 || (c !== el && !list.some((cx) => matchComplex(c, cx)))) continue;
    count++;
    if (c === el) index = count;
  }
  return last ? count - index + 1 : index;
}
function boolAttr(el, prop, attr) {
  const v = el[prop];
  return typeof v === 'boolean' ? v : el.hasAttribute(attr);
}
const NAMES = (s) => new Set(s.split(' '));
const READONLY_INPUT_TYPES = NAMES('text search url tel email password date month week time datetime-local number');
const KNOWN_INPUT_TYPES = NAMES('hidden text search tel url email password date month week time datetime-local number range color checkbox radio file submit image reset button');
const AUTO_DIR_INPUT_TYPES = NAMES('hidden text search tel url email password submit reset button');
function inputType(el) {
  const t = el.getAttribute('type');
  return t === null ? null : t.toLowerCase();
}
// HTML "actually disabled" (button/input/select/textarea/fieldset): own state or a disabled
// fieldset ancestor outside its first legend child.
function actuallyDisabled(el) {
  if (boolAttr(el, 'disabled', 'disabled')) return true;
  let child = el;
  for (let a = el.parentNode; a && a.nodeType === 1; child = a, a = a.parentNode) {
    if (a.localName !== 'fieldset' || !isHtml(a) || !boolAttr(a, 'disabled', 'disabled')) continue;
    const kids = rawChildren(a);
    let legend = null;
    for (let i = 0; i < kids.length && legend === null; i++) if (kids[i].nodeType === 1 && kids[i].localName === 'legend') legend = kids[i];
    if (legend !== child) return true;
  }
  return false;
}
// :disabled / :enabled: undefined when neither applies, else whether `el` is disabled.
function disabledState(el) {
  if (!isHtml(el)) return undefined;
  switch (el.localName) {
    case 'button': case 'input': case 'select': case 'textarea': case 'fieldset':
      return actuallyDisabled(el);
    case 'optgroup':
      return boolAttr(el, 'disabled', 'disabled');
    case 'option': {
      if (el.hasAttribute('disabled')) return true;
      let p = el.parentNode;
      if (p && p.nodeType === 1 && p.localName === 'optgroup') {
        if (p.hasAttribute('disabled')) return true;
        p = p.parentNode;
      }
      return !!(p && p.nodeType === 1 && p.localName === 'select' && actuallyDisabled(p));
    }
    default:
      return undefined;
  }
}
function isEditable(el) {
  for (let n = el; n && n.nodeType === 1; n = n.parentNode) {
    if (!isHtml(n)) continue;
    const v = n.getAttribute('contenteditable');
    if (v === null) continue;
    const l = v.toLowerCase();
    if (l === '' || l === 'true' || l === 'plaintext-only') return true;
    if (l === 'false') return false;
  }
  return false;
}
// The HTML :read-write rule (HTML elements; others match neither).
function readWrite(el) {
  if (el.localName === 'input') {
    const t = inputType(el);
    const applies = t === null || READONLY_INPUT_TYPES.has(t) || !KNOWN_INPUT_TYPES.has(t);
    return applies && !boolAttr(el, 'readOnly', 'readonly') && !actuallyDisabled(el);
  }
  if (el.localName === 'textarea') return !boolAttr(el, 'readOnly', 'readonly') && !actuallyDisabled(el);
  return isEditable(el);
}
function isEmpty(el) {
  const kids = rawChildren(el);
  for (let i = 0; i < kids.length; i++) {
    const c = kids[i];
    if (c.nodeType === 1) return false;
    if (c.nodeType === 3 && c.data !== '') return false;
  }
  return true;
}
function isLink(el) {
  if (isHtml(el)) return (el.localName === 'a' || el.localName === 'area') && el.hasAttribute('href');
  return el.namespaceURI === SVG_NS && el.localName === 'a' && el.getAttribute('href') !== null;
}
// :defined — the element's custom element state (custom-elements.mjs: the creation-time
// state, upgrades and constructions), as the Rust engine's Tree::custom_element_state.
// :lang(range): the nearest `lang` / `xml:lang` (basic prefix match, case-insensitive).
function langMatches(el, range) {
  for (let n = el; n && n.nodeType === 1; n = n.parentNode) {
    const v = n.getAttribute('lang');
    if (v === null) continue;
    const l = v.toLowerCase();
    return l === range || (l.length > range.length && l.startsWith(range) && l[range.length] === '-');
  }
  return false;
}
function dirState(el) {
  const v = el.getAttribute('dir');
  if (v === null) return null;
  const l = v.toLowerCase();
  return l === 'ltr' || l === 'rtl' || l === 'auto' ? l : null;
}
// First strong character: true = RTL (bidi R / AL), false = LTR, null = none. The RTL
// ranges cover the Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan, Mandaic and RTL
// presentation-form blocks; every other letter is LTR.
const RTL_RE = /[\u0590-\u08FF\uFB1D-\uFDFF\uFE70-\uFEFF]|\uD802[\uDC00-\uDFFF]|\uD803[\uDC00-\uDFFF]|\uD83A[\uDC00-\uDFFF]|\uD83B[\uDC00-\uDEFF]/;
const LETTER_RE = /\p{L}/u;
function strongRtl(text) {
  for (const ch of text) {
    if (RTL_RE.test(ch)) return true;
    if (LETTER_RE.test(ch)) return false;
  }
  return null;
}
function autoRtl(el) {
  const ln = el.localName;
  if (ln === 'textarea' || (ln === 'input' && (inputType(el) === null || AUTO_DIR_INPUT_TYPES.has(inputType(el)) || !KNOWN_INPUT_TYPES.has(inputType(el))))) {
    const value = ln === 'textarea' ? el.textContent : (el.getAttribute('value') || '');
    const r = strongRtl(value);
    return r !== null ? r : value !== '' ? false : null;
  }
  const walk = (node) => {
    const kids = rawChildren(node);
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType === 3) { const r = strongRtl(c.data); if (r !== null) return r; continue; }
      if (c.nodeType !== 1) continue;
      if (isHtml(c) && (c.localName === 'bdi' || c.localName === 'script' || c.localName === 'style' || c.localName === 'textarea' || dirState(c) !== null)) continue;
      const r = walk(c);
      if (r !== null) return r;
    }
    return null;
  };
  return walk(el);
}
// HTML directionality: true = rtl. Mirrors the Rust query_cascade::is_rtl.
function isRtl(el) {
  for (let n = el; n;) {
    if (isHtml(n)) {
      const d = dirState(n);
      if (d === 'ltr') return false;
      if (d === 'rtl') return true;
      if (d === 'auto' || (d === null && n.localName === 'bdi')) return autoRtl(n) === true;
      if (d === null && n.localName === 'input' && inputType(n) === 'tel') return false;
    }
    const p = n.parentNode;
    n = p && p.__isShadowRoot ? p.host : p && p.nodeType === 1 ? p : null;
  }
  return false;
}

// ── matching ────────────────────────────────────────────────────────────────
function matchValue(m, got, ci) {
  if (m.op === 'present') return true;
  let v = got, t = m.value;
  if (ci) { v = v.toLowerCase(); t = t.toLowerCase(); }
  switch (m.op) {
    case 'equals': return v === t;
    case 'prefix': return t !== '' && v.startsWith(t);
    case 'suffix': return t !== '' && v.endsWith(t);
    case 'substr': return t !== '' && v.includes(t);
    case 'includes': return t !== '' && !/\s/.test(t) && v.split(/[ \t\n\r\f]+/).includes(t);
    default: return v === t || v.startsWith(t + '-'); // dash
  }
}
function matchAttr(el, a) {
  const html = !el.__ns;
  const name = html ? a.lname : a.name;
  if (a.ns === 'any') {
    const at = attrList(el);
    for (let i = 0; i < at.length; i++) if (at[i].name === name && matchValue(a.match, at[i].value, a.ci === true)) return true;
    return false;
  }
  const raw = html ? el.getAttribute(name) : plainAttr(el, name);
  if (raw === null) return false;
  if (a.match.op === 'present') return true;
  // HTML `type` on input/button/ol compares case-insensitively by default (as Rust)
  const implicit = html && name === 'type' && (el.localName === 'input' || el.localName === 'button' || el.localName === 'ol');
  return matchValue(a.match, raw, a.ci === true || implicit);
}

// The element `:scope` matches during the current match (null = stylesheet: `:root`).
let currentScope = null;

function matchPseudo(el, p) {
  const parsed = p.parsed;
  switch (p.name) {
    case 'not': return !parsed.list.some((cx) => matchComplex(el, cx));
    case 'is': case 'where': case '-webkit-any': return parsed.list.some((cx) => matchComplex(el, cx));
    case 'has': return parsed.rel.some((rel) => hasMatch(el, rel));
    // O(1) sibling steps (a parentless element is its own only sibling)
    case 'first-child': return previousElement(el) === null;
    case 'last-child': return nextElement(el) === null;
    case 'only-child': return previousElement(el) === null && nextElement(el) === null;
    case 'first-of-type': return position(el, true)[0] === 1;
    case 'last-of-type': { const [i, n] = position(el, true); return i === n; }
    case 'only-of-type': return position(el, true)[1] === 1;
    case 'nth-child': case 'nth-last-child': {
      const last = p.name === 'nth-last-child';
      if (parsed.of) {
        if (!parsed.of.some((cx) => matchComplex(el, cx))) return false;
        return nthMatch(parsed.a, parsed.b, positionOf(el, parsed.of, last));
      }
      const [i, n] = position(el, false);
      return nthMatch(parsed.a, parsed.b, last ? n - i + 1 : i);
    }
    case 'nth-of-type': case 'nth-last-of-type': {
      const [i, n] = position(el, true);
      return nthMatch(parsed.a, parsed.b, p.name === 'nth-last-of-type' ? n - i + 1 : i);
    }
    case 'empty': return isEmpty(el);
    case 'root': return !!el.parentNode && el.parentNode.nodeType === 9;
    case 'scope': return currentScope === null ? !!el.parentNode && el.parentNode.nodeType === 9 : el === currentScope;
    // form state reads the live PROPERTY first (what React sets), then the attribute
    case 'checked':
      if (el.localName === 'option') return !!el.selected;
      return el.localName === 'input' && (inputType(el) === 'checkbox' || inputType(el) === 'radio') && boolAttr(el, 'checked', 'checked');
    case 'disabled': return disabledState(el) === true;
    case 'enabled': return disabledState(el) === false;
    case 'required': return boolAttr(el, 'required', 'required');
    case 'optional': return (el.localName === 'input' || el.localName === 'select' || el.localName === 'textarea') && !boolAttr(el, 'required', 'required');
    case 'read-only': return isHtml(el) && !readWrite(el);
    case 'read-write': return isHtml(el) && readWrite(el);
    case 'link': case 'any-link': return isLink(el);
    case 'defined': return isDefinedElement(el);
    case 'lang': return langMatches(el, parsed.v);
    case 'dir': return (parsed.v === 'rtl' || parsed.v === 'ltr') && isRtl(el) === (parsed.v === 'rtl');
    // `:state()` parses; this runtime has no ElementInternals custom states, so it never
    // matches (falls through to the never-matching default)
    default: return false; // the valid never-matching pseudo-classes (:hover, :focus, …)
  }
}

// allocation-free "does the class attribute contain this whole-word class"
function hasClass(cn, cls) {
  if (cn === cls) return true;
  const L = cls.length;
  let idx = cn.indexOf(cls);
  while (idx !== -1) {
    const before = idx === 0 || isWs(cn[idx - 1]);
    const after = idx + L === cn.length || isWs(cn[idx + L]);
    if (before && after) return true;
    idx = cn.indexOf(cls, idx + 1);
  }
  return false;
}

function matchCompound(el, compound) {
  if (!el || el.nodeType !== 1) return false;
  if (compound.pe) return false; // a pseudo-element is never an element
  if (compound.ns === 'none' && el.namespaceURI !== null) return false;
  const tag = compound.tag;
  if (tag !== null && tag !== '*') {
    const ln = el.localName;
    // type selectors compare ASCII case-insensitively (as the Rust engine); HTML local
    // names are already lowercase, so only foreign elements pay for the fold
    if (ln !== tag && (!el.__ns || ln.toLowerCase() !== tag)) return false;
  }
  if (compound.id !== null && plainAttr(el, 'id') !== compound.id) return false;
  // classes: read the class attribute ONCE and test membership without allocating
  if (compound.classes.length) {
    const cn = plainAttr(el, 'class');
    if (!cn) return false;
    for (const cls of compound.classes) if (!hasClass(cn, cls)) return false;
  }
  for (const a of compound.attrs) if (!matchAttr(el, a)) return false;
  for (const p of compound.pseudos) if (!matchPseudo(el, p)) return false;
  return true;
}

// The k-th compound of a Complex, indexed left-to-right (0 = `first`, the leftmost).
function compoundAt(cx, k) {
  return k === 0 ? cx.first : cx.rest[k - 1].compound;
}

// Match a parsed complex selector against `el` (the rightmost compound applies to el).
function matchComplex(el, cx) {
  return matchChain(el, cx, cx.rest.length, null);
}

// Match compounds 0..=k of `cx` ending at `el`, recursing leftward. Descendant (' ') and
// general-sibling ('~') combinators try EVERY candidate ancestor/sibling and backtrack.
// `scope` supports `:has()`: `{ comb, node }` anchors the leftmost compound to the subject.
function matchChain(el, cx, k, scope = null) {
  if (!matchCompound(el, compoundAt(cx, k))) return false;
  if (k === 0) return scope === null ? true : relatesToScope(el, scope.comb, scope.node);
  const comb = cx.rest[k - 1].combinator;
  if (comb === '>') {
    const p = el.parentNode;
    return !!p && p.nodeType === 1 && matchChain(p, cx, k - 1, scope);
  }
  if (comb === '+') {
    const prev = previousElement(el);
    return !!prev && matchChain(prev, cx, k - 1, scope);
  }
  if (comb === ' ') {
    for (let anc = el.parentNode; anc && anc.nodeType === 1; anc = anc.parentNode) {
      if (matchChain(anc, cx, k - 1, scope)) return true;
    }
    return false;
  }
  for (let prev = previousElement(el); prev; prev = previousElement(prev)) {
    if (matchChain(prev, cx, k - 1, scope)) return true;
  }
  return false;
}

// Does `node` stand in relation `comb` to the `:has()` subject `el`?
function relatesToScope(node, comb, el) {
  if (comb === '>') return node.parentNode === el;
  if (comb === '+') return previousElement(node) === el;
  if (comb === '~') { for (let p = previousElement(node); p; p = previousElement(p)) if (p === el) return true; return false; }
  for (let a = node.parentNode; a; a = a.parentNode) if (a === el) return true;
  return false;
}

// :has() existence search (mirrors the Rust has_match).
function hasMatch(el, rel) {
  const { combinator, complex } = rel;
  const scope = { comb: combinator, node: el };
  const test = (x) => matchChain(x, complex, complex.rest.length, scope);
  if (combinator === '+') {
    const sib = nextElement(el);
    return !!sib && someSelfOrDescendant(sib, test);
  }
  if (combinator === '~') {
    for (let sib = nextElement(el); sib; sib = nextElement(sib)) if (someSelfOrDescendant(sib, test)) return true;
    return false;
  }
  return someDescendant(el, test);
}

// Visit every descendant element of `node` (document order; template content is not a
// child); short-circuit on the first that satisfies `pred`.
function someDescendant(node, pred) {
  const kids = rawChildren(node);
  for (let i = 0; i < kids.length; i++) {
    const c = kids[i];
    if (c.nodeType !== 1) continue;
    if (pred(c)) return true;
    if (someDescendant(c, pred)) return true;
  }
  return false;
}
function someSelfOrDescendant(node, pred) {
  if (node.nodeType === 1 && pred(node)) return true;
  return someDescendant(node, pred);
}

// `el.matches(selector)`; `scope` is the `:scope` element (`el` by default; null in a
// stylesheet, where `:scope` is `:root`). Throws SyntaxError on an invalid selector.
export function matchesSelector(el, selector, scope = el) {
  const list = parseSelectorList(selector);
  const saved = currentScope;
  currentScope = scope;
  try {
    return list.some((cx) => matchComplex(el, cx));
  } finally {
    currentScope = saved;
  }
}

// Fast paths for the overwhelmingly common simple selectors, skipping the parse +
// per-element matchComplex machinery.
const SIMPLE = /^\s*(#[A-Za-z_][\w-]*|\.[A-Za-z_][\w-]*|[a-zA-Z][\w-]*)\s*$/;
function simpleMatcher(selector) {
  const m = SIMPLE.exec(selector);
  if (!m) return null;
  const s = m[1];
  if (s[0] === '#') { const id = s.slice(1); return (el) => (el.__ns ? plainAttr(el, 'id') : el.getAttribute('id')) === id; }
  if (s[0] === '.') { const cls = s.slice(1); return (el) => { const cn = el.__ns ? plainAttr(el, 'class') : el.getAttribute('class'); return cn ? hasClass(cn, cls) : false; }; }
  const tag = s.toLowerCase();
  return (el) => el.localName === tag || (!!el.__ns && el.localName.toLowerCase() === tag);
}

// The `:scope` element of a query rooted at `root` (the document element for a document).
function scopeOf(root) {
  if (root.nodeType === 1) return root;
  return root.nodeType === 9 ? root.documentElement || null : undefined;
}

function queryWalk(root, selector, first) {
  const out = [];
  const simple = simpleMatcher(selector);
  if (simple) {
    // no pseudo-classes in a simple selector, so no :scope to set
    const visit = (node) => {
      const kids = rawChildren(node);
      for (let i = 0; i < kids.length; i++) {
        const c = kids[i];
        if (c.nodeType !== 1) continue;
        if (simple(c)) { out.push(c); if (first) return true; }
        if (visit(c)) return true;
      }
      return false;
    };
    visit(root);
    return out;
  }
  const list = parseSelectorList(selector);
  const single = list.length === 1 ? list[0] : null;
  const saved = currentScope;
  currentScope = scopeOf(root);
  try {
    const visit = (node) => {
      const kids = rawChildren(node);
      for (let i = 0; i < kids.length; i++) {
        const c = kids[i];
        if (c.nodeType !== 1) continue;
        if (single ? matchComplex(c, single) : list.some((cx) => matchComplex(c, cx))) { out.push(c); if (first) return true; }
        if (visit(c)) return true;
      }
      return false;
    };
    visit(root);
  } finally {
    currentScope = saved;
  }
  return out;
}

export function querySelectorAll(root, selector) {
  return queryWalk(root, selector, false);
}

export function querySelector(root, selector) {
  return queryWalk(root, selector, true)[0] || null;
}

export const _internal = { parseComplex, parseSelectorList, matchCompound, matchComplex, parseAnB, ruleSelectors, specificity, compareSpecificity };

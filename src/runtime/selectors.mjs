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
function startsIdentAfterDash(src, i) {
  const d = src[i + 1];
  return d !== undefined && (d === '-' || isNameStart(d) || (d === '\\' && validEscape(src, i + 1)));
}
function startsIdent(src, i) {
  const c = src[i];
  if (c === undefined) return false;
  if (isNameStart(c) || (c === '\\' && validEscape(src, i))) return true;
  return c === '-' && startsIdentAfterDash(src, i);
}
// Advance past one hex-escape trailer (CRLF or a single whitespace) at `src[j]`.
function escapeTrailer(src, j) {
  if (src[j] === '\r' && src[j + 1] === '\n') return j + 2;
  if (j < src.length && isWs(src[j])) return j + 1;
  return j;
}
// Decode one hex escape starting at `j`; the caller has already seen the first hex digit.
function decodeHexEscape(src, j) {
  let hex = '';
  while (j < src.length && hex.length < 6 && isHex(src[j])) hex += src[j++];
  const end = escapeTrailer(src, j);
  const cp = parseInt(hex, 16);
  const bad = cp === 0 || cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff);
  return { ch: bad ? '\uFFFD' : String.fromCodePoint(cp), end };
}
// Consume the escape at `src[i]` (a valid `\`): { ch, end }.
function consumeEscape(src, i) {
  const j = i + 1;
  if (j >= src.length) return { ch: '\uFFFD', end: j };
  if (isHex(src[j])) return decodeHexEscape(src, j);
  const cp = src.codePointAt(j);
  return { ch: String.fromCodePoint(cp), end: j + (cp > 0xffff ? 2 : 1) };
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
function scanBalancedStep(d, state, open, close) {
  if (state.quote) { if (d === state.quote) state.quote = null; return true; }
  if (d === '"' || d === "'") { state.quote = d; return true; }
  if (d === open) state.depth++;
  else if (d === close) { state.depth--; if (state.depth === 0) return false; }
  return true;
}
function scanBalanced(src, start, open, close) {
  const n = src.length;
  const state = { depth: 1, quote: null };
  let i = start;
  for (; i < n; i++) {
    if (src[i] === '\\') { i++; continue; }
    if (!scanBalancedStep(src[i], state, open, close)) break;
  }
  return { inner: src.slice(start, i), end: i };
}
// Split at top-level commas (outside quotes, escapes, () and []).
function splitTopLevel(src) {
  const out = [];
  const state = { depth: 0, quote: null };
  let last = 0;
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    if (c === '\\') { i++; continue; }
    if (splitStepStays(c, state)) continue;
    if (c === ',' && state.depth === 0) { out.push(src.slice(last, i)); last = i + 1; }
  }
  out.push(src.slice(last));
  return out;
}
// Update `state` for the one-character step; returns true when `c` was handled here (string
// delimiter / bracket) and the caller should skip the comma check for it.
function splitStepStays(c, state) {
  if (state.quote) { if (c === state.quote) state.quote = null; return true; }
  if (c === '"' || c === "'") { state.quote = c; return true; }
  if (c === '(' || c === '[') { state.depth++; return true; }
  if (c === ')' || c === ']') { state.depth--; return true; }
  return false;
}

// ── attribute selectors ─────────────────────────────────────────────────────
const ATTR_OP = { '~': 'includes', '|': 'dash', '^': 'prefix', $: 'suffix', '*': 'substr' };
// CSS Syntax preprocessing of selector source (miaskiewicz/turbo-dom#91; mirrors the Rust
// rtdom `selector_source`): comments (outside strings / escapes) are removed — between two
// name characters a comment still separates tokens, so it becomes a space inside `[…]` and
// the never-valid `!` elsewhere (`div/**/p` throws, `[title=a/**/i]` is `[title=a i]`); an
// unterminated comment runs to the end. `closeAtEof` (the DOM selector APIs): an
// unterminated string and every open `[` / `(` are closed, as CSS does at EOF.
const nameLike = (c) => c !== undefined && (c >= '\x80' || c === '-' || c === '_' || c === '\\' || isLetter(c) || isDigit(c));
function needsNormalize(src, closeAtEof) {
  return src.indexOf('/*') !== -1 || (closeAtEof && /[[("'\\]/.test(src));
}
// The index just past the comment opening at `i` (the input's end when unterminated).
function commentEnd(src, i) {
  const end = src.indexOf('*/', i + 2);
  return end === -1 ? src.length : end + 2;
}
// What a comment between `out` and `next` leaves behind: a separator between two name
// characters (a space inside `[…]`, the never-valid `!` elsewhere), else nothing.
function commentSeparator(out, next, open) {
  const prev = out[out.length - 1];
  if ((nameLike(prev) || prev === '#') && nameLike(next)) return open[open.length - 1] === ']' ? ' ' : '!';
  return '';
}
function trackBracket(c, open) {
  if (c === '[') open.push(']');
  else if (c === '(') open.push(')');
  else if (c === open[open.length - 1]) open.pop();
}
// Close an unterminated string and every open `[` / `(` at EOF.
function closeAtEnd(out, quote, open) {
  if (quote) out += quote;
  while (open.length) out += open.pop();
  return out;
}
// A `\` at `i`: emit it and its next char (and whatever it consumed from `state`).
// Returns the next source index, or -1 when scanning should stop (final stray `\` at EOF).
function normalizeStepBackslash(src, i, state, closeAtEof) {
  // an escape at the end: U+FFFD in a name, nothing in a string (closers stay unescaped)
  if (i + 1 === src.length && closeAtEof) { if (!state.quote) state.out += '\uFFFD'; return -1; }
  state.out += src.slice(i, i + 2);
  return i + 2;
}
function normalizeStepComment(src, i, state) {
  const end = commentEnd(src, i);
  state.out += commentSeparator(state.out, src[end], state.open);
  return end;
}
// Return the next index after processing src[i].
function normalizeStep(src, i, state, closeAtEof) {
  const c = src[i];
  if (c === '\\') return normalizeStepBackslash(src, i, state, closeAtEof);
  if (state.quote) { if (c === state.quote) state.quote = null; state.out += c; return i + 1; }
  if (c === '/' && src[i + 1] === '*') return normalizeStepComment(src, i, state);
  if (c === '"' || c === "'") state.quote = c;
  else trackBracket(c, state.open);
  state.out += c;
  return i + 1;
}
function normalizeSource(src, closeAtEof) {
  if (!needsNormalize(src, closeAtEof)) return src;
  const state = { out: '', quote: null, open: [] };
  for (let i = 0; i < src.length;) {
    const next = normalizeStep(src, i, state, closeAtEof);
    if (next < 0) break;
    i = next;
  }
  return closeAtEof ? closeAtEnd(state.out, state.quote, state.open) : state.out;
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
function skipWsAt(s, i) {
  while (i < s.length && isWs(s[i])) i++;
  return i;
}
function skipDigits(s, i) {
  while (i < s.length && isDigit(s[i])) i++;
  return i;
}
// The attribute value at `i` (a string or an identifier): { value, end }.
function scanAttrValue(s, i, src) {
  if (s[i] === '"' || s[i] === "'") return scanString(s, i);
  if (startsIdent(s, i)) return scanIdent(s, i);
  throw syntax(src, 'attribute value');
}
// The operator at `i`: { op, end }.
function scanAttrOp(s, i, src) {
  if (ATTR_OP[s[i]] !== undefined && s[i + 1] === '=') return { op: ATTR_OP[s[i]], end: i + 2 };
  if (s[i] === '=') return { op: 'equals', end: i + 1 };
  throw syntax(src, 'attribute operator');
}
// The optional modifier at `i` (only `i`), then the end of the selector.
function parseAttrModifier(attr, s, i, src) {
  if (i >= s.length) return;
  const m = s[i];
  if (m === 'i' || m === 'I') attr.ci = true;
  else throw syntax(src, 'attribute modifier');
  if (skipWsAt(s, i + 1) < s.length) throw syntax(src, 'attribute selector');
}
function parseAttrNsPrefix(s, i) {
  if (s[i] === '*' && s[i + 1] === '|') return { ns: 'any', end: i + 2 };
  if (s[i] === '|' && s[i + 1] !== '=') return { ns: null, end: i + 1 };
  return { ns: null, end: i };
}
function parseAttr(raw, src) {
  const s = raw;
  const prefix = parseAttrNsPrefix(s, skipWsAt(s, 0));
  let i = prefix.end;
  if (!startsIdent(s, i)) throw syntax(src, 'attribute name');
  const nameTok = scanIdent(s, i);
  i = nameTok.end;
  if (s[i] === '|' && s[i + 1] !== '=') throw syntax(src, 'undeclared namespace prefix');
  const attr = { name: nameTok.value, match: { op: 'present' } };
  if (prefix.ns) attr.ns = prefix.ns;
  Object.defineProperty(attr, 'lname', { value: nameTok.value.toLowerCase() });
  i = skipWsAt(s, i);
  if (i === s.length) return attr;
  const op = scanAttrOp(s, i, src);
  const v = scanAttrValue(s, skipWsAt(s, op.end), src);
  attr.match = { op: op.op, value: v.value };
  parseAttrModifier(attr, s, skipWsAt(s, v.end), src);
  return attr;
}

// ── pseudo-classes and pseudo-elements Chrome 156 accepts ───────────────────
// Implemented, argument-less.
const PLAIN = new Set([
  'first-child', 'last-child', 'only-child', 'first-of-type', 'last-of-type', 'only-of-type',
  'empty', 'root', 'scope', 'checked', 'disabled', 'enabled', 'required', 'optional',
  'read-only', 'read-write', 'link', 'any-link', 'defined',
]);
// Valid and never matching (a static document's resting state; the npm runtime has no
// fullscreen API, so the fullscreen pseudo-classes never match — turbo-dom#58).
const NEVER = new Set([
  'active', 'autofill', '-webkit-autofill', 'buffering', 'current', 'default', 'focus',
  'focus-visible', 'focus-within', 'fullscreen', '-webkit-full-screen', '-webkit-full-screen-ancestor', 'future', 'host',
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
// The scrollbar pseudo-elements and the pseudo-classes that may follow them (scrollbar
// states and :hover / :active, no focus).
const SCROLLBAR = new Set([
  '-webkit-resizer', '-webkit-scrollbar', '-webkit-scrollbar-button', '-webkit-scrollbar-corner',
  '-webkit-scrollbar-thumb', '-webkit-scrollbar-track', '-webkit-scrollbar-track-piece',
]);
const AFTER_SCROLLBAR = new Set([
  'active', 'corner-present', 'decrement', 'disabled', 'double-button', 'enabled', 'end',
  'horizontal', 'hover', 'increment', 'no-button', 'single-button', 'start', 'vertical',
  'window-inactive',
]);
const USER_ACTION = new Set(['active', 'focus', 'focus-visible', 'focus-within', 'hover']);
// Pseudo-elements that are real elements: any non-structural pseudo-class may follow,
// and so may another pseudo-element (but ::part() / ::slotted()).
const ELEMENT_LIKE = new Set(['details-content', 'part', 'picker']);
// The never-matching pseudo-classes Chrome 156 rejects after an ELEMENT_LIKE one.
const NOT_AFTER_ELEMENT_LIKE = new Set([
  'buffering', 'current', 'host', 'muted', 'paused', 'playing', 'seeking', 'stalled', 'volume-locked',
]);
const ELEMENT_LIKE_PLAIN = new Set(['target-current', 'checked', 'disabled', 'enabled', 'required', 'optional', 'read-only', 'read-write', 'link', 'any-link', 'defined', 'window-inactive']);
// What each pseudo-element may be followed by (Chrome 156): the pseudo-elements that may
// follow it, and the argument-less pseudo-classes. `:is()` / `:where()` (forgiving) may
// follow any but ::column and ::slotted().
const SUB_AFTER_SLOTTED = new Set([
  'after', 'backdrop', 'before', 'checkmark', 'details-content', 'file-selector-button',
  'marker', 'picker', 'picker-icon', 'placeholder',
]);
const PLAIN_AFTER = new Map([
  ['file-selector-button', USER_ACTION],
  ['scroll-marker', new Set([...USER_ACTION, 'target-current'])],
  ['scroll-button', new Set([...USER_ACTION, 'disabled', 'enabled'])],
  ['scroll-marker-group', new Set(['hover', 'focus-within'])],
  ['selection', new Set(['window-inactive'])],
  ['search-text', new Set(['current'])],
  ['view-transition-group', new Set(['only-child'])],
  ['view-transition-image-pair', new Set(['only-child'])],
  ['view-transition-old', new Set(['only-child'])],
  ['view-transition-new', new Set(['only-child'])],
]);
// May pseudo-class `p` follow pseudo-element `pe` ({ name, arg })? (Chrome 156; mirrors
// the Rust allowed_after_pseudo_element.)
// After an ELEMENT_LIKE pseudo-element.
function allowedAfterElementLike(p) {
  if (p.arg !== null) return p.name === 'state' || p.name === 'lang' || p.name === 'dir';
  return ELEMENT_LIKE_PLAIN.has(p.name) || (NEVER.has(p.name) && !NOT_AFTER_ELEMENT_LIKE.has(p.name));
}
function allowedAfterPlain(pe, p) {
  if (SCROLLBAR.has(pe.name)) return AFTER_SCROLLBAR.has(p.name);
  if (pe.name.startsWith('-webkit-') || (pe.name === 'cue' && pe.arg === null)) return USER_ACTION.has(p.name);
  const plain = PLAIN_AFTER.get(pe.name);
  return plain !== undefined && plain.has(p.name);
}
function allowedAfter(pe, p) {
  if ((p.name === 'is' || p.name === 'where') && p.arg !== null) return pe.name !== 'column' && pe.name !== 'slotted';
  if (ELEMENT_LIKE.has(pe.name)) return allowedAfterElementLike(p);
  if (p.arg !== null) return false;
  return allowedAfterPlain(pe, p);
}
// May pseudo-element `next` follow pseudo-element `prev` (names)? (Chrome 156,
// miaskiewicz/turbo-dom#195; mirrors the Rust allowed_after_pseudo_element_chain.)
function allowedSub(prev, next) {
  if (ELEMENT_LIKE.has(prev)) return next !== 'part' && next !== 'slotted';
  if (prev === 'slotted') return SUB_AFTER_SLOTTED.has(next) || next.startsWith('view-transition');
  if (prev === 'before' || prev === 'after') return next === 'marker';
  return prev === 'column' && next === 'scroll-marker';
}

// ── An+B ────────────────────────────────────────────────────────────────────
// `<an+b>` (CSS Syntax 3 §6) → { a, b } or null. Mirrors the Rust valid_anb + parse_nth.
// Chrome 156's An+B integers (miaskiewicz/turbo-dom#98): a number token clamps to 32 bits;
// an integer glued into an ident / dimension unit (`n-123`) must fit exactly; an A or B
// outside 31 bits never matches (nthMatch).
const INT_MIN = -2147483648, INT_MAX = 2147483647;
const clampInt = (sign, digits) => {
  let v = 0;
  for (const d of digits) v = Math.min(v * 10 + (d.charCodeAt(0) - 48), INT_MAX + 2);
  return sign < 0 ? Math.max(-v, INT_MIN - 1) : Math.min(v, INT_MAX + 1);
};
const clamp32 = (v) => Math.min(Math.max(v, INT_MIN), INT_MAX);
function parseAnBNumeric(s, sign, ds) {
  let i = skipDigits(s, ds);
  const digits = s.slice(ds, i);
  if (s[i] !== 'n') {
    if (!digits || i !== s.length) return null;
    return { a: 0, b: clamp32(clampInt(sign, digits)) };
  }
  const a = digits ? clamp32(clampInt(sign, digits)) : sign;
  return parseAnBTail(s, i + 1, a);
}
function parseAnB(arg) {
  const s = trimWs(arg).toLowerCase();
  if (s === 'odd') return { a: 2, b: 1 };
  if (s === 'even') return { a: 2, b: 0 };
  const sign = s[0] === '-' ? -1 : 1;
  const start = (s[0] === '+' || s[0] === '-') ? 1 : 0;
  return parseAnBNumeric(s, sign, start);
}
// `n-123` is one ident / dimension unit: its integer (after the `n` at i - 1) is converted
// exactly and must fit.
function gluedIntegerFits(s, i) {
  if (s[i] !== '-' || i + 1 >= s.length || !isDigit(s[i + 1])) return true;
  return clampInt(-1, s.slice(i + 1, skipDigits(s, i + 1))) >= INT_MIN;
}
// The part after An's `n` (at i - 1): nothing, or a signed B.
function parseAnBTail(s, i, a) {
  if (!gluedIntegerFits(s, i)) return null;
  i = skipWsAt(s, i);
  if (i === s.length) return { a, b: 0 };
  if (s[i] !== '+' && s[i] !== '-') return null;
  const bs = s[i] === '-' ? -1 : 1;
  const bd = skipWsAt(s, i + 1);
  i = skipDigits(s, bd);
  if (i === bd) return null;
  const b = clamp32(clampInt(bs, s.slice(bd, i)));
  return skipWsAt(s, i) === s.length ? { a, b } : null;
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
  if (a < -1073741824 || a >= 1073741824 || b < -1073741824 || b >= 1073741824) return false;
  if (a === 0) return index === b;
  const d = index - b;
  return d % a === 0 && d / a >= 0;
}

// ── lexer ───────────────────────────────────────────────────────────────────
// Token kinds: ws | comma | comb (v) | type (ns, name) | id (v) | class (v) | attr |
// pseudo (name, arg) | pelem (name, arg).
// The type / universal selector at `i` (src[i] is `*`, `|` or an identifier start), with
// an optional `*|` / `|` namespace prefix: pushes its token, returns the next index.
function scanTypeAfterPipe(src, i, ns, tokens) {
  let name = null;
  if (src[i] === '*') i++;
  else if (startsIdent(src, i)) { const t = scanIdent(src, i); name = t.value; i = t.end; }
  else throw syntax(src, 'type selector after a namespace prefix');
  tokens.push({ k: 'type', ns, name: name === null ? null : name.toLowerCase() });
  return i;
}
function scanTypeToken(src, i, tokens) {
  const c = src[i];
  let name = null;
  if (c === '*') i++;
  else if (c !== '|') { const t = scanIdent(src, i); name = t.value; i = t.end; }
  if (src[i] === '|' && src[i + 1] !== '=') {
    if (name !== null) throw syntax(src, 'undeclared namespace prefix');
    return scanTypeAfterPipe(src, i + 1, c === '*' ? 'any' : 'none', tokens);
  }
  if (c === '|') throw syntax(src, `unexpected '|'`);
  tokens.push({ k: 'type', ns: null, name: name === null ? null : name.toLowerCase() });
  return i;
}
// The pseudo-class / pseudo-element at `i` (src[i] is `:`): pushes its token, returns the
// next index.
function scanPseudoToken(src, i, tokens) {
  const n = src.length;
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
  return i;
}
// The attribute selector at `i` (src[i] is `[`): pushes its token, returns the next index.
function scanAttrToken(src, i, tokens) {
  const { inner, end } = scanBalanced(src, i + 1, '[', ']');
  if (end >= src.length) throw new SyntaxError(`unterminated attribute selector: ${src.slice(i)}`);
  tokens.push({ k: 'attr', attr: parseAttr(inner, src) });
  return end + 1;
}
// The id / class selector at `i` (src[i] is `#` / `.`): pushes its token, returns the next
// index.
function scanHashOrClassToken(src, i, tokens) {
  const c = src[i];
  if (!startsIdent(src, i + 1)) throw syntax(src, `unexpected '${c}'`);
  const t = scanIdent(src, i + 1);
  tokens.push({ k: c === '#' ? 'id' : 'class', v: t.value });
  return t.end;
}
const COMBINATORS = new Set(['>', '+', '~']);
// One tokenize step: returns the next source index after consuming src[i].
function tokenizeStep(src, i, tokens) {
  const c = src[i];
  if (isWs(c)) { tokens.push({ k: 'ws' }); return skipWsAt(src, i); }
  if (c === ',') { tokens.push({ k: 'comma' }); return i + 1; }
  if (COMBINATORS.has(c)) { tokens.push({ k: 'comb', v: c }); return i + 1; }
  return tokenizeSimple(src, i, c, tokens);
}
function tokenizeSimple(src, i, c, tokens) {
  if (c === '*' || c === '|' || startsIdent(src, i)) return scanTypeToken(src, i, tokens);
  if (c === '#' || c === '.') return scanHashOrClassToken(src, i, tokens);
  if (c === '[') return scanAttrToken(src, i, tokens);
  if (c === ':') return scanPseudoToken(src, i, tokens);
  throw new SyntaxError(`unexpected '${c}' in selector`);
}
function tokenize(src) {
  const tokens = [];
  const n = src.length;
  for (let i = 0; i < n;) i = tokenizeStep(src, i, tokens);
  return tokens;
}

// ── parser ──────────────────────────────────────────────────────────────────
// A Complex is `{ first, rest: [{ combinator, compound }] }`; a compound is
// `{ tag, ns, id, classes, attrs, pseudos, pe }`: `tag` null = no type selector, '*' =
// universal, else the lowercase name; `ns` the type's namespace prefix; `pe` the pseudo-element; pseudos are `{ name, arg }` with the
// parsed argument kept in a non-enumerable `parsed`.

// Parse one pseudo-class (throws on what Chrome rejects). `opts.inHas`: inside `:has()`
// (which cannot nest); `opts.inCompound`: inside `:-webkit-any()`, whose arguments are
// compound selectors — nested `:not()` takes compounds, nested `:is()` / `:where()` drop
// complex items, `of S` items may be complex (their own nested lists may not).
function parsePseudo(tok, src, opts) {
  const { inHas, inCompound = false } = opts;
  const { name, arg } = tok;
  const p = { name, arg };
  const parsed = arg === null ? parsePlainPseudo(name, src) : parseFunctionalPseudo(name, arg, src, inHas, inCompound);
  if (parsed) Object.defineProperty(p, 'parsed', { value: parsed });
  return p;
}
// An argument-less pseudo-class: its `parsed` (null for an implemented one).
function parsePlainPseudo(name, src) {
  if (PLAIN.has(name)) return null;
  if (NEVER.has(name)) return { never: true };
  throw syntax(src, `unknown pseudo-class :${name}`);
}
const NTH = new Set(['nth-child', 'nth-last-child', 'nth-of-type', 'nth-last-of-type']);
// A nested selector-list pseudo (`:not` / `:is` / `:where` / `:-webkit-any`): its `parsed`.
function parseListPseudo(name, arg, inHas, inCompound) {
  if (name === 'not') return { list: parseList(arg, { nested: true, forgiving: false, inHas, compound: inCompound, inCompound }) };
  if (name === 'is' || name === 'where') return { list: parseList(arg, { nested: true, forgiving: true, inHas, compound: inCompound, inCompound }) };
  // unforgiving compound selectors, no :has() (a nested :is() drops items with one)
  return { list: parseList(arg, { nested: true, forgiving: false, inHas: true, compound: true, inCompound: true }) };
}
const LIST_PSEUDO = new Set(['not', 'is', 'where', '-webkit-any']);
// A functional pseudo-class: its `parsed`.
function parseFunctionalPseudoValue(name, arg, src) {
  if (name === 'lang' || name === 'dir') return { v: identArg(name, arg, src).toLowerCase() };
  if (name === 'state') return { v: identArg(name, arg, src) };
  throw syntax(src, `unknown pseudo-class :${name}()`);
}
function parseFunctionalPseudo(name, arg, src, inHas, inCompound) {
  if (PLAIN.has(name)) throw syntax(src, `:${name}() takes no argument`);
  if (NEVER_FUNCTIONAL.has(name)) return { never: true };
  if (LIST_PSEUDO.has(name)) return parseListPseudo(name, arg, inHas, inCompound);
  if (name === 'has') {
    if (inHas) throw syntax(src, ':has() inside :has()');
    return { rel: parseRelative(arg, src) };
  }
  if (NTH.has(name)) return parseNthPseudo(name, arg, src, inHas, inCompound);
  return parseFunctionalPseudoValue(name, arg, src);
}
// The one identifier argument of `:name()` (throws otherwise).
function identArg(name, arg, src) {
  const v = identValue(trimWs(arg));
  if (v === null) throw syntax(src, `:${name}() takes one identifier`);
  return v;
}
function parseNthPseudo(name, arg, src, inHas, inCompound) {
  const at = name.endsWith('child') ? nthOfPosition(arg) : -1;
  const anb = parseAnB(at === -1 ? arg : arg.slice(0, at));
  if (anb === null) throw syntax(src, `:${name}() argument`);
  return { a: anb.a, b: anb.b, of: at === -1 ? null : parseList(arg.slice(at + 2), { nested: true, forgiving: false, inHas, inCompound }) };
}

// A view-transition pseudo-element argument (<pt-name-and-class-selector>): `*` or a name,
// then `.class` parts (no space after the dot), or only class parts; whitespace may
// separate the parts (Chrome 156).
function isPtNameAndClass(arg) {
  const s = trimWs(arg);
  let i = 0;
  if (s[0] === '*') i = 1;
  else if (startsIdent(s, 0)) i = scanIdent(s, 0).end;
  let parts = i;
  for (;;) {
    while (isWs(s[i])) i++;
    if (s[i] !== '.') break;
    if (!startsIdent(s, i + 1)) return false;
    i = scanIdent(s, i + 1).end;
    parts++;
  }
  return parts > 0 && i === s.length;
}

// The (decoded) identifiers of a whitespace-separated list, or null when an item is not an
// identifier (or the list is empty).
function identList(arg) {
  const s = trimWs(arg);
  const out = [];
  let i = 0;
  while (i < s.length) {
    if (!startsIdent(s, i)) return null;
    const t = scanIdent(s, i);
    out.push(t.value);
    i = t.end;
    if (i < s.length && !isWs(s[i])) return null;
    while (isWs(s[i])) i++;
  }
  return out.length ? out : null;
}
const SCROLL_BUTTON_DIRECTIONS = new Set([
  'block-end', 'block-start', 'down', 'inline-end', 'inline-start', 'left', 'right', 'up',
]);
// Is `arg` a valid argument of the functional pseudo-element `name` (Chrome 156)?
function validPseudoElementArg(name, arg) {
  if (name.startsWith('view-transition-')) return isPtNameAndClass(arg);
  const idents = identList(arg);
  const one = idents !== null && idents.length === 1 ? idents[0].toLowerCase() : null;
  if (name === 'part') return idents !== null;
  if (name === 'highlight') return one !== null;
  if (name === 'picker') return one === 'select';
  if (name === 'scroll-button') return trimWs(arg) === '*' || SCROLL_BUTTON_DIRECTIONS.has(one);
  return true; // ::cue() and ::slotted() take selectors (parsed below)
}

const PE_COMPOUND_OPTS = { nested: true, forgiving: false, inHas: true, compound: true, inCompound: true };
// Validate the argument of a functional pseudo-element and stash any ::slotted() parse on `pe`.
function pseudoElementArg(pe, src) {
  const { name, arg } = pe;
  if (!validPseudoElementArg(name, arg)) throw syntax(src, `::${name}() argument`);
  // ::cue(): compound selectors, nested lists restricted as in :-webkit-any() (Chrome 156)
  if (name === 'cue') parseList(arg, PE_COMPOUND_OPTS);
  if (name === 'slotted') {
    // one compound selector, nested lists restricted as in :-webkit-any() (Chrome 156)
    const list = parseList(arg, PE_COMPOUND_OPTS);
    if (list.length !== 1) throw syntax(src, '::slotted() takes one compound selector');
    // kept for specificity (non-enumerable, like a pseudo-class's `parsed`)
    Object.defineProperty(pe, 'parsed', { value: list });
  }
}
// Parse one pseudo-element (throws on unknown ones): { name, arg, slotted? }.
function parsePseudoElement(tok, src) {
  const { name, arg } = tok;
  if (tok.legacy && arg !== null) throw syntax(src, `:${name}() takes no argument`);
  const known = arg === null ? PSEUDO_ELEMENTS.has(name) : FUNCTIONAL_PSEUDO_ELEMENTS.has(name);
  if (!known && !name.startsWith('-webkit-')) throw syntax(src, `unknown pseudo-element ::${name}`);
  const pe = { name, arg };
  if (arg !== null) pseudoElementArg(pe, src);
  return pe;
}

// Parse tokens[lo..hi) (one complex selector, commas already split out). Throws on an empty
// item, a dangling combinator, a type selector after another simple selector, and a
// pseudo-element anywhere but at the end of the last compound.
// The last pseudo-element of `pe`'s chain (`pe` itself without sub-pseudo-elements).
function lastPseudoElement(pe) {
  return pe.chain ? pe.chain[pe.chain.length - 1] : pe;
}
// A pseudo-element token `p` of `compound`: the compound's pseudo-element, or a
// sub-pseudo-element (`::slotted(li)::marker`: valid, never matched here) of it.
function addPseudoElement(compound, p, src, opts) {
  if (opts.nested) throw syntax(src, 'pseudo-element inside a pseudo-class argument');
  const next = parsePseudoElement(p, src);
  const pe = compound.pe;
  if (pe === null) { compound.pe = next; return; }
  const last = lastPseudoElement(pe);
  if (!allowedSub(last.name, next.name)) throw syntax(src, `::${next.name} after ::${last.name}`);
  if (!pe.chain) Object.defineProperty(pe, 'chain', { value: [] });
  pe.chain.push(next);
}
// A simple-selector token `p` after the compound's pseudo-element: only the pseudo-classes
// Chrome allows there (the selector never matches an element either way) and further
// pseudo-elements.
function addAfterPseudoElement(compound, p, src, opts) {
  if (p.k === 'pelem') { addPseudoElement(compound, p, src, opts); return; }
  if (p.k !== 'pseudo') throw syntax(src, 'simple selector after a pseudo-element');
  const last = lastPseudoElement(compound.pe);
  if (!allowedAfter(last, p)) throw syntax(src, `:${p.name} after ::${last.name}`);
  compound.pseudos.push({ name: p.name, arg: p.arg });
}
// Add simple-selector token `p` (the `parts`-th of its compound) to `compound`.
function addSimple(compound, p, parts, src, opts) {
  if (compound.pe !== null) { addAfterPseudoElement(compound, p, src, opts); return; }
  if (p.k === 'type') {
    if (parts) throw syntax(src, 'type selector after another simple selector');
    compound.tag = p.name === null ? '*' : p.name;
    compound.ns = p.ns;
  } else if (p.k === 'id') compound.id = p.v;
  else if (p.k === 'class') compound.classes.push(p.v);
  else if (p.k === 'attr') compound.attrs.push(p.attr);
  else if (p.k === 'pelem') addPseudoElement(compound, p, src, opts);
  else compound.pseudos.push(parsePseudo(p, src, opts));
}
// Fill `compound` from tokens[pos..end) up to the next whitespace / combinator; returns
// the index of that token (or `end`).
function parseCompound(tokens, pos, end, src, opts, compound) {
  const start = pos;
  for (; pos < end; pos++) {
    const p = tokens[pos];
    if (p.k === 'ws' || p.k === 'comb') break;
    addSimple(compound, p, pos - start, src, opts);
  }
  return pos;
}
function firstNonWs(tokens, lo, hi) {
  while (lo < hi && tokens[lo].k === 'ws') lo++;
  return lo;
}
function endNonWs(tokens, lo, hi) {
  while (hi > lo && tokens[hi - 1].k === 'ws') hi--;
  return hi;
}
// Whitespace after a compound is the descendant combinator unless another follows.
function afterWhitespace(combinator, first) {
  return combinator === null && first !== null ? ' ' : combinator;
}
// A pending combinator other than the descendant one has no compound after it.
function isDangling(combinator) {
  return combinator !== null && combinator !== ' ';
}
// Consume one compound at `state.pos`, appending it to `state` (first / rest / sawPe).
function parseComplexCompound(tokens, end, src, opts, state) {
  if (state.sawPe) throw syntax(src, 'combinator after a pseudo-element');
  // one object shape for every compound (monomorphic matching): `ns` is null / 'any' /
  // 'none', `pe` the pseudo-element or null
  const compound = { tag: null, ns: null, id: null, classes: [], attrs: [], pseudos: [], pe: null };
  state.pos = parseCompound(tokens, state.pos, end, src, opts, compound);
  if (compound.pe !== null) state.sawPe = true;
  if (state.first === null) state.first = compound;
  else state.rest.push({ combinator: state.combinator === null ? ' ' : state.combinator, compound });
  state.combinator = null;
}
function parseComplexStep(tokens, end, src, opts, state) {
  const t = tokens[state.pos];
  if (t.k === 'ws') { state.pos++; state.combinator = afterWhitespace(state.combinator, state.first); return; }
  if (t.k === 'comb') {
    if (state.first === null || isDangling(state.combinator)) throw syntax(src, 'dangling combinator');
    state.combinator = t.v; state.pos++; return;
  }
  parseComplexCompound(tokens, end, src, opts, state);
}
function parseComplexTokens(tokens, lo, hi, src, opts) {
  const start = firstNonWs(tokens, lo, hi);
  const end = endNonWs(tokens, start, hi);
  if (start >= end) throw syntax(src, 'empty selector');
  const state = { first: null, rest: [], combinator: null, pos: start, sawPe: false };
  while (state.pos < end) parseComplexStep(tokens, end, src, opts, state);
  if (isDangling(state.combinator)) throw syntax(src, 'dangling combinator');
  if (opts.compound && state.rest.length) throw syntax(src, 'combinator inside :-webkit-any()');
  return { first: state.first, rest: state.rest };
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
  const parsed = parseSource(selector, true);
  if (__selectorCache.size > 10000) __selectorCache.clear();
  __selectorCache.set(selector, parsed);
  return parsed;
}
// `closeAtEof`: a DOM-API selector string (a stylesheet prelude never ends inside a block).
function parseSource(selector, closeAtEof) {
  // CSS preprocessing: NUL becomes U+FFFD (a name code point); comments go
  const src = normalizeSource(String(selector).replace(/\0/g, '\uFFFD'), closeAtEof);
  return parseList(src, { nested: false, forgiving: false, inHas: false });
}

// The items of a style rule's selector list (top-level comma split), or [] when the list is
// invalid — a browser drops the whole rule (cascade.mjs; mirrors rtdom::cascade).
export function ruleSelectors(prelude) {
  try { parseSource(prelude, false); } catch { return []; }
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
function firstLegendChild(fieldset) {
  const kids = rawChildren(fieldset);
  for (let i = 0; i < kids.length; i++) if (kids[i].nodeType === 1 && kids[i].localName === 'legend') return kids[i];
  return null;
}
function disabledByFieldset(el) {
  let child = el;
  for (let a = el.parentNode; a && a.nodeType === 1; child = a, a = a.parentNode) {
    if (a.localName !== 'fieldset' || !isHtml(a) || !boolAttr(a, 'disabled', 'disabled')) continue;
    if (firstLegendChild(a) !== child) return true;
  }
  return false;
}
function actuallyDisabled(el) {
  return boolAttr(el, 'disabled', 'disabled') || disabledByFieldset(el);
}
// :disabled / :enabled: undefined when neither applies, else whether `el` is disabled.
function disabledState(el) {
  if (!isHtml(el)) return undefined;
  switch (el.localName) {
    case 'button': case 'input': case 'select': case 'textarea': case 'fieldset':
      return actuallyDisabled(el);
    case 'optgroup':
      return boolAttr(el, 'disabled', 'disabled');
    case 'option':
      return optionDisabled(el);
    default:
      return undefined;
  }
}
function isElementNamed(n, localName) {
  return !!n && n.nodeType === 1 && n.localName === localName;
}
// An option is disabled by its own attribute, its optgroup's or a disabled select.
function optionDisabled(el) {
  if (el.hasAttribute('disabled')) return true;
  let p = el.parentNode;
  if (isElementNamed(p, 'optgroup')) {
    if (p.hasAttribute('disabled')) return true;
    p = p.parentNode;
  }
  return isElementNamed(p, 'select') && actuallyDisabled(p);
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
// A textarea's value: its child text content (direct Text children only; turbo-dom#106).
function childText(el) {
  let text = '';
  const kids = rawChildren(el);
  for (let i = 0; i < kids.length; i++) if (kids[i].nodeType === 3) text += kids[i].data;
  return text;
}
function isAutoDirInput(el) {
  const t = inputType(el);
  return t === null || AUTO_DIR_INPUT_TYPES.has(t) || !KNOWN_INPUT_TYPES.has(t);
}
function autoDirValueRtl(value) {
  const r = strongRtl(value);
  return r !== null ? r : value !== '' ? false : null;
}
const DIR_SKIP = new Set(['bdi', 'script', 'style', 'textarea']);
function walkDirRtl(node) {
  const kids = rawChildren(node);
  for (let i = 0; i < kids.length; i++) {
    const c = kids[i];
    if (c.nodeType === 3) { const r = strongRtl(c.data); if (r !== null) return r; continue; }
    if (c.nodeType !== 1) continue;
    if (isHtml(c) && (DIR_SKIP.has(c.localName) || dirState(c) !== null)) continue;
    const r = walkDirRtl(c);
    if (r !== null) return r;
  }
  return null;
}
function autoRtl(el) {
  const ln = el.localName;
  if (ln === 'textarea') return autoDirValueRtl(childText(el));
  if (ln === 'input' && isAutoDirInput(el)) return autoDirValueRtl(el.getAttribute('value') || '');
  return walkDirRtl(el);
}
// HTML directionality: true = rtl. Mirrors the Rust query_cascade::is_rtl.
function isRtlImplicitDir(n) {
  if (n.localName === 'bdi') return autoRtl(n) === true;
  if (n.localName === 'input' && inputType(n) === 'tel') return false;
  return undefined;
}
function isRtlStep(n) {
  if (!isHtml(n)) return undefined;
  const d = dirState(n);
  if (d === 'ltr') return false;
  if (d === 'rtl') return true;
  if (d === 'auto') return autoRtl(n) === true;
  return d === null ? isRtlImplicitDir(n) : undefined;
}
function dirAncestor(n) {
  const p = n.parentNode;
  if (p && p.__isShadowRoot) return p.host;
  return p && p.nodeType === 1 ? p : null;
}
function isRtl(el) {
  for (let n = el; n; n = dirAncestor(n)) {
    const r = isRtlStep(n);
    if (r !== undefined) return r;
  }
  return false;
}

// ── matching ────────────────────────────────────────────────────────────────
const MATCH_VALUE_OPS = {
  equals: (v, t) => v === t,
  prefix: (v, t) => t !== '' && v.startsWith(t),
  suffix: (v, t) => t !== '' && v.endsWith(t),
  substr: (v, t) => t !== '' && v.includes(t),
  includes: (v, t) => t !== '' && !/\s/.test(t) && v.split(/[ \t\n\r\f]+/).includes(t),
  dash: (v, t) => v === t || v.startsWith(t + '-'),
};
function matchValue(m, got, ci) {
  if (m.op === 'present') return true;
  const v = ci ? got.toLowerCase() : got;
  const t = ci ? m.value.toLowerCase() : m.value;
  return (MATCH_VALUE_OPS[m.op] || MATCH_VALUE_OPS.dash)(v, t);
}
function matchAttrAnyNs(el, name, match, ci) {
  const at = attrList(el);
  for (let i = 0; i < at.length; i++) if (at[i].name === name && matchValue(match, at[i].value, ci)) return true;
  return false;
}
const IMPLICIT_CI_TYPE = new Set(['input', 'button', 'ol']);
function matchAttr(el, a) {
  const html = !el.__ns;
  const name = html ? a.lname : a.name;
  if (a.ns === 'any') return matchAttrAnyNs(el, name, a.match, a.ci === true);
  const raw = html ? el.getAttribute(name) : plainAttr(el, name);
  if (raw === null) return false;
  if (a.match.op === 'present') return true;
  return matchValue(a.match, raw, a.ci === true || (html && name === 'type' && IMPLICIT_CI_TYPE.has(el.localName)));
}

// The element `:scope` matches during the current match (null = stylesheet: `:root`).
let currentScope = null;

// matchPseudo dispatches by family: tree-structural here, then the selector-argument and
// language ones (matchLogicalPseudo), then element state (matchStatePseudo).
function matchNthOfType(el, p) {
  const [i, n] = position(el, true);
  return nthMatch(p.parsed.a, p.parsed.b, p.name === 'nth-last-of-type' ? n - i + 1 : i);
}
const PSEUDO_TREE = {
  'first-child': (el) => previousElement(el) === null,
  'last-child': (el) => nextElement(el) === null,
  'only-child': (el) => previousElement(el) === null && nextElement(el) === null,
  'first-of-type': (el) => position(el, true)[0] === 1,
  'last-of-type': (el) => { const [i, n] = position(el, true); return i === n; },
  'only-of-type': (el) => position(el, true)[1] === 1,
  'scope': (el) => currentScope === null ? isRootElement(el) : el === currentScope,
};
function matchPseudo(el, p) {
  const tree = PSEUDO_TREE[p.name];
  if (tree) return tree(el);
  if (p.name === 'nth-child') return matchNthChild(el, p.parsed, false);
  if (p.name === 'nth-last-child') return matchNthChild(el, p.parsed, true);
  if (p.name === 'nth-of-type' || p.name === 'nth-last-of-type') return matchNthOfType(el, p);
  return matchLogicalPseudo(el, p);
}
function isRootElement(el) {
  return !!el.parentNode && el.parentNode.nodeType === 9;
}
function matchNthChild(el, parsed, last) {
  if (parsed.of) {
    if (!parsed.of.some((cx) => matchComplex(el, cx))) return false;
    return nthMatch(parsed.a, parsed.b, positionOf(el, parsed.of, last));
  }
  const [i, n] = position(el, false);
  return nthMatch(parsed.a, parsed.b, last ? n - i + 1 : i);
}
function matchListPseudo(el, p) {
  const parsed = p.parsed;
  if (p.name === 'not') return !parsed.list.some((cx) => matchComplex(el, cx));
  if (p.name === 'has') return parsed.rel.some((rel) => hasMatch(el, rel));
  return parsed.list.some((cx) => matchComplex(el, cx));
}
const LIST_MATCH = new Set(['not', 'is', 'where', '-webkit-any', 'has']);
function matchLogicalPseudo(el, p) {
  if (LIST_MATCH.has(p.name)) return matchListPseudo(el, p);
  if (p.name === 'link' || p.name === 'any-link') return isLink(el);
  if (p.name === 'defined') return isDefinedElement(el);
  if (p.name === 'lang') return langMatches(el, p.parsed.v);
  if (p.name === 'dir') return (p.parsed.v === 'rtl' || p.parsed.v === 'ltr') && isRtl(el) === (p.parsed.v === 'rtl');
  return matchStatePseudo(el, p);
}
// element state; form state reads the live PROPERTY first (what React sets), then the attribute
const STATE_PSEUDO = {
  'empty': (el) => isEmpty(el),
  'root': (el) => isRootElement(el),
  'checked': (el) => isChecked(el),
  'disabled': (el) => disabledState(el) === true,
  'enabled': (el) => disabledState(el) === false,
  'required': (el) => boolAttr(el, 'required', 'required'),
  'optional': (el) => isOptional(el),
  'read-only': (el) => isHtml(el) && !readWrite(el),
  'read-write': (el) => isHtml(el) && readWrite(el),
};
function matchStatePseudo(el, p) {
  const fn = STATE_PSEUDO[p.name];
  return fn ? fn(el) : false;
}
function isChecked(el) {
  if (el.localName === 'option') return !!el.selected;
  return el.localName === 'input' && (inputType(el) === 'checkbox' || inputType(el) === 'radio') && boolAttr(el, 'checked', 'checked');
}
function isOptional(el) {
  return (el.localName === 'input' || el.localName === 'select' || el.localName === 'textarea') && !boolAttr(el, 'required', 'required');
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

// The type selector `tag` (null = none, '*' = universal) against `el`.
function tagMatches(el, tag) {
  if (tag === null || tag === '*') return true;
  const ln = el.localName;
  // type selectors compare ASCII case-insensitively (as the Rust engine); HTML local
  // names are already lowercase, so only foreign elements pay for the fold
  return ln === tag || (!!el.__ns && ln.toLowerCase() === tag);
}
// classes: read the class attribute ONCE and test membership without allocating
function classesMatch(el, classes) {
  const cn = plainAttr(el, 'class');
  if (!cn) return false;
  for (let i = 0; i < classes.length; i++) if (!hasClass(cn, classes[i])) return false;
  return true;
}

function matchAttrsAndPseudos(el, compound) {
  for (const a of compound.attrs) if (!matchAttr(el, a)) return false;
  for (const p of compound.pseudos) if (!matchPseudo(el, p)) return false;
  return true;
}
function matchCompoundSimple(el, compound) {
  if (compound.ns === 'none' && el.namespaceURI !== null) return false;
  if (!tagMatches(el, compound.tag)) return false;
  if (compound.id !== null && plainAttr(el, 'id') !== compound.id) return false;
  return !(compound.classes.length && !classesMatch(el, compound.classes));
}
function matchCompound(el, compound) {
  if (!el || el.nodeType !== 1 || compound.pe) return false;
  return matchCompoundSimple(el, compound) && matchAttrsAndPseudos(el, compound);
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
// (Every caller passes `scope`: null outside `:has()`.)
function matchChainCombinator(el, cx, k, scope, comb) {
  if (comb === '>') {
    const p = el.parentNode;
    return !!p && p.nodeType === 1 && matchChain(p, cx, k, scope);
  }
  if (comb === '+') {
    const prev = previousElement(el);
    return !!prev && matchChain(prev, cx, k, scope);
  }
  if (comb === ' ') return someAncestorMatches(el, cx, k, scope);
  return somePrecedingSiblingMatches(el, cx, k, scope);
}
function matchChain(el, cx, k, scope) {
  if (!matchCompound(el, compoundAt(cx, k))) return false;
  if (k === 0) return scope === null || relatesToScope(el, scope.comb, scope.node);
  return matchChainCombinator(el, cx, k - 1, scope, cx.rest[k - 1].combinator);
}
// The descendant combinator: compounds 0..=k end at SOME element ancestor of `el`.
function someAncestorMatches(el, cx, k, scope) {
  for (let anc = el.parentNode; anc && anc.nodeType === 1; anc = anc.parentNode) {
    if (matchChain(anc, cx, k, scope)) return true;
  }
  return false;
}
// The general-sibling combinator: compounds 0..=k end at SOME preceding element sibling.
function somePrecedingSiblingMatches(el, cx, k, scope) {
  for (let prev = previousElement(el); prev; prev = previousElement(prev)) {
    if (matchChain(prev, cx, k, scope)) return true;
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

// Does the `::slotted(S)` rule item `selector` (one complex selector of a shadow tree's
// stylesheet) apply to `el`, which is assigned to `slot`? `el` matches S and `slot` the
// rest of the selector (`slot[name=a]::slotted(S)`, `div ::slotted(S)`). A sub-pseudo-
// element (`::slotted(S)::before`) targets generated content, never `el` itself.
export function matchesSlotted(el, slot, selector) {
  const cx = parseSelectorList(selector)[0];
  const subject = cx.rest.length ? cx.rest[cx.rest.length - 1].compound : cx.first;
  const pe = subject.pe;
  if (pe === null || pe.name !== 'slotted' || pe.chain) return false;
  if (!pe.parsed.some((arg) => matchComplex(el, arg))) return false;
  subject.pe = null; // match the slot against the originating compound
  try {
    return matchComplex(slot, cx);
  } finally {
    subject.pe = pe;
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

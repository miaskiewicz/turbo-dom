// Compact CSS selector engine. Correctness-first, right-to-left matching.
// Supports: type, *, #id, .class, [attr], [attr op val] (= ^= $= *= ~= |=),
// combinators (descendant ' ', child '>', adjacent '+', sibling '~'),
// comma selector lists, :not()/:is()/:where() (selector lists), the relational
// :has(), :first-child, :last-child, :only-child, :empty, :root. Enough for React
// Testing Library usage.

// ── Tokenizer + recursive-descent parser ─────────────────────────────────────
// The selector string is first turned into a flat token stream by `tokenize`,
// which centrally handles the two things an ad-hoc character scanner keeps
// getting wrong: quoted strings and balanced `[]`/`()` runs. The grammar layer
// (`parseCompound`/`parseComplexTokens`) then only ever sees whole tokens, so it
// never has to re-discover where an attribute or pseudo argument ends. The AST it
// emits — a Complex `{ first, rest: [{ combinator, compound }] }` (mirroring the
// Rust port's "parse, don't validate" shape: no parallel head/tail arrays that can
// desync) with the same compound/attr/pseudo object shapes — is exactly what the
// matcher below consumes.
//
// Token kinds:
//   { k:'comma' }                       a top-level ','
//   { k:'ws' }                          a run of whitespace (a candidate descendant combinator)
//   { k:'comb', v:'>'|'+'|'~' }         an explicit combinator
//   { k:'star' }                        '*'
//   { k:'type', v }                     a type/element name
//   { k:'id',    v }                    '#name'
//   { k:'class', v }                    '.name'
//   { k:'attr', name, match }           '[ … ]' (match = { op:'present' } | { op, value })
//   { k:'pseudo', name, arg }           ':name' / ':name( arg )' (arg null ⇒ no parens)

const NAME_CHAR = /[a-zA-Z0-9_-]/;
function isTypeStart(c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
function isWs(c) {
  return c === ' ' || c === '\t' || c === '\n' || c === '\r' || c === '\f';
}

// Split a raw `[ … ]` interior into `{ name, match }`, where `match` FOLDS the
// operator and value so an illegal state is unrepresentable: a presence test is
// `{ op: 'present' }` (NO `value` key at all), and a valued test is
// `{ op, value }` with op ∈ equals|includes|dash|prefix|suffix|substr (mirroring
// the Rust `AttrMatch` variants). The first '=' (if any) splits name from value;
// an operator char (~ | ^ $ *) immediately before it selects the operator. A
// surrounding matching quote pair on the value is stripped. No '=' ⇒ presence.
const ATTR_OP = { '~': 'includes', '|': 'dash', '^': 'prefix', $: 'suffix', '*': 'substr' };
function parseAttr(inner) {
  const eq = inner.indexOf('=');
  if (eq === -1) return { name: inner.trim(), match: { op: 'present' } };
  let name = inner.slice(0, eq);
  let op = 'equals';
  const last = name[name.length - 1];
  if (ATTR_OP[last] !== undefined) {
    op = ATTR_OP[last];
    name = name.slice(0, -1);
  }
  let value = inner.slice(eq + 1).trim();
  if (
    value.length >= 2 &&
    ((value[0] === '"' && value[value.length - 1] === '"') ||
      (value[0] === "'" && value[value.length - 1] === "'"))
  ) {
    value = value.slice(1, -1);
  }
  return { name: name.trim(), match: { op, value } };
}

// Scan from `start` (just past an opening `open`) to its matching `close`,
// honoring quotes and nesting. Returns { inner, end } where `end` is the index of
// the closing delimiter (or src.length if unterminated). One shared scanner for
// both `[...]` and `(...)`, mirroring the Rust `scan_balanced` — a `]`/`)` inside a
// quoted value never terminates early, and `:not(:nth-child(2))` nests correctly.
function scanBalanced(src, start, open, close) {
  const n = src.length;
  let depth = 1, i = start, quote = null;
  for (; i < n; i++) {
    const d = src[i];
    if (quote) { if (d === quote) quote = null; continue; }
    if (d === '"' || d === "'") quote = d;
    else if (d === open) depth++;
    else if (d === close) { depth--; if (depth === 0) break; }
  }
  return { inner: src.slice(start, i), end: i };
}

function tokenize(src) {
  const tokens = [];
  const n = src.length;
  let i = 0;
  while (i < n) {
    const c = src[i];
    if (isWs(c)) {
      i++;
      while (i < n && isWs(src[i])) i++;
      tokens.push({ k: 'ws' });
      continue;
    }
    if (c === ',') { tokens.push({ k: 'comma' }); i++; continue; }
    if (c === '>' || c === '+' || c === '~') { tokens.push({ k: 'comb', v: c }); i++; continue; }
    if (c === '*') { tokens.push({ k: 'star' }); i++; continue; }
    if (c === '#') {
      let j = i + 1;
      while (j < n && NAME_CHAR.test(src[j])) j++;
      tokens.push({ k: 'id', v: src.slice(i + 1, j) });
      i = j; continue;
    }
    if (c === '.') {
      let j = i + 1;
      while (j < n && NAME_CHAR.test(src[j])) j++;
      tokens.push({ k: 'class', v: src.slice(i + 1, j) });
      i = j; continue;
    }
    if (c === '[') {
      const { inner, end } = scanBalanced(src, i + 1, '[', ']');
      if (end >= n) throw new SyntaxError(`unterminated attribute selector: ${src.slice(i)}`);
      tokens.push({ k: 'attr', ...parseAttr(inner) });
      i = end + 1; continue;
    }
    if (c === ':') {
      let j = i + 1;
      while (j < n && (isTypeStart(src[j]) || src[j] === '-')) j++;
      const name = src.slice(i + 1, j);
      let arg = null;
      if (j < n && src[j] === '(') {
        // balanced parens (quotes respected) so a nested ':not(:nth-child(2))'
        // or a quoted ')' survives intact for the recursive parse.
        const { inner, end } = scanBalanced(src, j + 1, '(', ')');
        arg = inner;
        i = end < n ? end + 1 : n;
      } else {
        i = j;
      }
      tokens.push({ k: 'pseudo', name, arg });
      continue;
    }
    if (isTypeStart(c)) {
      let j = i + 1;
      while (j < n && NAME_CHAR.test(src[j])) j++;
      tokens.push({ k: 'type', v: src.slice(i, j) });
      i = j; continue;
    }
    throw new SyntaxError(`unexpected '${c}' in selector`);
  }
  return tokens;
}

// compound := [type|'*'] ( '#'id | '.'class | '['attr']' | ':'pseudo )*  — but
// lenient: parts may appear in any order/multiplicity, and a redundant type/'*'
// after the first is dropped (so `div[id=x]y` ≡ `div[id=x]`). Consuming an empty
// compound (no parts at all, e.g. a leading combinator) is a SyntaxError.
function parseCompound(tokens, pos, end) {
  const compound = { tag: null, id: null, classes: [], attrs: [], pseudos: [] };
  let matched = false;
  while (pos < end) {
    const t = tokens[pos];
    if (t.k === 'star') { if (compound.tag === null) compound.tag = '*'; }
    else if (t.k === 'type') { if (compound.tag === null) compound.tag = t.v.toLowerCase(); }
    else if (t.k === 'id') { compound.id = t.v; }
    else if (t.k === 'class') { compound.classes.push(t.v); }
    else if (t.k === 'attr') { compound.attrs.push({ name: t.name, match: t.match }); }
    else if (t.k === 'pseudo') { compound.pseudos.push({ name: t.name, arg: t.arg }); }
    else break; // ws / comb / comma ends the compound
    pos++; matched = true;
  }
  if (!matched) throw new SyntaxError(`empty compound at token ${pos}`);
  return { compound, pos };
}

// Parse tokens[lo..hi) (one complex selector — commas already split out) into a
// Complex `{ first, rest: [{ combinator, compound }] }`: `first` is the leftmost
// compound and each `rest[i].combinator` connects `rest[i].compound` to the
// compound on its LEFT (`rest[i-1].compound`, or `first` when i==0). Building the
// pair only AFTER a trailing compound is read keeps combinator/compound in lockstep
// — there is no way to represent a stray combinator (the parallel-array desync the
// old shape allowed). An empty segment (no compound at all) returns `null`.
function parseComplexTokens(tokens, lo, hi) {
  // trim leading / trailing ws tokens (mirrors the old `src.trim()`)
  let start = lo, end = hi;
  while (start < end && tokens[start].k === 'ws') start++;
  while (end > start && tokens[end - 1].k === 'ws') end--;
  if (start >= end) return null; // empty segment ⇒ no compound (filtered by parseSelectorList)
  let r = parseCompound(tokens, start, end);
  const first = r.compound;
  const rest = [];
  let pos = r.pos;
  while (pos < end) {
    while (pos < end && tokens[pos].k === 'ws') pos++; // a ws run here ⇒ candidate descendant
    const t = tokens[pos];
    let combinator;
    if (t.k === 'comb') {
      combinator = t.v;
      pos++;
      while (pos < end && tokens[pos].k === 'ws') pos++;
      if (pos >= end) break; // dangling combinator (`a >`) — drop it (no trailing compound to pair)
    } else {
      // the only token that can follow a compound after a ws run is the next
      // compound's first part ⇒ a descendant combinator
      combinator = ' ';
    }
    r = parseCompound(tokens, pos, end);
    rest.push({ combinator, compound: r.compound });
    pos = r.pos;
  }
  return { first, rest };
}

// Parse one complex selector STRING (no top-level comma). Exposed via `_internal`.
// Returns the Complex `{ first, rest }` or `null` (empty selector).
function parseComplex(src) {
  const tokens = tokenize(src);
  return parseComplexTokens(tokens, 0, tokens.length);
}

// Yield each top-level (comma-separated) segment of `tokens` as a `[lo, hi)` range.
// One shared loop for both the selector-list and the relative-selector-list parses.
function* splitTopLevelSegments(tokens) {
  let start = 0;
  for (let i = 0; i <= tokens.length; i++) {
    if (i === tokens.length || tokens[i].k === 'comma') {
      yield [start, i];
      start = i + 1;
    }
  }
}

// parsed-selector cache: querySelector(All)/matches re-run the same selector
// strings constantly; parsing once and reusing is a large win on query-heavy
// suites. Bounded so a pathological generator can't grow it without limit.
const __selectorCache = new Map();
export function parseSelectorList(selector) {
  const hit = __selectorCache.get(selector);
  if (hit !== undefined) return hit;
  const tokens = tokenize(selector);
  const parsed = [];
  for (const [lo, hi] of splitTopLevelSegments(tokens)) {
    const cx = parseComplexTokens(tokens, lo, hi);
    if (cx !== null) parsed.push(cx); // a Complex is always non-empty (empties dropped)
  }
  if (__selectorCache.size > 10000) __selectorCache.clear();
  __selectorCache.set(selector, parsed);
  return parsed;
}

// :has() relative-selector-list parse + cache. Each top-level comma segment may
// begin with a combinator ('>'/'+'/'~'); absent ⇒ descendant. The remainder is a
// normal complex (reused parseComplex). Segments yielding no complex are dropped.
// Mirrors the Rust parse_relative_selector_list.
const __relativeCache = new Map();
function parseRelativeSelectorList(arg) {
  const src = arg || '';
  const hit = __relativeCache.get(src);
  if (hit !== undefined) return hit;
  const tokens = tokenize(src);
  const out = [];
  for (let [lo, hi] of splitTopLevelSegments(tokens)) {
    while (lo < hi && tokens[lo].k === 'ws') lo++; // skip leading ws of the segment
    let combinator = ' ';
    if (lo < hi && tokens[lo].k === 'comb') {
      combinator = tokens[lo].v;
      lo++;
    }
    const complex = parseComplexTokens(tokens, lo, hi);
    if (complex !== null) out.push({ combinator, complex });
  }
  if (__relativeCache.size > 10000) __relativeCache.clear();
  __relativeCache.set(src, out);
  return out;
}

function matchAttr(el, a) {
  const raw = el.getAttribute(a.name);   // single lookup (null = absent)
  if (raw === null) return false;
  const m = a.match;
  if (m.op === 'present') return true;   // presence carries no value
  const v = raw;
  const t = m.value;
  switch (m.op) {
    case 'equals': return v === t;
    case 'prefix': return t !== '' && v.startsWith(t);
    case 'suffix': return t !== '' && v.endsWith(t);
    case 'substr': return t !== '' && v.includes(t);
    case 'includes': return v.split(/\s+/).includes(t);
    case 'dash': return v === t || v.startsWith(t + '-');
    default: return false;
  }
}

function matchPseudo(el, p) {
  switch (p.name) {
    case 'not':
      return !parseSelectorList(p.arg).some((cx) => matchComplex(el, cx));
    // :is()/:where() — element matches if ANY complex in the list matches it
    // (anchored at the element). :where matches identically to :is; the
    // specificity difference is a cascade concern, out of scope here.
    case 'is':
    case 'where':
      return parseSelectorList(p.arg).some((cx) => matchComplex(el, cx));
    // :has() — relational. SOME relative selector finds a matching element
    // reachable from `el` per its leading combinator, scoped to `el`.
    case 'has':
      return parseRelativeSelectorList(p.arg).some((rel) => hasMatch(el, rel));
    case 'first-child':
      return previousElement(el) === null;
    case 'last-child':
      return nextElement(el) === null;
    case 'only-child':
      return previousElement(el) === null && nextElement(el) === null;
    case 'empty':
      return el.childNodes.length === 0;
    case 'root':
      return el.parentNode == null || el.parentNode.nodeType === 9;
    // form-state pseudo-classes — read the live PROPERTY (what React sets),
    // not just the HTML attribute.
    case 'checked':
      return el.localName === 'option' ? !!el.selected : !!el.checked;
    case 'disabled':
      return !!el.disabled || el.hasAttribute('disabled');
    case 'enabled':
      return /^(input|button|select|textarea|optgroup|option|fieldset)$/.test(el.localName) && !(el.disabled || el.hasAttribute('disabled'));
    case 'required':
      return !!el.required || el.hasAttribute('required');
    case 'optional':
      return /^(input|select|textarea)$/.test(el.localName) && !(el.required || el.hasAttribute('required'));
    case 'read-only':
      return el.hasAttribute('readonly') || !!el.readOnly;
    case 'read-write':
      return !(el.hasAttribute('readonly') || el.readOnly);
    case 'selected':
      return !!el.selected;
    // positional pseudo-classes (1-based, support An+B / odd / even)
    case 'nth-child':
      return nthMatch(p.arg, siblingIndex(el) + 1);
    case 'nth-last-child':
      return nthMatch(p.arg, siblingCount(el) - siblingIndex(el));
    case 'nth-of-type':
      return nthMatch(p.arg, typeIndex(el) + 1);
    case 'nth-last-of-type':
      return nthMatch(p.arg, typeCount(el) - typeIndex(el));
    case 'first-of-type':
      return typeIndex(el) === 0;
    case 'last-of-type':
      return typeIndex(el) === typeCount(el) - 1;
    case 'only-of-type':
      return typeCount(el) === 1;
    default:
      return false; // unknown pseudo: never matches (honest, not a silent true)
  }
}

function elementSiblings(el) {
  const p = el.parentNode;
  if (!p) return [el];
  const kids = typeof p.__children === 'function' ? p.__children() : Array.from(p.childNodes || []);
  return kids.filter((n) => n.nodeType === 1);
}
function siblingIndex(el) { return elementSiblings(el).indexOf(el); }
function siblingCount(el) { return elementSiblings(el).length; }
function typeIndex(el) { return elementSiblings(el).filter((n) => n.localName === el.localName).indexOf(el); }
function typeCount(el) { return elementSiblings(el).filter((n) => n.localName === el.localName).length; }

// match An+B / odd / even / integer against a 1-based index
function nthMatch(arg, index) {
  const a0 = String(arg || '').trim().toLowerCase();
  if (a0 === 'odd') return index % 2 === 1;
  if (a0 === 'even') return index % 2 === 0;
  const m = /^([+-]?\d*)n\s*([+-]\s*\d+)?$/.exec(a0.replace(/\s+/g, ''));
  if (m) {
    let a = m[1] === '' || m[1] === '+' ? 1 : m[1] === '-' ? -1 : parseInt(m[1], 10);
    const b = m[2] ? parseInt(m[2].replace(/\s/g, ''), 10) : 0;
    if (a === 0) return index === b;
    return (index - b) % a === 0 && (index - b) / a >= 0;
  }
  const n = parseInt(a0, 10);
  return !Number.isNaN(n) && index === n;
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

// allocation-free "does the class attribute contain this whole-word class"
function hasClass(cn, cls) {
  if (cn === cls) return true;
  const L = cls.length;
  let idx = cn.indexOf(cls);
  while (idx !== -1) {
    const before = idx === 0 || cn.charCodeAt(idx - 1) <= 32;
    const after = idx + L === cn.length || cn.charCodeAt(idx + L) <= 32;
    if (before && after) return true;
    idx = cn.indexOf(cls, idx + 1);
  }
  return false;
}

function matchCompound(el, compound) {
  if (!el || el.nodeType !== 1) return false;
  // cheapest checks first; tag/local is a plain property
  if (compound.tag && compound.tag !== '*' && el.localName !== compound.tag) return false;
  if (compound.id !== null && el.getAttribute('id') !== compound.id) return false;
  // classes: read the class attribute ONCE and test membership without
  // allocating (no ClassList, no split, no padded copy) — dominated matching.
  if (compound.classes.length) {
    const cn = el.getAttribute('class');
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
// The rightmost compound sits at index `cx.rest.length`.
function matchComplex(el, cx) {
  return matchChain(el, cx, cx.rest.length, null);
}

// Match compounds 0..=k of `cx` ending at `el`, recursing leftward. Descendant (' ')
// and general-sibling ('~') combinators try EVERY candidate ancestor/sibling and
// backtrack — committing to the nearest one (the old greedy walk) wrongly rejected
// chains like `.a > .b .c` where a farther `.b` is the one that is a child of `.a`.
// `cx.rest[k-1].combinator` is the relation between compounds k-1 and k.
// `scope` supports `:has()`: when set to `{ comb, node }`, the LEFTMOST compound must,
// in addition to matching, relate to the scope element `node` by `comb` (so a relative
// selector's head is anchored to the `:has()` subject). `null` (what matchComplex and
// plain matching pass) means the leftmost compound stands alone — one sentinel.
function matchChain(el, cx, k, scope = null) {
  if (!matchCompound(el, compoundAt(cx, k))) return false;
  if (k === 0) return scope === null ? true : relatesToScope(el, scope.comb, scope.node);
  const comb = cx.rest[k - 1].combinator; // relation between compounds k-1 and k
  if (comb === '>') {
    const p = el.parentNode;
    return !!p && p.nodeType === 1 && matchChain(p, cx, k - 1, scope);
  }
  if (comb === '+') {
    const prev = previousElement(el);
    return !!prev && matchChain(prev, cx, k - 1, scope);
  }
  if (comb === ' ') {
    let anc = el.parentNode;
    while (anc && anc.nodeType === 1) {
      if (matchChain(anc, cx, k - 1, scope)) return true;
      anc = anc.parentNode;
    }
    return false;
  }
  // '~' general sibling
  let prev = previousElement(el);
  while (prev) {
    if (matchChain(prev, cx, k - 1, scope)) return true;
    prev = previousElement(prev);
  }
  return false;
}

// Does `node` stand in relation `comb` to the `:has()` scope element `el`? This anchors
// the leftmost compound of a relative selector: `> .a` means the `.a` head is a direct
// child of the subject, `.a .b` means the `.a` head is a descendant, etc. The leading
// combinator constrains the HEAD compound — NOT the whole complex (the bug in anchoring
// the entire relative complex at a child/sibling candidate).
function relatesToScope(node, comb, el) {
  if (comb === '>') return node.parentNode === el;
  if (comb === '+') return previousElement(node) === el;
  if (comb === '~') { let p = previousElement(node); while (p) { if (p === el) return true; p = previousElement(p); } return false; }
  // ' ' descendant: `el` is a proper ancestor of `node`
  let a = node.parentNode;
  while (a) { if (a === el) return true; a = a.parentNode; }
  return false;
}

// :has() existence search. The relative complex must match some target element (its
// rightmost compound) reachable from the subject `el`, with its HEAD compound related
// to `el` by the leading combinator (enforced at k===0 via `relatesToScope`). Candidate
// targets: descendants of `el` for `>`/descendant heads (the head, and thus the whole
// match, lives in `el`'s subtree); a following-sibling subtree for `+`/`~` heads.
// Mirrors the Rust has_match.
function hasMatch(el, rel) {
  const { combinator, complex } = rel;
  const scope = { comb: combinator, node: el };
  const test = (x) => matchChain(x, complex, complex.rest.length, scope);
  if (combinator === '+') {
    const sib = nextElement(el);
    return !!sib && someSelfOrDescendant(sib, test);
  }
  if (combinator === '~') {
    let sib = nextElement(el);
    while (sib) {
      if (someSelfOrDescendant(sib, test)) return true;
      sib = nextElement(sib);
    }
    return false;
  }
  // '>' and descendant leading combinator: the head (hence every match) is within
  // `el`'s subtree, so any descendant can be the target endpoint.
  return someDescendant(el, test);
}

// Visit every descendant element of `node` (document order); short-circuit on the
// first that satisfies `pred`. No filtered-copy allocation per level.
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

// `node` itself (if an element) or any of its descendants satisfies `pred`.
function someSelfOrDescendant(node, pred) {
  if (node.nodeType === 1 && pred(node)) return true;
  return someDescendant(node, pred);
}

export function matchesSelector(el, selector) {
  const list = parseSelectorList(selector);
  return list.some((cx) => matchComplex(el, cx));
}

// Fast paths for the overwhelmingly common simple selectors, skipping the
// parse + per-element matchComplex machinery.
const SIMPLE = /^\s*(#[\w-]+|\.[\w-]+|[a-zA-Z][\w-]*)\s*$/;
function simpleMatcher(selector) {
  const m = SIMPLE.exec(selector);
  if (!m) return null;
  const s = m[1];
  if (s[0] === '#') { const id = s.slice(1); return (el) => el.getAttribute('id') === id; }
  if (s[0] === '.') { const cls = s.slice(1); return (el) => { const cn = el.getAttribute('class'); return cn ? hasClass(cn, cls) : false; }; }
  const tag = s.toLowerCase(); return (el) => el.localName === tag;
}

// child node array without allocating a filtered copy per call
function rawChildren(node) {
  return typeof node.__children === 'function' ? node.__children() : Array.from(node.childNodes || []);
}

export function querySelectorAll(root, selector) {
  const simple = simpleMatcher(selector);
  const out = [];
  if (simple) {
    const visit = (node) => {
      const kids = rawChildren(node);
      for (let i = 0; i < kids.length; i++) { const c = kids[i]; if (c.nodeType !== 1) continue; if (simple(c)) out.push(c); visit(c); }
    };
    visit(root);
    return out;
  }
  const list = parseSelectorList(selector);
  const single = list.length === 1 ? list[0] : null;
  const visit = (node) => {
    const kids = rawChildren(node);
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType !== 1) continue;
      if (single ? matchComplex(c, single) : list.some((cx) => matchComplex(c, cx))) out.push(c);
      visit(c);
    }
  };
  visit(root);
  return out;
}

export function querySelector(root, selector) {
  const simple = simpleMatcher(selector);
  if (simple) {
    const visit = (node) => {
      const kids = rawChildren(node);
      for (let i = 0; i < kids.length; i++) { const c = kids[i]; if (c.nodeType !== 1) continue; if (simple(c)) return c; const r = visit(c); if (r) return r; }
      return null;
    };
    return visit(root);
  }
  const list = parseSelectorList(selector);
  const single = list.length === 1 ? list[0] : null;
  const visit = (node) => {
    const kids = rawChildren(node);
    for (let i = 0; i < kids.length; i++) {
      const c = kids[i];
      if (c.nodeType !== 1) continue;
      if (single ? matchComplex(c, single) : list.some((cx) => matchComplex(c, cx))) return c;
      const r = visit(c);
      if (r) return r;
    }
    return null;
  };
  return visit(root);
}

export const _internal = { parseComplex, parseSelectorList, matchCompound, matchComplex };

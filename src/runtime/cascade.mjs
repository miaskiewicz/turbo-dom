// Partial computed-style cascade — resolves getComputedStyle from injected <style>
// sheets (emotion/MUI `.css-HASH { … }`), inline styles, and specificity ordering.
//
// PERF: every hot path (parse, querySelectorAll, getElementsBy*, matching, events)
// NEVER calls getComputedStyle — only test assertions do. So all work here is lazy,
// gated behind getComputedStyle, and memoized on Document.__version. Zero bytes are
// added to any benchmarked path.
//
// HONEST: this only ever returns values that come from a REAL matched rule or inline
// style. A property no stylesheet/inline set still reads '' — it never invents
// layout/cascade numbers. Out of scope (return ''): @media/@supports/@keyframes,
// :hover & other stateful pseudo-classes, pseudo-elements, full inheritance,
// CSS custom-property resolution, length normalization.

import { matchesSelector } from './selectors.mjs';

const kebab = (s) => s.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());

// Longhand → shorthand fallback for single-token shorthands (mirrors dom.mjs styleGet).
const SHORTHAND_OF = {
  'background-color': 'background',
  'margin-top': 'margin', 'margin-right': 'margin', 'margin-bottom': 'margin', 'margin-left': 'margin',
  'padding-top': 'padding', 'padding-right': 'padding', 'padding-bottom': 'padding', 'padding-left': 'padding',
};

// Length properties whose bare `0` real browsers serialize as `0px` in computed style.
const LENGTH_PROPS = new Set([
  'width', 'height', 'min-width', 'max-width', 'min-height', 'max-height',
  'top', 'right', 'bottom', 'left', 'flex-basis', 'gap', 'row-gap', 'column-gap',
  'margin-top', 'margin-right', 'margin-bottom', 'margin-left',
  'padding-top', 'padding-right', 'padding-bottom', 'padding-left',
  'border-width', 'border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width',
  'font-size', 'letter-spacing', 'word-spacing', 'text-indent',
]);

const BORDER_STYLES = new Set(['none', 'hidden', 'dotted', 'dashed', 'solid', 'double', 'groove', 'ridge', 'inset', 'outset']);

// Expand the common shorthands into longhands so longhand computed getters resolve
// (e.g. `border:1px solid` → borderWidth '1px'). Cheap, test-time only; later
// declarations of the same longhand override naturally (cascade order preserved).
function setProp(map, name, val) {
  map.set(name, val);
  if (name === 'margin' || name === 'padding') {
    const t = val.trim().split(/\s+/);
    const top = t[0], right = t[1] ?? top, bottom = t[2] ?? top, left = t[3] ?? right;
    map.set(name + '-top', top); map.set(name + '-right', right);
    map.set(name + '-bottom', bottom); map.set(name + '-left', left);
  } else if (name === 'border') {
    let width, style, color;
    for (const p of val.trim().split(/\s+/)) {
      if (BORDER_STYLES.has(p)) style = p;
      else if (/^(thin|medium|thick)$/.test(p) || /^[\d.]+(px|em|rem|%|pt|vh|vw)?$/.test(p)) width = p;
      else color = p;
    }
    if (width !== undefined) { map.set('border-width', width); for (const s of ['top', 'right', 'bottom', 'left']) map.set(`border-${s}-width`, width); }
    if (style !== undefined) map.set('border-style', style);
    if (color !== undefined) map.set('border-color', color);
  } else if (name === 'background' && !/\s/.test(val.trim())) {
    map.set('background-color', val.trim());
  }
}

function parseDecls(text, into) {
  const map = into || new Map();
  for (const decl of text.split(';')) {
    const c = decl.indexOf(':');
    if (c === -1) continue;
    const name = decl.slice(0, c).trim().toLowerCase();
    if (!name) continue;
    setProp(map, name, decl.slice(c + 1).trim().replace(/\s*!\s*important\s*$/i, ''));
  }
  return map;
}

// rough WHATWG specificity: a*10000 + b*100 + c (ids, classes/attrs/pseudos, types)
function specificity(sel) {
  const a = (sel.match(/#[\w-]+/g) || []).length;
  const b = (sel.match(/\.[\w-]+|\[[^\]]*\]|:[\w-]+/g) || []).length;
  const c = (sel.match(/(^|[\s>+~])[a-zA-Z][\w-]*/g) || []).length;
  return a * 10000 + b * 100 + c;
}

// Brace-depth scan: emit only depth-1 rules whose selector isn't an at-rule; the
// bodies of @media/@keyframes (nested braces) are skipped wholesale.
function parseStylesheet(css, startOrder, rules) {
  css = css.replace(/\/\*[\s\S]*?\*\//g, '');
  let order = startOrder, i = 0;
  const n = css.length;
  while (i < n) {
    let j = i;
    while (j < n && css[j] !== '{' && css[j] !== '}') j++;
    if (j >= n) break;
    if (css[j] === '}') { i = j + 1; continue; }
    const sel = css.slice(i, j).trim();
    let depth = 1, k = j + 1;
    while (k < n && depth > 0) { const ch = css[k]; if (ch === '{') depth++; else if (ch === '}') depth--; k++; }
    if (sel && sel[0] !== '@') {
      const decls = parseDecls(css.slice(j + 1, k - 1));
      if (decls.size) for (const part of sel.split(',')) {
        const t = part.trim();
        if (t) rules.push({ selector: t, decls, spec: specificity(t), order: order++ });
      }
    } else { order++; }
    i = k;
  }
  return order;
}

function buildIndex(document) {
  const styles = document.getElementsByTagName('style');
  const rules = [];
  let order = 0;
  for (let i = 0; i < styles.length; i++) order = parseStylesheet(styles[i].textContent || '', order, rules);
  return rules;
}

function getIndex(doc, v) {
  const cached = doc.__styleIndex;
  if (cached && cached.v === v) return cached.rules;
  const rules = buildIndex(doc);
  doc.__styleIndex = { v, rules };
  return rules;
}

function resolve(el, rules) {
  const out = new Map();
  const matched = [];
  for (let i = 0; i < rules.length; i++) {
    const r = rules[i];
    try { if (matchesSelector(el, r.selector)) matched.push(r); } catch { /* unsupported selector → skip */ }
  }
  matched.sort((x, y) => x.spec - y.spec || x.order - y.order);
  for (const r of matched) for (const [k, val] of r.decls) out.set(k, val);
  // inline style wins over stylesheet rules
  const inline = el.getAttribute && el.getAttribute('style');
  if (inline) parseDecls(inline, out);
  return out;
}

function lookup(map, prop) {
  let v = map.get(prop);
  if (v === undefined) {
    const sh = SHORTHAND_OF[prop];
    if (sh) { const s = map.get(sh); if (s !== undefined && !/\s/.test(s.trim())) v = s; }
  }
  if (v === undefined) return '';
  // px-normalize a bare `0` for length properties (browsers report `0px`)
  if (v === '0' && LENGTH_PROPS.has(prop)) return '0px';
  // font-family: browsers serialize the list with ", " regardless of source spacing
  // (emotion minifies to "a",b,c). Scoped to font-family so we don't rewrite commas
  // inside rgb()/cubic-bezier() values, which browsers leave compact.
  if (prop === 'font-family') return v.replace(/\s*,\s*/g, ', ');
  return v;
}

function makeProxy(map, v) {
  return new Proxy({}, {
    get(_t, key) {
      if (key === '__v') return v;
      if (key === '__honest') return 'computed style resolves injected <style> + inline; no layout/@media/state';
      if (key === 'getPropertyValue') return (p) => lookup(map, String(p).toLowerCase());
      if (key === 'getPropertyPriority') return () => '';
      if (key === 'length') return map.size;
      if (key === 'item') return (i) => [...map.keys()][i] ?? '';
      if (key === Symbol.iterator) { const ks = [...map.keys()]; return ks[Symbol.iterator].bind(ks); }
      if (typeof key !== 'string') return undefined;
      return lookup(map, kebab(key));
    },
  });
}

// getComputedStyle(el): resolves cascade, memoized per element on the current
// Document.__version (any style/DOM mutation bumps it → cache auto-invalidates).
export function makeGetComputedStyle() {
  return (el) => {
    if (!el) return makeProxy(new Map(), -1);
    const doc = el.ownerDocument;
    const v = doc ? (doc.__version || 0) : 0;
    const cached = el.__computedStyle;
    if (cached && cached.__v === v) return cached;
    const map = doc ? resolve(el, getIndex(doc, v)) : new Map();
    const proxy = makeProxy(map, v);
    el.__computedStyle = proxy;
    return proxy;
  };
}

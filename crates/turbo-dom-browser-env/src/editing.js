// Editing: Document.execCommand and queryCommand{Enabled,Indeterm,State,Supported,Value} with
// Chrome 156's DOM command table over contenteditable / designMode, an undo stack of reversible
// DOM primitives that coalesces typing like Blink's open TypingCommand, trusted `input` events
// (and `beforeinput` where Chrome fires one) and selection updates through the public
// Selection/Range API. Also HTMLElement.contentEditable/isContentEditable and
// InputEvent.getTargetRanges for engine-created events.
//
// Layout-dependent editing decisions (caret candidates, collapsed whitespace, paragraph and block
// boundaries) use the engine's computed display/white-space when it reports one and the HTML UA
// defaults otherwise. Inline text styles (weight, slant, decoration, script, color, font) are
// resolved from markup: UA element defaults, presentational attributes and style attributes.
(function () {
  'use strict';
  const native = globalThis.__editingNative; delete globalThis.__editingNative;
  const editContexts = globalThis.__editContextHandoff; delete globalThis.__editContextHandoff;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty, getOwn = Object.getOwnPropertyDescriptor, getProto = Object.getPrototypeOf, create = Object.create;
  const ErrorType = TypeError, StringValue = String, WeakMapType = WeakMap, NumberValue = Number, MathRound = Math.round, MathMax = Math.max, MathMin = Math.min, MathFloor = Math.floor;
  const wmGet = WeakMap.prototype.get, wmSet = WeakMap.prototype.set;
  const lowerF = String.prototype.toLowerCase, sliceF = String.prototype.slice, codeAtF = String.prototype.charCodeAt, indexOfF = String.prototype.indexOf, trimF = String.prototype.trim, fromCode = String.fromCharCode;
  const toFixedF = Number.prototype.toFixed;
  const lower = s => apply(lowerF, s, []);
  const sub = (s, a, b) => apply(sliceF, s, b === undefined ? [a] : [a, b]);
  const code = (s, i) => apply(codeAtF, s, [i]);
  const find = (s, t, from) => apply(indexOfF, s, [t, from || 0]);
  const trim = s => apply(trimF, s, []);
  const push = (a, v) => { define(a, a.length, { __proto__: null, value: v, writable: true, enumerable: true, configurable: true }); return v; };
  const list = () => [];
  const wget = (m, k) => apply(wmGet, m, [k]);
  const wset = (m, k, v) => { apply(wmSet, m, [k, v]); return v; };
  const record = () => create(null);
  function lookup(object, name) {
    for (let o = object; o !== null && o !== undefined; o = getProto(o)) { const d = getOwn(o, name); if (d) return d; }
    throw new ErrorType('editing: missing native member ' + name);
  }
  const getter = (object, name) => { const f = lookup(object, name).get; return target => apply(f, target, []); };
  const method = (object, name) => lookup(object, name).value;

  // ---- DOM primordials ------------------------------------------------------------------------
  const NP = g.Node.prototype, EP = g.Element.prototype, CP = g.CharacterData.prototype, DP = g.Document.prototype, HP = g.HTMLElement.prototype;
  const document = g.document;
  const rawTypeOf = getter(NP, 'nodeType'), rawOwnerOf = getter(NP, 'ownerDocument');
  const parentOf = getter(NP, 'parentNode'), firstOf = getter(NP, 'firstChild'), lastOf = getter(NP, 'lastChild');
  const nextOf = getter(NP, 'nextSibling'), prevOf = getter(NP, 'previousSibling'), connectedOf = getter(NP, 'isConnected');
  // nodeType never changes and ownerDocument only on adoption: both are comparatively costly native
  // accessors on hot paths, so they are memoized (the owner per API call).
  const typeMemo = new WeakMapType();
  let ownerMemo = new WeakMapType();
  function typeOf(node) { let t = apply(wmGet, typeMemo, [node]); if (t === undefined) { t = rawTypeOf(node); apply(wmSet, typeMemo, [node, t]); } return t; }
  function ownerOf(node) { let d = apply(wmGet, ownerMemo, [node]); if (d === undefined) { d = rawOwnerOf(node); apply(wmSet, ownerMemo, [node, d]); } return d; }
  const insertBeforeF = method(NP, 'insertBefore'), removeChildF = method(NP, 'removeChild'), cloneNodeF = method(NP, 'cloneNode');
  const localOf = getter(EP, 'localName'), namespaceOf = getter(EP, 'namespaceURI');
  const getAttrF = method(EP, 'getAttribute'), setAttrF = method(EP, 'setAttribute'), removeAttrF = method(EP, 'removeAttribute'), attributesOf = getter(EP, 'attributes');
  const dataOf = getter(CP, 'data'), insertDataF = method(CP, 'insertData'), deleteDataF = method(CP, 'deleteData');
  const createElementF = method(DP, 'createElement'), createTextF = method(DP, 'createTextNode');
  const designModeOf = getter(DP, 'designMode'), activeOf = getter(DP, 'activeElement');
  // NamedNodeMap members from its interface prototype, or on first use from a real map when the
  // interface object is absent (installing creates no nodes).
  let mapLength = null, mapItemF = null;
  try { if (typeof g.NamedNodeMap === 'function') { const proto = g.NamedNodeMap.prototype; const l = lookup(proto, 'length').get, item = lookup(proto, 'item').value; if (typeof l === 'function' && typeof item === 'function') { mapLength = map => apply(l, map, []); mapItemF = item; } } } catch (error) { mapLength = null; mapItemF = null; }
  function namedNodeMap(map) { if (mapLength === null) { mapLength = getter(map, 'length'); mapItemF = method(map, 'item'); } return map; }
  const attrNameOf = getter(g.Attr.prototype, 'name'), attrValueOf = getter(g.Attr.prototype, 'value');
  const getSelectionF = method(document, 'getSelection');
  const sample = apply(getSelectionF, document, []);
  const selCount = getter(sample, 'rangeCount'), selAnchor = getter(sample, 'anchorNode'), selAnchorOffset = getter(sample, 'anchorOffset');
  const selFocus = getter(sample, 'focusNode'), selFocusOffset = getter(sample, 'focusOffset');
  const selSetF = method(sample, 'setBaseAndExtent'), selClearF = method(sample, 'removeAllRanges'), selRangeAtF = method(sample, 'getRangeAt');
  const innerHTMLSet = lookup(EP, 'innerHTML').set;
  const computedF = g.getComputedStyle, cssValueF = typeof g.CSSStyleDeclaration === 'function' ? lookup(g.CSSStyleDeclaration.prototype, 'getPropertyValue').value : null;
  const InputEventType = g.InputEvent, EventType = g.Event, StaticRangeType = g.StaticRange;
  const originalPrint = typeof g.print === 'function' ? g.print : null;
  const HTML = 'http://www.w3.org/1999/xhtml';
  const NBSP = ' ';

  // The body element (the first <body> or <frameset> child of the root <html>), without the
  // Document.body accessor.
  function rootElementOf(doc) { for (let n = firstOf(doc); n !== null; n = nextOf(n)) if (typeOf(n) === 1) return n; return null; }
  function bodyOf(doc) {
    const root = rootElementOf(doc);
    if (root === null || nameOf(root) !== 'html') return null;
    for (let n = firstOf(root); n !== null; n = nextOf(n)) { const name = nameOf(n); if (name === 'body' || name === 'frameset') return n; }
    return null;
  }
  const isText = n => typeOf(n) === 3;
  const isElement = n => typeOf(n) === 1;
  const nameOf = n => typeOf(n) === 1 && namespaceOf(n) === HTML ? localOf(n) : '';
  const attr = (el, name) => apply(getAttrF, el, [name]);
  const hasAttr = (el, name) => attr(el, name) !== null;
  function indexOf(node) { let i = 0; for (let n = prevOf(node); n !== null; n = prevOf(n)) i++; return i; }
  function childAt(parent, offset) { let n = firstOf(parent); for (let i = 0; n !== null && i < offset; i++) n = nextOf(n); return n; }
  function lengthOf(node) { const t = typeOf(node); if (t === 3 || t === 4 || t === 7 || t === 8) return dataOf(node).length; let i = 0; for (let n = firstOf(node); n !== null; n = nextOf(n)) i++; return i; }
  function contains(a, b) { for (let n = b; n !== null; n = parentOf(n)) if (n === a) return true; return false; }
  function attributeList(el) { const map = namedNodeMap(attributesOf(el)), out = list(), n = mapLength(map); for (let i = 0; i < n; i++) { const a = apply(mapItemF, map, [i]); push(out, [attrNameOf(a), attrValueOf(a)]); } return out; }
  const P = (c, o) => ({ __proto__: null, c, o });
  const before = n => P(parentOf(n), indexOf(n));
  const after = n => P(parentOf(n), indexOf(n) + 1);
  function chain(node) { const out = list(); for (let n = node; n !== null; n = parentOf(n)) push(out, n); return out; }
  // Tree-order comparison of boundary points.
  function compare(a, b) {
    if (a.c === b.c) return a.o < b.o ? -1 : a.o > b.o ? 1 : 0;
    const A = chain(a.c), B = chain(b.c);
    let i = A.length - 1, j = B.length - 1;
    if (A[i] !== B[j]) return 0;
    while (i > 0 && j > 0 && A[i - 1] === B[j - 1]) { i--; j--; }
    if (i === 0) return a.o <= indexOf(B[j - 1]) ? -1 : 1;
    if (j === 0) return indexOf(A[i - 1]) < b.o ? -1 : 1;
    return indexOf(A[i - 1]) < indexOf(B[j - 1]) ? -1 : 1;
  }
  const same = (a, b) => a !== null && b !== null && a.c === b.c && a.o === b.o;

  // ---- inline style declarations ----------------------------------------------------------------
  let declarationMemo = new WeakMapType();
  function declarations(el) {
    const memo = apply(wmGet, declarationMemo, [el]);
    if (memo !== undefined) return memo;
    const value = parseDeclarations(el);
    apply(wmSet, declarationMemo, [el, value]);
    return value;
  }
  function parseDeclarations(el) {
    const text = attr(el, 'style'), out = list();
    if (text === null) return out;
    let depth = 0, quote = 0, start = 0;
    const flush = end => {
      const part = sub(text, start, end), colon = find(part, ':');
      if (colon > 0) { let value = trim(sub(part, colon + 1)); const bang = find(lower(value), '!important'); if (bang >= 0) value = trim(sub(value, 0, bang)); push(out, [lower(trim(sub(part, 0, colon))), value]); }
    };
    for (let i = 0; i < text.length; i++) {
      const c = code(text, i);
      if (quote) { if (c === quote) quote = 0; continue; }
      if (c === 34 || c === 39) quote = c; else if (c === 40) depth++; else if (c === 41) depth--; else if (c === 59 && depth <= 0) { flush(i); start = i + 1; }
    }
    flush(text.length);
    return out;
  }
  function declared(el, prop) { const d = declarations(el); let value = null; for (let i = 0; i < d.length; i++) if (d[i][0] === prop) value = d[i][1]; return value; }
  const serializeDeclarations = d => { let s = ''; for (let i = 0; i < d.length; i++) s += (i ? ' ' : '') + d[i][0] + ': ' + d[i][1] + ';'; return s; };

  // ---- display / white-space ---------------------------------------------------------------------
  const blockNames = record();
  for (const n of ['html', 'body', 'address', 'blockquote', 'center', 'dialog', 'div', 'figure', 'figcaption', 'footer', 'form', 'header', 'hr', 'legend', 'listing', 'main', 'p', 'plaintext', 'pre', 'search', 'xmp', 'article', 'aside', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'hgroup', 'nav', 'section', 'dd', 'dl', 'dt', 'menu', 'ol', 'ul', 'fieldset', 'details', 'summary', 'li', 'table', 'caption', 'thead', 'tbody', 'tfoot', 'tr', 'td', 'th', 'optgroup', 'frameset', 'frame'])
    blockNames[n] = 1;
  const hiddenNames = record();
  for (const n of ['head', 'script', 'style', 'template', 'title', 'meta', 'link', 'base', 'noscript', 'area', 'datalist', 'param', 'rp', 'colgroup', 'col']) hiddenNames[n] = 1;
  const atomicNames = record();
  for (const n of ['img', 'input', 'textarea', 'select', 'iframe', 'object', 'embed', 'video', 'audio', 'canvas', 'hr', 'meter', 'progress', 'br', 'wbr']) atomicNames[n] = 1;
  function computed(el, prop) {
    if (computedF === null || cssValueF === null) return '';
    try { const style = apply(computedF, g, [el]); return StringValue(apply(cssValueF, style, [prop])); } catch (error) { return ''; }
  }
  // Display is memoized per API call; author style sheets are consulted through the engine's
  // computed style only when the document has any (UA defaults and style attributes otherwise).
  let displayMemo = new WeakMapType();
  const styleSheetsOf = getter(document, 'styleSheets'), sheetCount = list => { try { return list.length; } catch (error) { return 0; } };
  let authorSheets = null;
  function hasAuthorSheets() { if (authorSheets === null) { try { authorSheets = sheetCount(styleSheetsOf(document)) > 0; } catch (error) { authorSheets = false; } } return authorSheets; }
  function resetMemo() { displayMemo = new WeakMapType(); ownerMemo = new WeakMapType(); declarationMemo = new WeakMapType(); editMemo = new WeakMapType(); selectionMemo = null; authorSheets = null; }
  function displayOf(el) {
    const memo = wget(displayMemo, el);
    if (memo !== undefined) return memo;
    return wset(displayMemo, el, computeDisplay(el));
  }
  function computeDisplay(el) {
    const own = declared(el, 'display');
    if (own !== null) return lower(own);
    const value = hasAuthorSheets() ? computed(el, 'display') : '';
    if (value) return value;
    const name = nameOf(el);
    if (name && hasAttr(el, 'hidden')) return 'none';
    if (hiddenNames[name]) return 'none';
    if (name === 'li') return 'list-item';
    if (typeOf(el) !== 1) return 'inline';
    if (namespaceOf(el) !== HTML) return localOf(el) === 'svg' || localOf(el) === 'math' ? 'inline' : 'inline';
    return blockNames[name] ? 'block' : 'inline';
  }
  const inlineDisplays = record();
  for (const n of ['inline', 'inline-block', 'inline-flex', 'inline-grid', 'inline-table', 'contents', 'ruby', 'ruby-text', 'none']) inlineDisplays[n] = 1;
  const isBlockDisplay = d => !inlineDisplays[d];
  const isBlock = node => isElement(node) && isBlockDisplay(displayOf(node));
  function whiteSpaceOf(el, inherited) {
    const own = declared(el, 'white-space');
    if (own !== null) return lower(own);
    const name = nameOf(el);
    if (name === 'pre' || name === 'listing' || name === 'xmp' || name === 'plaintext' || name === 'textarea') return 'pre';
    if (name === 'nobr') return 'nowrap';
    if (name && editState(el) === 'plain' && hasAttr(el, 'contenteditable')) return 'pre-wrap';
    return inherited;
  }
  const preserveMode = ws => ws === 'pre' || ws === 'pre-wrap' || ws === 'break-spaces' ? 1 : ws === 'pre-line' ? 2 : 0;

  // ---- editability -------------------------------------------------------------------------------
  // contenteditable on HTML elements sets -webkit-user-modify, which inherits; designMode makes the
  // document element editable. Returns 'rich', 'plain' or null.
  let editMemo = new WeakMapType();
  function editState(node) {
    const memo = apply(wmGet, editMemo, [node]);
    if (memo !== undefined) return memo;
    const value = computeEditState(node);
    apply(wmSet, editMemo, [node, value]);
    return value;
  }
  function computeEditState(node) {
    let el = isElement(node) ? node : parentOf(node);
    for (; el !== null && typeOf(el) === 1; el = parentOf(el)) {
      if (namespaceOf(el) !== HTML) continue;
      const value = attr(el, 'contenteditable');
      if (value === null) continue;
      const v = lower(value);
      if (v === '' || v === 'true') return 'rich';
      if (v === 'plaintext-only') return 'plain';
      if (v === 'false') return null;
    }
    if (el !== null && typeOf(el) === 11) return null;
    const owner = node && typeOf(node) === 9 ? node : ownerOf(node);
    if (owner !== null && el !== null && typeOf(el) === 9 && designModeOf(owner) === 'on') return 'rich';
    return null;
  }
  const isEditable = node => node !== null && editState(node) !== null;
  // Blink's RootEditableElement: the highest editable element ancestor, stopping at <body>.
  function rootEditable(node) {
    let result = null;
    const doc = typeOf(node) === 9 ? node : ownerOf(node), body = doc ? bodyOf(doc) : null;
    for (let n = node; n !== null && typeOf(n) !== 9 && isEditable(n); n = parentOf(n)) {
      if (isElement(n)) result = n;
      if (n === body) break;
    }
    return result;
  }
  const richly = node => editState(node) === 'rich';
  function blockOf(node, root) {
    for (let n = isElement(node) ? node : parentOf(node); n !== null && typeOf(n) === 1; n = parentOf(n)) { if (isBlock(n) || n === root) return n; }
    return root;
  }

  // ---- caret stream -------------------------------------------------------------------------------
  // A flattened rendering model of a root: rendered characters, line breaks, atomic inlines, block
  // atoms and block start/end markers. Visible positions are the gaps between items.
  const CHAR = 1, BR = 2, ATOM = 3, BATOM = 4, BS = 5, BE = 6;
  function buildStream(root) {
    const items = list();
    let lineStart = true, lastSpace = null;
    const emit = (t, n, o) => push(items, { __proto__: null, t, n, o });
    const endLine = () => { if (lastSpace !== null && items[items.length - 1] === lastSpace) items.length--; lastSpace = null; lineStart = true; };
    const editableRoot = isEditable(root);
    function text(node, mode) {
      const d = dataOf(node);
      for (let i = 0; i < d.length; i++) {
        const c = code(d, i);
        if (c === 10 && mode) { if (mode === 2) endLine(); emit(BR, node, i); lineStart = true; lastSpace = null; continue; }
        if (mode === 1) { emit(CHAR, node, i); lineStart = false; lastSpace = null; continue; }
        if (c === 32 || c === 9 || c === 10 || c === 13 || c === 12) { if (lineStart || lastSpace !== null) continue; lastSpace = emit(CHAR, node, i); continue; }
        emit(CHAR, node, i); lineStart = false; lastSpace = null;
      }
    }
    function walk(node, ws) {
      const mode = preserveMode(ws);
      for (let child = firstOf(node); child !== null; child = nextOf(child)) {
        const type = typeOf(child);
        if (type === 3) { text(child, mode); continue; }
        if (type !== 1) continue;
        const display = displayOf(child);
        if (display === 'none') continue;
        const name = nameOf(child);
        if (name === 'br') { if (mode !== 1) endLine(); emit(BR, child, -1); lineStart = true; lastSpace = null; continue; }
        const island = editableRoot && !isEditable(child);
        if ((name && atomicNames[name] && name !== 'wbr') || island || (namespaceOf(child) !== HTML && (localOf(child) === 'svg' || localOf(child) === 'math'))) {
          if (isBlockDisplay(display)) { endLine(); emit(BATOM, child, -1); lineStart = true; } else { emit(ATOM, child, -1); lineStart = false; lastSpace = null; }
          continue;
        }
        if (name === 'wbr') continue;
        if (isBlockDisplay(display)) { endLine(); emit(BS, child, -1); walk(child, whiteSpaceOf(child, ws)); endLine(); emit(BE, child, -1); continue; }
        walk(child, whiteSpaceOf(child, ws));
      }
    }
    let ws = 'normal';
    for (let n = parentOf(root); n !== null && typeOf(n) === 1; n = parentOf(n)) { const own = whiteSpaceOf(n, null); if (own !== null) { ws = own; break; } }
    emit(BS, root, -1); walk(root, whiteSpaceOf(root, ws)); endLine(); emit(BE, root, -1);
    return items;
  }
  let streamCache = null, mutationCount = 0;
  function streamOf(root) {
    if (streamCache !== null && streamCache.root === root && streamCache.count === mutationCount) return streamCache.items;
    const items = buildStream(root);
    streamCache = { __proto__: null, root, count: mutationCount, items };
    return items;
  }
  function itemEnd(item) {
    if (item.t === CHAR || (item.t === BR && item.o >= 0)) return P(item.n, item.o + 1);
    if (item.t === BS) return P(item.n, 0);
    return after(item.n);
  }
  function itemStart(item) {
    if (item.t === CHAR || (item.t === BR && item.o >= 0)) return P(item.n, item.o);
    if (item.t === BE) return P(item.n, lengthOf(item.n));
    return before(item.n);
  }
  function gapOf(items, pos) {
    let lo = 0, hi = items.length;
    while (lo < hi) { const mid = (lo + hi) >> 1; if (compare(itemEnd(items[mid]), pos) <= 0) lo = mid + 1; else hi = mid; }
    return lo;
  }
  function upstream(items, k) {
    const prev = k > 0 ? items[k - 1] : null;
    if (prev === null) return null;
    if (prev.t === CHAR) return P(prev.n, prev.o + 1);
    if (prev.t === ATOM || prev.t === BATOM) return after(prev.n);
    return null;
  }
  function downstream(items, k) {
    const next = k < items.length ? items[k] : null;
    if (next === null) return null;
    if (next.t === CHAR || (next.t === BR && next.o >= 0)) return P(next.n, next.o);
    if (next.t === ATOM || next.t === BATOM || next.t === BR) {
      // The position after a block's trailing line break is not a caret position.
      return before(next.n);
    }
    return null;
  }
  // An empty editing host keeps one line (LayoutBlock::HasLineIfEmpty), so its start is a caret.
  function emptyHost(items, k) {
    if (k !== 1 || items.length !== 2 || items[0].t !== BS || items[1].t !== BE) return null;
    const root = items[0].n;
    return rootEditable(root) === root || (typeOf(root) === 1 && ownerOf(root) !== null && bodyOf(ownerOf(root)) === root) ? P(root, 0) : null;
  }
  const candidate = (items, k) => upstream(items, k) || downstream(items, k) || emptyHost(items, k);
  const forwardCandidate = (items, k) => downstream(items, k) || upstream(items, k) || emptyHost(items, k);
  const isCandidateGap = (items, k) => candidate(items, k) !== null;
  // The root a position's visible context lives in: its editing host, or its block when not editable.
  function contextRoot(pos) {
    const node = pos.c;
    const root = rootEditable(node);
    if (root !== null) return root;
    // A non-editable position stays within its non-editable island.
    let top = isElement(node) ? node : parentOf(node);
    while (top !== null && parentOf(top) !== null && typeOf(parentOf(top)) === 1 && !isEditable(parentOf(top))) top = parentOf(top);
    return top === null ? node : top;
  }
  function canonical(pos) { if (pos === null) return null; const items = streamOf(contextRoot(pos)); return candidate(items, gapOf(items, pos)); }
  function canonicalForward(pos) { const items = streamOf(contextRoot(pos)); return forwardCandidate(items, gapOf(items, pos)); }
  function prevGap(items, k) { for (let j = k - 1; j >= 0; j--) if (isCandidateGap(items, j) && !same(candidate(items, j), candidate(items, k))) return j; return -1; }
  function nextGap(items, k) { for (let j = k + 1; j <= items.length; j++) if (isCandidateGap(items, j) && !same(candidate(items, j), candidate(items, k))) return j; return -1; }
  const boundaryType = t => t === BS || t === BE || t === BR || t === BATOM;
  const startOfParagraph = (items, k) => k === 0 || boundaryType(items[k - 1].t);
  const endOfParagraph = (items, k) => k >= items.length || boundaryType(items[k].t);
  const startOfBlock = (items, k) => k === 0 || items[k - 1].t === BS;
  const endOfBlock = (items, k) => k >= items.length || items[k].t === BE || (items[k].t === BR && k + 1 < items.length && items[k + 1].t === BE);
  const lineBreakAt = (items, k) => k < items.length && items[k].t === BR;

  // ---- reversible DOM primitives -------------------------------------------------------------------
  let journal = null;
  // Positions the running command tracks across its own primitives (Blink keeps its Positions up to
  // date the same way): offsets follow text edits and sibling changes; a removal relocates contained
  // marks to the removal point unless the node is being moved.
  let marks = list();
  function mark(pos) { const m = { __proto__: null, c: pos.c, o: pos.o }; push(marks, m); return m; }
  const marked = m => P(m.c, m.o);
  const log = op => { mutationCount++; editMemo = new WeakMapType(); selectionMemo = null; if (journal !== null) push(journal, op); };
  function insertNode(node, parent, ref) {
    apply(insertBeforeF, parent, [node, ref]);
    const at = indexOf(node);
    for (let i = 0; i < marks.length; i++) { const m = marks[i]; if (m.c === parent && m.o > at) m.o++; }
    log({ __proto__: null, k: 1, node, parent, ref });
  }
  function removeNode(node, moving) {
    const parent = parentOf(node);
    if (parent === null) return;
    const ref = nextOf(node), at = indexOf(node);
    apply(removeChildF, parent, [node]);
    for (let i = 0; i < marks.length; i++) {
      const m = marks[i];
      if (!moving && contains(node, m.c)) { m.c = parent; m.o = at; continue; }
      if (m.c === parent && m.o > at) m.o--;
    }
    log({ __proto__: null, k: 2, node, parent, ref });
  }
  function insertText(node, offset, text) {
    if (!text.length) return;
    apply(insertDataF, node, [offset, text]);
    for (let i = 0; i < marks.length; i++) { const m = marks[i]; if (m.c === node && m.o > offset) m.o += text.length; }
    log({ __proto__: null, k: 3, node, o: offset, s: text });
  }
  function deleteText(node, offset, count) {
    if (count <= 0) return;
    const s = sub(dataOf(node), offset, offset + count);
    apply(deleteDataF, node, [offset, count]);
    for (let i = 0; i < marks.length; i++) { const m = marks[i]; if (m.c === node && m.o > offset) m.o = m.o <= offset + count ? offset : m.o - count; }
    log({ __proto__: null, k: 4, node, o: offset, s });
  }
  function replaceText(node, offset, count, text) { deleteText(node, offset, count); insertText(node, offset, text); }
  function setAttr(el, name, value) { const old = attr(el, name); apply(setAttrF, el, [name, value]); resetMemo(); log({ __proto__: null, k: 5, el, name, old, now: value }); }
  function removeAttr(el, name) { const old = attr(el, name); if (old === null) return; apply(removeAttrF, el, [name]); resetMemo(); log({ __proto__: null, k: 5, el, name, old, now: null }); }
  const appendNode = (node, parent) => insertNode(node, parent, null);
  const insertAfter = (node, ref) => insertNode(node, parentOf(ref), nextOf(ref));
  function moveNode(node, parent, ref) { removeNode(node, true); insertNode(node, parent, ref); }
  function insertAt(node, pos) {
    if (isText(pos.c)) {
      if (pos.o === 0) { insertNode(node, parentOf(pos.c), pos.c); return; }
      if (pos.o >= dataOf(pos.c).length) { insertAfter(node, pos.c); return; }
      splitText(pos.c, pos.o); insertNode(node, parentOf(pos.c), pos.c); return;
    }
    insertNode(node, pos.c, childAt(pos.c, pos.o));
  }
  function undoOp(op) {
    switch (op.k) {
      case 1: if (parentOf(op.node) === op.parent) apply(removeChildF, op.parent, [op.node]); break;
      case 2: apply(insertBeforeF, op.parent, [op.node, op.ref !== null && parentOf(op.ref) === op.parent ? op.ref : null]); break;
      case 3: apply(deleteDataF, op.node, [op.o, op.s.length]); break;
      case 4: apply(insertDataF, op.node, [op.o, op.s]); break;
      case 5: if (op.old === null) apply(removeAttrF, op.el, [op.name]); else apply(setAttrF, op.el, [op.name, op.old]); break;
    }
    mutationCount++;
  }
  function redoOp(op) {
    switch (op.k) {
      case 1: apply(insertBeforeF, op.parent, [op.node, op.ref !== null && parentOf(op.ref) === op.parent ? op.ref : null]); break;
      case 2: if (parentOf(op.node) === op.parent) apply(removeChildF, op.parent, [op.node]); break;
      case 3: apply(insertDataF, op.node, [op.o, op.s]); break;
      case 4: apply(deleteDataF, op.node, [op.o, op.s.length]); break;
      case 5: if (op.now === null) apply(removeAttrF, op.el, [op.name]); else apply(setAttrF, op.el, [op.name, op.now]); break;
    }
    mutationCount++;
  }
  const createElement = (doc, name) => apply(createElementF, doc, [name]);
  const createText = (doc, data) => apply(createTextF, doc, [data]);
  const cloneShallow = node => apply(cloneNodeF, node, [false]);
  // Blink's SplitTextNodeCommand: a new prefix node takes [0, offset); the original keeps the rest.
  function splitText(node, offset) {
    const prefix = createText(ownerOf(node), sub(dataOf(node), 0, offset));
    const inside = list();
    for (let i = 0; i < marks.length; i++) { const m = marks[i]; if (m.c === node) { push(inside, [m, m.o]); m.c = null; } }
    insertNode(prefix, parentOf(node), node);
    deleteText(node, 0, offset);
    for (let i = 0; i < inside.length; i++) { const m = inside[i][0], o = inside[i][1]; if (o < offset) { m.c = prefix; m.o = o; } else { m.c = node; m.o = o - offset; } }
    return prefix;
  }
  // Blink's SplitElementCommand: a clone takes the children before `child`; the original keeps the
  // rest and loses its id, which cannot be shared.
  function splitElement(element, child) {
    const clone = cloneShallow(element);
    insertNode(clone, parentOf(element), element);
    for (let n = firstOf(element); n !== null && n !== child;) { const next = nextOf(n); moveNode(n, clone, null); n = next; }
    removeAttr(element, 'id');
    return clone;
  }
  function unwrap(element) {
    const parent = parentOf(element);
    for (let n = firstOf(element); n !== null;) { const next = nextOf(n); moveNode(n, parent, element); n = next; }
    removeNode(element);
  }
  // Blink's MergeIdenticalElementsCommand: the second element survives with the first's children.
  function mergeIdentical(first, second) {
    const at = firstOf(second);
    for (let n = firstOf(first); n !== null;) { const next = nextOf(n); moveNode(n, second, at); n = next; }
    removeNode(first);
  }
  function identical(a, b) {
    if (!isElement(a) || !isElement(b) || nameOf(a) !== nameOf(b) || !nameOf(a)) return false;
    const x = attributeList(a), y = attributeList(b);
    if (x.length !== y.length) return false;
    for (let i = 0; i < x.length; i++) { if (attr(b, x[i][0]) !== x[i][1]) return false; }
    return true;
  }
  function replaceWithSpanOrRemove(element) {
    const attrs = attributeList(element);
    if (!attrs.length) { unwrap(element); return null; }
    const span = createElement(ownerOf(element), 'span');
    for (let i = 0; i < attrs.length; i++) apply(setAttrF, span, [attrs[i][0], attrs[i][1]]);
    insertNode(span, parentOf(element), element);
    for (let n = firstOf(element); n !== null;) { const next = nextOf(n); moveNode(n, span, null); n = next; }
    removeNode(element);
    return span;
  }
  function pruneEmptyAncestors(node, stop) {
    let n = node;
    while (n !== null && n !== stop && isElement(n) && firstOf(n) === null && !atomicNames[nameOf(n)] && !isBlock(n)) { const parent = parentOf(n); removeNode(n); n = parent; }
  }

  // ---- per-document editor state -------------------------------------------------------------------
  const editors = new WeakMapType();
  function editorOf(doc) {
    let state = wget(editors, doc);
    if (!state) { state = record(); state.undo = list(); state.redo = list(); state.typing = null; state.css = false; state.separator = 'div'; state.running = false; state.open = null; state.openSelection = null; state.openValid = false; state.pendingTyping = null; state.openRange = null; state.doc = doc; wset(editors, doc, state); }
    return state;
  }
  const targetRanges = new WeakMapType();

  // ---- selection ------------------------------------------------------------------------------------
  function selectionOf(doc) { try { return apply(getSelectionF, doc, []); } catch (error) { return null; } }
  // Script selection changes replace the selection's range object; the identity of the current range
  // tells an engine-set selection apart from any later Selection API call (which closes typing).
  function currentRange(doc) { const s = selectionOf(doc); if (!s || selCount(s) === 0) return null; try { return apply(selRangeAtF, s, [0]); } catch (error) { return null; } }
  function rawSelection(doc) {
    const s = selectionOf(doc);
    if (!s || selCount(s) === 0) return null;
    const anchor = P(selAnchor(s), selAnchorOffset(s)), focus = P(selFocus(s), selFocusOffset(s));
    if (anchor.c === null || focus.c === null) return null;
    const forward = compare(anchor, focus) <= 0;
    return { __proto__: null, anchor, focus, start: forward ? anchor : focus, end: forward ? focus : anchor, forward };
  }
  // Blink's VisibleSelection: a caret canonicalizes; a range's start moves downstream and its end
  // upstream, and the range is kept within the start's editing host.
  let selectionMemo = null;
  function visibleSelection(doc) {
    const range = currentRange(doc);
    if (selectionMemo !== null && selectionMemo.doc === doc && selectionMemo.range === range && selectionMemo.count === mutationCount) {
      const raw = rawSelection(doc);
      if (raw !== null && same(raw.anchor, selectionMemo.anchor) && same(raw.focus, selectionMemo.focus)) return selectionMemo.value;
    }
    const value = computeVisibleSelection(doc);
    const raw = rawSelection(doc);
    selectionMemo = raw === null ? null : { __proto__: null, doc, range, count: mutationCount, anchor: raw.anchor, focus: raw.focus, value };
    return value;
  }
  function computeVisibleSelection(doc) {
    const raw = rawSelection(doc);
    if (raw === null || ownerOf(raw.start.c) !== doc && raw.start.c !== doc) return null;
    if (!connectedOf(raw.start.c) || !connectedOf(raw.end.c)) return null;
    const root = rootEditable(raw.start.c);
    if (same(raw.start, raw.end)) {
      const caret = canonical(raw.start);
      if (caret === null) return null;
      return { __proto__: null, start: caret, end: caret, caret: true, root: rootEditable(caret.c), raw };
    }
    let start = canonicalForward(raw.start), end = canonical(raw.end);
    if (root !== null && !contains(root, raw.end.c)) end = canonical(P(root, lengthOf(root)));
    if (start === null || end === null) { const c = canonical(raw.start); if (c === null) return null; return { __proto__: null, start: c, end: c, caret: true, root: rootEditable(c.c), raw }; }
    if (compare(start, end) >= 0) return { __proto__: null, start, end: start, caret: true, root: rootEditable(start.c), raw };
    return { __proto__: null, start, end, caret: false, root: rootEditable(start.c), raw };
  }
  function setSelection(doc, start, end) {
    const s = selectionOf(doc);
    if (!s) return;
    if (start === null) { apply(selClearF, s, []); return; }
    const a = canonical(start) || start;
    const b = end === undefined || end === null ? a : (same(start, end) ? a : canonical(end) || end);
    apply(selSetF, s, [a.c, a.o, b.c, b.o]);
  }
  function setRangeSelection(doc, start, end) {
    const s = selectionOf(doc);
    if (!s) return;
    const a = canonicalForward(start) || start, b = canonical(end) || end;
    apply(selSetF, s, [a.c, a.o, b.c, b.o]);
  }

  // ---- events -----------------------------------------------------------------------------------------
  // Guest listeners may mutate the tree: no stream survives a dispatch.
  const dispatch = (target, event) => { const r = native('dispatch', target, event); streamCache = null; resetMemo(); return r; };
  function inputEvent(type, inputType, data, cancelable) {
    return new InputEventType(type, { __proto__: null, bubbles: true, cancelable, composed: true, inputType, data, isComposing: false, dataTransfer: null });
  }
  function fireInput(start, end, inputType, data) {
    if (start !== null) dispatch(start, inputEvent('input', inputType, data, false));
    if (end !== null && end !== start) dispatch(end, inputEvent('input', inputType, data, false));
  }
  function fireBeforeInput(target, inputType, data, range) {
    const event = inputEvent('beforeinput', inputType, data, true);
    if (range) wset(targetRanges, event, range);
    return dispatch(target, event);
  }

  // ---- inline style model ------------------------------------------------------------------------------
  const boldNames = { __proto__: null, b: 1, strong: 1, h1: 1, h2: 1, h3: 1, h4: 1, h5: 1, h6: 1, th: 1 };
  const italicNames = { __proto__: null, i: 1, em: 1, cite: 1, var: 1, dfn: 1, address: 1 };
  const headingSizes = { __proto__: null, h1: 2, h2: 1.5, h3: 1.17, h4: 1, h5: 0.83, h6: 0.67 };
  function weightNumber(value, parent) {
    const v = lower(trim(value));
    if (v === 'normal') return 400; if (v === 'bold') return 700;
    if (v === 'bolder') return parent < 350 ? 400 : parent < 550 ? 700 : 900;
    if (v === 'lighter') return parent < 550 ? 100 : parent < 750 ? 400 : 700;
    const n = NumberValue(v);
    return n === n && n >= 1 && n <= 1000 ? n : parent;
  }
  // Inherited text style at an element: weight, slant, decorations in effect, script, color, family, size.
  function styleAt(node) {
    const els = list();
    for (let n = isElement(node) ? node : parentOf(node); n !== null && typeOf(n) === 1; n = parentOf(n)) push(els, n);
    const s = record();
    s.weight = 400; s.italic = false; s.underline = false; s.strike = false; s.script = 'baseline'; s.color = null; s.background = null; s.family = null; s.size = 16; s.link = false; s.align = null;
    for (let i = els.length - 1; i >= 0; i--) {
      const el = els[i], name = nameOf(el);
      if (boldNames[name]) s.weight = name === 'b' || name === 'strong' ? weightNumber('bolder', s.weight) : 700;
      if (italicNames[name]) s.italic = true;
      if (name === 'u' || name === 'ins') s.underline = true;
      if (name === 's' || name === 'strike' || name === 'del') s.strike = true;
      if (name === 'sub') { s.script = 'sub'; s.size = s.size / 1.2; }
      if (name === 'sup') { s.script = 'super'; s.size = s.size / 1.2; }
      if (name === 'small') s.size = s.size / 1.2;
      if (name === 'big') s.size = s.size * 1.2;
      if (headingSizes[name]) s.size = 16 * headingSizes[name];
      if (name === 'a' && hasAttr(el, 'href')) { s.underline = true; s.link = true; s.color = 'rgb(0, 0, 238)'; }
      if (name === 'font') {
        const color = attr(el, 'color'); if (color !== null) { const c = parseColor(color, true); if (c) s.color = c; }
        const face = attr(el, 'face'); if (face !== null) s.family = face;
        const size = attr(el, 'size'); if (size !== null) { const n = legacySizeNumber(size); if (n) s.size = legacyPixels[n]; }
      }
      if (name === 'center' || (name && attr(el, 'align') !== null && isBlock(el))) { const a = name === 'center' ? 'center' : lower(attr(el, 'align')); if (a === 'center' || a === 'left' || a === 'right' || a === 'justify') s.align = a; }
      if (name === 'th') s.align = 'center';
      const d = declarations(el);
      for (let j = 0; j < d.length; j++) {
        const prop = d[j][0], value = d[j][1], v = lower(value);
        if (prop === 'font-weight') s.weight = weightNumber(value, s.weight);
        else if (prop === 'font-style') s.italic = v === 'italic' || v === 'oblique';
        else if (prop === 'text-decoration' || prop === 'text-decoration-line') { if (find(v, 'underline') >= 0) s.underline = true; if (find(v, 'line-through') >= 0) s.strike = true; }
        else if (prop === 'vertical-align') { if (v === 'sub' || v === 'super') s.script = v; }
        else if (prop === 'color') { const c = parseColor(value, false); if (c) s.color = c; }
        else if (prop === 'background-color' || prop === 'background') { const c = parseColor(value, false); if (c && c !== 'rgba(0, 0, 0, 0)') s.background = c; }
        else if (prop === 'font-family') s.family = value;
        else if (prop === 'font-size') { const px = cssPixels(v, s.size); if (px) s.size = px; }
        else if (prop === 'text-align' && isBlock(el)) s.align = v === 'start' ? 'left' : v === 'end' ? 'right' : v;
      }
    }
    return s;
  }
  function cssPixels(v, parent) {
    const keywords = { __proto__: null, 'xx-small': 9, 'x-small': 10, small: 13, medium: 16, large: 18, 'x-large': 24, 'xx-large': 32, 'xxx-large': 48, smaller: parent / 1.2, larger: parent * 1.2 };
    if (keywords[v]) return keywords[v];
    const n = NumberValue.parseFloat(v);
    if (n !== n) return 0;
    if (sub(v, v.length - 2) === 'px') return n;
    if (sub(v, v.length - 2) === 'em') return n * parent;
    if (sub(v, v.length - 3) === 'rem') return n * 16;
    if (sub(v, v.length - 1) === '%') return n * parent / 100;
    if (sub(v, v.length - 2) === 'pt') return n * 4 / 3;
    return 0;
  }
  const legacyPixels = [0, 10, 13, 16, 18, 24, 32, 48];
  const legacyKeywords = [null, 'x-small', 'small', 'medium', 'large', 'x-large', 'xx-large', 'xxx-large'];
  // HTMLFontElement::CssValueFromFontSizeNumber: optional sign, digits, clamped to 1..7.
  function legacySizeNumber(value) {
    let s = trim(value), i = 0, sign = 0;
    if (s[0] === '+') { sign = 1; i = 1; } else if (s[0] === '-') { sign = -1; i = 1; }
    let n = 0, digits = 0;
    while (i < s.length && code(s, i) >= 48 && code(s, i) <= 57) { n = n * 10 + code(s, i) - 48; i++; digits++; if (n > 1000) n = 1000; }
    if (!digits) return 0;
    if (sign) n = 3 + sign * n;
    return n < 1 ? 1 : n > 7 ? 7 : n;
  }
  function legacySizeOfPixels(px) {
    const table = [9, 10, 13, 16, 18, 24, 32, 48], p = MathFloor(px);
    for (let i = 1; i < 7; i++) if (p * 2 < table[i] + table[i + 1]) return i;
    return 7;
  }

  // ---- CSS colors ----------------------------------------------------------------------------------------
  const namedColors = record();
  (function () {
    const table = 'aliceblue f0f8ff antiquewhite faebd7 aqua 00ffff aquamarine 7fffd4 azure f0ffff beige f5f5dc bisque ffe4c4 black 000000 blanchedalmond ffebcd blue 0000ff blueviolet 8a2be2 brown a52a2a burlywood deb887 cadetblue 5f9ea0 chartreuse 7fff00 chocolate d2691e coral ff7f50 cornflowerblue 6495ed cornsilk fff8dc crimson dc143c cyan 00ffff darkblue 00008b darkcyan 008b8b darkgoldenrod b8860b darkgray a9a9a9 darkgreen 006400 darkgrey a9a9a9 darkkhaki bdb76b darkmagenta 8b008b darkolivegreen 556b2f darkorange ff8c00 darkorchid 9932cc darkred 8b0000 darksalmon e9967a darkseagreen 8fbc8f darkslateblue 483d8b darkslategray 2f4f4f darkslategrey 2f4f4f darkturquoise 00ced1 darkviolet 9400d3 deeppink ff1493 deepskyblue 00bfff dimgray 696969 dimgrey 696969 dodgerblue 1e90ff firebrick b22222 floralwhite fffaf0 forestgreen 228b22 fuchsia ff00ff gainsboro dcdcdc ghostwhite f8f8ff gold ffd700 goldenrod daa520 gray 808080 green 008000 greenyellow adff2f grey 808080 honeydew f0fff0 hotpink ff69b4 indianred cd5c5c indigo 4b0082 ivory fffff0 khaki f0e68c lavender e6e6fa lavenderblush fff0f5 lawngreen 7cfc00 lemonchiffon fffacd lightblue add8e6 lightcoral f08080 lightcyan e0ffff lightgoldenrodyellow fafad2 lightgray d3d3d3 lightgreen 90ee90 lightgrey d3d3d3 lightpink ffb6c1 lightsalmon ffa07a lightseagreen 20b2aa lightskyblue 87cefa lightslategray 778899 lightslategrey 778899 lightsteelblue b0c4de lightyellow ffffe0 lime 00ff00 limegreen 32cd32 linen faf0e6 magenta ff00ff maroon 800000 mediumaquamarine 66cdaa mediumblue 0000cd mediumorchid ba55d3 mediumpurple 9370db mediumseagreen 3cb371 mediumslateblue 7b68ee mediumspringgreen 00fa9a mediumturquoise 48d1cc mediumvioletred c71585 midnightblue 191970 mintcream f5fffa mistyrose ffe4e1 moccasin ffe4b5 navajowhite ffdead navy 000080 oldlace fdf5e6 olive 808000 olivedrab 6b8e23 orange ffa500 orangered ff4500 orchid da70d6 palegoldenrod eee8aa palegreen 98fb98 paleturquoise afeeee palevioletred db7093 papayawhip ffefd5 peachpuff ffdab9 peru cd853f pink ffc0cb plum dda0dd powderblue b0e0e6 purple 800080 rebeccapurple 663399 red ff0000 rosybrown bc8f8f royalblue 4169e1 saddlebrown 8b4513 salmon fa8072 sandybrown f4a460 seagreen 2e8b57 seashell fff5ee sienna a0522d silver c0c0c0 skyblue 87ceeb slateblue 6a5acd slategray 708090 slategrey 708090 snow fffafa springgreen 00ff7f steelblue 4682b4 tan d2b48c teal 008080 thistle d8bfd8 tomato ff6347 turquoise 40e0d0 violet ee82ee wheat f5deb3 white ffffff whitesmoke f5f5f5 yellow ffff00 yellowgreen 9acd32';
    let at = 0;
    while (at < table.length) { const a = find(table, ' ', at), b = find(table, ' ', a + 1), end = b < 0 ? table.length : b; namedColors[sub(table, at, a)] = sub(table, a + 1, end); at = end + 1; }
  })();
  const hexDigit = c => c >= 48 && c <= 57 ? c - 48 : c >= 97 && c <= 102 ? c - 87 : c >= 65 && c <= 70 ? c - 55 : -1;
  function rgbOf(value, legacy) {
    let v = lower(trim(value));
    if (v === 'transparent') return [0, 0, 0, 0];
    if (namedColors[v]) v = '#' + namedColors[v];
    else if (legacy && v[0] !== '#' && v.length === 6 && hexDigit(code(v, 0)) >= 0 && hexDigit(code(v, 1)) >= 0 && hexDigit(code(v, 2)) >= 0 && hexDigit(code(v, 3)) >= 0 && hexDigit(code(v, 4)) >= 0 && hexDigit(code(v, 5)) >= 0) v = '#' + v;
    if (v[0] === '#') {
      const h = sub(v, 1); const d = list();
      for (let i = 0; i < h.length; i++) { const x = hexDigit(code(h, i)); if (x < 0) return null; push(d, x); }
      if (d.length === 3 || d.length === 4) return [d[0] * 17, d[1] * 17, d[2] * 17, d.length === 4 ? d[3] * 17 / 255 : 1];
      if (d.length === 6 || d.length === 8) return [d[0] * 16 + d[1], d[2] * 16 + d[3], d[4] * 16 + d[5], d.length === 8 ? (d[6] * 16 + d[7]) / 255 : 1];
      return null;
    }
    const open = find(v, '(');
    if (open > 0 && v[v.length - 1] === ')') {
      const fn = sub(v, 0, open);
      if (fn !== 'rgb' && fn !== 'rgba') return null;
      let body = sub(v, open + 1, v.length - 1), parts = list(), cur = '';
      for (let i = 0; i < body.length; i++) { const c = body[i]; if (c === ',' || c === ' ' || c === '/') { if (cur) push(parts, cur); cur = ''; } else cur += c; }
      if (cur) push(parts, cur);
      if (parts.length !== 3 && parts.length !== 4) return null;
      const out = list();
      for (let i = 0; i < 3; i++) { const p = parts[i], pct = p[p.length - 1] === '%', n = NumberValue.parseFloat(p); if (n !== n) return null; push(out, MathMax(0, MathMin(255, MathRound(pct ? n * 2.55 : n)))); }
      let alpha = 1;
      if (parts.length === 4) { const p = parts[3], n = NumberValue.parseFloat(p); if (n !== n) return null; alpha = MathMax(0, MathMin(1, p[p.length - 1] === '%' ? n / 100 : n)); }
      push(out, alpha);
      return out;
    }
    return null;
  }
  function serializeRgb(c) {
    if (c[3] === 1) return 'rgb(' + c[0] + ', ' + c[1] + ', ' + c[2] + ')';
    let a = apply(toFixedF, MathRound(c[3] * 255) / 255, [3]);
    while (a.length > 1 && (a[a.length - 1] === '0')) a = sub(a, 0, a.length - 1);
    if (a[a.length - 1] === '.') a = sub(a, 0, a.length - 1);
    return 'rgba(' + c[0] + ', ' + c[1] + ', ' + c[2] + ', ' + a + ')';
  }
  const parseColor = (value, legacy) => { const c = rgbOf(value, legacy); return c ? serializeRgb(c) : null; };
  const hex2 = n => { const s = '0123456789abcdef'; return s[n >> 4] + s[n & 15]; };
  const hexColor = c => '#' + hex2(c[0]) + hex2(c[1]) + hex2(c[2]);

  // ---- style application (Blink ApplyStyleCommand) -----------------------------------------------------
  // A desired style is a record of CSS properties: font-weight, font-style, text-decoration-line
  // (a set: underline / line-through), vertical-align, color, background-color, font-family, font-size.
  function newStyle() { const s = record(); return s; }
  function styleIsEmpty(s) { for (const k in s) return false; return true; }
  // Does `el` carry one of the desired properties (tag equivalent, presentational or CSS)?
  function conflicts(style, el) {
    const name = nameOf(el);
    if (!name) return false;
    if ('font-weight' in style && (name === 'b' || name === 'strong')) return true;
    if ('font-style' in style && (name === 'i' || name === 'em')) return true;
    if ('text-decoration-line' in style && (name === 'u' || name === 's' || name === 'strike')) return true;
    if ('vertical-align' in style && (name === 'sub' || name === 'sup')) return true;
    if (name === 'font' && (('color' in style && hasAttr(el, 'color')) || ('font-family' in style && hasAttr(el, 'face')) || ('font-size' in style && hasAttr(el, 'size')))) return true;
    if ('link' in style && name === 'a') return true;
    const d = declarations(el);
    for (let i = 0; i < d.length; i++) {
      const p = d[i][0];
      if (p in style || (p === 'text-decoration' && 'text-decoration-line' in style)) return true;
    }
    return false;
  }
  function decorationRemoves(style, name) {
    const set = style['text-decoration-line'];
    if (name === 'u') return !set.underline;
    return !set.strike;
  }
  // Removes the desired properties from `el`; returns the element that remains in its place (or null).
  function removeStyleFrom(style, el) {
    const name = nameOf(el);
    let tag = false;
    if ('font-weight' in style && (name === 'b' || name === 'strong')) tag = true;
    if ('font-style' in style && (name === 'i' || name === 'em')) tag = true;
    if ('text-decoration-line' in style && (name === 'u' || name === 's' || name === 'strike') && decorationRemoves(style, name)) tag = true;
    if ('vertical-align' in style && (name === 'sub' || name === 'sup')) tag = true;
    if ('link' in style && name === 'a') { unwrap(el); return null; }
    if (name === 'font') {
      if ('color' in style) removeAttr(el, 'color');
      if ('font-family' in style) removeAttr(el, 'face');
      if ('font-size' in style) removeAttr(el, 'size');
    }
    const d = declarations(el);
    if (d.length) {
      const keep = list(); let changed = false;
      for (let i = 0; i < d.length; i++) {
        const p = d[i][0];
        if (p in style || (p === 'text-decoration' && 'text-decoration-line' in style)) { changed = true; continue; }
        push(keep, [p, normalizeValue(p, d[i][1])]);
      }
      if (changed) setAttr(el, 'style', serializeDeclarations(keep));
    }
    if (tag) return replaceWithSpanOrRemove(el);
    if ((name === 'span' || name === 'font') && unstyled(el)) { unwrap(el); return null; }
    return el;
  }
  function unstyled(el) {
    const attrs = attributeList(el);
    for (let i = 0; i < attrs.length; i++) { if (attrs[i][0] !== 'style' || trim(attrs[i][1]) !== '') return false; }
    return true;
  }
  function normalizeValue(prop, value) {
    if (prop === 'color' || prop === 'background-color') { const c = rgbOf(value, false); if (c && lower(trim(value))[0] !== '#' && find(lower(value), '(') >= 0) return serializeRgb(c); }
    return value;
  }
  // The highest inline ancestor of `node` (below its block) carrying a conflicting style.
  function highestConflicting(style, node) {
    let result = null;
    for (let n = parentOf(node); n !== null && isElement(n) && !isBlock(n) && isEditable(n) && rootEditable(n) !== n; n = parentOf(n)) if (conflicts(style, n)) result = n;
    return result;
  }
  // Blink's PushDownInlineStyleAroundNode: removes conflicting ancestors of `target` and re-wraps the
  // siblings off the path with clones of the removed elements.
  function pushDown(style, target) {
    const highest = highestConflicting(style, target);
    if (highest === null) return;
    let current = highest;
    const pushed = list();
    while (current !== null && current !== target && contains(current, target)) {
      const children = list();
      for (let c = firstOf(current); c !== null; c = nextOf(c)) push(children, c);
      let styled = false;
      if (conflicts(style, current)) { push(pushed, current); styled = true; }
      const clones = list();
      for (let i = 0; i < pushed.length; i++) { const c = cloneShallow(pushed[i]); push(clones, c); }
      if (styled) {
        const replaced = removeStyleFrom(style, current);
        if (replaced !== null) current = replaced;
      }
      let next = null;
      for (let i = 0; i < children.length; i++) {
        const child = children[i];
        if (parentOf(child) === null) continue;
        if (!contains(child, target) && pushed.length) {
          for (let j = 0; j < pushed.length; j++) {
            const wrapper = cloneShallow(pushed[j]);
            if (hasAttr(wrapper, 'style')) apply(removeAttrF, wrapper, ['style']);
            removeAttr(pushed[j], 'id');
            const d = declarations(pushed[j]), keep = list();
            for (let k = 0; k < d.length; k++) if (d[k][0] in style) push(keep, [d[k][0], d[k][1]]);
            if (keep.length) apply(setAttrF, wrapper, ['style', attr(pushed[j], 'style')]);
            surround(child, child, wrapper);
          }
        }
        if (child === target || contains(child, target)) next = child;
      }
      if (next === null) break;
      if (styled && isElement(next) && next !== target && !hasAttr(next, 'style')) setAttr(next, 'style', '');
      current = next;
    }
  }
  // Blink's SurroundNodeRangeWithElement, merging with identical neighbours.
  function surround(startNode, endNode, element) {
    insertNode(element, parentOf(startNode), startNode);
    for (let n = startNode; n !== null;) { const next = nextOf(n); moveNode(n, element, null); if (n === endNode) break; n = next; }
    const nextSibling = nextOf(element), previousSibling = prevOf(element);
    if (nextSibling !== null && isElement(nextSibling) && isEditable(nextSibling) && identical(element, nextSibling)) { mergeIdentical(element, nextSibling); element = nextSibling; }
    if (previousSibling !== null && isElement(previousSibling) && isEditable(previousSibling) && identical(previousSibling, element)) mergeIdentical(previousSibling, element);
    return element;
  }
  // The changes `style` needs at `node` given its current inherited style.
  function styleChange(style, node, css) {
    const at = styleAt(node), change = record(), cssOut = list();
    change.tags = list(); change.font = null; change.css = '';
    if ('font-weight' in style) {
      const want = weightNumber(style['font-weight'], at.weight) >= 600, have = at.weight >= 600;
      if (want !== have) { if (want && !css) push(change.tags, 'b'); else push(cssOut, ['font-weight', want ? 'bold' : 'normal']); }
    }
    if ('font-style' in style) {
      const want = style['font-style'] === 'italic';
      if (want !== at.italic) { if (want && !css) push(change.tags, 'i'); else push(cssOut, ['font-style', want ? 'italic' : 'normal']); }
    }
    if ('text-decoration-line' in style) {
      const set = style['text-decoration-line'];
      const addU = set.underline && !at.underline, addS = set.strike && !at.strike;
      if (css) { if (addU || addS) push(cssOut, ['text-decoration-line', addU && addS ? 'underline line-through' : addU ? 'underline' : 'line-through']); }
      else { if (addU) push(change.tags, 'u'); if (addS) push(change.tags, 'strike'); }
    }
    if ('vertical-align' in style) {
      const want = style['vertical-align'];
      if (want !== at.script) { if (!css && want === 'sub') push(change.tags, 'sub'); else if (!css && want === 'super') push(change.tags, 'sup'); else push(cssOut, ['vertical-align', want]); }
    }
    const fontAttrs = list();
    if ('color' in style) {
      const want = parseColor(style.color, false), have = at.color || defaultColor();
      if (want !== null && want !== have) { if (!css) push(fontAttrs, ['color', hexColor(rgbOf(style.color, false))]); else push(cssOut, ['color', style.color]); }
    }
    if ('font-family' in style) {
      const want = style['font-family'], have = at.family || defaultFamily();
      if (want !== have) { if (!css) push(fontAttrs, ['face', want]); else push(cssOut, ['font-family', want]); }
    }
    if ('font-size' in style) {
      const want = style['font-size'];
      if (legacyPixels[want] !== at.size) { if (!css) push(fontAttrs, ['size', StringValue(want)]); else push(cssOut, ['font-size', legacyKeywords[want]]); }
    }
    if ('background-color' in style) {
      const want = parseColor(style['background-color'], false);
      if (want !== null && want !== 'rgba(0, 0, 0, 0)' && want !== at.background) push(cssOut, ['background-color', style['background-color']]);
    }
    if (fontAttrs.length) change.font = fontAttrs;
    if (cssOut.length) change.css = serializeDeclarations(cssOut);
    return change;
  }
  let defaults = null;
  function defaultStyle() {
    if (defaults === null) {
      defaults = record();
      const root = rootElementOf(document);
      const family = root !== null ? computed(root, 'font-family') : '';
      const color = root !== null ? parseColor(computed(root, 'color'), false) : null;
      defaults.family = family || 'Times';
      defaults.color = color || 'rgb(0, 0, 0)';
    }
    return defaults;
  }
  const defaultFamily = () => defaultStyle().family;
  const defaultColor = () => defaultStyle().color;
  function changeIsEmpty(change) { return !change.tags.length && change.font === null && !change.css && !change.link; }
  // Blink's ApplyInlineStyleChange: font element first (outermost), then a style span, then tags.
  function applyChange(startNode, endNode, change, doc) {
    let fontContainer = null, styleContainer = null, s = startNode, e = endNode;
    for (let container = s; container !== null && s === e; container = firstOf(container)) {
      if (nameOf(container) === 'font') fontContainer = container;
      const name = nameOf(container);
      if (name && (name === 'span' || ((styleContainer === null || nameOf(styleContainer) !== 'span') && firstOf(container) !== null))) styleContainer = container;
      if (firstOf(container) === null) break;
      s = firstOf(container); e = lastOf(container);
    }
    if (change.font !== null) {
      if (fontContainer !== null) { for (let i = 0; i < change.font.length; i++) setAttr(fontContainer, change.font[i][0], change.font[i][1]); }
      else { const font = createElement(doc, 'font'); for (let i = 0; i < change.font.length; i++) apply(setAttrF, font, [change.font[i][0], change.font[i][1]]); surround(s, e, font); }
    }
    if (change.css) {
      if (styleContainer !== null && nameOf(styleContainer) === 'span') {
        const existing = attr(styleContainer, 'style');
        setAttr(styleContainer, 'style', existing !== null && trim(existing) !== '' ? serializeDeclarations(declarations(styleContainer)) + ' ' + change.css : change.css);
      } else { const span = createElement(doc, 'span'); apply(setAttrF, span, ['style', change.css]); surround(s, e, span); }
    }
    for (let i = 0; i < change.tags.length; i++) surround(s, e, createElement(doc, change.tags[i]));
    if (change.link) { const a = createElement(doc, 'a'); apply(setAttrF, a, ['href', change.link]); surround(s, e, a); }
  }
  const nodeAfter = node => { if (firstOf(node) !== null) return firstOf(node); for (let n = node; n !== null; n = parentOf(n)) { if (nextOf(n) !== null) return nextOf(n); } return null; };
  const nodeAfterSkipping = node => { for (let n = node; n !== null; n = parentOf(n)) { if (nextOf(n) !== null) return nextOf(n); } return null; };
  function caretMax(node) { return isText(node) ? dataOf(node).length : lengthOf(node); }
  function visiblyContained(node, start, end) {
    const first = P(node, 0), last = P(node, lengthOf(node));
    const startIn = compare(start, first) <= 0 || same(canonicalForward(first), canonicalForward(start));
    const endIn = compare(last, end) <= 0 || same(canonical(last), canonical(end));
    return startIn && endIn;
  }
  // Blink's ApplyStyleCommand::ApplyInlineStyle over [start, end].
  function applyInlineStyle(doc, style, start, end, css) {
    let s = start, e = end;
    if (styleIsEmpty(style)) return [s, e];
    const inText = p => isText(p.c) && p.o > 0 && p.o < dataOf(p.c).length;
    const splitStart = inText(s);
    if (splitStart) {
      const node = s.c, parent = parentOf(node), sameNode = e.c === node;
      splitText(node, s.o);
      if (sameNode) e = P(node, e.o - s.o);
      s = P(node, 0);
      if (isElement(parent) && conflicts(style, parent) && rootEditable(parent) !== parent) splitElement(parent, node);
    }
    const splitEnd = inText(e);
    if (splitEnd) {
      const node = e.c, parent = parentOf(node);
      const prefix = splitText(node, e.o);
      if (s.c === node) s = P(prefix, s.o);
      e = P(prefix, dataOf(prefix).length);
      if (isElement(parent) && conflicts(style, parent) && rootEditable(parent) !== parent) splitElement(parent, node);
    }
    removeInlineStyle(style, s, e);
    if (splitStart) { const merged = mergeStartWithPrevious(s, e); s = merged[0]; e = merged[1]; }
    if (splitEnd) { const merged = mergeEndWithNext(s, e); s = merged[0]; e = merged[1]; }
    const range = fixRangeAndApply(doc, style, s, e, css);
    return range || [s, e];
  }
  function removeInlineStyle(style, start, end) {
    pushDown(style, start.c);
    pushDown(style, end.c);
    const endNode = end.c;
    for (let node = start.c; node !== null;) {
      let next = nodeAfter(node);
      if (isElement(node) && nameOf(node) && isEditable(node) && rootEditable(node) !== node && visiblyContained(node, start, end) && conflicts(style, node)) {
        const last = node === endNode || contains(node, endNode);
        removeStyleFrom(style, node);
        if (last) break;
      }
      if (node === endNode) break;
      node = next;
    }
  }
  function mergeStartWithPrevious(s, e) {
    let node = s.c;
    if (s.o !== 0) return [s, e];
    if (isText(node) || firstOf(node) === null) { if (prevOf(node) !== null) return [s, e]; node = parentOf(node); }
    const previous = prevOf(node);
    if (previous !== null && isElement(previous) && identical(node, previous) && isEditable(previous)) {
      const startChild = firstOf(node);
      mergeIdentical(previous, node);
      if (startChild === null) return [s, e];
      return [s.c === startChild || isText(s.c) ? s : P(node, indexOf(startChild)), e];
    }
    return [s, e];
  }
  function mergeEndWithNext(s, e) {
    let node = e.c;
    if (isText(node) || firstOf(node) === null) { if (e.o < caretMax(node)) return [s, e]; if (nextOf(node) !== null) return [s, e]; node = parentOf(node); }
    if (!isElement(node) || nameOf(node) === 'br') return [s, e];
    const next = nextOf(node);
    if (next !== null && isElement(next) && identical(node, next) && isEditable(next)) { mergeIdentical(node, next); }
    return [s, e];
  }
  function fixRangeAndApply(doc, style, start, end, css) {
    let startNode = start.c;
    if (start.o >= caretMax(startNode)) { startNode = nodeAfter(startNode); if (startNode === null || compare(end, P(parentOf(startNode), indexOf(startNode))) < 0) return null; }
    let pastEnd = end.c;
    if (end.o >= caretMax(end.c)) pastEnd = nodeAfterSkipping(end.c);
    else if (!isText(end.c)) pastEnd = childAt(end.c, end.o) || nodeAfterSkipping(end.c);
    const root = rootEditable(startNode);
    if (startNode !== root) {
      while (root !== null && parentOf(startNode) !== root && parentOf(startNode) !== null && visiblyContained(parentOf(startNode), start, end)) startNode = parentOf(startNode);
    }
    applyToNodeRange(doc, style, startNode, pastEnd, css);
    return null;
  }
  function applyToNodeRange(doc, style, startNode, pastEnd, css) {
    const runs = list();
    for (let node = startNode, next; node !== null && node !== pastEnd; node = next) {
      next = nodeAfter(node);
      if (!isEditable(node) || (isElement(node) && displayOf(node) === 'none')) continue;
      if (isElement(node) && isBlock(node)) continue;
      if (isElement(node) && firstOf(node) !== null) {
        if ((pastEnd !== null && contains(node, pastEnd)) || !isEditable(parentOf(node))) continue;
        if (atomicNames[nameOf(node)]) { next = nodeAfterSkipping(node); continue; }
      }
      const runStart = node; let runEnd = node;
      for (let sibling = nextOf(node); sibling !== null && sibling !== pastEnd && !(pastEnd !== null && contains(sibling, pastEnd)) && (!isBlock(sibling) || contains(sibling, runEnd)); sibling = nextOf(runEnd)) runEnd = sibling;
      next = nodeAfterSkipping(runEnd);
      push(runs, [runStart, runEnd]);
    }
    for (let i = 0; i < runs.length; i++) {
      const run = runs[i];
      if (parentOf(run[0]) === null) continue;
      const position = firstTextIn(run[0], run[1]) || run[0];
      const change = styleChange(style, position, css);
      if ('link' in style) change.link = style.link;
      if (changeIsEmpty(change)) continue;
      applyChange(run[0], run[1], change, doc);
    }
  }
  function firstTextIn(startNode, endNode) {
    for (let n = startNode; n !== null;) {
      if (isText(n)) return n;
      if (atomicNames[nameOf(n)]) return n;
      const nn = nodeAfter(n);
      if (n === endNode && !contains(n, nn)) return null;
      n = nn;
      if (n !== null && !contains(startNode, n) && nextOf(endNode) !== null && n === nextOf(endNode)) return null;
    }
    return null;
  }

  // ---- whitespace (Blink RebalanceWhitespace) --------------------------------------------------------
  const isSpace = c => c === 32 || c === 0xa0 || c === 10 || c === 9;
  // Chrome 156 alternates nbsp/space starting with nbsp; a lone inner space stays a space and a run
  // ending a paragraph ends with nbsp.
  function rebalanced(run, startOfParagraph, endOfParagraph) {
    const n = run.length;
    if (n === 1 && !startOfParagraph && !endOfParagraph) return ' ';
    let out = '';
    for (let i = 0; i < n; i++) out += i % 2 === 0 ? NBSP : ' ';
    if (endOfParagraph && out[n - 1] === ' ') out = sub(out, 0, n - 1) + NBSP;
    return out;
  }
  function collapses(textNode) {
    let ws = 'normal';
    for (let n = parentOf(textNode); n !== null && typeOf(n) === 1; n = parentOf(n)) { const own = whiteSpaceOf(n, null); if (own !== null) { ws = own; break; } }
    return preserveMode(ws) === 0;
  }
  function rebalanceSubstring(node, startOffset, endOffset) {
    if (!isText(node) || !collapses(node)) return;
    const text = dataOf(node);
    if (!text.length) return;
    let up = startOffset, down = endOffset;
    while (up > 0 && isSpace(code(text, up - 1))) up--;
    while (down < text.length && isSpace(code(text, down))) down++;
    if (down === up) return;
    const run = sub(text, up, down), result = rebalanced(run, up === 0, down === text.length);
    if (run !== result) replaceText(node, up, down - up, result);
  }
  const rebalanceAt = pos => { if (pos !== null && isText(pos.c)) rebalanceSubstring(pos.c, pos.o, pos.o); };

  // ---- typing style --------------------------------------------------------------------------------
  function typingStyleFor(editor, sel) {
    const t = editor.typing;
    if (t === null || sel === null || !sel.caret || !same(t.at, sel.start) || t.range !== currentRange(editor.doc)) return null;
    return t.style;
  }
  // Removes properties already in effect at the caret (EditingStyle::PrepareToApplyAt).
  function prepareStyleAt(style, node) {
    const at = styleAt(node), out = record();
    for (const key in style) {
      const v = style[key];
      // Blink compares the keyword with the computed numeric weight, which never matches.
      if (key === 'font-weight') out[key] = v;
      else if (key === 'font-style') { if ((v === 'italic') !== at.italic) out[key] = v; }
      else if (key === 'text-decoration-line') { if (v.underline !== at.underline || v.strike !== at.strike) out[key] = v; }
      else if (key === 'vertical-align') { if (v !== at.script) out[key] = v; }
      else if (key === 'color') { if (parseColor(v, false) !== (at.color || defaultColor())) out[key] = v; }
      else if (key === 'font-family') { if (v !== (at.family || defaultFamily())) out[key] = v; }
      else if (key === 'font-size') { if (legacyPixels[v] !== at.size) out[key] = v; }
      else if (key === 'background-color') { if (parseColor(v, false) !== at.background) out[key] = v; }
      else out[key] = v;
    }
    return out;
  }
  // The inline style at `node` expressed as a desired style (for type-over after a delete).
  function styleRecordAt(node) {
    const at = styleAt(node), out = record();
    if (at.weight >= 600) out['font-weight'] = 'bold';
    if (at.italic) out['font-style'] = 'italic';
    if (at.underline || at.strike) { const d = record(); d.underline = at.underline && !at.link; d.strike = at.strike; if (d.underline || d.strike) out['text-decoration-line'] = d; }
    if (at.script !== 'baseline') out['vertical-align'] = at.script;
    if (at.color !== null && !at.link) out.color = at.color;
    if (at.family !== null) out['font-family'] = at.family;
    if (at.size !== 16) { const n = legacySizeOfPixels(at.size); if (legacyPixels[n] === at.size) out['font-size'] = n; }
    if (at.background !== null) out['background-color'] = at.background;
    return out;
  }

  // ---- deletion (Blink DeleteSelectionCommand, simplified) ------------------------------------------
  function hasRenderedContent(block) {
    const items = streamOf(block);
    for (let i = 0; i < items.length; i++) { const t = items[i].t; if (t === CHAR || t === ATOM || t === BATOM || t === BR) return true; }
    return false;
  }
  function removeAndPrune(node, stop) { const parent = parentOf(node); removeNode(node); if (parent !== null) pruneEmptyAncestors(parent, stop); }
  function deleteRange(doc, root, start, end, merge) {
    const editor = editorOf(doc);
    // Type-over keeps the style of the deleted start that differs from the surviving context.
    const first = canonicalForward(start) || start;
    const deletedStyle = styleRecordAt(first.c);
    // Blink deletes from the upstream start to the downstream end, taking collapsed whitespace along.
    start = canonical(start) || start; end = canonicalForward(end) || end;
    const startBlock = blockOf(start.c, root), endBlock = blockOf(end.c, root);
    const caret = mark(start), endMark = mark(end);
    if (start.c === end.c && isText(start.c)) {
      deleteText(start.c, start.o, end.o - start.o);
      if (!dataOf(start.c).length) removeAndPrune(start.c, startBlock);
    } else {
      const doomed = list();
      for (let n = isText(start.c) ? nodeAfterSkipping(start.c) : (childAt(start.c, start.o) || nodeAfterSkipping(start.c)); n !== null;) {
        const b = before(n), a = after(n);
        if (compare(b, end) >= 0) break;
        if (compare(start, b) <= 0 && compare(a, end) <= 0) { push(doomed, n); n = nodeAfterSkipping(n); continue; }
        n = nodeAfter(n);
      }
      if (isText(start.c)) deleteText(start.c, start.o, dataOf(start.c).length - start.o);
      if (isText(end.c)) deleteText(end.c, 0, end.o);
      for (let i = 0; i < doomed.length; i++) if (parentOf(doomed[i]) !== null) removeNode(doomed[i]);
      if (isText(end.c) && !dataOf(end.c).length && parentOf(end.c) !== null) removeAndPrune(end.c, endBlock);
      if (isText(start.c) && !dataOf(start.c).length && parentOf(start.c) !== null) removeAndPrune(start.c, startBlock);
      else if (isElement(start.c)) pruneEmptyAncestors(start.c, startBlock);
      if (merge && startBlock !== endBlock && connectedOf(endBlock)) {
        const from = marked(endMark), to = marked(caret);
        const joined = moveParagraph(doc, root, from, to, endBlock);
        if (joined) { caret.c = joined.c; caret.o = joined.o; }
        let block = endBlock;
        while (block !== null && block !== root && connectedOf(block) && !hasRenderedContent(block) && !contains(block, marked(caret).c)) { const parent = parentOf(block); removeNode(block); block = parent; }
      }
    }
    let at = marked(caret);
    const block = blockOf(at.c, root);
    if (!hasRenderedContent(block)) {
      const br = createElement(doc, 'br');
      insertAt(br, at);
      at = before(br);
    } else {
      const c = canonical(at);
      if (c !== null) at = c;
      rebalanceAt(at);
      at = canonical(marked(mark(at))) || at;
    }
    const kept = prepareStyleAt(deletedStyle, at.c);
    if (!styleIsEmpty(kept) && richly(at.c)) editor.pendingTyping = kept;
    return at;
  }
  // Moves the paragraph starting at `from` (to the next paragraph boundary) to `to`, merging text.
  function moveParagraph(doc, root, from, to, sourceBlock) {
    let first;
    if (isText(from.c)) {
      if (from.o >= dataOf(from.c).length) first = nodeAfterSkipping(from.c);
      else { if (from.o > 0) splitText(from.c, from.o); first = from.c; }
    } else first = childAt(from.c, from.o) || (from.c === sourceBlock ? null : nodeAfterSkipping(from.c));
    if (first === null || !contains(sourceBlock, first)) return null;
    let top = first;
    while (parentOf(top) !== sourceBlock && parentOf(top) !== null) { const parent = parentOf(top); if (prevOf(top) !== null) splitElement(parent, top); top = parent; }
    const nodes = list();
    for (let n = top; n !== null; n = nextOf(n)) {
      if (nameOf(n) === 'br') { removeNode(n); break; }
      if (isBlock(n)) break;
      push(nodes, n);
    }
    if (!nodes.length) return null;
    let target = to;
    if (isText(target.c) && target.o > 0 && target.o < dataOf(target.c).length) { splitText(target.c, target.o); target = before(target.c); }
    let anchorParent, anchorRef;
    if (isText(target.c)) { anchorParent = parentOf(target.c); anchorRef = target.o === 0 ? target.c : nextOf(target.c); }
    else { anchorParent = target.c; anchorRef = childAt(target.c, target.o); }
    for (let i = 0; i < nodes.length; i++) moveNode(nodes[i], anchorParent, anchorRef);
    const head = nodes[0];
    if (isText(head)) { const previous = prevOf(head), offset = previous !== null && isText(previous) ? dataOf(previous).length : 0; mergeTextsAround(head); return P(head, offset); }
    return before(head);
  }
  // Blink's ReplaceSelectionCommand::MergeTextNodesAroundPosition for a text node.
  function mergeTextsAround(node) {
    if (!isText(node)) return node;
    const previous = prevOf(node);
    if (previous !== null && isText(previous)) { insertText(node, 0, dataOf(previous)); removeNode(previous); }
    const next = nextOf(node);
    if (next !== null && isText(next)) { insertText(node, dataOf(node).length, dataOf(next)); removeNode(next); }
    return node;
  }
  // Backspace boundaries: a code point, keeping emoji sequences (ZWJ, variation selectors, flags,
  // keycaps, modifiers) together.
  function backwardBoundary(text, offset) {
    const cp = at => { const c = code(text, at - 1); if (c >= 0xdc00 && c <= 0xdfff && at >= 2) { const h = code(text, at - 2); if (h >= 0xd800 && h <= 0xdbff) return [((h - 0xd800) << 10) + (c - 0xdc00) + 0x10000, 2]; } return [c, 1]; };
    let at = offset;
    let [c, n] = cp(at); at -= n;
    const regional = x => x >= 0x1f1e6 && x <= 0x1f1ff;
    if (c === 0xfe0f || c === 0x20e3 || (c >= 0x1f3fb && c <= 0x1f3ff)) { while (at > 0) { const [p, m] = cp(at); at -= m; if (p !== 0xfe0f && p !== 0x20e3 && !(p >= 0x1f3fb && p <= 0x1f3ff)) break; } }
    while (at > 0) { const [p, m] = cp(at); if (p !== 0x200d) break; at -= m; if (at <= 0) break; const [q, k] = cp(at); at -= k; if (q === 0xfe0f && at > 0) { const [r, j] = cp(at); at -= j; } }
    if (regional(c) && at > 0) { let count = 0, scan = at; while (scan > 0) { const [p, m] = cp(scan); if (!regional(p)) break; count++; scan -= m; } if (count % 2 === 1) at -= 2; }
    return at;
  }
  function forwardBoundary(text, offset) {
    let at = offset;
    const cpAt = i => { const c = code(text, i); if (c >= 0xd800 && c <= 0xdbff && i + 1 < text.length) { const l = code(text, i + 1); if (l >= 0xdc00 && l <= 0xdfff) return [((c - 0xd800) << 10) + (l - 0xdc00) + 0x10000, 2]; } return [c, 1]; };
    const extend = c => (c >= 0x300 && c <= 0x36f) || (c >= 0x1ab0 && c <= 0x1aff) || (c >= 0x1dc0 && c <= 0x1dff) || (c >= 0x20d0 && c <= 0x20ff) || (c >= 0xfe20 && c <= 0xfe2f) || c === 0xfe0f || c === 0xfe0e || c === 0x200d || (c >= 0x1f3fb && c <= 0x1f3ff) || (c >= 0xe0020 && c <= 0xe007f);
    let [c, n] = cpAt(at); at += n;
    if (code(text, at - n) === 13 && code(text, at) === 10) return at + 1;
    let joined = false;
    while (at < text.length) { const [p, m] = cpAt(at); if (extend(p)) { at += m; joined = p === 0x200d; continue; } if (joined) { at += m; joined = false; continue; } break; }
    if (c >= 0x1f1e6 && c <= 0x1f1ff && at < text.length) { const [p, m] = cpAt(at); if (p >= 0x1f1e6 && p <= 0x1f1ff) at += m; }
    return at;
  }
  // TypingCommand::DeleteKeyPressed / ForwardDeleteKeyPressed for a caret: returns the range to delete.
  function deletionRange(root, caret, forward) {
    const items = streamOf(root), k = gapOf(items, caret);
    const j = forward ? nextGap(items, k) : prevGap(items, k);
    if (j < 0) return null;
    const lo = forward ? k : j, hi = forward ? j : k;
    let hasBlock = false;
    for (let i = lo; i < hi; i++) if (items[i].t === BS || items[i].t === BE) hasBlock = true;
    if (!hasBlock && hi - lo === 1 && items[lo].t === CHAR) {
      const item = items[lo], text = dataOf(item.n);
      if (forward) { const s = P(item.n, item.o); return [s, P(item.n, forwardBoundary(text, item.o)), false]; }
      const e = caret.c === item.n ? caret : P(item.n, item.o + 1);
      return [P(item.n, backwardBoundary(text, item.o + 1)), e, false];
    }
    const start = forward ? caret : (items[lo].t === BE || items[lo].t === BS ? candidate(items, lo) : itemStart(items[lo]));
    const end = forward ? (items[hi - 1].t === BE || items[hi - 1].t === BS ? candidate(items, hi) : itemEnd(items[hi - 1])) : caret;
    return [start, end, hasBlock];
  }

  // ---- insertion -----------------------------------------------------------------------------------
  function isPlaceholder(items, k) { return k < items.length && items[k].t === BR && items[k].o < 0 && startOfParagraph(items, k) && endOfBlock(items, k); }
  // Blink's InsertTextCommand for one run without newlines; returns the caret after the text.
  function insertTextRun(doc, root, sel, text, typing) {
    const editor = editorOf(doc);
    let start;
    if (!sel.caret) {
      if (typing === null && sel.start.c === sel.end.c && isText(sel.start.c) && find(text, ' ') < 0 && find(text, '\t') < 0 && find(text, '\n') < 0 && text.length) {
        replaceText(sel.start.c, sel.start.o, sel.end.o - sel.start.o, text);
        return P(sel.start.c, sel.start.o + text.length);
      }
      start = deleteRange(doc, root, sel.start, sel.end, true);
      if (editor.pendingTyping) { typing = mergeStyles(editor.pendingTyping, typing); editor.pendingTyping = null; }
    } else start = sel.start;
    if (!text.length) return start;
    const items = streamOf(root), k = gapOf(items, start);
    if (isPlaceholder(items, k)) { const br = items[k].n; start = upstream(items, k) || before(br); const m = mark(start); removeNode(br); start = marked(m); }
    start = canonical(start) || start;
    if (text === '\t' && richly(start.c)) return insertTab(doc, start);
    if (!isText(start.c)) { const node = createText(doc, ''); insertAt(node, start); start = P(node, 0); }
    const node = start.c, offset = start.o;
    insertText(node, offset, text);
    let end = P(node, offset + text.length);
    rebalanceAt(end);
    let onlySpace = true;
    for (let i = 0; i < text.length; i++) if (!isSpace(code(text, i))) onlySpace = false;
    if (!onlySpace) rebalanceAt(P(node, offset));
    end = P(node, offset + text.length);
    if (typing !== null && !styleIsEmpty(typing)) {
      const range = applyInlineStyle(doc, typing, P(node, offset), end, editor.css);
      end = range[1];
    }
    return end;
  }
  function mergeStyles(base, over) { const out = record(); for (const k in base) out[k] = base[k]; if (over) for (const k in over) out[k] = over[k]; return out; }
  function insertTab(doc, pos) {
    const span = createElement(doc, 'span'), tab = createText(doc, '\t');
    apply(setAttrF, span, ['style', 'white-space:pre']);
    apply(insertBeforeF, span, [tab, null]);
    insertAt(span, pos);
    return P(tab, 1);
  }
  // Blink's InsertLineBreakCommand.
  function insertLineBreak(doc, root, caret) {
    const items = streamOf(root), k = gapOf(items, caret);
    let pos = candidate(items, k) || caret;
    const plain = !richly(pos.c);
    const make = () => plain ? createText(doc, '\n') : createElement(doc, 'br');
    let node = make();
    if (endOfParagraph(items, k) && !lineBreakAt(items, k)) {
      insertAt(node, pos);
      const extra = plain ? createText(doc, '\n') : createElement(doc, 'br');
      insertAfter(extra, node);
      return before(extra);
    }
    if (isText(pos.c) ? pos.o === 0 : true) {
      insertAt(node, pos);
      return after(node);
    }
    if (pos.o >= dataOf(pos.c).length) { insertAfter(node, pos.c); return after(node); }
    splitText(pos.c, pos.o);
    insertNode(node, parentOf(pos.c), pos.c);
    return P(pos.c, 0);
  }
  const formatBlockNames = { __proto__: null, address: 1, article: 1, aside: 1, blockquote: 1, dd: 1, div: 1, dl: 1, dt: 1, footer: 1, h1: 1, h2: 1, h3: 1, h4: 1, h5: 1, h6: 1, header: 1, hgroup: 1, main: 1, nav: 1, p: 1, pre: 1, section: 1 };
  const headingNames = { __proto__: null, h1: 1, h2: 1, h3: 1, h4: 1, h5: 1, h6: 1 };
  function ancestorsInsideBlock(node, block) { const out = list(); for (let n = isElement(node) ? node : parentOf(node); n !== null && n !== block; n = parentOf(n)) push(out, n); return out; }
  function cloneHierarchy(ancestors, block) { let parent = block; for (let i = ancestors.length - 1; i >= 0; i--) { const c = cloneShallow(ancestors[i]); apply(removeAttrF, c, ['id']); appendNode(c, parent); parent = c; } return parent; }
  function listChild(node, root) { for (let n = node; n !== null && n !== root; n = parentOf(n)) { const name = nameOf(n); if (name === 'li' || name === 'dd' || name === 'dt') return n; } return null; }
  // Blink's InsertParagraphSeparatorCommand.
  function insertParagraph(doc, root, caret) {
    const editor = editorOf(doc);
    let items = streamOf(root), k = gapOf(items, caret);
    const pos = candidate(items, k) || caret;
    const startBlock = blockOf(pos.c, root);
    const sname = nameOf(startBlock);
    if (sname === 'td' || sname === 'th' || sname === 'form' || sname === 'pre' || sname === 'listing' || sname === 'xmp' || parentOf(startBlock) === null) return insertLineBreak(doc, root, caret);
    // Breaking out of an empty list item.
    const li = listChild(pos.c, root);
    if (li !== null && nameOf(li) === 'li' && !hasRenderedContentExceptPlaceholder(li)) {
      const listEl = parentOf(li);
      if (listEl !== null && (nameOf(listEl) === 'ul' || nameOf(listEl) === 'ol') && nextOf(li) === null) {
        const block = createElement(doc, editor.separator);
        insertAfter(block, listEl);
        removeNode(li);
        if (firstOf(listEl) === null) removeNode(listEl);
        appendNode(createElement(doc, 'br'), block);
        return P(block, 0);
      }
    }
    const first = startOfBlock(items, k), last = endOfBlock(items, k);
    const isRoot = startBlock === root;
    let blockToInsert, nest = false;
    if (isRoot) { blockToInsert = createElement(doc, editor.separator); nest = true; }
    else if (last && headingNames[sname]) blockToInsert = createElement(doc, editor.separator);
    else blockToInsert = cloneShallow(startBlock);
    if (last) {
      if (nest) {
        if (first && !lineBreakAt(items, k)) { const extra = createElement(doc, editor.separator); appendNode(extra, startBlock); appendNode(createElement(doc, 'br'), extra); }
        appendNode(blockToInsert, startBlock);
      } else insertAfter(blockToInsert, startBlock);
      const parent = cloneHierarchy(ancestorsInsideBlock(pos.c, startBlock), blockToInsert);
      appendNode(createElement(doc, 'br'), parent);
      return P(parent, 0);
    }
    if (first) {
      let ref;
      if (!nest) ref = startBlock;
      else ref = firstOf(startBlock);
      const insertion = canonicalForward(pos) || pos;
      insertNode(blockToInsert, parentOf(ref), ref);
      const parent = cloneHierarchy(ancestorsInsideBlock(insertion.c, startBlock), blockToInsert);
      appendNode(createElement(doc, 'br'), parent);
      return insertion;
    }
    // General case: split at the caret and move the rest of the paragraph's block content.
    let at = canonicalForward(pos) || pos;
    if (isText(at.c) && at.o > 0 && at.o < dataOf(at.c).length) { splitText(at.c, at.o); at = P(at.c, 0); }
    else if (isText(at.c) && at.o >= dataOf(at.c).length) { const n = nodeAfterSkipping(at.c); at = n !== null && contains(startBlock, n) ? before(n) : P(startBlock, lengthOf(startBlock)); }
    if (nest) appendNode(blockToInsert, startBlock); else insertAfter(blockToInsert, startBlock);
    let n;
    if (at.c === startBlock) n = childAt(startBlock, at.o);
    else {
      let splitTo = isText(at.c) ? at.c : (childAt(at.c, at.o) || at.c);
      // SplitTreeToNode(splitTo, startBlock): split each inline ancestor at the path.
      let top = splitTo;
      while (parentOf(top) !== startBlock && parentOf(top) !== null) { const parent = parentOf(top); if (prevOf(top) !== null) splitElement(parent, top); top = parent; }
      n = top;
    }
    for (let m = n; m !== null && m !== blockToInsert;) { const next = nextOf(m); moveNode(m, blockToInsert, null); m = next; }
    return P(blockToInsert, 0);
  }
  function hasRenderedContentExceptPlaceholder(block) {
    const items = streamOf(block);
    let count = 0;
    for (let i = 0; i < items.length; i++) { const t = items[i].t; if (t === CHAR || t === ATOM || t === BATOM) return true; if (t === BR) count++; }
    return count > 1;
  }

  // ---- fragments (Blink ReplaceSelectionCommand, simplified) ----------------------------------------
  function parseFragment(doc, markup) {
    const holder = createElement(doc, 'div');
    apply(innerHTMLSet, holder, [markup]);
    const nodes = list();
    for (let n = firstOf(holder); n !== null; n = nextOf(n)) push(nodes, n);
    for (let i = 0; i < nodes.length; i++) apply(removeChildF, holder, [nodes[i]]);
    return nodes;
  }
  function textContentOf(nodes) { let s = ''; const walk = n => { if (isText(n)) s += dataOf(n); for (let c = firstOf(n); c !== null; c = nextOf(c)) walk(c); }; for (let i = 0; i < nodes.length; i++) walk(nodes[i]); return s; }
  function insertNodesAt(doc, root, nodes, caret, selectInserted) {
    let pos = canonical(caret) || caret;
    const items = streamOf(root), k = gapOf(items, pos);
    let placeholder = isPlaceholder(items, k) ? items[k].n : null;
    let parent, ref;
    if (isText(pos.c)) {
      if (pos.o === 0) { parent = parentOf(pos.c); ref = pos.c; }
      else if (pos.o >= dataOf(pos.c).length) { parent = parentOf(pos.c); ref = nextOf(pos.c); }
      else { splitText(pos.c, pos.o); parent = parentOf(pos.c); ref = pos.c; }
    } else { parent = pos.c; ref = childAt(pos.c, pos.o); }
    for (let i = 0; i < nodes.length; i++) insertNode(nodes[i], parent, ref);
    if (placeholder !== null && parentOf(placeholder) !== null && nodes.length) removeNode(placeholder);
    const last = nodes[nodes.length - 1];
    return { __proto__: null, first: nodes[0], last, end: after(last) };
  }
  function insertHTML(doc, root, sel, markup) {
    let caret = sel.caret ? sel.start : deleteRange(doc, root, sel.start, sel.end, true);
    editorOf(doc).pendingTyping = null;
    let nodes = parseFragment(doc, markup);
    if (!nodes.length) return caret;
    if (!richly(root)) nodes = [createText(doc, textContentOf(nodes))];
    if (nodes.length === 1 && isText(nodes[0])) {
      const pos = canonical(caret) || caret;
      if (isText(pos.c)) { const s = dataOf(nodes[0]); insertText(pos.c, pos.o, s); return P(pos.c, pos.o + s.length); }
    }
    let blocks = 0;
    for (let i = 0; i < nodes.length; i++) if (isElement(nodes[i]) && blockNames[nameOf(nodes[i])] && nameOf(nodes[i]) !== 'hr') blocks++;
    let flat = nodes, trailingBreak = false;
    if (blocks) {
      flat = list();
      for (let i = 0; i < nodes.length; i++) {
        const n = nodes[i];
        if (isElement(n) && blockNames[nameOf(n)] && nameOf(n) !== 'hr') {
          if (flat.length && nameOf(flat[flat.length - 1]) !== 'br') push(flat, createElement(doc, 'br'));
          const kids = list(); for (let c = firstOf(n); c !== null; c = nextOf(c)) push(kids, c);
          for (let j = 0; j < kids.length; j++) { apply(removeChildF, n, [kids[j]]); push(flat, kids[j]); }
        } else push(flat, n);
      }
      trailingBreak = blocks === 1 && nodes.length === 1 && blockOf((canonical(caret) || caret).c, root) === root;
      if (!flat.length) return caret;
    }
    const inserted = insertNodesAt(doc, root, flat, caret, false);
    const endMark = blocks && isText(inserted.last) ? mark(P(inserted.last, dataOf(inserted.last).length)) : mark(inserted.end);
    if (isText(inserted.first)) mergeTextsAround(inserted.first);
    else { let f = inserted.first; while (isElement(f) && firstOf(f) !== null && !atomicNames[nameOf(f)]) f = firstOf(f); if (isText(f) && f !== inserted.first) mergeTextsAround(f); }
    if (blocks && isText(inserted.last) && parentOf(inserted.last) !== null) {
      const lastText = inserted.last, next = nextOf(lastText);
      if (next !== null && isText(next)) { insertText(lastText, dataOf(lastText).length, dataOf(next)); removeNode(next); }
    }
    let end = blocks && isText(inserted.last) ? marked(endMark) : (canonical(marked(endMark)) || marked(endMark));
    if (trailingBreak) {
      const items = streamOf(root), k = gapOf(items, end);
      let j = k; while (j < items.length && !boundaryType(items[j].t)) j++;
      const at = j < items.length ? itemStart(items[j]) : P(root, lengthOf(root));
      const m = mark(end);
      insertAt(createElement(doc, 'br'), j > k ? itemEnd(items[j - 1]) : at);
      end = marked(m);
    }
    return end;
  }
  function insertElement(doc, root, sel, element) {
    let caret = sel.caret ? sel.start : deleteRange(doc, root, sel.start, sel.end, true);
    editorOf(doc).pendingTyping = null;
    const inserted = insertNodesAt(doc, root, [element], caret, false);
    return after(inserted.last);
  }

  // ---- paragraphs ------------------------------------------------------------------------------------
  // Paragraph ranges [startGap, endGap] covering the selection.
  function paragraphsOf(items, sel) {
    let ks = gapOf(items, sel.start), ke = gapOf(items, sel.end);
    if (!sel.caret && ke > ks && startOfParagraph(items, ke)) { let j = ke - 1; while (j > ks && !isCandidateGap(items, j)) j--; ke = j; }
    while (ks > 0 && !boundaryType(items[ks - 1].t)) ks--;
    const out = list();
    let k = ks;
    while (k <= ke && k <= items.length) {
      let e = k; while (e < items.length && !boundaryType(items[e].t)) e++;
      if (e > k || (e === k && e < items.length && items[e].t === BR)) push(out, [k, e]);
      if (e >= items.length) break;
      k = e + 1;
      while (k < items.length && (items[k - 1].t === BE || items[k - 1].t === BS) && (items[k].t === BS || items[k].t === BE)) k++;
      if (k > ke) break;
    }
    return out;
  }
  // Splits so that the paragraph [startGap, endGap) is a sibling range under its block.
  function isolateParagraph(items, p, block) {
    const s = itemStart(items[p[0]]), e = p[1] > p[0] ? itemEnd(items[p[1] - 1]) : itemStart(items[p[0]]);
    let first, last;
    if (isText(s.c)) { if (s.o > 0) { if (e.c === s.c) { splitText(s.c, s.o); first = s.c; e.o -= s.o; } else { splitText(s.c, s.o); first = s.c; } } else first = s.c; }
    else first = childAt(s.c, s.o);
    if (isText(e.c)) { if (e.o < dataOf(e.c).length) { last = splitText(e.c, e.o); if (first === e.c) first = last; } else last = e.c; }
    else last = e.o > 0 ? childAt(e.c, e.o - 1) : childAt(e.c, 0);
    if (first === null || last === null) return null;
    while (parentOf(first) !== block && parentOf(first) !== null) { const parent = parentOf(first); if (prevOf(first) !== null) splitElement(parent, first); first = parent; }
    while (parentOf(last) !== block && parentOf(last) !== null) { const parent = parentOf(last); if (nextOf(last) !== null) { const clone = splitElement(parent, nextOf(last)); last = clone; } else last = parent; }
    return [first, last];
  }
  function siblingsBetween(first, last) { const out = list(); for (let n = first; n !== null; n = nextOf(n)) { push(out, n); if (n === last) break; } return out; }

  // ---- block commands --------------------------------------------------------------------------------
  function formatBlock(doc, root, sel, tag) {
    const items = streamOf(root), paragraphs = paragraphsOf(items, sel);
    let blockElement = null;
    for (let i = 0; i < paragraphs.length; i++) {
      const cur = streamOf(root);
      const p = paragraphsOf(cur, sel)[i] || paragraphs[i];
      if (p[1] <= p[0] && !(p[0] < cur.length && cur[p[0]].t === BR)) continue;
      const startItem = cur[p[0]];
      const block = blockOf(startItem.n, root);
      const whole = startOfBlock(cur, p[0]) && endOfBlock(cur, p[1]);
      let insertionRef = null;
      if (blockElement === null && block !== root && formatBlockNames[nameOf(block)] && whole && !contains(block, root)) {
        if (nameOf(block) === tag) continue;
        insertionRef = block;
      }
      const iso = isolateParagraph(cur, p, block);
      if (iso === null) continue;
      const nodes = siblingsBetween(iso[0], iso[1]);
      const trailing = nextOf(iso[1]);
      if (blockElement === null) {
        blockElement = createElement(doc, tag);
        const ref = insertionRef !== null ? insertionRef : iso[0];
        insertNode(blockElement, parentOf(ref), ref);
        if (insertionRef !== null && hasAttr(insertionRef, 'style')) setAttr(blockElement, 'style', attr(insertionRef, 'style'));
      } else if (firstOf(blockElement) !== null) appendNode(createElement(doc, 'br'), blockElement);
      for (let j = 0; j < nodes.length; j++) moveNode(nodes[j], blockElement, null);
      if (trailing !== null && nameOf(trailing) === 'br' && parentOf(trailing) === block) removeNode(trailing);
      let b = block;
      while (b !== null && b !== root && connectedOf(b) && !hasRenderedContent(b) && !contains(b, blockElement)) { const parent = parentOf(b); removeNode(b); b = parent; }
    }
    return blockElement !== null;
  }
  const listNames = { __proto__: null, ul: 1, ol: 1 };
  function enclosingList(node, root) { for (let n = node; n !== null && n !== root; n = parentOf(n)) if (listNames[nameOf(n)]) return n; return null; }
  function insertList(doc, root, sel, tag) {
    const editor = editorOf(doc);
    const startList = enclosingList(sel.start.c, root), endList = enclosingList(sel.end.c, root);
    if (startList !== null && startList === endList) {
      if (nameOf(startList) !== tag) {
        const replacement = createElement(doc, tag);
        const attrs = attributeList(startList);
        for (let i = 0; i < attrs.length; i++) apply(setAttrF, replacement, [attrs[i][0], attrs[i][1]]);
        insertNode(replacement, parentOf(startList), startList);
        for (let n = firstOf(startList); n !== null;) { const next = nextOf(n); moveNode(n, replacement, null); n = next; }
        removeNode(startList);
        return null;
      }
      const item = listChild(sel.start.c, root);
      if (item === null) return null;
      const parent = parentOf(startList);
      const hasBefore = prevOf(item) !== null, hasAfter = nextOf(item) !== null;
      let ref = startList;
      if (hasAfter) {
        const tail = cloneShallow(startList); apply(removeAttrF, tail, ['id']);
        insertAfter(tail, startList);
        for (let n = nextOf(item); n !== null;) { const next = nextOf(n); moveNode(n, tail, null); n = next; }
        ref = tail;
      }
      const kids = list(); for (let c = firstOf(item); c !== null; c = nextOf(c)) push(kids, c);
      for (let i = 0; i < kids.length; i++) moveNode(kids[i], parent, ref);
      if (hasAfter && !isBlock(parent) || (hasAfter && parent === root)) insertNode(createElement(doc, 'br'), parent, ref);
      removeNode(item);
      if (!hasBefore) removeNode(startList);
      return null;
    }
    const items = streamOf(root), paragraphs = paragraphsOf(items, sel);
    if (!paragraphs.length) return null;
    let listEl = null;
    for (let i = 0; i < paragraphs.length; i++) {
      const cur = streamOf(root);
      const p = paragraphsOf(cur, sel)[i] || paragraphs[i];
      const block = blockOf(cur[p[0]].n, root);
      const iso = isolateParagraph(cur, p, block);
      if (iso === null) continue;
      const nodes = siblingsBetween(iso[0], iso[1]);
      const trailing = nextOf(iso[1]);
      if (listEl === null) { listEl = createElement(doc, tag); insertNode(listEl, parentOf(iso[0]), iso[0]); }
      const li = createElement(doc, 'li');
      appendNode(li, listEl);
      for (let j = 0; j < nodes.length; j++) moveNode(nodes[j], li, null);
      if (trailing !== null && nameOf(trailing) === 'br' && parentOf(trailing) === block) removeNode(trailing);
      let b = block;
      while (b !== null && b !== root && connectedOf(b) && !hasRenderedContent(b) && !contains(b, listEl)) { const parent = parentOf(b); removeNode(b); b = parent; }
    }
    if (listEl !== null && paragraphs.length === 1) return P(firstOf(listEl), 0);
    return null;
  }
  function justify(doc, root, sel, value) {
    const editor = editorOf(doc), items = streamOf(root), paragraphs = paragraphsOf(items, sel);
    for (let i = 0; i < paragraphs.length; i++) {
      const cur = streamOf(root);
      const p = paragraphsOf(cur, sel)[i] || paragraphs[i];
      const block = blockOf(cur[p[0]].n, root);
      let target = block;
      if (block === root || !(startOfBlock(cur, p[0]) && endOfBlock(cur, p[1]))) {
        const iso = isolateParagraph(cur, p, block);
        if (iso === null) continue;
        const nodes = siblingsBetween(iso[0], iso[1]), trailing = nextOf(iso[1]);
        target = createElement(doc, editor.separator);
        insertNode(target, parentOf(iso[0]), iso[0]);
        for (let j = 0; j < nodes.length; j++) moveNode(nodes[j], target, null);
        if (trailing !== null && nameOf(trailing) === 'br') removeNode(trailing);
      }
      const d = declarations(target), keep = list();
      for (let j = 0; j < d.length; j++) if (d[j][0] !== 'text-align') push(keep, d[j]);
      if (value !== null) push(keep, ['text-align', value]);
      setAttr(target, 'style', serializeDeclarations(keep));
    }
  }
  const indentStyle = 'margin: 0 0 0 40px; border: none; padding: 0px;';
  function indent(doc, root, sel) {
    const items = streamOf(root), paragraphs = paragraphsOf(items, sel);
    let quote = null;
    for (let i = 0; i < paragraphs.length; i++) {
      const cur = streamOf(root);
      const p = paragraphsOf(cur, sel)[i] || paragraphs[i];
      const block = blockOf(cur[p[0]].n, root);
      let nodes, ref;
      if (block !== root && startOfBlock(cur, p[0]) && endOfBlock(cur, p[1])) { nodes = [block]; ref = block; }
      else { const iso = isolateParagraph(cur, p, block); if (iso === null) continue; nodes = siblingsBetween(iso[0], iso[1]); ref = iso[0]; const trailing = nextOf(iso[1]); if (trailing !== null && nameOf(trailing) === 'br') removeNode(trailing); }
      if (quote === null) { quote = createElement(doc, 'blockquote'); apply(setAttrF, quote, ['style', indentStyle]); insertNode(quote, parentOf(ref), ref); }
      else if (firstOf(quote) !== null && !isBlock(lastOf(quote)) && !isBlock(nodes[0])) appendNode(createElement(doc, 'br'), quote);
      for (let j = 0; j < nodes.length; j++) moveNode(nodes[j], quote, null);
    }
  }
  function outdent(doc, root, sel) {
    for (let n = sel.start.c; n !== null && n !== root; n = parentOf(n)) {
      if (nameOf(n) === 'blockquote') { unwrap(n); return; }
      if (listNames[nameOf(n)] && enclosingList(parentOf(n), root) !== null) { unwrap(n); return; }
    }
  }

  // ---- transactions and undo ----------------------------------------------------------------------------
  function snapshot(sel) { return sel === null ? null : [sel.raw.anchor, sel.raw.focus]; }
  // Runs `body` recording its DOM primitives; registers or extends the undo step, fires `input` at the
  // editing hosts and moves the selection (Blink's Editor::AppliedEditing).
  function edit(doc, kind, inputType, data, body, always) {
    const editor = editorOf(doc), sel = visibleSelection(doc);
    if (sel === null) return false;
    const ops = list(), saved = journal;
    journal = ops;
    let result;
    try { result = body(sel); } finally { journal = saved; }
    if (!ops.length && !always) return result !== undefined && result !== null && result.selection !== undefined;
    const startRoot = sel.root;
    let endSel = result && result.selection ? result.selection : null;
    let endRoot = endSel !== null && connectedOf(endSel[0].c) ? rootEditable(endSel[0].c) : startRoot;
    if (endRoot === null) endRoot = startRoot;
    const open = editor.open;
    const continuing = kind !== null && open !== null && editor.undo[editor.undo.length - 1] === open && (kind === 'insert' || kind === 'forward' || open.kind === kind) && editor.openValid;
    let step;
    if (continuing) { step = open; for (let i = 0; i < ops.length; i++) push(step.ops, ops[i]); if (result && result.extendStart && step.start !== null) step.start = result.extendStart(step.start); }
    else {
      step = record(); step.ops = ops; step.kind = kind; step.start = result && result.startSelection ? result.startSelection : snapshot(sel); step.startRoot = startRoot;
    }
    step.endRoot = endRoot;
    fireInput(startRoot !== null && connectedOf(startRoot) ? startRoot : null, endRoot !== null && connectedOf(endRoot) ? endRoot : null, inputType, data);
    if (endSel !== null && endSel[0] !== null && connectedOf(endSel[0].c)) {
      if (endSel[2]) setRangeSelection(doc, endSel[0], endSel[1]); else setSelection(doc, endSel[0], endSel[1]);
    }
    const now = visibleSelection(doc);
    step.end = snapshot(now);
    if (!continuing) { push(editor.undo, step); if (editor.undo.length > 1000) { const kept = list(); for (let i = 1; i < editor.undo.length; i++) push(kept, editor.undo[i]); editor.undo = kept; } editor.redo = list(); }
    editor.open = kind !== null ? step : null;
    editor.openSelection = now === null ? null : [now.start, now.end];
    editor.openValid = true;
    editor.openRange = currentRange(doc);
    if (!(result && result.keepTyping)) editor.typing = null;
    else if (editor.typing !== null && now !== null && now.caret) { editor.typing.at = now.start; editor.typing.range = editor.openRange; }
    if (editor.pendingTyping && now !== null && now.caret) { editor.typing = { __proto__: null, at: now.start, style: editor.pendingTyping, range: editor.openRange }; }
    editor.pendingTyping = null;
    return true;
  }
  // An open typing step only continues while the selection is where the last edit left it.
  function refreshOpen(doc) {
    const editor = editorOf(doc);
    if (editor.open === null) return;
    if (editor.open.control) { const c = focusedControl(doc); editor.openValid = c !== null && c.el === editor.open.control && controlState(c.el)[0] === editor.open.after[0] && controlState(c.el)[1] === editor.open.after[1] && controlState(c.el)[2] === editor.open.after[2]; if (!editor.openValid) editor.open = null; return; }
    const sel = visibleSelection(doc), last = editor.openSelection;
    editor.openValid = sel !== null && last !== null && same(sel.start, last[0]) && same(sel.end, last[1]) && currentRange(doc) === editor.openRange;
    if (!editor.openValid) editor.open = null;
  }
  // Blink's UndoStack::ElementRemoved drops steps whose editing host left the document.
  function prune(steps) {
    const kept = list();
    for (let i = 0; i < steps.length; i++) { const s = steps[i]; if (s.control ? connectedOf(s.control) : (s.startRoot === null || connectedOf(s.startRoot)) && (s.endRoot === null || connectedOf(s.endRoot))) push(kept, s); }
    return kept;
  }
  function pruneHistory(editor) { editor.undo = prune(editor.undo); editor.redo = prune(editor.redo); }
  function undo(doc) {
    const editor = editorOf(doc);
    pruneHistory(editor);
    if (!editor.undo.length) return false;
    const step = editor.undo[editor.undo.length - 1];
    editor.undo.length--;
    for (let i = step.ops.length - 1; i >= 0; i--) undoOp(step.ops[i]);
    editor.open = null; editor.typing = null;
    if (step.control) { controlSet(step.control, step.before[0], step.before[1], step.before[2]); dispatch(step.control, inputEvent('input', 'historyUndo', null, false)); push(editor.redo, step); return true; }
    fireInput(step.startRoot !== null && connectedOf(step.startRoot) ? step.startRoot : null, step.endRoot !== null && connectedOf(step.endRoot) ? step.endRoot : null, 'historyUndo', null);
    if (step.start !== null && connectedOf(step.start[0].c) && connectedOf(step.start[1].c)) { const s = selectionOf(doc); if (s) apply(selSetF, s, [step.start[0].c, step.start[0].o, step.start[1].c, step.start[1].o]); }
    push(editor.redo, step);
    return true;
  }
  function redo(doc) {
    const editor = editorOf(doc);
    pruneHistory(editor);
    if (!editor.redo.length) return false;
    const step = editor.redo[editor.redo.length - 1];
    editor.redo.length--;
    for (let i = 0; i < step.ops.length; i++) redoOp(step.ops[i]);
    editor.open = null; editor.typing = null;
    if (step.control) { controlSet(step.control, step.after[0], step.after[1], step.after[2]); dispatch(step.control, inputEvent('input', 'historyRedo', null, false)); push(editor.undo, step); return true; }
    fireInput(step.startRoot !== null && connectedOf(step.startRoot) ? step.startRoot : null, step.endRoot !== null && connectedOf(step.endRoot) ? step.endRoot : null, 'historyRedo', null);
    if (step.end !== null && connectedOf(step.end[0].c) && connectedOf(step.end[1].c)) { const s = selectionOf(doc); if (s) apply(selSetF, s, [step.end[0].c, step.end[0].o, step.end[1].c, step.end[1].o]); }
    push(editor.undo, step);
    return true;
  }


  // ---- focused text controls ------------------------------------------------------------------------
  // With an <input> or <textarea> focused the selection lives in the control's inner editor: commands
  // edit its value as plain text and fire their events at the control.
  const InputProto = g.HTMLInputElement.prototype, AreaProto = g.HTMLTextAreaElement.prototype;
  const controlAccessors = proto => ({ __proto__: null, value: lookup(proto, 'value'), start: lookup(proto, 'selectionStart').get, end: lookup(proto, 'selectionEnd').get, range: lookup(proto, 'setSelectionRange').value, readOnly: lookup(proto, 'readOnly').get, disabled: lookup(proto, 'disabled').get });
  const inputAccess = controlAccessors(InputProto), areaAccess = controlAccessors(AreaProto);
  function focusedControl(doc) {
    let el;
    try { el = activeOf(doc); } catch (error) { return null; }
    if (el === null) return null;
    const name = nameOf(el);
    if (name === 'textarea') return { __proto__: null, el, area: true, access: areaAccess };
    if (name === 'input' && apply(inputAccess.start, el, []) !== null) return { __proto__: null, el, area: false, access: inputAccess };
    return null;
  }
  const controlOf = el => nameOf(el) === 'textarea' ? areaAccess : inputAccess;
  function controlState(el) { const a = controlOf(el); return [apply(a.value.get, el, []), apply(a.start, el, []), apply(a.end, el, [])]; }
  function controlEditable(c) { return !apply(c.access.readOnly, c.el, []) && !apply(c.access.disabled, c.el, []); }
  function controlSet(el, value, start, end) { const a = controlOf(el); if (apply(a.value.get, el, []) !== value) apply(a.value.set, el, [value]); apply(a.range, el, [start, end]); }
  function controlEdit(doc, c, kind, inputType, data, compute) {
    const editor = editorOf(doc), el = c.el, before = controlState(el);
    const change = compute(before[0], before[1], before[2]);
    if (change === null) return;
    const open = editor.open;
    const continuing = kind !== null && open !== null && open.control === el && editor.undo[editor.undo.length - 1] === open && (kind === 'insert' || kind === 'forward' || open.kind === kind) && editor.openValid;
    controlSet(el, change[0], change[1], change[1]);
    let step;
    if (continuing) step = open;
    else {
      step = record(); step.ops = list(); step.kind = kind; step.control = el; step.startRoot = null; step.endRoot = null;
      step.before = change[2] ? [before[0], change[2][0], change[2][1]] : before;
      push(editor.undo, step); editor.redo = list();
    }
    step.after = controlState(el);
    editor.open = kind !== null ? step : null;
    editor.openValid = true;
    dispatch(el, inputEvent('input', inputType, data, false));
  }
  function controlInsertText(doc, c, text) {
    if (!c.area) {
      let flat = '';
      for (let i = 0; i < text.length; i++) { const ch = text[i]; if (ch === '\r' && text[i + 1] === '\n') continue; flat += ch === '\n' || ch === '\r' ? ' ' : ch; }
      if (!flat.length) { const st = controlState(c.el); if (st[1] === st[2]) return true; }
      controlEdit(doc, c, 'insert', 'insertText', flat, (v, a, b) => [sub(v, 0, a) + flat + sub(v, b), a + flat.length]);
      return true;
    }
    const lines = list(); let at = 0;
    for (;;) { const nl = find(text, '\n', at); if (nl < 0) { push(lines, sub(text, at)); break; } push(lines, sub(text, at, nl)); at = nl + 1; }
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.length || (i === 0 && lines.length === 1)) {
        const st = controlState(c.el);
        if (line.length || st[1] !== st[2]) controlEdit(doc, c, 'insert', 'insertText', line, (v, a, b) => [sub(v, 0, a) + line + sub(v, b), a + line.length]);
      }
      if (i < lines.length - 1) controlEdit(doc, c, 'insert', 'insertText', null, (v, a, b) => [sub(v, 0, a) + '\n' + sub(v, b), a + 1]);
    }
    return true;
  }
  function controlDelete(doc, c, forward) {
    controlEdit(doc, c, forward ? 'forward' : 'delete', forward ? 'deleteContentForward' : 'deleteContentBackward', null, (v, a, b) => {
      if (a !== b) return [sub(v, 0, a) + sub(v, b), a, [a, b]];
      if (!forward) { if (a === 0) return null; const from = backwardBoundary(v, a); return [sub(v, 0, from) + sub(v, a), from, [from, a]]; }
      if (a >= v.length) return null;
      const to = forwardBoundary(v, a);
      return [sub(v, 0, a) + sub(v, to), a, [a, to]];
    });
    return true;
  }
  function controlParagraph(doc, c, lineBreak) {
    if (!c.area) return true;
    controlEdit(doc, c, 'insert', lineBreak ? 'insertLineBreak' : 'insertParagraph', null, (v, a, b) => [sub(v, 0, a) + '\n' + sub(v, b), a + 1]);
    return true;
  }
  function controlSelectAll(doc, c) {
    if (!dispatch(c.el, new EventType('selectstart', { __proto__: null, bubbles: true, cancelable: true }))) return true;
    const st = controlState(c.el);
    apply(c.access.range, c.el, [0, st[0].length]);
    dispatch(c.el, new EventType('select', { __proto__: null, bubbles: true, cancelable: false }));
    const editor = editorOf(doc); editor.open = null;
    return true;
  }

  // ---- commands ---------------------------------------------------------------------------------------
  const editableSelection = doc => { const s = visibleSelection(doc); return s !== null && s.root !== null ? s : null; };
  const richSelection = doc => { const s = editableSelection(doc); return s !== null && richly(s.start.c) ? s : null; };
  // The position whose style answers "style at selection start" (CreateStyleAtSelectionStart).
  function styleNodeAt(sel) {
    let pos = sel.start;
    if (!sel.caret && isText(pos.c) && pos.o >= dataOf(pos.c).length) {
      const items = streamOf(contextRoot(pos)), k = gapOf(items, pos), n = nextGap(items, k);
      if (n >= 0) { const c = forwardCandidate(items, n); if (c !== null) pos = c; }
    }
    return isText(pos.c) ? pos.c : (childAt(pos.c, pos.o) && isElement(pos.c) && !isText(childAt(pos.c, pos.o)) ? pos.c : pos.c);
  }
  function startStyle(doc, sel) {
    const s = styleAt(styleNodeAt(sel)), typing = typingStyleFor(editorOf(doc), sel);
    if (typing !== null) {
      for (const key in typing) {
        const v = typing[key];
        if (key === 'font-weight') s.weight = weightNumber(v, s.weight);
        else if (key === 'font-style') s.italic = v === 'italic';
        else if (key === 'text-decoration-line') { s.underline = v.underline; s.strike = v.strike; }
        else if (key === 'vertical-align') s.script = v;
        else if (key === 'color') s.color = parseColor(v, false);
        else if (key === 'font-family') s.family = v;
        else if (key === 'font-size') s.size = legacyPixels[v];
        else if (key === 'background-color') s.background = parseColor(v, false);
      }
    }
    return s;
  }
  function applyStyleCommand(doc, style, inputType) {
    const editor = editorOf(doc), sel = visibleSelection(doc);
    if (sel === null) return false;
    if (sel.caret) {
      const merged = editor.typing !== null && same(editor.typing.at, sel.start) ? mergeStyles(editor.typing.style, style) : style;
      const prepared = prepareStyleAt(merged, sel.start.c);
      editor.typing = styleIsEmpty(prepared) ? null : { __proto__: null, at: sel.start, style: prepared, range: currentRange(doc) };
      return true;
    }
    edit(doc, null, inputType, null, s => { const r = applyInlineStyle(doc, style, s.start, s.end, editor.css); return { __proto__: null, selection: [r[0], r[1], true] }; }, true);
    return true;
  }
  function toggle(doc, key, inputType) {
    const sel = visibleSelection(doc);
    if (sel === null) return false;
    const s = startStyle(doc, sel), style = record();
    if (key === 'bold') style['font-weight'] = s.weight >= 600 ? 'normal' : 'bold';
    else if (key === 'italic') style['font-style'] = s.italic ? 'normal' : 'italic';
    else if (key === 'subscript') style['vertical-align'] = s.script === 'sub' ? 'baseline' : 'sub';
    else if (key === 'superscript') style['vertical-align'] = s.script === 'super' ? 'baseline' : 'super';
    else {
      const d = record(); d.underline = s.underline; d.strike = s.strike;
      if (key === 'underline') d.underline = !s.underline; else d.strike = !s.strike;
      style['text-decoration-line'] = d;
    }
    return applyStyleCommand(doc, style, inputType);
  }
  function valueStyle(doc, key, value) {
    const style = record();
    if (key === 'color' || key === 'background-color') { if (rgbOf(value, false) === null) return applyStyleCommand(doc, style, ''); style[key] = trim(value); }
    else if (key === 'font-family') { if (!trim(value).length) return applyStyleCommand(doc, style, ''); style[key] = value; }
    else if (key === 'font-size') { const n = legacySizeNumber(value); if (!n) return false; style[key] = n; }
    return applyStyleCommand(doc, style, '');
  }
  function typingEdit(doc, kind, inputType, data, body) {
    refreshOpen(doc);
    return edit(doc, kind, inputType, data, body, false);
  }
  function execInsertText(doc, value) {
    const control = focusedControl(doc);
    if (control !== null) { refreshOpen(doc); return controlInsertText(doc, control, value); }
    const editor = editorOf(doc);
    let sel = editableSelection(doc);
    if (sel === null) return false;
    const lines = list(); let at = 0;
    for (;;) { const nl = find(value, '\n', at); if (nl < 0) { push(lines, sub(value, at)); break; } push(lines, sub(value, at, nl)); at = nl + 1; }
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i], last = i === lines.length - 1;
      if (line.length || (last && i === 0)) {
        sel = editableSelection(doc);
        if (sel === null) return true;
        if (sel.caret && !line.length) continue;
        typingEdit(doc, 'insert', 'insertText', line, s => {
          const typing = typingStyleFor(editor, s);
          const end = insertTextRun(doc, s.root, s, line, typing === null ? null : prepareStyleAt(typing, s.start.c));
          return { __proto__: null, selection: [end, end] };
        });
      }
      if (!last) typingEdit(doc, 'insert', 'insertText', null, s => {
        const caret = s.caret ? s.start : deleteRange(doc, s.root, s.start, s.end, true);
        const end = insertParagraph(doc, s.root, caret);
        return { __proto__: null, selection: [end, end], keepTyping: true };
      });
    }
    return true;
  }
  function execDelete(doc, forward) {
    const control = focusedControl(doc);
    if (control !== null) { refreshOpen(doc); return controlDelete(doc, control, forward); }
    const sel = editableSelection(doc);
    if (sel === null) return false;
    const kind = forward ? 'forward' : 'delete', inputType = forward ? 'deleteContentForward' : 'deleteContentBackward';
    if (!sel.caret) {
      typingEdit(doc, kind, inputType, null, s => { const c = deleteRange(doc, s.root, s.start, s.end, true); return { __proto__: null, selection: [c, c], keepTyping: true }; });
      return true;
    }
    const range = deletionRange(sel.root, sel.start, forward);
    if (range === null) return true;
    typingEdit(doc, kind, inputType, null, s => {
      const r = deletionRange(s.root, s.start, forward);
      if (r === null) return null;
      const startSelection = forward ? [s.start, r[1]] : [s.start, r[0]];
      // TypingCommand::DeleteKeyPressed / ForwardDeleteKeyPressed: undo selects all that was deleted.
      const extendStart = previous => {
        const ordered = compare(previous[0], previous[1]) <= 0, first = ordered ? previous[0] : previous[1], last = ordered ? previous[1] : previous[0];
        if (!forward) return [last, r[0]];
        if (last.c !== r[1].c) return [first, r[1]];
        const extra = r[0].c === r[1].c ? r[1].o - r[0].o : r[1].o;
        return [first, P(last.c, last.o + extra)];
      };
      const c = deleteRange(doc, s.root, r[0], r[1], true);
      return { __proto__: null, selection: [c, c], keepTyping: true, startSelection, extendStart };
    });
    return true;
  }
  function execParagraph(doc, lineBreak) {
    const control = focusedControl(doc);
    if (control !== null) { refreshOpen(doc); return controlParagraph(doc, control, lineBreak); }
    const sel = editableSelection(doc);
    if (sel === null) return false;
    typingEdit(doc, 'insert', lineBreak ? 'insertLineBreak' : 'insertParagraph', null, s => {
      const caret = s.caret ? s.start : deleteRange(doc, s.root, s.start, s.end, true);
      const end = lineBreak ? insertLineBreak(doc, s.root, caret) : insertParagraph(doc, s.root, caret);
      return { __proto__: null, selection: [end, end], keepTyping: true };
    });
    return true;
  }
  function execInsertHTML(doc, value) {
    const control = focusedControl(doc);
    if (control !== null) { refreshOpen(doc); const text = textContentOf(parseFragment(doc, value)); controlEdit(doc, control, null, 'insertText', text, (v, a, b) => [sub(v, 0, a) + text + sub(v, b), a + text.length]); return true; }
    if (editableSelection(doc) === null) return false;
    edit(doc, null, '', null, s => { const end = insertHTML(doc, s.root, s, value); return { __proto__: null, selection: [end, end] }; }, true);
    return true;
  }
  function execInsertElement(doc, name, attribute, value) {
    if (richSelection(doc) === null) return false;
    edit(doc, null, '', null, s => {
      const element = createElement(doc, name);
      if (value.length) apply(setAttrF, element, [attribute, value]);
      const end = insertElement(doc, s.root, s, element);
      return { __proto__: null, selection: [end, end] };
    }, true);
    return true;
  }
  function execCreateLink(doc, value) {
    if (!value.length) return false;
    const editor = editorOf(doc), sel = richSelection(doc);
    if (sel === null) return false;
    edit(doc, null, 'insertLink', value, s => {
      if (s.caret) {
        const a = createElement(doc, 'a');
        apply(setAttrF, a, ['href', value]);
        insertAt(a, s.start);
        appendNode(createText(doc, value), a);
        return { __proto__: null, selection: [before(a), after(a), true] };
      }
      const style = record(); style.link = value;
      const r = applyInlineStyle(doc, style, s.start, s.end, editor.css);
      return { __proto__: null, selection: [r[0], r[1], true] };
    }, true);
    return true;
  }
  function removeStyled(doc, style, inputType, filter) {
    const editor = editorOf(doc);
    edit(doc, null, inputType, null, s => {
      let a = s.start, b = s.end;
      const inText = p => isText(p.c) && p.o > 0 && p.o < dataOf(p.c).length;
      if (inText(a)) { const node = a.c, sameNode = b.c === node; splitText(node, a.o); if (sameNode) b = P(node, b.o - a.o); a = P(node, 0); }
      if (inText(b)) { const node = b.c; const prefix = splitText(node, b.o); if (a.c === node) a = P(prefix, a.o); b = P(prefix, dataOf(prefix).length); }
      const startNode = a.c, endNode = b.c;
      // Split removable ancestors at the boundaries so the selected part can be unwrapped alone.
      const splitAround = (node, atStart) => {
        for (let n = node; parentOf(n) !== null && !isBlock(parentOf(n)) && rootEditable(parentOf(n)) !== parentOf(n);) {
          const parent = parentOf(n);
          if (filter(parent)) {
            if (atStart && prevOf(n) !== null) splitElement(parent, n);
            if (!atStart && nextOf(n) !== null) splitElement(parent, nextOf(n));
          }
          n = parent;
        }
      };
      splitAround(startNode, true);
      splitAround(endNode, false);
      const doomed = list();
      const top = rootEditable(startNode) || startNode;
      for (let n = top; n !== null; n = nodeAfter(n)) {
        if (!contains(top, n)) break;
        if (isElement(n) && n !== top && filter(n) && visiblyContained(n, P(parentOf(startNode), indexOf(startNode)), P(parentOf(endNode), indexOf(endNode) + 1))) push(doomed, n);
      }
      for (let i = 0; i < doomed.length; i++) {
        const el = doomed[i];
        if (parentOf(el) === null) continue;
        if (nameOf(el) === 'a' && style === 'link') { unwrap(el); continue; }
        if (style === 'format') {
          if (removableFormat[nameOf(el)]) { unwrap(el); continue; }
          if (hasAttr(el, 'style')) { removeAttr(el, 'style'); if (nameOf(el) === 'span' && !attributeList(el).length) unwrap(el); }
        }
      }
      return { __proto__: null, selection: [P(startNode, 0), P(endNode, isText(endNode) ? dataOf(endNode).length : lengthOf(endNode)), true] };
    }, true);
    return true;
  }
  const removableFormat = { __proto__: null, acronym: 1, b: 1, bdo: 1, big: 1, cite: 1, code: 1, dfn: 1, em: 1, font: 1, i: 1, ins: 1, kbd: 1, nobr: 1, q: 1, s: 1, samp: 1, small: 1, strike: 1, strong: 1, sub: 1, sup: 1, tt: 1, u: 1, var: 1 };
  const xmlNameStart = c => (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c === 95 || c >= 0xc0;
  const xmlNameChar = c => xmlNameStart(c) || (c >= 48 && c <= 57) || c === 45 || c === 46 || c === 0xb7;
  function qualifiedLocal(value) {
    if (!value.length) return null;
    const colon = find(value, ':');
    const parts = colon >= 0 ? [sub(value, 0, colon), sub(value, colon + 1)] : [value];
    if (colon >= 0 && find(parts[1], ':') >= 0) return null;
    for (let i = 0; i < parts.length; i++) { const p = parts[i]; if (!p.length || !xmlNameStart(code(p, 0))) return null; for (let j = 1; j < p.length; j++) if (!xmlNameChar(code(p, j))) return null; }
    return parts[parts.length - 1];
  }
  function execFormatBlock(doc, value) {
    let tag = lower(value);
    if (tag.length >= 2 && tag[0] === '<' && tag[tag.length - 1] === '>') tag = sub(tag, 1, tag.length - 1);
    const local = qualifiedLocal(tag);
    if (local === null) return false;
    if (richSelection(doc) === null) return false;
    let applied = false;
    edit(doc, null, '', null, s => {
      if (!formatBlockNames[tag]) return null;
      const points = [mark(s.raw.anchor), mark(s.raw.focus)];
      applied = true;
      formatBlock(doc, s.root, s, tag);
      const a = marked(points[0]), f = marked(points[1]);
      return { __proto__: null, selection: [a, f, !same(a, f)] };
    }, true);
    return applied;
  }
  function execList(doc, tag) {
    if (richSelection(doc) === null) return false;
    edit(doc, null, tag === 'ol' ? 'insertOrderedList' : 'insertUnorderedList', null, s => {
      const points = [mark(s.raw.anchor), mark(s.raw.focus)];
      const caret = insertList(doc, s.root, s, tag);
      if (caret !== null) return { __proto__: null, selection: [caret, caret] };
      const a = marked(points[0]), f = marked(points[1]);
      return { __proto__: null, selection: [a, f, !same(a, f)] };
    }, true);
    return true;
  }
  function execBlock(doc, inputType, body) {
    if (richSelection(doc) === null) return false;
    edit(doc, null, inputType, null, s => {
      const points = [mark(s.raw.anchor), mark(s.raw.focus)];
      body(s);
      const a = marked(points[0]), f = marked(points[1]);
      return { __proto__: null, selection: [a, f, !same(a, f)] };
    }, true);
    return true;
  }
  function highestEditableRoot(root) {
    let highest = root;
    const body = bodyOf(ownerOf(root));
    if (highest === body) return highest;
    for (let n = parentOf(root); n !== null && typeOf(n) === 1; n = parentOf(n)) { if (isEditable(n)) highest = n; if (n === body) break; }
    return highest;
  }
  function selectAllEnabled(doc) {
    if (focusedControl(doc) !== null) return true;
    const sel = visibleSelection(doc);
    if (sel === null || sel.root === null) return true;
    const root = highestEditableRoot(sel.root), first = firstOf(root);
    if (first === null) return false;
    if (first === lastOf(root) && nameOf(first) === 'br') return false;
    return true;
  }
  function execSelectAll(doc) {
    const control = focusedControl(doc);
    if (control !== null) return controlSelectAll(doc, control);
    const editor = editorOf(doc), sel = visibleSelection(doc);
    let root, target;
    if (sel !== null && sel.root !== null) { root = highestEditableRoot(sel.root); target = root; }
    else { root = rootElementOf(doc); target = bodyOf(doc); }
    if (root === null) return true;
    if (target !== null) {
      const event = new EventType('selectstart', { __proto__: null, bubbles: true, cancelable: true });
      if (!dispatch(target, event)) return true;
      if (!connectedOf(root)) return true;
    }
    const items = streamOf(root);
    let first = null, last = null;
    for (let k = 0; k <= items.length && first === null; k++) first = forwardCandidate(items, k);
    for (let k = items.length; k >= 0 && last === null; k--) last = candidate(items, k);
    editor.open = null; editor.typing = null;
    const s = selectionOf(doc);
    if (s && first !== null && last !== null) apply(selSetF, s, [first.c, first.o, last.c, last.o]);
    return true;
  }
  function execFindString(doc, value) {
    if (!value.length) return false;
    const root = rootElementOf(doc);
    if (root === null) return false;
    const items = buildStream(root);
    let text = '';
    const map = list();
    for (let i = 0; i < items.length; i++) {
      const it = items[i];
      if (it.t === CHAR) { let c = dataOf(it.n)[it.o]; if (c === NBSP) c = ' '; text += c; push(map, i); }
      else if (it.t === BR || it.t === BS || it.t === BE || it.t === BATOM) { if (text.length && text[text.length - 1] !== '\n') { text += '\n'; push(map, -1); } }
    }
    text = lower(text);
    let needle = '';
    for (let i = 0; i < value.length; i++) { const c = value[i]; needle += c === NBSP ? ' ' : c; }
    needle = lower(needle);
    const sel = visibleSelection(doc);
    let from = 0;
    if (sel !== null) { const k = gapOf(items, sel.end); while (from < map.length && (map[from] < 0 || map[from] < k)) from++; }
    let hit = find(text, needle, from);
    if (hit < 0) hit = find(text, needle, 0);
    if (hit < 0) return false;
    const a = items[map[hit]], b = items[map[hit + needle.length - 1]];
    if (a === undefined || b === undefined) return false;
    const editor = editorOf(doc); editor.open = null; editor.typing = null;
    const s = selectionOf(doc);
    if (s) apply(selSetF, s, [a.n, a.o, b.n, b.o + 1]);
    return true;
  }
  function execTranspose(doc) {
    const sel = editableSelection(doc);
    if (sel === null || !sel.caret) return true;
    const root = sel.root, items = streamOf(root), k = gapOf(items, sel.start);
    const next = endOfParagraph(items, k) ? k : nextGap(items, k);
    if (next < 0) return true;
    const previous = prevGap(items, next);
    if (previous < 0) return true;
    const previous2 = prevGap(items, previous);
    if (previous2 < 0) return true;
    for (let i = previous2; i < next; i++) if (items[i].t !== CHAR) return true;
    if (next - previous2 !== 2) return true;
    const a = items[previous2], b = items[next - 1];
    const text = dataOf(a.n)[a.o] + dataOf(b.n)[b.o];
    const transposed = text[1] + text[0];
    const start = P(a.n, a.o), end = P(b.n, b.o + 1);
    const active = activeOf(doc), target = active !== null ? active : bodyOf(doc);
    const range = list();
    push(range, new StaticRangeType({ __proto__: null, startContainer: start.c, startOffset: start.o, endContainer: end.c, endOffset: end.o }));
    if (target !== null && !fireBeforeInput(target, 'insertTranspose', transposed, range)) return false;
    const s = selectionOf(doc);
    if (s && connectedOf(start.c) && connectedOf(end.c)) apply(selSetF, s, [start.c, start.o, end.c, end.o]);
    edit(doc, null, 'insertTranspose', null, v => {
      if (v.start.c === v.end.c && isText(v.start.c)) { replaceText(v.start.c, v.start.o, v.end.o - v.start.o, transposed); const c = P(v.start.c, v.start.o + transposed.length); return { __proto__: null, selection: [c, c] }; }
      const c = insertTextRun(doc, v.root, v, transposed, null);
      return { __proto__: null, selection: [c, c] };
    }, true);
    return true;
  }
  function execUnselect(doc) { const s = selectionOf(doc); if (s) apply(selClearF, s, []); const editor = editorOf(doc); editor.open = null; editor.typing = null; return true; }
  // Chrome hands printing to the embedder; the engine prints through window.print when the embedder provides it.
  function execPrint() { if (originalPrint !== null) apply(originalPrint, g, []); return true; }
  function execFontSizeDelta(doc, value) {
    const delta = NumberValue.parseFloat(value);
    const sel = richSelection(doc);
    if (sel === null) return false;
    if (delta !== delta) return applyStyleCommand(doc, record(), '');
    if (sel.caret) return true;
    edit(doc, null, '', null, s => {
      let a = s.start, b = s.end;
      const inText = p => isText(p.c) && p.o > 0 && p.o < dataOf(p.c).length;
      if (inText(a)) { const node = a.c, sameNode = b.c === node; splitText(node, a.o); if (sameNode) b = P(node, b.o - a.o); a = P(node, 0); }
      if (inText(b)) { const node = b.c; const prefix = splitText(node, b.o); if (a.c === node) a = P(prefix, a.o); b = P(prefix, dataOf(prefix).length); }
      const texts = list();
      for (let n = a.c; n !== null; n = nodeAfter(n)) { if (isText(n) && dataOf(n).length) push(texts, n); if (n === b.c) break; }
      for (let i = 0; i < texts.length; i++) {
        const t = texts[i], size = styleAt(t).size + delta, parent = parentOf(t);
        if (nameOf(parent) === 'span' && firstOf(parent) === t && lastOf(parent) === t && attributeList(parent).length === 1 && hasAttr(parent, 'style')) {
          const d = declarations(parent), keep = list();
          for (let j = 0; j < d.length; j++) if (d[j][0] !== 'font-size') push(keep, d[j]);
          push(keep, ['font-size', StringValue(size) + 'px']);
          setAttr(parent, 'style', serializeDeclarations(keep));
        } else { const span = createElement(doc, 'span'); apply(setAttrF, span, ['style', 'font-size: ' + StringValue(size) + 'px;']); surround(t, t, span); }
      }
      return { __proto__: null, selection: [a, b, true] };
    }, true);
    return true;
  }

  // ---- Chrome 156 DOM command table ---------------------------------------------------------------------
  const always = () => true, never = () => false;
  const inEditable = doc => { const c = focusedControl(doc); return c !== null ? controlEditable(c) : editableSelection(doc) !== null; };
  const inRich = doc => focusedControl(doc) === null && richSelection(doc) !== null;
  const rangeInRich = doc => { if (focusedControl(doc) !== null) return false; const s = richSelection(doc); return s !== null && !s.caret; };
  const caretInEditable = doc => { if (focusedControl(doc) !== null) return false; const s = editableSelection(doc); return s !== null && s.caret; };
  const visibleSelectionEnabled = doc => { const s = visibleSelection(doc); return s !== null && (!s.caret || s.root !== null); };
  const canUndo = doc => { const e = editorOf(doc); pruneHistory(e); return e.undo.length > 0; }, canRedo = doc => { const e = editorOf(doc); pruneHistory(e); return e.redo.length > 0; };
  const noState = () => false;
  function styleState(key) {
    return doc => {
      const sel = visibleSelection(doc);
      if (sel === null) return false;
      const s = startStyle(doc, sel);
      if (key === 'bold') return s.weight >= 600;
      if (key === 'italic') return s.italic;
      if (key === 'underline') return s.underline;
      if (key === 'strikethrough') return s.strike;
      if (key === 'subscript') return s.script === 'sub';
      if (key === 'superscript') return s.script === 'super';
      return false;
    };
  }
  function listState(tag) {
    return doc => {
      const sel = visibleSelection(doc);
      if (sel === null) return false;
      const enclosing = node => { for (let n = node; n !== null; n = parentOf(n)) if (nameOf(n) === tag) return n; return null; };
      if (sel.caret) return enclosing(sel.start.c) !== null;
      const a = enclosing(sel.start.c), b = enclosing(sel.end.c);
      if (a === null || a !== b) return false;
      const other = tag === 'ul' ? 'ol' : 'ul';
      for (let n = firstOf(a); n !== null; n = nextOf(n)) if (nameOf(n) === other) return false;
      return true;
    };
  }
  function alignOf(doc) {
    const sel = visibleSelection(doc);
    if (sel === null) return null;
    const s = styleAt(styleNodeAt(sel));
    return s.align === null ? 'left' : s.align === '-webkit-center' ? 'center' : s.align;
  }
  const justifyState = want => doc => { const a = alignOf(doc); return a !== null && (a === want || (want === 'left' && (a === 'start' || a === 'auto'))); };
  const justifyValue = doc => { const a = alignOf(doc); return a === null ? '' : a; };
  function styleValue(key) {
    return doc => {
      const sel = visibleSelection(doc);
      if (sel === null) return '';
      const s = startStyle(doc, sel);
      if (key === 'family') return s.family !== null ? s.family : defaultFamily();
      if (key === 'size') return StringValue(legacySizeOfPixels(s.size));
      if (key === 'color') return s.color !== null ? s.color : defaultColor();
      if (key === 'background') return s.background !== null ? s.background : 'rgba(0, 0, 0, 0)';
      return '';
    };
  }
  function formatBlockValue(doc) {
    const sel = visibleSelection(doc);
    if (sel === null || sel.root === null) return '';
    let common = sel.start.c;
    while (common !== null && !contains(common, sel.end.c)) common = parentOf(common);
    while (common !== null && !(isElement(common) && formatBlockNames[nameOf(common)])) common = parentOf(common);
    if (common === null) return '';
    const root = rootEditable(sel.start.c);
    if (root === null || contains(common, root)) return '';
    return localOf(common);
  }
  const table = record();
  function command(name, exec, enabled, state, value, extra) {
    const entry = record();
    entry.exec = exec; entry.enabled = enabled; entry.state = state; entry.value = value; entry.stateful = state !== noState;
    entry.supported = true; entry.allowDisabled = false;
    if (extra) for (const k in extra) entry[k] = extra[k];
    table[lower(name)] = entry;
  }
  const valueEmpty = () => '';
  command('BackColor', (doc, v) => valueStyle(doc, 'background-color', v), inRich, noState, styleValue('background'));
  command('Bold', doc => toggle(doc, 'bold', 'formatBold'), inRich, styleState('bold'), null);
  command('Copy', never, never, noState, valueEmpty, { allowDisabled: true });
  command('CreateLink', execCreateLink, inRich, noState, valueEmpty);
  command('Cut', never, never, noState, valueEmpty, { allowDisabled: true });
  command('DefaultParagraphSeparator', (doc, v) => { const l = lower(v); if (l === 'div' || l === 'p') editorOf(doc).separator = l; return true; }, always, noState, doc => editorOf(doc).separator);
  command('Delete', doc => execDelete(doc, false), inEditable, noState, valueEmpty);
  command('FindString', execFindString, always, noState, valueEmpty);
  command('FontName', (doc, v) => valueStyle(doc, 'font-family', v), inRich, noState, styleValue('family'));
  command('FontSize', (doc, v) => valueStyle(doc, 'font-size', v), inRich, noState, styleValue('size'));
  command('FontSizeDelta', execFontSizeDelta, inRich, noState, valueEmpty);
  command('ForeColor', (doc, v) => valueStyle(doc, 'color', v), inRich, noState, styleValue('color'));
  command('FormatBlock', execFormatBlock, inRich, noState, formatBlockValue);
  command('ForwardDelete', doc => execDelete(doc, true), inEditable, noState, valueEmpty);
  command('HiliteColor', (doc, v) => valueStyle(doc, 'background-color', v), inRich, noState, valueEmpty);
  command('Indent', doc => execBlock(doc, 'formatIndent', s => indent(doc, s.root, s)), inRich, noState, valueEmpty);
  command('InsertHorizontalRule', (doc, v) => execInsertElement(doc, 'hr', 'id', v), inRich, noState, valueEmpty);
  command('InsertHTML', execInsertHTML, inEditable, noState, valueEmpty);
  command('InsertImage', (doc, v) => execInsertElement(doc, 'img', 'src', v), inRich, noState, valueEmpty);
  command('InsertLineBreak', doc => execParagraph(doc, true), inEditable, noState, valueEmpty);
  command('InsertOrderedList', doc => execList(doc, 'ol'), inRich, listState('ol'), null);
  command('InsertParagraph', doc => execParagraph(doc, false), inEditable, noState, valueEmpty);
  command('InsertText', execInsertText, inEditable, noState, valueEmpty);
  command('InsertUnorderedList', doc => execList(doc, 'ul'), inRich, listState('ul'), null);
  command('Italic', doc => toggle(doc, 'italic', 'formatItalic'), inRich, styleState('italic'), null);
  command('JustifyCenter', doc => execBlock(doc, 'formatJustifyCenter', s => justify(doc, s.root, s, 'center')), inRich, justifyState('center'), justifyValue);
  command('JustifyFull', doc => execBlock(doc, 'formatJustifyFull', s => justify(doc, s.root, s, 'justify')), inRich, justifyState('justify'), justifyValue);
  command('JustifyLeft', doc => execBlock(doc, 'formatJustifyLeft', s => justify(doc, s.root, s, 'left')), inRich, justifyState('left'), justifyValue);
  command('JustifyNone', doc => execBlock(doc, 'formatJustifyLeft', s => justify(doc, s.root, s, null)), inRich, noState, justifyValue);
  command('JustifyRight', doc => execBlock(doc, 'formatJustifyRight', s => justify(doc, s.root, s, 'right')), inRich, justifyState('right'), justifyValue);
  command('Outdent', doc => execBlock(doc, 'formatOutdent', s => outdent(doc, s.root, s)), inRich, noState, valueEmpty);
  command('Paste', never, never, noState, valueEmpty, { supported: false, allowDisabled: true });
  command('PasteAndMatchStyle', never, never, noState, valueEmpty, { allowDisabled: true });
  command('Print', execPrint, always, noState, valueEmpty);
  command('Redo', doc => { redo(doc); return true; }, canRedo, noState, valueEmpty);
  command('RemoveFormat', doc => removeStyled(doc, 'format', 'formatRemove', el => isElement(el) && (removableFormat[nameOf(el)] === 1 || (hasAttr(el, 'style') && !isBlock(el)))), rangeInRich, noState, valueEmpty);
  command('SelectAll', execSelectAll, selectAllEnabled, noState, valueEmpty);
  command('Strikethrough', doc => toggle(doc, 'strikethrough', 'formatStrikeThrough'), inRich, styleState('strikethrough'), null);
  command('StyleWithCSS', (doc, v) => { editorOf(doc).css = lower(v) !== 'false'; return true; }, always, doc => editorOf(doc).css, valueEmpty);
  command('Subscript', doc => toggle(doc, 'subscript', 'formatSubscript'), inRich, styleState('subscript'), null);
  command('Superscript', doc => toggle(doc, 'superscript', 'formatSuperscript'), inRich, styleState('superscript'), null);
  command('Transpose', execTranspose, caretInEditable, noState, valueEmpty);
  command('Underline', doc => toggle(doc, 'underline', 'formatUnderline'), inRich, styleState('underline'), null);
  command('Undo', doc => { undo(doc); return true; }, canUndo, noState, valueEmpty);
  command('Unlink', doc => removeStyled(doc, 'link', '', el => nameOf(el) === 'a'), rangeInRich, noState, valueEmpty);
  command('Unselect', execUnselect, visibleSelectionEnabled, noState, valueEmpty);
  command('UseCSS', (doc, v) => { editorOf(doc).css = lower(v) === 'false'; return true; }, always, noState, valueEmpty);
  table.stylewithcss.stateful = false;

  // ---- Document methods -----------------------------------------------------------------------------------
  const contentTypeOf = getter(DP, 'contentType');
  const documentType = receiver => { try { return typeOf(receiver) === 9; } catch (error) { return false; } };
  function checkDocument(receiver) { if (!documentType(receiver)) throw new ErrorType('Illegal invocation'); }
  const domString = (value, operation) => { if (typeof value === 'symbol') throw new ErrorType("Failed to execute '" + operation + "' on 'Document': Cannot convert a Symbol value to a string"); return StringValue(value); };
  function requireHTML(receiver, operation) {
    let type = '';
    try { type = contentTypeOf(receiver); } catch (error) { type = ''; }
    if (type !== 'text/html' && type !== 'application/xhtml+xml') throw new g.DOMException("Failed to execute '" + operation + "' on 'Document': " + operation + ' is only supported on HTML documents.', 'InvalidStateError');
  }
  function entryFor(doc, name) {
    if (doc !== document) return null;
    const entry = table[lowerAscii(name)];
    return entry !== undefined && entry.supported ? entry : null;
  }
  function lowerAscii(s) { let out = ''; for (let i = 0; i < s.length; i++) { const c = code(s, i); out += c >= 65 && c <= 90 ? fromCode(c + 32) : s[i]; } return out; }
  const argumentCount = (operation, args) => { if (args.length < 1) throw new ErrorType("Failed to execute '" + operation + "' on 'Document': 1 argument required, but only 0 present."); };
  function isEnabled(doc, entry) { refreshOpen(doc); return !!entry.enabled(doc); }
  function execCommand(commandId) {
    checkDocument(this);
    argumentCount('execCommand', arguments);
    const name = domString(commandId, 'execCommand');
    // showUI is converted (ToBoolean) and ignored, as in Chrome.
    const value = arguments.length > 2 && arguments[2] !== undefined ? domString(arguments[2], 'execCommand') : '';
    requireHTML(this, 'execCommand');
    const editor = editorOf(this);
    if (editor.running) return false;
    streamCache = null; resetMemo();
    const entry = entryFor(this, name);
    if (entry === null) return false;
    editor.running = true;
    try {
      if (!isEnabled(this, entry) && !entry.allowDisabled) return false;
      return !!entry.exec(this, value);
    } finally { editor.running = false; marks = list(); streamCache = null; }
  }
  function query(operation, body) {
    return function (commandId) {
      checkDocument(this);
      argumentCount(operation, arguments);
      const name = domString(commandId, operation);
      requireHTML(this, operation);
      streamCache = null; resetMemo();
      try { return body(this, entryFor(this, name)); } finally { streamCache = null; }
    };
  }
  const methods = [
    ['execCommand', execCommand],
    ['queryCommandEnabled', query('queryCommandEnabled', (doc, entry) => entry !== null && isEnabled(doc, entry))],
    ['queryCommandIndeterm', query('queryCommandIndeterm', () => false)],
    ['queryCommandState', query('queryCommandState', (doc, entry) => entry !== null && !!entry.state(doc))],
    ['queryCommandSupported', query('queryCommandSupported', (doc, entry) => entry !== null)],
    ['queryCommandValue', query('queryCommandValue', (doc, entry) => {
      if (entry === null) return '';
      if (entry.value === null) return entry.state(doc) ? 'true' : 'false';
      return StringValue(entry.value(doc));
    })],
  ];
  // ---- trusted user input (embedder input drivers only) ---------------------------------------------------
  // Chrome's typing path, unlike execCommand, fires a cancelable beforeinput carrying the target
  // ranges first, then runs the same typing command (and its input event). An element with an
  // EditContext receives the text in its EditContext instead of the DOM.
  // The upstream-most equivalent DOM position inside the same block (an element offset at a block's start).
  function mostBackward(pos) {
    let p = pos;
    while (p.o === 0 && p.c !== null && parentOf(p.c) !== null && !(isElement(p.c) && isBlock(p.c))) p = before(p.c);
    return p;
  }
  function staticRange(start, end) { return new StaticRangeType({ __proto__: null, startContainer: start.c, startOffset: start.o, endContainer: end.c, endOffset: end.o }); }
  function userInput(target, inputType, data) {
    const doc = typeOf(target) === 9 ? target : ownerOf(target);
    if (doc !== document) return false;
    streamCache = null; resetMemo();
    try {
      if (focusedControl(doc) !== null) return false;
      const active = activeOf(doc);
      if (active !== null && editContexts && editContexts.contextOf(active) !== null)
        return editContexts.input(active, inputType, data, (element, type, value) => fireBeforeInput(element, type, value, list()));
      const sel = editableSelection(doc);
      if (sel === null) {
        // Keyboard input still reaches a focused editing host whose document selection moved into
        // static content: beforeinput fires there and nothing is edited.
        const raw = rawSelection(doc);
        if (active !== null && isElement(active) && isEditable(active) && raw !== null) fireBeforeInput(active, inputType, inputType === 'insertText' ? data : null, [staticRange(raw.start, raw.end)]);
        return false;
      }
      const ranges = list();
      if (inputType === 'deleteContentBackward' || inputType === 'deleteContentForward') {
        const forward = inputType === 'deleteContentForward';
        const r = sel.caret ? deletionRange(sel.root, sel.start, forward) : [sel.start, sel.end];
        if (r === null) push(ranges, staticRange(sel.start, sel.start));
        else push(ranges, staticRange(r[0], r[2] && !forward ? mostBackward(r[1]) : r[1]));
      } else push(ranges, staticRange(sel.start, sel.end));
      if (!fireBeforeInput(sel.root, inputType, inputType === 'insertText' ? data : null, ranges)) return true;
      refreshOpen(doc);
      if (inputType === 'insertText') execInsertText(doc, data);
      else if (inputType === 'deleteContentBackward') execDelete(doc, false);
      else if (inputType === 'deleteContentForward') execDelete(doc, true);
      else if (inputType === 'insertParagraph') execParagraph(doc, false);
      else if (inputType === 'insertLineBreak') execParagraph(doc, true);
      return true;
    } finally { marks = list(); streamCache = null; }
  }
  native('capture', userInput);

  for (let i = 0; i < methods.length; i++) define(DP, methods[i][0], { __proto__: null, value: make(methods[i][1], methods[i][0], 1), writable: true, enumerable: true, configurable: true });

  // ---- HTMLElement.contentEditable / isContentEditable ----------------------------------------------------
  const contentEditableDescriptor = getOwn(HP, 'contentEditable'), isEditableDescriptor = getOwn(HP, 'isContentEditable');
  const brandGet = contentEditableDescriptor && contentEditableDescriptor.get;
  const brand = receiver => { if (brandGet) apply(brandGet, receiver, []); else if (!(receiver instanceof g.HTMLElement)) throw new ErrorType('Illegal invocation'); };
  define(HP, 'contentEditable', {
    __proto__: null, enumerable: true, configurable: true,
    get: make(function () {
      brand(this);
      const value = attr(this, 'contenteditable');
      if (value === null) return 'inherit';
      const v = lowerAscii(value);
      return v === '' || v === 'true' ? 'true' : v === 'false' ? 'false' : v === 'plaintext-only' ? 'plaintext-only' : 'inherit';
    }, 'get contentEditable', 0),
    set: make(function (value) {
      brand(this);
      if (typeof value === 'symbol') throw new ErrorType("Failed to set the 'contentEditable' property on 'HTMLElement': Cannot convert a Symbol value to a string");
      const text = StringValue(value), v = lowerAscii(text);
      if (v === 'true' || v === 'false' || v === 'plaintext-only') apply(setAttrF, this, ['contenteditable', v]);
      else if (v === 'inherit') apply(removeAttrF, this, ['contenteditable']);
      else throw new g.DOMException("Failed to set the 'contentEditable' property on 'HTMLElement': The value provided ('" + text + "') is not one of 'true', 'false', 'plaintext-only', or 'inherit'.", 'SyntaxError');
    }, 'set contentEditable', 1),
  });
  define(HP, 'isContentEditable', {
    __proto__: null, enumerable: true, configurable: true,
    get: make(function () {
      if (isEditableDescriptor && isEditableDescriptor.get) apply(isEditableDescriptor.get, this, []); else brand(this);
      return editState(this) !== null;
    }, 'get isContentEditable', 0),
  });

  // ---- InputEvent.getTargetRanges for engine-created events -------------------------------------------------
  const targetRangesDescriptor = getOwn(InputEventType.prototype, 'getTargetRanges');
  if (targetRangesDescriptor && typeof targetRangesDescriptor.value === 'function') {
    const original = targetRangesDescriptor.value;
    define(InputEventType.prototype, 'getTargetRanges', {
      __proto__: null, writable: true, enumerable: true, configurable: true,
      value: make(function getTargetRanges() {
        apply(original, this, []);
        const ranges = wget(targetRanges, this), out = [];
        if (ranges) for (let i = 0; i < ranges.length; i++) push(out, ranges[i]);
        return out;
      }, 'getTargetRanges', 0),
    });
  }
})();

// Selection API (Chrome 156 / Blink DOMSelection + FrameSelection + SelectionModifier).
// One frame selection per document with a browsing context: the editor anchor/focus (live
// with Blink's SelectionEditor mutation rules, see range_live.rs), its directional flag, the
// document's cached live Range, and per-ShadowRoot shadow-adjusted views. Text controls keep
// their own cached selection; while a focused control holds the frame selection its anchor
// and focus live inside the control's (virtual) inner editor. Caret movement and plain text
// use the native visible-text model (selection.rs).
(function (Native, record, staticRecord) {
  'use strict';
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const apply = Reflect.apply, define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor;
  const create = Object.create, getPrototype = Object.getPrototypeOf, setPrototype = Object.setPrototypeOf;
  const TypeErrorClass = TypeError, StringClass = String, NumberClass = Number, Exception = g.DOMException;
  const EventClass = g.Event, RangeClass = g.Range, StaticRangeClass = g.StaticRange;
  const WeakMapClass = WeakMap, weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set, weakHas = WeakMap.prototype.has;
  const min = Math.min, max = Math.max, floor = Math.floor;
  const sliceText = String.prototype.slice, charCode = String.prototype.charCodeAt, lower = String.prototype.toLowerCase;
  // Runtime patterns carry their own exec so guest RegExp.prototype changes stay unseen.
  const regexTest = RegExp.prototype.test, regexExec = RegExp.prototype.exec;
  const pattern = source => { const result = new RegExp(source, 'u'); define(result, 'exec', {__proto__: null, value: regexExec}); return result; };
  const test = (regex, text) => apply(regexTest, regex, [text]);
  const slice = (text, from, to) => apply(sliceText, text, [from, to]);
  const code = (text, index) => apply(charCode, text, [index]);
  const ascii = text => apply(lower, text, []);
  const mapGet = (map, key) => apply(weakGet, map, [key]);
  const mapSet = (map, key, value) => apply(weakSet, map, [key, value]);
  const mapHas = (map, key) => apply(weakHas, map, [key]);
  const push = (list, value) => define(list, list.length, {__proto__: null, value, writable: true, enumerable: true, configurable: true});
  function descriptor(proto, name) {
    while (proto) { const found = describe(proto, name); if (found) return found; proto = getPrototype(proto); }
    throw new TypeErrorClass('Missing native DOM member: ' + name);
  }
  const reader = (proto, name) => { const getter = descriptor(proto, name).get; return node => apply(getter, node, []); };
  const method = (proto, name) => { const fn = descriptor(proto, name).value; return (node, ...args) => apply(fn, node, args); };
  const nodeType = reader(g.Node.prototype, 'nodeType'), parent = reader(g.Node.prototype, 'parentNode');
  const firstChild = reader(g.Node.prototype, 'firstChild'), nextSibling = reader(g.Node.prototype, 'nextSibling');
  const ownerDocument = reader(g.Node.prototype, 'ownerDocument'), connected = reader(g.Node.prototype, 'isConnected');
  const nodeName = reader(g.Node.prototype, 'nodeName'), characterData = reader(g.CharacterData.prototype, 'data');
  const rootNode = method(g.Node.prototype, 'getRootNode'), hostOfRoot = reader(g.ShadowRoot.prototype, 'host');
  const localName = reader(g.Element.prototype, 'localName'), namespace = reader(g.Element.prototype, 'namespaceURI');
  const getAttribute = method(g.Element.prototype, 'getAttribute');
  const focusElement = method(g.HTMLElement.prototype, 'focus');
  const rangeStart = method(RangeClass.prototype, 'setStart'), rangeEnd = method(RangeClass.prototype, 'setEnd');
  const rangeCollapse = method(RangeClass.prototype, 'collapse'), rangeClone = method(RangeClass.prototype, 'cloneRange');
  const rangeDelete = method(RangeClass.prototype, 'deleteContents');
  const HTML = 'http://www.w3.org/1999/xhtml';
  const isCharacter = kind => kind === 3 || kind === 4 || kind === 7 || kind === 8;
  const isNode = value => { if (value === null || (typeof value !== 'object' && typeof value !== 'function')) return false; try { nodeType(value); return true; } catch (_) { return false; } };
  const isShadowRoot = value => { if (!isNode(value) || nodeType(value) !== 11) return false; try { hostOfRoot(value); return true; } catch (_) { return false; } };
  const childCount = node => { let count = 0; for (let child = firstChild(node); child; child = nextSibling(child)) count++; return count; };
  const nodeLength = node => { const kind = nodeType(node); return isCharacter(kind) ? characterData(node).length : kind === 10 ? 0 : childCount(node); };
  const indexOf = node => { const container = parent(node); let at = 0; for (let child = firstChild(container); child && child !== node; child = nextSibling(child)) at++; return at; };
  const html = (node, name) => nodeType(node) === 1 && namespace(node) === HTML && localName(node) === name;
  const point = (node, offset) => { const result = create(null); result.node = node; result.offset = offset; result.inner = false; return result; };
  const innerPoint = (control, offset) => { const result = create(null); result.node = control; result.offset = offset; result.inner = true; return result; };
  const samePoint = (a, b) => a.node === b.node && a.offset === b.offset && a.inner === b.inner;
  const fail = (operation, message, name) => new Exception("Failed to execute '" + operation + "' on 'Selection': " + message, name);
  const typeFail = (operation, message) => new TypeErrorClass("Failed to execute '" + operation + "' on 'Selection': " + message);
  const required = (args, count, operation) => {
    if (args.length < count) throw typeFail(operation, count + ' argument' + (count === 1 ? '' : 's') + ' required, but only ' + args.length + ' present.');
  };
  // Web IDL DOMString / unsigned long conversion; `prefix` names the failing operation.
  const string = (value, prefix = '') => { if (typeof value === 'symbol') throw new TypeErrorClass(prefix + 'Cannot convert a Symbol value to a string'); return StringClass(value); };
  const unsigned = (value, prefix = '') => {
    if (typeof value === 'symbol') throw new TypeErrorClass(prefix + 'Cannot convert a Symbol value to a number');
    if (typeof value === 'bigint') throw new TypeErrorClass(prefix + 'Cannot convert a BigInt value to a number');
    return NumberClass(value) >>> 0;
  };
  const on = operation => "Failed to execute '" + operation + "' on 'Selection': ";
  const nodeArgument = (value, index, operation, nullable) => {
    if (nullable && (value === null || value === undefined)) return null;
    if (!isNode(value)) throw typeFail(operation, 'parameter ' + index + " is not of type 'Node'.");
    return value;
  };

  // ---- Tree scopes and position order (Blink ComparePositionsInDOMTree) --------------------
  const innerScopes = new WeakMapClass();
  const innerScope = control => { let scope = mapGet(innerScopes, control); if (!scope) { scope = create(null); scope.control = control; mapSet(innerScopes, control, scope); } return scope; };
  const scopeHost = scope => { if (!isNode(scope)) return scope.control; return isShadowRoot(scope) ? hostOfRoot(scope) : null; };
  // [{scope, node}] from the position's own tree scope outward through shadow hosts.
  function chain(position) {
    const rows = [];
    let scope = position.inner ? innerScope(position.node) : rootNode(position.node);
    let node = position.inner ? null : position.node;
    for (;;) {
      const entry = create(null); entry.scope = scope; entry.node = node; push(rows, entry);
      const host = scopeHost(scope);
      if (!host) return rows;
      node = host; scope = rootNode(host);
    }
  }
  function ancestor(node, descendant) { while (descendant) { if (node === descendant) return true; descendant = parent(descendant); } return false; }
  function treeCompare(a, aOffset, b, bOffset) {
    if (a === b) return aOffset < bOffset ? -1 : aOffset > bOffset ? 1 : 0;
    let shared = a; while (shared && !ancestor(shared, b)) shared = parent(shared);
    if (!shared) return 0;
    if (shared === a) { let child = b; while (parent(child) !== shared) child = parent(child); return aOffset <= indexOf(child) ? -1 : 1; }
    if (shared === b) return -treeCompare(b, bOffset, a, aOffset);
    let left = a, right = b;
    while (parent(left) !== shared) left = parent(left);
    while (parent(right) !== shared) right = parent(right);
    return indexOf(left) < indexOf(right) ? -1 : 1;
  }
  function compare(a, b) {
    if (a.inner && b.inner && a.node === b.node) return a.offset < b.offset ? -1 : a.offset > b.offset ? 1 : 0;
    const left = chain(a), right = chain(b);
    for (let i = 0; i < left.length; i++) for (let j = 0; j < right.length; j++) {
      if (left[i].scope !== right[j].scope) continue;
      const deepA = i > 0, deepB = j > 0;
      const result = treeCompare(left[i].node, deepA ? 0 : a.offset, right[j].node, deepB ? 0 : b.offset);
      if (result) return result;
      return deepA && !deepB ? 1 : !deepA && deepB ? -1 : 0;
    }
    return 0;
  }
  const scopeOf = position => position.inner ? innerScope(position.node) : rootNode(position.node);
  // Blink DOMSelection::ShadowAdjustedNode/Offset for a view rooted at `scope`.
  function adjust(position, scope) {
    const rows = chain(position);
    for (let i = 0; i < rows.length; i++) {
      if (rows[i].scope !== scope) continue;
      if (i === 0) return position;
      const host = rows[i].node;
      return point(parent(host), indexOf(host));
    }
    return null;
  }
  const after = node => point(parent(node), indexOf(node) + 1), before = node => point(parent(node), indexOf(node));
  const container = node => { const kind = nodeType(node); return kind === 1 || kind === 9 || kind === 11; };
  // SelectionAdjuster::AdjustSelectionToAvoidCrossingShadowBoundaries.
  function avoidCrossing(anchor, focus) {
    const scope = scopeOf(anchor);
    if (scopeOf(focus) === scope || anchor.inner || focus.inner) return focus;
    const forward = compare(anchor, focus) <= 0, rows = chain(focus);
    for (let i = 0; i < rows.length; i++) {
      if (rows[i].scope !== scope) continue;
      const host = rows[i].node;
      if (forward) return ancestor(host, anchor.node) ? after(host) : container(host) ? point(host, childCount(host)) : after(host);
      return ancestor(host, anchor.node) ? before(host) : container(host) ? point(host, 0) : before(host);
    }
    // Outside every host of the anchor's scope: the scope's edge, which canonicalizes.
    let edge = null;
    if (forward) { let last = null; for (let child = firstChild(scope); child; child = nextSibling(child)) last = child; edge = last ? after(last) : null; }
    else { const first = firstChild(scope); edge = first ? before(first) : null; }
    if (!edge) return focus;
    edge.edge = true;
    return edge;
  }

  // ---- Per-document frame selection --------------------------------------------------------
  const states = new WeakMapClass(), views = new WeakMapClass(), byRecord = new WeakMapClass();
  const SelectionPrototype = create(g.Object.prototype);
  function stateOf(document) {
    let state = mapGet(states, document);
    if (state) return state;
    state = create(null);
    state.document = document; state.directional = false; state.control = null;
    state.anchor = 0; state.focus = 0; state.cache = null; state.pending = false; state.xpos = null;
    state.edgeAnchor = false; state.edgeFocus = false;
    // The editor positions: start pair = anchor, end pair = focus (null containers: none).
    const editor = create(null);
    editor.startContainer = null; editor.startOffset = 0; editor.endContainer = null; editor.endOffset = 0;
    state.editor = editor;
    Native('track', document, editor, () => { state.xpos = null; scheduleChange(state); });
    state.roots = new WeakMapClass();
    state.view = view(state, null);
    mapSet(states, document, state);
    return state;
  }
  function view(state, root) {
    const result = create(SelectionPrototype), entry = create(null);
    entry.state = state; entry.root = root;
    mapSet(views, result, entry);
    return result;
  }
  function entryOf(receiver) {
    const entry = (receiver !== null && typeof receiver === 'object') ? mapGet(views, receiver) : undefined;
    if (!entry) throw new TypeErrorClass('Illegal invocation');
    return entry;
  }
  // Control mode lasts while the editor stays at (control, 0) and the control is connected.
  function controlOf(state) {
    const control = state.control;
    if (control !== null && (state.editor.startContainer !== control || !connected(control))) state.control = null;
    return state.control;
  }
  const none = state => state.editor.startContainer === null && controlOf(state) === null;
  function anchorPoint(state) { const control = controlOf(state); return control ? innerPoint(control, state.anchor) : point(state.editor.startContainer, state.editor.startOffset); }
  function focusPoint(state) { const control = controlOf(state); return control ? innerPoint(control, state.focus) : point(state.editor.endContainer, state.editor.endOffset); }
  const anchorFirst = state => compare(anchorPoint(state), focusPoint(state)) <= 0;
  const caret = state => samePoint(anchorPoint(state), focusPoint(state));
  const valid = (state, node) => (nodeType(node) === 9 ? node : ownerDocument(node)) === state.document && connected(node);

  function scheduleChange(state) {
    const control = controlOf(state);
    if (control) return scheduleElement(control, 'selectionchange');
    if (state.pending) return;
    state.pending = true;
    Native('schedule', () => { state.pending = false; dispatch(state.document, 'selectionchange', false); });
  }
  const pendingEvents = new WeakMapClass();
  function scheduleElement(element, type) {
    // select events queue one per change; selectionchange coalesces until it fires.
    if (type === 'select') { Native('schedule', () => dispatch(element, type, true)); return; }
    let pending = mapGet(pendingEvents, element);
    if (!pending) { pending = create(null); mapSet(pendingEvents, element, pending); }
    if (pending[type]) return;
    pending[type] = true;
    Native('schedule', () => { pending[type] = false; dispatch(element, type, true); });
  }
  function dispatch(target, type, bubbles) {
    const options = create(null); options.bubbles = bubbles;
    Native('dispatch', target, new EventClass(type, options));
  }
  // FrameSelection::SetSelection + DidSetSelection: positions, directionality, caching,
  // selectionchange and SetFocusedNodeIfNeeded. Returns whether the selection changed.
  function setSelection(state, anchor, focus, directional, cached, viewRoot, options) {
    const before = none(state) ? null : [anchorPoint(state), focusPoint(state)];
    const control = anchor && anchor.inner ? anchor.node : null;
    const changed = before === null ? anchor !== null : anchor === null || !samePoint(before[0], anchor) || !samePoint(before[1], focus) || state.directional !== directional;
    if (!changed) { if (viewRoot === null && cached !== undefined) state.cache = cached; return false; }
    state.cache = null; state.xpos = null;
    state.edgeAnchor = !!(options && options.edgeAnchor); state.edgeFocus = !!(options && options.edgeFocus);
    const editor = state.editor;
    state.control = control;
    if (anchor === null) {
      editor.startContainer = editor.endContainer = null; editor.startOffset = editor.endOffset = 0;
      state.directional = false;
    } else if (control) {
      editor.startContainer = editor.endContainer = control; editor.startOffset = editor.endOffset = 0;
      state.anchor = anchor.offset; state.focus = focus.offset; state.directional = directional;
    } else {
      editor.startContainer = anchor.node; editor.startOffset = anchor.offset;
      editor.endContainer = focus.node; editor.endOffset = focus.offset;
      state.directional = directional;
    }
    if (viewRoot === null && cached !== undefined) state.cache = cached;
    if (control) controlSelectionChanged(control, state);
    scheduleChange(state);
    if (anchor !== null && !control && !(options && options.keepFocus)) {
      const host = Native('host', anchor.node);
      if (host !== null && Native('focused', state.document) !== host) focusElement(host);
    }
    return true;
  }
  const clear = (state, root) => setSelection(state, null, null, false, null, root, null);
  const rangeState = range => record(range);
  // The private record of a genuine Range argument, or undefined.
  const rangeArgument = value => {
    if (value === null || (typeof value !== 'object' && typeof value !== 'function')) return undefined;
    try { const data = record(value); return data && data.startContainer ? data : undefined; } catch (_) { return undefined; }
  };
  function newRange(document, start, end) {
    const range = new RangeClass();
    rangeStart(range, start.node, start.offset);
    rangeEnd(range, end.node, end.offset);
    return range;
  }
  // Range::UpdateSelectionIfAddedToSelection for the document's cached range.
  function sync(rangeRecord) {
    const state = mapGet(byRecord, rangeRecord);
    if (!state || state.cache === null || rangeState(state.cache) !== rangeRecord) return;
    const range = state.cache, start = point(rangeRecord.startContainer, rangeRecord.startOffset);
    const end = point(rangeRecord.endContainer, rangeRecord.endOffset);
    const node = start.node;
    if ((nodeType(node) === 9 ? node : ownerDocument(node)) !== state.document || !connected(node)) { clear(state, null); return; }
    setSelection(state, start, end, false, range, null, {__proto__: null, keepFocus: true});
  }
  function cacheRange(state, range) { if (range) mapSet(byRecord, rangeState(range), state); return range; }

  // ---- Visible selection (canonical positions, no shadow-crossing) ---------------------------
  // Visible text of one tree scope: {texts, slots, queries} (see selection.rs).
  function model(root, positions) {
    const args = ['model', root];
    for (let i = 0; i < positions.length; i++) { push(args, positions[i].node); push(args, positions[i].offset); }
    return apply(Native, undefined, args);
  }
  const slotPoint = (layout, segment, slot) => point(layout.slots[segment][slot * 2], layout.slots[segment][slot * 2 + 1]);
  // Resolve a query to [segment, slot] (forward to the next block, else backward to the end).
  function located(layout, query) {
    if (query[0] >= 0) return [query[0], query[1]];
    if (query[1] < layout.texts.length) return [query[1], 0];
    const last = layout.texts.length - 1;
    return last < 0 ? null : [last, layout.texts[last].length];
  }
  // `edge`: the position was produced by caret movement onto the after-table candidate.
  function locate(layout, index, position, edge) {
    if (edge) for (let i = 0; i < layout.edges.length; i++) if (layout.edges[i] === 2 && samePoint(slotPoint(layout, i, 0), position)) return [i, 0];
    return located(layout, layout.queries[index]);
  }
  function canonical(position, edge) {
    if (position.inner) return position;
    const layout = model(rootNode(position.node), [position]);
    const at = locate(layout, 0, position, edge);
    if (!at) return null;
    const result = slotPoint(layout, at[0], at[1]);
    result.forward = point(layout.forward[at[0]][at[1] * 2], layout.forward[at[0]][at[1] * 2 + 1]);
    return result;
  }
  // VisibleSelection: canonical endpoints, then the focus kept in the anchor's tree scope.
  // Null when either endpoint has no visible position (VisibleSelection::IsNone).
  function visible(state) {
    const anchor = canonical(anchorPoint(state), state.edgeAnchor);
    const canonicalFocus = canonical(focusPoint(state), state.edgeFocus);
    if (!anchor || !canonicalFocus) return null;
    let focus = avoidCrossing(anchor, canonicalFocus);
    // A scope-edge focus only moves to an equivalent caret inside its own block.
    if (focus.edge) {
      const layout = model(rootNode(focus.node), [focus]), query = layout.queries[0];
      if (query[0] >= 0) focus = slotPoint(layout, query[0], query[1]);
    }
    const result = create(null); result.anchor = anchor; result.focus = focus;
    const forward = compare(anchor, focus) <= 0;
    result.start = forward ? anchor : focus; result.end = forward ? focus : anchor;
    // A range's normalized start is its most-forward equivalent (no leading collapsed space).
    if (!samePoint(anchor, focus) && result.start.forward && compare(result.start.forward, result.end) <= 0) result.start = result.start.forward;
    return result;
  }
  // DOMSelection::CreateRangeFromSelectionEditor for the document (root null) or a ShadowRoot.
  function editorRange(state, root) {
    let vis = visible(state);
    if (!vis) {
      vis = create(null); vis.anchor = anchorPoint(state); vis.focus = avoidCrossing(vis.anchor, focusPoint(state));
      const forward = compare(vis.anchor, vis.focus) <= 0;
      vis.start = forward ? vis.anchor : vis.focus; vis.end = forward ? vis.focus : vis.anchor;
    }
    if (root === null && !vis.anchor.inner && rootNode(vis.anchor.node) === state.document) return newRange(state.document, vis.start, vis.end);
    const scope = root === null ? state.document : root;
    const anchor = adjust(vis.anchor, scope);
    if (!anchor) return null;
    const focus = adjust(vis.focus, scope) || anchor;
    return compare(anchor, focus) <= 0 ? newRange(state.document, anchor, focus) : newRange(state.document, focus, anchor);
  }
  function rangeCount(entry) {
    const state = entry.state;
    if (none(state)) return 0;
    if (entry.root === null) return 1;
    return editorRange(state, entry.root) ? 1 : 0;
  }
  function primary(entry) {
    const state = entry.state;
    if (none(state)) return null;
    if (entry.root === null) {
      if (state.cache === null) state.cache = cacheRange(state, editorRange(state, null));
      return state.cache;
    }
    return editorRange(state, entry.root);
  }
  function boundary(entry, focus) {
    const range = primary(entry);
    if (!range) return null;
    const data = rangeState(range), first = anchorFirst(entry.state);
    return focus === first ? point(data.endContainer, data.endOffset) : point(data.startContainer, data.startOffset);
  }
  function checkPoint(node, offset, operation) {
    const kind = nodeType(node);
    if (kind === 10) throw fail(operation, "The node provided is of type '" + nodeName(node) + "'.", 'InvalidNodeTypeError');
    if (isCharacter(kind)) { const length = characterData(node).length; if (offset > length) throw fail(operation, 'The offset ' + offset + " is larger than the node's length (" + length + ').', 'IndexSizeError'); }
    else if (offset > childCount(node)) throw fail(operation, 'There is no child at offset ' + offset + '.', 'IndexSizeError');
  }

  // ---- Selection interface -----------------------------------------------------------------
  const members = [];
  function getter(name, fn) { push(members, [name, 'get', fn]); }
  function operation(name, length, fn) { push(members, [name, 'value', fn, length]); }
  const anchorGetter = function (focus, offset) {
    return function () { const position = boundary(entryOf(this), focus); return offset ? (position ? position.offset : 0) : (position ? position.node : null); };
  };
  getter('anchorNode', anchorGetter(false, false));
  getter('anchorOffset', anchorGetter(false, true));
  getter('focusNode', anchorGetter(true, false));
  getter('focusOffset', anchorGetter(true, true));
  getter('isCollapsed', function () { const range = primary(entryOf(this)); if (!range) return true; const data = rangeState(range); return data.startContainer === data.endContainer && data.startOffset === data.endOffset; });
  getter('rangeCount', function () { return rangeCount(entryOf(this)); });
  getter('type', function () { const entry = entryOf(this); if (!rangeCount(entry)) return 'None'; return caret(entry.state) ? 'Caret' : 'Range'; });
  getter('direction', function () {
    const state = entryOf(this).state;
    if (none(state) || !state.directional || caret(state)) return 'none';
    if (!visible(state)) return 'none';
    return anchorFirst(state) ? 'forward' : 'backward';
  });
  getter('baseNode', anchorGetter(false, false));
  getter('baseOffset', anchorGetter(false, true));
  getter('extentNode', anchorGetter(true, false));
  getter('extentOffset', anchorGetter(true, true));
  operation('addRange', 1, function (range) {
    const entry = entryOf(this); required(arguments, 1, 'addRange');
    const data = rangeArgument(range);
    if (!data) throw typeFail('addRange', "parameter 1 is not of type 'Range'.");
    const state = entry.state, start = data.startContainer;
    if ((nodeType(start) === 9 ? start : ownerDocument(start)) !== state.document || !connected(start)) return;
    if (rangeCount(entry) !== 0) return;
    setSelection(state, point(data.startContainer, data.startOffset), point(data.endContainer, data.endOffset), false, cacheRange(state, range), entry.root, null);
  });
  function collapse(operationName) {
    return function (node, offset) {
      const entry = entryOf(this); required(arguments, 1, operationName);
      node = nodeArgument(node, 1, operationName, true); offset = offset === undefined ? 0 : unsigned(offset, on(operationName));
      const state = entry.state;
      if (node === null) { clear(state, entry.root); return; }
      checkPoint(node, offset, operationName);
      if (!valid(state, node)) return;
      const position = point(node, offset);
      setSelection(state, position, position, state.directional, cacheRange(state, newRange(state.document, position, position)), entry.root, null);
    };
  }
  operation('collapse', 1, collapse('collapse'));
  function collapseTo(end, operationName) {
    return function () {
      const entry = entryOf(this), state = entry.state;
      if (!rangeCount(entry)) throw fail(operationName, 'there is no selection.', 'InvalidStateError');
      if (entry.root === null && state.cache !== null) {
        const range = rangeClone(state.cache); rangeCollapse(range, !end);
        const data = rangeState(range), position = point(data.startContainer, data.startOffset);
        setSelection(state, position, position, state.directional, cacheRange(state, range), null, null);
        return;
      }
      const forward = anchorFirst(state), anchor = anchorPoint(state), focus = focusPoint(state);
      const position = end === forward ? focus : anchor;
      setSelection(state, position, position, state.directional, undefined, entry.root, null);
    };
  }
  operation('collapseToEnd', 0, collapseTo(true, 'collapseToEnd'));
  operation('collapseToStart', 0, collapseTo(false, 'collapseToStart'));
  operation('containsNode', 1, function (node, allowPartial) {
    const entry = entryOf(this); required(arguments, 1, 'containsNode');
    node = nodeArgument(node, 1, 'containsNode', false); allowPartial = !!allowPartial;
    const state = entry.state;
    if ((nodeType(node) === 9 ? node : ownerDocument(node)) !== state.document || none(state)) return false;
    const container = parent(node);
    if (!container || !connected(container)) return false;
    const vis = visible(state);
    if (!vis || vis.start.inner || vis.end.inner) return false;
    const index = indexOf(node), self = point(container, index), next = point(container, index + 1);
    const sameRoot = position => rootNode(position.node) === rootNode(container);
    if (!sameRoot(vis.start) || !sameRoot(vis.end)) return false;
    if (treeCompare(container, index, vis.start.node, vis.start.offset) >= 0 && treeCompare(container, index + 1, vis.end.node, vis.end.offset) <= 0) return true;
    if (treeCompare(self.node, self.offset, vis.end.node, vis.end.offset) > 0 || treeCompare(next.node, next.offset, vis.start.node, vis.start.offset) < 0) return false;
    return allowPartial || nodeType(node) === 3;
  });
  operation('deleteFromDocument', 0, function () {
    const entry = entryOf(this);
    if (!rangeCount(entry)) return;
    const range = primary(entry);
    if (range) rangeDelete(range);
  });
  operation('empty', 0, function () { const entry = entryOf(this); clear(entry.state, entry.root); });
  operation('extend', 1, function (node, offset) {
    const entry = entryOf(this); required(arguments, 1, 'extend');
    node = nodeArgument(node, 1, 'extend', false); offset = offset === undefined ? 0 : unsigned(offset, on('extend'));
    const state = entry.state;
    if (!valid(state, node)) return;
    if (!rangeCount(entry)) throw fail('extend', "This Selection object doesn't have any Ranges.", 'InvalidStateError');
    checkPoint(node, offset, 'extend');
    const anchor = anchorPoint(state), focus = point(node, offset);
    let range;
    if (anchor.inner || rootNode(anchor.node) !== rootNode(node)) range = newRange(state.document, focus, focus);
    else if (compare(anchor, focus) <= 0) range = newRange(state.document, anchor, focus);
    else range = newRange(state.document, focus, anchor);
    const data = rangeState(range), collapsed = data.startContainer === data.endContainer && data.startOffset === data.endOffset;
    setSelection(state, collapsed ? focus : anchor, focus, true, cacheRange(state, range), entry.root, null);
  });
  operation('getComposedRanges', 0, function (options) {
    const entry = entryOf(this), roots = [];
    if (options !== undefined && options !== null) {
      if (typeof options !== 'object' && typeof options !== 'function') throw typeFail('getComposedRanges', "The provided value is not of type 'GetComposedRangesOptions'.");
      const list = options.shadowRoots;
      if (list !== undefined) {
        const prefix = "Failed to read the 'shadowRoots' property from 'GetComposedRangesOptions': ";
        if (list === null || (typeof list !== 'object' && typeof list !== 'function') || typeof list[Symbol.iterator] !== 'function') throw typeFail('getComposedRanges', prefix + 'The provided value cannot be converted to a sequence.');
        for (const root of list) { if (!isShadowRoot(root)) throw typeFail('getComposedRanges', prefix + "Failed to convert value to 'ShadowRoot'."); push(roots, root); }
      }
    }
    const state = entry.state, result = [];
    if (none(state)) return result;
    let start = anchorPoint(state), end = focusPoint(state);
    if (compare(start, end) > 0) { const swap = start; start = end; end = swap; }
    // A scope stays when it is a shadow-including inclusive ancestor of a given root.
    const included = scope => {
      for (let i = 0; i < roots.length; i++) {
        for (let root = roots[i]; root; root = scopeHost(root) ? rootNode(scopeHost(root)) : null) if (root === scope) return true;
      }
      return false;
    };
    const rescope = (position, last) => {
      let scope = scopeOf(position);
      while (scopeHost(scope) && (position.inner || !included(scope))) {
        const host = scopeHost(scope);
        position = point(parent(host), indexOf(host) + (last ? 1 : 0));
        scope = rootNode(host);
      }
      return position;
    };
    start = rescope(start, false); end = rescope(end, true);
    const init = create(null);
    init.startContainer = start.node; init.startOffset = start.offset; init.endContainer = end.node; init.endOffset = end.offset;
    push(result, new StaticRangeClass(init));
    return result;
  });
  operation('getRangeAt', 1, function (index) {
    const entry = entryOf(this); required(arguments, 1, 'getRangeAt');
    index = unsigned(index, on('getRangeAt'));
    if (index >= rangeCount(entry)) throw fail('getRangeAt', index + ' is not a valid index.', 'IndexSizeError');
    return primary(entry);
  });
  operation('modify', 0, function (alter, direction, granularity) {
    const entry = entryOf(this);
    alter = ascii(string(alter, on('modify'))); direction = ascii(string(direction, on('modify'))); granularity = ascii(string(granularity, on('modify')));
    if (alter !== 'move' && alter !== 'extend') return;
    if (direction !== 'forward' && direction !== 'backward' && direction !== 'left' && direction !== 'right') return;
    if (!GRANULARITIES[granularity]) return;
    modify(entry.state, alter === 'extend', direction === 'forward' || direction === 'right', granularity);
  });
  operation('removeAllRanges', 0, function () { const entry = entryOf(this); clear(entry.state, entry.root); });
  operation('removeRange', 1, function (range) {
    const entry = entryOf(this); required(arguments, 1, 'removeRange');
    const data = rangeArgument(range);
    if (!data) throw typeFail('removeRange', "parameter 1 is not of type 'Range'.");
    if (range === primary(entry)) { clear(entry.state, entry.root); return; }
    throw fail('removeRange', 'Range not found.', 'NotFoundError');
  });
  operation('selectAllChildren', 1, function (node) {
    const entry = entryOf(this); required(arguments, 1, 'selectAllChildren');
    node = nodeArgument(node, 1, 'selectAllChildren', false);
    if (nodeType(node) === 10) throw fail('selectAllChildren', "The node provided is of type '" + nodeName(node) + "'.", 'InvalidNodeTypeError');
    const state = entry.state;
    if (!valid(state, node)) return;
    const start = point(node, 0), end = point(node, isCharacter(nodeType(node)) ? 0 : childCount(node));
    setSelection(state, start, end, true, cacheRange(state, newRange(state.document, start, end)), entry.root, null);
  });
  operation('setBaseAndExtent', 4, function (baseNode, baseOffset, extentNode, extentOffset) {
    const entry = entryOf(this); required(arguments, 4, 'setBaseAndExtent');
    baseNode = nodeArgument(baseNode, 1, 'setBaseAndExtent', true); baseOffset = unsigned(baseOffset, on('setBaseAndExtent'));
    extentNode = nodeArgument(extentNode, 3, 'setBaseAndExtent', true); extentOffset = unsigned(extentOffset, on('setBaseAndExtent'));
    const state = entry.state;
    if (baseNode === null) { clear(state, entry.root); return; }
    if (extentNode === null) extentOffset = 0;
    checkPoint(baseNode, baseOffset, 'setBaseAndExtent');
    if (extentNode !== null) checkPoint(extentNode, extentOffset, 'setBaseAndExtent');
    if (!valid(state, baseNode) || (extentNode !== null && !valid(state, extentNode))) return;
    const base = point(baseNode, baseOffset), extent = extentNode === null ? base : point(extentNode, extentOffset);
    const range = compare(base, extent) < 0 ? newRange(state.document, base, extent) : newRange(state.document, extent, base);
    setSelection(state, base, extent, true, cacheRange(state, range), entry.root, null);
  });
  operation('setPosition', 1, collapse('setPosition'));
  operation('toString', 0, function () {
    const state = entryOf(this).state;
    if (none(state)) return '';
    const vis = visible(state);
    if (!vis) return '';
    if (vis.start.inner) return slice(controlValue(vis.start.node), vis.start.offset, vis.end.offset);
    if (rootNode(vis.start.node) !== rootNode(vis.end.node) && scopeOf(vis.start) !== scopeOf(vis.end)) return '';
    return Native('text', state.document, vis.start.node, vis.start.offset, vis.end.node, vis.end.offset);
  });
  const illegal = name => function () { throw new TypeErrorClass(new.target === undefined ? 'Illegal constructor' : "Failed to construct '" + name + "': Illegal constructor"); };
  const Selection = make(illegal('Selection'), 'Selection', 0, true, true, false);
  for (let i = 0; i < members.length; i++) {
    const [name, kind, fn, length] = members[i];
    if (kind === 'get') define(SelectionPrototype, name, {__proto__: null, get: make(fn, 'get ' + name, 0), set: undefined, enumerable: true, configurable: true});
    else define(SelectionPrototype, name, {__proto__: null, value: make(fn, name, length), writable: true, enumerable: true, configurable: true});
  }
  define(SelectionPrototype, 'constructor', {__proto__: null, value: Selection, writable: true, enumerable: false, configurable: true});
  define(SelectionPrototype, Symbol.toStringTag, {__proto__: null, value: 'Selection', writable: false, enumerable: false, configurable: true});
  define(Selection, 'prototype', {__proto__: null, value: SelectionPrototype, writable: false, enumerable: false, configurable: false});
  define(g, 'Selection', {__proto__: null, value: Selection, writable: true, enumerable: false, configurable: true});

  const documentSelection = document => Native('active', document) ? stateOf(document).view : null;
  const checkDocument = ns.domAccessor('nodeType', false, 'Document');
  define(g.Document.prototype, 'getSelection', {__proto__: null, value: make(function () {
    apply(checkDocument, this, []);
    if (nodeType(this) !== 9) throw new TypeErrorClass('Illegal invocation');
    return documentSelection(this);
  }, 'getSelection', 0), writable: true, enumerable: true, configurable: true});
  define(g.ShadowRoot.prototype, 'getSelection', {__proto__: null, value: make(function () {
    if (!isShadowRoot(this)) throw new TypeErrorClass('Illegal invocation');
    const document = ownerDocument(this);
    if (!Native('active', document)) return null;
    const state = stateOf(document);
    let result = mapGet(state.roots, this);
    if (!result) { result = view(state, this); mapSet(state.roots, this, result); }
    return result;
  }, 'getSelection', 0), writable: true, enumerable: true, configurable: true});
  const windowDocument = describe(g, 'document');
  const currentDocument = () => windowDocument && windowDocument.get ? apply(windowDocument.get, g, []) : windowDocument ? windowDocument.value : g.document;
  define(g, 'getSelection', {__proto__: null, value: make(function () {
    const document = currentDocument();
    return document ? documentSelection(document) : null;
  }, 'getSelection', 0), writable: true, enumerable: true, configurable: true});
  for (const document of [g.document]) if (document && Object.hasOwn(document, 'getSelection')) delete document.getSelection;

  // ---- Caret movement (SelectionModifier) --------------------------------------------------
  const GRANULARITIES = create(null);
  for (const name of ['character', 'word', 'sentence', 'line', 'paragraph', 'lineboundary', 'sentenceboundary', 'paragraphboundary', 'documentboundary']) GRANULARITIES[name] = true;
  const SegmenterClass = g.Intl.Segmenter, segmentMethod = SegmenterClass.prototype.segment;
  const segmenters = create(null);
  const sampleSegments = apply(segmentMethod, new SegmenterClass('en-US', {__proto__: null, granularity: 'word'}), ['a b']);
  const segmentsIterator = getPrototype(sampleSegments)[Symbol.iterator];
  const segmentNext = getPrototype(apply(segmentsIterator, sampleSegments, [])).next;
  // Boundary offsets (including 0 and length) of `text` for an Intl.Segmenter granularity.
  function boundaries(text, granularity) {
    let segmenter = segmenters[granularity];
    if (!segmenter) { const options = create(null); options.granularity = granularity; segmenter = segmenters[granularity] = new SegmenterClass('en-US', options); }
    const iterator = apply(segmentsIterator, apply(segmentMethod, segmenter, [text]), []), result = [];
    for (;;) { const step = apply(segmentNext, iterator, []); if (step.done) break; push(result, step.value.index); }
    push(result, text.length);
    return result;
  }
  const following = (list, offset) => { for (let i = 0; i < list.length; i++) if (list[i] > offset) return list[i]; return -1; };
  const preceding = (list, offset) => { for (let i = list.length - 1; i >= 0; i--) if (list[i] < offset) return list[i]; return -1; };
  const lineBreak = unit => unit === 10 || unit === 13;
  const PUNCT = pattern('^\\p{P}$'), ALNUM = pattern('^[\\p{L}\\p{Nd}]$'), SPACE = pattern('^\\s$'), CONTROL = pattern('^\\p{Cc}$');
  const punct = ch => test(PUNCT, ch);
  const alnum = ch => test(ALNUM, ch);
  const surrogate = unit => unit >= 0xD800 && unit <= 0xDFFF;
  const wordBreak = (text, index) => { const unit = code(text, index); return surrogate(unit) || lineBreak(unit) || unit === 0x5F || (!test(SPACE, text[index]) && !test(CONTROL, text[index])); };
  // Blink NextWordPosition / PreviousWordPosition finders over one text segment.
  function nextWord(text, offset, first) {
    if (!first && offset < text.length && wordBreak(text, offset)) return offset;
    if (offset === text.length || !text.length) return -1;
    const list = boundaries(text, 'word');
    for (let runner = following(list, offset); runner !== -1; runner = following(list, runner)) {
      if (runner < text.length && lineBreak(code(text, runner))) return runner;
      if (runner < text.length && punct(text[runner])) { if (alnum(text[runner - 1])) return runner; continue; }
      if (runner < text.length && wordBreak(text, runner - 1)) return runner;
    }
    return lineBreak(code(text, text.length - 1)) ? -1 : text.length;
  }
  function previousWord(text, offset, first) {
    if (!first && offset > 0 && wordBreak(text, offset - 1)) return offset;
    if (offset === 0) return -1;
    const list = boundaries(text, 'word');
    for (let runner = preceding(list, offset); runner !== -1; runner = preceding(list, runner)) {
      if (lineBreak(code(text, runner))) return runner + 1;
      if (runner < text.length && punct(text[runner])) { if (runner === 0 || alnum(text[runner - 1])) return runner; continue; }
      if (runner < text.length && wordBreak(text, runner)) return runner;
    }
    return 0;
  }
  const graphemes = text => boundaries(text, 'grapheme');
  // Paragraph bounds inside a segment (newline-delimited).
  function paragraphStart(text, offset) { while (offset > 0 && !lineBreak(code(text, offset - 1))) offset--; return offset; }
  function paragraphEnd(text, offset) { while (offset < text.length && !lineBreak(code(text, offset))) offset++; return offset; }
  // A layout over segments: {texts, at(segment, slot) -> position}. Text controls use their value.
  function controlLayout(control) {
    const layout = create(null), value = controlValue(control);
    layout.texts = [value]; layout.control = control;
    return layout;
  }
  const at = (layout, segment, slot) => layout.control ? innerPoint(layout.control, slot) : slotPoint(layout, segment, slot);
  // Per granularity: next/previous caret position from [segment, slot], or null.
  function step(layout, from, forward, granularity, startEdge) {
    const texts = layout.texts;
    let segment = from[0], slot = from[1];
    const text = texts[segment];
    switch (granularity) {
      case 'character': {
        if (forward) {
          if (slot < text.length) return [segment, following(graphemes(text), slot)];
          return segment + 1 < texts.length ? [segment + 1, 0] : null;
        }
        if (slot > 0) return [segment, preceding(graphemes(text), slot)];
        return segment > 0 ? [segment - 1, texts[segment - 1].length] : null;
      }
      case 'word': {
        let first = true;
        if (forward) {
          for (; segment < texts.length; segment++, slot = 0, first = false) { const found = nextWord(texts[segment], slot, first); if (found !== -1) return [segment, found]; }
          return null;
        }
        for (; segment >= 0; first = false) {
          const found = previousWord(texts[segment], slot, first);
          if (found !== -1 && !(first && found === slot)) return [segment, found];
          segment--; if (segment >= 0) slot = texts[segment].length;
        }
        return null;
      }
      case 'sentence': {
        if (forward) {
          if (slot === text.length) return segment + 1 < texts.length ? [segment + 1, 0] : null;
          const found = following(boundaries(text, 'sentence'), slot);
          return found === -1 ? null : [segment, found];
        }
        const found = preceding(boundaries(text, 'sentence'), slot);
        if (found !== -1) return [segment, found];
        return segment > 0 ? [segment - 1, 0] : [segment, 0];
      }
      case 'sentenceboundary': {
        if (forward) { const found = following(boundaries(text, 'sentence'), slot); return [segment, found === -1 ? text.length : found]; }
        const found = preceding(boundaries(text, 'sentence'), slot);
        return [segment, found === -1 ? 0 : found];
      }
      case 'paragraphboundary': case 'lineboundary':
        return [segment, forward ? paragraphEnd(text, slot) : paragraphStart(text, slot)];
      case 'documentboundary':
        return forward ? [texts.length - 1, texts[texts.length - 1].length] : [0, 0];
      case 'line': case 'paragraph': {
        // Without layout geometry every caret has x = 0: the target is the adjacent
        // paragraph's start; past the last (first) paragraph the caret goes to its end (start).
        if (forward) {
          const end = paragraphEnd(text, slot);
          if (end < text.length) return [segment, end + 1];
          return segment + 1 < texts.length ? [segment + 1, 0] : [segment, text.length];
        }
        const start = paragraphStart(text, slot);
        if (start > 0) return [segment, paragraphStart(text, start - 1)];
        return segment > 0 ? [segment - 1, paragraphStart(texts[segment - 1], texts[segment - 1].length)] : [segment, 0];
      }
    }
    return null;
  }
  // An embedder with layout registers caret geometry once: {caret(node, offset) ->
  // {x, top, bottom} | null, at(document, x, y) -> {node, offset} | null}. Line moves then
  // follow line boxes and keep the caret's x like Blink; without it every x is 0 and a
  // line is a whole paragraph (no wrapping).
  let caretLayout = null;
  define(ns, 'registerSelectionLayout', {__proto__: null, configurable: true, value: hooks => { if (hooks && caretLayout === null) caretLayout = hooks; delete ns.registerSelectionLayout; }});
  function geometric(layout, root, from, forward, granularity, state) {
    const texts = layout.texts;
    const box = (segment, slot) => { const position = slotPoint(layout, segment, slot); return caretLayout.caret(position.node, position.offset); };
    const here = box(from[0], from[1]);
    if (!here) return undefined;
    const next = position => {
      const segment = position[0], slot = position[1];
      if (forward) return slot < texts[segment].length ? [segment, slot + 1] : segment + 1 < texts.length ? [segment + 1, 0] : null;
      return slot > 0 ? [segment, slot - 1] : segment > 0 ? [segment - 1, texts[segment - 1].length] : null;
    };
    if (granularity === 'lineboundary') {
      let edge = from;
      for (let position = next(from); position && position[0] === from[0]; position = next(position)) {
        const candidate = box(position[0], position[1]);
        if (!candidate || candidate.top >= here.bottom - 0.5 || candidate.bottom <= here.top + 0.5) break;
        edge = position;
      }
      return edge;
    }
    const x = state.xpos !== null ? state.xpos : here.x;
    const adjacent = (start, current) => {
      for (let position = next(start); position; position = next(position)) {
        const candidate = box(position[0], position[1]);
        if (!candidate) continue;
        if (forward ? candidate.top < current.bottom - 0.5 : candidate.bottom > current.top + 0.5) continue;
        const hit = caretLayout.at(state.document, x, (candidate.top + candidate.bottom) / 2);
        if (hit) {
          const hitLayout = model(root, [point(hit.node, hit.offset)]);
          const located = locate(hitLayout, 0, point(hit.node, hit.offset), false);
          const hitBox = located && hitLayout.texts.length === texts.length ? box(located[0], located[1]) : null;
          // Only a hit beyond the current line counts (lines then strictly advance, so the
          // paragraph walk terminates); anything else falls back to the line's first caret.
          if (hitBox && (forward ? hitBox.top >= current.bottom - 0.5 : hitBox.bottom <= current.top + 0.5)) return [located, hitBox];
        }
        return [position, candidate];
      }
      return null;
    };
    const paragraph = position => { const text = texts[position[0]]; return position[0] * 65536 + paragraphStart(text, position[1]); };
    let position = from, current = here;
    for (let guard = 0; guard < 100000; guard++) {
      const found = adjacent(position, current);
      if (!found) return [position === from ? (forward ? [texts.length - 1, texts[texts.length - 1].length] : [0, 0]) : position, x];
      position = found[0]; current = found[1];
      if (granularity === 'line' || paragraph(position) !== paragraph(from)) return [position, x];
    }
    return [position, x];
  }
  function modify(state, extend, forward, granularity) {
    if (none(state)) return;
    const control = controlOf(state);
    let layout, anchorAt, focusAt;
    const anchor = anchorPoint(state), focus = focusPoint(state);
    if (control) { layout = controlLayout(control); anchorAt = [0, anchor.offset]; focusAt = [0, focus.offset]; }
    else {
      const vis = visible(state);
      if (!vis) return;
      layout = model(scopeOf(vis.anchor), [vis.anchor, vis.focus]);
      if (!layout.texts.length) return;
      anchorAt = locate(layout, 0, vis.anchor, state.edgeAnchor); focusAt = locate(layout, 1, vis.focus, state.edgeFocus);
      if (!anchorAt || !focusAt) return;
    }
    const order = (a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] - b[1];
    const isCaret = order(anchorAt, focusAt) === 0;
    let base = anchorAt, extent = focusAt;
    // PrepareToModifySelection: a non-directional range extends from its start or end.
    if (extend && !state.directional && !isCaret) {
      const start = order(anchorAt, focusAt) <= 0 ? anchorAt : focusAt, end = start === anchorAt ? focusAt : anchorAt;
      if (forward) { base = start; extent = end; } else { base = end; extent = start; }
    }
    const baseFirst = order(base, extent) <= 0, start = baseFirst ? base : extent, end = baseFirst ? extent : base;
    let target, xpos = null;
    const boundaryGranularity = granularity === 'sentenceboundary' || granularity === 'lineboundary' || granularity === 'paragraphboundary' || granularity === 'documentboundary';
    const origin = boundaryGranularity || (granularity === 'paragraph' && !extend) ? (forward ? end : start) : extent;
    const vertical = granularity === 'line' || granularity === 'paragraph';
    if (!extend && granularity === 'character' && !isCaret) target = forward ? end : start;
    else if (caretLayout !== null && !control && (vertical || granularity === 'lineboundary')) {
      const result = geometric(layout, scopeOf(anchor), origin, forward, granularity, state);
      if (result === undefined) target = step(layout, origin, forward, granularity);
      else if (vertical) { target = result[0]; xpos = result[1]; }
      else target = result;
    } else target = step(layout, origin, forward, granularity);
    if (!target) return;
    if (extend && !isCaret && (granularity === 'word' || granularity === 'paragraph' || granularity === 'line')) {
      if (baseFirst && order(target, base) < 0) target = base;
      else if (!baseFirst && order(target, base) > 0) target = base;
    }
    let newAnchor, newFocus;
    if (!extend) { newAnchor = newFocus = target; }
    else if (isCaret || !(granularity === 'lineboundary' || granularity === 'paragraphboundary' || granularity === 'documentboundary')) { newAnchor = base; newFocus = target; }
    else if (forward) { newAnchor = baseFirst ? base : target; newFocus = baseFirst ? target : extent; }
    else { newAnchor = baseFirst ? target : base; newFocus = baseFirst ? extent : target; }
    const anchorPosition = at(layout, newAnchor[0], newAnchor[1]), focusPosition = at(layout, newFocus[0], newFocus[1]);
    const options = create(null);
    options.edgeAnchor = !layout.control && layout.edges[newAnchor[0]] === 2;
    options.edgeFocus = !layout.control && layout.edges[newFocus[0]] === 2;
    setSelection(state, anchorPosition, focusPosition, extend, undefined, null, options);
    if (xpos !== null) state.xpos = xpos;
  }

  // ---- Range.prototype.expand (WebKit legacy) ----------------------------------------------
  define(RangeClass.prototype, 'expand', {__proto__: null, value: make(function (unit) {
    const data = rangeArgument(this);
    if (!data) throw new TypeErrorClass('Illegal invocation');
    unit = unit === undefined ? '' : string(unit, "Failed to execute 'expand' on 'Range': ");
    if (unit !== 'word' && unit !== 'sentence' && unit !== 'block' && unit !== 'document') return;
    if (!connected(data.startContainer) || !connected(data.endContainer)) return;
    const layout = model(rootNode(data.startContainer), [point(data.startContainer, data.startOffset), point(data.endContainer, data.endOffset)]);
    if (!layout.texts.length) return;
    let start = located(layout, layout.queries[0]), end = located(layout, layout.queries[1]);
    if (!start || !end) return;
    const texts = layout.texts;
    if (unit === 'word') {
      const startList = boundaries(texts[start[0]], 'word'), endList = boundaries(texts[end[0]], 'word');
      const startText = texts[start[0]];
      if (start[1] < startText.length) { let found = 0; for (let i = 0; i < startList.length; i++) if (startList[i] <= start[1]) found = startList[i]; start = [start[0], found]; }
      const endText = texts[end[0]];
      if (end[1] < endText.length) { const found = following(endList, end[1]); end = [end[0], found === -1 ? endText.length : found]; }
    } else if (unit === 'sentence') {
      const startFound = preceding(boundaries(texts[start[0]], 'sentence'), start[1]);
      start = [start[0], startFound === -1 ? 0 : startFound];
      const endFound = following(boundaries(texts[end[0]], 'sentence'), end[1]);
      end = [end[0], endFound === -1 ? texts[end[0]].length : endFound];
    } else if (unit === 'block') {
      start = [start[0], paragraphStart(texts[start[0]], start[1])];
      end = [end[0], paragraphEnd(texts[end[0]], end[1])];
    } else {
      start = [0, 0]; end = [texts.length - 1, texts[texts.length - 1].length];
    }
    const from = slotPoint(layout, start[0], start[1]), to = slotPoint(layout, end[0], end[1]);
    rangeStart(this, from.node, from.offset);
    rangeEnd(this, to.node, to.offset);
  }, 'expand', 0), writable: true, enumerable: true, configurable: true});

  // ---- Text controls -----------------------------------------------------------------------
  const inputPrototype = g.HTMLInputElement.prototype, textareaPrototype = g.HTMLTextAreaElement.prototype;
  const inputValue = describe(inputPrototype, 'value') || descriptor(inputPrototype, 'value');
  const textareaValue = describe(textareaPrototype, 'value') || descriptor(textareaPrototype, 'value');
  const isInput = node => html(node, 'input'), isTextarea = node => html(node, 'textarea');
  const inputType = node => { const type = getAttribute(node, 'type'); if (type === null) return 'text'; const lowered = ascii(type); return KNOWN_TYPES[lowered] ? lowered : 'text'; };
  const KNOWN_TYPES = create(null);
  for (const type of ['hidden', 'text', 'search', 'tel', 'url', 'email', 'password', 'date', 'month', 'week', 'time', 'datetime-local', 'number', 'range', 'color', 'checkbox', 'radio', 'file', 'submit', 'image', 'reset', 'button']) KNOWN_TYPES[type] = true;
  const SELECTION_TYPES = create(null);
  for (const type of ['text', 'search', 'tel', 'url', 'password']) SELECTION_TYPES[type] = true;
  // Text controls with an inner editor (HTMLInputElement::IsTextField / textarea).
  const textField = node => isTextarea(node) || (isInput(node) && (SELECTION_TYPES[inputType(node)] || inputType(node) === 'email' || inputType(node) === 'number'));
  const supportsSelection = node => isTextarea(node) || (isInput(node) && !!SELECTION_TYPES[inputType(node)]);
  const controlValue = node => apply(isTextarea(node) ? textareaValue.get : inputValue.get, node, []);
  const controls = new WeakMapClass();
  let valueEdited = null;
  // The control's cached selection. Changing an input's type rebuilds its inner editor
  // (selection back to 0); a non-dirty textarea clamps when its children change.
  function cached(control) {
    let selection = mapGet(controls, control);
    const type = isInput(control) ? inputType(control) : 'textarea';
    if (!selection) {
      selection = create(null); selection.start = 0; selection.end = 0; selection.direction = 'none';
      selection.suppress = 0; selection.dirty = false; selection.clamp = -1; selection.type = type;
      mapSet(controls, control, selection);
    }
    if (selection.type !== type) { selection.type = type; selection.start = selection.end = 0; selection.direction = 'none'; }
    if (selection.clamp !== -1) {
      const length = selection.clamp === 0 ? 0 : controlValue(control).length;
      selection.clamp = -1;
      selection.end = min(selection.end, length); selection.start = min(selection.start, selection.end);
    }
    return selection;
  }
  function typeChanged(control) {
    if (!isInput(control)) return;
    const selection = mapGet(controls, control);
    if (!selection) return;
    const type = inputType(control);
    if (selection.type === type) return;
    selection.type = type;
    const changed = selection.start !== 0 || selection.end !== 0 || selection.direction !== 'none';
    selection.start = selection.end = 0; selection.direction = 'none';
    if (changed) scheduleElement(control, 'selectionchange');
  }
  function textareaChanged(control, reset) {
    const selection = mapGet(controls, control);
    if (!selection || selection.dirty) return;
    if (reset) { selection.start = selection.end = 0; selection.clamp = -1; }
    else selection.clamp = 1;
  }
  const focusedIn = control => { const document = ownerDocument(control); return Native('focused', document) === control; };
  const frameState = control => { const document = ownerDocument(control); return document && Native('active', document) ? stateOf(document) : null; };
  // TextControlElement::ComputeSelectionStart/End/Direction from the frame selection.
  function computed(control) {
    const state = frameState(control), result = create(null);
    result.start = 0; result.end = 0; result.direction = 'none';
    if (!state) return result;
    if (controlOf(state) === control) {
      result.start = min(state.anchor, state.focus); result.end = max(state.anchor, state.focus);
    }
    if (!none(state) && state.directional) result.direction = anchorFirst(state) ? 'forward' : 'backward';
    return result;
  }
  function current(control) {
    if (!focusedIn(control)) return cached(control);
    return computed(control);
  }
  // Called whenever the frame selection moves inside `control` (TextControlElement::SelectionChanged).
  function controlSelectionChanged(control, state) {
    const selection = cached(control);
    selection.start = min(state.anchor, state.focus); selection.end = max(state.anchor, state.focus);
    selection.direction = state.directional ? (state.anchor <= state.focus ? 'forward' : 'backward') : 'none';
  }
  // TextControlElement::SetSelectionRange; returns whether the cached selection changed.
  function setControlRange(control, start, end, direction) {
    if (!textField(control)) return false;
    const length = controlValue(control).length;
    end = min(end, length); start = min(start, end);
    const selection = cached(control);
    const changed = selection.start !== start || selection.end !== end || selection.direction !== direction;
    selection.start = start; selection.end = end; selection.direction = direction;
    const state = frameState(control);
    if (!focusedIn(control) || !state) { if (changed) scheduleElement(control, 'selectionchange'); return changed; }
    const backward = direction === 'backward';
    setSelection(state, innerPoint(control, backward ? end : start), innerPoint(control, backward ? start : end), direction !== 'none', undefined, null, {__proto__: null, keepFocus: true});
    return changed;
  }
  function setRangeForBinding(control, start, end, direction) {
    direction = direction === 'forward' || direction === 'backward' ? direction : 'none';
    if (setControlRange(control, start, end, direction)) scheduleElement(control, 'select');
  }
  function restore(control) {
    const selection = cached(control);
    if (setControlRange(control, selection.start, selection.end, selection.direction)) scheduleElement(control, 'select');
  }
  // The focus hook (after focus events): script focus restores a text control's selection.
  function focused(element) {
    if (textField(element) && focusedIn(element) && Native('focusable', element)) restore(element);
  }
  // Value setter hook (browser_env.js): a changed API value moves the selection to its end.
  ns.controlValueChanged = function (control, oldValue, newValue) {
    if (textField(control)) cached(control).dirty = true;
    if (oldValue === newValue || !textField(control) || cached(control).suppress) return;
    opaqueEach(control, state => {
      if (state.value !== oldValue) { opaqueReplace(state, 0, state.value.length, oldValue.length); }
      opaqueReplace(state, 0, oldValue.length, newValue.length); state.value = newValue;
    });
    const length = newValue.length;
    setControlRange(control, length, length, 'none');
  };
  const controlFail = (owner, operation, message) => new Exception("Failed to execute '" + operation + "' on '" + owner + "': " + message, 'InvalidStateError');
  const controlSetFail = (owner, name, message) => new Exception("Failed to set the '" + name + "' property on '" + owner + "': " + message, 'InvalidStateError');
  const typeMessage = control => "The input element's type ('" + inputType(control) + "') does not support selection.";
  for (const [prototype, owner, check] of [[inputPrototype, 'HTMLInputElement', isInput], [textareaPrototype, 'HTMLTextAreaElement', isTextarea]]) {
    const brand = receiver => { if (!isNode(receiver) || !check(receiver)) throw new TypeErrorClass('Illegal invocation'); };
    const accessor = (name, get, set) => define(prototype, name, {__proto__: null, configurable: true, enumerable: true,
      get: make(function () { brand(this); return get(this); }, 'get ' + name, 0),
      set: make(function (value) { brand(this); set(this, value); }, 'set ' + name, 1)});
    const operation = (name, length, fn) => define(prototype, name, {__proto__: null, value: make(function () { brand(this); return apply(fn, this, arguments); }, name, length), writable: true, enumerable: true, configurable: true});
    const nullable = control => owner === 'HTMLInputElement' && !supportsSelection(control);
    const setPrefix = name => "Failed to set the '" + name + "' property on '" + owner + "': ";
    const execPrefix = name => "Failed to execute '" + name + "' on '" + owner + "': ";
    accessor('selectionStart', control => nullable(control) ? null : current(control).start, (control, value) => {
      value = value === null ? 0 : unsigned(value, setPrefix('selectionStart'));
      if (nullable(control)) throw controlSetFail(owner, 'selectionStart', typeMessage(control));
      const selection = current(control);
      setRangeForBinding(control, value, max(value, selection.end), selection.direction);
    });
    accessor('selectionEnd', control => nullable(control) ? null : current(control).end, (control, value) => {
      value = value === null ? 0 : unsigned(value, setPrefix('selectionEnd'));
      if (nullable(control)) throw controlSetFail(owner, 'selectionEnd', typeMessage(control));
      const selection = current(control);
      setRangeForBinding(control, min(value, selection.start), value, selection.direction);
    });
    accessor('selectionDirection', control => nullable(control) ? null : current(control).direction, (control, value) => {
      value = value === null ? null : string(value, setPrefix('selectionDirection'));
      if (nullable(control)) throw controlSetFail(owner, 'selectionDirection', typeMessage(control));
      const selection = current(control);
      setRangeForBinding(control, selection.start, selection.end, value);
    });
    operation('select', 0, function () {
      setRangeForBinding(this, 0, 0xFFFFFFFF, 'none');
      if (Native('focusable', this)) focusElement(this);
      if (focusedIn(this) && Native('focusable', this)) restore(this);
    });
    operation('setSelectionRange', 2, function (start, end, direction) {
      if (arguments.length < 2) throw new TypeErrorClass("Failed to execute 'setSelectionRange' on '" + owner + "': 2 arguments required, but only " + arguments.length + ' present.');
      start = unsigned(start, execPrefix('setSelectionRange')); end = unsigned(end, execPrefix('setSelectionRange')); direction = direction === undefined ? undefined : string(direction, execPrefix('setSelectionRange'));
      if (nullable(this)) throw controlFail(owner, 'setSelectionRange', typeMessage(this));
      setRangeForBinding(this, start, end, direction);
    });
    operation('setRangeText', 1, function (replacement, start, end, selectMode) {
      const count = arguments.length;
      const prefix = "Failed to execute 'setRangeText' on '" + owner + "': ";
      if (count === 0) throw new TypeErrorClass(prefix + '1 argument required, but only 0 present.');
      if (count === 2) throw new TypeErrorClass(prefix + 'Overload resolution failed.');
      replacement = string(replacement, prefix);
      let mode = 'preserve';
      if (count >= 3) {
        start = unsigned(start, prefix); end = unsigned(end, prefix);
        if (selectMode !== undefined) {
          mode = string(selectMode, prefix);
          if (mode !== 'select' && mode !== 'start' && mode !== 'end' && mode !== 'preserve') throw new TypeErrorClass(prefix + "The provided value '" + mode + "' is not a valid enum value of type SelectionMode.");
        }
      }
      if (nullable(this)) throw controlFail(owner, 'setRangeText', typeMessage(this));
      const selection = current(this);
      if (count < 3) { start = selection.start; end = selection.end; }
      if (start > end) throw new Exception(prefix + 'The provided start value (' + start + ') is larger than the provided end value (' + end + ').', 'IndexSizeError');
      let text = controlValue(this);
      let newStart = selection.start, newEnd = selection.end;
      start = min(start, text.length); end = min(end, text.length);
      text = slice(text, 0, start) + replacement + slice(text, end, text.length);
      const state = cached(this), before = controlValue(this);
      state.suppress++;
      try { apply(isTextarea(this) ? textareaValue.set : inputValue.set, this, [text]); } finally { state.suppress--; }
      valueEdited(this, before, start, end, replacement.length);
      // Replacing a focused control's inner text moves its frame selection.
      const frame = frameState(this);
      if (frame && controlOf(frame) === this && focusedIn(this) && controlValue(this) !== before) scheduleChange(frame);
      if (mode === 'select') { newStart = start; newEnd = start + replacement.length; }
      else if (mode === 'start') newStart = newEnd = start;
      else if (mode === 'end') newStart = newEnd = start + replacement.length;
      else {
        const delta = replacement.length - (end - start);
        if (newStart > end) newStart += delta; else if (newStart > start) newStart = start;
        if (newEnd > end) newEnd += delta; else if (newEnd > start) newEnd = start + replacement.length;
      }
      setRangeForBinding(this, newStart, newEnd, 'none');
    });
  }
  // ---- OpaqueRange (HTMLInputElement/HTMLTextAreaElement.createValueRange) ------------------
  // Offsets into a text control's value, live under value edits like a Range over text:
  // setRangeText follows the replace-data rules, any other value change replaces the whole
  // value, and the range disconnects (0, 0) when disconnect() runs or its control leaves
  // the document. Geometry comes from the engine's layout hook (none: no layout boxes).
  const AbstractRangeClass = g.AbstractRange, WeakRefClass = g.WeakRef, deref = WeakRefClass.prototype.deref;
  const OpaqueRange = make(illegal('OpaqueRange'), 'OpaqueRange', 0, true, true, false);
  const OpaquePrototype = create(AbstractRangeClass.prototype);
  setPrototype(OpaqueRange, AbstractRangeClass);
  const opaqueRanges = new WeakMapClass();
  const opaqueState = receiver => {
    const state = (receiver !== null && typeof receiver === 'object') ? staticRecord(receiver) : undefined;
    if (!state || state.opaque !== true) throw new TypeErrorClass('Illegal invocation');
    return state;
  };
  // Any value change not reported by setRangeText replaces the whole value.
  function opaqueRefresh(state) {
    if (state.control === null) return state;
    if (!connected(state.control) && state.wasConnected) { opaqueDisconnect(state); return state; }
    const value = controlValue(state.control);
    if (value !== state.value) { opaqueReplace(state, 0, state.value.length, value.length); state.value = value; }
    return state;
  }
  function opaqueDisconnect(state) { state.control = null; state.start = state.end = 0; }
  function opaqueReplace(state, offset, count, inserted) {
    const moved = position => position > offset && position <= offset + count ? offset : position > offset + count ? position - count + inserted : position;
    state.start = moved(state.start); state.end = moved(state.end);
  }
  function opaqueEach(control, fn) {
    const list = mapGet(opaqueRanges, control);
    if (!list) return;
    let kept = 0;
    for (let i = 0; i < list.length; i++) {
      const range = apply(deref, list[i], []);
      if (!range) continue;
      list[kept++] = list[i];
      const state = staticRecord(range);
      if (state.control === control) fn(state);
    }
    list.length = kept;
  }
  valueEdited = (control, before, start, end, inserted) => {
    const value = controlValue(control);
    opaqueEach(control, state => {
      if (state.value !== before) { opaqueReplace(state, 0, state.value.length, before.length); state.value = before; }
      opaqueReplace(state, start, end - start, inserted); state.value = value;
    });
  };
  const opaqueMethods = [
    ['disconnect', function () { opaqueDisconnect(opaqueState(this)); }],
    ['getBoundingClientRect', function () { return opaqueGeometry(opaqueRefresh(opaqueState(this)), false); }],
    ['getClientRects', function () { return opaqueGeometry(opaqueRefresh(opaqueState(this)), true); }],
  ];
  for (const [name, fn] of opaqueMethods) define(OpaquePrototype, name, {__proto__: null, value: make(fn, name, 0), writable: true, enumerable: true, configurable: true});
  define(OpaquePrototype, 'constructor', {__proto__: null, value: OpaqueRange, writable: true, enumerable: false, configurable: true});
  define(OpaquePrototype, Symbol.toStringTag, {__proto__: null, value: 'OpaqueRange', writable: false, enumerable: false, configurable: true});
  define(OpaqueRange, 'prototype', {__proto__: null, value: OpaquePrototype, writable: false, enumerable: false, configurable: false});
  define(g, 'OpaqueRange', {__proto__: null, value: OpaqueRange, writable: true, enumerable: false, configurable: true});
  const emptyRects = method(RangeClass.prototype, 'getClientRects'), emptyBounds = method(RangeClass.prototype, 'getBoundingClientRect');
  // An embedder with layout registers value-range geometry once, before page script.
  let opaqueLayout = null;
  define(ns, 'registerOpaqueRangeGeometry', {__proto__: null, configurable: true, value: fn => { if (typeof fn === 'function' && opaqueLayout === null) opaqueLayout = fn; delete ns.registerOpaqueRangeGeometry; }});
  function opaqueGeometry(state, list) {
    const layout = opaqueLayout;
    if (state.control !== null && connected(state.control) && layout !== null) return layout(state.control, state.start, state.end, list);
    const empty = new RangeClass();
    return list ? emptyRects(empty) : emptyBounds(empty);
  }
  for (const [prototype, owner, check] of [[inputPrototype, 'HTMLInputElement', isInput], [textareaPrototype, 'HTMLTextAreaElement', isTextarea]]) {
    define(prototype, 'createValueRange', {__proto__: null, value: make(function (start, end) {
      if (!isNode(this) || !check(this)) throw new TypeErrorClass('Illegal invocation');
      const prefix = "Failed to execute 'createValueRange' on '" + owner + "': ";
      if (arguments.length < 2) throw new TypeErrorClass(prefix + '2 arguments required, but only ' + arguments.length + ' present.');
      start = unsigned(start, prefix); end = unsigned(end, prefix);
      if (owner === 'HTMLInputElement' && !SELECTION_TYPES[inputType(this)]) throw new Exception(prefix + '<input> element must be of a text field type: text, search, url, tel, or password.', 'NotSupportedError');
      const value = controlValue(this);
      if (start > value.length || end > value.length) throw new Exception(prefix + 'Start or end offset exceeds value length.', 'IndexSizeError');
      if (start > end) end = start;
      const state = create(null);
      state.opaque = true; state.startContainer = null; state.endContainer = null;
      state.start = start; state.end = end; state.control = this; state.value = value;
      state.wasConnected = connected(this);
      // AbstractRange's accessors read these through the private record.
      define(state, 'startOffset', {__proto__: null, get: () => opaqueRefresh(state).start});
      define(state, 'endOffset', {__proto__: null, get: () => opaqueRefresh(state).end});
      const range = staticRecord(undefined, state, OpaquePrototype);
      let list = mapGet(opaqueRanges, this);
      if (!list) { list = []; mapSet(opaqueRanges, this, list); }
      push(list, new WeakRefClass(range));
      if (state.wasConnected) Native('watch', this);
      return range;
    }, 'createValueRange', 2), writable: true, enumerable: true, configurable: true});
  }
  // Trusted typing (an embedder's input driver): replace the control's selection with
  // `text` like Blink's InsertText editing command — the caret follows the insertion,
  // value ranges follow the edit and selectionchange fires, but never a select event.
  define(ns, 'replaceControlSelection', {__proto__: null, configurable: true, value: (control, text) => {
    if (!textField(control)) return false;
    const selection = current(control), before = controlValue(control);
    const start = min(selection.start, before.length), end = min(max(selection.end, start), before.length);
    const record = cached(control);
    record.suppress++;
    try { apply(isTextarea(control) ? textareaValue.set : inputValue.set, control, [slice(before, 0, start) + text + slice(before, end, before.length)]); } finally { record.suppress--; }
    valueEdited(control, before, start, end, text.length);
    const caret = start + text.length, frame = frameState(control);
    setControlRange(control, caret, caret, 'none');
    if (frame && controlOf(frame) === control && focusedIn(control)) scheduleChange(frame);
    return true;
  }});
  // A watched control is leaving the document: its live value ranges disconnect.
  function controlRemoved(control) { opaqueEach(control, opaqueDisconnect); }

  return [focused, sync, textareaChanged, typeChanged, controlRemoved];
});

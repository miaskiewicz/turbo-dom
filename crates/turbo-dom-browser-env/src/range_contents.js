(function (record, make, convertTrustedType, staticRecord, parseContextualFragment) {
  "use strict";
  let Exception = DOMException;
  const LocalRange = Range,
    TypeErrorClass = TypeError,
    StringClass = String,
    define = Object.defineProperty,
    apply = Reflect.apply,
    getDescriptor = Object.getOwnPropertyDescriptor,
    getPrototype = Object.getPrototypeOf;
  const stringSlice = String.prototype.slice;
  const substring = (value, from, to) => apply(stringSlice, value, [from, to]);
  function descriptor(proto, name) {
    while (proto) {
      const found = getDescriptor(proto, name);
      if (found) return found;
      proto = getPrototype(proto);
    }
    throw new TypeErrorClass("Missing native DOM member: " + name);
  }
  const read = (proto, name) => {
    const getter = descriptor(proto, name).get;
    return node => apply(getter, node, []);
  };
  const type = read(Node.prototype, "nodeType"),
    parent = read(Node.prototype, "parentNode"),
    first = read(Node.prototype, "firstChild"),
    next = read(Node.prototype, "nextSibling"),
    owner = read(Node.prototype, "ownerDocument"),
    data = read(Text.prototype, "data");
  const setData = descriptor(Text.prototype, "data").set,
    clone = descriptor(Node.prototype, "cloneNode").value,
    append = descriptor(Node.prototype, "appendChild").value,
    remove = descriptor(Node.prototype, "removeChild").value,
    insert = descriptor(Node.prototype, "insertBefore").value,
    split = descriptor(Text.prototype, "splitText").value,
    deleteData = descriptor(Text.prototype, "deleteData").value,
    fragment = descriptor(Document.prototype, "createDocumentFragment").value;
  const fail = name => { throw new Exception("Invalid Range operation", name); };
  // Selection hooks: Blink pushes boundary changes of the document's cached selection range
  // back into the frame selection (Range::UpdateSelectionIfAddedToSelection).
  // The hook is installed privately by the selection module through the returned updater.
  let selectionSync = null;
  const sync = state => { if (selectionSync !== null) selectionSync(state); };
  const required = (args, count) => {
    if (args.length < count) throw new TypeErrorClass("Insufficient Range arguments");
  };
  const push = (list, value) => define(list, list.length, {
    __proto__: null, value, writable: true, enumerable: true, configurable: true,
  });
  const character = kind => kind === 3 || kind === 4 || kind === 7 || kind === 8;
  function children(node) {
    const rows = [];
    for (let child = first(node); child; child = next(child)) push(rows, child);
    return rows;
  }
  function length(node) {
    return character(type(node)) ? data(node).length : children(node).length;
  }
  const setPrototype = Object.setPrototypeOf;
  const AbstractRange = make(function () {
    throw new TypeErrorClass("Illegal constructor");
  }, "AbstractRange", 0, true);
  const NodeRange = make(function () {
    throw new TypeErrorClass("Illegal constructor");
  }, "NodeRange", 0, true);
  const abstractState = receiver => staticRecord(receiver) || record(receiver);
  for (const name of ["startOffset", "endOffset", "collapsed"]) {
    define(AbstractRange.prototype, name, {__proto__: null,
      get: make(function () {
        const state = abstractState(this);
        return name === "collapsed" ? state.startContainer === state.endContainer && state.startOffset === state.endOffset : state[name];
      }, "get " + name, 0), configurable: true, enumerable: true,
    });
  }
  for (const name of ["startContainer", "endContainer"]) {
    define(NodeRange.prototype, name, {__proto__: null,
      get: make(function () { return abstractState(this)[name]; }, "get " + name, 0),
      configurable: true, enumerable: true,
    });
  }
  const StaticRange = make(function (init) {
    required(arguments, 1);
    if (init !== null && init !== undefined && typeof init !== "object" && typeof init !== "function")
      throw new TypeErrorClass("StaticRangeInit must be a dictionary");
    const state = {__proto__: null};
    // Web IDL reads and converts dictionary members in lexicographic order.
    const names = ["endContainer", "endOffset", "startContainer", "startOffset"];
    for (let index = 0; index < names.length; index++) {
      const name = names[index];
      const value = init == null ? undefined : init[name];
      if (value === undefined) throw new TypeErrorClass("Missing required StaticRangeInit member: " + name);
      if (name === "endContainer" || name === "startContainer") {
        type(value); state[name] = value;
      } else state[name] = (+value) >>> 0;
    }
    const startType = type(state.startContainer), endType = type(state.endContainer);
    if (startType === 2 || startType === 10 || endType === 2 || endType === 10)
      fail("InvalidNodeTypeError");
    return staticRecord(undefined, state, getPrototype(this));
  }, "StaticRange", 1, true);
  setPrototype(NodeRange.prototype, AbstractRange.prototype);
  setPrototype(NodeRange, AbstractRange);
  for (const constructor of [LocalRange, StaticRange]) {
    setPrototype(constructor.prototype, NodeRange.prototype);
    setPrototype(constructor, NodeRange);
  }
  for (const name of ["startContainer", "endContainer", "startOffset", "endOffset", "collapsed"])
    delete LocalRange.prototype[name];
  for (const constructor of [AbstractRange, NodeRange, LocalRange, StaticRange]) {
    define(constructor, "prototype", {writable: false});
    define(constructor.prototype, Symbol.toStringTag, {value: constructor.name, configurable: true});
    define(globalThis, constructor.name, {value: constructor, configurable: true, writable: true, enumerable: false});
  }
  function root(node) {
    let ancestor;
    while ((ancestor = parent(node))) node = ancestor;
    return node;
  }
  function ancestor(node, descendant) {
    while (descendant) {
      if (node === descendant) return true;
      descendant = parent(descendant);
    }
    return false;
  }
  function index(node) {
    const container = parent(node);
    let at = 0;
    for (let child = first(container); child && child !== node; child = next(child)) at++;
    return at;
  }
  function common(left, right) {
    while (!ancestor(left, right)) left = parent(left);
    return left;
  }
  function compare(left, leftOffset, right, rightOffset) {
    if (left === right) return leftOffset < rightOffset ? -1 : leftOffset > rightOffset ? 1 : 0;
    const shared = common(left, right);
    if (shared === left) {
      let child = right;
      while (parent(child) !== shared) child = parent(child);
      return leftOffset <= index(child) ? -1 : 1;
    }
    if (shared === right) return -compare(right, rightOffset, left, leftOffset);
    let leftChild = left, rightChild = right;
    while (parent(leftChild) !== shared) leftChild = parent(leftChild);
    while (parent(rightChild) !== shared) rightChild = parent(rightChild);
    return index(leftChild) < index(rightChild) ? -1 : 1;
  }
  function validate(node, offset) {
    if (type(node) === 10) fail("InvalidNodeTypeError");
    if (offset > length(node)) fail("IndexSizeError");
  }
  function boundary(state, node, offset, start) {
    validate(node, offset);
    if (start) {
      if (root(node) !== root(state.endContainer) ||
          compare(node, offset, state.endContainer, state.endOffset) > 0) {
        state.endContainer = node; state.endOffset = offset;
      }
      state.startContainer = node; state.startOffset = offset;
    } else {
      if (root(node) !== root(state.startContainer) ||
          compare(node, offset, state.startContainer, state.startOffset) < 0) {
        state.startContainer = node; state.startOffset = offset;
      }
      state.endContainer = node; state.endOffset = offset;
    }
    state.document = type(node) === 9 ? node : owner(node);
  }
  const collapsed = state => state.startContainer === state.endContainer &&
    state.startOffset === state.endOffset;
  function collapseAt(state, node, offset) {
    state.startContainer = state.endContainer = node;
    state.startOffset = state.endOffset = offset;
  }
  function collapsePoint(state) {
    let node = state.startContainer;
    if (ancestor(node, state.endContainer)) return {node, offset: state.startOffset};
    while (!ancestor(parent(node), state.endContainer)) node = parent(node);
    return {node: parent(node), offset: index(node) + 1};
  }
  // Decide containment before moving any nodes; mutation changes sibling indexes.
  function plan(state, node, container, at) {
    const kind = type(node), starts = ancestor(node, state.startContainer),
      ends = ancestor(node, state.endContainer);
    if (!starts && !ends &&
        compare(container, at, state.startContainer, state.startOffset) >= 0 &&
        compare(container, at + 1, state.endContainer, state.endOffset) <= 0)
      return {__proto__: null, node, kind, full: true};
    if (!starts && !ends) return null;
    if (character(kind)) {
      const from = node === state.startContainer ? state.startOffset : 0,
        to = node === state.endContainer ? state.endOffset : length(node);
      return {__proto__: null, node, kind, from, count: to - from};
    }
    const rows = [], nodes = children(node);
    for (let i = 0; i < nodes.length; i++) {
      const child = plan(state, nodes[i], node, i);
      if (child) push(rows, child);
    }
    return {__proto__: null, node, kind, rows};
  }
  function plans(state) {
    const shared = common(state.startContainer, state.endContainer);
    if (character(type(shared))) return [plan(state, shared, null, 0)];
    const rows = [], nodes = children(shared);
    for (let i = 0; i < nodes.length; i++) {
      const entry = plan(state, nodes[i], shared, i);
      if (entry) push(rows, entry);
    }
    return rows;
  }
  function checkDoctypes(rows) {
    for (let i = 0; i < rows.length; i++) {
      if (rows[i].kind === 10) fail("HierarchyRequestError");
      if (rows[i].rows) checkDoctypes(rows[i].rows);
    }
  }
  function execute(rows, target, mode) {
    for (let i = 0; i < rows.length; i++) {
      const entry = rows[i];
      if (entry.full) {
        if (mode === "delete") apply(remove, parent(entry.node), [entry.node]);
        else apply(append, target, [mode === "clone" ? apply(clone, entry.node, [true]) : entry.node]);
      } else if (character(entry.kind)) {
        if (mode !== "delete") {
          const copy = apply(clone, entry.node, [false]);
          apply(setData, copy, [substring(data(entry.node), entry.from, entry.from + entry.count)]);
          apply(append, target, [copy]);
        }
        if (mode !== "clone") apply(deleteData, entry.node, [entry.from, entry.count]);
      } else if (mode === "delete") execute(entry.rows, null, mode);
      else {
        const shell = apply(clone, entry.node, [false]);
        execute(entry.rows, shell, mode);
        apply(append, target, [shell]);
      }
    }
  }
  // Blink's ProcessContents collapses an extract/delete range before moving any node, through
  // setStart (which updates a selection holding this range) and a direct end assignment.
  function collapseFirst(state) {
    if (state.startContainer === state.endContainer) return;
    const shared = common(state.startContainer, state.endContainer);
    const highest = node => { if (node === shared) return null; while (parent(node) !== shared) node = parent(node); return node; };
    const start = highest(state.startContainer), end = highest(state.endContainer);
    if (start) { state.startContainer = shared; state.startOffset = index(start) + 1; sync(state); }
    else if (end) { state.startContainer = shared; state.startOffset = index(end); sync(state); }
    state.endContainer = state.startContainer; state.endOffset = state.startOffset;
  }
  function contents(state, mode) {
    const document = type(state.startContainer) === 9 ? state.startContainer : owner(state.startContainer),
      target = mode === "delete" ? null : apply(fragment, document, []);
    if (collapsed(state)) return target;
    const rows = plans(state), point = collapsePoint(state);
    if (mode !== "delete") checkDoctypes(rows);
    if (mode !== "clone") collapseFirst(state);
    execute(rows, target, mode);
    if (mode !== "clone") collapseAt(state, point.node, point.offset);
    return target;
  }
  function insertNode(state, node) {
    const kind = type(state.startContainer);
    type(node);
    if (kind === 7 || kind === 8 || node === state.startContainer ||
        (kind === 3 && !parent(state.startContainer))) fail("HierarchyRequestError");
    const wasCollapsed = collapsed(state), count = type(node) === 11 ? length(node) : 1;
    let container, reference;
    if (kind === 3) {
      container = parent(state.startContainer);
      ensureInsertion(container, node, state.startContainer);
      reference = apply(split, state.startContainer, [state.startOffset]);
    } else {
      container = state.startContainer;
      reference = children(container)[state.startOffset] || null;
      ensureInsertion(container, node, reference);
    }
    if (node === reference) reference = next(reference);
    const oldParent = parent(node);
    if (oldParent) apply(remove, oldParent, [node]);
    const offset = reference ? index(reference) : length(container);
    apply(insert, container, [node, reference]);
    if (wasCollapsed) { state.endContainer = container; state.endOffset = offset + count; }
  }
  function ensureInsertion(container, node, reference) {
    const containerKind = type(container), kind = type(node);
    if ((containerKind !== 1 && containerKind !== 9 && containerKind !== 11) ||
        ancestor(node, container) ||
        (kind !== 1 && kind !== 3 && kind !== 4 && kind !== 7 && kind !== 8 && kind !== 10 && kind !== 11) ||
        (kind === 10 && containerKind !== 9)) fail("HierarchyRequestError");
    if (containerKind !== 9) return;
    const sequence = [], existing = children(container), incoming = kind === 11 ? children(node) : [node];
    let included = false;
    for (let i = 0; i < existing.length; i++) {
      if (existing[i] === reference) {
        for (let j = 0; j < incoming.length; j++) push(sequence, incoming[j]);
        included = true;
      }
      if (existing[i] !== node) push(sequence, existing[i]);
    }
    if (!included) for (let i = 0; i < incoming.length; i++) push(sequence, incoming[i]);
    let elements = 0, doctypes = 0;
    for (let i = 0; i < sequence.length; i++) {
      const childKind = type(sequence[i]);
      if (childKind === 3 || childKind === 4) fail("HierarchyRequestError");
      if (childKind === 1 && ++elements > 1) fail("HierarchyRequestError");
      if (childKind === 10 && (++doctypes > 1 || elements)) fail("HierarchyRequestError");
    }
  }
  function method(name, length, implementation) {
    define(LocalRange.prototype, name, {__proto__: null,
      value: make(function () { return apply(implementation, record(this), arguments); }, name, length),
      writable: true, configurable: true, enumerable: true,
    });
  }
  define(LocalRange.prototype, "commonAncestorContainer", {__proto__: null,
    get: make(function () {
      const state = record(this); return common(state.startContainer, state.endContainer);
    }, "get commonAncestorContainer", 0), enumerable: true, configurable: true,
  });
  method("setStart", 2, function (node, offset) { required(arguments, 2); type(node); boundary(this, node, (+offset) >>> 0, true); sync(this); });
  method("setEnd", 2, function (node, offset) { required(arguments, 2); type(node); boundary(this, node, (+offset) >>> 0, false); sync(this); });
  function around(state, node, after, start) {
    type(node); const container = parent(node);
    if (!container) fail("InvalidNodeTypeError");
    boundary(state, container, index(node) + (after ? 1 : 0), start);
    sync(state);
  }
  method("setStartBefore", 1, function (node) { required(arguments, 1); around(this, node, false, true); });
  method("setStartAfter", 1, function (node) { required(arguments, 1); around(this, node, true, true); });
  method("setEndBefore", 1, function (node) { required(arguments, 1); around(this, node, false, false); });
  method("setEndAfter", 1, function (node) { required(arguments, 1); around(this, node, true, false); });
  method("collapse", 0, function (start = false) {
    collapseAt(this, start ? this.startContainer : this.endContainer, start ? this.startOffset : this.endOffset);
    sync(this);
  });
  function select(state, node) {
    type(node); const container = parent(node);
    if (!container) fail("InvalidNodeTypeError");
    state.startContainer = state.endContainer = container;
    state.startOffset = index(node); state.endOffset = state.startOffset + 1;
  }
  method("selectNode", 1, function (node) { required(arguments, 1); select(this, node); sync(this); });
  method("selectNodeContents", 1, function (node) {
    required(arguments, 1); type(node); validate(node, 0);
    this.startContainer = this.endContainer = node; this.startOffset = 0; this.endOffset = length(node);
    sync(this);
  });
  method("compareBoundaryPoints", 2, function (how, other) {
    required(arguments, 2); how = (+how) & 65535; other = record(other);
    if (how > 3) fail("NotSupportedError");
    if (root(this.startContainer) !== root(other.startContainer)) fail("WrongDocumentError");
    const leftEnd = how === 1 || how === 2, rightEnd = how === 2 || how === 3;
    return compare(leftEnd ? this.endContainer : this.startContainer, leftEnd ? this.endOffset : this.startOffset,
      rightEnd ? other.endContainer : other.startContainer, rightEnd ? other.endOffset : other.startOffset);
  });
  function point(state, node, offset, boolean) {
    if (root(node) !== root(state.startContainer)) { if (boolean) return false; fail("WrongDocumentError"); }
    validate(node, offset);
    const result = compare(node, offset, state.startContainer, state.startOffset) < 0 ? -1 :
      compare(node, offset, state.endContainer, state.endOffset) > 0 ? 1 : 0;
    return boolean ? result === 0 : result;
  }
  method("comparePoint", 2, function (node, offset) { required(arguments, 2); type(node); return point(this, node, (+offset) >>> 0, false); });
  method("isPointInRange", 2, function (node, offset) { required(arguments, 2); type(node); return point(this, node, (+offset) >>> 0, true); });
  method("intersectsNode", 1, function (node) {
    required(arguments, 1); type(node);
    if (root(node) !== root(this.startContainer)) return false;
    const container = parent(node);
    return !container || (compare(container, index(node), this.endContainer, this.endOffset) < 0 &&
      compare(container, index(node) + 1, this.startContainer, this.startOffset) > 0);
  });
  method("createContextualFragment", 1, function (html) {
    required(arguments, 1);
    const start = this.startContainer;
    html = convertTrustedType(start, html, "TrustedHTML", "Range createContextualFragment", "Failed to execute 'createContextualFragment' on 'Range': ", false);
    return parseContextualFragment(this.startContainer, html);
  });
  method("cloneContents", 0, function () { return contents(this, "clone"); });
  method("extractContents", 0, function () { return contents(this, "extract"); });
  method("deleteContents", 0, function () { contents(this, "delete"); });
  method("insertNode", 1, function (node) { required(arguments, 1); insertNode(this, node); });
  method("surroundContents", 1, function (node) {
    required(arguments, 1); const kind = type(node);
    const shared = common(this.startContainer, this.endContainer);
    for (let boundary = this.startContainer; boundary !== shared; boundary = parent(boundary))
      if (type(boundary) !== 3 && type(boundary) !== 4) fail("InvalidStateError");
    for (let boundary = this.endContainer; boundary !== shared; boundary = parent(boundary))
      if (type(boundary) !== 3 && type(boundary) !== 4) fail("InvalidStateError");
    if (kind === 9 || kind === 10 || kind === 11) fail("InvalidNodeTypeError");
    const selected = contents(this, "extract");
    let child;
    while ((child = first(node))) apply(remove, node, [child]);
    insertNode(this, node); apply(append, node, [selected]); select(this, node); sync(this);
  });
  function text(rows) {
    let result = "";
    for (let i = 0; i < rows.length; i++) {
      const entry = rows[i];
      if (entry.full) {
        const nodes = children(entry.node);
        if (entry.kind === 3 || entry.kind === 4) result += data(entry.node);
        else for (let j = 0; j < nodes.length; j++) result += text([{node: nodes[j], kind: type(nodes[j]), full: true}]);
      } else if (entry.kind === 3 || entry.kind === 4) result += substring(data(entry.node), entry.from, entry.from + entry.count);
      else if (entry.rows) result += text(entry.rows);
    }
    return result;
  }
  method("toString", 0, function () { return collapsed(this) ? "" : text(plans(this)); });
  define(Document.prototype, "createRange", {__proto__: null,
    value: make(function () {
      if (type(this) !== 9) throw new TypeErrorClass("Illegal invocation");
      const result = new LocalRange(), state = record(result);
      collapseAt(state, this, 0); state.document = this;
      return result;
    }, "createRange", 0), writable: true, configurable: true, enumerable: true,
  });
  if (Object.hasOwn(document, "createRange")) delete document.createRange;
  for (const [name, value] of [["START_TO_START", 0], ["START_TO_END", 1], ["END_TO_END", 2], ["END_TO_START", 3]]) {
    for (const target of [LocalRange, LocalRange.prototype])
      if (!Object.hasOwn(target, name)) define(target, name, {value, enumerable: true, configurable: true});
  }
  return function (constructor, hook) {
    if (constructor !== undefined) Exception = constructor;
    if (typeof hook === "function") selectionSync = hook;
  };
});

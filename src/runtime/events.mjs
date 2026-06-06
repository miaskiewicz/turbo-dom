// Event system — implemented fully (it's small and load-bearing; laziness saves
// nothing here). Capture + target + bubble phases, composedPath, stop/prevent.

const PHASE_NONE = 0;
const PHASE_CAPTURING = 1;
const PHASE_AT_TARGET = 2;
const PHASE_BUBBLING = 3;

export class Event {
  static NONE = PHASE_NONE;
  static CAPTURING_PHASE = PHASE_CAPTURING;
  static AT_TARGET = PHASE_AT_TARGET;
  static BUBBLING_PHASE = PHASE_BUBBLING;

  constructor(type, init = {}) {
    this.type = type;
    this.bubbles = !!init.bubbles;
    this.cancelable = !!init.cancelable;
    this.composed = !!init.composed;
    this.target = null;
    this.currentTarget = null;
    this.eventPhase = PHASE_NONE;
    this.defaultPrevented = false;
    this.isTrusted = false;
    this.timeStamp = 0;
    this._stopPropagation = false;
    this._stopImmediate = false;
    this._path = null;            // set by dispatchEvent; composedPath() reads [] until then
    this._passiveListener = false;
  }

  get NONE() { return PHASE_NONE; }
  get CAPTURING_PHASE() { return PHASE_CAPTURING; }
  get AT_TARGET() { return PHASE_AT_TARGET; }
  get BUBBLING_PHASE() { return PHASE_BUBBLING; }

  stopPropagation() { this._stopPropagation = true; }
  stopImmediatePropagation() { this._stopPropagation = true; this._stopImmediate = true; }
  preventDefault() { if (this.cancelable && !this._passiveListener) this.defaultPrevented = true; }
  get returnValue() { return !this.defaultPrevented; }
  set returnValue(v) { if (v === false) this.preventDefault(); }

  composedPath() { return this._path ? this._path.slice() : []; }

  // legacy init — react-dom's dev rethrow path uses createEvent('Event')+initEvent
  initEvent(type, bubbles = false, cancelable = false) {
    this.type = String(type);
    this.bubbles = !!bubbles;
    this.cancelable = !!cancelable;
  }
}

export class CustomEvent extends Event {
  constructor(type, init = {}) {
    super(type, init);
    this.detail = init.detail ?? null;
  }
  initCustomEvent(type, bubbles = false, cancelable = false, detail = null) {
    this.initEvent(type, bubbles, cancelable);
    this.detail = detail;
  }
}

// Typed events. Real libraries (RTL/user-event) construct these by name; we copy
// the init dict onto the instance so common props (button, key, relatedTarget…) read back.
const TYPED_DEFAULTS = {
  UIEvent: { detail: 0, view: null },
  MouseEvent: { button: 0, buttons: 0, clientX: 0, clientY: 0, screenX: 0, screenY: 0, relatedTarget: null, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false },
  PointerEvent: { pointerId: 0, pointerType: '', button: 0, buttons: 0, clientX: 0, clientY: 0 },
  KeyboardEvent: { key: '', code: '', keyCode: 0, which: 0, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, repeat: false },
  InputEvent: { data: null, inputType: '', isComposing: false },
  FocusEvent: { relatedTarget: null },
  CompositionEvent: { data: '' },
  WheelEvent: { deltaX: 0, deltaY: 0, deltaZ: 0, deltaMode: 0 },
  TouchEvent: { touches: [], targetTouches: [], changedTouches: [] },
  DragEvent: { dataTransfer: null },
  ProgressEvent: { lengthComputable: false, loaded: 0, total: 0 },
};

function makeTyped(name) {
  const defaults = TYPED_DEFAULTS[name];
  const cls = class extends Event {
    constructor(type, init = {}) {
      super(type, init);
      Object.assign(this, defaults, init);
    }
  };
  // legacy initMouseEvent/initKeyboardEvent/initUIEvent(type, bubbles, cancelable, ...rest)
  cls.prototype['init' + name] = function (type, bubbles = false, cancelable = false, ...rest) {
    this.initEvent(type, bubbles, cancelable);
    // view is the common 4th arg for UI/Mouse/Keyboard events; ignore the rest's exact slots
    if (rest.length) this.view = rest[0];
  };
  return cls;
}

export const UIEvent = makeTyped('UIEvent');
export const MouseEvent = makeTyped('MouseEvent');
export const PointerEvent = makeTyped('PointerEvent');
export const KeyboardEvent = makeTyped('KeyboardEvent');
export const InputEvent = makeTyped('InputEvent');
export const FocusEvent = makeTyped('FocusEvent');
export const CompositionEvent = makeTyped('CompositionEvent');
export const WheelEvent = makeTyped('WheelEvent');
export const TouchEvent = makeTyped('TouchEvent');
export const DragEvent = makeTyped('DragEvent');
export const ProgressEvent = makeTyped('ProgressEvent');

// ---- shadow DOM event support ----
// Duck-typed (ShadowRoot sets `__isShadowRoot`/`host`) to avoid a dom.mjs import
// cycle. NONE of this runs unless the document has a shadow root attached — the
// hot dispatch path branches on `doc.__hasShadow` and skips it entirely.
function treeRootOf(node) { let n = node; while (n.parentNode) n = n.parentNode; return n; }
// Is `b` a shadow-including inclusive descendant of tree-root `root`? Walks up
// through parentNode, hopping shadow-root→host at each boundary.
function shadowInclusiveDescendant(b, root) {
  let n = b;
  while (n) {
    if (n === root) return true;
    n = n.parentNode || (n.__isShadowRoot ? n.host : null); // hop boundary, else stop
  }
  return false;
}
// WHATWG retarget(A, B): the version of A visible to a listener whose
// currentTarget is B. If A lives in a shadow tree that B is outside of, A is
// reported as that tree's host (and recursively, for nested shadows). The loop
// always terminates — each hop climbs one shadow boundary toward the document.
function retarget(a, b) {
  for (;;) {
    const r = treeRootOf(a);
    if (!r.__isShadowRoot || shadowInclusiveDescendant(b, r)) return a;
    a = r.host;
  }
}

export class EventTarget {
  constructor() {
    // type -> array of { callback, capture, once, passive }. Lazily created on
    // first addEventListener — most inflated nodes never get a listener, so this
    // skips a Map allocation per node (inflation is a top hot path).
    this.__listeners = null;
  }

  addEventListener(type, callback, options) {
    if (callback == null) return;
    // Inlined option parsing — avoids a throwaway normalizeOptions object AND a
    // per-call .some() closure on this hot path (React attaches many listeners
    // at mount). Boolean options → capture; object → capture/once/passive.
    let capture = false, once = false, passive = false;
    if (typeof options === 'boolean') capture = options;
    else if (options) { capture = !!options.capture; once = !!options.once; passive = !!options.passive; }
    if (!this.__listeners) this.__listeners = new Map();
    let list = this.__listeners.get(type);
    if (!list) { list = []; this.__listeners.set(type, list); }
    // dedupe on (callback, capture) per spec
    for (let i = 0; i < list.length; i++) if (list[i].callback === callback && list[i].capture === capture) return;
    list.push({ callback, capture, once, passive });
  }

  removeEventListener(type, callback, options) {
    if (!this.__listeners) return;
    const capture = typeof options === 'boolean' ? options : !!(options && options.capture);
    const list = this.__listeners.get(type);
    if (!list) return;
    for (let i = 0; i < list.length; i++) {
      if (list[i].callback === callback && list[i].capture === capture) { list.splice(i, 1); return; }
    }
  }

  // Build the event path: target up through ancestors to the root.
  __eventPath() {
    const path = [];
    let node = this;
    while (node) {
      path.push(node);
      node = node.parentNode || node.__owner || null; // element->parent, then document->window
    }
    return path;
  }

  dispatchEvent(event) {
    if (!(event instanceof Event)) throw new TypeError('dispatchEvent requires an Event');
    const target = this;
    event.target = target;

    // Single ancestor walk: build the path AND note whether any node on it has a
    // listener for this type. React fires thousands of events with zero matching
    // listeners on the path — those skip the capture/target/bubble invoke loops.
    const type = event.type;
    const path = [];
    let hasListener = false;
    // Pay-for-what-you-use: the shadow-aware walk + per-invoke retargeting only
    // runs when a shadow root exists in this document. Otherwise the original
    // flat walk runs byte-for-byte (one boolean read, predicted false).
    const doc = this.ownerDocument || this;
    const useShadow = !!(doc && doc.__hasShadow);
    if (!useShadow) {
      let node = this;
      while (node) {
        path.push(node);
        if (!hasListener) { const l = node.__listeners && node.__listeners.get(type); if (l && l.length) hasListener = true; }
        node = node.parentNode || node.__owner || null;
      }
    } else {
      const targetRoot = treeRootOf(target);
      let node = this;
      while (node) {
        path.push(node);
        if (!hasListener) { const l = node.__listeners && node.__listeners.get(type); if (l && l.length) hasListener = true; }
        if (node.__isShadowRoot) {
          // a non-composed event stops at the shadow boundary enclosing its
          // target; a composed event continues up through the host.
          node = (!event.composed && node === targetRoot) ? null : (node.host || null);
        } else {
          node = node.parentNode || node.__owner || null;
        }
      }
    }
    event._path = path;

    // pre-click activation (WHATWG): checkbox/radio toggle BEFORE click listeners
    // run, so React's change detection sees the new value. Undone if preventDefault.
    let activation = null;
    if (event.type === 'click' && typeof this.__preClickActivation === 'function') {
      activation = this.__preClickActivation();
    }

    // relatedTarget (focus/mouseenter/mouseleave) is retargeted per listener
    // exactly like target. Only relevant under shadow with a non-null value.
    const relatedTarget = useShadow ? event.relatedTarget : null;
    const invoke = (node, phase) => {
      const list = node.__listeners && node.__listeners.get(event.type);
      if (!list || list.length === 0) return;
      event.currentTarget = node;
      event.eventPhase = phase;
      // present `target`/`relatedTarget` retargeted to the listener's tree
      if (useShadow) {
        event.target = retarget(target, node);
        if (relatedTarget != null) event.relatedTarget = retarget(relatedTarget, node);
      }
      // snapshot — listeners added during dispatch don't fire this round. Skip the
      // slice alloc for the single-listener case (common React-delegated one): a
      // 1-element, length-captured iteration can't be disturbed by an add/remove in
      // that one call. Multi-listener keeps the slice (concurrent-mutation guard).
      const snap = list.length === 1 ? list : list.slice();
      for (let k = 0, len = snap.length; k < len; k++) {
        const l = snap[k];
        if (phase === PHASE_CAPTURING && !l.capture) continue;
        if (phase === PHASE_BUBBLING && l.capture) continue;
        if (l.once) {
          const cur = node.__listeners.get(event.type);
          const idx = cur ? cur.indexOf(l) : -1;
          if (idx !== -1) cur.splice(idx, 1);
        }
        event._passiveListener = l.passive;
        const handler = typeof l.callback === 'function' ? l.callback : l.callback.handleEvent;
        try {
          handler.call(node, event);
        } finally {
          event._passiveListener = false;
        }
        if (event._stopImmediate) return;
      }
    };

    // no listener anywhere on the path → skip all three propagation phases
    if (hasListener) {
      // capturing: root -> just before target
      for (let i = path.length - 1; i >= 1; i--) {
        if (event._stopPropagation) break;
        invoke(path[i], PHASE_CAPTURING);
      }
      // at target
      if (!event._stopPropagation) invoke(path[0], PHASE_AT_TARGET);
      // bubbling: target's parent -> root
      if (event.bubbles) {
        for (let i = 1; i < path.length; i++) {
          if (event._stopPropagation) break;
          invoke(path[i], PHASE_BUBBLING);
        }
      }
    }

    event.eventPhase = PHASE_NONE;
    event.currentTarget = null;
    if (useShadow) { // restore from any retargeting
      event.target = target;
      if (relatedTarget != null) event.relatedTarget = relatedTarget;
    }

    // canceled activation: undo the pre-click toggle if default was prevented.
    // Otherwise a checkbox/radio toggle fires input then change (activation
    // default action) — user-event/React rely on these to see the new value.
    if (activation && event.defaultPrevented) activation.undo();
    else if (activation && activation.fireChange) {
      this.dispatchEvent(new Event('input', { bubbles: true }));
      this.dispatchEvent(new Event('change', { bubbles: true }));
    }

    // remaining default actions (label→control, form submit) unless prevented
    if (!event.defaultPrevented && typeof this.__runDefaultAction === 'function') {
      this.__runDefaultAction(event);
    }
    return !event.defaultPrevented;
  }
}

// on<event> handler properties (onclick, oninput, …). Defining these makes
// `'oninput' in document` true, so React skips its legacy attachEvent polyfill,
// and lets libraries assign el.onX = fn directly.
const ON_EVENTS = [
  'click', 'dblclick', 'input', 'change', 'focus', 'blur', 'focusin', 'focusout',
  'keydown', 'keyup', 'keypress', 'mousedown', 'mouseup', 'mousemove', 'mouseover',
  'mouseout', 'mouseenter', 'mouseleave', 'submit', 'reset', 'load', 'error',
  'scroll', 'wheel', 'contextmenu', 'pointerdown', 'pointerup', 'pointermove',
  'pointerenter', 'pointerleave', 'pointercancel', 'touchstart', 'touchend',
  'touchmove', 'animationstart', 'animationend', 'transitionend', 'paste', 'copy',
  'cut', 'drop', 'dragstart', 'dragover', 'dragend', 'select', 'invalid', 'beforeinput',
];
for (const type of ON_EVENTS) {
  const slot = '__on_' + type;
  Object.defineProperty(EventTarget.prototype, 'on' + type, {
    configurable: true,
    get() { return this[slot] || null; },
    set(fn) {
      const prev = this[slot];
      if (prev) this.removeEventListener(type, prev);
      this[slot] = (typeof fn === 'function') ? fn : null;
      if (this[slot]) this.addEventListener(type, this[slot]);
    },
  });
}

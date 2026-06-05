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
    this._path = [];
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

  composedPath() { return this._path.slice(); }
}

export class CustomEvent extends Event {
  constructor(type, init = {}) {
    super(type, init);
    this.detail = init.detail ?? null;
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
  return class extends Event {
    constructor(type, init = {}) {
      super(type, init);
      Object.assign(this, defaults, init);
    }
  };
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

function normalizeOptions(options) {
  if (typeof options === 'boolean') return { capture: options, once: false, passive: false };
  options = options || {};
  return { capture: !!options.capture, once: !!options.once, passive: !!options.passive };
}

export class EventTarget {
  constructor() {
    // type -> array of { callback, capture, once, passive }
    this.__listeners = new Map();
  }

  addEventListener(type, callback, options) {
    if (callback == null) return;
    const o = normalizeOptions(options);
    let list = this.__listeners.get(type);
    if (!list) { list = []; this.__listeners.set(type, list); }
    // dedupe on (callback, capture) per spec
    if (list.some((l) => l.callback === callback && l.capture === o.capture)) return;
    list.push({ callback, capture: o.capture, once: o.once, passive: o.passive });
  }

  removeEventListener(type, callback, options) {
    const o = normalizeOptions(options);
    const list = this.__listeners.get(type);
    if (!list) return;
    const i = list.findIndex((l) => l.callback === callback && l.capture === o.capture);
    if (i !== -1) list.splice(i, 1);
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
    event.target = this;
    event._path = this.__eventPath();
    const path = event._path;

    const invoke = (node, phase) => {
      const list = node.__listeners && node.__listeners.get(event.type);
      if (!list || list.length === 0) return;
      event.currentTarget = node;
      event.eventPhase = phase;
      // snapshot — listeners added during dispatch don't fire this round
      for (const l of list.slice()) {
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

    event.eventPhase = PHASE_NONE;
    event.currentTarget = null;

    // default actions (checkbox toggle, label→control, etc.) unless prevented
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

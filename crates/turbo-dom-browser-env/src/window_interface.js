// Chrome 156's Window interface chain (miaskiewicz/turbo-dom#223):
// window -> Window.prototype -> WindowProperties -> EventTarget.prototype -> Object.prototype.
// The window's own add/remove/dispatchEvent move behind EventTarget.prototype's methods,
// which route a window receiver to the window's listener list.
(function () {
  'use strict';
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction;
  const define = Object.defineProperty, apply = Reflect.apply, setPrototype = Object.setPrototypeOf;
  const ErrorType = TypeError, target = g.EventTarget.prototype, windowCallbacks = ns.windowEventCallbacks;
  for (const [name, length] of [['addEventListener', 2], ['removeEventListener', 2], ['dispatchEvent', 1]]) {
    const generic = target[name], forWindow = windowCallbacks[name];
    define(target, name, { value: make({ [name](...args) { return apply(this === g ? forWindow : generic, this, args); } }[name], name, length), writable: true, enumerable: true, configurable: true });
    delete g[name];
  }
  const properties = setPrototype({}, target);
  define(properties, Symbol.toStringTag, { value: 'WindowProperties', configurable: true });
  const Window = make(function Window() { throw new ErrorType(new.target ? "Failed to construct 'Window': Illegal constructor" : 'Illegal constructor'); }, 'Window', 0, true, true);
  const proto = setPrototype({}, properties);
  define(Window, 'TEMPORARY', { value: 0, enumerable: true });
  define(Window, 'PERSISTENT', { value: 1, enumerable: true });
  define(proto, 'TEMPORARY', { value: 0, enumerable: true });
  define(proto, 'PERSISTENT', { value: 1, enumerable: true });
  define(proto, 'constructor', { value: Window, writable: true, configurable: true });
  define(proto, Symbol.toStringTag, { value: 'Window', configurable: true });
  define(Window, 'prototype', { value: proto, writable: false, enumerable: false, configurable: false });
  setPrototype(Window, g.EventTarget);
  define(g, 'Window', { value: Window, writable: true, configurable: true });
  setPrototype(g, proto);
})();

// focusgroup: the `focusGroupStart` boolean reflection (the `focusGroup` DOMTokenList lives
// with the other token lists) and the keydown default action the native dispatch calls.
(function () {
  'use strict';
  const Native = globalThis.__focusGroupNative; delete globalThis.__focusGroupNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction;
  const accessor = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).get;
  const nodeType = accessor(g.Node.prototype, 'nodeType'), namespace = accessor(g.Element.prototype, 'namespaceURI');
  const hasAttribute = g.Element.prototype.hasAttribute, setAttribute = g.Element.prototype.setAttribute, removeAttribute = g.Element.prototype.removeAttribute;
  const focus = g.HTMLElement.prototype.focus, computed = g.getComputedStyle;
  const getPropertyValue = g.CSSStyleDeclaration ? g.CSSStyleDeclaration.prototype.getPropertyValue : null;
  const parentNode = accessor(g.Node.prototype, 'parentNode'), hostOf = accessor(g.ShadowRoot.prototype, 'host');
  const slotOf = Object.getOwnPropertyDescriptor(g.Element.prototype, 'assignedSlot');
  const assignedSlot = slotOf && slotOf.get;
  const up = node => { try { const slot = assignedSlot && assignedSlot.call(node); if (slot) return slot; } catch {} const parent = parentNode.call(node); if (parent) return parent; try { return hostOf.call(node); } catch { return null; } };
  // `writing-mode` is inherited; the snapshot holds authored values, so take the nearest one.
  const writingMode = node => {
    for (let current = node; current; current = up(current)) {
      if (nodeType.call(current) !== 1) continue;
      let value = '';
      try { const style = computed(current); value = getPropertyValue ? Reflect.apply(getPropertyValue, style, ['writing-mode']) : style['writing-mode']; } catch {}
      value = String(value || '').trim().toLowerCase();
      if (value && value !== 'inherit' && value !== 'unset' && value !== 'revert') return value === 'initial' ? 'horizontal-tb' : value;
    }
    return 'horizontal-tb';
  };
  const keyOf = accessor(g.KeyboardEvent.prototype, 'key');
  const modifiers = ['altKey', 'ctrlKey', 'metaKey', 'shiftKey'].map(name => accessor(g.KeyboardEvent.prototype, name));
  for (const [constructor, space] of [[g.HTMLElement, 'http://www.w3.org/1999/xhtml'], [g.SVGElement, 'http://www.w3.org/2000/svg']]) {
    if (typeof constructor !== 'function') continue;
    const check = receiver => { let valid = false; try { valid = nodeType.call(receiver) === 1 && namespace.call(receiver) === space; } catch {} if (!valid) throw new TypeError('Illegal invocation'); };
    Object.defineProperty(constructor.prototype, 'focusGroupStart', {
      enumerable: true, configurable: true,
      get: make(function () { check(this); return hasAttribute.call(this, 'focusgroupstart'); }, 'get focusGroupStart', 0),
      set: make(function (value) { check(this); if (value) setAttribute.call(this, 'focusgroupstart', ''); else removeAttribute.call(this, 'focusgroupstart'); }, 'set focusGroupStart', 1),
    });
  }
  const keys = ['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown', 'Home', 'End'];
  g.__captureFocusGroupHook = event => {
    let key;
    try { key = keyOf.call(event); for (let index = 0; index < modifiers.length; index++) if (modifiers[index].call(event)) return; } catch { return; }
    let known = false;
    for (let index = 0; index < keys.length; index++) if (keys[index] === key) known = true;
    if (!known) return;
    const focused = Native('grouped');
    if (!focused) return;
    const next = Native('navigate', key, writingMode(focused));
    if (next) focus.call(next);
  };
})();

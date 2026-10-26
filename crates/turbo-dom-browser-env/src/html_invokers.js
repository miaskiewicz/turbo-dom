// Invoker reflection (command/commandfor, popovertarget/popovertargetaction), a genuine
// CommandEvent, and the private button/input activation the native click dispatch calls.
(function () {
  'use strict';
  const Native = globalThis.__invokerNative; delete globalThis.__invokerNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction, StringValue = String;
  const reference = ns.elementReference, retarget = ns.retargetEventSource;
  const dialogSteps = ns.dialogCommandSteps, resolve = ns.resolveReferenceTarget;
  delete ns.elementReference; delete ns.retargetEventSource; delete ns.dialogCommandSteps; delete ns.resolveReferenceTarget;
  const accessor = (proto, name) => Object.getOwnPropertyDescriptor(proto, name).get;
  const nodeType = accessor(g.Node.prototype, 'nodeType'), namespace = accessor(g.Element.prototype, 'namespaceURI');
  const localName = accessor(g.Element.prototype, 'localName'), parentNode = accessor(g.Node.prototype, 'parentNode');
  const connected = accessor(g.Node.prototype, 'isConnected'), hostOf = accessor(g.ShadowRoot.prototype, 'host');
  const bubblesOf = accessor(g.Event.prototype, 'bubbles'), composedOf = accessor(g.Event.prototype, 'composed');
  const mouseButton = accessor(g.MouseEvent.prototype, 'button'), popoverOf = accessor(g.HTMLElement.prototype, 'popover');
  const formOf = accessor(g.HTMLButtonElement.prototype, 'form'), inputFormOf = accessor(g.HTMLInputElement.prototype, 'form');
  const reset = g.HTMLFormElement.prototype.reset, contains = g.Node.prototype.contains;
  const getAttribute = g.Element.prototype.getAttribute, hasAttribute = g.Element.prototype.hasAttribute, setAttribute = g.Element.prototype.setAttribute, removeAttribute = g.Element.prototype.removeAttribute;
  const construct = Reflect.construct, Event = g.Event;
  const HTML = 'http://www.w3.org/1999/xhtml';
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return StringValue(value); };
  const lower = value => { let result = ''; for (let index = 0; index < value.length; index++) { const code = value.charCodeAt(index); result += code >= 65 && code <= 90 ? StringValue.fromCharCode(code + 32) : value[index]; } return result; };
  const element = value => { try { return nodeType.call(value) === 1; } catch (_) { return false; } };
  const html = (value, name) => element(value) && namespace.call(value) === HTML && localName.call(value) === name;
  const brand = name => receiver => { if (!html(receiver, name)) throw new TypeError('Illegal invocation'); };
  const attr = (node, name) => getAttribute.call(node, name);

  const popoverCommands = ['toggle-popover', 'show-popover', 'hide-popover'], dialogCommands = ['show-modal', 'close', 'request-close'];
  const known = value => { for (const list of [popoverCommands, dialogCommands]) for (let index = 0; index < list.length; index++) if (list[index] === value) return true; return false; };
  // Known keywords canonicalize ASCII-case-insensitively; custom `--` commands keep their case.
  const commandState = node => { const value = attr(node, 'command'); if (value === null) return ''; const canonical = lower(value); return known(canonical) ? canonical : value.slice(0, 2) === '--' ? value : ''; };
  const targetAction = node => { const value = lower(attr(node, 'popovertargetaction') || ''); return value === 'show' || value === 'hide' ? value : 'toggle'; };
  // A button whose type is missing or invalid is in the Auto state: submit, unless it is an invoker.
  const buttonType = node => { const value = lower(attr(node, 'type') || ''); if (value === 'submit' || value === 'reset' || value === 'button') return value; return hasAttribute.call(node, 'command') || hasAttribute.call(node, 'commandfor') ? 'auto' : 'submit'; };

  const define = (proto, name, get, set) => Object.defineProperty(proto, name, { enumerable: true, configurable: true, get: make(get, 'get ' + name, 0), set: make(set, 'set ' + name, 1) });
  for (const [constructor, tag] of [[g.HTMLButtonElement, 'button'], [g.HTMLInputElement, 'input']]) {
    const check = brand(tag);
    define(constructor.prototype, 'popoverTargetElement', function () { check(this); return reference.read(this, 'popovertarget'); }, function (value) { check(this); reference.assign(this, 'popovertarget', value); });
    define(constructor.prototype, 'popoverTargetAction', function () { check(this); return targetAction(this); }, function (value) { check(this); setAttribute.call(this, 'popovertargetaction', string(value)); });
  }
  const button = brand('button');
  define(g.HTMLButtonElement.prototype, 'commandForElement', function () { button(this); return reference.read(this, 'commandfor'); }, function (value) { button(this); reference.assign(this, 'commandfor', value); });
  define(g.HTMLButtonElement.prototype, 'command', function () { button(this); return commandState(this); }, function (value) { button(this); setAttribute.call(this, 'command', string(value)); });
  define(g.HTMLButtonElement.prototype, 'type', function () { button(this); const type = buttonType(this); return type === 'auto' ? 'button' : type; }, function (value) { button(this); setAttribute.call(this, 'type', string(value)); });

  const records = new WeakMap(), read = records.get.bind(records), store = records.set.bind(records);
  const Delegate = function (type, init) {
    if (!new.target) throw new TypeError('CommandEvent requires new');
    if (!arguments.length) throw new TypeError('CommandEvent requires 1 argument');
    const convertedType = string(type);
    if (init != null && typeof init !== 'object' && typeof init !== 'function') throw new TypeError('CommandEventInit must be a dictionary');
    init = init == null ? {} : init;
    const eventInit = { bubbles: !!init.bubbles, cancelable: !!init.cancelable, composed: !!init.composed };
    const commandValue = init.command, command = commandValue === undefined ? '' : string(commandValue);
    const sourceValue = init.source, source = sourceValue === undefined || sourceValue === null ? null : sourceValue;
    if (source !== null && !element(source)) throw new TypeError('CommandEvent source must be an Element');
    const event = construct(Event, [convertedType, eventInit], new.target);
    store(event, { source, command });
    return event;
  };
  Object.setPrototypeOf(Delegate.prototype, Event.prototype);
  delete Delegate.prototype.constructor;
  Object.defineProperty(Delegate.prototype, Symbol.toStringTag, { value: 'CommandEvent', configurable: true });
  for (const name of ['source', 'command']) Object.defineProperty(Delegate.prototype, name, {
    enumerable: true, configurable: true,
    get: make(function () { const record = read(this); if (!record) throw new TypeError('Illegal invocation'); return name === 'source' ? retarget(record.source, this) : record.command; }, 'get ' + name, 0),
  });
  const CommandEvent = make(Delegate, 'CommandEvent', 1, true, true);
  CommandEvent.prototype = Delegate.prototype;
  Object.defineProperty(Delegate.prototype, 'constructor', { value: CommandEvent, writable: true, configurable: true });
  Object.defineProperty(CommandEvent, 'prototype', { writable: false });
  Object.setPrototypeOf(CommandEvent, Event);
  Object.defineProperty(g, 'CommandEvent', { value: CommandEvent, writable: true, configurable: true });

  const disabled = node => Native('disabled', node);
  const shadowHost = root => { try { return hostOf.call(root); } catch (_) { return null; } };
  const shadowIncludingDescendant = (node, ancestor) => { for (let current = node; current;) { if (current === ancestor) return true; current = parentNode.call(current) || shadowHost(current); } return false; };
  // https://html.spec.whatwg.org/#popover-target-attribute-activation-behavior
  function popoverTargetActivation(node, eventTarget) {
    // ID and explicit references to a shadow host continue through its reference target.
    const popover = resolve(reference.read(node, 'popovertarget'));
    if (popover === null || namespace.call(popover) !== HTML || popoverOf.call(popover) === null) return;
    if (shadowIncludingDescendant(eventTarget, popover) && popover !== node && shadowIncludingDescendant(popover, node)) return;
    Native('popover', popover, targetAction(node), node);
  }
  function validCommand(target, command) {
    if (command.slice(0, 2) === '--') return true;
    if (namespace.call(target) !== HTML) return false;
    for (let index = 0; index < popoverCommands.length; index++) if (popoverCommands[index] === command) return true;
    return localName.call(target) === 'dialog';
  }
  function buttonActivation(node, eventTarget) {
    if (disabled(node)) return;
    const type = buttonType(node);
    if (formOf.call(node) !== null) {
      if (type === 'reset') reset.call(formOf.call(node));
      if (type !== 'button') return;
    }
    const target = resolve(reference.read(node, 'commandfor')), command = commandState(node);
    if (target !== null && command !== '' && validCommand(target, command)) {
      const event = new CommandEvent('command', { cancelable: true, composed: true, command, source: node });
      if (!Native('dispatch', target, event) || !connected.call(target) || command.slice(0, 2) === '--') return;
      if (command === 'toggle-popover' || command === 'show-popover' || command === 'hide-popover') {
        if (namespace.call(target) === HTML && popoverOf.call(target) !== null) Native('popover', target, command.slice(0, -8), node);
      } else if (html(target, 'dialog')) dialogSteps(target, command, node);
      return;
    }
    popoverTargetActivation(node, eventTarget);
  }
  function inputActivation(node, eventTarget) {
    const type = lower(attr(node, 'type') || '');
    if (type !== 'button' && type !== 'reset' && type !== 'submit' && type !== 'image') return;
    if (disabled(node)) return;
    const form = inputFormOf.call(node);
    if (form !== null && type !== 'button') { if (type === 'reset') reset.call(form); return; }
    popoverTargetActivation(node, eventTarget);
  }
  // https://dom.spec.whatwg.org/#eventtarget-activation-behavior: elements with activation behavior.
  const activates = node => {
    if (!element(node) || namespace.call(node) !== HTML) return false;
    const name = localName.call(node);
    return name === 'button' || name === 'input' || name === 'label' || name === 'summary' || ((name === 'a' || name === 'area') && hasAttribute.call(node, 'href'));
  };
  // Only a MouseEvent `click` has an activation target (isTrusted is irrelevant): the target
  // itself, or for bubbling events its nearest ancestor with activation behavior, crossing
  // shadow roots only when the event is composed.
  g.__captureActivationTarget = function (event, target) {
    try { mouseButton.call(event); } catch (_) { return null; }
    let node = target;
    if (activates(node)) return node;
    if (!bubblesOf.call(event)) return null;
    do { const parent = parentNode.call(node); node = parent && nodeType.call(parent) === 11 ? (composedOf.call(event) ? shadowHost(parent) : null) : parent; } while (node && !activates(node));
    return node || null;
  };
  const labelPrototype = g.HTMLLabelElement.prototype, focus = g.HTMLElement.prototype.focus;
  let labelControl = null;
  const firstChild = accessor(g.Node.prototype, 'firstChild'), nextSibling = accessor(g.Node.prototype, 'nextSibling');
  // https://html.spec.whatwg.org/#interactive-content between the event target and its label.
  const interactive = node => {
    if (!element(node) || namespace.call(node) !== HTML) return false;
    const name = localName.call(node);
    if (name === 'button' || name === 'details' || name === 'embed' || name === 'iframe' || name === 'label' || name === 'select' || name === 'textarea') return true;
    if (name === 'a') return hasAttribute.call(node, 'href');
    if (name === 'input') return lower(attr(node, 'type') || '') !== 'hidden';
    if (name === 'audio' || name === 'video') return hasAttribute.call(node, 'controls');
    if (name === 'img') return hasAttribute.call(node, 'usemap');
    return false;
  };
  // Chrome's label activation: focus the labeled control and fire a synthetic click at it,
  // trusted exactly when the activating click was.
  function labelActivation(label, eventTarget, event) {
    if (labelControl === null) labelControl = accessor(labelPrototype, 'control');
    const control = labelControl.call(label);
    if (control === null || shadowIncludingDescendant(eventTarget, control)) return;
    for (let node = eventTarget; node && node !== label; node = parentNode.call(node) || shadowHost(node)) if (interactive(node)) return;
    if (disabled(control)) return;
    focus.call(control);
    Native('click', control, event);
  }
  // https://html.spec.whatwg.org/#the-summary-element activation behavior.
  function summaryActivation(summary) {
    const details = parentNode.call(summary);
    if (!html(details, 'details')) return;
    let first = firstChild.call(details);
    while (first !== null && !html(first, 'summary')) first = nextSibling.call(first);
    if (first !== summary) return;
    if (hasAttribute.call(details, 'open')) removeAttribute.call(details, 'open'); else setAttribute.call(details, 'open', '');
  }
  // Runs after an uncanceled `click` whose activation target is `node`. Hyperlink following
  // for a/area is not implemented: this engine has no navigation from activation.
  g.__captureInvokerActivation = function (event, node, target) {
    const name = localName.call(node);
    if (name === 'button') buttonActivation(node, target);
    else if (name === 'input') inputActivation(node, target);
    else if (name === 'label') labelActivation(node, target, event);
    else if (name === 'summary') summaryActivation(node);
  };
  // Light dismiss treats an invoker of a showing popover as inside that popover.
  ns.invokerPopoverTarget = node => {
    if (!html(node, 'button') && !html(node, 'input')) return null;
    const popover = reference.read(node, 'popovertarget');
    return popover !== null && namespace.call(popover) === HTML && popoverOf.call(popover) !== null ? popover : null;
  };
})();

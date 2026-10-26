// HTMLDialogElement: open/returnValue/closedBy reflection and show/showModal/close/requestClose
// with Chrome's trusted beforetoggle/toggle/cancel/close events and dialog focusing steps,
// plus the private dialog steps invoker commands and method=dialog form submission run.
// Open dialogs establish a native close watcher (user_activation_gates.rs), which runs Esc
// close requests from embedder input and closedby=any light dismiss through the steps below.
(function () {
  'use strict';
  const Native = globalThis.__dialogNative; delete globalThis.__dialogNative;
  const g = globalThis, ns = g[Symbol.for('ts')], make = ns.makeNativeFunction, StringValue = String;
  const apply = Reflect.apply, define = Object.defineProperty, create = Object.create;
  const check = ns.domAccessor('nodeType', false, 'HTML:dialog'), connectedGetter = ns.domAccessor('isConnected', false, 'Node');
  const getAttribute = g.Element.prototype.getAttributeNS, setAttribute = g.Element.prototype.setAttributeNS, removeAttribute = g.Element.prototype.removeAttributeNS;
  const focus = g.HTMLElement.prototype.focus;
  const ToggleEvent = g.ToggleEvent, EventType = g.Event, Exception = g.DOMException, ErrorType = TypeError;
  const records = new WeakMap(), weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set;
  const lower = Function.prototype.call.bind(String.prototype.toLowerCase);
  const string = (value, operation) => { if (typeof value === 'symbol') throw new ErrorType(operation + ': Cannot convert a Symbol value to a string'); return StringValue(value); };
  const brand = receiver => apply(check, receiver, []);
  const connected = node => apply(connectedGetter, node, []);
  const open = node => apply(getAttribute, node, [null, 'open']) !== null;
  const modal = node => Native('modal', node);
  const fail = (operation, message) => new Exception("Failed to execute '" + operation + "' on 'HTMLDialogElement': " + message, 'InvalidStateError');
  // returnValue, previously focused element, request-close state, the close-in-progress and
  // cancel-action guards and the dialog toggle task tracker.
  function record(node) {
    let state = apply(weakGet, records, [node]);
    if (!state) { state = create(null); state.returnValue = ''; state.previous = null; state.requestValue = null; state.requestSource = null; state.closing = false; state.canceling = false; state.toggle = null; apply(weakSet, records, [node, state]); }
    return state;
  }
  const dispatch = (node, event) => Native('dispatch', node, event);
  // Chrome composes toggle events exactly when they carry a source.
  const beforetoggle = (node, opening, source) => dispatch(node, new ToggleEvent('beforetoggle', { __proto__: null, oldState: opening ? 'closed' : 'open', newState: opening ? 'open' : 'closed', cancelable: opening, composed: source !== null, source }));
  // https://html.spec.whatwg.org/#queue-a-dialog-toggle-event-task: a pending task is
  // canceled and requeued, keeping its old state.
  function queueToggle(node, oldState, newState, source) {
    const state = record(node), pending = state.toggle;
    if (pending !== null) { oldState = pending.oldState; pending.canceled = true; }
    const task = create(null); task.oldState = oldState; task.newState = newState; task.source = source; task.canceled = false;
    state.toggle = task;
    Native('schedule', node, () => {
      if (task.canceled) return;
      state.toggle = null;
      dispatch(node, new ToggleEvent('toggle', { __proto__: null, oldState: task.oldState, newState: task.newState, composed: task.source !== null, source: task.source }));
    });
  }
  function focusingSteps(node) {
    if (!connected(node)) return;
    apply(focus, Native('delegate', node), []);
  }
  // https://html.spec.whatwg.org/#show-a-modal-dialog and the show() method steps.
  function show(node, isModal, source) {
    const operation = isModal ? 'showModal' : 'show';
    if (open(node)) {
      if (modal(node) === isModal) return;
      throw fail(operation, isModal ? 'The dialog is already open as a non-modal dialog, and therefore cannot be opened as a modal dialog.' : 'The dialog is already open as a modal dialog, and therefore cannot be opened as a non-modal dialog.');
    }
    if (isModal) {
      if (!connected(node)) throw fail(operation, 'The element is not in a Document.');
      if (!Native('active', node)) throw fail(operation, 'Invalid for dialogs within documents that are not fully active.');
      if (Native('popover', node)) throw fail(operation, 'The dialog is already open as a Popover, and therefore cannot be opened as a modal dialog.');
    }
    if (!beforetoggle(node, true, source)) return;
    if (open(node) || isModal && (!connected(node) || Native('popover', node))) return;
    queueToggle(node, 'closed', 'open', source);
    apply(setAttribute, node, [null, 'open', '']);
    if (isModal) Native('set-modal', node, true);
    Native('opened', node);
    record(node).previous = Native('focused', node);
    Native('hide-popovers', node);
    focusingSteps(node);
  }
  // https://html.spec.whatwg.org/#close-the-dialog
  function close(node, result, source) {
    const state = record(node);
    if (!open(node) || state.closing) return;
    state.closing = true;
    try {
      beforetoggle(node, false, source);
      if (!open(node)) return;
      queueToggle(node, 'open', 'closed', source);
      apply(removeAttribute, node, [null, 'open']);
      const wasModal = modal(node);
      Native('set-modal', node, false);
      Native('closed', node);
      if (result !== null) state.returnValue = result;
      state.requestValue = null; state.requestSource = null;
      const previous = state.previous;
      if (previous !== null) {
        state.previous = null;
        if ((Native('focus-within', node) || wasModal) && connected(previous)) apply(focus, previous, []);
      }
      Native('schedule', node, () => dispatch(node, new EventType('close', { __proto__: null })));
    } finally { state.closing = false; }
  }
  // https://html.spec.whatwg.org/#dialog-request-close: the close watcher exists while the
  // dialog is open and connected; requestClose() never requires history-action activation,
  // so its cancel event is always cancelable.
  function requestClose(node, result, source) {
    const state = record(node);
    if (!open(node) || !connected(node)) return;
    state.requestValue = result; state.requestSource = source;
    if (state.canceling) return;
    state.canceling = true;
    let proceed;
    try { proceed = dispatch(node, new EventType('cancel', { __proto__: null, cancelable: true })); } finally { state.canceling = false; }
    if (!proceed) { state.requestValue = null; state.requestSource = null; return; }
    if (!open(node) || !connected(node)) return;
    close(node, state.requestValue, state.requestSource);
  }
  const proto = g.HTMLDialogElement.prototype;
  const accessor = (name, get, set) => define(proto, name, { enumerable: true, configurable: true, get: make(get, 'get ' + name, 0), set: make(set, 'set ' + name, 1) });
  accessor('open', function () { brand(this); return open(this); }, function (value) { brand(this); if (value) apply(setAttribute, this, [null, 'open', '']); else apply(removeAttribute, this, [null, 'open']); });
  accessor('returnValue', function () { brand(this); return record(this).returnValue; }, function (value) { brand(this); record(this).returnValue = string(value, "Failed to set the 'returnValue' property on 'HTMLDialogElement'"); });
  // Auto (missing or invalid) is closerequest while modal and none otherwise.
  accessor('closedBy', function () {
    brand(this); const value = apply(getAttribute, this, [null, 'closedby']), keyword = value === null ? '' : lower(value);
    return keyword === 'any' || keyword === 'closerequest' || keyword === 'none' ? keyword : modal(this) ? 'closerequest' : 'none';
  }, function (value) { brand(this); apply(setAttribute, this, [null, 'closedby', string(value, "Failed to set the 'closedBy' property on 'HTMLDialogElement'")]); });
  const optional = (value, operation) => value === undefined ? null : string(value, "Failed to execute '" + operation + "' on 'HTMLDialogElement'");
  const methods = {
    close(returnValue) { brand(this); close(this, optional(returnValue, 'close'), null); },
    requestClose(returnValue) { brand(this); requestClose(this, optional(returnValue, 'requestClose'), null); },
    show() { brand(this); show(this, false, null); },
    showModal() { brand(this); show(this, true, null); },
  };
  for (const name of ['close', 'requestClose', 'show', 'showModal']) define(proto, name, { value: make(methods[name], name, 0), writable: true, enumerable: true, configurable: true });
  // Chrome lists the interface members before constructor and @@toStringTag.
  for (const key of ['constructor', Symbol.toStringTag]) { const descriptor = Object.getOwnPropertyDescriptor(proto, key); if (descriptor) { delete proto[key]; define(proto, key, descriptor); } }

  // The close watcher's cancel action fires `cancel`, cancelable only when the watcher may
  // prevent closing; its close action closes with the pending request-close result.
  ns.dialogCloseWatcher = (node, action, cancelable) => {
    const state = record(node);
    if (action === 'close') { close(node, state.requestValue, state.requestSource); return true; }
    state.canceling = true;
    try { return dispatch(node, new EventType('cancel', { __proto__: null, cancelable: !!cancelable })); } finally { state.canceling = false; }
  };
  // https://html.spec.whatwg.org/#the-dialog-element command steps; exceptions are suppressed.
  ns.dialogCommandSteps = (node, command, source) => {
    const value = apply(getAttribute, source, [null, 'value']);
    if (command === 'show-modal') { if (!open(node)) try { show(node, true, source); } catch (_) {} }
    else if (command === 'close') { if (open(node)) close(node, value, source); }
    else if (command === 'request-close') { if (open(node)) requestClose(node, value, source); }
  };
  // https://html.spec.whatwg.org/#form-submission-algorithm method=dialog: close the form's
  // nearest ancestor dialog with the submitter's result. Chrome's result is '' without a
  // submitter, "x,y" for an image button (activation is never pointer-located here, so 0,0),
  // else the submitter's value attribute.
  const parentGetter = ns.domAccessor('parentElement', false, 'Node'), localName = ns.domAccessor('localName', false, 'Element'), namespace = ns.domAccessor('namespaceURI', false, 'Element');
  ns.dialogFormSubmission = (form, submitter) => {
    let node = apply(parentGetter, form, []);
    while (node !== null && !(apply(localName, node, []) === 'dialog' && apply(namespace, node, []) === 'http://www.w3.org/1999/xhtml')) node = apply(parentGetter, node, []);
    if (node === null) return;
    let result = '';
    if (submitter !== null) {
      const type = apply(getAttribute, submitter, [null, 'type']);
      result = apply(localName, submitter, []) === 'input' && type !== null && lower(type) === 'image' ? '0,0' : apply(getAttribute, submitter, [null, 'value']);
    }
    close(node, result, null);
  };
})();

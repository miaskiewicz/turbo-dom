(async () => {
  // miaskiewicz/turbo-dom#194: the page replaces every configurable member of the builtins with
  // throwing functions, calls only originals it captured first, then drives the ARIA element
  // references, DOMTokenList, focusgroup, referenceTarget and interest-invoker members (and the
  // trusted keydown / hover / Escape paths behind them), and restores the builtins. Chrome
  // 156.0.8078.47 (headless, localhost) is the control. Offline, the trusted-input driver is
  // page JS, so the few builtins it needs (DRIVER_NEEDS) stay unpatched.
  const A = Reflect.apply, gopd = Object.getOwnPropertyDescriptor, def = Object.defineProperty, own = Reflect.ownKeys;
  const Err = Error, str = JSON.stringify, Prom = Promise, round = Math.round, wait = globalThis.setTimeout;
  const getProto = Object.getPrototypeOf, find = (o, k) => { for (; o !== null; o = getProto(o)) { const d = gopd(o, k); if (d) return d; } };
  const fn = (o, k) => find(o, k).value, get = (o, k) => find(o, k).get, set = (o, k) => find(o, k).set;
  const E = Element.prototype, H = HTMLElement.prototype, N = Node.prototype;
  const byId = fn(Document.prototype, 'getElementById'), idOf = get(E, 'id'), focus = fn(H, 'focus'), matches = fn(E, 'matches');
  const setAttr = fn(E, 'setAttribute'), rect = fn(E, 'getBoundingClientRect');
  const active = get(Document.prototype, 'activeElement');
  const attachShadow = fn(E, 'attachShadow'), createElement = fn(Document.prototype, 'createElement'), shadowRootOf = get(E, 'shadowRoot');
  const T = DOMTokenList.prototype, IE = InterestEvent, sourceOf = get(IE.prototype, 'source'), lengthOf = get(T, 'length');
  const cdp = globalThis.__cdpInput;
  let next = 0; const waiters = { __proto__: null }; const targets = globalThis.__cdpTargets = [];
  globalThis.__cdpInputDone = id => { const resolve = waiters[id]; waiters[id] = undefined; resolve(); };
  const input = step => new Prom(resolve => { const id = ++next; waiters[id] = resolve; cdp(str({ id, steps: [step] })); });
  const sleep = ms => new Prom(resolve => { wait(resolve, ms); });
  const q = id => A(byId, document, [id]);
  const hover = node => { const box = A(rect, node, []); targets[targets.length] = node; return input({ type: 'move', target: targets.length - 1, x: round(box.left + box.width / 2), y: round(box.top + box.height / 2) }); };
  const key = name => input({ type: 'key', key: name, code: name, keyCode: name === 'Escape' ? 27 : 0 });
  const ids = list => { let text = ''; for (let index = 0; index < list.length; index++) text += (index ? ' ' : '') + A(idOf, list[index], []); return text; };

  document.body.innerHTML = '<div id=park style="position:fixed;top:520px;left:0;width:20px;height:20px"></div>'
    + '<span id=a></span><span id=b></span><div id=host class="one two"></div><iframe id=frame></iframe><template id=tpl></template>'
    + '<div id=group focusgroup="toolbar"><button id=f1>1</button><button id=f2>2</button></div>'
    + '<button id=inv interestfor=pop style="interest-delay:800ms 800ms">i</button><div id=pop popover>p</div><button id=inv2>j</button><input id=ro readonly required>';
  await sleep(100);

  // Every member exercised below, captured before the patch.
  const Unit = CSSUnitValue, cssNumber = value => new Unit(value, 'number'), styleText = fn(CSSUnitValue.prototype, 'toString'), willValidate = get(HTMLInputElement.prototype, 'willValidate'), validity = fn(HTMLInputElement.prototype, 'checkValidity'),
    fn_T_add = fn(T, 'add'),
    fn_T_contains = fn(T, 'contains'),
    fn_T_forEach = fn(T, 'forEach'),
    fn_T_item = fn(T, 'item'),
    fn_T_remove = fn(T, 'remove'),
    fn_T_replace = fn(T, 'replace'),
    fn_T_supports = fn(T, 'supports'),
    fn_T_toString = fn(T, 'toString'),
    fn_T_toggle = fn(T, 'toggle'),
    get_E_ariaActiveDescendantElement = get(E, 'ariaActiveDescendantElement'),
    get_E_ariaDescribedByElements = get(E, 'ariaDescribedByElements'),
    get_E_ariaLabelledByElements = get(E, 'ariaLabelledByElements'),
    get_E_classList = get(E, 'classList'),
    get_H_focusGroup = get(H, 'focusGroup'),
    get_H_focusGroupStart = get(H, 'focusGroupStart'),
    get_HTMLButtonElement_interestForElement = get(HTMLButtonElement.prototype, 'interestForElement'),
    get_HTMLIFrameElement_sandbox = get(HTMLIFrameElement.prototype, 'sandbox'),
    get_HTMLTemplateElement_shadowRootReferenceTarget = get(HTMLTemplateElement.prototype, 'shadowRootReferenceTarget'),
    get_ShadowRoot_referenceTarget = get(ShadowRoot.prototype, 'referenceTarget'),
    get_T_value = get(T, 'value'),
    set_E_ariaActiveDescendantElement = set(E, 'ariaActiveDescendantElement'),
    set_E_ariaDescribedByElements = set(E, 'ariaDescribedByElements'),
    set_H_focusGroupStart = set(H, 'focusGroupStart'),
    set_HTMLButtonElement_interestForElement = set(HTMLButtonElement.prototype, 'interestForElement'),
    set_HTMLTemplateElement_shadowRootReferenceTarget = set(HTMLTemplateElement.prototype, 'shadowRootReferenceTarget'),
    set_ShadowRoot_referenceTarget = set(ShadowRoot.prototype, 'referenceTarget'),
    set_T_value = set(T, 'value');

  // Patch every configurable member of the builtins with a throwing function.
  const DRIVER_NEEDS = { __proto__: null, 'JSON.parse': 1, 'Array.prototype.@@Symbol(Symbol.iterator)': 1, 'Array.prototype.push': 1, 'Array.prototype.findIndex': 1, 'Array.prototype.splice': 1, 'Math.max': 1, 'Array.@@Symbol(Symbol.species)': 1 };
  const saved = []; let count = 0;
  const thrower = name => function () { throw new Err('patched ' + name); };
  const patchTargets = [[Array.prototype, 'Array.prototype'], [Array, 'Array'], [Map.prototype, 'Map.prototype'], [Set.prototype, 'Set.prototype'],
    [WeakMap.prototype, 'WeakMap.prototype'], [WeakSet.prototype, 'WeakSet.prototype'], [Object, 'Object'], [Object.prototype, 'Object.prototype'], [Reflect, 'Reflect'],
    [String, 'String'], [String.prototype, 'String.prototype'], [Function.prototype, 'Function.prototype'], [Promise, 'Promise'], [Promise.prototype, 'Promise.prototype'],
    [JSON, 'JSON'], [Symbol, 'Symbol'], [Number, 'Number'], [Number.prototype, 'Number.prototype'], [Math, 'Math'], [RegExp.prototype, 'RegExp.prototype'],
    [EventTarget.prototype, 'EventTarget.prototype'], [N, 'Node.prototype'], [E, 'Element.prototype'], [Document.prototype, 'Document.prototype'], [H, 'HTMLElement.prototype'],
    [CSSStyleDeclaration.prototype, 'CSSStyleDeclaration.prototype'], [ShadowRoot.prototype, 'ShadowRoot.prototype'], [RegExp, 'RegExp'], [Event.prototype, 'Event.prototype'],
    [KeyboardEvent.prototype, 'KeyboardEvent.prototype'], [MouseEvent.prototype, 'MouseEvent.prototype']];
  const skip = { __proto__: null, constructor: 1, prototype: 1, length: 1, name: 1, __proto__x: 1 };
  for (let t = 0; t < patchTargets.length; t++) {
    const o = patchTargets[t][0], label = patchTargets[t][1], ks = own(o);
    for (let i = 0; i < ks.length; i++) {
      const k = ks[i];
      if (typeof k === 'string' && (skip[k] || k === '__proto__')) continue;
      if (k === Symbol.hasInstance) continue;
      const d = gopd(o, k); if (!d.configurable) continue;
      const n = label + '.' + (typeof k === 'symbol' ? '@@' + String(k) : k);
      if (DRIVER_NEEDS[n]) continue;
      if (typeof d.value === 'function') { saved[saved.length] = [o, k, d]; count++; def(o, k, { value: thrower(n), writable: true, configurable: true, enumerable: d.enumerable }); }
      else if (d.get || d.set) { saved[saved.length] = [o, k, d]; count++; def(o, k, { get: thrower('get ' + n), set: thrower('set ' + n), configurable: true, enumerable: d.enumerable }); }
    }
  }
  const out = { __proto__: null };
  const attempt = async (name, f) => { try { out[name] = await f(); } catch (e) { out[name] = 'threw ' + (e && e.message); } };
  const host = q('host'), frame = q('frame'), tpl = q('tpl'), group = q('group'), inv = q('inv'), pop = q('pop');

  await attempt('relations', () => {
    A(setAttr, host, ['aria-labelledby', 'a b']);
    const labelled = ids(A(get_E_ariaLabelledByElements, host, []));
    A(set_E_ariaDescribedByElements, host, [[q('b'), q('a'), q('b')]]);
    const described = ids(A(get_E_ariaDescribedByElements, host, []));
    A(set_E_ariaActiveDescendantElement, host, [q('a')]);
    const activeId = A(idOf, A(get_E_ariaActiveDescendantElement, host, []), []);
    A(set_E_ariaDescribedByElements, host, [null]);
    return labelled + '|' + described + '|' + activeId + '|' + A(get_E_ariaDescribedByElements, host, []);
  });
  await attempt('tokens', () => {
    const list = A(get_E_classList, host, []);
    A(fn_T_add, list, ['x', 'y']); A(fn_T_remove, list, ['one']);
    const toggled = A(fn_T_toggle, list, ['z']) + ',' + A(fn_T_toggle, list, ['x', false]);
    const replaced = A(fn_T_replace, list, ['y', 'w']);
    let seen = ''; A(fn_T_forEach, list, [(value, index) => { seen += index + value; }]);
    const parts = [toggled, replaced, A(fn_T_contains, list, ['w']), A(fn_T_item, list, [1]), A(lengthOf, list, []), A(get_T_value, list, []), A(fn_T_toString, list, []), seen, list[0]];
    A(set_T_value, list, ['p q']);
    parts[parts.length] = A(get_T_value, list, []);
    const sandbox = A(get_HTMLIFrameElement_sandbox, frame, []);
    parts[parts.length] = A(fn_T_supports, sandbox, ['ALLOW-FORMS']) + ',' + A(fn_T_supports, sandbox, ['nope']);
    let text = ''; for (let index = 0; index < parts.length; index++) text += (index ? '|' : '') + parts[index];
    return text;
  });
  await attempt('focusGroupMembers', () => {
    A(set_H_focusGroupStart, group, [true]);
    const start = A(get_H_focusGroupStart, group, []);
    A(set_H_focusGroupStart, group, [false]);
    const list = A(get_H_focusGroup, group, []);
    return start + '|' + A(get_H_focusGroupStart, group, []) + '|' + A(fn_T_supports, list, ['Toolbar']) + '|' + A(fn_T_contains, list, ['toolbar']);
  });
  await attempt('referenceTarget', () => {
    const div = A(createElement, document, ['div']); A(attachShadow, div, [{ mode: 'open' }]);
    const root = A(shadowRootOf, div, []);
    A(set_ShadowRoot_referenceTarget, root, ['x']);
    const first = A(get_ShadowRoot_referenceTarget, root, []);
    A(set_HTMLTemplateElement_shadowRootReferenceTarget, tpl, ['y']);
    const second = A(get_HTMLTemplateElement_shadowRootReferenceTarget, tpl, []);
    A(set_HTMLTemplateElement_shadowRootReferenceTarget, tpl, [null]);
    return first + '|' + second + '|' + A(get_HTMLTemplateElement_shadowRootReferenceTarget, tpl, []);
  });
  await attempt('interestMembers', () => {
    const other = q('inv2');
    A(set_HTMLButtonElement_interestForElement, other, [pop]);
    const read = A(idOf, A(get_HTMLButtonElement_interestForElement, other, []), []);
    A(set_HTMLButtonElement_interestForElement, other, [null]);
    const event = new IE('interest', { source: inv, cancelable: true });
    return read + '|' + A(get_HTMLButtonElement_interestForElement, other, []) + '|' + A(idOf, A(sourceOf, event, []), []);
  });
  // The sweep: form validation and typed-OM number serialization use regular expressions.
  await attempt('sweep', () => {
    const ro = q('ro');
    return A(willValidate, ro, []) + '|' + A(validity, ro, []) + '|' + A(styleText, cssNumber(1.5), []) + '|' + A(styleText, cssNumber(1.0000051), []) + '|' + A(styleText, cssNumber(2.5e-7), []);
  });
  await attempt('focusGroupKeys', async () => {
    A(focus, q('f1'), []);
    await key('ArrowRight');
    const first = A(idOf, A(active, document, []), []);
    await key('Home');
    return first + '|' + A(idOf, A(active, document, []), []);
  });
  await attempt('interestHover', async () => {
    await hover(inv);
    await sleep(1200);
    const shown = A(matches, inv, [':interest-source']) + ',' + A(matches, pop, [':popover-open']);
    await key('Escape');
    await sleep(100);
    return shown + '|' + A(matches, inv, [':interest-source']) + ',' + A(matches, pop, [':popover-open']);
  });

  for (let i = saved.length - 1; i >= 0; i--) def(saved[i][0], saved[i][1], saved[i][2]);
  out.patched = count > 600;
  return out;
})()

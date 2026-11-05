(function () {
  'use strict';
  // The HTML interface members Chrome 156 has and the binding lacked (turbo-dom#323), measured
  // in interface_parity_chrome156.json: content attribute reflections (DOMString, URL, boolean,
  // enumerated, unsigned long, DOMTokenList), and the computed members beside them: a.text,
  // the window-reflecting body event handlers, style.disabled, legend.form, the
  // NonDocumentTypeChildNode members of CharacterData, getSVGDocument / the object element's
  // nested browsing context (the binding never loads an <object>), and the script element's
  // TrustedScript sinks textContent / innerText. Only a prototype without the member gets it,
  // so an embedder's own definition wins. fieldset.elements and datalist.options are live
  // collections and live in form_collections.js; the media members in html_media.js.
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction, accessorOf = ns.domAccessor;
  const apply = Reflect.apply, define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor;
  const hasOwn = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
  const ErrorType = TypeError, StringValue = String, NumberValue = Number;
  const E = g.Element.prototype, N = g.Node.prototype;
  const getAttribute = E.getAttribute, setAttribute = E.setAttribute, removeAttribute = E.removeAttribute, hasAttribute = E.hasAttribute;
  const resolve = ns.resolveDocumentURL;
  const charCodeAt = Function.prototype.call.bind(String.prototype.charCodeAt), fromCharCode = StringValue.fromCharCode;
  const lower = (value) => {
    let out = '';
    for (let index = 0; index < value.length; index++) {
      const code = charCodeAt(value, index);
      out += code >= 65 && code <= 90 ? fromCharCode(code + 32) : value[index];
    }
    return out;
  };
  const string = (value) => { if (typeof value === 'symbol') throw new ErrorType('Cannot convert a Symbol value to a string'); return StringValue(value); };
  const attr = (element, name) => apply(getAttribute, element, [name]);
  const set = (element, name, value) => apply(setAttribute, element, [name, value]);
  const remove = (element, name) => apply(removeAttribute, element, [name]);
  const checks = { __proto__: null };
  const brand = (tags) => checks[tags] || (checks[tags] = accessorOf('nodeType', false, 'HTML:' + tags));
  // The descriptor `name` resolves to on `object` (own or inherited).
  const find = (object, name) => { for (; object; object = Object.getPrototypeOf(object)) { const found = describe(object, name); if (found) return found; } return undefined; };
  const prototypeOf = (owner) => { const ctor = g[owner]; return typeof ctor === 'function' ? ctor.prototype : null; };

  // Setters Chrome 156 runs without an argument (as undefined); the others require one.
  const lenient = { __proto__: null };
  for (const key of ['HTMLAnchorElement.attributionSrc', 'HTMLAnchorElement.ping', 'HTMLAnchorElement.text', 'HTMLAreaElement.attributionSrc', 'HTMLAreaElement.ping', 'HTMLIFrameElement.longDesc', 'HTMLIFrameElement.privateToken', 'HTMLImageElement.attributionSrc', 'HTMLImageElement.hspace', 'HTMLImageElement.vspace', 'HTMLLinkElement.disabled', 'HTMLMediaElement.preload', 'HTMLObjectElement.hspace', 'HTMLObjectElement.vspace', 'HTMLScriptElement.attributionSrc', 'HTMLTemplateElement.shadowRootMode', 'HTMLTemplateElement.shadowRootSerializable', 'HTMLVideoElement.width', 'HTMLVideoElement.height']) lenient[key] = true;
  // One accessor on `owner`'s prototype, brand-checked against the HTML elements `tags`.
  function member(owner, name, tags, get, put) {
    const proto = prototypeOf(owner);
    if (!proto || hasOwn(proto, name)) return;
    const check = brand(tags), optional = !!lenient[owner + '.' + name];
    define(proto, name, {
      enumerable: true, configurable: true,
      get: make(function () { apply(check, this, []); return get(this); }, 'get ' + name, 0),
      set: put ? make(function (value) {
        apply(check, this, []);
        if (!arguments.length && !optional) throw new ErrorType("Failed to set the '" + name + "' property on '" + owner + "': 1 argument required, but only 0 present.");
        put(this, value);
      }, 'set ' + name, 1) : undefined,
    });
  }
  const content = (name) => name === 'htmlFor' ? 'for' : lower(name);

  // DOMString reflections; `nullable` ones are [LegacyNullToEmptyString].
  const text = (owner, tags, names, legacyNull) => {
    for (const name of names) {
      const attribute = content(name);
      member(owner, name, tags, (element) => { const value = attr(element, attribute); return value === null ? '' : value; },
        (element, value) => set(element, attribute, legacyNull && value === null ? '' : string(value)));
    }
  };
  // USVString URL reflections: the attribute resolved against the document base, "" without one.
  const url = (owner, tags, names) => {
    for (const name of names) {
      const attribute = content(name);
      member(owner, name, tags, (element) => { const value = attr(element, attribute); return value === null ? '' : resolve(element, value); },
        (element, value) => set(element, attribute, string(value)));
    }
  };
  const flag = (owner, tags, names) => {
    for (const name of names) {
      const attribute = content(name);
      member(owner, name, tags, (element) => apply(hasAttribute, element, [attribute]),
        (element, value) => { if (value) set(element, attribute, ''); else remove(element, attribute); });
    }
  };
  // Enumerated attributes: keywords (ASCII case-insensitive, `''` for the empty value's state),
  // the missing and invalid value defaults. `nullable` setters remove the attribute for null.
  const enumerated = (owner, tags, name, keywords, missing, invalid, nullable) => {
    const attribute = content(name);
    member(owner, name, tags, (element) => {
      const value = attr(element, attribute);
      if (value === null) return missing;
      const key = lower(value);
      return hasOwn(keywords, key) ? keywords[key] : invalid;
    }, (element, value) => { if (nullable && value == null) remove(element, attribute); else set(element, attribute, string(value)); });
  };
  const keywords = (...names) => { const map = { __proto__: null }; for (const name of names) map[name] = name; return map; };
  const crossOrigin = keywords('anonymous', 'use-credentials');
  crossOrigin[''] = 'anonymous';
  const referrerPolicy = keywords('', 'no-referrer', 'no-referrer-when-downgrade', 'same-origin', 'origin', 'strict-origin', 'origin-when-cross-origin', 'strict-origin-when-cross-origin', 'unsafe-url');
  const priority = keywords('high', 'low', 'auto'), loading = keywords('lazy', 'eager', 'auto');
  // unsigned long reflections (default 0): HTML's rules for parsing non-negative integers, kept
  // in [0, 2^31 - 1]; a larger value sets the default.
  const parseNonNegative = (value) => {
    let index = 0;
    while (index < value.length && (value[index] === ' ' || value[index] === '\t' || value[index] === '\n' || value[index] === '\f' || value[index] === '\r')) index++;
    if (value[index] === '+') index++;
    let digits = '';
    while (index < value.length && value[index] >= '0' && value[index] <= '9') digits += value[index++];
    if (!digits) return null;
    const number = NumberValue(digits);
    return number <= 2147483647 ? number : null;
  };
  const unsigned = (owner, tags, names) => {
    for (const name of names) {
      const attribute = content(name);
      member(owner, name, tags, (element) => { const value = attr(element, attribute); const parsed = value === null ? null : parseNonNegative(value); return parsed === null ? 0 : parsed; },
        (element, value) => { const number = NumberValue(value); const integer = number === number && number !== Infinity && number !== -Infinity ? number - number % 1 : 0; const modulo = ((integer % 4294967296) + 4294967296) % 4294967296; set(element, attribute, StringValue(modulo <= 2147483647 ? modulo : 0)); });
    }
  };
  const tokens = (owner, tags, name, attribute, supported) => {
    const proto = prototypeOf(owner);
    if (proto && !hasOwn(proto, name)) ns.defineTokenList(proto, name, attribute, brand(tags), supported);
  };

  // HTMLMediaElement / HTMLVideoElement reflections (state members: html_media.js).
  enumerated('HTMLMediaElement', 'audio|video', 'crossOrigin', crossOrigin, null, 'anonymous', true);
  const preload = keywords('none', 'metadata', 'auto');
  preload[''] = 'auto';
  enumerated('HTMLMediaElement', 'audio|video', 'preload', preload, 'metadata', 'metadata', false);
  tokens('HTMLMediaElement', 'audio|video', 'controlsList', 'controlslist', ['nodownload', 'nofullscreen', 'noplaybackrate', 'noremoteplayback']);
  unsigned('HTMLVideoElement', 'video', ['width', 'height']);
  url('HTMLVideoElement', 'video', ['poster']);
  flag('HTMLVideoElement', 'video', ['disablePictureInPicture']);
  // HTMLBodyElement: the legacy colors and background.
  text('HTMLBodyElement', 'body', ['text', 'link', 'vLink', 'aLink', 'bgColor'], true);
  text('HTMLBodyElement', 'body', ['background'], false);
  // HTMLIFrameElement.
  flag('HTMLIFrameElement', 'iframe', ['allowFullscreen', 'allowPaymentRequest', 'browsingTopics', 'adAuctionHeaders']);
  enumerated('HTMLIFrameElement', 'iframe', 'referrerPolicy', referrerPolicy, '', '', false);
  text('HTMLIFrameElement', 'iframe', ['csp', 'allow'], false);
  enumerated('HTMLIFrameElement', 'iframe', 'loading', loading, 'auto', 'auto', false);
  text('HTMLIFrameElement', 'iframe', ['align', 'scrolling', 'frameBorder', 'privateToken'], false);
  url('HTMLIFrameElement', 'iframe', ['longDesc']);
  text('HTMLIFrameElement', 'iframe', ['marginHeight', 'marginWidth'], true);
  // HTMLImageElement.
  text('HTMLImageElement', 'img', ['alt'], false);
  enumerated('HTMLImageElement', 'img', 'crossOrigin', crossOrigin, null, 'anonymous', true);
  text('HTMLImageElement', 'img', ['useMap'], false);
  flag('HTMLImageElement', 'img', ['isMap']);
  enumerated('HTMLImageElement', 'img', 'referrerPolicy', referrerPolicy, '', '', false);
  enumerated('HTMLImageElement', 'img', 'decoding', keywords('sync', 'async', 'auto'), 'auto', 'auto', false);
  enumerated('HTMLImageElement', 'img', 'fetchPriority', priority, 'auto', 'auto', false);
  enumerated('HTMLImageElement', 'img', 'loading', loading, 'auto', 'auto', false);
  url('HTMLImageElement', 'img', ['lowsrc']);
  text('HTMLImageElement', 'img', ['align'], false);
  unsigned('HTMLImageElement', 'img', ['hspace', 'vspace']);
  url('HTMLImageElement', 'img', ['longDesc']);
  text('HTMLImageElement', 'img', ['border'], true);
  flag('HTMLImageElement', 'img', ['browsingTopics']);
  text('HTMLImageElement', 'img', ['attributionSrc'], false);
  // HTMLObjectElement.
  text('HTMLObjectElement', 'object', ['useMap', 'width', 'height', 'align', 'archive', 'code'], false);
  flag('HTMLObjectElement', 'object', ['declare']);
  unsigned('HTMLObjectElement', 'object', ['hspace', 'vspace']);
  text('HTMLObjectElement', 'object', ['standby', 'codeType'], false);
  text('HTMLObjectElement', 'object', ['border'], true);
  // HTMLAnchorElement / HTMLAreaElement (the hyperlink members besides HTMLHyperlinkElementUtils).
  const links = ['noopener', 'noreferrer', 'opener'];
  text('HTMLAnchorElement', 'a', ['download', 'ping'], false);
  tokens('HTMLAnchorElement', 'a', 'relList', 'rel', links);
  enumerated('HTMLAnchorElement', 'a', 'referrerPolicy', referrerPolicy, '', '', false);
  member('HTMLAnchorElement', 'text', 'a', (element) => apply(textContent.get, element, []), (element, value) => apply(textContent.set, element, [string(value)]));
  text('HTMLAnchorElement', 'a', ['coords', 'charset', 'rev', 'shape', 'target', 'hreflang', 'type', 'attributionSrc', 'hrefTranslate'], false);
  text('HTMLAreaElement', 'area', ['alt', 'coords', 'shape', 'download', 'ping'], false);
  flag('HTMLAreaElement', 'area', ['noHref']);
  tokens('HTMLAreaElement', 'area', 'relList', 'rel', links);
  enumerated('HTMLAreaElement', 'area', 'referrerPolicy', referrerPolicy, '', '', false);
  text('HTMLAreaElement', 'area', ['target', 'attributionSrc'], false);
  // HTMLLinkElement. `disabled` reflects the attribute the style engine reads.
  flag('HTMLLinkElement', 'link', ['disabled']);
  enumerated('HTMLLinkElement', 'link', 'crossOrigin', crossOrigin, null, 'anonymous', true);
  tokens('HTMLLinkElement', 'link', 'relList', 'rel', ['alternate', 'apple-touch-icon', 'apple-touch-icon-precomposed', 'canonical', 'compression-dictionary', 'dns-prefetch', 'icon', 'manifest', 'modulepreload', 'next', 'preconnect', 'prefetch', 'preload', 'prerender', 'stylesheet']);
  text('HTMLLinkElement', 'link', ['hreflang'], false);
  enumerated('HTMLLinkElement', 'link', 'referrerPolicy', referrerPolicy, '', '', false);
  tokens('HTMLLinkElement', 'link', 'sizes', 'sizes', null);
  enumerated('HTMLLinkElement', 'link', 'fetchPriority', priority, 'auto', 'auto', false);
  text('HTMLLinkElement', 'link', ['imageSrcset', 'imageSizes', 'charset', 'rev', 'target'], false);
  const blocking = ['render'];
  tokens('HTMLLinkElement', 'link', 'blocking', 'blocking', blocking);
  // HTMLScriptElement.
  flag('HTMLScriptElement', 'script', ['noModule']);
  text('HTMLScriptElement', 'script', ['charset'], false);
  flag('HTMLScriptElement', 'script', ['defer']);
  enumerated('HTMLScriptElement', 'script', 'crossOrigin', crossOrigin, null, 'anonymous', true);
  enumerated('HTMLScriptElement', 'script', 'referrerPolicy', referrerPolicy, '', '', true);
  enumerated('HTMLScriptElement', 'script', 'fetchPriority', priority, 'auto', 'auto', false);
  text('HTMLScriptElement', 'script', ['event', 'htmlFor'], false);
  tokens('HTMLScriptElement', 'script', 'blocking', 'blocking', blocking);
  text('HTMLScriptElement', 'script', ['attributionSrc'], false);
  // The script element's own textContent / innerText (its TrustedScript sinks) run Node's and
  // HTMLElement's algorithms.
  const textContent = find(N, 'textContent'), innerText = find(g.HTMLElement.prototype, 'innerText');
  member('HTMLScriptElement', 'textContent', 'script', (element) => apply(textContent.get, element, []), (element, value) => apply(textContent.set, element, [value]));
  if (innerText) member('HTMLScriptElement', 'innerText', 'script', (element) => apply(innerText.get, element, []), (element, value) => apply(innerText.set, element, [value]));
  // HTMLTemplateElement: the declarative shadow root attributes.
  enumerated('HTMLTemplateElement', 'template', 'shadowRootMode', keywords('open', 'closed'), '', '', false);
  flag('HTMLTemplateElement', 'template', ['shadowRootDelegatesFocus', 'shadowRootClonable', 'shadowRootSerializable']);
  text('HTMLTemplateElement', 'template', ['shadowRootCustomElementRegistry'], false);
  enumerated('HTMLTemplateElement', 'template', 'shadowRootSlotAssignment', keywords('named', 'manual'), 'named', 'named', false);
  text('HTMLTemplateElement', 'template', ['htmlFor'], false);
  // HTMLFieldSetElement / HTMLLegendElement / HTMLOptGroupElement.
  flag('HTMLFieldSetElement', 'fieldset', ['disabled']);
  member('HTMLFieldSetElement', 'type', 'fieldset', () => 'fieldset');
  text('HTMLLegendElement', 'legend', ['align'], false);
  // legend.form: the form owner of a fieldset parent, else null.
  const parentElement = describe(N, 'parentElement').get, localName = describe(E, 'localName').get, namespaceURI = describe(E, 'namespaceURI').get;
  const formOf = find(g.HTMLFieldSetElement.prototype, 'form');
  member('HTMLLegendElement', 'form', 'legend', (element) => {
    const parent = apply(parentElement, element, []);
    return parent !== null && apply(namespaceURI, parent, []) === 'http://www.w3.org/1999/xhtml' && apply(localName, parent, []) === 'fieldset' ? apply(formOf.get, parent, []) : null;
  });
  flag('HTMLOptGroupElement', 'optgroup', ['disabled']);
  text('HTMLOptGroupElement', 'optgroup', ['label'], false);
  // HTMLStyleElement.disabled: its CSS style sheet's disabled flag (false without a sheet).
  const sheet = find(g.HTMLStyleElement.prototype, 'sheet'), sheetDisabled = find(g.CSSStyleSheet.prototype, 'disabled');
  member('HTMLStyleElement', 'disabled', 'style', (element) => { const value = apply(sheet.get, element, []); return value ? apply(sheetDisabled.get, value, []) : false; },
    (element, value) => { const current = apply(sheet.get, element, []); if (current) apply(sheetDisabled.set, current, [!!value]); });
  tokens('HTMLStyleElement', 'style', 'blocking', 'blocking', blocking);
  text('HTMLHtmlElement', 'html', ['version'], false);
  text('HTMLEmbedElement', 'embed', ['align', 'width', 'height'], false);
  text('HTMLDivElement', 'div', ['align'], false);
  // getSVGDocument(): the content document when it is an SVG document. The binding builds no
  // <object>/<embed> browsing context and an iframe's documents are HTML documents, so for each
  // it is null (Chrome's answer for every non-SVG frame); object.contentDocument /
  // contentWindow are null for the same reason.
  for (const [owner, tags] of [['HTMLIFrameElement', 'iframe'], ['HTMLObjectElement', 'object'], ['HTMLEmbedElement', 'embed']]) {
    const proto = prototypeOf(owner);
    if (!proto || hasOwn(proto, 'getSVGDocument')) continue;
    const frame = owner === 'HTMLIFrameElement' ? find(proto, 'contentDocument') : null;
    const check = brand(tags);
    const contentType = find(g.Document.prototype, 'contentType').get;
    define(proto, 'getSVGDocument', { value: make(function getSVGDocument() {
      apply(check, this, []);
      const document = frame && frame.get ? apply(frame.get, this, []) : null;
      return document && apply(contentType, document, []) === 'image/svg+xml' ? document : null;
    }, 'getSVGDocument', 0), writable: true, enumerable: true, configurable: true });
  }
  member('HTMLObjectElement', 'contentDocument', 'object', () => null);
  member('HTMLObjectElement', 'contentWindow', 'object', () => null);
  // CharacterData: NonDocumentTypeChildNode.
  const previousSibling = describe(N, 'previousSibling').get, nextSibling = describe(N, 'nextSibling').get, nodeType = describe(N, 'nodeType').get;
  const characterData = accessorOf('nodeType', false, 'CharacterData');
  for (const [name, step] of [['previousElementSibling', previousSibling], ['nextElementSibling', nextSibling]]) {
    if (hasOwn(g.CharacterData.prototype, name)) continue;
    define(g.CharacterData.prototype, name, { enumerable: true, configurable: true, get: make(function () {
      apply(characterData, this, []);
      let node = apply(step, this, []);
      while (node !== null && apply(nodeType, node, []) !== 1) node = apply(step, node, []);
      return node;
    }, 'get ' + name, 0) });
  }
  // Document.prototype.createEvent (turbo-dom#341): the DOM createEvent table plus the legacy
  // names Chrome 156 keeps, ASCII case-insensitive, returning an uninitialized event of that
  // interface; NotSupportedError for every other name. An interface the realm does not define
  // at install (an embedder's) is looked up when it is asked for.
  {
    const table = { __proto__: null, event: 'Event', events: 'Event', htmlevents: 'Event', svgevents: 'Event', uievent: 'UIEvent', uievents: 'UIEvent', mouseevent: 'MouseEvent', mouseevents: 'MouseEvent', keyboardevent: 'KeyboardEvent', keyboardevents: 'KeyboardEvent', focusevent: 'FocusEvent', compositionevent: 'CompositionEvent', textevent: 'TextEvent', dragevent: 'DragEvent', customevent: 'CustomEvent', messageevent: 'MessageEvent', hashchangeevent: 'HashChangeEvent', storageevent: 'StorageEvent', beforeunloadevent: 'BeforeUnloadEvent', devicemotionevent: 'DeviceMotionEvent', deviceorientationevent: 'DeviceOrientationEvent', transitionevent: 'TransitionEvent' };
    const captured = { __proto__: null }, construct = Reflect.construct, checkDocument = accessorOf('nodeType', false, 'Document');
    // Only a real Event interface: the binding's `instanceof` placeholders (browser_env.js
    // ctors) are not one.
    const EventPrototype = g.Event.prototype, isPrototypeOf = Function.prototype.call.bind(Object.prototype.isPrototypeOf);
    const eventInterface = (ctor) => typeof ctor === 'function' && (ctor === g.Event || isPrototypeOf(EventPrototype, ctor.prototype));
    for (const key in table) if (eventInterface(g[table[key]])) captured[table[key]] = g[table[key]];
    const failure = (message) => "Failed to execute 'createEvent' on 'Document': " + message;
    define(g.Document.prototype, 'createEvent', { value: make(function createEvent(type) {
      apply(checkDocument, this, []);
      if (!arguments.length) throw new ErrorType(failure('1 argument required, but only 0 present.'));
      if (typeof type === 'symbol') throw new ErrorType(failure('Cannot convert a Symbol value to a string'));
      type = StringValue(type);
      const name = table[lower(type)], ctor = name && (captured[name] || g[name]);
      if (!eventInterface(ctor)) throw new ns.DOMException(failure("The provided event type ('" + type + "') is invalid."), 'NotSupportedError');
      return construct(ctor, ['']);
    }, 'createEvent', 1), writable: true, enumerable: true, configurable: true });
  }
  // HTMLBodyElement's window-reflecting event handlers: they read and write the handlers of the
  // body's document's window, and are null / ignored without one. The Gamepad handlers stay
  // the body's own (Chrome's WindowEventHandlers partial interface on the element).
  const ownerDocument = find(N, 'ownerDocument').get, defaultView = find(g.Document.prototype, 'defaultView').get;
  const get = Reflect.get, put = Reflect.set;
  const view = (element) => { const document = apply(ownerDocument, element, []); return document ? apply(defaultView, document, []) : null; };
  for (const type of ['afterprint', 'beforeprint', 'beforeunload', 'blur', 'error', 'focus', 'hashchange', 'languagechange', 'load', 'message', 'messageerror', 'offline', 'online', 'pagehide', 'pageshow', 'popstate', 'rejectionhandled', 'resize', 'scroll', 'storage', 'unhandledrejection', 'unload']) {
    const name = 'on' + type;
    member('HTMLBodyElement', name, 'body', (element) => { const window = view(element); if (!window) return null; const value = get(window, name); return value === undefined ? null : value; },
      (element, value) => { const window = view(element); if (window) put(window, name, value); });
  }
  for (const type of ['gamepadconnected', 'gamepaddisconnected']) {
    const name = 'on' + type, handlers = new WeakMap(), mapGet = WeakMap.prototype.get, mapSet = WeakMap.prototype.set;
    const callback = make(function (event) { const handler = apply(mapGet, handlers, [this]); if (handler && apply(handler, this, [event]) === false) event.preventDefault(); }, name, 1);
    const add = g.EventTarget.prototype.addEventListener, drop = g.EventTarget.prototype.removeEventListener;
    member('HTMLBodyElement', name, 'body', (element) => apply(mapGet, handlers, [element]) || null, (element, value) => {
      const handler = typeof value === 'function' ? value : null, previous = apply(mapGet, handlers, [element]) || null;
      if (handler && !previous) apply(add, element, [type, callback]);
      if (!handler && previous) apply(drop, element, [type, callback]);
      apply(mapSet, handlers, [element, handler]);
    });
  }
})();

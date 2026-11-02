(async () => {
  const id = document.documentElement.dataset.case;
  const factory = trustedTypes;
  const error = (fn) => { try { const value = fn(); return { ok: true, value: value === undefined ? null : value }; } catch (e) { return { error: e.name, message: e.message }; } };
  const units = (s) => Array.from({ length: s.length }, (_, i) => s.charCodeAt(i));
  const key = (k) => typeof k === 'symbol' ? String(k) : k;
  const descriptor = (o, k) => { const d = Object.getOwnPropertyDescriptor(o, k); const row = { key: key(k), enumerable: d.enumerable, configurable: d.configurable }; if ('value' in d) { row.writable = d.writable; row.type = typeof d.value; if (typeof d.value === 'function') { row.name = d.value.name; row.length = d.value.length; } else if (d.value === null || ['string', 'number', 'boolean'].includes(typeof d.value)) row.value = d.value; } else { row.get = d.get ? [d.get.name, d.get.length] : null; row.set = d.set ? [d.set.name, d.set.length] : null; } return row; };
  const shape = (o) => Reflect.ownKeys(o).map((k) => descriptor(o, k));
  const rule = () => ({ createHTML: (s) => s, createScript: (s) => s, createScriptURL: (s) => s });
  const policy = (name = 'alpha', rules = rule()) => factory.createPolicy(name, rules);
  if (id === 'shape') return { global: descriptor(window, 'trustedTypes'), factory: shape(factory), interfaces: ['TrustedTypePolicyFactory', 'TrustedTypePolicy', 'TrustedHTML', 'TrustedScript', 'TrustedScriptURL'].map((name) => ({ name, global: descriptor(window, name), constructor: shape(window[name]), prototype: shape(window[name].prototype), construct: error(() => { new window[name](); return true; }) })) };
  if (id === 'values') {
    const p = policy(); const rows = [];
    for (const [method, is] of [['createHTML', 'isHTML'], ['createScript', 'isScript'], ['createScriptURL', 'isScriptURL']]) {
      const value = p[method]('\ud800X\udfff'); const proto = Object.getPrototypeOf(value);
      rows.push({ method, type: typeof value, keys: Reflect.ownKeys(value).map(key), tag: Object.prototype.toString.call(value), units: units(String(value)), json: JSON.stringify(value), is: factory[is](value), proxyIs: factory[is](new Proxy(value, {})), fakeIs: factory[is](Object.create(proto)), other: ['isHTML', 'isScript', 'isScriptURL'].map((name) => factory[name](value)), methods: ['toString', 'toJSON'].map((name) => ({ name, units: units(proto[name].call(value)), bad: error(() => proto[name].call({})) })) });
    }
    return { rows, emptyHTML: { tag: Object.prototype.toString.call(factory.emptyHTML), same: factory.emptyHTML === factory.emptyHTML, is: factory.isHTML(factory.emptyHTML), text: String(factory.emptyHTML) }, emptyScript: { tag: Object.prototype.toString.call(factory.emptyScript), same: factory.emptyScript === factory.emptyScript, is: factory.isScript(factory.emptyScript), text: String(factory.emptyScript) }, defaultPolicy: factory.defaultPolicy, policyName: p.name, policyShape: shape(p) };
  }
  if (id === 'callbacks') {
    const trace = []; const options = {};
    for (const name of ['createHTML', 'createScript', 'createScriptURL']) Object.defineProperty(options, name, { get() { trace.push('get:' + name); return function (...args) { trace.push([name, this === undefined ? 'undefined' : this === options ? 'options' : typeof this, args]); return { toString() { trace.push('resultToString'); return '\ud800X\udfff'; } }; }; } });
    const p = policy({ toString() { trace.push('name'); return 'alpha'; } }, options); const value = p.createHTML({ toString() { trace.push('input'); return 'input'; } }, 7, { x: 1 });
    return { trace, units: units(String(value)), name: p.name };
  }
  if (id === 'missing-rules') {
    const noOptions = error(() => factory.createPolicy('no-options').name); const p = factory.createPolicy('alpha', {}); const trace = [];
    return { noOptions, methods: ['createHTML', 'createScript', 'createScriptURL'].map((name) => ({ name, noArgument: error(() => p[name]()), input: error(() => p[name]({ toString() { trace.push(name); return 'x'; } })) })), trace, policyWithoutName: error(() => factory.createPolicy()) };
  }
  if (id === 'option-conversion') return [null, undefined, 1, 'x', false, Symbol('x')].map((options, index) => ({ type: options === null ? 'null' : typeof options, result: error(() => { const p = factory.createPolicy('p' + index, options); return p.name; }) })).concat([null, undefined, 1, 'x', false].map((callback, index) => ({ callbackType: callback === null ? 'null' : typeof callback, result: error(() => { const p = factory.createPolicy('c' + index, { createHTML: callback }); return String(p.createHTML('x')); }) })));
  if (id === 'mapping') {
    const html = 'http://www.w3.org/1999/xhtml', svg = 'http://www.w3.org/2000/svg', xlink = 'http://www.w3.org/1999/xlink';
    const attributes = [['script', 'src'], ['SCRIPT', 'SRC'], ['iframe', 'srcdoc'], ['div', 'onclick'], ['DIV', 'ONLOAD'], ['script', 'text'], ['script', 'href', svg, xlink], ['script', 'href', svg], ['img', 'src'], ['a', 'href'], ['*', 'onclick'], ['svg', 'onload', svg], ['div', 'onclick', html, 'urn:other'], ['script', 'src', null], ['script', 'src', '']];
    const properties = [['script', 'text'], ['script', 'textContent'], ['script', 'innerText'], ['script', 'src'], ['iframe', 'srcdoc'], ['div', 'innerHTML'], ['div', 'outerHTML'], ['*', 'innerHTML'], ['SCRIPT', 'TEXT'], ['div', 'textContent'], ['svg', 'innerHTML', svg], ['script', 'href', svg]];
    return { attributes: attributes.map((args) => [args, factory.getAttributeType(...args)]), properties: properties.map((args) => [args, factory.getPropertyType(...args)]), missing: ['getAttributeType', 'getPropertyType'].map((name) => [name, error(() => factory[name]())]), typeMapping: typeof factory.getTypeMapping === 'function' ? factory.getTypeMapping() : null };
  }
  if (id === 'brands') {
    const p = policy(); const value = p.createHTML('x');
    return { factoryMethods: ['createPolicy', 'isHTML', 'isScript', 'isScriptURL', 'getPropertyType', 'getAttributeType'].map((name) => [name, error(() => factory[name].call({}, 'x', {}))]), policyMethods: ['createHTML', 'createScript', 'createScriptURL'].map((name) => [name, error(() => p[name].call({}, 'x'))]), defaultGetter: error(() => Object.getOwnPropertyDescriptor(Object.getPrototypeOf(factory), 'defaultPolicy').get.call({})), nameGetter: error(() => Object.getOwnPropertyDescriptor(Object.getPrototypeOf(p), 'name').get.call({})), symbolInput: error(() => p.createHTML(Symbol('x'))), htmlToScript: factory.isScript(value) };
  }
  if (id === 'cross-realm') {
    const frame = document.createElement('iframe'); const loaded = new Promise((resolve) => frame.onload = resolve); document.body.append(frame); await loaded;
    const child = frame.contentWindow; const childPolicy = child.trustedTypes.createPolicy('alpha', { createHTML: (s) => s }); const foreign = childPolicy.createHTML('foreign'); const own = policy().createHTML('own');
    const result = { parentAccepts: factory.isHTML(foreign), childAccepts: child.trustedTypes.isHTML(own), parentInstance: foreign instanceof TrustedHTML, childInstance: foreign instanceof child.TrustedHTML, borrowed: TrustedHTML.prototype.toString.call(foreign) }; frame.remove(); result.afterRemoval = String(foreign); return result;
  }
  if (id === 'edge-cases') {
    const names = ['alpha', 'alpha', '', 'with space', "bad'name", 'with:colon', 'default', 'default'];
    const rows = names.map(name => [name, error(() => policy(name).name)]);
    const returns = [null, undefined, 1, false, Symbol('x')].map((v, i) => { const p = policy('return' + i, { createHTML: () => v, createScriptURL: () => v }); return [typeof v, error(() => String(p.createHTML('x'))), error(() => String(p.createScriptURL('x')))]; });
    let receiver; const p = policy('receiver', { createHTML: function() { 'use strict'; receiver = this; return 'x'; } }); p.createHTML('x');
    return { rows, returns, callbackReceiver: receiver === undefined ? 'undefined' : receiver === null ? 'null' : receiver === p ? 'policy' : receiver === window ? 'global' : Object.prototype.toString.call(receiver), mappings: [null,'','http://www.w3.org/1999/xhtml','http://www.w3.org/2000/svg','urn:unknown'].map(ns => [ns, factory.getTypeMapping(ns)]), symbolNames: error(() => factory.createPolicy(Symbol('x'))), getMissingOne: error(() => factory.getPropertyType('script')), propertyCase: [['SCRIPT','text'],['script','Text'],['script','src','urn:unknown']].map(args=>[args,factory.getPropertyType(...args)]) };
  }
  if (id.startsWith('policy-csp-')) {
    const violations = []; document.addEventListener('securitypolicyviolation', (e) => violations.push({ effectiveDirective: e.effectiveDirective, blockedURI: e.blockedURI, disposition: e.disposition, sample: e.sample }));
    const rows = ['alpha', 'other', 'alpha', 'default', 'default', ''].map((name) => ({ name, result: error(() => policy(name).name) }));
    await new Promise((resolve) => setTimeout(resolve, 50)); return { rows, defaultName: factory.defaultPolicy?.name ?? null, violations };
  }
  if (id === 'default-sinks') {
    const trace = []; const p = policy('default', { createHTML(value, ...args) { trace.push(['html', value, args]); return '<b>' + value + '</b>'; }, createScript(value, ...args) { trace.push(['script', value, args]); return value; }, createScriptURL(value, ...args) { trace.push(['url', value, args]); return value; } });
    const div = document.createElement('div'); div.innerHTML = 'X'; const script = document.createElement('script'); script.text = 'void 0'; script.src = '/script.js';
    return { trace, html: div.innerHTML, text: script.text, src: script.getAttribute('src'), defaultSame: factory.defaultPolicy === p };
  }
  throw new Error('Unknown authored case ' + id);
})()

(async () => {
  'use strict';
  // turbo-surf#32: HTMLImageElement's image-state members are prototype accessors, never own
  // properties of an <img>, for parser-inserted and script-created images alike. Load state
  // (natural size, completeness, current URL) is the embedder's: the binding reports Chrome's
  // values for images without a source and for images that are not loading (broken); a
  // loaded image's natural size comes from the host's image-state hook.
  const names = ['complete', 'currentSrc', 'naturalHeight', 'naturalWidth'];
  const P = HTMLImageElement.prototype;
  const out = {};
  const error = (f) => { try { return ['ok', f()]; } catch (e) { return ['throw', e.name, e.message]; } };
  const shape = (key) => {
    const d = Object.getOwnPropertyDescriptor(P, key);
    if (!d) return null;
    return {
      keys: Object.keys(d), enumerable: d.enumerable, configurable: d.configurable,
      get: d.get && [d.get.name, d.get.length, Function.prototype.toString.call(d.get), Object.hasOwn(d.get, 'prototype')],
      set: d.set === undefined ? 'undefined' : typeof d.set,
    };
  };
  out.shape = Object.fromEntries(names.map((name) => [name, shape(name)]));
  out.inPrototype = names.map((name) => name in P);
  out.inElement = names.map((name) => name in HTMLElement.prototype);
  out.prototypeRead = names.map((name) => error(() => P[name]));
  const svg = 'http://www.w3.org/2000/svg';
  const receivers = {
    plain: {}, inherited: Object.create(P), div: document.createElement('div'), input: document.createElement('input'),
    svgImage: document.createElementNS(svg, 'image'), foreignImg: document.createElementNS('urn:x', 'img'), document,
  };
  out.receivers = Object.fromEntries(Object.entries(receivers).map(([label, receiver]) =>
    [label, names.map((name) => error(() => Object.getOwnPropertyDescriptor(P, name).get.call(receiver)))]));
  out.nullReceiver = error(() => Object.getOwnPropertyDescriptor(P, 'naturalWidth').get.call(null));
  const settle = (img) => img.complete ? null : new Promise((done) => {
    img.addEventListener('load', () => done(), { once: true });
    img.addEventListener('error', () => done(), { once: true });
  });
  const parsed = ['nosrc', 'empty', 'loaded', 'broken'].map((id) => document.getElementById(id));
  await Promise.all(parsed.map(settle));
  const view = (img) => ({
    complete: img.complete, naturalWidth: img.naturalWidth, naturalHeight: img.naturalHeight, currentSrc: img.currentSrc,
    own: Object.getOwnPropertyNames(img), ownState: names.filter((name) => Object.hasOwn(img, name)),
    enumerated: (() => { const seen = []; for (const key in img) if (names.includes(key)) seen.push(key); return seen.sort(); })(),
  });
  out.parsed = Object.fromEntries(parsed.map((img) => [img.id, view(img)]));
  out.created = view(document.createElement('img'));
  out.createdNS = view(document.createElementNS('http://www.w3.org/1999/xhtml', 'img'));
  const host = document.createElement('div');
  host.innerHTML = '<img alt="x">';
  out.innerHTML = view(host.firstChild);
  out.assignStrict = error(() => { const img = document.createElement('img'); img.naturalWidth = 5; return img.naturalWidth; });
  out.assignSloppy = Function('const img = document.createElement("img"); img.complete = false; img.currentSrc = "x"; return [img.complete, img.currentSrc, Object.getOwnPropertyNames(img)];')();
  out.defineOwn = (() => {
    const img = document.createElement('img');
    Object.defineProperty(img, 'naturalWidth', { value: 7, configurable: true });
    const shadowed = img.naturalWidth;
    delete img.naturalWidth;
    return [shadowed, img.naturalWidth];
  })();
  // turbo-dom#23: the legacy factory function Image(width, height) (HTML 4.8.4.1, Web IDL
  // [LegacyFactoryFunction]): new-only, length 0, shares HTMLImageElement.prototype, honours
  // NewTarget's prototype; width / height (unsigned long) set the content attributes.
  const fshape = (object, key) => {
    const d = Object.getOwnPropertyDescriptor(object, key);
    if (!d) return null;
    const row = { enumerable: d.enumerable, configurable: d.configurable };
    if ('value' in d) row.writable = d.writable;
    if ('get' in d) row.get = typeof d.get;
    if ('set' in d) row.set = typeof d.set;
    return row;
  };
  const made = (i) => ({
    proto: Object.getPrototypeOf(i) === P, tag: i.localName, ns: i.namespaceURI, owner: i.ownerDocument === document,
    parent: i.parentNode, widthAttr: i.getAttribute('width'),
    heightAttr: i.getAttribute('height'), attrs: i.attributes.length, src: i.src, complete: i.complete,
    own: Object.getOwnPropertyNames(i), keys: Object.keys(i), symbols: Object.getOwnPropertySymbols(i).length, html: i.outerHTML,
  });
  const f = {};
  f.type = typeof Image;
  f.name = Image.name;
  f.length = Image.length;
  f.ownKeys = Reflect.ownKeys(Image).map(String);
  f.nameShape = fshape(Image, 'name');
  f.lengthShape = fshape(Image, 'length');
  f.prototypeShape = fshape(Image, 'prototype');
  f.globalShape = fshape(globalThis, 'Image');
  f.sameProto = Image.prototype === HTMLImageElement.prototype;
  f.protoConstructor = Image.prototype.constructor === HTMLImageElement;
  f.notInterface = Image !== HTMLImageElement;
  f.functionProto = Object.getPrototypeOf(Image) === Function.prototype;
  f.source = Function.prototype.toString.call(Image);
  f.call = error(() => Image());
  f.callArgs = error(() => Image(1, 2));
  f.instance = new Image() instanceof HTMLImageElement;
  f.distinct = new Image() !== new Image();
  f.empty = made(new Image());
  f.width = made(new Image(5));
  f.both = made(new Image(5, 7));
  f.undefinedWidth = made(new Image(undefined, 3));
  f.undefinedOnly = made(new Image(undefined));
  f.undefinedBoth = made(new Image(undefined, undefined));
  f.nulls = made(new Image(null, null));
  f.range = made(new Image(-1, 2147483648));
  f.max = made(new Image(2147483647, 4294967295));
  f.fractions = made(new Image(1.9, -1));
  f.strings = made(new Image('12', 'x'));
  f.wrap = made(new Image(2 ** 32 + 5, NaN));
  f.extra = made(new Image(1, 2, 3));
  f.symbol = error(() => new Image(Symbol('s')));
  f.symbolHeight = error(() => new Image(1, Symbol('s')));
  f.throwingValueOf = error(() => new Image({ valueOf() { throw new RangeError('w'); } }));
  f.bigint = error(() => new Image(1n));
  class Sub extends Image {}
  const sub = new Sub(4);
  f.subclass = [sub instanceof Sub, Object.getPrototypeOf(sub) === Sub.prototype, sub.localName, sub.getAttribute('width')];
  const target = function () {};
  target.prototype = { marker: true };
  f.reflect = Object.getPrototypeOf(Reflect.construct(Image, [], target)) === target.prototype;
  const primitive = function () {};
  primitive.prototype = 5;
  const proto = Object.getPrototypeOf(Reflect.construct(Image, [], primitive));
  f.reflectPrimitive = proto === P ? 'HTMLImageElement' : proto === HTMLElement.prototype ? 'HTMLElement' : proto === Object.prototype ? 'Object' : proto === Function.prototype ? 'Function' : String(proto && proto.constructor && proto.constructor.name);
  out.factory = f;
  // turbo-dom#23: an image never carries own properties — src, decode and the event
  // handlers live on the prototypes, for factory-made, created and parsed images alike.
  const owned = (i) => [Object.getOwnPropertyNames(i), Object.keys(i)];
  const o = {};
  const factorySrc = new Image();
  factorySrc.src = 'http://localhost/a.png';
  o.factorySrc = [factorySrc.getAttribute('src'), factorySrc.src, ...owned(factorySrc)];
  factorySrc.onload = () => {};
  o.factoryOnload = [typeof factorySrc.onload, ...owned(factorySrc)];
  const createdSrc = document.createElement('img');
  createdSrc.src = 'http://localhost/b.png';
  o.createdSrc = [createdSrc.getAttribute('src'), createdSrc.src, ...owned(createdSrc)];
  o.parsed = owned(document.getElementById('loaded'));
  o.srcShape = fshape(P, 'src');
  o.decode = [typeof P.decode, Object.hasOwn(P, 'decode'), Object.hasOwn(factorySrc, 'decode'), P.decode && P.decode.length];
  o.onload = [Object.hasOwn(P, 'onload'), 'onload' in factorySrc, Object.hasOwn(factorySrc, 'onload')];
  o.patchedMarker = ['__tcImagePatched' in factorySrc, '__tcImagePatched' in createdSrc];
  out.own = o;
  // decode() is a prototype operation; its promise follows the image's load state.
  const decoded = (promise) => promise.then(() => 'resolved', (e) => ['rejected', e.name, e.message]);
  const d = {};
  d.shape = fshape(P, 'decode');
  d.fn = [P.decode.name, P.decode.length, Function.prototype.toString.call(P.decode), Object.hasOwn(P.decode, 'prototype')];
  d.parsed = Object.fromEntries(await Promise.all(parsed.map(async (img) => [img.id, await decoded(img.decode())])));
  d.factory = await decoded(new Image().decode());
  d.receiver = await decoded((async () => P.decode.call(document.createElement('div')))());
  d.returnsPromise = document.createElement('img').decode() instanceof Promise;
  out.decode = d;
  return out;
})();

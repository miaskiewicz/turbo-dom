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
  return out;
})();

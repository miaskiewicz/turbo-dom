(() => {
  'use strict';
  // turbo-dom#71: HTMLImageElement srcset / sizes and HTMLSourceElement srcset / sizes / media /
  // type / width / height reflect their content attributes; an image's source is selected from
  // its <picture> sources (media, type), srcset (x and w descriptors) and sizes (media
  // conditions) and reported by currentSrc, with the natural size of the chosen candidate
  // divided by its density. Run after the page's load event (html_image_srcset_cases.json).
  const out = {};
  const shape = (proto, key) => {
    const d = Object.getOwnPropertyDescriptor(proto, key);
    if (!d) return null;
    return {
      keys: Object.keys(d), enumerable: d.enumerable, configurable: d.configurable,
      get: d.get && [d.get.name, d.get.length],
      set: d.set && [d.set.name, d.set.length],
    };
  };
  const image = HTMLImageElement.prototype, source = HTMLSourceElement.prototype;
  out.shape = {
    image: Object.fromEntries(['srcset', 'sizes'].map((key) => [key, shape(image, key)])),
    source: Object.fromEntries(['src', 'srcset', 'sizes', 'media', 'type', 'width', 'height'].map((key) => [key, shape(source, key)])),
    elsewhere: ['srcset', 'sizes'].map((key) => key in HTMLElement.prototype),
  };
  const attempt = (f) => { try { return ['ok', f()]; } catch (e) { return ['throw', e.name]; } };
  const strings = ['x', '', null, undefined, 12, ' a b '];
  const reflect = (create, name) => {
    const rows = [];
    const el = create();
    rows.push(el[name]);
    for (const value of strings) {
      el[name] = value;
      rows.push([el[name], el.getAttribute(name)]);
    }
    el.setAttribute(name, 'from-attribute');
    rows.push(el[name]);
    el.removeAttribute(name);
    rows.push(el[name]);
    return rows;
  };
  const img = () => document.createElement('img'), src = () => document.createElement('source');
  out.reflect = {
    imageSrcset: reflect(img, 'srcset'),
    imageSizes: reflect(img, 'sizes'),
    sourceSrcset: reflect(src, 'srcset'),
    sourceSizes: reflect(src, 'sizes'),
    sourceMedia: reflect(src, 'media'),
    sourceType: reflect(src, 'type'),
  };
  const numbers = ['12', '12px', ' 7', '-1', '-0', '+5', '2147483647', '2147483648', '4294967296', 'x', ''];
  const setters = [5, -1, 2147483647, 2147483648, 4294967295, 1.9, '7', null, NaN];
  for (const name of ['width', 'height']) {
    out[name] = {
      initial: src()[name],
      attributes: numbers.map((value) => { const el = src(); el.setAttribute(name, value); return el[name]; }),
      setters: setters.map((value) => { const el = src(); const r = attempt(() => { el[name] = value; }); return [r[0], el.getAttribute(name), el[name]]; }),
      symbol: attempt(() => { src()[name] = Symbol(); }),
    };
  }
  out.receivers = ['srcset', 'sizes'].map((key) => attempt(() => Object.getOwnPropertyDescriptor(image, key).get.call(document.createElement('div'))));
  const origin = location.origin;
  const strip = (url) => url.startsWith(origin) ? url.slice(origin.length) : url;
  out.images = {};
  for (const el of document.querySelectorAll('body img[id]')) {
    out.images[el.id] = [strip(el.currentSrc), el.complete, el.naturalWidth, el.naturalHeight];
  }
  return out;
})()

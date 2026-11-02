(() => {
  'use strict';
  // Small interface members: HTMLMediaElement constants and loading, HTMLScriptElement.supports,
  // document.plugins and HTMLCanvasElement.toBlob arity.
  const run = (f) => { try { return ['ok', f()]; } catch (error) { return ['throw', error.name]; } };
  const fn = (f) => (typeof f === 'function' ? { name: f.name, length: f.length } : typeof f);
  const desc = (o, k) => {
    const d = Object.getOwnPropertyDescriptor(o, k);
    if (!d) return null;
    const r = { c: d.configurable, e: d.enumerable };
    if ('value' in d) { r.w = d.writable; r.v = typeof d.value === 'function' ? fn(d.value) : d.value !== null && typeof d.value === 'object' ? 'object' : d.value; } else { r.get = fn(d.get); r.set = fn(d.set); }
    return r;
  };
  const media = ['NETWORK_EMPTY', 'NETWORK_IDLE', 'NETWORK_LOADING', 'NETWORK_NO_SOURCE', 'HAVE_NOTHING', 'HAVE_METADATA', 'HAVE_CURRENT_DATA', 'HAVE_FUTURE_DATA', 'HAVE_ENOUGH_DATA'];
  const out = {};
  out.mediaConstants = media.map((k) => [desc(HTMLMediaElement, k), desc(HTMLMediaElement.prototype, k), HTMLVideoElement[k], document.createElement('audio')[k]]);
  out.mediaLoadingDesc = desc(HTMLMediaElement.prototype, 'loading');
  const video = document.createElement('video');
  const steps = [];
  steps.push([video.loading, video.getAttribute('loading')]);
  for (const value of ['lazy', 'LAZY', 'eager', 'bogus', '', null, 7]) {
    video.loading = value;
    steps.push([video.loading, video.getAttribute('loading')]);
  }
  video.setAttribute('loading', ' lazy');
  steps.push([video.loading]);
  video.removeAttribute('loading');
  steps.push([video.loading]);
  out.mediaLoading = steps;
  out.mediaLoadingBrand = run(() => Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'loading').get.call(document.createElement('img')));
  out.scriptSupports = [desc(HTMLScriptElement, 'supports'), ...['classic', 'module', 'importmap', 'speculationrules', 'webbundle', 'text/javascript', 'MODULE', '', 'json'].map((t) => HTMLScriptElement.supports(t)), run(() => HTMLScriptElement.supports())[0], run(() => HTMLScriptElement.supports.call(null, 'module'))];
  out.plugins = [desc(Document.prototype, 'plugins'), document.plugins === document.embeds, Object.prototype.toString.call(document.plugins)];
  out.toBlob = desc(HTMLCanvasElement.prototype, 'toBlob');
  // Document.prototype.fragmentDirective / FragmentDirective.
  const shapeOf = (C) => typeof C === 'function' ? { ctor: Object.fromEntries(Reflect.ownKeys(C).map((k) => [String(k), desc(C, k)])), proto: Object.fromEntries(Reflect.ownKeys(C.prototype).map((k) => [String(k), desc(C.prototype, k)])), parent: Object.getPrototypeOf(C) === Function.prototype, protoParent: Object.getPrototypeOf(C.prototype) === Object.prototype, construct: run(() => new C())[1], global: desc(globalThis, C.name) } : typeof C;
  out.fragmentDirective = [desc(Document.prototype, 'fragmentDirective'), shapeOf(globalThis.FragmentDirective), Object.prototype.toString.call(document.fragmentDirective), document.fragmentDirective === document.fragmentDirective, Reflect.ownKeys(document.fragmentDirective).length, run(() => Object.getOwnPropertyDescriptor(Document.prototype, 'fragmentDirective').get.call(document.body))[0], document.implementation.createHTMLDocument('').fragmentDirective === document.fragmentDirective];
  // Element.prototype.pseudo() / CSSPseudoElement.
  out.pseudoShape = [desc(Element.prototype, 'pseudo'), shapeOf(globalThis.CSSPseudoElement)];
  const host = document.createElement('div');
  document.body.append(host);
  const view = (p) => p === null ? null : [p.type, p.element === host, p.parent === host, p === host.pseudo(p.type)];
  const types = ['::before', '::after', '::marker', '::backdrop', '::first-line', '::first-letter', '::selection', '::placeholder', '::file-selector-button', '::view-transition', '::view-transition-group(x)', '::view-transition-group(*)', '::view-transition-image-pair(x)', '::view-transition-old(x)', '::view-transition-new(x)', '::view-transition-group()', '::column', '::scroll-marker', '::scroll-marker-group', '::scroll-button(left)', '::picker(select)', '::checkmark', '::picker-icon', '::details-content', '::part(x)', '::slotted(x)', ':before', ':after', ':marker', ':first-line', 'before', '::BEFORE', '::Marker', ' ::before', '::before ', '', '::', '::bogus', '::cue', '::highlight(x)', '::search-text', '::target-text', '::spelling-error', '::before::marker', null, 7];
  out.pseudoTypes = types.map((type) => [String(type), run(() => { const p = host.pseudo(type); return p === null ? null : [p.type, p.element === host, p.parent === host, p === host.pseudo(type)]; })]);
  const group = host.pseudo('::view-transition-group(x)');
  const pair = host.pseudo('::view-transition-image-pair(x)');
  out.pseudoGroup = group && [group.parent === host.pseudo('::view-transition'), group === host.pseudo('::view-transition-group(x)'), group === host.pseudo('::view-transition-group(y)'), host.pseudo('::view-transition-group(y)').type, pair.parent === group, host.pseudo('::view-transition-old(x)').parent === pair, host.pseudo('::view-transition-new(x)').parent === pair, host.pseudo('::view-transition-group(X)') === group, ['a-b', '_a', '-a', '--a', '1a', 'a b', 'a.b', '\\31 a', 'é'].map((name) => host.pseudo('::view-transition-group(' + name + ')') !== null), group.pseudo('::view-transition') === null, host.pseudo('::view-transition').pseudo('::view-transition-group(x)') === null];
  const before = host.pseudo('::before');
  out.pseudoNested = ['::marker', '::before', '::after', '::backdrop', ':marker', '::MARKER'].map((type) => [type, run(() => { const p = before.pseudo(type); return p === null ? null : [p.type, p.element === host, p.parent === before, p === before.pseudo(type)]; })]);
  out.pseudoMarkerNested = run(() => host.pseudo('::marker').pseudo('::marker') === null);
  out.pseudoMisc = [run(() => host.pseudo())[1], run(() => Element.prototype.pseudo.call({}, '::before'))[1], document.createElement('span').pseudo('::after') !== null, document.createElementNS('http://www.w3.org/2000/svg', 'g').pseudo('::before') !== null, Object.getOwnPropertyNames(before), Object.keys(before), before instanceof EventTarget, run(() => Object.getOwnPropertyDescriptor(CSSPseudoElement.prototype, 'type').get.call(host))[1], run(() => CSSPseudoElement.prototype.pseudo.call(host, '::marker'))[1]];
  host.remove();
  out.pseudoDetached = view(host.pseudo('::before'));
  return JSON.parse(JSON.stringify(out));
})()

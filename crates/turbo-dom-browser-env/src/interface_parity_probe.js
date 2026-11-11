(async () => {
  'use strict';
  // turbo-dom#323: the interface members Chrome 156 has on the HTML / DOM interfaces the binding
  // defines: their presence and descriptors, the reflections' content attribute behaviour, and
  // the computed and media members' behaviour. URLs are reported relative to the document base.
  const out = {};
  const base = (() => { const a = document.createElement('a'); a.href = 'x'; return a.href.endsWith('/x') ? a.href.slice(0, -1) : '\u0000'; })();
  const show = (v) => typeof v === 'string' && v.startsWith(base) ? '<base>' + v.slice(base.length) : v;
  const fn = (f) => (typeof f === 'function' ? [f.name, f.length] : typeof f);
  const desc = (o, k) => {
    const d = o && Object.getOwnPropertyDescriptor(o, k);
    if (!d) return null;
    return 'value' in d ? ['value', typeof d.value === 'function' ? fn(d.value) : d.value !== null && typeof d.value === 'object' ? tag(d.value) : d.value, d.writable, d.enumerable, d.configurable] : ['accessor', fn(d.get), fn(d.set), d.enumerable, d.configurable];
  };
  const run = (f) => { try { return f(); } catch (e) { return 'throw ' + e.name + ': ' + e.message; } };
  const tag = (o) => Object.prototype.toString.call(o);
  // Each member of the issue's list, by interface.
  const members = {
    HTMLMediaElement: 'error currentSrc crossOrigin networkState preload buffered readyState seeking currentTime duration paused defaultPlaybackRate playbackRate played seekable ended preservesPitch controlsList volume muted textTracks webkitAudioDecodedByteCount webkitVideoDecodedByteCount onencrypted onwaitingforkey srcObject addTextTrack load pause play sinkId remote setSinkId mediaKeys setMediaKeys',
    HTMLBodyElement: 'text link vLink aLink bgColor background onafterprint onbeforeprint onbeforeunload onblur onerror onfocus ongamepadconnected ongamepaddisconnected onhashchange onlanguagechange onload onmessage onmessageerror onoffline ononline onpagehide onpageshow onpopstate onrejectionhandled onresize onscroll onstorage onunhandledrejection onunload',
    HTMLIFrameElement: 'allowFullscreen referrerPolicy csp allow loading align scrolling frameBorder longDesc marginHeight marginWidth getSVGDocument allowPaymentRequest privateToken browsingTopics adAuctionHeaders',
    HTMLImageElement: 'alt crossOrigin useMap isMap referrerPolicy decoding fetchPriority loading lowsrc align hspace vspace longDesc border browsingTopics attributionSrc',
    HTMLObjectElement: 'useMap width height contentDocument contentWindow align archive code declare hspace standby vspace codeType border getSVGDocument',
    HTMLEmbedElement: 'align width height getSVGDocument',
    HTMLVideoElement: 'width height videoWidth videoHeight poster webkitDecodedFrameCount webkitDroppedFrameCount onenterpictureinpicture onleavepictureinpicture disablePictureInPicture cancelVideoFrameCallback getVideoPlaybackQuality requestPictureInPicture requestVideoFrameCallback',
    HTMLAnchorElement: 'download ping relList referrerPolicy text coords charset rev shape target hreflang type attributionSrc hrefTranslate',
    HTMLAreaElement: 'alt coords download noHref ping referrerPolicy relList shape target attributionSrc',
    HTMLLinkElement: 'disabled crossOrigin relList hreflang referrerPolicy sizes fetchPriority imageSrcset imageSizes charset rev target blocking',
    HTMLScriptElement: 'noModule charset defer crossOrigin referrerPolicy fetchPriority event htmlFor blocking textContent innerText attributionSrc',
    HTMLTemplateElement: 'shadowRootMode shadowRootDelegatesFocus shadowRootClonable shadowRootSerializable shadowRootCustomElementRegistry shadowRootSlotAssignment htmlFor',
    HTMLFieldSetElement: 'disabled type elements',
    HTMLLegendElement: 'form align',
    HTMLOptGroupElement: 'disabled label',
    HTMLStyleElement: 'disabled blocking',
    CharacterData: 'previousElementSibling nextElementSibling',
    HTMLDataListElement: 'options',
    HTMLHtmlElement: 'version',
    HTMLDivElement: 'align',
  };
  out.members = {};
  for (const [name, list] of Object.entries(members)) for (const k of list.split(' ')) out.members[name + '.' + k] = desc(globalThis[name].prototype, k);
  // The interfaces behind the media members.
  const probeInterface = (name) => {
    const C = globalThis[name];
    if (typeof C !== 'function') return typeof C;
    return {
      proto: Object.fromEntries(Reflect.ownKeys(C.prototype).map((k) => [String(k), desc(C.prototype, k)])),
      ctor: Object.fromEntries(Reflect.ownKeys(C).map((k) => [String(k), desc(C, k)])),
      parent: Object.getPrototypeOf(C.prototype) === Object.prototype ? 'Object' : Object.getPrototypeOf(C.prototype).constructor.name,
      construct: run(() => new C()),
      global: desc(globalThis, name),
    };
  };
  out.interfaces = {};
  for (const name of ['TimeRanges', 'MediaError', 'TextTrackList', 'VideoPlaybackQuality', 'RemotePlayback']) {
    out.interfaces[name] = probeInterface(name);
  }

  // Content attribute reflections: the default, the IDL value per attribute value, the IDL value
  // and attribute per assignment, and the receiver check.
  const reflected = {
    HTMLMediaElement: ['video', 'crossOrigin preload controlsList'],
    HTMLBodyElement: ['body', 'text link vLink aLink bgColor background'],
    HTMLIFrameElement: ['iframe', 'allowFullscreen referrerPolicy csp allow loading align scrolling frameBorder longDesc marginHeight marginWidth allowPaymentRequest privateToken browsingTopics adAuctionHeaders'],
    HTMLImageElement: ['img', 'alt crossOrigin useMap isMap referrerPolicy decoding fetchPriority loading lowsrc align hspace vspace longDesc border browsingTopics attributionSrc'],
    HTMLObjectElement: ['object', 'useMap width height align archive code declare hspace standby vspace codeType border'],
    HTMLEmbedElement: ['embed', 'align width height'],
    HTMLVideoElement: ['video', 'width height poster disablePictureInPicture'],
    HTMLAnchorElement: ['a', 'download ping relList referrerPolicy text coords charset rev shape target hreflang type attributionSrc hrefTranslate'],
    HTMLAreaElement: ['area', 'alt coords download noHref ping referrerPolicy relList shape target attributionSrc'],
    HTMLLinkElement: ['link', 'disabled crossOrigin relList hreflang referrerPolicy sizes fetchPriority imageSrcset imageSizes charset rev target blocking'],
    HTMLScriptElement: ['script', 'noModule charset defer crossOrigin referrerPolicy fetchPriority event htmlFor blocking attributionSrc'],
    HTMLTemplateElement: ['template', 'shadowRootMode shadowRootDelegatesFocus shadowRootClonable shadowRootSerializable shadowRootCustomElementRegistry shadowRootSlotAssignment htmlFor'],
    HTMLFieldSetElement: ['fieldset', 'disabled'],
    HTMLLegendElement: ['legend', 'align'],
    HTMLOptGroupElement: ['optgroup', 'disabled label'],
    HTMLStyleElement: ['style', 'blocking'],
    HTMLHtmlElement: ['html', 'version'],
    HTMLDivElement: ['div', 'align'],
  };
  const read = (el, k) => { const v = el[k]; return v !== null && typeof v === 'object' ? (v instanceof DOMTokenList ? 'T:' + v.value : tag(v)) : show(v); };
  const attrValues = ['', 'x', 'LAZY', 'lazy', 'eager', 'auto', ' 12 ', '+3', '-5', '0', '4294967296', 'use-credentials', 'Anonymous', 'No-Referrer', 'unsafe-url', 'never', 'HIGH', 'low', 'Open', 'closed', 'manual', 'sync', 'ASYNC', 'none', 'metadata', 'a/b c'];
  const idlValues = ['abc', 7, -1, 0, null, undefined, true, false, 'Use-Credentials', '', 'rel/p', 'closed', 'LOW', 2147483648, 1.9];
  const attrForProp = (k) => k === 'htmlFor' ? 'for' : k === 'relList' ? 'rel' : k.toLowerCase();
  const probeReflected = (iface, name, k) => {
    const attr = attrForProp(k);
    const fresh = () => document.createElement(name);
    return {
      def: run(() => read(fresh(), k)),
      attr: attrValues.map((v) => run(() => { const el = fresh(); el.setAttribute(attr, v); return read(el, k); })),
      idl: idlValues.map((v) => run(() => { const el = fresh(); el[k] = v; return [read(el, k), el.getAttribute(attr)]; })),
      brand: run(() => Object.getOwnPropertyDescriptor(globalThis[iface].prototype, k).get.call(document.createElement('span'))),
      missing: run(() => Object.getOwnPropertyDescriptor(globalThis[iface].prototype, k).set.call(fresh())),
    };
  };
  out.reflected = {};
  for (const [iface, [name, list]] of Object.entries(reflected)) {
    for (const k of list.split(' ')) {
      out.reflected[iface + '.' + k] = probeReflected(iface, name, k);
    }
  }
  // DOMTokenList supported tokens.
  const candidates = ['alternate', 'author', 'canonical', 'compression-dictionary', 'dns-prefetch', 'expect', 'help', 'icon', 'license', 'manifest', 'modulepreload', 'next', 'nofollow', 'noopener', 'noreferrer', 'opener', 'pingback', 'preconnect', 'prefetch', 'preload', 'prerender', 'prev', 'search', 'stylesheet', 'apple-touch-icon', 'apple-touch-icon-precomposed', 'ICON', 'render', 'RENDER', 'nodownload', 'NoFullscreen', 'noplaybackrate', 'noremoteplayback', 'any', ''];
  const supported = (list) => run(() => candidates.filter((c) => list.supports(c)));
  out.tokens = {
    link: supported(document.createElement('link').relList), a: supported(document.createElement('a').relList), area: supported(document.createElement('area').relList),
    sizes: supported(document.createElement('link').sizes), blocking: ['link', 'script', 'style'].map((t) => supported(document.createElement(t).blocking)),
    controls: supported(document.createElement('audio').controlsList),
    same: (() => { const l = document.createElement('link'); return [l.relList === l.relList, l.sizes === l.sizes, tag(l.relList)]; })(),
  };
  { const l = document.createElement('link'); l.relList.add('icon', 'preload'); l.sizes = '16x16 32x32'; out.tokens.put = [l.getAttribute('rel'), l.getAttribute('sizes'), l.sizes.length, l.sizes[1]]; }

  // Computed members.
  const fs = document.createElement('fieldset');
  fs.innerHTML = '<legend>L</legend><input><input type=image><button></button><div><select></select><textarea></textarea><output></output><object></object><fieldset><input></fieldset><img><label></label><progress></progress><meter></meter><option></option></div>';
  const els = fs.elements;
  out.fieldset = [fs.type, tag(els), els === fs.elements, els.length, Array.from(els, (e) => e.localName).join(' ')];
  fs.append(document.createElement('input'));
  out.fieldset.push(els.length, run(() => Object.getOwnPropertyDescriptor(HTMLFieldSetElement.prototype, 'elements').get.call(document.createElement('div'))));
  { const form = document.createElement('form'); form.innerHTML = '<fieldset><legend id=l>x</legend></fieldset><legend id=l2></legend>'; out.legend = [form.querySelector('#l').form === form, form.querySelector('#l2').form, document.createElement('legend').form]; }
  { const form = document.createElement('form'); form.id = 'ff'; document.body.append(form); const set = document.createElement('fieldset'); set.setAttribute('form', 'ff'); const legend = document.createElement('legend'); set.append(legend); document.body.append(set); out.legend.push(legend.form === form, set.form === form); form.remove(); set.remove(); }
  { const list = document.createElement('datalist'); list.innerHTML = '<option>a<div><option>b</div><optgroup><option>c</optgroup>'; const options = list.options; out.datalist = [tag(options), options === list.options, options.length]; list.append(document.createElement('option')); out.datalist.push(options.length); }
  { const style = document.createElement('style'); out.style = [style.disabled]; style.disabled = true; out.style.push(style.disabled, style.hasAttribute('disabled')); document.head.append(style); out.style.push(style.disabled, style.sheet.disabled); style.disabled = true; out.style.push(style.disabled, style.sheet.disabled, style.hasAttribute('disabled')); style.sheet.disabled = false; out.style.push(style.disabled); style.remove(); }
  { const p = document.createElement('p'); p.innerHTML = '<b></b>t<!--c--><i></i>'; const t = p.childNodes[1], c = p.childNodes[2]; out.characterData = [t.previousElementSibling.localName, t.nextElementSibling.localName, c.previousElementSibling.localName, c.nextElementSibling.localName, document.createTextNode('x').nextElementSibling, run(() => Object.getOwnPropertyDescriptor(CharacterData.prototype, 'nextElementSibling').get.call(document.body))]; }
  {
    const body = document.body, handler = () => 1;
    out.body = [];
    for (const k of ['onload', 'onblur', 'onerror', 'onfocus', 'onresize', 'onscroll', 'onhashchange', 'onmessage', 'onpopstate', 'onstorage', 'onunload', 'onbeforeunload', 'onlanguagechange', 'ongamepadconnected']) { body[k] = handler; out.body.push([k, body[k] === handler, window[k] === handler]); body[k] = null; out.body.push(body[k]); }
    const inert = document.implementation.createHTMLDocument('').body; inert.onload = handler; out.body.push(inert.onload, window.onload, document.createElement('span').onload);
  }
  { const frame = document.createElement('iframe'), object = document.createElement('object'); out.frames = [frame.getSVGDocument(), object.getSVGDocument(), object.contentDocument, object.contentWindow, document.createElement('embed').getSVGDocument(), run(() => HTMLIFrameElement.prototype.getSVGDocument.call(object))]; document.body.append(frame); out.frames.push(frame.getSVGDocument(), !!frame.contentDocument); frame.remove(); }
  { const a = document.createElement('a'); a.innerHTML = 'x<b>y</b>'; out.anchorText = [a.text]; a.text = 'z'; out.anchorText.push(a.innerHTML); a.text = null; out.anchorText.push(a.innerHTML); }
  { const script = document.createElement('script'); script.textContent = 'a'; out.script = [script.text, script.innerText, script.textContent]; script.innerText = 'b\nc'; out.script.push(script.innerHTML, script.textContent); script.textContent = null; out.script.push(script.innerHTML); }

  // Media: a FIFO of settled timers stands for "later".
  const tick = () => new Promise((r) => setTimeout(r, 30));
  const types = 'abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange seeked seeking stalled suspend timeupdate volumechange waiting waitingforkey resize'.split(' ');
  const watch = (el, log) => { for (const t of types) el.addEventListener(t, (e) => log.push(t + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '') + ':' + el.networkState + el.readyState + (el.paused ? 'p' : 'r'))); };
  const state = (v) => ({ ns: v.networkState, rs: v.readyState, err: v.error && [v.error.code, v.error.message, tag(v.error)], src: show(v.currentSrc), paused: v.paused, t: v.currentTime, d: String(v.duration), seeking: v.seeking, ended: v.ended, rate: [v.playbackRate, v.defaultPlaybackRate], vol: [v.volume, v.muted], pp: v.preservesPitch, ranges: [v.buffered.length, v.played.length, v.seekable.length, v.buffered === v.buffered, tag(v.buffered)], tt: [v.textTracks.length, v.textTracks === v.textTracks, tag(v.textTracks)], bytes: [v.webkitAudioDecodedByteCount, v.webkitVideoDecodedByteCount], so: v.srcObject, mk: v.mediaKeys, sink: v.sinkId, video: [v.videoWidth, v.videoHeight, v.webkitDecodedFrameCount, v.webkitDroppedFrameCount] });
  const settle = (p) => { const r = { s: 'pending' }; p.then((v) => { r.s = 'ok:' + v; }, (e) => { r.s = 'err:' + e.name + ':' + e.message; }); return r; };
  const media = {};
  out.media = media;
  { const v = document.createElement('video'), log = []; watch(v, log); media.fresh = state(v); const p = settle(v.play()); media.blocked = [state(v)]; await tick(); media.blocked.push(state(v), p.s, log.slice()); v.pause(); media.blocked.push(v.networkState); await tick(); v.load(); await tick(); media.blocked.push(log.slice(), state(v)); }
  { const v = document.createElement('video'), log = []; watch(v, log); v.muted = true; const p = settle(v.play()); media.muted = [[v.paused, v.networkState, log.slice()]]; await tick(); media.muted.push([v.paused, v.networkState, p.s, log.slice()]); const p2 = settle(v.play()); v.pause(); media.muted.push([v.paused, v.networkState]); await tick(); media.muted.push([p.s, p2.s, log.slice(), v.networkState]); const p3 = settle(v.play()); await tick(); v.load(); media.muted.push([v.paused, v.networkState]); await tick(); media.muted.push([p3.s, log.slice(), v.paused]); }
  { const v = document.createElement('video'), log = []; watch(v, log); v.currentTime = 5; media.time = [v.currentTime]; v.currentTime = -3; media.time.push(v.currentTime, run(() => { v.currentTime = NaN; })); v.volume = 0.5; media.time.push(v.volume, run(() => { v.volume = 2; }), run(() => { v.volume = -Infinity; }), run(() => { v.volume = -0.1; })); v.playbackRate = 2; v.defaultPlaybackRate = 0.5; v.preservesPitch = false; media.time.push(v.playbackRate, v.defaultPlaybackRate, v.preservesPitch, run(() => { v.playbackRate = 100; }), run(() => { v.playbackRate = -1; }), run(() => { v.playbackRate = 0; return v.playbackRate; }), run(() => { v.defaultPlaybackRate = 100; return v.defaultPlaybackRate; })); v.playbackRate = 2; v.muted = true; v.volume = 0.5; await tick(); media.time.push(log.slice()); v.load(); media.time.push(v.playbackRate, v.currentTime, v.muted, v.volume); await tick(); media.time.push(log.slice()); }
  { const v = document.createElement('video'), log = []; watch(v, log); v.src = ''; media.emptySrc = [state(v)]; await tick(); media.emptySrc.push(state(v), log.slice()); v.muted = true; const p = settle(v.play()); await tick(); media.emptySrc.push(p.s, log.slice()); v.removeAttribute('src'); await tick(); media.emptySrc.push(state(v), log.slice()); }
  { const v = document.createElement('video'); v.src = 'clip.webm'; media.src = [v.networkState, show(v.currentSrc)]; await Promise.resolve(); await Promise.resolve(); media.src.push(show(v.currentSrc)); }
  { const host = document.createElement('div'); host.innerHTML = '<audio><source type="video/x-bogus" src="a.bogus"><source src=""></audio>'; const v = host.firstChild, log = [], sourceLog = []; await tick(); watch(v, log); for (const s of v.children) s.addEventListener('error', (e) => sourceLog.push('error' + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '') + ':' + v.networkState)); media.sources = [state(v)]; v.load(); media.sources.push(v.networkState); await tick(); media.sources.push(state(v), log.slice(), sourceLog.slice()); }
  { const v = document.createElement('video'), t = v.addTextTrack('captions', 'L', 'en'); media.tracks = [t.kind, t.label, t.language, t.mode, t.id, t.cues.length, v.textTracks.length, v.textTracks[0] === t, v.textTracks.getTrackById(''), run(() => v.addTextTrack('bogus')), run(() => v.addTextTrack())]; const track = document.createElement('track'); track.id = 'tr'; v.append(track); media.tracks.push(v.textTracks.length, v.textTracks[0] === track.track, v.textTracks[1] === t, v.textTracks.getTrackById('tr') === track.track, Object.keys(v.textTracks).join()); track.remove(); media.tracks.push(v.textTracks.length, v.addTextTrack('metadata').label, [...v.textTracks].length); }
  { const v = document.createElement('video'), ranges = v.buffered; media.ranges = [ranges.length, run(() => ranges.start(0)), run(() => ranges.end(1)), run(() => ranges.start()), run(() => TimeRanges.prototype.start.call({}, 0))]; }
  {
    const v = document.createElement('video'), q = v.getVideoPlaybackQuality();
    media.quality = [tag(q), typeof q.creationTime, q.totalVideoFrames, q.droppedVideoFrames, q.corruptedVideoFrames, v.getVideoPlaybackQuality() === q];
    const pip = settle(v.requestPictureInPicture());
    const w = document.createElement('video'); w.disablePictureInPicture = true; const pip2 = settle(w.requestPictureInPicture());
    const id = v.requestVideoFrameCallback(() => {});
    media.frames = [typeof id, v.cancelVideoFrameCallback(id), run(() => v.requestVideoFrameCallback(1)), run(() => v.cancelVideoFrameCallback())];
    const s1 = settle(v.setSinkId('')), m1 = settle(v.setMediaKeys(null)), m2 = settle(v.setMediaKeys({}));
    const r = v.remote;
    media.remote = [tag(r), r.state, r === v.remote, r instanceof EventTarget, r.onconnect];
    const w1 = settle(r.watchAvailability(() => {})), w2 = settle(r.watchAvailability(() => {})), c1 = settle(r.cancelWatchAvailability(12345)), pr = settle(r.prompt());
    media.remote.push(run(() => r.watchAvailability(1)));
    await tick();
    media.async = [pip.s, pip2.s, s1.s, v.sinkId, m1.s, m2.s, w2.s.slice(0, 3), c1.s, pr.s];
    v.disableRemotePlayback = true; const w3 = settle(r.watchAvailability(() => {})); await tick(); media.async.push(w3.s, w1.s.slice(0, 3));
  }
  { const v = document.createElement('video'); media.handlers = ['onencrypted', 'onwaitingforkey', 'onenterpictureinpicture', 'onleavepictureinpicture'].map((k) => v[k]); let n = 0; v.onencrypted = () => { n++; return false; }; const e = new Event('encrypted', { cancelable: true }); v.dispatchEvent(e); media.handlers.push(n, e.defaultPrevented); v.onencrypted = 5; media.handlers.push(v.onencrypted); }
  { const v = document.createElement('video'); media.srcObject = [run(() => { v.srcObject = {}; }), run(() => { v.srcObject = null; return v.srcObject; })]; }
  { const a = document.createElement('audio'); media.brand = [run(() => a.videoWidth), run(() => HTMLMediaElement.prototype.play.call(document.createElement('div'))), run(() => Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'paused').get.call({})), run(() => HTMLVideoElement.prototype.getVideoPlaybackQuality.call(a))]; }
  await new Promise((resolve) => { globalThis.__cdpGestureDone = resolve; __cdpGesture('click'); });
  { const v = document.createElement('video'), log = []; watch(v, log); const p = settle(v.play()); media.activated = [v.paused]; await tick(); media.activated.push(v.paused, p.s, log.slice()); const pr = settle(v.remote.prompt()); await tick(); media.activated.push(pr.s); }
  // Platform objects report their class.
  return JSON.parse(JSON.stringify(out, (_key, value) => value !== null && typeof value === 'object' && !Array.isArray(value) && Object.getPrototypeOf(value) !== Object.prototype ? tag(value) : value));
})()

(async () => {
  'use strict';
  // turbo-dom#339: HTMLMediaElement.captureStream() and MediaStream, resource selection restarted
  // by <source> insertion, TextTrackList's addtrack / removetrack / change events (TrackEvent),
  // automatic text track selection and the track processing model of a track without a src.
  // Nothing here waits on the network: a typeless <source> is read when its loadstart fires.
  const out = {};
  const run = (f) => { try { return f(); } catch (e) { return 'throw ' + e.name + ': ' + e.message; } };
  const tag = (o) => Object.prototype.toString.call(o);
  const fn = (f) => (typeof f === 'function' ? [f.name, f.length] : typeof f);
  const desc = (o, k) => { const d = o && Object.getOwnPropertyDescriptor(o, k); if (!d) return null; return 'value' in d ? ['value', typeof d.value === 'function' ? fn(d.value) : d.value !== null && typeof d.value === 'object' ? tag(d.value) : d.value, d.writable, d.enumerable, d.configurable] : ['accessor', fn(d.get), fn(d.set), d.enumerable, d.configurable]; };
  const tick = () => new Promise((r) => setTimeout(r, 30));
  const settle = async () => { await tick(); await tick(); };
  const outcome = (p) => { const r = { s: 'pending' }; p.then((v) => { r.s = 'ok:' + v; }, (e) => { r.s = 'err:' + e.name + ':' + e.message; }); return r; };
  const base = (u) => u.replace(/^.*\//, '');

  const probeInterface = (name) => {
    const C = globalThis[name];
    return typeof C !== 'function' ? typeof C : {
      proto: Object.fromEntries(Reflect.ownKeys(C.prototype).map((k) => [String(k), desc(C.prototype, k)])),
      ctor: Object.fromEntries(Reflect.ownKeys(C).map((k) => [String(k), desc(C, k)])),
      parent: Object.getPrototypeOf(C.prototype).constructor.name,
      call: run(() => C('x')),
      global: desc(globalThis, name),
    };
  };
  out.interfaces = {};
  for (const name of ['MediaStream', 'MediaStreamTrack', 'TrackEvent']) {
    out.interfaces[name] = probeInterface(name);
  }
  out.captureStreamMember = desc(HTMLMediaElement.prototype, 'captureStream');

  // ---- MediaStream ----
  const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
  {
    const v = document.createElement('video'), s = v.captureStream();
    const shape = (m) => [tag(m), uuid.test(m.id), m.active, m.getTracks().length, m.getAudioTracks().length, m.getVideoTracks().length, Array.isArray(m.getTracks()), m.getTracks() === m.getTracks()];
    out.stream = [shape(s), v.captureStream() === s, v.captureStream().id === s.id, shape(document.createElement('audio').captureStream()), run(() => v.captureStream(1, 2).active)];
    out.stream.push(s.getTrackById('x'), s.getTrackById(5), run(() => s.getTrackById()), run(() => s.addTrack({})), run(() => s.addTrack(null)), run(() => s.addTrack()), run(() => s.removeTrack(null)), run(() => s.removeTrack()));
    const c = s.clone();
    out.stream.push(shape(c), c.id === s.id, c instanceof MediaStream, c instanceof EventTarget);
    out.stream.push(shape(new MediaStream()), shape(new MediaStream([])), shape(new MediaStream(new Set())), shape(new MediaStream(s)), new MediaStream(s).id === s.id);
    out.stream.push(run(() => new MediaStream(undefined)), run(() => new MediaStream(null)), run(() => new MediaStream({})), run(() => new MediaStream('abc')), run(() => new MediaStream(5)), run(() => new MediaStream([1])), run(() => new MediaStream([s])), run(() => new MediaStream({ [Symbol.iterator]() { throw new Error('boom'); } })));
    out.stream.push(run(() => HTMLMediaElement.prototype.captureStream.call(document.createElement('div'))), run(() => Object.getOwnPropertyDescriptor(MediaStream.prototype, 'id').get.call({})), run(() => MediaStream.prototype.getTracks.call(new EventTarget())), run(() => MediaStream.prototype.clone.call(v)));
    const ids = new Set();
    for (let i = 0; i < 16; i++) ids.add(new MediaStream().id);
    out.stream.push(ids.size, [...ids].every((id) => uuid.test(id)));
    const handler = () => 1, object = {};
    out.stream.push(['onaddtrack', 'onremovetrack', 'onactive', 'oninactive'].map((k) => s[k]));
    s.onactive = handler; out.stream.push(s.onactive === handler); s.onactive = object; out.stream.push(s.onactive === object); s.onactive = 5; out.stream.push(s.onactive);
    let n = 0; s.onaddtrack = () => { n++; }; s.dispatchEvent(new Event('addtrack')); out.stream.push(n);
    const p = outcome(MediaStreamTrack.prototype.applyConstraints.call({}));
    out.stream.push(run(() => Object.getOwnPropertyDescriptor(MediaStreamTrack.prototype, 'kind').get.call({})), run(() => MediaStreamTrack.prototype.stop.call(s)), run(() => MediaStreamTrack.prototype.getSettings.call({})), run(() => Object.getOwnPropertyDescriptor(MediaStreamTrack.prototype, 'enabled').set.call({}, true)), run(() => Object.getOwnPropertyDescriptor(MediaStreamTrack.prototype, 'onended').get.call({})));
    await tick();
    out.stream.push(p.s);
  }

  // ---- srcObject = MediaStream ----
  const types = 'abort canplay emptied ended error loadeddata loadedmetadata loadstart pause play playing ratechange suspend stalled timeupdate volumechange waiting'.split(' ');
  const watch = (v, log) => { for (const t of types) v.addEventListener(t, () => log.push(t + ':' + v.networkState + v.readyState + (v.paused ? 'p' : 'r'))); };
  {
    const v = document.createElement('video'), log = [], s = new MediaStream();
    watch(v, log);
    v.srcObject = s;
    out.srcObject = [v.srcObject === s, v.networkState, v.currentSrc];
    await tick();
    out.srcObject.push(log.slice(), v.networkState, v.readyState, String(v.duration), v.error, v.currentSrc);
    v.muted = true;
    const p = outcome(v.play());
    out.srcObject.push(v.paused);
    await tick();
    out.srcObject.push(p.s, log.slice());
    const c = v.captureStream();
    out.srcObject.push(c === s, c.id === s.id, c.active);
    v.pause();
    await tick();
    out.srcObject.push(p.s, log.slice());
    v.srcObject = null;
    out.srcObject.push(v.networkState, v.srcObject);
    await tick();
    out.srcObject.push(log.slice(), v.networkState);
    v.src = '';
    await tick();
    v.srcObject = s;
    await tick();
    out.srcObject.push(v.networkState, v.error, v.currentSrc, log.slice());
    v.srcObject = s;
    await tick();
    v.load();
    await tick();
    out.srcObject.push(log.slice(), v.srcObject === s);
    const w = document.createElement('video'), wlog = [];
    watch(w, wlog);
    w.srcObject = s;
    const q = outcome(w.play());
    await tick();
    out.srcObject.push(q.s, wlog.slice(), w.paused);
    const x = document.createElement('video'), xlog = [];
    watch(x, xlog);
    x.srcObject = s;
    const source = document.createElement('source');
    source.type = 'video/x-bogus'; source.src = 'a';
    source.addEventListener('error', () => xlog.push('source error'));
    x.append(source);
    await tick();
    out.srcObject.push(xlog.slice(), x.networkState);
    out.srcObject.push(run(() => { v.srcObject = 5; }), run(() => { v.srcObject = {}; }), run(() => { v.srcObject = new EventTarget(); }));
  }

  // ---- <source> insertion ----
  const sources = out.sources = {};
  const make = (log, id, type, url = id) => {
    const s = document.createElement('source');
    s.id = id;
    if (url !== null) s.src = url;
    if (type) s.type = type;
    s.addEventListener('error', (e) => log.push('error@' + id + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '')));
    return s;
  };
  const bogus = 'video/x-bogus';
  // A typeless source would be fetched: its scenario ends when loadstart fires.
  const fetching = (v, log) => new Promise((resolve) => v.addEventListener('loadstart', () => { if (v.networkState === 2) resolve([log.slice(), base(v.currentSrc)]); }));
  const scenario = async (name, body) => {
    const log = [], v = document.createElement('video');
    watch(v, log);
    const r = [];
    await body(v, (id, type, url) => make(log, id, type, url), r, log);
    if (!r.done) { await tick(); r.push(log.slice(), v.networkState, base(v.currentSrc), v.error && v.error.code); }
    sources[name] = r;
  };
  await scenario('bogus', (v, S, r) => { v.append(S('a', bogus)); r.push(v.networkState); });
  await scenario('noSrc', (v, S, r) => { v.append(S('a', null, null)); r.push(v.networkState); });
  await scenario('emptySrc', (v, S, r) => { v.append(S('a', null, '')); r.push(v.networkState); });
  await scenario('typeless', async (v, S, r, log) => { const loading = fetching(v, log); v.append(S('a')); r.push(v.networkState); r.push(await loading); r.done = true; });
  await scenario('two', async (v, S, r, log) => { const loading = fetching(v, log); v.append(S('a', bogus), S('b')); r.push(v.networkState); r.push(await loading); r.done = true; });
  await scenario('fragment', (v, S, r) => { const f = document.createDocumentFragment(); f.append(S('a', bogus), S('b', bogus)); v.append(f); r.push(v.networkState); });
  await scenario('waiting', async (v, S, r, log) => {
    v.append(S('a', bogus)); await tick(); r.push(v.networkState, log.slice());
    v.append(S('b', bogus)); r.push(v.networkState); await tick(); r.push(v.networkState, log.slice());
    const loading = fetching(v, log); v.append(S('c')); r.push(v.networkState); r.push(await loading); r.done = true;
  });
  await scenario('waitingBefore', async (v, S, r, log) => { const a = S('a', bogus); v.append(a); await tick(); const loading = fetching(v, log); v.insertBefore(S('b'), a); r.push(v.networkState); r.push(await loading); r.done = true; });
  await scenario('waitingTwice', async (v, S, r) => { v.append(S('a', bogus)); await tick(); v.append(S('b', bogus)); v.prepend(S('c', bogus)); r.push(v.networkState); });
  await scenario('pending', (v, S, r) => { v.append(S('a', bogus)); v.append(S('b', bogus)); r.push(v.networkState); });
  await scenario('loadPending', async (v, S, r) => { v.append(S('a', bogus)); await tick(); v.load(); v.append(S('b', bogus)); r.push(v.networkState); });
  await scenario('innerHTML', (v, S, r) => { v.innerHTML = '<source id=a type="video/x-bogus" src=a><source id=b type="video/x-bogus" src=b>'; r.push(v.networkState, v.children.length); });
  await scenario('innerHTMLTypeless', async (v, S, r, log) => { const loading = fetching(v, log); v.innerHTML = '<source id=a type="video/x-bogus" src=a><source id=b src=b>'; r.push(v.networkState); r.push(await loading); r.done = true; });
  await scenario('insertAdjacentHTML', (v, S, r) => { v.insertAdjacentHTML('beforeend', '<source id=a type="video/x-bogus" src=a>'); r.push(v.networkState); });
  await scenario('replaceChildren', (v, S, r) => { v.replaceChildren(S('a', bogus)); r.push(v.networkState); });
  await scenario('replaceChild', async (v, S, r) => { const a = S('a', bogus); v.append(a); await tick(); v.replaceChild(S('b', bogus), a); r.push(v.networkState); });
  await scenario('textContent', async (v, S, r) => { v.textContent = 'x'; r.push(v.networkState); });
  await scenario('nested', (v, S, r) => { const d = document.createElement('div'); v.append(d); d.append(S('a', bogus)); r.push(v.networkState); });
  await scenario('afterLoad', async (v, S, r) => { v.load(); await tick(); r.push(v.networkState); v.append(S('a', bogus)); r.push(v.networkState); });
  await scenario('connected', async (v, S, r) => { document.body.append(v); v.append(S('a', bogus)); r.push(v.networkState); await tick(); v.remove(); });
  await scenario('removeSource', async (v, S, r) => { const a = S('a', bogus); v.append(a); await tick(); a.remove(); r.push(v.networkState); });
  await scenario('srcPresent', async (v, S, r) => { v.src = ''; await tick(); r.push(v.networkState); v.append(S('a', bogus)); r.push(v.networkState); });
  await scenario('srcRemoved', async (v, S, r) => { v.src = ''; await tick(); v.removeAttribute('src'); r.push(v.networkState); v.append(S('a', bogus)); r.push(v.networkState); });
  await scenario('moved', async (v, S, r, log) => {
    const a = S('a', bogus); v.append(a); await tick();
    const d = document.createElement('div'); d.append(v); r.push(v.networkState); await tick(); r.push(log.slice(), v.networkState);
    document.body.append(v); r.push(v.networkState); await tick(); v.remove();
  });
  await scenario('subtree', async (v, S, r, log) => {
    const w = document.createElement('video'); watch(w, log); w.append(S('a', bogus)); await tick();
    const d = document.createElement('div'); d.append(w); r.push(w.networkState); await tick(); r.push(log.slice(), w.networkState);
  });
  await scenario('parsed', async (v, S, r, log) => {
    const d = document.createElement('div'); d.innerHTML = '<video><source type="video/x-bogus" src=a></video>'; const w = d.firstChild; await tick();
    watch(w, log); r.push(w.networkState); w.append(S('b', bogus)); r.push(w.networkState); await tick(); r.push(log.slice(), w.networkState);
  });
  await scenario('play', async (v, S, r) => { v.muted = true; const a = S('a', bogus); v.append(a); const p = outcome(v.play()); await tick(); r.push(p.s, v.paused); });

  // ---- TrackEvent ----
  {
    const v = document.createElement('video'), t = v.addTextTrack('captions');
    out.trackEvent = [
      run(() => { const e = new TrackEvent('x'); return [e.type, e.track, e.bubbles, e.cancelable, e.composed, e.isTrusted, tag(e)]; }),
      run(() => new TrackEvent('x', { track: t }).track === t),
      run(() => new TrackEvent('x', { track: document.createElement('track').track }).track instanceof TextTrack),
      run(() => new TrackEvent('x', { track: null }).track), run(() => new TrackEvent('x', { track: undefined }).track), run(() => new TrackEvent('x', null).track),
      run(() => new TrackEvent('x', { track: {} })), run(() => new TrackEvent('x', { track: document.createElement('track') })), run(() => new TrackEvent('x', 5)),
      run(() => { const e = new TrackEvent('x', { bubbles: 1, cancelable: 1, composed: 1 }); return [e.bubbles, e.cancelable, e.composed]; }),
      run(() => new TrackEvent('x', { get track() { throw new Error('track'); }, get bubbles() { throw new Error('bubbles'); } })),
      run(() => new TrackEvent()), run(() => TrackEvent('x')), run(() => new TrackEvent(Symbol())),
      run(() => Object.getOwnPropertyDescriptor(TrackEvent.prototype, 'track').get.call(new Event('x'))),
      run(() => document.createEvent('TrackEvent')),
    ];
  }

  // ---- TextTrackList events ----
  const show = (e, list) => e.type + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '') + ':' + tag(e) + ':' + (e.track === undefined ? '-' : e.track === null ? 'null' : e.track.label || e.track.kind) + ':' + list.length;
  const listen = (v, log) => { const list = v.textTracks; for (const t of ['addtrack', 'removetrack', 'change']) list.addEventListener(t, (e) => log.push(show(e, list))); return list; };
  const track = (o = {}) => { const t = document.createElement('track'); if (o.kind) t.kind = o.kind; if (o.label) t.label = o.label; if (o.def) t.default = true; if (o.mode) t.track.mode = o.mode; return t; };
  {
    const log = [], v = document.createElement('video');
    listen(v, log);
    const one = v.addTextTrack('captions', 'one');
    log.push('sync:' + log.length + ':' + one.mode);
    await tick();
    one.mode = 'showing'; one.mode = 'hidden'; log.push('modes:' + log.length);
    await tick();
    one.mode = 'hidden'; one.mode = 'bogus';
    await tick();
    log.push('unchanged');
    const two = track({ label: 'two' });
    v.append(two); log.push('append:' + log.length + ':' + two.track.mode);
    await tick();
    two.track.mode = 'showing';
    await tick();
    two.remove(); log.push('remove:' + log.length);
    await tick();
    v.append(two);
    await tick();
    log.push('reappend');
    v.innerHTML = ''; log.push('cleared:' + log.length);
    await tick();
    const d = document.createElement('div'), nested = track({ label: 'nested' });
    d.append(nested); v.append(d);
    await tick();
    log.push('nested:' + v.textTracks.length);
    out.trackEvents = log;
  }
  {
    const log = [], v = document.createElement('video'), list = v.textTracks;
    list.onaddtrack = (e) => log.push('on' + e.type + ':' + (e.track.label || e.track.kind));
    list.onremovetrack = (e) => log.push('on' + e.type + ':' + (e.track.label || e.track.kind));
    list.onchange = (e) => log.push('on' + e.type);
    v.innerHTML = '<track label=a><track label=b>';
    await tick(); log.push('|');
    const d = document.createElement('div'); d.append(v);
    await tick(); log.push('|');
    v.textContent = '';
    await tick(); log.push('|');
    v.innerHTML = '<track label=c>'; v.firstChild.replaceWith(track({ label: 'd' }));
    await tick(); log.push('|');
    const x = track({ label: 'x' }); v.append(x); v.append(x);
    await tick(); log.push('|');
    x.remove(); v.append(x); x.remove();
    await tick(); log.push('|');
    const added = v.addTextTrack('subtitles', 'added'); added.mode = 'disabled';
    await tick(); log.push('|');
    v.replaceChildren();
    await tick();
    list.onchange = {}; log.push(typeof list.onchange); list.onchange = 5; log.push(list.onchange);
    out.listHandlers = log;
    const w = document.createElement('video'), wlog = [], wlist = w.textTracks;
    wlist.onremovetrack = (e) => wlog.push('remove:' + e.track.label);
    const q = track({ label: 'q' }); w.append(q); q.remove();
    out.sameTask = [wlist.length];
    await tick();
    out.sameTask.push(wlog);
    const z = document.createElement('video'), zlog = [], t = track();
    z.append(t); z.textTracks.onchange = () => zlog.push('change');
    t.remove(); t.track.mode = 'showing';
    await tick();
    out.sameTask.push(zlog, t.readyState);
  }

  // ---- Automatic text track selection and the track processing model ----
  {
    const select = out.selection = {};
    const kinds = ['subtitles', 'captions', 'descriptions', 'chapters', 'metadata'];
    const probeKind = async (kind) => {
      const v = document.createElement('video'), log = [];
      v.textTracks.onchange = () => log.push('change');
      v.textTracks.onaddtrack = () => log.push('add');
      v.innerHTML = '<track kind=' + kind + ' default><track kind=' + kind + ' default><track kind=' + kind + '>';
      const tracks = [...v.children];
      for (const t of tracks) for (const type of ['load', 'error']) t.addEventListener(type, (e) => log.push(type + t.readyState + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '')));
      const before = tracks.map((t) => t.track.mode + t.readyState).join();
      await settle();
      return [before, tracks.map((t) => t.track.mode + t.readyState).join(), log.join()];
    };
    for (const kind of kinds) {
      select[kind] = await probeKind(kind);
    }
    const modes = async (body) => { const v = document.createElement('video'); const tracks = await body(v); await settle(); return tracks.map((t) => (t.track || t).mode).join(); };
    const cases = {
      hiddenAlone: (v) => { const t = track({ mode: 'hidden' }); v.append(t); return [t]; },
      showingAlone: (v) => { const t = track({ mode: 'showing' }); v.append(t); return [t]; },
      visibleThenHidden: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; await settle(); const t = track({ mode: 'hidden' }); v.append(t); return [s, t]; },
      visibleThenPlain: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; await settle(); const t = track(); v.append(t); return [s, t]; },
      visibleThenDefault: async (v) => { const s = track(); v.append(s); s.track.mode = 'showing'; const t = track({ def: true }); v.append(t); return [s, t]; },
      visibleThenTwo: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; await settle(); const x = track(), y = track({ def: true }); v.append(x, y); return [s, x, y]; },
      addedVisible: (v) => { const a = v.addTextTrack('subtitles'); a.mode = 'showing'; const t = track(); v.append(t); return [a, t]; },
      addedVisibleDefault: (v) => { const a = v.addTextTrack('subtitles'); a.mode = 'showing'; const t = track({ def: true }); v.append(t); return [a, t]; },
      addedHidden: (v) => { const a = v.addTextTrack('subtitles'); const t = track({ def: true }); v.append(t); return [a, t]; },
      addedWasVisible: (v) => { const a = v.addTextTrack('subtitles'); a.mode = 'showing'; a.mode = 'hidden'; const t = track(); v.append(t); return [a, t]; },
      reinserted: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; s.remove(); v.append(s); await settle(); const t = track({ def: true }); v.append(t); return [s, t]; },
      removedVisible: async (v) => { const s = track(); v.append(s); s.track.mode = 'showing'; await settle(); s.remove(); const t = track(); v.append(t); return [s, t]; },
      visibleOff: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; s.track.mode = 'disabled'; await settle(); const t = track(); v.append(t); return [s, t]; },
      chaptersVisible: async (v) => { const s = track({ kind: 'chapters' }); v.append(s); await settle(); s.track.mode = 'showing'; const t = track(); v.append(t); return [s, t]; },
      metadataVisible: async (v) => { const s = track({ kind: 'metadata' }); v.append(s); await settle(); s.track.mode = 'showing'; const t = track(); v.append(t); const u = track({ kind: 'chapters' }); v.append(u); return [s, t, u]; },
      descriptionsDefault: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'showing'; const t = track({ kind: 'descriptions', def: true }); v.append(t); return [s, t]; },
      twoGroups: (v) => { const t = track({ kind: 'subtitles', def: true }), u = track({ kind: 'captions', def: true }); v.append(t, u); return [t, u]; },
      autoThenPlain: async (v) => { const t = track({ def: true }); v.append(t); await settle(); const u = track(); v.append(u); return [t, u]; },
      autoThenDefault: async (v) => { const t = track({ def: true }); v.append(t); await settle(); const u = track({ def: true }); v.append(u); return [t, u]; },
      autoHiddenThenDefault: async (v) => { const t = track({ def: true }); v.append(t); await settle(); t.track.mode = 'hidden'; const u = track({ def: true }); v.append(u); return [t, u]; },
      scriptHiddenThenDefault: async (v) => { const s = track(); v.append(s); await settle(); s.track.mode = 'hidden'; await settle(); const t = track({ def: true }); v.append(t); return [s, t]; },
      disabledBeforeSelection: (v) => { const t = track({ def: true }); v.append(t); t.track.mode = 'disabled'; return [t]; },
      autoDisabledReinserted: async (v) => { const t = track({ def: true }); v.append(t); await settle(); t.track.mode = 'disabled'; t.remove(); v.append(t); return [t]; },
      removedBeforeSelection: async (v) => { const t = track({ def: true }); v.append(t); t.remove(); await settle(); v.append(t); return [t]; },
      defaultLater: async (v) => { const t = track(); v.append(t); await settle(); t.default = true; await settle(); const m = t.track.mode; const u = track(); v.append(u); await settle(); return [{ mode: m }, t]; },
      chaptersConfigured: async (v) => { const t = track({ kind: 'chapters', def: true }); v.append(t); await settle(); t.track.mode = 'disabled'; const u = track({ kind: 'chapters', def: true }); v.append(u); return [t, u]; },
      metadataLater: async (v) => { const t = track({ kind: 'metadata' }); v.append(t); await settle(); t.default = true; const u = track({ kind: 'metadata', def: true }); v.append(u); return [t, u]; },
      metadataAdded: async (v) => { const a = v.addTextTrack('metadata'); await tick(); a.mode = 'disabled'; const u = track({ kind: 'metadata', def: true }); v.append(u); return [a, u]; },
      metadataRemoved: (v) => { const t = track({ kind: 'metadata', def: true }); v.append(t); const m = t.track.mode; t.remove(); return [{ mode: m }, t]; },
    };
    for (const [name, body] of Object.entries(cases)) select[name] = await modes(body);
    {
      const log = [], v = document.createElement('video');
      watch(v, log);
      v.textTracks.onaddtrack = () => log.push('add'); v.textTracks.onchange = () => log.push('change');
      const t = track({ kind: 'metadata', def: true }); t.addEventListener('error', () => log.push('track error'));
      const s = document.createElement('source'); s.type = bogus; s.src = 'a'; s.addEventListener('error', () => log.push('source error'));
      v.append(t, s);
      await settle();
      log.push('|');
      const w = document.createElement('video');
      watch(w, log);
      w.textTracks.onaddtrack = () => log.push('add'); w.textTracks.onchange = () => log.push('change');
      const s2 = document.createElement('source'); s2.type = bogus; s2.src = 'a'; s2.addEventListener('error', () => log.push('source error'));
      const t2 = track({ kind: 'metadata', def: true }); t2.addEventListener('error', () => log.push('track error'));
      w.append(s2); w.append(t2);
      await settle();
      select.order = log;
    }
  }
  {
    const p = out.processing = [];
    const v = document.createElement('video'), t = track(), log = [];
    for (const type of ['load', 'error']) t.addEventListener(type, (e) => log.push(type + t.readyState + (e.isTrusted ? '' : '!') + (e.bubbles ? 'B' : '') + (e.cancelable ? 'C' : '')));
    t.onerror = () => log.push('onerror');
    v.append(t);
    t.track.mode = 'hidden';
    p.push(t.readyState);
    await Promise.resolve();
    p.push(t.readyState);
    await tick();
    p.push(t.readyState, log.slice(), t.track.cues.length);
    t.track.mode = 'disabled'; t.track.mode = 'showing';
    await tick();
    p.push(t.readyState, log.slice());
    const detached = track({ mode: 'hidden' });
    await tick();
    p.push(detached.readyState);
    const removed = track(); v.append(removed); removed.track.mode = 'hidden'; removed.remove();
    await tick();
    p.push(removed.readyState);
    const emptied = track(); v.append(emptied); emptied.track.mode = 'hidden'; emptied.src = '';
    await tick();
    p.push(emptied.readyState);
    const dropped = track(); dropped.src = 'z.vtt'; v.append(dropped); dropped.track.mode = 'hidden'; dropped.removeAttribute('src');
    await tick();
    p.push(dropped.readyState);
    const later = track(); v.append(later); later.track.mode = 'hidden'; later.src = 'x.vtt';
    p.push(later.readyState);
    const noMedia = document.createElement('div'), orphan = track(); noMedia.append(orphan); orphan.track.mode = 'hidden';
    await tick();
    p.push(orphan.readyState, later.readyState === 1 || later.readyState === 3);
  }
  // ---- The stream an element plays: captureStream() makes a new object over it, which its
  // srcObject (and every element's playing it) reads back from then on ----
  {
    const same = (c, s) => c.id === s.id;
    const cases = {
      pending: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; const r = [same(v.captureStream(), s)]; v.load(); r.push(same(v.captureStream(), s)); await tick(); r.push(same(v.captureStream(), s)); return r; },
      loaded: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; await tick(); v.load(); const r = [same(v.captureStream(), s)]; v.src = 'x'; r.push(same(v.captureStream(), s)); return r; },
      readBack: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; const c = v.captureStream(); const r = [v.srcObject === s, v.srcObject === c]; await tick(); r.push(v.srcObject === s, v.srcObject === c); return r; },
      shared: async () => { const s = new MediaStream(), v = document.createElement('video'), w = document.createElement('video'); v.srcObject = s; w.srcObject = s; const c = v.captureStream(); return [w.srcObject === c, v.srcObject === c, same(w.captureStream(), s)]; },
      twice: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; const c = v.captureStream(), d = v.captureStream(); return [c === d, v.srcObject === d, c.id === d.id]; },
      captured: async () => { const v = document.createElement('video'), c = v.captureStream(); v.srcObject = c; return [same(v.captureStream(), c), v.srcObject === c]; },
      cloned: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; const c = s.clone(); return [v.srcObject === s, c.id === s.id, same(v.captureStream(), c)]; },
      unset: async () => { const s = new MediaStream(), v = document.createElement('video'); v.srcObject = s; v.srcObject = null; return [same(v.captureStream(), s), v.srcObject]; },
    };
    out.played = {};
    for (const [name, body] of Object.entries(cases)) out.played[name] = await body();
  }

  // ---- Parsed media elements: their resource selection and automatic text track selection ran
  // when they were parsed ----
  {
    const tracksOf = (v) => [...v.children].filter((c) => c.localName === 'track').map((t) => t.track.mode + t.readyState).join();
    const v = document.getElementById('parsed');
    out.parsed = [v.networkState, tracksOf(v), v.textTracks.length];
    const d = document.createElement('div');
    d.innerHTML = '<video><track kind=metadata default><track default><track kind=chapters></video>';
    await settle();
    out.parsed.push(tracksOf(d.firstChild));
    const e = document.createElement('div');
    e.innerHTML = '<video><track kind=metadata default></video>';
    await settle();
    out.parsed.push(e.firstChild.firstChild.track.mode, e.firstChild.firstChild.readyState);
    const f = document.createElement('div');
    f.innerHTML = '<video><track kind=metadata default><source type="video/x-bogus" src=a></video>';
    await settle();
    const w = f.firstChild, log = [];
    w.textTracks.onaddtrack = () => log.push('add'); w.textTracks.onchange = () => log.push('change');
    const t = document.createElement('track'); t.kind = 'metadata'; t.default = true;
    w.append(t);
    await settle();
    out.parsed.push(tracksOf(w), log, w.networkState);
  }
  return out;
})()

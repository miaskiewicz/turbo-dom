(() => {
  // turbo-dom#74: HTMLTrackElement (src / kind / srclang / label / default / readyState / track,
  // NONE..ERROR) and its TextTrack (kind / label / language / id / mode / cues / activeCues /
  // oncuechange) as Chrome 156 reports them for tracks outside a media element (no load starts:
  // readyState NONE, mode disabled until set, empty cue lists once enabled).
  const run = (f) => { try { const v = f(); return v === undefined ? 'undefined' : v; } catch (e) { return [e.name, e.message]; } };
  const desc = (o, k) => { const d = Object.getOwnPropertyDescriptor(o, k); if (!d) return null; return ['value' in d ? typeof d.value : 'accessor', d.get ? d.get.name : null, d.set ? d.set.name : null, d.enumerable, d.configurable, 'writable' in d ? d.writable : null, 'value' in d && typeof d.value !== 'function' ? d.value : null]; };
  const out = {};
  out.iface = [typeof HTMLTrackElement, typeof TextTrack, typeof TextTrackCueList, run(() => Object.getPrototypeOf(HTMLTrackElement.prototype) === HTMLElement.prototype), run(() => Object.getPrototypeOf(TextTrack.prototype) === EventTarget.prototype), run(() => new HTMLTrackElement()), run(() => new TextTrack()), run(() => HTMLTrackElement.name), run(() => HTMLTrackElement.length)];
  out.constants = ['NONE', 'LOADING', 'LOADED', 'ERROR'].map((k) => [desc(HTMLTrackElement, k), desc(HTMLTrackElement.prototype, k)]);
  out.members = Object.getOwnPropertyNames(HTMLTrackElement.prototype).sort().map((k) => [k, desc(HTMLTrackElement.prototype, k)]);
  out.ttMembers = Object.getOwnPropertyNames(TextTrack.prototype).sort().map((k) => [k, desc(TextTrack.prototype, k)]);
  const parsed = document.getElementById('t'), plain = document.getElementById('p'), made = document.createElement('track');
  const view = (t) => [t instanceof HTMLTrackElement, t.constructor.name, Object.prototype.toString.call(t), t.kind, t.src, t.srclang, t.label, t.default, t.readyState, t.track instanceof TextTrack, t.track === t.track, t.track.kind, t.track.label, t.track.language, t.track.id, t.track.mode, t.track.cues, t.track.activeCues, t.track.inBandMetadataTrackDispatchType, t.track.oncuechange, Object.prototype.toString.call(t.track)];
  out.parsed = view(parsed); out.plain = view(plain); out.made = view(made);
  const kinds = {};
  for (const k of [null, '', 'subtitles', 'CAPTIONS', 'descriptions', 'chapters', 'metadata', 'bogus', ' captions']) { const t = document.createElement('track'); if (k !== null) t.setAttribute('kind', k); kinds[String(k)] = [t.kind, t.track.kind]; }
  out.kinds = kinds;
  const t = document.createElement('track');
  t.kind = 'chapters'; t.srclang = 'de'; t.label = 'X'; t.default = true; t.src = 'b.vtt';
  out.setters = [t.getAttribute('kind'), t.getAttribute('srclang'), t.getAttribute('label'), t.getAttribute('default'), t.getAttribute('src'), t.src, t.track.kind, t.track.label, t.track.language];
  t.default = false; t.kind = 'nonsense';
  out.setters2 = [t.hasAttribute('default'), t.getAttribute('kind'), t.kind, t.track.kind];
  t.id = 'tid';
  out.trackId = t.track.id;
  const tt = t.track;
  tt.mode = 'hidden'; const m1 = tt.mode; tt.mode = 'bogus'; const m2 = tt.mode; tt.mode = 'showing'; const m3 = tt.mode;
  out.mode = [m1, m2, m3, tt.cues === null ? null : [tt.cues.length, Object.prototype.toString.call(tt.cues)], tt.activeCues === null ? null : tt.activeCues.length, t.readyState];
  const empty = document.createElement('track'); empty.setAttribute('src', '');
  out.emptySrc = [empty.src, document.createElement('track').src];
  out.ttReadonly = run(() => { 'use strict'; tt.kind = 'captions'; return tt.kind; });
  out.brand = [run(() => Object.getOwnPropertyDescriptor(HTMLTrackElement.prototype, 'kind').get.call(document.body)), run(() => Object.getOwnPropertyDescriptor(TextTrack.prototype, 'mode').get.call({}))];
  return out;
})();

(function () {
  'use strict';
  // HTMLMediaElement / HTMLVideoElement playback state (turbo-dom#323), measured in
  // interface_parity_chrome156.json. The binding has no media pipeline: it fetches and decodes
  // nothing (canPlayType() answers "" for every type). So a media element runs the HTML
  // algorithms up to the point a resource would be fetched:
  //   - the media element load algorithm (load(), a src attribute set or changed): abort /
  //     emptied, the reset of its state, the playback rate back to the default;
  //   - the resource selection algorithm: no src and no <source> child -> NETWORK_EMPTY; an
  //     empty src -> MediaError 4 "Empty src attribute"; <source> children whose type nothing
  //     can play (here: any type) fire error at the source and leave NETWORK_NO_SOURCE; a
  //     src / typeless source URL becomes currentSrc and the fetch never completes
  //     (NETWORK_LOADING, HAVE_NOTHING), as on a network that never answers;
  //   - play() / pause() with Chrome's autoplay policy (muted, or the window has sticky
  //     activation), the play / waiting / timeupdate / pause events and play promises that stay
  //     pending until pause() or a load rejects them;
  //   - the attributes of a media element with no media data: readyState HAVE_NOTHING,
  //     duration NaN, empty TimeRanges, no decoded bytes or frames, videoWidth 0, and
  //     requestPictureInPicture() rejecting for missing metadata.
  // A media element parsed with a src attribute or <source> children starts in the state its
  // resource selection reached (no events were observable before the first script read it).
  // Not here (turbo-dom#339): captureStream() (no MediaStream), resource selection on <source>
  // insertion, TextTrackList's addtrack / removetrack / change events.
  const Native = globalThis.__mediaNative;
  delete globalThis.__mediaNative;
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction, accessorOf = ns.domAccessor;
  const apply = Reflect.apply, define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor, create = Object.create, construct = Reflect.construct;
  const ErrorType = TypeError, StringValue = String, NumberValue = Number, PromiseType = Promise, EventType = g.Event, Exception = ns.DOMException;
  // Captured, not page-replaceable (turbo-dom#194).
  const rejected = Function.prototype.call.bind(PromiseType.reject, PromiseType), resolved = Function.prototype.call.bind(PromiseType.resolve, PromiseType);
  const mapGet = WeakMap.prototype.get, mapSet = WeakMap.prototype.set;
  const E = g.Element.prototype, N = g.Node.prototype;
  const getAttribute = E.getAttribute, firstChild = describe(N, 'firstChild').get, nextSibling = describe(N, 'nextSibling').get;
  const nodeType = describe(N, 'nodeType').get, localName = describe(E, 'localName').get, namespaceURI = describe(E, 'namespaceURI').get;
  const now = g.performance && g.performance.now ? Function.prototype.call.bind(g.performance.now, g.performance) : () => 0;
  const P = g.HTMLMediaElement.prototype, V = g.HTMLVideoElement.prototype;
  const mediaBrand = accessorOf('nodeType', false, 'HTML:audio|video'), videoBrand = accessorOf('nodeType', false, 'HTML:video');
  const canPlayType = P.canPlayType;
  const required = (operation, owner, count, needed) => {
    if (count < needed) throw new ErrorType("Failed to execute '" + operation + "' on '" + owner + "': " + needed + ' argument required, but only ' + count + ' present.');
  };
  const finite = (value, property) => {
    const number = NumberValue(value);
    if (number !== number || number === Infinity || number === -Infinity) throw new ErrorType("Failed to set the '" + property + "' property on 'HTMLMediaElement': The provided double value is non-finite.");
    return number;
  };
  const html = (node, name) => apply(nodeType, node, []) === 1 && apply(namespaceURI, node, []) === 'http://www.w3.org/1999/xhtml' && apply(localName, node, []) === name;
  const children = (element, name) => {
    const out = [];
    for (let node = apply(firstChild, element, []); node !== null; node = apply(nextSibling, node, [])) if (html(node, name)) out[out.length] = node;
    return out;
  };
  const task = (element, callback) => Native('schedule', element, callback);
  const fire = (target, type, cancelable) => Native('dispatch', target, new EventType(type, { __proto__: null, cancelable }));
  // Blink queues media element events cancelable (and not bubbling).
  const queue = (element, type) => task(element, () => fire(element, type, true));
  function iface(name, parent) {
    const ctor = make(function () { throw new ErrorType("Failed to construct '" + name + "': Illegal constructor"); }, name, 0, true);
    const proto = ctor.prototype;
    if (parent) { Object.setPrototypeOf(proto, parent.prototype); Object.setPrototypeOf(ctor, parent); }
    define(proto, Symbol.toStringTag, { value: name, configurable: true });
    define(ctor, 'prototype', { writable: false });
    define(g, name, { value: ctor, writable: true, configurable: true, enumerable: false });
    return ctor;
  }
  const getter = (proto, name, get, put) => define(proto, name, {
    enumerable: true, configurable: true,
    get: make(get, 'get ' + name, 0),
    set: put ? make(put, 'set ' + name, 1) : undefined,
  });
  const method = (proto, name, body, length) => define(proto, name, { value: make(body, name, length), writable: true, enumerable: true, configurable: true });
  // A promise-returning operation reports its exceptions (receiver and argument checks
  // included) as a rejected promise.
  const promising = (proto, name, body, length) => method(proto, name, function () {
    try { return apply(body, this, arguments); } catch (error) { return rejected(error); }
  }, length);
  // An object state table: brand(object) returns the state or throws Illegal invocation.
  const table = () => {
    const map = new WeakMap();
    return {
      add: (object, state) => { apply(mapSet, map, [object, state]); return object; },
      brand: (object) => { const state = object !== null && typeof object === 'object' ? apply(mapGet, map, [object]) : undefined; if (!state) throw new ErrorType('Illegal invocation'); return state; },
    };
  };
  // EventHandler IDL attributes over addEventListener (like the binding's GlobalEventHandlers).
  const handlers = (proto, check, types) => {
    const add = g.EventTarget.prototype.addEventListener, drop = g.EventTarget.prototype.removeEventListener;
    for (const type of types) {
      const name = 'on' + type, values = new WeakMap();
      const callback = make(function (event) { const handler = apply(mapGet, values, [this]); if (handler && apply(handler, this, [event]) === false) event.preventDefault(); }, name, 1);
      getter(proto, name, function () { check(this); return apply(mapGet, values, [this]) || null; }, function (value) {
        check(this);
        const handler = typeof value === 'function' ? value : null, previous = apply(mapGet, values, [this]) || null;
        if (handler && !previous) apply(add, this, [type, callback]);
        if (!handler && previous) apply(drop, this, [type, callback]);
        apply(mapSet, values, [this, handler]);
      });
    }
  };

  // ---- TimeRanges: a media element without media data has no ranges ----
  const Ranges = iface('TimeRanges'), ranges = table();
  getter(Ranges.prototype, 'length', function () { ranges.brand(this); return 0; });
  for (const name of ['start', 'end']) method(Ranges.prototype, name, function (index) {
    ranges.brand(this);
    required(name, 'TimeRanges', arguments.length, 1);
    const value = NumberValue(index) >>> 0;
    throw new Exception("Failed to execute '" + name + "' on 'TimeRanges': The index provided (" + value + ') is greater than ' + (value === 0 ? 'or equal to ' : '') + 'the maximum bound (0).', 'IndexSizeError');
  }, 1);
  const newRanges = () => ranges.add(create(Ranges.prototype), {});

  // ---- MediaError ----
  const MediaErrorType = iface('MediaError'), errors = table();
  for (const [name, value] of [['MEDIA_ERR_ABORTED', 1], ['MEDIA_ERR_NETWORK', 2], ['MEDIA_ERR_DECODE', 3], ['MEDIA_ERR_SRC_NOT_SUPPORTED', 4]]) {
    for (const object of [MediaErrorType, MediaErrorType.prototype]) define(object, name, { value, writable: false, enumerable: true, configurable: true });
  }
  getter(MediaErrorType.prototype, 'code', function () { return errors.brand(this).code; });
  getter(MediaErrorType.prototype, 'message', function () { return errors.brand(this).message; });
  const newError = (code, message) => errors.add(create(MediaErrorType.prototype), { code, message });

  // ---- TextTrackList: the track element children's tracks, then addTextTrack()'s ----
  const TrackList = iface('TextTrackList', g.EventTarget), trackLists = table();
  const Shape = function () {};
  Shape.prototype = TrackList.prototype;
  const listTracks = (state) => {
    const out = [], elements = children(state.element, 'track');
    for (let index = 0; index < elements.length; index++) out[out.length] = ns.textTrackOf(elements[index]);
    for (let index = 0; index < state.added.length; index++) out[out.length] = state.added[index];
    return out;
  };
  getter(TrackList.prototype, 'length', function () { return listTracks(trackLists.brand(this)).length; });
  method(TrackList.prototype, 'getTrackById', function (id) {
    const tracks = listTracks(trackLists.brand(this));
    required('getTrackById', 'TextTrackList', arguments.length, 1);
    id = StringValue(id);
    for (let index = 0; index < tracks.length; index++) if (tracks[index].id === id) return tracks[index];
    return null;
  }, 1);
  handlers(TrackList.prototype, (list) => trackLists.brand(list), ['change', 'addtrack', 'removetrack']);
  define(TrackList.prototype, Symbol.iterator, { value: g.Array.prototype.values, writable: true, configurable: true });
  // The list's indexed properties (read-only), as the named-object interceptor asks for them.
  const arrayIndex = (key) => typeof key === 'string' && key !== '' && NumberValue(key) >>> 0 === NumberValue(key) && StringValue(NumberValue(key)) === key ? NumberValue(key) : -1;
  const indexed = {
    __proto__: null,
    indices: (tracks) => { const out = []; for (let i = 0; i < tracks.length; i++) out[i] = i; return out; },
    keys: () => [],
    get: (tracks, index) => tracks[index],
    has: (tracks, index) => index in tracks,
    descriptor: (tracks, index) => index in tracks ? { value: tracks[index], writable: false, enumerable: true, configurable: true } : undefined,
    set: (tracks, index) => index in tracks ? true : undefined,
    define: (tracks, index) => index in tracks ? true : undefined,
    delete: (tracks, index) => index in tracks ? false : undefined,
  };
  const newTrackList = (element) => {
    const list = ns.makeNamedObject(TrackList.prototype, function (operation, key) {
      const read = indexed[operation];
      return read ? read(listTracks(trackLists.brand(list)), arrayIndex(key)) : undefined;
    }, true);
    trackLists.add(list, { element, added: [] });
    return list;
  };

  // ---- VideoPlaybackQuality ----
  const Quality = iface('VideoPlaybackQuality'), qualities = table();
  for (const name of ['creationTime', 'totalVideoFrames', 'droppedVideoFrames', 'corruptedVideoFrames']) getter(Quality.prototype, name, function () { return qualities.brand(this)[name]; });

  // ---- RemotePlayback: no remote playback device is ever available ----
  const Remote = iface('RemotePlayback', g.EventTarget), remotes = table();
  const RemoteShape = function () {};
  RemoteShape.prototype = Remote.prototype;
  getter(Remote.prototype, 'state', function () { remotes.brand(this); return 'disconnected'; });
  handlers(Remote.prototype, (remote) => remotes.brand(remote), ['connecting', 'connect', 'disconnect']);
  const remoteFailure = (operation, name, message) => new Exception("Failed to execute '" + operation + "' on 'RemotePlayback': " + message, name);
  const disabledRemote = (state) => apply(getAttribute, state.element, ['disableremoteplayback']) !== null;
  promising(Remote.prototype, 'cancelWatchAvailability', function (id) {
    const state = remotes.brand(this);
    if (disabledRemote(state)) return rejected(remoteFailure('cancelWatchAvailability', 'InvalidStateError', 'disableRemotePlayback attribute is present.'));
    if (id === undefined) { state.watchers = create(null); return resolved(); }
    id = NumberValue(id) | 0;
    if (!state.watchers[id]) return rejected(remoteFailure('cancelWatchAvailability', 'NotFoundError', 'A callback with the given id is not found.'));
    delete state.watchers[id];
    return resolved();
  }, 0);
  promising(Remote.prototype, 'prompt', function () {
    const state = remotes.brand(this);
    if (disabledRemote(state)) return rejected(remoteFailure('prompt', 'InvalidStateError', 'disableRemotePlayback attribute is present.'));
    if (!Native('sticky', state.element)) return rejected(remoteFailure('prompt', 'InvalidAccessError', 'RemotePlayback::prompt() requires user gesture.'));
    return rejected(new Exception('The prompt was dismissed.', 'NotAllowedError'));
  }, 0);
  promising(Remote.prototype, 'watchAvailability', function (callback) {
    const state = remotes.brand(this);
    required('watchAvailability', 'RemotePlayback', arguments.length, 1);
    if (typeof callback !== 'function') throw new ErrorType("Failed to execute 'watchAvailability' on 'RemotePlayback': The callback provided as parameter 1 is not a function.");
    if (disabledRemote(state)) return rejected(remoteFailure('watchAvailability', 'InvalidStateError', 'disableRemotePlayback attribute is present.'));
    const id = ++state.lastWatcher;
    state.watchers[id] = callback;
    return resolved(id);
  }, 1);

  // ---- The media element's state ----
  const states = new WeakMap();
  function fresh(element) {
    const state = { __proto__: null, element, networkState: 0, readyState: 0, error: null, currentSrc: '', paused: true, start: 0, rate: 1, defaultRate: 1, volume: 1, muted: false, pitch: true, pending: [], selection: 0, tracks: null, sinkId: '', remote: null, frameCallbacks: 0 };
    apply(mapSet, states, [element, state]);
    return state;
  }
  // The state of a media element first read by script: where its resource selection got.
  function record(element) {
    let state = apply(mapGet, states, [element]);
    if (!state) {
      state = fresh(element);
      if (apply(getAttribute, element, ['src']) !== null || children(element, 'source').length) selectResource(state, false);
    }
    return state;
  }
  const media = (receiver) => { apply(mediaBrand, receiver, []); return record(receiver); };
  const video = (receiver) => { apply(videoBrand, receiver, []); return record(receiver); };
  const settle = (state, reject, error) => {
    const promises = state.pending;
    state.pending = [];
    for (let index = 0; index < promises.length; index++) reject ? promises[index].reject(error()) : promises[index].resolve();
  };
  const sourceFailure = (state, generation, message) => {
    state.networkState = 3;
    task(state.element, () => {
      if (state.selection !== generation) return;
      state.error = newError(4, message);
      fire(state.element, 'error', true);
      settle(state, true, () => new Exception('The element has no supported sources.', 'NotSupportedError'));
    });
  };
  // https://html.spec.whatwg.org/#concept-media-load-algorithm's resource selection: the
  // synchronous steps, then the rest from a task. `live` is false for the
  // catch-up of a parsed element (its events were never observable).
  function selectResource(state, live) {
    const generation = ++state.selection;
    state.networkState = 3;
    const run = () => {
      if (state.selection !== generation) return;
      const element = state.element, src = apply(getAttribute, element, ['src']), sources = src === null ? children(element, 'source') : [];
      if (src === null && !sources.length) { state.networkState = 0; return; }
      state.networkState = 2;
      if (live) queue(element, 'loadstart');
      if (src !== null) {
        if (src === '') { if (live) sourceFailure(state, generation, 'MEDIA_ELEMENT_ERROR: Empty src attribute'); else { state.networkState = 3; state.error = newError(4, 'MEDIA_ELEMENT_ERROR: Empty src attribute'); } return; }
        state.currentSrc = ns.resolveDocumentURL(element, src);
        return;
      }
      for (let index = 0; index < sources.length; index++) {
        const source = sources[index], url = apply(getAttribute, source, ['src']), type = apply(getAttribute, source, ['type']);
        if (url === null || url === '' || (type !== null && apply(canPlayType, element, [type]) === '')) {
          if (live) task(source, () => { if (state.selection === generation) fire(source, 'error', true); });
          continue;
        }
        state.currentSrc = ns.resolveDocumentURL(source, url);
        return;
      }
      // Waiting for another <source> child.
      state.networkState = 3;
    };
    // Blink runs the rest from a zero-delay task (its "stable state").
    if (live) task(state.element, run); else run();
  }
  // https://html.spec.whatwg.org/#media-element-load-algorithm
  function load(state) {
    const element = state.element;
    if (state.networkState === 1 || state.networkState === 2) queue(element, 'abort');
    if (state.networkState !== 0) {
      queue(element, 'emptied');
      state.networkState = 0;
      state.readyState = 0;
      if (!state.paused) { state.paused = true; settle(state, true, () => new Exception('The play() request was interrupted by a new load request. https://goo.gl/LdLk22', 'AbortError')); }
      state.start = 0;
    }
    if (state.rate !== state.defaultRate) { state.rate = state.defaultRate; queue(element, 'ratechange'); }
    state.error = null;
    state.currentSrc = '';
    selectResource(state, true);
  }
  ns.mediaLoad = (element) => load(record(element));
  g.__captureMediaHook = (operation, element) => {
    // "src": the attribute was set or changed (a removal does not load).
    if (apply(getAttribute, element, ['src']) === null) return;
    const state = apply(mapGet, states, [element]) || fresh(element);
    load(state);
  };

  // ---- HTMLMediaElement members ----
  getter(P, 'error', function () { return media(this).error; });
  getter(P, 'srcObject', function () { media(this); return null; }, function (value) {
    const state = media(this);
    if (value !== null && value !== undefined) throw new ErrorType("Failed to set the 'srcObject' property on 'HTMLMediaElement': The provided value is not of type '(MediaSourceHandle or MediaStream)'.");
    load(state);
  });
  getter(P, 'currentSrc', function () { return media(this).currentSrc; });
  getter(P, 'networkState', function () { return media(this).networkState; });
  for (const name of ['buffered', 'played', 'seekable']) getter(P, name, function () { media(this); return newRanges(); });
  method(P, 'load', function () { load(media(this)); }, 0);
  getter(P, 'readyState', function () { return media(this).readyState; });
  getter(P, 'seeking', function () { media(this); return false; });
  getter(P, 'currentTime', function () { return media(this).start; }, function (value) {
    const state = media(this);
    // HAVE_NOTHING: the default playback start position.
    state.start = finite(value, 'currentTime');
  });
  getter(P, 'duration', function () { media(this); return NaN; });
  getter(P, 'paused', function () { return media(this).paused; });
  // Blink's supported playback rates: 0, or [1/16, 16].
  const supportedRate = (rate) => rate === 0 || (rate >= 0.0625 && rate <= 16);
  getter(P, 'defaultPlaybackRate', function () { return media(this).defaultRate; }, function (value) {
    const state = media(this), rate = finite(value, 'defaultPlaybackRate');
    // An unsupported default rate is ignored.
    if (rate === state.defaultRate || !supportedRate(rate)) return;
    state.defaultRate = rate;
    queue(this, 'ratechange');
  });
  getter(P, 'playbackRate', function () { return media(this).rate; }, function (value) {
    const state = media(this), rate = finite(value, 'playbackRate');
    if (!supportedRate(rate)) throw new Exception("Failed to set the 'playbackRate' property on 'HTMLMediaElement': The provided playback rate (" + rate + ') is not in the supported playback range.', 'NotSupportedError');
    if (rate === state.rate) return;
    state.rate = rate;
    queue(this, 'ratechange');
  });
  getter(P, 'ended', function () { media(this); return false; });
  getter(P, 'preservesPitch', function () { return media(this).pitch; }, function (value) { media(this).pitch = !!value; });
  promising(P, 'play', function () {
    const state = media(this), element = this;
    if (!state.muted && !Native('sticky', element)) return rejected(new Exception("play() failed because the user didn't interact with the document first. https://goo.gl/xX8pDD", 'NotAllowedError'));
    if (state.error && state.error.code === 4) return rejected(new Exception('The element has no supported sources.', 'NotSupportedError'));
    if (state.networkState === 0) selectResource(state, true);
    const promise = new PromiseType((resolve, reject) => { state.pending[state.pending.length] = { resolve, reject }; });
    if (state.paused) {
      state.paused = false;
      queue(element, 'play');
      queue(element, 'waiting');
    }
    return promise;
  }, 0);
  method(P, 'pause', function () {
    const state = media(this), element = this;
    if (state.networkState === 0) selectResource(state, true);
    if (state.paused) return;
    state.paused = true;
    task(element, () => {
      fire(element, 'timeupdate', true);
      fire(element, 'pause', true);
      settle(state, true, () => new Exception('The play() request was interrupted by a call to pause(). https://goo.gl/LdLk22', 'AbortError'));
    });
  }, 0);
  getter(P, 'volume', function () { return media(this).volume; }, function (value) {
    const state = media(this), volume = finite(value, 'volume');
    if (volume < 0 || volume > 1) throw new Exception("Failed to set the 'volume' property on 'HTMLMediaElement': The volume provided (" + volume + ') is outside the range [0, 1].', 'IndexSizeError');
    if (volume === state.volume) return;
    state.volume = volume;
    queue(this, 'volumechange');
  });
  getter(P, 'muted', function () { return media(this).muted; }, function (value) {
    const state = media(this), muted = !!value;
    if (muted === state.muted) return;
    state.muted = muted;
    queue(this, 'volumechange');
  });
  getter(P, 'textTracks', function () { const state = media(this); return state.tracks || (state.tracks = newTrackList(this)); });
  const kinds = { __proto__: null, subtitles: true, captions: true, descriptions: true, chapters: true, metadata: true };
  method(P, 'addTextTrack', function (kind, label, language) {
    const state = media(this);
    required('addTextTrack', 'HTMLMediaElement', arguments.length, 1);
    kind = StringValue(kind);
    if (!kinds[kind]) throw new ErrorType("Failed to execute 'addTextTrack' on 'HTMLMediaElement': The provided value '" + kind + "' is not a valid enum value of type TextTrackKind.");
    const track = ns.newTextTrack(kind, label === undefined ? '' : StringValue(label), language === undefined ? '' : StringValue(language));
    const list = state.tracks || (state.tracks = newTrackList(this)), added = trackLists.brand(list).added;
    added[added.length] = track;
    return track;
  }, 1);
  for (const name of ['webkitAudioDecodedByteCount', 'webkitVideoDecodedByteCount']) getter(P, name, function () { media(this); return 0; });
  handlers(P, (element) => media(element), ['encrypted', 'waitingforkey']);
  getter(P, 'sinkId', function () { return media(this).sinkId; });
  promising(P, 'setSinkId', function (sinkId) {
    const state = media(this);
    required('setSinkId', 'HTMLMediaElement', arguments.length, 1);
    sinkId = StringValue(sinkId);
    // The binding has no audio output device but the default one ("").
    if (sinkId !== '') return rejected(new Exception('Requested device not found', 'NotFoundError'));
    state.sinkId = sinkId;
    return resolved();
  }, 1);
  getter(P, 'remote', function () {
    const state = media(this);
    if (!state.remote) { state.remote = construct(g.EventTarget, [], RemoteShape); remotes.add(state.remote, { element: this, watchers: create(null), lastWatcher: 0 }); }
    return state.remote;
  });
  getter(P, 'mediaKeys', function () { media(this); return null; });
  // No MediaKeys object can exist here (no key system), so only null is a MediaKeys?.
  promising(P, 'setMediaKeys', function (mediaKeys) {
    media(this);
    required('setMediaKeys', 'HTMLMediaElement', arguments.length, 1);
    if (mediaKeys !== null) throw (new ErrorType("Failed to execute 'setMediaKeys' on 'HTMLMediaElement': parameter 1 is not of type 'MediaKeys'."));
    return resolved();
  }, 1);

  // ---- HTMLVideoElement members ----
  for (const name of ['videoWidth', 'videoHeight', 'webkitDecodedFrameCount', 'webkitDroppedFrameCount']) getter(V, name, function () { video(this); return 0; });
  handlers(V, (element) => video(element), ['enterpictureinpicture', 'leavepictureinpicture']);
  method(V, 'getVideoPlaybackQuality', function () {
    video(this);
    return qualities.add(create(Quality.prototype), { creationTime: now(), totalVideoFrames: 0, droppedVideoFrames: 0, corruptedVideoFrames: 0 });
  }, 0);
  // Without media data readyState stays HAVE_NOTHING, which Chrome checks first.
  promising(V, 'requestPictureInPicture', function () {
    video(this);
    throw new Exception("Failed to execute 'requestPictureInPicture' on 'HTMLVideoElement': Metadata for the video element are not loaded yet.", 'InvalidStateError');
  }, 0);
  // Frame callbacks run when a frame is presented: never, without media data.
  method(V, 'requestVideoFrameCallback', function (callback) {
    const state = video(this);
    required('requestVideoFrameCallback', 'HTMLVideoElement', arguments.length, 1);
    if (typeof callback !== 'function') throw new ErrorType("Failed to execute 'requestVideoFrameCallback' on 'HTMLVideoElement': parameter 1 is not of type 'Function'.");
    return ++state.frameCallbacks;
  }, 1);
  method(V, 'cancelVideoFrameCallback', function (handle) {
    video(this);
    required('cancelVideoFrameCallback', 'HTMLVideoElement', arguments.length, 1);
    NumberValue(handle);
  }, 1);
})();

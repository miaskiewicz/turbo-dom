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
  //   - srcObject: a MediaStream (it has no tracks: nothing here produces one) is loaded like a
  //     src whose fetch never completes; captureStream() returns a new MediaStream;
  //   - a <source> inserted under a media element (turbo-dom#339; html_media.rs reports
  //     insertions): with no src attribute it starts resource selection from NETWORK_EMPTY,
  //     or restarts it (NETWORK_LOADING at once, a new loadstart and a rescan of every source)
  //     when the selection waits for a source, as Blink's HTMLMediaElement::SourceWasAdded;
  //   - the TextTrackList's addtrack / removetrack (TrackEvent) and change events, Chrome's
  //     automatic text track selection after a track element is inserted, and the track
  //     processing model a track's mode starts (no src: readyState ERROR and error).
  // A media element parsed with a src attribute or <source> children starts in the state its
  // resource selection reached (no events were observable before the first script read it).
  const Native = globalThis.__mediaNative;
  delete globalThis.__mediaNative;
  const g = globalThis, ns = __tsNs, make = ns.makeNativeFunction, accessorOf = ns.domAccessor;
  const apply = Reflect.apply, define = Object.defineProperty, describe = Object.getOwnPropertyDescriptor, create = Object.create, construct = Reflect.construct;
  const ErrorType = TypeError, StringValue = String, NumberValue = Number, PromiseType = Promise, EventType = g.Event, Exception = ns.DOMException;
  // Captured, not page-replaceable (turbo-dom#194).
  const rejected = Function.prototype.call.bind(PromiseType.reject, PromiseType), resolved = Function.prototype.call.bind(PromiseType.resolve, PromiseType);
  const mapGet = WeakMap.prototype.get, mapSet = WeakMap.prototype.set;
  const E = g.Element.prototype, N = g.Node.prototype;
  const getAttribute = E.getAttribute, hasAttribute = E.hasAttribute, firstChild = describe(N, 'firstChild').get, nextSibling = describe(N, 'nextSibling').get, parentNode = describe(N, 'parentNode').get;
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
  const isMedia = (node) => node !== null && (html(node, 'video') || html(node, 'audio'));
  const listed = (list, value) => { for (let index = 0; index < list.length; index++) if (list[index] === value) return true; return false; };
  // The element's `name` children, leaving out the ones in `skip`.
  const children = (element, name, skip) => {
    const out = [];
    for (let node = apply(firstChild, element, []); node !== null; node = apply(nextSibling, node, [])) if (html(node, name) && !listed(skip, node)) out[out.length] = node;
    return out;
  };
  const none = [];
  const task = (element, callback) => Native('schedule', element, callback);
  const fire = (target, type, cancelable) => Native('dispatch', target, new EventType(type, { __proto__: null, cancelable }));
  // An event at an object of the element's (its TextTrackList).
  const fireAt = (element, target, event) => Native('dispatchAt', element, event, target);
  // Blink queues media element events cancelable (and not bubbling).
  const queue = (element, type) => task(element, () => fire(element, type, true));
  // `call` is the message of a call without new (Blink's default otherwise).
  function iface(name, parent, call) {
    const ctor = make(function () { throw new ErrorType("Failed to construct '" + name + "': Illegal constructor"); }, name, 0, true, false, undefined, call);
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
      const callback = make(function (event) { const handler = apply(mapGet, values, [this]); if (typeof handler === 'function' && apply(handler, this, [event]) === false) event.preventDefault(); }, name, 1);
      getter(proto, name, function () { check(this); return apply(mapGet, values, [this]) || null; }, function (value) {
        check(this);
        // An EventHandler keeps any object; only a function is ever called.
        const handler = typeof value === 'function' || (value !== null && typeof value === 'object') ? value : null, previous = apply(mapGet, values, [this]) || null;
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
  const listTracks = (state) => {
    const out = [], elements = children(state.element, 'track', none);
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
    return ns.makeEventTarget(list);
  };

  // ---- TrackEvent ----
  const trackState = ns.textTrackState;
  const trackEvent = (() => {
    const trackEvents = new WeakMap();
    const readTrackEventInit = (init, base, prefix) => {
      let track = null;
      if (init === undefined || init === null) return track;
      base.bubbles = !!init.bubbles;
      base.cancelable = !!init.cancelable;
      base.composed = !!init.composed;
      const value = init.track;
      if (value !== undefined && value !== null && !trackState(value)) throw new ErrorType(prefix + "Failed to read the 'track' property from 'TrackEventInit': The provided value is not of type '(AudioTrack or TextTrack or VideoTrack)'.");
      if (value !== undefined) track = value;
      return track;
    };
    const TrackEventImplementation = function (type, init) {
      const prefix = "Failed to construct 'TrackEvent': ";
      if (!arguments.length) throw new ErrorType(prefix + '1 argument required, but only 0 present.');
      if (typeof type === 'symbol') throw new ErrorType(prefix + 'Cannot convert a Symbol value to a string');
      type = StringValue(type);
      if (init !== undefined && init !== null && typeof init !== 'object' && typeof init !== 'function') throw new ErrorType(prefix + "The provided value is not of type 'TrackEventInit'.");
      const base = { __proto__: null, bubbles: false, cancelable: false, composed: false };
      const track = readTrackEventInit(init, base, prefix);
      const event = construct(EventType, [type, base], new.target);
      apply(mapSet, trackEvents, [event, track]);
      return event;
    };
    TrackEventImplementation.prototype = create(EventType.prototype);
    getter(TrackEventImplementation.prototype, 'track', function () {
      const track = this !== null && typeof this === 'object' ? apply(mapGet, trackEvents, [this]) : undefined;
      if (track === undefined) throw new ErrorType('Illegal invocation');
      return track;
    });
    const TrackEventType = make(TrackEventImplementation, 'TrackEvent', 1, true, true);
    TrackEventType.prototype = TrackEventImplementation.prototype;
    define(TrackEventType.prototype, 'constructor', { value: TrackEventType, writable: true, configurable: true });
    define(TrackEventType.prototype, Symbol.toStringTag, { value: 'TrackEvent', configurable: true });
    define(TrackEventType, 'prototype', { writable: false });
    Object.setPrototypeOf(TrackEventType, EventType);
    define(g, 'TrackEvent', { value: TrackEventType, writable: true, configurable: true, enumerable: false });
    return (type, track) => construct(TrackEventType, [type, { __proto__: null, track }]);
  })();

  // ---- MediaStream: no source here produces a track, so every stream has none ----
  const { isStream, newStream, streamOver, streams } = (() => {
    // MediaStreamTrack instances never exist; its members exist and check their receiver.
    const StreamTrack = iface('MediaStreamTrack', g.EventTarget, 'Illegal constructor');
    const noTrack = () => { throw new ErrorType('Illegal invocation'); };
    for (const name of ['kind', 'id', 'label', 'muted', 'readyState', 'stats']) getter(StreamTrack.prototype, name, noTrack);
    for (const name of ['enabled', 'onmute', 'onunmute', 'onended', 'contentHint', 'oncapturehandlechange']) getter(StreamTrack.prototype, name, noTrack, noTrack);
    for (const name of ['clone', 'getCapabilities', 'getConstraints', 'getSettings', 'stop', 'getCaptureHandle']) method(StreamTrack.prototype, name, noTrack, 0);
    method(StreamTrack.prototype, 'applyConstraints', function () {
      return rejected(new ErrorType("Failed to execute 'applyConstraints' on 'MediaStreamTrack': Illegal invocation"));
    }, 0);
    const streams = table(), random = Math.random, floor = Math.floor;
    // A version 4 UUID, as Blink's MediaStream ids.
    const uuid = () => {
      const hex = '0123456789abcdef';
      let out = '';
      for (let index = 0; index < 36; index++) {
        if (index === 8 || index === 13 || index === 18 || index === 23) { out += '-'; continue; }
        const digit = floor(random() * 16);
        out += index === 14 ? '4' : index === 19 ? hex[8 + (digit & 3)] : hex[digit];
      }
      return out;
    };
    const isStream = (value) => {
      try { streams.brand(value); return true; } catch (error) { return false; }
    };
    class StreamImplementation extends g.EventTarget {
      constructor(init) {
        const prefix = "Failed to construct 'MediaStream': ";
        // The (MediaStream) and (sequence<MediaStreamTrack>) overloads; no track exists to convert to.
        if (arguments.length && !isStream(init)) {
          const iterable = init !== null && (typeof init === 'object' || typeof init === 'function') ? init[Symbol.iterator] : undefined;
          if (iterable === undefined || iterable === null) throw new ErrorType(prefix + 'Overload resolution failed.');
          const iterator = apply(iterable, init, []);
          if (!iterator.next().done) throw new ErrorType(prefix + "Failed to convert value to 'MediaStreamTrack'.");
        }
        super();
        // Blink's MediaStreamDescriptor: its id, and the stream object it last got.
        const descriptor = { __proto__: null, id: uuid(), client: this };
        streams.add(this, { descriptor });
      }
    }
    const StreamPrototype = StreamImplementation.prototype;
    delete StreamPrototype.constructor;
    const Stream = make(StreamImplementation, 'MediaStream', 0, true, true);
    Stream.prototype = StreamPrototype;
    define(StreamPrototype, 'constructor', { value: Stream, writable: true, configurable: true });
    define(StreamPrototype, Symbol.toStringTag, { value: 'MediaStream', configurable: true });
    define(Stream, 'prototype', { writable: false });
    Object.setPrototypeOf(Stream, g.EventTarget);
    define(g, 'MediaStream', { value: Stream, writable: true, configurable: true, enumerable: false });
    const newStream = () => construct(Stream, []);
    // A new stream object over an existing descriptor, which it becomes the object of.
    const streamOver = (descriptor) => {
      const stream = newStream();
      streams.brand(stream).descriptor = descriptor;
      descriptor.client = stream;
      return stream;
    };
    getter(StreamPrototype, 'id', function () { return streams.brand(this).descriptor.id; });
    getter(StreamPrototype, 'active', function () { streams.brand(this); return false; });
    handlers(StreamPrototype, (stream) => streams.brand(stream), ['addtrack', 'removetrack', 'active', 'inactive']);
    const notTrack = (operation) => new ErrorType("Failed to execute '" + operation + "' on 'MediaStream': parameter 1 is not of type 'MediaStreamTrack'.");
    for (const name of ['addTrack', 'removeTrack']) method(StreamPrototype, name, function () {
      streams.brand(this);
      required(name, 'MediaStream', arguments.length, 1);
      throw notTrack(name);
    }, 1);
    method(StreamPrototype, 'clone', function () { streams.brand(this); return newStream(); }, 0);
    for (const name of ['getAudioTracks', 'getTrackById', 'getTracks', 'getVideoTracks']) method(StreamPrototype, name, function (id) {
      streams.brand(this);
      if (name !== 'getTrackById') return [];
      required(name, 'MediaStream', arguments.length, 1);
      StringValue(id);
      return null;
    }, name === 'getTrackById' ? 1 : 0);
    return { isStream, newStream, streamOver, streams };
  })();

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
    // waiting: Blink's load state is "waiting for a source"; pointer: a <source> insertion
    // already restarted the pending selection; textVisible: a text track was showing at the
    // last mode change (Blink's text_tracks_visible_).
    const state = { __proto__: null, element, networkState: 0, readyState: 0, error: null, currentSrc: '', paused: true, start: 0, rate: 1, defaultRate: 1, volume: 1, muted: false, pitch: true, pending: [], selection: 0, tracks: null, sinkId: '', remote: null, frameCallbacks: 0, object: null, waiting: true, pointer: false, running: false, textVisible: false, honoring: false };
    apply(mapSet, states, [element, state]);
    return state;
  }
  // The state of a media element first read by script: where its resource selection got
  // (without the <source> children in `skip`, inserted since).
  // A parsed element's automatic text track selection ran too (as its track processing,
  // finished at once): no events were observable before.
  let quiet = false;
  function record(element, skip) {
    let state = apply(mapGet, states, [element]);
    skip = skip || none;
    if (!state) {
      state = fresh(element);
      if (apply(getAttribute, element, ['src']) !== null || children(element, 'source', skip).length) selectResource(state, false, skip);
      if (children(element, 'track', skip).length) {
        quiet = true;
        try { honor(state, skip); } finally { quiet = false; }
      }
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
  // The resource selection's src attribute mode.
  function fromSrc(state, generation, live, src) {
    if (src === '') { if (live) sourceFailure(state, generation, 'MEDIA_ELEMENT_ERROR: Empty src attribute'); else { state.networkState = 3; state.error = newError(4, 'MEDIA_ELEMENT_ERROR: Empty src attribute'); } return; }
    state.waiting = false;
    state.currentSrc = ns.resolveDocumentURL(state.element, src);
  }
  // Its source children mode: the first source with a URL and a type that can play.
  function fromSources(state, generation, live, sources) {
    for (let index = 0; index < sources.length; index++) {
      const source = sources[index], url = apply(getAttribute, source, ['src']), type = apply(getAttribute, source, ['type']);
      if (url === null || url === '' || (type !== null && apply(canPlayType, state.element, [type]) === '')) {
        if (live) task(source, () => { if (state.selection === generation) fire(source, 'error', true); });
        continue;
      }
      state.waiting = false;
      state.currentSrc = ns.resolveDocumentURL(source, url);
      return;
    }
    // Waiting for another <source> child.
    state.networkState = 3;
  }
  // https://html.spec.whatwg.org/#concept-media-load-algorithm's resource selection: the
  // synchronous steps, then the rest from a task. `live` is false for the
  // catch-up of a parsed element (its events were never observable).
  function selectResource(state, live, skip) {
    const generation = ++state.selection;
    state.networkState = 3;
    state.waiting = true;
    state.pointer = false;
    state.running = live;
    const run = () => {
      if (state.selection !== generation) return;
      state.running = false;
      state.pointer = false;
      const element = state.element, src = apply(getAttribute, element, ['src']), sources = src === null ? children(element, 'source', skip || none) : [];
      if (state.object) {
        // A MediaStream without tracks: its media never arrives.
        state.waiting = false;
        state.networkState = 2;
        queue(element, 'loadstart');
        return;
      }
      if (src === null && !sources.length) { state.networkState = 0; return; }
      state.networkState = 2;
      if (live) queue(element, 'loadstart');
      if (src !== null) fromSrc(state, generation, live, src); else fromSources(state, generation, live, sources);
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
  // Blink's HTMLMediaElement::SourceWasAdded: `skip` lists the sources inserted with this one.
  function sourceAdded(element, skip) {
    const state = record(element, skip);
    if (apply(getAttribute, element, ['src']) !== null) return;
    if (state.networkState === 0) {
      selectResource(state, true);
      state.pointer = true;
      return;
    }
    if (state.pointer || !state.waiting) return;
    // Waiting for a source: back to NETWORK_LOADING, and the selection runs again.
    if (!state.running) selectResource(state, true);
    state.networkState = 2;
    state.pointer = true;
  }

  // ---- Text tracks: list events, automatic selection, the track processing model ----
  const listOf = (state) => state.tracks || (state.tracks = newTrackList(state.element));
  const listEvent = (state, event) => task(state.element, () => fireAt(state.element, listOf(state), event()));
  const owners = new WeakMap();
  const parentMedia = (element) => { const parent = apply(parentNode, element, []); return isMedia(parent) ? parent : null; };
  // The media element whose list holds the track (a track element's media parent).
  const mediaOf = (track, tstate) => (tstate.element === null ? apply(mapGet, owners, [track]) : parentMedia(tstate.element));
  const isDefault = (tstate) => tstate.element !== null && apply(hasAttribute, tstate.element, ['default']);
  const textKind = describe(g.TextTrack.prototype, 'kind').get, setMode = describe(g.TextTrack.prototype, 'mode').set;
  // https://html.spec.whatwg.org/#start-the-track-processing-model, without a network that
  // answers: no URL -> ERROR and error; a new URL -> LOADING (a fetch still running for another
  // URL is canceled: ERROR and error), the same URL -> still loading.
  function loadTrack(tstate) {
    const element = tstate.element;
    if (tstate.loading || tstate.mode === 'disabled' || parentMedia(element) === null) return;
    tstate.loading = true;
    const silent = quiet, run = () => {
      tstate.loading = false;
      tstate.ready = 1;
      const src = apply(getAttribute, element, ['src']), url = src === null || src === '' ? null : ns.resolveDocumentURL(element, src);
      if (url !== null && url === tstate.url) return;
      const canceled = tstate.url !== null;
      tstate.url = url;
      if (url !== null && !canceled) return;
      tstate.ready = 3;
      if (!silent) fire(element, 'error', false);
    };
    if (silent) run(); else task(element, run);
  }
  // Blink's TextTrack mode change of a track in a list (html_track.js calls it).
  ns.textTrackModeChanged = (track, tstate) => {
    const element = mediaOf(track, tstate);
    if (!element) return;
    const state = record(element, none), tracks = listTracks(trackLists.brand(listOf(state)));
    tstate.configured = true;
    state.textVisible = false;
    for (let index = 0; index < tracks.length; index++) if (trackState(tracks[index]).mode === 'showing') state.textVisible = true;
    if (!quiet) listEvent(state, () => new EventType('change'));
    if (tstate.element !== null && tstate.ready === 0) loadTrack(tstate);
  };
  // A track element's TextTrack was made (html_track.js): its media parent is caught up.
  ns.textTrackCreated = (element) => {
    const parent = parentMedia(element);
    if (parent !== null) record(parent, none);
  };
  // Chrome's automatic text track selection (Blink's AutomaticTrackSelection, with no user
  // preference): per kind group, the tracks not yet configured, the group's showing track,
  // its first default track, and for subtitles / captions the first track while text tracks
  // are visible; default metadata tracks become hidden.
  const selected = ['subtitles', 'descriptions', 'chapters'];
  // The list's tracks by kind group (captions with subtitles): whether one shows, and the ones
  // not yet configured (nor in `skip`).
  function trackGroups(state, skip) {
    const tracks = listTracks(trackLists.brand(listOf(state))), groups = { __proto__: null };
    for (let index = 0; index < tracks.length; index++) {
      const track = tracks[index], tstate = trackState(track), kind = apply(textKind, track, []);
      if (listed(skip, tstate.element)) continue;
      const key = kind === 'captions' ? 'subtitles' : kind, group = groups[key] || (groups[key] = { __proto__: null, visible: false, tracks: [] });
      if (tstate.mode === 'showing') group.visible = true;
      if (!tstate.configured) group.tracks[group.tracks.length] = track;
    }
    return groups;
  }
  // The track a group shows: its first default one unless one shows, else for subtitles /
  // captions its first one while text tracks are visible.
  function groupChoice(state, key, group) {
    for (let index = 0; !group.visible && index < group.tracks.length; index++) if (isDefault(trackState(group.tracks[index]))) return group.tracks[index];
    return key === 'subtitles' && state.textVisible ? group.tracks[0] : null;
  }
  function honor(state, skip) {
    const groups = trackGroups(state, skip);
    for (let order = 0; order < selected.length; order++) {
      const group = groups[selected[order]], enable = group && group.tracks.length ? groupChoice(state, selected[order], group) : null;
      if (enable) apply(setMode, enable, ['showing']);
    }
    const metadata = groups.metadata ? groups.metadata.tracks : none;
    for (let index = 0; index < metadata.length; index++) {
      const tstate = trackState(metadata[index]);
      if (tstate.mode === 'disabled' && isDefault(tstate)) apply(setMode, metadata[index], ['hidden']);
    }
  }
  const operations = {
    __proto__: null,
    // The src attribute was set or changed (a removal does not load).
    src: (element) => {
      if (apply(getAttribute, element, ['src']) === null) return;
      load(apply(mapGet, states, [element]) || fresh(element));
    },
    // A track element's src attribute changed: it loads again (if its mode lets it).
    tracksrc: (element) => loadTrack(trackState(ns.textTrackOf(element))),
    source: (element, node, batch) => sourceAdded(element, batch),
    addtrack: (element, node, batch) => {
      const state = record(element, batch), track = ns.textTrackOf(node);
      listEvent(state, () => trackEvent('addtrack', track));
      if (state.honoring) return;
      state.honoring = true;
      task(element, () => { state.honoring = false; honor(state, none); });
    },
    removetrack: (element, node, batch) => {
      const state = record(element, batch), track = ns.textTrackOf(node);
      trackState(track).configured = false;
      listEvent(state, () => trackEvent('removetrack', track));
    },
  };
  // `batch`: every node of this element's steps from the same DOM call (not caught up).
  g.__captureMediaHook = (operation, element, node, batch) => operations[operation](element, node, batch);

  // ---- HTMLMediaElement members ----
  getter(P, 'error', function () { return media(this).error; });
  // The element keeps the stream's descriptor and reads back its current stream object.
  getter(P, 'srcObject', function () { const object = media(this).object; return object === null ? null : object.client; }, function (value) {
    const state = media(this);
    if (value !== null && value !== undefined && !isStream(value)) throw new ErrorType("Failed to set the 'srcObject' property on 'HTMLMediaElement': The provided value is not of type '(MediaSourceHandle or MediaStream)'.");
    state.object = value === null || value === undefined ? null : streams.brand(value).descriptor;
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
  getter(P, 'textTracks', function () { return listOf(media(this)); });
  const kinds = { __proto__: null, subtitles: true, captions: true, descriptions: true, chapters: true, metadata: true };
  method(P, 'addTextTrack', function (kind, label, language) {
    const state = media(this);
    required('addTextTrack', 'HTMLMediaElement', arguments.length, 1);
    kind = StringValue(kind);
    if (!kinds[kind]) throw new ErrorType("Failed to execute 'addTextTrack' on 'HTMLMediaElement': The provided value '" + kind + "' is not a valid enum value of type TextTrackKind.");
    const track = ns.newTextTrack(kind, label === undefined ? '' : StringValue(label), language === undefined ? '' : StringValue(language));
    const added = trackLists.brand(listOf(state)).added;
    added[added.length] = track;
    apply(mapSet, owners, [track, this]);
    // Blink appends the track (addtrack), then sets its mode to hidden (change).
    listEvent(state, () => trackEvent('addtrack', track));
    ns.textTrackModeChanged(track, trackState(track));
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
  // No MediaKeys: a new stream object over the MediaStream the element plays, else a new
  // stream (no media data, so no tracks).
  method(P, 'captureStream', function () { const state = media(this); return state.object === null ? newStream() : streamOver(state.object); }, 0);
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

(function () {
  "use strict";
  // HTMLTrackElement and TextTrack (turbo-dom#74), measured in html_track_chrome156.json. The
  // binding has no media pipeline, so a track never loads: readyState stays NONE, and its
  // TextTrack has no cues (`cues` / `activeCues` are empty lists once the mode is not
  // disabled, null while disabled). Everything else is reflection: `src` (the shared URL
  // reflection), `kind` (enumerated: missing -> subtitles, invalid -> metadata), `srclang`,
  // `label`, `default`; the TextTrack's kind / label / language / id follow the element.
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction,
    accessor = ns.domAccessor;
  const apply = Reflect.apply,
    define = Object.defineProperty,
    construct = Reflect.construct,
    ErrorType = TypeError,
    StringValue = String,
    WeakMapType = WeakMap,
    WeakSetType = WeakSet;
  const P = g.HTMLTrackElement.prototype;
  const trackBrand = accessor("nodeType", false, "HTML:track");
  const getAttribute = g.Element.prototype.getAttribute,
    setAttribute = g.Element.prototype.setAttribute,
    removeAttribute = g.Element.prototype.removeAttribute,
    hasAttribute = g.Element.prototype.hasAttribute;
  const mapGet = WeakMapType.prototype.get,
    mapSet = WeakMapType.prototype.set,
    setHas = WeakSetType.prototype.has,
    setAdd = WeakSetType.prototype.add;
  const lower = (value) => {
    let out = "";
    for (let index = 0; index < value.length; index++) {
      const code = value.charCodeAt(index);
      out += code >= 65 && code <= 90 ? StringValue.fromCharCode(code + 32) : value[index];
    }
    return out;
  };
  const text = (value) => {
    if (typeof value === "symbol") throw new ErrorType("Cannot convert a Symbol value to a string");
    return StringValue(value);
  };
  const attribute = (element, name) => {
    const value = apply(getAttribute, element, [name]);
    return value === null ? "" : value;
  };
  const kinds = { __proto__: null, subtitles: 1, captions: 1, descriptions: 1, chapters: 1, metadata: 1 };
  const kindOf = (element) => {
    const value = apply(getAttribute, element, ["kind"]);
    if (value === null) return "subtitles";
    const key = lower(value);
    return kinds[key] ? key : "metadata";
  };
  const member = (proto, name, get, set) =>
    define(proto, name, {
      get: make(get, "get " + name, 0),
      set: set ? make(set, "set " + name, 1) : undefined,
      enumerable: true,
      configurable: true,
    });
  const method = (proto, name, body, length) =>
    define(proto, name, { value: make(body, name, length), writable: true, enumerable: true, configurable: true });
  function iface(name, parent) {
    const ctor = make(
      function () {
        throw new ErrorType("Failed to construct '" + name + "': Illegal constructor");
      },
      name,
      0,
      true,
    );
    const proto = Object.create(parent ? parent.prototype : Object.prototype);
    define(proto, "constructor", { value: ctor, writable: true, configurable: true });
    define(proto, Symbol.toStringTag, { value: name, configurable: true });
    ctor.prototype = proto;
    define(ctor, "prototype", { writable: false });
    if (parent) Object.setPrototypeOf(ctor, parent);
    define(g, name, { value: ctor, writable: true, configurable: true, enumerable: false });
    return ctor;
  }

  // ---- TextTrackCueList: always empty (no cues are ever parsed) ----
  const CueList = iface("TextTrackCueList");
  const cueLists = new WeakSetType();
  const cueListBrand = (list) => {
    if (!apply(setHas, cueLists, [list])) throw new ErrorType("Illegal invocation");
  };
  member(CueList.prototype, "length", function () {
    cueListBrand(this);
    return 0;
  });
  method(CueList.prototype, "getCueById", function (id) {
    cueListBrand(this);
    if (!arguments.length) throw new ErrorType("Failed to execute 'getCueById' on 'TextTrackCueList': 1 argument required, but only 0 present.");
    text(id);
    return null;
  }, 1);
  const newCueList = () => {
    const list = Object.create(CueList.prototype);
    apply(setAdd, cueLists, [list]);
    return list;
  };

  // ---- TextTrack ----
  const Track = iface("TextTrack", g.EventTarget);
  const states = new WeakMapType();
  const stateOf = (track) => {
    const state = apply(mapGet, states, [track]);
    if (!state) throw new ErrorType("Illegal invocation");
    return state;
  };
  // A real EventTarget whose prototype is TextTrack's (addEventListener works on it).
  const Shape = function () {};
  Shape.prototype = Track.prototype;
  const newTrack = (element) => {
    const track = construct(g.EventTarget, [], Shape);
    apply(mapSet, states, [track, { __proto__: null, element, mode: "disabled", cues: null, activeCues: null, handler: null }]);
    return track;
  };
  member(Track.prototype, "kind", function () {
    return kindOf(stateOf(this).element);
  });
  member(Track.prototype, "label", function () {
    return attribute(stateOf(this).element, "label");
  });
  member(Track.prototype, "language", function () {
    return attribute(stateOf(this).element, "srclang");
  });
  member(Track.prototype, "id", function () {
    return attribute(stateOf(this).element, "id");
  });
  const modes = { __proto__: null, disabled: 1, hidden: 1, showing: 1 };
  member(
    Track.prototype,
    "mode",
    function () {
      return stateOf(this).mode;
    },
    function (value) {
      const state = stateOf(this),
        mode = text(value);
      if (!modes[mode]) return;
      state.mode = mode;
      if (mode !== "disabled" && !state.cues) {
        state.cues = newCueList();
        state.activeCues = newCueList();
      }
    },
  );
  member(Track.prototype, "cues", function () {
    const state = stateOf(this);
    return state.mode === "disabled" ? null : state.cues;
  });
  member(Track.prototype, "activeCues", function () {
    const state = stateOf(this);
    return state.mode === "disabled" ? null : state.activeCues;
  });
  const cueError = (name) => new ErrorType("Failed to execute '" + name + "' on 'TextTrack': parameter 1 is not of type 'TextTrackCue'.");
  method(Track.prototype, "addCue", function (cue) {
    stateOf(this);
    if (!arguments.length) throw new ErrorType("Failed to execute 'addCue' on 'TextTrack': 1 argument required, but only 0 present.");
    throw cueError("addCue");
  }, 1);
  method(Track.prototype, "removeCue", function (cue) {
    stateOf(this);
    if (!arguments.length) throw new ErrorType("Failed to execute 'removeCue' on 'TextTrack': 1 argument required, but only 0 present.");
    throw cueError("removeCue");
  }, 1);
  const handlerCallback = function (event) {
    const state = apply(mapGet, states, [this]);
    if (state && state.handler && apply(state.handler, this, [event]) === false) event.preventDefault();
  };
  member(
    Track.prototype,
    "oncuechange",
    function () {
      return stateOf(this).handler;
    },
    function (value) {
      const state = stateOf(this),
        handler = typeof value === "function" ? value : null;
      if (handler && !state.handler) this.addEventListener("cuechange", handlerCallback);
      if (!handler && state.handler) this.removeEventListener("cuechange", handlerCallback);
      state.handler = handler;
    },
  );

  // ---- HTMLTrackElement (members in Chrome's key order: kind, src, srclang, label, default,
  // readyState, track, then the constants) ----
  const tracks = new WeakMapType();
  const reflectedSource = Object.getOwnPropertyDescriptor(P, "src");
  if (reflectedSource) delete P.src;
  member(
    P,
    "kind",
    function () {
      apply(trackBrand, this, []);
      return kindOf(this);
    },
    function (value) {
      apply(trackBrand, this, []);
      apply(setAttribute, this, ["kind", text(value)]);
    },
  );
  if (reflectedSource) define(P, "src", reflectedSource);
  for (const name of ["srclang", "label"])
    member(
      P,
      name,
      function () {
        apply(trackBrand, this, []);
        return attribute(this, name);
      },
      function (value) {
        apply(trackBrand, this, []);
        apply(setAttribute, this, [name, text(value)]);
      },
    );
  member(
    P,
    "default",
    function () {
      apply(trackBrand, this, []);
      return apply(hasAttribute, this, ["default"]);
    },
    function (value) {
      apply(trackBrand, this, []);
      if (value) apply(setAttribute, this, ["default", ""]);
      else apply(removeAttribute, this, ["default"]);
    },
  );
  member(P, "readyState", function () {
    apply(trackBrand, this, []);
    return 0;
  });
  member(P, "track", function () {
    apply(trackBrand, this, []);
    let track = apply(mapGet, tracks, [this]);
    if (!track) {
      track = newTrack(this);
      apply(mapSet, tracks, [this, track]);
    }
    return track;
  });
  ["NONE", "LOADING", "LOADED", "ERROR"].forEach((name, value) => {
    for (const owner of [g.HTMLTrackElement, P]) define(owner, name, { value, writable: false, enumerable: true, configurable: false });
  });
})();

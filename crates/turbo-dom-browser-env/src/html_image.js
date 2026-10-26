(function () {
  "use strict";
  // HTMLImageElement's image-state members (turbo-surf#32), measured in html_image_chrome156.json:
  // complete / naturalWidth / naturalHeight / currentSrc are read-only accessors on
  // HTMLImageElement.prototype that reject every receiver but an HTML <img> ("Illegal
  // invocation"), so no image ever carries them as own properties, however it was inserted.
  //
  // The binding does not fetch or decode images; the embedder owns load state. Without host
  // state an image is "not loading": complete is true, the natural size is 0x0 and currentSrc is
  // the resolved src ("" without one), which is Chrome's answer for an image without a source and
  // for a broken one. An embedder that loads images publishes each image's state through the
  // document's private interface record (never reachable from page JS):
  //   record.imageHost.setState(img, {complete, naturalWidth, naturalHeight, currentSrc})
  //   record.imageHost.setState(img, null)   // back to the not-loading defaults
  // The state is copied when it is set (complete -> boolean, sizes -> unsigned long, currentSrc
  // -> string) and kept per element until the host replaces or clears it, e.g. when a new load
  // starts ({complete: false}) and when it finishes. See the crate README "Image load state".
  const g = globalThis,
    ns = g[Symbol.for("ts")],
    make = ns.makeNativeFunction,
    accessor = ns.domAccessor;
  const getRecord = g.__getCollectionRecord,
    setRecord = g.__setCollectionRecord;
  const P = g.HTMLImageElement.prototype,
    apply = Reflect.apply,
    define = Object.defineProperty,
    ErrorType = TypeError,
    StringValue = String,
    NumberValue = Number;
  const imageBrand = accessor("nodeType", false, "HTML:img");
  const getAttribute = g.Element.prototype.getAttribute;
  const sourceGetter = Object.getOwnPropertyDescriptor(P, "src").get;
  const states = new WeakMap(),
    stateOf = WeakMap.prototype.get,
    storeState = WeakMap.prototype.set,
    dropState = WeakMap.prototype.delete;
  const check = (receiver) => {
    if (receiver === null || (typeof receiver !== "object" && typeof receiver !== "function"))
      throw new ErrorType("Illegal invocation");
    apply(imageBrand, receiver, []);
  };
  const current = (image) => apply(stateOf, states, [image]);
  const defaultSource = (image) => {
    const source = apply(getAttribute, image, ["src"]);
    return source === null || source === "" ? "" : apply(sourceGetter, image, []);
  };
  const members = {
    complete: (image) => { const state = current(image); return state ? state.complete : true; },
    currentSrc: (image) => { const state = current(image); return state ? state.currentSrc : defaultSource(image); },
    naturalHeight: (image) => { const state = current(image); return state ? state.naturalHeight : 0; },
    naturalWidth: (image) => { const state = current(image); return state ? state.naturalWidth : 0; },
  };
  for (const name of ["complete", "currentSrc", "naturalHeight", "naturalWidth"]) {
    const read = members[name];
    define(P, name, {
      get: make(function () { check(this); return read(this); }, "get " + name, 0),
      set: undefined,
      enumerable: true,
      configurable: true,
    });
  }
  const unsigned = (value) => NumberValue(value) >>> 0;
  const imageHost = {
    __proto__: null,
    setState(image, state) {
      check(image);
      if (state === null || state === undefined) {
        apply(dropState, states, [image]);
        return;
      }
      if (typeof state !== "object" && typeof state !== "function") throw new ErrorType("Image state must be an object or null");
      const source = state.currentSrc;
      const copy = {
        __proto__: null,
        complete: !!state.complete,
        naturalWidth: unsigned(state.naturalWidth),
        naturalHeight: unsigned(state.naturalHeight),
        currentSrc: source === null || source === undefined ? "" : StringValue(source),
      };
      apply(storeState, states, [image, copy]);
    },
  };
  const documentRecord = getRecord(g.document) || { kind: "Document" };
  documentRecord.imageHost = imageHost;
  setRecord(g.document, documentRecord);
})();

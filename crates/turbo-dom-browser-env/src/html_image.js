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
  // starts ({complete: false}) and when it finishes; decode() promises settle with it. Also here:
  // the Image() legacy factory and HTMLImageElement.prototype.decode (turbo-dom#23). See the
  // crate README "Image load state".
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
  // decode() (turbo-dom#23), measured in html_image_chrome156.json `decode`: a prototype
  // operation returning a promise that resolves once the image is decoded (host state complete
  // with a natural size) and rejects with an EncodingError when it cannot be (no source, broken,
  // not loading); a call while the host reports a load in progress settles at the host's next
  // state. A bad receiver rejects with a TypeError.
  const PromiseValue = Promise,
    rejected = (error) => apply(Promise.reject, PromiseValue, [error]),
    pendingDecodes = new WeakMap();
  const DOMExceptionValue = g.DOMException;
  const decodeError = () => new DOMExceptionValue("The source image cannot be decoded.", "EncodingError");
  const settleDecode = (state, resolve, reject) => {
    if (state && state.naturalWidth > 0 && state.naturalHeight > 0) resolve(undefined);
    else reject(decodeError());
  };
  const flushDecodes = (image) => {
    const waiting = apply(stateOf, pendingDecodes, [image]);
    if (!waiting) return;
    const state = current(image);
    if (state && !state.complete) return;
    apply(dropState, pendingDecodes, [image]);
    for (let i = 0; i < waiting.length; i++) settleDecode(state, waiting[i][0], waiting[i][1]);
  };
  define(P, "decode", {
    value: make(
      function decode() {
        try {
          check(this);
        } catch {
          return rejected(new ErrorType("Failed to execute 'decode' on 'HTMLImageElement': Illegal invocation"));
        }
        const image = this;
        return new PromiseValue((resolve, reject) => {
          const state = current(image);
          if (state && !state.complete) {
            const waiting = apply(stateOf, pendingDecodes, [image]) || [];
            waiting[waiting.length] = [resolve, reject];
            apply(storeState, pendingDecodes, [image, waiting]);
          } else settleDecode(state, resolve, reject);
        });
      },
      "decode",
      0,
    ),
    writable: true,
    enumerable: true,
    configurable: true,
  });
  const unsigned = (value) => NumberValue(value) >>> 0;
  const imageHost = {
    __proto__: null,
    setState(image, state) {
      check(image);
      if (state === null || state === undefined) {
        apply(dropState, states, [image]);
        flushDecodes(image);
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
      flushDecodes(image);
    },
  };
  // The legacy factory function Image(width, height) (HTML 4.8.4.1, Web IDL
  // [LegacyFactoryFunction]; turbo-dom#23), measured in html_image_chrome156.json `factory`: a
  // native constructor (length 0) that only works with `new`, shares HTMLImageElement.prototype
  // and honours NewTarget's prototype (subclasses, Reflect.construct; a non-object prototype
  // gives Object.prototype, as Blink). Trailing undefined arguments are absent; a given width /
  // height is an unsigned long (ToUint32) reflected like the `width` / `height` IDL setters
  // (above 2147483647 -> 0) into the content attribute. The element is an ordinary <img>: `src`,
  // `decode()` and the event handlers live on the prototypes and loading is the host's
  // (imageHost above), so a factory-made image has no own properties.
  const realmDocument = g.document,
    // the native createElement at install time (page script cannot redirect the factory)
    createElement = g.document.createElement,
    setAttribute = g.Element.prototype.setAttribute,
    getProperty = Reflect.get,
    setPrototypeOf = Object.setPrototypeOf,
    objectPrototype = Object.prototype;
  const factoryError = (detail) => new ErrorType("Failed to construct 'Image': " + detail);
  const dimension = (value) => {
    if (typeof value === "symbol") throw factoryError("Cannot convert a Symbol value to a number");
    if (typeof value === "bigint") throw factoryError("Cannot convert a BigInt value to a number");
    const number = NumberValue(value) >>> 0;
    return number > 2147483647 ? 0 : number;
  };
  const ImageFactory = make(
    class {
      constructor(width, height) {
        let count = arguments.length;
        while (count > 0 && arguments[count - 1] === undefined) count--;
        const w = count > 0 ? dimension(width) : undefined;
        const h = count > 1 ? dimension(height) : undefined;
        const image = apply(createElement, realmDocument, ["img"]);
        if (w !== undefined) apply(setAttribute, image, ["width", StringValue(w)]);
        if (h !== undefined) apply(setAttribute, image, ["height", StringValue(h)]);
        const prototype = getProperty(new.target, "prototype");
        const usable = prototype !== null && (typeof prototype === "object" || typeof prototype === "function");
        if (!usable) setPrototypeOf(image, objectPrototype);
        else if (prototype !== P) setPrototypeOf(image, prototype);
        return image;
      }
    },
    "Image",
    0,
    true,
    true,
    true,
    "Failed to construct 'Image': Please use the 'new' operator, this DOM object constructor cannot be called as a function.",
  );
  define(ImageFactory, "prototype", { value: P, writable: false, enumerable: false, configurable: false });
  define(g, "Image", { value: ImageFactory, writable: true, enumerable: false, configurable: true });
  const documentRecord = getRecord(g.document) || { kind: "Document" };
  documentRecord.imageHost = imageHost;
  setRecord(g.document, documentRecord);
})();

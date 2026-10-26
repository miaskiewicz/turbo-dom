(function () {
  "use strict";
  // HTMLImageElement's image-state members (turbo-surf#32), measured in html_image_chrome156.json:
  // complete / naturalWidth / naturalHeight / currentSrc are read-only accessors on
  // HTMLImageElement.prototype that reject every receiver but an HTML <img> ("Illegal
  // invocation"), so no image ever carries them as own properties, however it was inserted.
  //
  // The binding does not fetch or decode images; the embedder owns load state. Without host
  // state an image is "not loading": complete is true, the natural size is 0x0 and currentSrc is
  // the selected source (srcset / sizes / <picture>, else src; "" without one), which is Chrome's
  // answer for an image without a source and for a broken one. An embedder that loads images publishes each image's state through the
  // document's private interface record (never reachable from page JS):
  //   record.imageHost.setState(img, {complete, naturalWidth, naturalHeight, currentSrc})
  //   record.imageHost.setState(img, null)   // back to the not-loading defaults
  // The state is copied when it is set (complete -> boolean, sizes -> unsigned long, currentSrc
  // -> string) and kept per element until the host replaces or clears it, e.g. when a new load
  // starts ({complete: false}) and when it finishes; decode() promises settle with it. Also here:
  // the Image() legacy factory and HTMLImageElement.prototype.decode (turbo-dom#23), and the
  // width / height / x / y accessors with the host's layout query (turbo-dom#26), and srcset /
  // sizes / <picture> source selection (imageHost.selectSource / naturalSize; turbo-dom#71).
  // See the crate README "Image load state".
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
  const getAttribute = g.Element.prototype.getAttribute,
    setAttribute = g.Element.prototype.setAttribute;
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
  // Without host state: the source a browser selects (srcset / sizes / <picture>, below), "" without one.
  const defaultSource = (image) => {
    const chosen = select(image, undefined);
    return chosen === null ? "" : chosen.url;
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
  const DOMExceptionValue = g[Symbol.for("ts")].DOMException;
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
  // width / height (HTML 4.8.4.3) and x / y (CSSOM View; turbo-dom#26), measured in
  // html_image_chrome156.json `dimensions`. width / height are unsigned long accessors reflecting
  // the content attributes: while the image is being rendered they return its rendered size
  // (content box, CSS px), else — as Blink — the attribute when it parses as a non-negative
  // integer (up to 4294967295), else the natural size (the host state above; 0 without one).
  // The setters write the attribute (ToUint32; above 2147483647 -> 0). x / y are read-only longs:
  // the box position relative to the initial containing block, 0 without a box. The binding has
  // no layout: the host answers through record.imageHost.setLayoutQuery(query), where
  // query(img) returns {width, height, x, y} for an image that is being rendered and null (or
  // undefined) for one that is not; without a query no image is being rendered.
  let layoutQuery = null;
  const exec = RegExp.prototype.exec,
    integerPattern = /^[\t\n\f\r ]*([-+]?)([0-9]+)/;
  const nonNegative = (text) => {
    if (text === null) return null;
    const match = apply(exec, integerPattern, [text]);
    if (!match) return null;
    const value = NumberValue(match[2]);
    if (match[1] === "-") return value === 0 ? 0 : null;
    return value <= 4294967295 ? value : null;
  };
  const layoutBox = (image) => {
    if (layoutQuery === null) return null;
    const box = apply(layoutQuery, undefined, [image]);
    return box === null || box === undefined ? null : box;
  };
  const reflectedSize = (name, natural) =>
    function () {
      check(this);
      const box = layoutBox(this);
      if (box !== null) return unsigned(box[name]);
      const value = nonNegative(apply(getAttribute, this, [name]));
      if (value !== null) return value;
      const state = current(this);
      return state ? state[natural] : 0;
    };
  const setterError = (name, kind) =>
    new ErrorType("Failed to set the '" + name + "' property on 'HTMLImageElement': Cannot convert a " + kind + " value to a number");
  for (const [name, natural] of [["width", "naturalWidth"], ["height", "naturalHeight"]]) {
    define(P, name, {
      get: make(reflectedSize(name, natural), "get " + name, 0),
      set: make(
        function (value) {
          check(this);
          if (typeof value === "symbol") throw setterError(name, "Symbol");
          if (typeof value === "bigint") throw setterError(name, "BigInt");
          const number = unsigned(value);
          apply(setAttribute, this, [name, StringValue(number > 2147483647 ? 0 : number)]);
        },
        "set " + name,
        1,
      ),
      enumerable: true,
      configurable: true,
    });
  }
  for (const name of ["x", "y"]) {
    define(P, name, {
      get: make(
        function () {
          check(this);
          const box = layoutBox(this);
          return box === null ? 0 : NumberValue(box[name]) | 0;
        },
        "get " + name,
        0,
      ),
      set: undefined,
      enumerable: true,
      configurable: true,
    });
  }
  // srcset / sizes and <picture> source selection (HTML 4.8.4.3 "select an image source";
  // turbo-dom#71), measured in html_image_srcset_chrome156.json. HTMLImageElement srcset / sizes
  // and HTMLSourceElement srcset / sizes are DOMString attributes reflecting the content
  // attributes; HTMLSourceElement width / height are unsigned longs (0 by default, above
  // 2147483647 -> 0). The binding does not load images: it selects the source a browser would
  // fetch, which the host loads (imageHost.selectSource) and which currentSrc reports without
  // host state (Chrome's answer when nothing loads: the selected URL, complete, 0x0).
  const sourceBrand = accessor("nodeType", false, "HTML:source"),
    localName = accessor("localName", false, "Element"),
    namespaceOf = accessor("namespaceURI", false, "Element"),
    parentOf = accessor("parentNode", false, "Node"),
    firstChildOf = accessor("firstChild", false, "Node"),
    nextSiblingOf = accessor("nextSibling", false, "Node"),
    nodeTypeOf = accessor("nodeType", false, "Node");
  const resolveNative = ns.resolveDocumentURL;
  const S = P, SP = g.HTMLSourceElement && g.HTMLSourceElement.prototype;
  const MathFround = Math.fround, MathFloor = Math.floor, MathMin = Math.min, MathMax = Math.max,
    NumberIsFinite = Number.isFinite, parseFloatValue = parseFloat,
    stringSlice = String.prototype.slice, stringTrim = String.prototype.trim,
    stringLower = String.prototype.toLowerCase, stringIndexOf = String.prototype.indexOf,
    arraySort = Array.prototype.sort, regexTest = RegExp.prototype.test;
  const lower = (s) => apply(stringLower, s, []), trim = (s) => apply(stringTrim, s, []),
    slice = (s, a, b) => apply(stringSlice, s, [a, b]), test = (re, s) => apply(regexTest, re, [s]);
  const toDOMString = (value) => "" + value; // a Symbol throws TypeError, null -> "null"
  const reflectString = (proto, brand, name, owner) =>
    define(proto, name, {
      get: make(function () { apply(brand, this, []); const v = apply(getAttribute, this, [name]); return v === null ? "" : v; }, "get " + name, 0),
      set: make(function (value) { apply(brand, this, []); apply(setAttribute, this, [name, toDOMString(value)]); }, "set " + name, 1),
      enumerable: true,
      configurable: true,
    });
  const sourceCheck = (receiver) => {
    if (receiver === null || (typeof receiver !== "object" && typeof receiver !== "function")) throw new ErrorType("Illegal invocation");
    apply(sourceBrand, receiver, []);
  };
  const sourceBrandFn = function () { sourceCheck(this); };
  const imageBrandFn = function () { check(this); };
  reflectString(S, imageBrandFn, "srcset");
  reflectString(S, imageBrandFn, "sizes");
  if (SP) {
    reflectString(SP, sourceBrandFn, "srcset");
    reflectString(SP, sourceBrandFn, "sizes");
    // media / type: DOMString (null -> "null"), unlike the shared nullable accessors.
    reflectString(SP, sourceBrandFn, "media");
    reflectString(SP, sourceBrandFn, "type");
    for (const name of ["width", "height"]) {
      define(SP, name, {
        get: make(function () { sourceCheck(this); const v = nonNegative(apply(getAttribute, this, [name])); return v === null || v > 2147483647 ? 0 : v; }, "get " + name, 0),
        set: make(
          function (value) {
            sourceCheck(this);
            if (typeof value === "symbol") throw new ErrorType("Failed to set the '" + name + "' property on 'HTMLSourceElement': Cannot convert a Symbol value to a number");
            if (typeof value === "bigint") throw new ErrorType("Failed to set the '" + name + "' property on 'HTMLSourceElement': Cannot convert a BigInt value to a number");
            const number = unsigned(value);
            apply(setAttribute, this, [name, StringValue(number > 2147483647 ? 0 : number)]);
          },
          "set " + name,
          1,
        ),
        enumerable: true,
        configurable: true,
      });
    }
  }

  // ---- srcset (HTML "parse a srcset attribute") ----
  const isSpace = (c) => c === " " || c === "\t" || c === "\n" || c === "\f" || c === "\r";
  const integerText = /^[0-9]+$/, floatText = /^-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$/;
  const parseSrcset = (input) => {
    const candidates = [];
    const length = input.length;
    let position = 0;
    for (;;) {
      while (position < length && (isSpace(input[position]) || input[position] === ",")) position++;
      if (position >= length) return candidates;
      const start = position;
      while (position < length && !isSpace(input[position])) position++;
      let url = slice(input, start, position);
      const descriptors = [];
      if (url[url.length - 1] === ",") {
        let end = url.length;
        while (end > 0 && url[end - 1] === ",") end--;
        url = slice(url, 0, end);
      } else {
        while (position < length && isSpace(input[position])) position++;
        let current = "", state = 0; // 0 in descriptor, 1 in parens, 2 after descriptor
        for (;; position++) {
          const c = position < length ? input[position] : null;
          if (state === 0) {
            if (c !== null && isSpace(c)) {
              if (current !== "") { descriptors[descriptors.length] = current; current = ""; }
              state = 2;
            } else if (c === ",") {
              position++;
              if (current !== "") descriptors[descriptors.length] = current;
              break;
            } else if (c === "(") { current += c; state = 1; }
            else if (c === null) { if (current !== "") descriptors[descriptors.length] = current; break; }
            else current += c;
          } else if (state === 1) {
            if (c === ")") { current += c; state = 0; }
            else if (c === null) { descriptors[descriptors.length] = current; break; }
            else current += c;
          } else if (c === null) break;
          else if (!isSpace(c)) { state = 0; position--; }
        }
      }
      let error = false, width, density, height;
      for (let i = 0; i < descriptors.length; i++) {
        const descriptor = descriptors[i], last = descriptor[descriptor.length - 1], value = slice(descriptor, 0, descriptor.length - 1);
        if (last === "w" && test(integerText, value)) {
          const n = NumberValue(value);
          if (width !== undefined || density !== undefined || n === 0) error = true;
          else width = n;
        } else if (last === "x" && test(floatText, value)) {
          const n = parseFloatValue(value);
          if (width !== undefined || density !== undefined || height !== undefined || n < 0) error = true;
          else density = n;
        } else if (last === "h" && test(integerText, value)) {
          const n = NumberValue(value);
          if (height !== undefined || density !== undefined || n === 0) error = true;
          else height = n;
        } else error = true;
      }
      if (height !== undefined && width === undefined) error = true;
      if (!error) candidates[candidates.length] = { url, width, density };
    }
  };

  // ---- CSS values for sizes / media (lengths, calc(), media conditions) ----
  // Tokens: [kind, value, unit]: "n" number / dimension, "i" ident, "(" ")" ":" "," "/" and
  // "<" "<=" ">" ">=" "=", "f" a function name (its "(" follows), "?" anything else.
  const numberToken = /^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?/,
    identToken = /^-?[A-Za-z_][A-Za-z0-9_-]*|^--[A-Za-z0-9_-]*/;
  const tokenize = (text) => {
    const tokens = [];
    let i = 0;
    while (i < text.length) {
      const c = text[i];
      if (isSpace(c)) { i++; continue; }
      const rest = slice(text, i);
      let m = apply(exec, numberToken, [rest]);
      if (m && !(c === "-" && rest[1] && test(/[A-Za-z_-]/, rest[1]))) {
        let j = m[0].length, unit = "";
        if (rest[j] === "%") { unit = "%"; j++; }
        else { const u = apply(exec, /^[A-Za-z]+/, [slice(rest, j)]); if (u) { unit = lower(u[0]); j += u[0].length; } }
        tokens[tokens.length] = ["n", NumberValue(m[0]), unit];
        i += j;
        continue;
      }
      m = apply(exec, identToken, [rest]);
      if (m) {
        const name = lower(m[0]);
        i += m[0].length;
        if (text[i] === "(") { tokens[tokens.length] = ["f", name]; i++; tokens[tokens.length] = ["("]; }
        else tokens[tokens.length] = ["i", name];
        continue;
      }
      if ((c === "<" || c === ">") && text[i + 1] === "=") { tokens[tokens.length] = [c + "="]; i += 2; continue; }
      if (c === "(" || c === ")" || c === ":" || c === "," || c === "/" || c === "<" || c === ">" || c === "=" || c === "+" || c === "*") {
        tokens[tokens.length] = [c];
        i++;
        continue;
      }
      if (c === "-") { tokens[tokens.length] = ["-"]; i++; continue; }
      tokens[tokens.length] = ["?", c];
      i++;
    }
    return tokens;
  };
  const UNITS = { __proto__: null, px: 1, em: 16, rem: 16, cm: 96 / 2.54, mm: 96 / 25.4, q: 96 / 101.6, in: 96, pt: 4 / 3, pc: 16 };
  // A length (CSS px) of a dimension token, or null; a bare 0 is a length.
  const lengthOf = (token, env) => {
    if (!token || token[0] !== "n") return null;
    const [, n, unit] = token;
    if (unit === "") return n === 0 ? 0 : null;
    if (UNITS[unit] !== undefined) return n * UNITS[unit];
    const w = env.width / 100, h = env.height / 100;
    switch (unit) {
      case "vw": case "svw": case "lvw": case "dvw": return n * w;
      case "vh": case "svh": case "lvh": case "dvh": return n * h;
      case "vmin": case "svmin": case "lvmin": case "dvmin": return n * MathMin(w, h);
      case "vmax": case "svmax": case "lvmax": case "dvmax": return n * MathMax(w, h);
      default: return null;
    }
  };
  // calc() / min() / max() / clamp() over lengths and numbers: [value, isLength] or null.
  const mathFunction = (tokens, at, env) => {
    // tokens[at] is ["f", name], tokens[at + 1] is "(".
    const name = tokens[at][1];
    let i = at + 2;
    const sum = () => {
      let left = product();
      while (left && i < tokens.length && (tokens[i][0] === "+" || tokens[i][0] === "-")) {
        const op = tokens[i++][0], right = product();
        if (!right || right[1] !== left[1]) return null;
        left = [op === "+" ? left[0] + right[0] : left[0] - right[0], left[1]];
      }
      return left;
    };
    const product = () => {
      let left = unit();
      while (left && i < tokens.length && (tokens[i][0] === "*" || tokens[i][0] === "/")) {
        const op = tokens[i++][0], right = unit();
        if (!right) return null;
        if (op === "*") { if (left[1] && right[1]) return null; left = [left[0] * right[0], left[1] || right[1]]; }
        else { if (right[1] || right[0] === 0) return null; left = [left[0] / right[0], left[1]]; }
      }
      return left;
    };
    const unit = () => {
      const t = tokens[i];
      if (!t) return null;
      if (t[0] === "(") { i++; const v = sum(); if (!v || !tokens[i] || tokens[i][0] !== ")") return null; i++; return v; }
      if (t[0] === "f") { const r = mathFunction(tokens, i, env); if (!r) return null; i = r[2]; return [r[0], r[1]]; }
      if (t[0] === "n") {
        i++;
        if (t[2] === "") return [t[1], false];
        const l = lengthOf(t, env);
        return l === null ? null : [l, true];
      }
      return null;
    };
    const args = [];
    for (;;) {
      const v = sum();
      if (!v) return null;
      args[args.length] = v;
      const t = tokens[i];
      if (t && t[0] === ",") { i++; continue; }
      if (t && t[0] === ")") { i++; break; }
      return null;
    }
    const isLength = args[0][1];
    for (const a of args) if (a[1] !== isLength) return null;
    const values = args.map((a) => a[0]);
    let value;
    if (name === "calc") { if (values.length !== 1) return null; value = values[0]; }
    else if (name === "min") value = MathMin(...values);
    else if (name === "max") value = MathMax(...values);
    else if (name === "clamp") { if (values.length !== 3) return null; value = MathMax(values[0], MathMin(values[1], values[2])); }
    else return null;
    return [value, isLength, i];
  };
  // A <source-size-value>: a non-negative length (a math function clamps to 0); null if invalid.
  const sourceSizeValue = (tokens, env) => {
    if (tokens.length === 1) {
      const l = lengthOf(tokens[0], env);
      return l === null || l < 0 ? null : l;
    }
    if (tokens[0] && tokens[0][0] === "f") {
      const r = mathFunction(tokens, 0, env);
      if (!r || !r[1] || r[2] !== tokens.length) return null;
      return MathMax(0, r[0]);
    }
    return null;
  };

  // Media features over the environment; undefined for an unknown feature or value.
  const resolutionOf = (token) => {
    if (!token || token[0] !== "n") return undefined;
    const [, n, unit] = token;
    if (unit === "dppx" || unit === "x") return n;
    if (unit === "dpi") return n / 96;
    if (unit === "dpcm") return (n * 2.54) / 96;
    return undefined;
  };
  const featureValue = (name, tokens, env) => {
    // name without min- / max-; returns [actual, wanted] numbers, or undefined if invalid.
    const one = tokens.length === 1 ? tokens[0] : null;
    switch (name) {
      case "width": case "device-width": { const l = lengthOf(one, env); return l === null ? undefined : [env.width, l]; }
      case "height": case "device-height": { const l = lengthOf(one, env); return l === null ? undefined : [env.height, l]; }
      case "aspect-ratio": case "device-aspect-ratio": {
        let ratio;
        if (tokens.length === 1 && one[0] === "n" && one[2] === "") ratio = one[1];
        else if (tokens.length === 3 && tokens[0][0] === "n" && tokens[0][2] === "" && tokens[1][0] === "/" && tokens[2][0] === "n" && tokens[2][2] === "") ratio = tokens[0][1] / tokens[2][1];
        else return undefined;
        return [env.width / env.height, ratio];
      }
      case "resolution": { const r = resolutionOf(one); return r === undefined ? undefined : [env.dpr, r]; }
      case "-webkit-device-pixel-ratio": return one && one[0] === "n" && one[2] === "" ? [env.dpr, one[1]] : undefined;
      case "color": return one && one[0] === "n" && one[2] === "" ? [8, one[1]] : undefined;
      case "monochrome": case "color-index": case "grid": return one && one[0] === "n" && one[2] === "" ? [0, one[1]] : undefined;
      default: return undefined;
    }
  };
  const IDENT_FEATURES = { __proto__: null, orientation: (env) => (env.height >= env.width ? "portrait" : "landscape"), scan: () => undefined,
    "prefers-color-scheme": () => "light", "prefers-reduced-motion": () => "no-preference", "prefers-contrast": () => "no-preference",
    "forced-colors": () => "none", "display-mode": () => "browser" };
  const BOOLEAN = { __proto__: null, width: (e) => e.width !== 0, height: (e) => e.height !== 0, color: () => true, monochrome: () => false,
    grid: () => false, resolution: () => true, orientation: () => true, "aspect-ratio": () => true, "prefers-reduced-motion": () => false,
    "prefers-color-scheme": () => true, "forced-colors": () => false };
  const compare = (actual, op, wanted) => {
    const eps = 1e-9;
    switch (op) {
      case "=": return Math.abs(actual - wanted) < eps;
      case "<": return actual < wanted - eps;
      case "<=": return actual <= wanted + eps;
      case ">": return actual > wanted + eps;
      case ">=": return actual >= wanted - eps;
      default: return false;
    }
  };
  const flip = { "<": ">", "<=": ">=", ">": "<", ">=": "<=", "=": "=" };
  const RANGE = { __proto__: null, width: 1, height: 1, "aspect-ratio": 1, resolution: 1, "device-width": 1, "device-height": 1, color: 1, monochrome: 1, "-webkit-device-pixel-ratio": 1 };
  // One "( … )" feature body: true / false, or undefined when it is not a valid feature.
  const feature = (tokens, env) => {
    if (tokens.length === 1 && tokens[0][0] === "i") {
      const f = BOOLEAN[tokens[0][1]];
      return f ? f(env) : undefined;
    }
    if (tokens.length >= 3 && tokens[0][0] === "i" && tokens[1][0] === ":") {
      let name = tokens[0][1];
      const value = tokens.slice(2);
      if (IDENT_FEATURES[name]) {
        if (value.length !== 1 || value[0][0] !== "i") return undefined;
        const actual = IDENT_FEATURES[name](env);
        return actual === undefined ? undefined : actual === value[0][1];
      }
      let op = "=";
      if (name === "-webkit-min-device-pixel-ratio" || name === "-webkit-max-device-pixel-ratio") {
        op = name[9] === "i" ? ">=" : "<=";
        name = "-webkit-device-pixel-ratio";
      } else if (slice(name, 0, 4) === "min-") { op = ">="; name = slice(name, 4); }
      else if (slice(name, 0, 4) === "max-") { op = "<="; name = slice(name, 4); }
      const pair = featureValue(name, value, env);
      return pair === undefined ? undefined : compare(pair[0], op, pair[1]);
    }
    // Range syntax: "value op name", "name op value", "value op name op value".
    const ops = [];
    for (let i = 0; i < tokens.length; i++) { const k = tokens[i][0]; if (k === "<" || k === "<=" || k === ">" || k === ">=" || k === "=") ops[ops.length] = i; }
    if (ops.length === 0 || ops.length > 2) return undefined;
    const parts = [];
    let from = 0;
    for (const at of ops) { parts[parts.length] = tokens.slice(from, at); from = at + 1; }
    parts[parts.length] = tokens.slice(from);
    const opsText = ops.map((at) => tokens[at][0]);
    const nameAt = parts.findIndex((p) => p.length === 1 && p[0][0] === "i" && RANGE[p[0][1]]);
    if (nameAt < 0) return undefined;
    const name = parts[nameAt][0][1];
    if (ops.length === 2) {
      if (nameAt !== 1) return undefined;
      const lt = opsText[0][0] === "<" && opsText[1][0] === "<", gt = opsText[0][0] === ">" && opsText[1][0] === ">";
      if (!lt && !gt) return undefined;
      const a = featureValue(name, parts[0], env), b = featureValue(name, parts[2], env);
      if (!a || !b) return undefined;
      return compare(a[0], flip[opsText[0]], a[1]) && compare(b[0], opsText[1], b[1]);
    }
    if (nameAt === 0) { const v = featureValue(name, parts[1], env); return v === undefined ? undefined : compare(v[0], opsText[0], v[1]); }
    const v = featureValue(name, parts[0], env);
    return v === undefined ? undefined : compare(v[0], flip[opsText[0]], v[1]);
  };
  // <media-condition> (allowOr) / <media-condition-without-or> over tokens[start, end):
  // true / false, or undefined when invalid. General enclosed "( … )" is false.
  const matching = (tokens, start) => {
    let depth = 0;
    for (let i = start; i < tokens.length; i++) {
      const k = tokens[i][0];
      if (k === "(") depth++;
      else if (k === ")" && --depth === 0) return i;
    }
    return -1;
  };
  const inParens = (tokens, start, env) => {
    if (!tokens[start] || tokens[start][0] !== "(") return undefined;
    const end = matching(tokens, start);
    if (end < 0) return undefined;
    const inner = tokens.slice(start + 1, end);
    let value;
    if (inner.length && (inner[0][0] === "(" || (inner[0][0] === "i" && inner[0][1] === "not"))) value = condition(inner, env, true);
    else { value = feature(inner, env); if (value === undefined && inner.length) value = false; }
    return value === undefined ? undefined : [value, end + 1];
  };
  const condition = (tokens, env, allowOr) => {
    if (tokens.length === 0) return undefined;
    if (tokens[0][0] === "i" && tokens[0][1] === "not") {
      const r = inParens(tokens, 1, env);
      return r && r[1] === tokens.length ? !r[0] : undefined;
    }
    let r = inParens(tokens, 0, env);
    if (!r) return undefined;
    let value = r[0], at = r[1], joiner = null;
    while (at < tokens.length) {
      const t = tokens[at];
      if (t[0] !== "i" || (t[1] !== "and" && t[1] !== "or")) return undefined;
      if (joiner && joiner !== t[1]) return undefined;
      if (t[1] === "or" && !allowOr) return undefined;
      joiner = t[1];
      r = inParens(tokens, at + 1, env);
      if (!r) return undefined;
      value = joiner === "and" ? value && r[0] : value || r[0];
      at = r[1];
    }
    return value;
  };
  const MEDIA_TYPES = { __proto__: null, all: true, screen: true, print: false, tty: false, tv: false, projection: false, handheld: false, braille: false, embossed: false, aural: false, speech: false };
  const mediaQuery = (tokens, env) => {
    if (tokens.length === 0) return undefined;
    if (tokens[0][0] === "(" || (tokens[0][0] === "i" && tokens[0][1] === "not" && tokens[1] && tokens[1][0] === "(")) return condition(tokens, env, true);
    let at = 0, negate = false;
    if (tokens[0][0] === "i" && (tokens[0][1] === "not" || tokens[0][1] === "only")) { negate = tokens[0][1] === "not"; at = 1; }
    const type = tokens[at];
    if (!type || type[0] !== "i" || type[1] === "and" || type[1] === "or" || type[1] === "not" || type[1] === "only") return undefined;
    let value = MEDIA_TYPES[type[1]] === true;
    at++;
    if (at < tokens.length) {
      if (tokens[at][0] !== "i" || tokens[at][1] !== "and") return undefined;
      const rest = condition(tokens.slice(at + 1), env, false);
      if (rest === undefined) return undefined;
      value = value && rest;
    }
    return negate ? !value : value;
  };
  const splitCommas = (tokens) => {
    const groups = [[]];
    let depth = 0;
    for (const t of tokens) {
      if (t[0] === "(") depth++;
      else if (t[0] === ")") depth--;
      if (t[0] === "," && depth === 0) groups[groups.length] = [];
      else groups[groups.length - 1][groups[groups.length - 1].length] = t;
    }
    return groups;
  };
  // A media query list (the <source media> attribute): any query matches; an invalid query is "not all".
  const mediaMatches = (text, env) => {
    const groups = splitCommas(tokenize(text));
    if (groups.length === 1 && groups[0].length === 0) return true;
    for (const group of groups) if (mediaQuery(group, env) === true) return true;
    return false;
  };
  // HTML "parse a sizes attribute": the first entry whose media condition matches (or that has
  // none) with a valid size; else 100vw.
  const sourceSize = (text, env) => {
    if (text !== null) {
      for (const entry of splitCommas(tokenize(text))) {
        if (entry.length === 0) continue;
        let sizeStart = entry.length - 1;
        if (entry[sizeStart][0] === ")") {
          let depth = 0;
          for (; sizeStart >= 0; sizeStart--) {
            const k = entry[sizeStart][0];
            if (k === ")") depth++;
            else if (k === "(" && --depth === 0) break;
          }
          sizeStart--; // the function name
          if (sizeStart < 0 || entry[sizeStart][0] !== "f") continue;
        }
        const size = sourceSizeValue(entry.slice(sizeStart), env);
        if (size === null) continue;
        const media = entry.slice(0, sizeStart);
        if (media.length === 0) return size;
        if (condition(media, env, true) === true) return size;
      }
    }
    return env.width;
  };

  // ---- source selection ----
  const SUPPORTED_TYPES = { __proto__: null, "image/jpeg": 1, "image/pjpeg": 1, "image/jpg": 1, "image/png": 1, "image/x-png": 1, "image/apng": 1,
    "image/gif": 1, "image/bmp": 1, "image/vnd.microsoft.icon": 1, "image/x-icon": 1, "image/x-xbitmap": 1, "image/webp": 1, "image/avif": 1,
    "image/jxl": 1, "image/svg+xml": 1 };
  const typeSupported = (type) => {
    const semicolon = apply(stringIndexOf, type, [";"]);
    const essence = lower(trim(semicolon < 0 ? type : slice(type, 0, semicolon)));
    return essence === "" || SUPPORTED_TYPES[essence] === 1;
  };
  const isHTML = (node, name) =>
    apply(nodeTypeOf, node, []) === 1 && apply(namespaceOf, node, []) === "http://www.w3.org/1999/xhtml" && apply(localName, node, []) === name;
  const environment = (overrides) => {
    const o = overrides !== null && typeof overrides === "object" ? overrides : {};
    const number = (value, fallback) => { const n = NumberValue(value); return NumberIsFinite(n) ? n : fallback; };
    const width = number(o.viewportWidth, number(g.innerWidth, 0) > 0 ? number(g.innerWidth, 1024) : 1024);
    const height = number(o.viewportHeight, number(g.innerHeight, 0) > 0 ? number(g.innerHeight, 768) : 768);
    const dpr = number(o.devicePixelRatio, number(g.devicePixelRatio, 0) > 0 ? number(g.devicePixelRatio, 1) : 1);
    return { width, height, dpr: dpr > 0 ? dpr : 1 };
  };
  // Chrome 156's candidate choice: densities normalized (w / source size, float), sorted,
  // deduplicated (the first of equal densities; the src candidate dropped when any w
  // descriptor is present), then the first density at or above the device pixel ratio, else
  // the densest.
  const pick = (candidates, size, dpr, src) => {
    let ignoreSrc = false;
    const list = [];
    for (const c of candidates) {
      let density;
      if (c.width !== undefined) { density = MathFround(c.width / size); ignoreSrc = true; }
      else density = c.density === undefined ? 1 : MathFround(c.density);
      list[list.length] = { url: c.url, density, src: false };
    }
    if (src !== null) list[list.length] = { url: src, density: 1, src: true };
    apply(arraySort, list, [(a, b) => (a.density < b.density ? -1 : a.density > b.density ? 1 : 0)]);
    const unique = [];
    let previous = -1;
    for (const c of list) {
      if (c.density !== previous && (!ignoreSrc || !c.src)) unique[unique.length] = c;
      previous = c.density;
    }
    if (unique.length === 0) return null;
    for (let i = 0; i < unique.length; i++) if (unique[i].density >= dpr) return unique[i];
    return unique[unique.length - 1];
  };
  // Parsed against the image's document base URL (<base href>, else the document URL).
  const resolve = (image, url) => resolveNative(image, url);
  const select = (image, overrides) => {
    const env = environment(overrides);
    const parent = apply(parentOf, image, []);
    if (parent && isHTML(parent, "picture")) {
      for (let child = apply(firstChildOf, parent, []); child && child !== image; child = apply(nextSiblingOf, child, [])) {
        if (!isHTML(child, "source")) continue;
        const srcset = apply(getAttribute, child, ["srcset"]);
        if (srcset === null) continue;
        const candidates = parseSrcset(srcset);
        if (candidates.length === 0) continue;
        const media = apply(getAttribute, child, ["media"]);
        if (media !== null && !mediaMatches(media, env)) continue;
        const type = apply(getAttribute, child, ["type"]);
        if (type !== null && !typeSupported(type)) continue;
        const chosen = pick(candidates, sourceSize(apply(getAttribute, child, ["sizes"]), env), env.dpr, null);
        if (chosen) return { url: resolve(image, chosen.url), density: chosen.density };
      }
    }
    const srcset = apply(getAttribute, image, ["srcset"]);
    const src = apply(getAttribute, image, ["src"]);
    const source = src === null || src === "" ? null : src;
    const candidates = srcset === null ? [] : parseSrcset(srcset);
    if (candidates.length === 0) return source === null ? null : { url: resolve(image, source), density: 1 };
    const chosen = pick(candidates, sourceSize(apply(getAttribute, image, ["sizes"]), env), env.dpr, source);
    if (!chosen) return null;
    return { url: resolve(image, chosen.url), density: chosen.density };
  };
  // The natural size of a w x h resource selected at `density` (Blink: the size scaled by the
  // float reciprocal of the density, floored).
  const densityCorrected = (size, density) => MathFloor(MathFround(size * MathFround(1 / MathFround(density))));
  const imageHost = {
    __proto__: null,
    // The source to load for an image (turbo-dom#71): {url, density} — the absolute URL the
    // srcset / sizes / <picture> selection chooses and its pixel density — or null when the
    // image has no source. `environment` ({viewportWidth, viewportHeight, devicePixelRatio},
    // each optional) defaults to the window's innerWidth / innerHeight / devicePixelRatio
    // (1024 x 768 at 1 where the window has none).
    selectSource(image, environment) {
      check(image);
      const chosen = select(image, environment);
      return chosen === null ? null : { __proto__: null, url: chosen.url, density: chosen.density };
    },
    // The natural size of a width x height resource selected at `density` (what
    // naturalWidth / naturalHeight report for it).
    naturalSize(width, height, density) {
      const d = NumberValue(density);
      // An infinite density (a w descriptor over a 0 source size) gives 0, as in Chrome.
      if (!(d > 0)) return { __proto__: null, naturalWidth: unsigned(width), naturalHeight: unsigned(height) };
      return { __proto__: null, naturalWidth: densityCorrected(unsigned(width), d), naturalHeight: densityCorrected(unsigned(height), d) };
    },
    setLayoutQuery(query) {
      if (query !== null && query !== undefined && typeof query !== "function")
        throw new ErrorType("Layout query must be a function or null");
      layoutQuery = query === undefined ? null : query;
    },
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

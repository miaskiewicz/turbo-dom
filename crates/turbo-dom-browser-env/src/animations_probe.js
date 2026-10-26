(async () => {
  // Captured with the document timeline frozen (CDP Animation.setPlaybackRate 0), so every
  // time below is exact relative to that timeline time T; numbers keep 12 significant digits.
  const out = {};
  const t = document.getElementById("t");
  const T = document.timeline.currentTime;
  const num = (value) => (typeof value === "number" && Number.isFinite(value) ? +value.toPrecision(12) : value);
  const deep = (value) =>
    Array.isArray(value)
      ? value.map(deep)
      : value && typeof value === "object"
        ? Object.fromEntries(Object.keys(value).sort().map((key) => [key, deep(value[key])]))
        : typeof value === "number" && !Number.isFinite(value)
          ? String(value)
          : num(value);
  const rel = (value) => (value === null ? null : num(value - T));
  const attempt = (callback) => {
    try {
      const value = callback();
      return value === undefined ? "undefined" : deep(value);
    } catch (error) {
      return [error.name, error.message, error instanceof DOMException];
    }
  };
  const frame = () => new Promise((resolve) => requestAnimationFrame(() => resolve()));

  // Interface shape: descriptors, own-key order, constructors and prototype chains.
  const describe = (object) =>
    Reflect.ownKeys(object).map((key) => {
      const descriptor = Object.getOwnPropertyDescriptor(object, key);
      const row = [String(key), descriptor.enumerable, descriptor.configurable];
      if ("writable" in descriptor) row.push(descriptor.writable);
      for (const kind of ["get", "set", "value"])
        if (typeof descriptor[kind] === "function")
          row.push(kind, descriptor[kind].name, descriptor[kind].length, Object.hasOwn(descriptor[kind], "prototype"));
        else if (kind === "value" && "value" in descriptor) row.push("value", typeof descriptor.value === "object" ? "object" : descriptor.value);
      return row;
    });
  const names = ["Animation", "KeyframeEffect", "AnimationEffect", "AnimationTimeline", "DocumentTimeline", "AnimationPlaybackEvent"];
  out.shape = {};
  for (const name of names) {
    const constructor = globalThis[name];
    out.shape[name] = {
      global: ((descriptor) => [descriptor.enumerable, descriptor.writable, descriptor.configurable])(Object.getOwnPropertyDescriptor(globalThis, name)),
      constructor: describe(constructor),
      prototype: describe(constructor.prototype),
      parent: Object.getPrototypeOf(constructor).name,
      prototypeParent: Object.getPrototypeOf(constructor.prototype).constructor.name,
      tag: Object.prototype.toString.call(constructor.prototype),
    };
  }
  const pick = (object, keys) => describe(object).filter((row) => keys.includes(row[0]));
  out.shape.members = {
    element: pick(Element.prototype, ["animate", "getAnimations"]),
    document: pick(Document.prototype, ["getAnimations", "timeline"]),
    shadowRoot: pick(ShadowRoot.prototype, ["getAnimations"]),
  };

  // Brand checks on every accessor and operation with a foreign receiver.
  out.brands = {};
  for (const name of names) {
    const prototype = globalThis[name].prototype;
    for (const key of Object.getOwnPropertyNames(prototype)) {
      if (key === "constructor") continue;
      const descriptor = Object.getOwnPropertyDescriptor(prototype, key);
      if (descriptor.get) out.brands[`${name}.get ${key}`] = attempt(() => descriptor.get.call({}));
      if (descriptor.set) out.brands[`${name}.set ${key}`] = attempt(() => descriptor.set.call({}, null));
      if (typeof descriptor.value === "function") out.brands[`${name}.${key}`] = attempt(() => descriptor.value.call({}));
    }
  }
  out.brands.animate = attempt(() => Element.prototype.animate.call(document, null));
  out.brands.elementGetAnimations = attempt(() => Element.prototype.getAnimations.call(document));
  out.brands.documentGetAnimations = attempt(() => Document.prototype.getAnimations.call(t));
  out.brands.shadowGetAnimations = attempt(() => ShadowRoot.prototype.getAnimations.call(document.createDocumentFragment()));
  out.brands.timeline = attempt(() => Object.getOwnPropertyDescriptor(Document.prototype, "timeline").get.call(t));
  out.brands.playbackEventCurrentTime = attempt(() => Object.getOwnPropertyDescriptor(AnimationPlaybackEvent.prototype, "currentTime").get.call(new Event("x")));

  // Constructors and conversions.
  out.constructors = {
    animationCall: attempt(() => Animation()),
    effectIllegal: attempt(() => new AnimationEffect()),
    timelineIllegal: attempt(() => new AnimationTimeline()),
    animationDefaults: attempt(() => {
      const a = new Animation();
      return [a.effect, a.timeline === document.timeline, a.startTime, a.currentTime, a.playbackRate, a.pending, a.replaceState, a.id, a.overallProgress, a.onfinish, a.playState, a instanceof EventTarget];
    }),
    nullTimeline: attempt(() => new Animation(null, null).timeline),
    undefinedTimeline: attempt(() => new Animation(undefined, undefined).timeline === document.timeline),
    badEffect: attempt(() => new Animation({})),
    badTimeline: attempt(() => new Animation(null, {})),
    subclass: attempt(() => {
      class Sub extends Animation {}
      const s = new Sub();
      return [s instanceof Sub, s instanceof Animation, s.playState, Object.getPrototypeOf(s) === Sub.prototype];
    }),
    timelineOrigin: attempt(() => {
      const timeline = new DocumentTimeline({ originTime: 100 });
      return [typeof timeline.currentTime, timeline.duration, timeline instanceof AnimationTimeline, timeline !== document.timeline];
    }),
    timelineOriginString: attempt(() => typeof new DocumentTimeline({ originTime: "5" }).currentTime),
    timelineOptions: attempt(() => new DocumentTimeline(5)),
    timelineInfinite: attempt(() => new DocumentTimeline({ originTime: Infinity })),
    documentTimeline: attempt(() => [document.timeline === document.timeline, document.timeline instanceof DocumentTimeline, document.timeline.duration, typeof document.timeline.currentTime]),
    inactiveTimeline: attempt(() => {
      const other = document.implementation.createHTMLDocument("");
      return [other.timeline === document.timeline, other.timeline.currentTime, other.timeline instanceof DocumentTimeline];
    }),
    effectNoArgs: attempt(() => new KeyframeEffect()),
    effectOneArg: attempt(() => new KeyframeEffect(null)),
    effectBadTarget: attempt(() => new KeyframeEffect(5, null)),
    effectBadKeyframes: attempt(() => new KeyframeEffect(t, 5)),
    effectStringKeyframes: attempt(() => new KeyframeEffect(t, "abc")),
    effectCopy: attempt(() => {
      const source = new KeyframeEffect(t, [{ opacity: 0 }], { duration: 100, composite: "add", easing: "ease-in" });
      const copy = new KeyframeEffect(source);
      return [copy.target === t, copy.getKeyframes().length, copy.getTiming().duration, copy.getTiming().easing, copy.composite, copy !== source];
    }),
    event: attempt(() => {
      const event = new AnimationPlaybackEvent("finish", { currentTime: 5, timelineTime: null, bubbles: true });
      return [event.type, event.currentTime, event.timelineTime, event.bubbles, event.cancelable, event.isTrusted, event instanceof Event, String(event)];
    }),
    eventNoArgs: attempt(() => new AnimationPlaybackEvent()),
    eventBadInit: attempt(() => new AnimationPlaybackEvent("x", 5)),
    eventStrings: attempt(() => {
      const event = new AnimationPlaybackEvent("x", { currentTime: "7", timelineTime: "3", composed: 1 });
      return [event.currentTime, event.timelineTime, event.composed];
    }),
    eventNonFinite: attempt(() => new AnimationPlaybackEvent("x", { currentTime: "abc" })),
    eventCall: attempt(() => AnimationPlaybackEvent("x")),
  };

  // Keyframe processing; properties are compared in code-point order (see keyframeOrder).
  const frames = (keyframes, options) => attempt(() => new KeyframeEffect(t, keyframes, options).getKeyframes());
  out.keyframes = {
    pair: frames([{ opacity: 0 }, { opacity: 1 }]),
    indexed: frames({ opacity: [0, 1], width: ["0px", "10px", "20px"] }),
    members: frames([{ opacity: 0, offset: 0.2, easing: "ease-in", composite: "add" }, { opacity: "1", color: "red", marginLeft: "10px", margin: "1px 2px" }]),
    dropped: frames([{ opacity: "abc", foo: "x", width: "bad", cssFloat: "left", "--x": "y", float: "right", webkitTransform: "none", transform: "translateX(10px)" }]),
    descending: frames([{ opacity: 0, offset: 0.8 }, { opacity: 1, offset: 0.2 }]),
    range: frames([{ opacity: 0, offset: 2 }]),
    badEasing: frames([{ opacity: 0, easing: "bogus" }]),
    indexedLists: frames({ opacity: [0, 1], offset: [0, 0.5], easing: ["ease", "linear"], composite: ["add"] }),
    nullish: [frames(null), frames(undefined)],
    nonObjectItem: frames([1]),
    nullItem: frames([{ opacity: 0 }, null]),
    single: frames({ opacity: 0.5 }),
    nullOffsets: frames([{ opacity: 0, offset: null }, { opacity: 1, offset: undefined }]),
    singleOffset: frames({ opacity: [0, 1], offset: 0.5 }),
    offsetOnly: frames([{ offset: 0.5 }]),
    easings: frames([{ opacity: 1, easing: "cubic-bezier(0.1, 0.2, 0.3, 0.4)" }, { opacity: 0, easing: "steps(2, start)" }, { opacity: 1, easing: "linear(0, 0.5 25%, 1)" }]),
    badBezier: frames([{ opacity: 0, easing: "cubic-bezier(1.1, 0, 0, 1)" }]),
    colors: frames([{ color: "#f00", backgroundColor: "rgb(0 0 255 / 50%)", transform: "rotate(45deg) scale(2)" }]),
    badComposite: frames([{ opacity: 0, composite: "bogus" }]),
    autoComposite: frames([{ opacity: 0, composite: "auto" }]),
    listInArray: frames([{ opacity: [0, 1] }]),
    discrete: frames([{ display: "none" }, { visibility: "hidden" }]),
    kebab: frames([{ "background-color": "red", "margin-left": "1px", MarginLeft: "2px" }]),
    negativeZero: frames([{ offset: -0, opacity: 0 }, { opacity: 1 }]),
    optionComposite: frames([{ opacity: 0 }], { composite: "add" }),
    indexedBadEasing: frames({ opacity: [0, 1], easing: "bogus" }),
    indexedBadComposite: frames({ opacity: [0, 1], composite: "bogus" }),
    indexedDescending: frames({ opacity: [0, 1], offset: [0.5, 0.2] }),
    indexedRange: frames({ opacity: [0, 1], offset: [2] }),
    stringOffsets: [frames([{ opacity: 0, offset: "50%" }]), frames([{ opacity: 0, offset: "0.5" }]), frames([{ opacity: 0, offset: "x" }]), frames([{ opacity: 0, offset: true }]), frames([{ opacity: 0, offset: Infinity }])],
    monotonicBeforeEasing: frames([{ opacity: 0, offset: 0.8, easing: "x" }, { offset: 0.2 }]),
    objectsBeforeEasing: frames([{ easing: "x" }, 5]),
    excessEasing: frames({ opacity: [0, 1], easing: ["linear", "x", "ease"] }),
    cyclicEasing: frames({ opacity: [0, 1, 0.5], easing: ["ease-in", "ease-out"] }),
    excessComposite: frames({ opacity: [0, 1], composite: ["add", "accumulate", "replace"] }),
    emptyList: frames({ opacity: [], width: "5px" }),
    excessOffsets: frames({ opacity: [0, 1], offset: [0, 0.5, 1] }),
    zIndex: frames([{ zIndex: "2.5" }, { zIndex: 3 }]),
    iterable: frames({ [Symbol.iterator]: function* () { yield { opacity: 0.25 }; yield { opacity: 0.75 }; } }),
  };
  const admitted = (name) => {
    try {
      const frame = new KeyframeEffect(t, [{ [name]: "inherit" }]).getKeyframes()[0];
      return Object.keys(frame).filter((key) => !["offset", "easing", "composite", "computedOffset"].includes(key)).join(",") || "-";
    } catch (error) {
      return error.message;
    }
  };
  out.keyframes.admission = Object.fromEntries(
    ["webkitTransform", "animationName", "transitionProperty", "transition", "direction", "unicodeBidi", "writingMode", "all", "willChange", "display", "cursor", "zIndex", "cssOffset", "float", "cssFloat", "border", "background", "font", "inset", "webkitUserSelect", "appearance", "contain", "containerType", "--foo", "--", "---x", "--Foo-Bar", "translate", "filter"].map((name) => [name, admitted(name)]),
  );
  const specified = (property, value) => {
    try {
      const frame = new KeyframeEffect(t, [{ [property]: value }]).getKeyframes()[0];
      return frame[property] === undefined ? "-" : frame[property];
    } catch (error) {
      return error.message;
    }
  };
  out.keyframes.values = [
    ["opacity", "0.50"], ["opacity", ".5"], ["opacity", "1e-1"], ["opacity", "50%"], ["opacity", " 1 "], ["width", "10.0px"], ["width", "0"], ["width", "10PX"], ["width", "auto"], ["width", "inherit"], ["width", "var(--x)"], ["marginLeft", "-0px"], ["transform", "translate(10px,20px)"], ["transform", "ROTATE(1turn)"], ["transform", "none"], ["transform", "matrix(1,0,0,1,0,0)"], ["transform", "translateX( 10px )"], ["color", "RED"], ["color", "transparent"], ["color", "currentColor"], ["color", "#ff000080"], ["color", "hsl(120deg 100% 50%)"], ["color", "rgb(1.5, 2, 3)"], ["backgroundColor", "rgba(0,0,0,0.333333)"], ["cssFloat", "LEFT"], ["display", "Block"], ["fontWeight", "bold"], ["lineHeight", "1.50"], ["margin", "1px  2px"], ["width", "1e1px"], ["width", "12.3456789px"], ["filter", "blur(2px)"],
  ].map(([property, value]) => [property, value, specified(property, value)]);
  // KNOWN_GAP: Chrome enumerates keyframe properties in Blink's property-set hash order.
  out.keyframeOrder = attempt(() => Object.keys(new KeyframeEffect(t, [{ opacity: "1", color: "red", marginLeft: "10px", margin: "1px 2px" }]).getKeyframes()[0]));
  const order = [];
  const options = {};
  for (const name of ["delay", "direction", "duration", "easing", "endDelay", "fill", "iterationStart", "iterations", "composite", "pseudoElement", "id", "rangeEnd", "rangeStart", "timeline", "iterationComposite"])
    Object.defineProperty(options, name, { get() { order.push("option:" + name); return undefined; }, enumerable: true });
  const keyframe = {};
  for (const name of ["offset", "easing", "composite", "opacity", "width", "color"])
    Object.defineProperty(keyframe, name, { get() { order.push("keyframe:" + name); return name === "opacity" ? "0" : name === "width" ? "1px" : name === "color" ? "red" : undefined; }, enumerable: true });
  t.animate([keyframe], options).cancel();
  out.readOrder = order;

  // Easing serialization.
  out.easings = ["ease", "linear", "ease-in", "ease-out", "ease-in-out", "step-start", "step-end", "steps(3)", "steps(3, end)", "steps(3, jump-end)", "steps(3, jump-start)", "steps(3,start)", "steps(3, jump-both)", "steps(1, jump-none)", "steps(2, jump-none)", "cubic-bezier(0.25, 0.1, 0.25, 1)", "cubic-bezier(0,0,1,1)", "cubic-bezier( .5 , -2 , .5 , 3 )", "linear(0, 1)", "linear(0 0% 50%, 1)", "linear(0, 0.25, 1)", "linear(1)", "linear()", "EASE", " ease ", "ease ease", "initial", "var(--x)", "steps(0)", "steps(1.5)", "linear(0, 1 120%)", "linear(0 -10%, 1)", "cubic-bezier(0, 0, 1)"].map((value) => [value, attempt(() => new KeyframeEffect(t, null, { easing: value }).getTiming().easing)]);

  // Timing dictionaries and the computed timing model.
  const timing = (options) => attempt(() => { const effect = new KeyframeEffect(t, null, options); return [effect.getTiming(), effect.getComputedTiming()]; });
  out.timing = {
    defaults: timing(undefined),
    number: timing(100),
    full: timing({ duration: 100, delay: 10, endDelay: 5, iterations: 2.5, iterationStart: 0.25, direction: "alternate", fill: "both", easing: "ease-out" }),
    auto: timing({ duration: "auto" }),
    badDuration: [timing({ duration: "x" }), timing(-1), timing(NaN), timing({ duration: -5 }), timing({ duration: "100" })],
    infinite: [timing(Infinity), timing({ iterations: Infinity, duration: 10 })],
    badIterations: [timing({ iterations: -1 }), timing({ iterations: NaN })],
    badStart: timing({ iterationStart: -1 }),
    badEnums: [timing({ fill: "x" }), timing({ direction: "x" }), timing({ easing: "x" })],
    nonFiniteDelay: timing({ delay: Infinity }),
    ignored: timing({ id: "x", composite: "add", iterationComposite: "accumulate", pseudoElement: "::before" }),
    badPseudo: timing({ pseudoElement: "bogus" }),
    stringOptions: timing("100"),
    zeroIterations: timing({ duration: 100, iterationStart: 0.5, iterations: 0 }),
    zeroDuration: timing({ duration: 0, iterations: 3, fill: "forwards" }),
    nullOptions: timing(null),
    negativeEndDelay: timing({ endDelay: -50, duration: 100 }),
    order: [timing({ iterationStart: -1, iterations: -1, duration: -1, easing: "x" }), timing({ iterations: -1, duration: -1, easing: "x" }), timing({ duration: -1, easing: "x" })],
    timingBeforeKeyframes: attempt(() => new KeyframeEffect(t, [{ opacity: 0, offset: 2 }], { duration: -1 })),
  };
  const effect = new KeyframeEffect(t, null, 100);
  out.updateTiming = [
    attempt(() => effect.updateTiming({ duration: 200, fill: "backwards" })),
    attempt(() => effect.getTiming()),
    attempt(() => effect.updateTiming({ duration: -1 })),
    attempt(() => effect.updateTiming(5)),
    attempt(() => effect.updateTiming({ iterations: -1 })),
    attempt(() => effect.updateTiming({ easing: "nope" })),
    attempt(() => effect.updateTiming({ fill: "nope" })),
    attempt(() => effect.updateTiming({ delay: NaN })),
    attempt(() => effect.getTiming()),
    attempt(() => effect.updateTiming()),
    attempt(() => effect.updateTiming({ duration: "auto" })),
    attempt(() => effect.getTiming().duration),
  ];
  out.pseudo = ["::before", ":before", "::after", ":after", "::marker", ":marker", "::first-line", "::selection", "::placeholder", "::backdrop", "::part(x)", "::BEFORE", "", "before", "::foo", "::slotted(div)", "::first-letter"].map((value) => [value, attempt(() => new KeyframeEffect(t, null, { pseudoElement: value }).pseudoElement)]);
  out.pseudoNull = attempt(() => new KeyframeEffect(t, null, { pseudoElement: null }).pseudoElement);

  // Effect attributes, Element.animate arguments and Animation attribute conversions.
  out.members = {
    animateOptions: attempt(() => { const a = t.animate(null, { id: "foo", duration: 50 }); const row = [a.id, a.playState, a.pending, a.effect.getTiming().duration, a.timeline === document.timeline, a.effect.target === t, a.effect instanceof KeyframeEffect]; a.cancel(); return row; }),
    animateNullTimeline: attempt(() => { const a = t.animate(null, { timeline: null }); const row = [a.timeline, a.playState, a.pending]; a.cancel(); return row; }),
    animateBadTimeline: attempt(() => t.animate(null, { timeline: {} })),
    animateNoArgs: attempt(() => t.animate()),
    animateNumber: attempt(() => t.animate(5)),
    animateBadDuration: attempt(() => t.animate([{ opacity: 0 }], { duration: -1 })),
    animateStringOptions: attempt(() => t.animate(null, "abc")),
    animateRange: attempt(() => t.animate(null, { rangeStart: "bogus" })),
    play: attempt(() => { const a = new Animation(); a.play(); return [a.playState, a.pending, a.currentTime]; }),
    nullTimelinePlay: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, 100), null); a.play(); return [a.playState, a.pending, a.currentTime, a.startTime]; }),
    nullTimelinePause: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, 100), null); a.pause(); return [a.playState, a.currentTime, a.pending]; }),
    nullTimelineFinish: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, 100), null); a.finish(); return [a.playState, a.currentTime]; }),
    finishInfinite: attempt(() => new Animation(new KeyframeEffect(t, null, { duration: 100, iterations: Infinity })).finish()),
    finishZeroRate: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, 100)); a.playbackRate = 0; a.finish(); }),
    reverseInfinite: attempt(() => new Animation(new KeyframeEffect(t, null, { duration: 100, iterations: Infinity })).reverse()),
    reverseNoTimeline: attempt(() => new Animation(new KeyframeEffect(t, null, 100), null).reverse()),
    pauseReversedInfinite: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, { duration: 100, iterations: Infinity })); a.playbackRate = -1; a.pause(); }),
    playReversedInfinite: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, { duration: 100, iterations: Infinity })); a.playbackRate = -1; a.play(); }),
    commitNoTarget: attempt(() => new Animation(new KeyframeEffect(null, null, 100)).commitStyles()),
    commitDetached: attempt(() => new Animation(new KeyframeEffect(document.createElement("div"), null, 100)).commitStyles()),
    commitPseudo: attempt(() => new Animation(new KeyframeEffect(t, null, { duration: 100, pseudoElement: "::before" })).commitStyles()),
    updateRateNoArgs: attempt(() => new Animation().updatePlaybackRate()),
    updateRateNaN: attempt(() => new Animation().updatePlaybackRate("x")),
    getAnimationsBadOptions: attempt(() => t.getAnimations(5)),
    compositeSetter: attempt(() => { const e = new KeyframeEffect(t, null); e.composite = "bogus"; const first = e.composite; e.composite = "accumulate"; return [first, e.composite]; }),
    pseudoSetter: attempt(() => { const e = new KeyframeEffect(t, null); e.pseudoElement = "bogus"; }),
    pseudoLegacy: attempt(() => { const e = new KeyframeEffect(t, null); e.pseudoElement = ":after"; const first = e.pseudoElement; e.pseudoElement = null; return [first, e.pseudoElement]; }),
    targetSetter: attempt(() => { const e = new KeyframeEffect(t, null); e.target = 5; }),
    targetNull: attempt(() => { const e = new KeyframeEffect(t, null); e.target = null; return e.target; }),
    setKeyframesNoArgs: attempt(() => new KeyframeEffect(t, null).setKeyframes()),
    setKeyframesNumber: attempt(() => new KeyframeEffect(t, null).setKeyframes(5)),
    setKeyframes: attempt(() => { const e = new KeyframeEffect(t, null); e.setKeyframes({ opacity: [0.5, 1] }); return e.getKeyframes(); }),
    setKeyframesError: attempt(() => new KeyframeEffect(t, null).setKeyframes([{ offset: 3 }])),
    timelineSetter: attempt(() => { const a = new Animation(); a.timeline = null; const first = a.timeline; a.timeline = document.timeline; return [first, a.timeline === document.timeline]; }),
    timelineBad: attempt(() => { new Animation().timeline = 5; }),
    effectBad: attempt(() => { new Animation().effect = 5; }),
    id: attempt(() => { const a = new Animation(); a.id = 5; return a.id; }),
    handlers: attempt(() => { const a = new Animation(); a.onfinish = 5; const first = a.onfinish; const fn = () => {}; a.oncancel = fn; const object = {}; a.onremove = object; return [first, a.oncancel === fn, a.onremove === object]; }),
    currentTimeConversions: attempt(() => { const a = new Animation(); const rows = []; for (const value of [{}, "5", true, [], [7], { valueOf() { return 9; } }]) { try { a.currentTime = value; rows.push(a.currentTime); } catch (error) { rows.push(error.message); } } return rows; }),
    currentTimeNull: attempt(() => { const a = new Animation(); a.currentTime = null; return a.currentTime; }),
    currentTimeUnresolve: attempt(() => { const a = new Animation(); a.currentTime = 5; a.currentTime = null; }),
    rates: attempt(() => { const a = new Animation(); a.playbackRate = "x"; }),
    startTimeBad: attempt(() => { const a = new Animation(); a.startTime = "x"; }),
    effectSwap: attempt(() => { const e = new KeyframeEffect(t, { opacity: [0, 1] }, 100); const x = new Animation(e); const y = new Animation(); y.effect = e; return [x.effect, y.effect === e, e.target === t]; }),
    overallProgress: attempt(() => { const a = t.animate(null, { duration: 100, delay: 50 }); a.pause(); const rows = [-10, 0, 75, 150, 200].map((time) => { a.currentTime = time; return a.overallProgress; }); a.cancel(); rows.push(a.overallProgress); return rows; }),
    overallZero: attempt(() => { const a = new Animation(new KeyframeEffect(t, null, 0)); a.currentTime = -1; const first = a.overallProgress; a.currentTime = 0; return [first, a.overallProgress]; }),
  };

  // Computed values while seeked (fill and phase rules, easing, interpolation and composition).
  const sample = (keyframes, options, time, properties) => {
    const a = t.animate(keyframes, options);
    a.pause();
    a.currentTime = time;
    const style = getComputedStyle(t);
    const row = properties.map((property) => style.getPropertyValue(property));
    a.cancel();
    return row;
  };
  out.style = {
    opacity: [0, 25, 50, 100].map((time) => sample({ opacity: [0, 1] }, 100, time, ["opacity"])),
    ease: [10, 50, 90].map((time) => sample({ opacity: [0, 1] }, { duration: 100, easing: "ease" }, time, ["opacity"])),
    keyframeEasing: [10, 50, 90].map((time) => sample([{ opacity: 0, easing: "ease-in-out" }, { opacity: 1 }], { duration: 100 }, time, ["opacity"])),
    steps: [0, 10, 49, 50, 99].map((time) => sample({ opacity: [0, 1] }, { duration: 100, easing: "steps(4, jump-none)" }, time, ["opacity"])),
    linear: [10, 25, 60].map((time) => sample({ opacity: [0, 1] }, { duration: 100, easing: "linear(0, 0.8 25%, 1)" }, time, ["opacity"])),
    lengths: [0, 25, 50].map((time) => sample({ width: ["0px", "100px"], marginLeft: ["10px", "20px"] }, 100, time, ["width", "margin-left"])),
    fractionalMargin: [33.333].map((time) => sample({ marginLeft: ["10px", "20px"] }, 100, time, ["margin-left"])),
    colors: [0, 33.333, 50].map((time) => sample({ color: ["red", "blue"], backgroundColor: ["rgba(255,0,0,0)", "rgba(0,0,255,1)"] }, 100, time, ["color", "background-color"])),
    translate: [0, 33.333, 50].map((time) => sample({ transform: ["translateX(0px)", "translateX(100px)"] }, 100, time, ["transform"])),
    rotateScale: [0, 50].map((time) => sample({ transform: ["rotate(0deg)", "rotate(90deg) scale(2)"] }, 100, time, ["transform"])),
    fromNone: [0, 50, 100].map((time) => sample({ transform: ["none", "scale(2)"] }, 100, time, ["transform"])),
    mismatch: [50].map((time) => sample({ transform: ["translate(10px, 20px)", "rotate(30deg)"] }, 100, time, ["transform"])),
    pairs: [25].map((time) => sample({ transform: ["translate(10px) rotate(10deg)", "translate(50px) rotate(50deg)"] }, 100, time, ["transform"])),
    translateZ: [50].map((time) => sample({ transform: ["translateZ(0px)", "translateZ(100px)"] }, 100, time, ["transform"])),
    skew: [50].map((time) => sample({ transform: ["skewX(0deg)", "skewX(40deg)"] }, 100, time, ["transform"])),
    matrix: [50].map((time) => sample({ transform: ["matrix(1,0,0,1,0,0)", "matrix(2,0,0,2,10,10)"] }, 100, time, ["transform"])),
    transformAdd: [50].map((time) => sample([{ transform: "translateX(10px)", composite: "add" }, { transform: "translateX(20px)", composite: "add" }], 100, time, ["transform"])),
    fillNone: [-10, 150].map((time) => sample({ opacity: [0.2, 0.8] }, { duration: 100, fill: "none" }, time, ["opacity"])),
    fillBoth: [-10, 150].map((time) => sample({ opacity: [0.2, 0.8] }, { duration: 100, fill: "both" }, time, ["opacity"])),
    alternate: [25, 125, 175].map((time) => sample({ opacity: [0, 1] }, { duration: 100, iterations: 2, direction: "alternate" }, time, ["opacity"])),
    implicit: [50].map((time) => sample({ opacity: 0 }, 100, time, ["opacity"])),
    add: [50].map((time) => sample([{ marginLeft: "10px", composite: "add" }, { marginLeft: "20px" }], 100, time, ["margin-left"])),
    colorAdd: [50].map((time) => sample([{ color: "rgb(100, 0, 0)", composite: "add" }, { color: "rgb(100, 0, 0)", composite: "add" }], 100, time, ["color"])),
    opacityAdd: [50].map((time) => sample([{ opacity: 0.5, composite: "add" }, { opacity: 0.5, composite: "add" }], 100, time, ["opacity"])),
    opacityAccumulate: [50].map((time) => sample([{ opacity: 0.5, composite: "accumulate" }, { opacity: 0.5, composite: "accumulate" }], 100, time, ["opacity"])),
    clamp: [10, 50].map((time) => sample({ opacity: [-1, 3] }, 100, time, ["opacity"])),
    visibility: [0, 49, 50, 99].map((time) => sample({ visibility: ["visible", "hidden"] }, 100, time, ["visibility"])),
    visibilityHidden: [0, 10, 90].map((time) => sample({ visibility: ["hidden", "collapse"] }, 100, time, ["visibility"])),
    float: [0, 49, 50].map((time) => sample({ cssFloat: ["left", "right"] }, 100, time, ["float"])),
    display: [0, 10, 50, 90, 99].map((time) => sample({ display: ["none", "block"] }, 100, time, ["display"])),
    displayPair: [0, 10, 50, 90].map((time) => sample({ display: ["block", "inline"] }, 100, time, ["display"])),
    zIndex: [0, 30, 50, 70].map((time) => sample({ zIndex: [1, 4] }, 100, time, ["z-index"])),
    em: [50].map((time) => sample({ marginLeft: ["1em", "3em"] }, 100, time, ["margin-left"])),
    zeroDuration: [0].map((time) => sample({ opacity: [0.1, 0.9] }, { duration: 0, fill: "forwards" }, time, ["opacity"])),
    currentColor: [50].map((time) => sample({ color: ["hsl(0, 100%, 50%)", "currentcolor"] }, 100, time, ["color"])),
    transparent: [50].map((time) => sample({ backgroundColor: ["transparent", "white"] }, 100, time, ["background-color"])),
  };
  out.style.computedTiming = (() => {
    const a = t.animate({ opacity: [0, 1] }, { duration: 100, delay: 20, endDelay: 10, iterations: 2, iterationStart: 0.5, direction: "alternate-reverse", fill: "both", easing: "ease-in" });
    a.pause();
    const rows = [-5, 0, 20, 70, 120, 170, 219, 220, 225, 230].map((time) => {
      a.currentTime = time;
      const computed = a.effect.getComputedTiming();
      return [computed.localTime, num(computed.progress), computed.currentIteration, getComputedStyle(t).opacity];
    });
    a.cancel();
    return rows;
  })();
  out.style.cascade = (() => {
    const child = document.createElement("span");
    t.append(child);
    const a = t.animate({ color: ["rgb(0, 0, 0)", "rgb(200, 100, 0)"], opacity: [0, 1] }, 100);
    a.pause();
    a.currentTime = 50;
    const inherited = getComputedStyle(child).color;
    t.style.setProperty("opacity", "0.3", "important");
    const important = getComputedStyle(t).opacity;
    t.style.removeProperty("opacity");
    t.style.opacity = "0.9";
    const normal = getComputedStyle(t).opacity;
    t.style.opacity = "";
    t.style.marginLeft = "10px";
    const b = t.animate([{ marginLeft: "20px", composite: "add" }, { marginLeft: "20px", composite: "add" }], 100);
    b.pause();
    const underlying = getComputedStyle(t).marginLeft;
    b.cancel();
    a.cancel();
    t.style.marginLeft = "";
    child.remove();
    return [inherited, important, normal, underlying, getComputedStyle(t).opacity];
  })();

  // Playback control against the frozen timeline: pending tasks, promises and events.
  const log = [];
  const record = (label, animation) => log.push([label, animation.playState, animation.pending, rel(animation.startTime), num(animation.currentTime), animation.playbackRate]);
  const watch = (animation, name) => {
    for (const type of ["finish", "cancel", "remove"])
      animation.addEventListener(type, (event) => log.push([name + ":" + type, event.type, num(event.currentTime), rel(event.timelineTime), event.isTrusted, event instanceof AnimationPlaybackEvent, event.bubbles, event.cancelable, event.target === animation, animation.playState]));
    animation.finished.then(() => log.push([name + ":finished", animation.playState]), (error) => log.push([name + ":finished-rejected", error.name, error.message, error instanceof DOMException]));
  };
  const a = t.animate({ opacity: [0, 1] }, { duration: 100, fill: "both" });
  watch(a, "a");
  const firstReady = a.ready;
  record("created", a);
  await a.ready;
  record("ready", a);
  log.push(["same-ready", firstReady === a.ready]);
  a.pause();
  record("pause", a);
  log.push(["pause-new-ready", firstReady !== a.ready]);
  await a.ready;
  record("paused", a);
  a.currentTime = 40;
  record("seek", a);
  log.push(["seek-style", getComputedStyle(t).opacity]);
  a.play();
  record("play", a);
  await a.ready;
  record("playing", a);
  a.playbackRate = 2;
  record("rate", a);
  a.updatePlaybackRate(0.5);
  record("update-rate", a);
  await a.ready;
  record("updated", a);
  a.reverse();
  record("reverse", a);
  await a.ready;
  record("reversed", a);
  a.finish();
  record("finish", a);
  log.push(["finish-style", getComputedStyle(t).opacity]);
  await Promise.resolve();
  await frame();
  log.push(["raf-1"]);
  await frame();
  log.push(["raf-2"]);
  a.play();
  record("replay", a);
  a.cancel();
  record("cancel", a);
  log.push(["cancel-style", getComputedStyle(t).opacity]);
  await frame();
  await frame();
  const b = t.animate({ opacity: [0, 1] }, 100);
  watch(b, "b");
  const bReady = b.ready;
  b.startTime = T - 30;
  record("start-time", b);
  log.push(["start-time-ready", bReady === b.ready]);
  await b.ready;
  log.push(["start-time-resolved"]);
  b.currentTime = 150;
  record("past-end", b);
  await frame();
  await frame();
  b.currentTime = 50;
  record("rewound", b);
  log.push(["rewound-new-finished", true]);
  b.cancel();
  await frame();
  const n = new Animation(new KeyframeEffect(t, { opacity: [0, 1] }, 100), null);
  watch(n, "n");
  n.play();
  record("null-play", n);
  n.finish();
  record("null-finish", n);
  await frame();
  await frame();
  n.cancel();
  await frame();
  await frame();
  out.control = log;

  // Replacement, persistence, removal events and commitStyles.
  const removal = [];
  const r1 = t.animate({ marginLeft: ["0px", "10px"] }, { duration: 10, fill: "forwards" });
  r1.finish();
  const r2 = t.animate({ marginLeft: ["0px", "20px"] }, { duration: 10, fill: "forwards" });
  const p1 = t.animate({ marginTop: ["0px", "5px"] }, { duration: 10, fill: "forwards" });
  p1.finish();
  p1.persist();
  const p2 = t.animate({ marginTop: ["0px", "6px"] }, { duration: 10, fill: "forwards" });
  p2.finish();
  for (const [name, animation] of [["r1", r1], ["r2", r2], ["p1", p1], ["p2", p2]]) animation.onremove = (event) => removal.push([name, event.type, num(event.currentTime), rel(event.timelineTime), animation.replaceState]);
  await frame();
  await frame();
  removal.push(["before", r1.replaceState, r2.replaceState, p1.replaceState, p2.replaceState, getComputedStyle(t).marginLeft]);
  r2.finish();
  await frame();
  await frame();
  removal.push(["after", r1.replaceState, r2.replaceState, p1.replaceState, p2.replaceState, getComputedStyle(t).marginLeft, getComputedStyle(t).marginTop, t.getAnimations().length]);
  r1.commitStyles();
  removal.push(["commit-removed", t.style.marginLeft]);
  const c = t.animate({ opacity: [0.2, 0.8], transform: ["translateX(0px)", "translateX(100px)"], color: ["rgb(0, 0, 0)", "rgb(100, 200, 50)"] }, 100);
  c.pause();
  c.currentTime = 50;
  c.commitStyles();
  removal.push(["commit", t.style.opacity, t.style.color, t.style.marginLeft]);
  for (const animation of [r1, r2, p1, p2, c]) animation.cancel();
  t.removeAttribute("style");
  await frame();
  out.replacement = removal;

  // getAnimations relevance, order and scoping.
  out.getAnimations = (() => {
    document.body.insertAdjacentHTML("beforeend", "<div id=p><div id=c></div></div><div id=h></div>");
    const p = document.getElementById("p"), child = document.getElementById("c"), host = document.getElementById("h");
    const shadow = host.attachShadow({ mode: "open" });
    shadow.innerHTML = "<span id=s></span>";
    const s = shadow.getElementById("s");
    const before = [document.getAnimations().length, p.getAnimations().length];
    const a1 = child.animate(null, { duration: 100, id: "a1" }), a2 = p.animate(null, { duration: 100, id: "a2" }), a3 = s.animate(null, { duration: 100, id: "a3" });
    const a4 = p.animate(null, { duration: 100, id: "a4", fill: "forwards" }); a4.finish();
    const a5 = p.animate(null, { duration: 100, id: "a5" }); a5.finish();
    const a6 = p.animate(null, { duration: 100, id: "a6" }); a6.pause();
    const a7 = p.animate(null, { duration: 100, id: "a7", delay: 50 }); a7.playbackRate = -1;
    const a8 = new Animation(new KeyframeEffect(p, null, 100)); a8.id = "a8";
    const a9 = p.animate(null, { duration: 100, id: "a9" }); a9.cancel();
    const detached = document.createElement("div"); const a10 = detached.animate(null, { duration: 100, id: "a10" });
    const a11 = p.animate(null, { duration: 100, id: "a11", pseudoElement: "::before" });
    const a12 = p.animate(null, { duration: 100, id: "a12", delay: 30 });
    const ids = (list) => list.map((animation) => animation.id);
    const result = { before, document: ids(document.getAnimations()), p: ids(p.getAnimations()), subtree: ids(p.getAnimations({ subtree: true })), child: ids(child.getAnimations()), shadow: ids(shadow.getAnimations()), host: ids(host.getAnimations({ subtree: true })), detached: ids(detached.getAnimations()), array: Array.isArray(p.getAnimations()), fresh: p.getAnimations() !== p.getAnimations() };
    a2.effect.target = child;
    result.retargeted = { p: ids(p.getAnimations()), child: ids(child.getAnimations()) };
    for (const animation of [a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12]) animation.cancel();
    result.cancelled = document.getAnimations().length;
    p.remove();
    host.remove();
    return result;
  })();
  return deep(out);
})()

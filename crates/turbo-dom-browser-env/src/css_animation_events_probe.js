(async () => {
  // CSS transition and animation events (turbo-dom#313) on Chrome 156's live frame clock.
  // Durations are tens of seconds and every step seeks (or pauses) the CSS animations it
  // drives, so each event's phase and elapsedTime are exact. `frame N` rows mark each
  // rendering frame's animation-frame callbacks: the events before one were dispatched in that
  // frame. Within a frame the events are listed by target (each target's own order kept):
  // across targets Chrome dispatches in its animation-update order, which is not observable
  // through the DOM.
  const log = [];
  const $ = (id) => document.getElementById(id);
  const style = (element) => getComputedStyle(element).opacity + getComputedStyle(element).marginLeft;
  const handles = new Map();
  const name = (animation) => (animation ? (handles.get(animation) ?? "other") : null);
  const types = ["transitionrun", "transitionstart", "transitionend", "transitioncancel", "animationstart", "animationiteration", "animationend", "animationcancel"];
  let pending = [];
  for (const type of types)
    $("wrap").addEventListener(type, (event) => {
      pending.push([
        type,
        event.target.id,
        Object.prototype.toString.call(event),
        event.constructor.name,
        "propertyName" in event ? event.propertyName : event.animationName,
        +event.elapsedTime.toFixed(6),
        event.pseudoElement,
        name(event.animation),
        event.isTrusted,
        event.bubbles,
        event.cancelable,
        event.composed,
        event.eventPhase,
        event.currentTarget.id,
        event.defaultPrevented,
      ]);
    });
  // An event handler attribute and an own listener see the same event.
  $("a").onanimationiteration = (event) => pending.push([`on${event.type}`, event.target.id]);
  $("d").ontransitionend = (event) => pending.push([`on${event.type}`, event.target.id]);
  let frameCount = 0;
  const flush = () => {
    const rows = pending;
    pending = [];
    rows.sort((a, b) => (a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0));
    log.push(...rows);
  };
  const frame = () => new Promise((resolve) => requestAnimationFrame(() => { flush(); log.push(["frame", ++frameCount]); resolve(); }));
  const frames = async (count) => { for (let i = 0; i < count; i++) await frame(); };
  const step = async (label, action) => { flush(); log.push(["step", label]); action(); await frames(3); };
  const kept = new Map();
  const own = (id) => {
    const list = $(id).getAnimations();
    for (const animation of list) handles.set(animation, `${id}:${animation.constructor.name}`);
    if (list[0]) kept.set(id, list[0]);
    return list[0];
  };
  const on = (ids) => {
    for (const id of ids) $(id).classList.add("on");
    for (const id of ids) style($(id));
    for (const id of ids) own(id);
  };

  for (const id of ["d", "c", "n", "s", "h", "r", "v", "b", "a", "i", "e", "g", "x", "m", "z", "y", "w"]) style($(id));
  await frames(2);
  // CSS animations start with the style that names them.
  await step("animations", () => on(["a", "i", "e", "g", "x", "m", "z", "y", "w"]));
  // Transitions start on a transitioned property's computed-value change.
  await step("transitions", () => {
    on(["d", "c", "n", "s", "h", "r", "v", "b"]);
    own("b").pause();
    own("b").currentTime = 5000;
    own("c").pause();
    own("c").currentTime = 2500;
    own("h").pause();
    own("h").currentTime = 10000;
    own("v").pause();
    own("v").currentTime = 5000;
  });
  // A delayed transition reaches its active phase, then its end.
  await step("d active", () => { own("d").currentTime = 15000; });
  await step("d end", () => { own("d").currentTime = 25000; });
  // Skipping the active phase: start and end in one frame.
  await step("s skip", () => { own("s").currentTime = 30000; });
  // Removing the transition (transition: none) cancels it; elapsedTime is its active time.
  await step("c cancel", () => { $("c").classList.add("off"); style($("c")); });
  // display:none cancels a running transition.
  await step("h hidden", () => { $("h").classList.add("gone"); style($("h")); });
  // Animation.cancel() on a CSSTransition.
  await step("r cancel", () => { own("r").pause(); own("r").currentTime = 5000; own("r").cancel(); });
  // Reversing a running transition cancels it and runs the reverse one.
  await step("v reverse", () => { $("v").classList.remove("on"); style($("v")); own("v"); });
  // Iterations, then the end.
  await step("a iteration", () => { own("a").currentTime = 15000; });
  await step("a second iteration", () => { own("a").currentTime = 25000; });
  await step("a end", () => { own("a").currentTime = 35000; });
  // Backwards seeks across an iteration boundary.
  await step("m forward", () => { own("m").currentTime = 15000; });
  await step("m back", () => { own("m").currentTime = 5000; });
  // A delayed animation skipped straight past its end.
  await step("e skip", () => { own("e").currentTime = 50000; });
  // animation: none cancels; elapsedTime is the paused active time.
  await step("i cancel", () => { own("i").pause(); own("i").currentTime = 22500; $("i").classList.add("off"); style($("i")); });
  // display:none cancels; finish() ends.
  await step("x hidden", () => { own("x").pause(); own("x").currentTime = 5000; $("x").classList.add("gone"); style($("x")); });
  await step("g finish", () => { own("g").finish(); });
  // Cancelling a delayed animation in its active and its before phase.
  await step("y cancel active", () => { own("y").pause(); own("y").currentTime = 15000; $("y").classList.add("off"); style($("y")); });
  await step("w cancel before", () => { own("w").pause(); own("w").currentTime = 5000; $("w").classList.add("off"); style($("w")); });
  // Cancelling a transition in its before phase.
  await step("b cancel before", () => { $("b").classList.add("off"); style($("b")); });
  // Script cancel() of a finished transition and a finished animation.
  await step("finished cancel", () => { kept.get("d").cancel(); kept.get("a").cancel(); });
  flush();
  log.push(["states", ["a", "e", "g", "m", "z"].map((id) => $(id).getAnimations().map((animation) => animation.playState))]);
  // The event interfaces: constructor defaults, init members, conversions and brands.
  const outcome = (fn) => { try { return ["ok", fn()]; } catch (error) { return [error.name, error.message]; } };
  const fields = (event) => [event.type, event.animationName, event.propertyName, event.elapsedTime, event.pseudoElement, event.animation, event.pseudoTarget, event.bubbles, event.cancelable, event.composed, event.isTrusted];
  const interfaces = {};
  for (const [Type, member] of [[AnimationEvent, "animationName"], [TransitionEvent, "propertyName"]]) {
    const getter = (key) => Object.getOwnPropertyDescriptor(Type.prototype, key).get;
    interfaces[Type.name] = {
      length: Type.length,
      keys: Reflect.ownKeys(Type.prototype).map(String),
      parent: Object.getPrototypeOf(Type) === Event && Object.getPrototypeOf(Type.prototype) === Event.prototype,
      empty: outcome(() => fields(new Type("x"))),
      init: outcome(() => fields(new Type("y", { [member]: 7, elapsedTime: "2.5", pseudoElement: null, bubbles: 1, cancelable: true, composed: true, animation: null }))),
      animation: outcome(() => { const animation = new Animation(); return new Type("x", { animation }).animation === animation; }),
      notAnimation: outcome(() => new Type("x", { animation: {} })),
      // Dictionary members are read in code-unit order, the inherited EventInit ones first.
      order: outcome(() => {
        const read = [], init = {};
        for (const key of ["animation", member, "bubbles", "cancelable", "composed", "elapsedTime", "pseudoElement"]) Object.defineProperty(init, key, { get() { read.push(key); return key === "animation" ? null : key === "elapsedTime" ? 1 : "v"; } });
        new Type("x", init);
        return read;
      }),
      call: outcome(() => Type("x")),
      none: outcome(() => new Type()),
      nan: outcome(() => new Type("x", { elapsedTime: NaN })),
      infinite: outcome(() => new Type("x", { elapsedTime: Infinity })),
      notObject: outcome(() => new Type("x", 5)),
      brand: outcome(() => getter(member).call(new Event("x"))),
      elapsedBrand: outcome(() => getter("elapsedTime").call(new (Type === AnimationEvent ? TransitionEvent : AnimationEvent)("x"))),
      string: Object.prototype.toString.call(new Type("x")),
    };
  }
  return { events: log, interfaces };
})()

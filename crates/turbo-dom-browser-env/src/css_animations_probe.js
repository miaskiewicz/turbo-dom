(async () => {
  // Declarative CSS animations and transitions in getAnimations() (turbo-dom#187), captured with
  // the document timeline frozen (CDP Animation.setPlaybackRate 0).
  const out = {};
  const $ = (id) => document.getElementById(id);
  const num = (value) => (typeof value === "number" && Number.isFinite(value) ? +value.toPrecision(12) : typeof value === "number" ? String(value) : value);
  const timing = (effect) => {
    const t = effect.getTiming();
    return [t.delay, t.endDelay, num(t.iterations), t.direction, t.fill, t.easing, num(t.duration), t.iterationStart];
  };
  const row = (animation) => [
    Object.prototype.toString.call(animation),
    animation.constructor.name,
    animation instanceof Animation,
    "animationName" in animation ? animation.animationName : null,
    "transitionProperty" in animation ? animation.transitionProperty : null,
    animation.playState,
    // CSS animations start on the frame before the freeze in Chrome, so only the sign is stable.
    animation.currentTime === null ? null : animation.currentTime >= 0,
    animation.effect.constructor.name,
    animation.effect.target === null ? null : animation.effect.target.id,
    timing(animation.effect),
  ];
  // Blink lists a keyframe's properties in its PropertyHandleSet's hash order, which depends on
  // the set's insertion history (turbo-dom#233): the dictionary members lead, computedOffset
  // trails, and the property members are compared as a sorted set.
  const keyframes = (animation) =>
    animation.effect.getKeyframes().map((frame) => {
      const keys = Object.keys(frame);
      const properties = keys.slice(3, -1).sort();
      return [...keys.slice(0, 3), ...properties, keys.at(-1)].map((key) => [key, num(frame[key])]);
    });
  for (const id of ["a", "m", "n", "p"]) getComputedStyle($(id)).opacity;
  out.a = $("a").getAnimations().map(row);
  out.aKeyframes = $("a").getAnimations().map(keyframes);
  out.m = $("m").getAnimations().map(row);
  out.mKeyframes = $("m").getAnimations().map(keyframes);
  out.n = $("n").getAnimations().map(row);
  out.p = $("p").getAnimations().map(row);
  // Transitions: a computed-value change of a transitioned property after a style flush.
  getComputedStyle($("t")).opacity;
  getComputedStyle($("u")).opacity;
  $("t").style.opacity = "0";
  $("u").style.opacity = "0.5";
  $("u").style.color = "green";
  out.t = $("t").getAnimations().map(row);
  out.tKeyframes = $("t").getAnimations().map(keyframes);
  out.u = $("u").getAnimations().map(row);
  out.document = document.getAnimations().map((animation) => [animation.constructor.name, animation.effect.target.id, animation.animationName ?? animation.transitionProperty]);
  // Script animations sort after CSS ones in document.getAnimations().
  $("p").animate({ opacity: [0, 1] }, 1000);
  out.documentWithScript = document.getAnimations().map((animation) => animation.constructor.name);
  // Interfaces.
  out.shape = ["CSSAnimation", "CSSTransition"].map((name) => [
    typeof globalThis[name],
    Object.getPrototypeOf(globalThis[name]) === Animation,
    Reflect.ownKeys(globalThis[name].prototype).map(String),
    (() => { try { new globalThis[name](); return "constructed"; } catch (error) { return [error.name, error.message]; } })(),
  ]);
  // playState basics: pause/cancel a CSS animation.
  const [first] = $("a").getAnimations();
  first.pause();
  out.paused = [first.playState, first.currentTime >= 0];
  first.cancel();
  out.cancelled = [first.playState, first.currentTime, $("a").getAnimations().length];
  // Removing the declaration cancels the animation.
  const [mFirst] = $("m").getAnimations();
  $("m").style.animation = "none";
  out.removed = [mFirst.playState, $("m").getAnimations().length];
  return out;
})()

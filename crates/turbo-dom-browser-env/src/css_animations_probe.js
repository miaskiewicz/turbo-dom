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
  // Blink lists a keyframe's properties in its property hash map's bucket order (turbo-dom#233).
  const keyframes = (animation) => animation.effect.getKeyframes().map((frame) => Object.keys(frame).map((key) => [key, num(frame[key])]));
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
  // transition-property: all transitions every changed animatable property.
  getComputedStyle($("w")).opacity;
  $("w").style.opacity = "0";
  $("w").style.marginLeft = "5px";
  out.w = $("w").getAnimations().map(row);
  out.wKeyframes = $("w").getAnimations().map(keyframes);
  // No CSS animation runs under a display:none ancestor; it starts once the ancestor renders.
  out.h = $("h").getAnimations().map(row);
  $("hp").style.display = "block";
  out.hShown = $("h").getAnimations().map(row);
  $("hp").style.display = "none";
  out.hHidden = $("h").getAnimations().length;
  // Keyframe property order (turbo-dom#233): WTF hash map bucket order over CSSPropertyID, for
  // script keyframes (inserted in code-unit order) and @keyframes (inserted in declaration order).
  const vals = { color: "red", opacity: "0", transform: "none", marginLeft: "1px", width: "1px", backgroundColor: "red", left: "1px", top: "1px", zIndex: "1", height: "2px", fontSize: "3px", paddingTop: "1px", borderTopColor: "red", filter: "none", lineHeight: "2", translate: "1px", rotate: "1deg", scale: "2", visibility: "hidden", outlineColor: "red", letterSpacing: "1px", margin: "2px", right: "1px", bottom: "1px", maskImage: "none", maskComposite: "add", wordSpacing: "1px", fontWeight: "700" };
  const scriptOrder = (names) => { const frame = {}; for (const name of names) frame[name] = vals[name]; return Object.keys($("p").animate([frame, frame], 1000).effect.getKeyframes()[0]); };
  out.order = {
    small: scriptOrder(["opacity", "color", "transform"]),
    pair: scriptOrder(["outlineColor", "fontSize"]),
    collide: scriptOrder(["right", "bottom", "outlineColor", "fontSize"]),
    shorthand: scriptOrder(["marginLeft", "margin", "color", "opacity"]),
    big: scriptOrder(Object.keys(vals)),
    css: $("x").getAnimations().map((animation) => animation.effect.getKeyframes().map((frame) => Object.keys(frame))),
  };
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
  // Round 2 (#11): a kept CSS animation follows its own animation-* properties unless script
  // overrode them (Blink's ignored-properties model); @keyframes edits reach its keyframes.
  const extra = document.createElement("style");
  extra.textContent = "@keyframes kk { from { opacity: 0 } to { opacity: 1 } } .anim { animation: k 10s } .kk { animation: kk 10s } .tr { opacity: 1; color: red; margin-left: 0; transition: opacity 2s, color 2s, margin-left 2s }";
  document.head.append(extra);
  const make = (id, cls, parent = document.body) => { const el = document.createElement("div"); el.id = id; el.className = cls; parent.append(el); getComputedStyle(el).opacity; return el; };
  const one = (el) => el.getAnimations().map(row);
  const ps = make("ps", "anim"), [psA] = ps.getAnimations();
  ps.style.animationPlayState = "paused";
  out.playPaused = [one(ps), ps.getAnimations()[0] === psA];
  ps.style.animationPlayState = "running";
  out.playRunning = [one(ps), ps.getAnimations()[0] === psA];
  const dd = make("dd", "anim"), [ddA] = dd.getAnimations();
  dd.style.animationDuration = "3s";
  out.duration = [one(dd), dd.getAnimations()[0] === ddA];
  dd.style.animationDelay = "-1s";
  dd.style.animationIterationCount = "2.5";
  dd.style.animationDirection = "alternate-reverse";
  dd.style.animationFillMode = "forwards";
  dd.style.animationTimingFunction = "steps(2)";
  out.timing = [one(dd), dd.getAnimations()[0] === ddA, ddA.effect.getKeyframes().map((frame) => frame.easing)];
  // Script-set play state and timing win over later CSS changes.
  const so = make("so", "anim"), [soA] = so.getAnimations();
  soA.pause();
  so.style.animationPlayState = "running";
  out.scriptPaused = one(so);
  soA.play();
  so.style.animationPlayState = "paused";
  out.scriptPlayed = one(so);
  soA.effect.updateTiming({ duration: 5000, direction: "reverse" });
  so.style.animationDuration = "7s";
  so.style.animationDirection = "alternate";
  so.style.animationDelay = "2s";
  out.scriptTiming = [one(so), so.getAnimations()[0] === soA];
  // @keyframes content changes reach the kept animation, unless script set its keyframes.
  const kf = make("kf", "kk"), [kfA] = kf.getAnimations();
  const kfs = make("kfs", "kk"), [kfsA] = kfs.getAnimations();
  kfsA.effect.setKeyframes([{ opacity: 0.25 }, { opacity: 0.75 }]);
  extra.textContent = extra.textContent.replace("@keyframes kk { from { opacity: 0 } to { opacity: 1 } }", "@keyframes kk { from { opacity: 0.2 } to { opacity: 0.8; margin-left: 3px } }");
  out.keyframesChanged = [kf.getAnimations().map(keyframes), kf.getAnimations()[0] === kfA, kfs.getAnimations().map(keyframes), kfs.getAnimations()[0] === kfsA];
  // #12: a change while display:none starts no transition; becoming display:none (itself or an
  // ancestor) cancels running transitions.
  const td = make("td", "tr");
  td.style.display = "none";
  td.style.opacity = "0";
  out.hiddenChange = td.getAnimations().length;
  td.style.display = "block";
  out.shownAfterHiddenChange = td.getAnimations().length;
  td.style.opacity = "0.5";
  const [tdT] = td.getAnimations();
  out.shownChange = [one(td), td.getAnimations().map(keyframes)];
  td.style.display = "none";
  out.hiddenCancels = [td.getAnimations().length, tdT.playState];
  td.style.display = "";
  out.reshown = td.getAnimations().length;
  const tpar = make("tpar", ""), tc = make("tc", "tr", tpar);
  tc.style.opacity = "0";
  const [tcT] = tc.getAnimations();
  tpar.style.display = "none";
  out.ancestorHiddenCancels = [tc.getAnimations().length, tcT.playState];
  tpar.style.display = "";
  tc.style.opacity = "0.4";
  out.ancestorReshownChange = [one(tc), tc.getAnimations().map(keyframes)];
  // #13: transitions compare computed values, not the authored text.
  const tn = make("tn", "tr");
  tn.style.color = "#f00";
  tn.style.marginLeft = "0px";
  tn.style.opacity = "1.0";
  out.equalComputed = tn.getAnimations().length;
  tn.style.color = "rgb(255, 0, 0)";
  tn.style.marginLeft = "0em";
  out.equalComputedAgain = tn.getAnimations().length;
  tn.style.color = "blue";
  out.colorChange = [one(tn), tn.getAnimations().map(keyframes)];
  return out;
})()

(async () => {
  // turbo-dom MediaMode::Jsdom: jsdom 27.4.0 has no Web Animations and runs no CSS
  // transitions or @keyframes animations: getComputedStyle reports the declared value at
  // once and no transition / animation events fire. Chrome runs both.
  document.head.insertAdjacentHTML("beforeend", "<style>@keyframes k { from { color: rgb(1, 2, 3) } to { color: rgb(1, 2, 3) } } #a { color: rgb(9, 9, 9); animation: k 10s linear } #t { color: rgb(0, 0, 0); transition: color 1000s linear } #t.on { color: rgb(0, 0, 200) } #e { color: rgb(0, 0, 0); transition: color 50ms linear } #e.on { color: rgb(0, 0, 100) }</style>");
  document.body.innerHTML = '<div id="a"></div><div id="t"></div><div id="e"></div>';
  const events = [];
  for (const name of ["transitionrun", "transitionstart", "transitionend", "transitioncancel", "animationstart", "animationiteration", "animationend", "animationcancel"]) document.addEventListener(name, (event) => events.push([name, event.target.id]));
  const a = document.getElementById("a"), t = document.getElementById("t"), e = document.getElementById("e");
  const out = {};
  out.interfaces = ["Animation", "AnimationEffect", "AnimationTimeline", "DocumentTimeline", "KeyframeEffect", "AnimationPlaybackEvent", "CSSAnimation", "CSSTransition", "AnimationEvent", "TransitionEvent"].map((name) => name in window);
  out.members = ["animate" in Element.prototype, "getAnimations" in Element.prototype, "getAnimations" in Document.prototype, "timeline" in Document.prototype, "getAnimations" in ShadowRoot.prototype];
  out.animated = getComputedStyle(a).color;
  out.before = [getComputedStyle(t).color, getComputedStyle(e).color];
  t.className = "on";
  e.className = "on";
  out.after = [getComputedStyle(t).color, getComputedStyle(e).color];
  out.animations = typeof document.getAnimations === "function" ? document.getAnimations().map((animation) => [animation.constructor.name, animation.effect.target.id]) : null;
  await new Promise((resolve) => setTimeout(resolve, 300));
  out.later = [getComputedStyle(t).color, getComputedStyle(e).color];
  out.events = events;
  return out;
})()

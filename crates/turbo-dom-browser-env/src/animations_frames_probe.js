(async () => {
  // Live document timeline: only relations that hold for any frame cadence are recorded;
  // time comparisons allow Blink's 1e-13 seconds-to-milliseconds rounding noise.
  const log = [];
  const t = document.getElementById("t");
  const close = (a, b) => typeof a === "number" && typeof b === "number" && Math.abs(a - b) < 1e-6;
  const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
  const task = () => new Promise((resolve) => setTimeout(resolve, 0));

  const baseline = getComputedStyle(t).marginLeft;
  log.push(["no-css-animations", document.getAnimations().length, t.getAnimations().length, document.body.getAnimations({ subtree: true }).length]);
  const stamp = await frame();
  log.push(["raf-is-timeline", close(stamp, document.timeline.currentTime), typeof document.timeline.currentTime]);
  const first = document.timeline.currentTime;
  await frame();
  log.push(["timeline-advances", document.timeline.currentTime > first]);
  const origin = new DocumentTimeline({ originTime: 100 });
  log.push(["origin-time", close(origin.currentTime, document.timeline.currentTime - 100)]);

  // Created in a task: pending through the next frame's callbacks, started at that frame's time.
  await task();
  const a = t.animate({ opacity: [0, 1] }, 200);
  log.push(["created", a.pending, a.startTime, a.currentTime, a.playState]);
  let frameTime = null;
  requestAnimationFrame((time) => { frameTime = time; log.push(["first-callback", a.pending, a.startTime]); });
  await a.ready;
  log.push(["ready", a.pending, close(a.startTime, frameTime), close(a.startTime, document.timeline.currentTime), a.currentTime]);
  await frame();
  await frame();
  const progressed = a.currentTime;
  log.push(["progress", progressed > 0, close(progressed, document.timeline.currentTime - a.startTime), Math.abs(parseFloat(getComputedStyle(t).opacity) - progressed / 200) < 1e-5]);
  a.playbackRate = 2;
  log.push(["rate-keeps-time", close(a.currentTime, progressed), close(a.currentTime, (document.timeline.currentTime - a.startTime) * 2)]);

  // Finished promise reactions run before the finish event, both before that frame's callbacks.
  const order = [];
  // Callbacks before the finish depend on frame cadence; record the order from the finish on.
  const note = (entry) => { if (entry !== "callback" || (order.length && order[order.length - 1] !== "callback")) order.push(entry); };
  let watching = true;
  const watch = () => { if (!watching) return; note("callback"); requestAnimationFrame(watch); };
  requestAnimationFrame(watch);
  a.finished.then((value) => { note("finished:" + (value === a)); Promise.resolve().then(() => note("finished-microtask")); });
  a.onfinish = (event) => {
    note(["event", event.currentTime, close(event.timelineTime, document.timeline.currentTime), event.isTrusted, a.playState].join(":"));
    Promise.resolve().then(() => note("event-microtask"));
    requestAnimationFrame(() => note("callback-from-event"));
  };
  await a.finished;
  await frame();
  await frame();
  watching = false;
  log.push(["finish-order", order]);
  log.push(["finished", a.playState, a.currentTime, getComputedStyle(t).opacity, a.pending]);

  // Cancel events carry the timeline time of the cancel, dispatched on a later frame.
  const b = t.animate({ opacity: [0, 1] }, 1000);
  await b.ready;
  const cancelled = document.timeline.currentTime;
  const cancel = new Promise((resolve) => { b.oncancel = (event) => resolve([event.currentTime, close(event.timelineTime, cancelled), close(event.timelineTime, document.timeline.currentTime)]); });
  b.cancel();
  log.push(["cancel", await cancel, b.playState]);

  // Pausing in flight: the pause commits on the next frame and then holds.
  const c = t.animate({ opacity: [0, 1] }, 1000);
  await c.ready;
  await frame();
  c.pause();
  const pausing = c.currentTime;
  log.push(["pause-pending", c.pending, c.playState, close(pausing, document.timeline.currentTime - c.startTime)]);
  await c.ready;
  const held = c.currentTime;
  log.push(["paused", c.pending, c.startTime, held >= pausing]);
  await frame();
  await frame();
  log.push(["held", c.currentTime === held, Math.abs(parseFloat(getComputedStyle(t).opacity) - held / 1000) < 1e-5]);
  c.play();
  await c.ready;
  log.push(["resumed", close(c.currentTime, held), close(c.startTime, document.timeline.currentTime - held)]);
  c.finish();
  log.push(["finish-sync", c.playState, c.currentTime, c.pending]);

  // updatePlaybackRate and reverse apply at the next frame, keeping the current time.
  const d = t.animate({ opacity: [0, 1] }, 1000);
  await d.ready;
  await frame();
  d.updatePlaybackRate(-1);
  log.push(["update-pending", d.pending, d.playbackRate]);
  await d.ready;
  log.push(["updated", d.playbackRate, close(d.currentTime, (document.timeline.currentTime - d.startTime) * -1)]);
  d.reverse();
  await d.ready;
  log.push(["reversed", d.playbackRate, close(d.currentTime, document.timeline.currentTime - d.startTime)]);
  d.cancel();

  // A finished filling animation is removed once a later finished one replaces it.
  const r1 = t.animate({ marginLeft: ["0px", "10px"] }, { duration: 30, fill: "forwards" });
  const r2 = t.animate({ marginLeft: ["0px", "20px"] }, { duration: 60, fill: "forwards" });
  const removed = new Promise((resolve) => { r1.onremove = (event) => resolve([event.type, event.currentTime, r2.playState, r1.replaceState]); });
  log.push(["remove", await removed, getComputedStyle(t).marginLeft, t.getAnimations().length, document.getAnimations().length]);
  r2.cancel();
  log.push(["cleared", getComputedStyle(t).marginLeft === baseline, document.getAnimations().length]);
  return log;
})()

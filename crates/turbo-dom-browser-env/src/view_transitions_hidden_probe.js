(async () => {
  const error = (value) => [value.name, value.message, value instanceof DOMException];
  const settle = async (promise) => {
    try {
      await promise;
      return "ok";
    } catch (caught) {
      return error(caught);
    }
  };
  const outcome = async (transition) => ({
    ucd: await settle(transition.updateCallbackDone),
    ready: await settle(transition.ready),
    finished: await settle(transition.finished),
  });
  const log = [];
  const transition = document.startViewTransition(() => log.push("callback"));
  const sync = [
    document.visibilityState,
    document.activeViewTransition,
    document.documentElement.matches(":active-view-transition"),
  ];
  Promise.resolve().then(() => log.push("microtask"));
  setTimeout(() => log.push("task"));
  const documentOutcome = await outcome(transition);
  const element = document.body.appendChild(document.createElement("div"));
  const scoped = element.startViewTransition(() => log.push("element-callback"));
  const elementSync = [element.activeViewTransition, element.matches(":active-view-transition")];
  const elementOutcome = await outcome(scoped);
  await new Promise((resolve) => setTimeout(resolve, 10));
  element.remove();
  return { sync, documentOutcome, elementSync, elementOutcome, log };
})();

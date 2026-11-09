(async () => {
  // turbo-dom#450: observe how Chrome 156 handles `defaultPrevented` when a microtask
  // queued inside an AT_TARGET listener synchronously re-dispatches the SAME event
  // object on the SAME target, and the inner listener calls `event.preventDefault()`.
  //
  // Two scenarios:
  //   (A) listenerQueuesMicrotask: outer listener queues a microtask that re-dispatches
  //       the event. The inner listener calls preventDefault() (no outer listener does).
  //       Records: outer dispatchEvent return, outer event.defaultPrevented (after await),
  //       inner dispatchEvent return, inner event.defaultPrevented (immediately in micro).
  //   (B) listenerPreventsThenMicrotask: a slight variant — outer listener queues the
  //       microtask BEFORE a sibling listener that calls preventDefault(). This checks
  //       whether the microtask's own observations of defaultPrevented are affected by
  //       the outer's subsequent preventDefault.
  //   (C) listenerMicrotaskPreventsFirst: same as (A), but the microtask calls
  //       preventDefault BEFORE calling dispatchEvent — tests whether Chrome exposes a
  //       different value on the subsequent inner dispatch.
  //
  // Each scenario uses a fresh plain EventTarget (not a Node) and a fresh Event.
  const results = {};

  // Scenario A ---------------------------------------------------------------
  {
    const target = new EventTarget();
    let step = 0;
    const observed = {};
    const microDone = new Promise((resolve) => {
      target.addEventListener('e', (event) => {
        step++;
        if (step === 1) {
          // Outer dispatch: schedule re-entry.
          observed.outerListenerDefaultPreventedWhenFiring = event.defaultPrevented;
          queueMicrotask(() => {
            observed.microstartDefaultPrevented = event.defaultPrevented;
            observed.innerReturn = target.dispatchEvent(event);
            observed.innerDefaultPreventedAfter = event.defaultPrevented;
            resolve();
          });
        } else if (step === 2) {
          // Inner dispatch: cancel it.
          observed.innerListenerDefaultPreventedBeforePrevent = event.defaultPrevented;
          event.preventDefault();
          observed.innerListenerDefaultPreventedAfterPrevent = event.defaultPrevented;
        }
      });
    });
    const event = new Event('e', { cancelable: true, bubbles: false });
    observed.outerReturn = target.dispatchEvent(event);
    observed.outerDefaultPreventedImmediatelyAfterOuter = event.defaultPrevented;
    await microDone;
    observed.outerDefaultPreventedAfterMicrotasksDrained = event.defaultPrevented;
    results.listenerQueuesMicrotask = observed;
  }

  // Scenario B ---------------------------------------------------------------
  {
    const target = new EventTarget();
    let step = 0;
    const observed = {};
    const microDone = new Promise((resolve) => {
      // Two outer listeners: first queues a microtask, second calls preventDefault on outer.
      target.addEventListener('e', (event) => {
        step++;
        if (step === 1) {
          queueMicrotask(() => {
            observed.microstartDefaultPrevented = event.defaultPrevented;
            observed.innerReturn = target.dispatchEvent(event);
            observed.innerDefaultPreventedAfter = event.defaultPrevented;
            resolve();
          });
        } else if (step >= 2) {
          // Inner dispatch: call preventDefault on inner.
          event.preventDefault();
        }
      });
      // Second outer listener: cancels on OUTER only (step guard).
      target.addEventListener('e', (event) => {
        if (step === 1) {
          event.preventDefault();
          observed.outerListenerPreventedDefaultPrevented = event.defaultPrevented;
        }
      });
    });
    const event = new Event('e', { cancelable: true, bubbles: false });
    observed.outerReturn = target.dispatchEvent(event);
    observed.outerDefaultPreventedImmediatelyAfterOuter = event.defaultPrevented;
    await microDone;
    observed.outerDefaultPreventedAfterMicrotasksDrained = event.defaultPrevented;
    results.outerListenerAlsoPrevents = observed;
  }

  // Scenario C ---------------------------------------------------------------
  // Same shape as scenario A, but the microtask calls event.preventDefault()
  // BEFORE the inner dispatchEvent. Confirms whether the inner's return and
  // defaultPrevented reflect that preexisting state across dispatches.
  {
    const target = new EventTarget();
    let step = 0;
    const observed = {};
    const microDone = new Promise((resolve) => {
      target.addEventListener('e', (event) => {
        step++;
        if (step === 1) {
          queueMicrotask(() => {
            // Prevent BEFORE re-dispatch.
            event.preventDefault();
            observed.microstartDefaultPreventedAfterPreemptiveCall = event.defaultPrevented;
            observed.innerReturn = target.dispatchEvent(event);
            observed.innerDefaultPreventedAfter = event.defaultPrevented;
            resolve();
          });
        }
      });
    });
    const event = new Event('e', { cancelable: true, bubbles: false });
    observed.outerReturn = target.dispatchEvent(event);
    observed.outerDefaultPreventedImmediatelyAfterOuter = event.defaultPrevented;
    await microDone;
    observed.outerDefaultPreventedAfterMicrotasksDrained = event.defaultPrevented;
    results.microtaskPreventsBeforeRedispatch = observed;
  }

  return results;
})()

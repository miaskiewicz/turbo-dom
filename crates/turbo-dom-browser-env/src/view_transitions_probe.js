(async () => {
  const out = {};
  // A host may run the scenarios in several evaluations; Chrome runs them all at once.
  const sections = globalThis.viewTransitionProbeSections;
  const wanted = (name) => !Array.isArray(sections) || sections.includes(name);
  const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
  const task = () => new Promise((resolve) => setTimeout(resolve));
  // Start every scenario from a task right after a rendering update, so the next
  // frame is a full interval away and same-frame races cannot reorder the log.
  const fresh = async () => {
    await frame();
    await task();
  };
  const error = (value) =>
    value && typeof value === "object"
      ? [value.name, value.message, value instanceof DOMException]
      : String(value);
  const attempt = (callback) => {
    try {
      const value = callback();
      return value === undefined ? "undefined" : value;
    } catch (caught) {
      return error(caught);
    }
  };
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
  // Outcome of a transition whose cross-fade timing is not under test: skip it once ready
  // settles, so every scenario does not spend a full animation.
  const quick = async (transition) => {
    const ucd = await settle(transition.updateCallbackDone);
    const ready = await settle(transition.ready);
    transition.skipTransition();
    return { ucd, ready, finished: await settle(transition.finished) };
  };
  const describe = (object, key) => {
    const descriptor = Object.getOwnPropertyDescriptor(object, key);
    if (!descriptor) return null;
    const result = [];
    for (const field of ["writable", "enumerable", "configurable"])
      if (field in descriptor) result.push(field[0] + descriptor[field]);
    for (const field of ["value", "get", "set"])
      if (typeof descriptor[field] === "function")
        result.push(field, descriptor[field].name, descriptor[field].length);
      else if (field in descriptor && descriptor[field] !== undefined)
        result.push(field, String(descriptor[field]));
    return result;
  };
  const root = document.documentElement;
  const matches = (element, selector) => attempt(() => element.matches(selector));
  const state = () =>
    (document.activeViewTransition ? "A" : "-") +
    (root.matches(":active-view-transition") ? "M" : "-");

  // Interface objects and member descriptors.
  if (wanted("surface")) {
    const surface = {};
    for (const [name, proto] of [
      ["Document", Document.prototype],
      ["Element", Element.prototype],
      ["HTMLElement", HTMLElement.prototype],
    ])
      for (const key of ["startViewTransition", "activeViewTransition"])
        surface[`${name}.${key}`] = describe(proto, key);
    for (const [name, Interface] of [
      ["ViewTransition", ViewTransition],
      ["ViewTransitionTypeSet", ViewTransitionTypeSet],
    ]) {
      const proto = Interface.prototype;
      surface[name] = [
        Interface.name,
        Interface.length,
        Object.getOwnPropertyNames(Interface),
        describe(window, name),
        describe(Interface, "prototype"),
        Object.getPrototypeOf(proto) === Object.prototype,
        Object.getOwnPropertyNames(proto),
        Object.getOwnPropertySymbols(proto).map(String),
        Object.getOwnPropertyNames(proto).map((key) => describe(proto, key)),
        describe(proto, Symbol.toStringTag),
        attempt(() => new Interface()),
        attempt(() => Interface()),
      ];
    }
    const types = ViewTransitionTypeSet.prototype;
    surface.typeSetAliases = [
      types[Symbol.iterator] === types.values,
      types.keys === types.values,
      describe(types, Symbol.iterator),
    ];
    surface.globals = Object.getOwnPropertyNames(window)
      .filter((key) => /^ViewTransition/.test(key))
      .sort();
    out.surface = surface;
  }

  // Brand checks: promise attributes reject instead of throwing.
  if (wanted("brands")) {
    const VT = ViewTransition.prototype,
      TS = ViewTransitionTypeSet.prototype;
    const getter = (proto, key) => Object.getOwnPropertyDescriptor(proto, key).get;
    const brands = {};
    for (const key of ["updateCallbackDone", "ready", "finished"]) {
      const promise = attempt(() => getter(VT, key).call({}));
      brands[key] = [promise instanceof Promise, await settle(promise)];
    }
    brands.types = attempt(() => getter(VT, "types").call(document));
    brands.transitionRoot = attempt(() => getter(VT, "transitionRoot").call({}));
    brands.skipTransition = attempt(() => VT.skipTransition.call({}));
    brands.waitUntil = attempt(() => VT.waitUntil.call({}, 1));
    brands.documentStart = attempt(() =>
      Document.prototype.startViewTransition.call(document.body),
    );
    brands.elementStart = attempt(() => Element.prototype.startViewTransition.call(document));
    brands.documentActive = attempt(() =>
      getter(Document.prototype, "activeViewTransition").call(document.body),
    );
    brands.elementActive = attempt(() =>
      getter(Element.prototype, "activeViewTransition").call(document),
    );
    brands.size = attempt(() => getter(TS, "size").call(new Set()));
    for (const key of ["add", "clear", "delete", "entries", "forEach", "has", "keys", "values"])
      brands[key] = attempt(() => TS[key].call(new Set(), "a"));
    out.brands = brands;
  }

  // Argument conversion: (ViewTransitionUpdateCallback or ViewTransitionOptions).
  if (wanted("conversion")) {
    await fresh();
    const conversion = {};
    const started = (owner, argument) =>
      attempt(() => {
        const transition = owner.startViewTransition(argument);
        transition.skipTransition();
        transition.ready.catch(() => {});
        return [...transition.types];
      });
    const cases = {
      none: [],
      undefined: [undefined],
      null: [null],
      number: [5],
      string: ["x"],
      boolean: [true],
      symbol: [Symbol("s")],
      object: [{}],
      array: [[]],
      typesString: [{ types: "ab" }],
      typesNull: [{ types: null }],
      typesNumber: [{ types: 3 }],
      typesSymbol: [{ types: [Symbol("t")] }],
      updateNumber: [{ update: 5 }],
      updateNull: [{ update: null }],
      updateObject: [{ update: {} }],
    };
    for (const [name, args] of Object.entries(cases)) {
      conversion[name] = attempt(() => {
        const transition = document.startViewTransition(...args);
        transition.skipTransition();
        transition.ready.catch(() => {});
        return [...transition.types];
      });
    }
    const element = document.body.appendChild(document.createElement("div"));
    conversion.elementNumber = started(element, 5);
    conversion.elementTypes = started(element, { types: "ab" });
    element.remove();
    const reads = [];
    conversion.order = attempt(() => {
      const transition = document.startViewTransition({
        get update() {
          reads.push("update");
          return () => {};
        },
        get types() {
          reads.push("types");
          return ["b", "a", "b"];
        },
        get extra() {
          reads.push("extra");
          return 1;
        },
      });
      transition.skipTransition();
      transition.ready.catch(() => {});
      return [...transition.types];
    });
    conversion.reads = reads;
    conversion.iterable = attempt(() => {
      const transition = document.startViewTransition({
        types: {
          *[Symbol.iterator]() {
            yield 1;
            yield {
              toString() {
                return "z";
              },
            };
          },
        },
      });
      transition.skipTransition();
      transition.ready.catch(() => {});
      return [...transition.types];
    });
    out.conversion = conversion;
  }

  // Lifecycle against rendering frames. Consecutive identical frame entries
  // collapse, so the record is independent of how many frames the cross-fade spans.
  async function trace(start) {
    await fresh();
    const log = [];
    let running = true,
      readyAt = -1,
      firstAfterReady = null,
      finishedFrame = null,
      previousFrame = null,
      lastFrame = null,
      framesDuringCallback = 0,
      inCallback = false;
    const tick = (timestamp) => {
      previousFrame = lastFrame;
      lastFrame = timestamp;
      if (readyAt >= 0 && firstAfterReady === null) firstAfterReady = timestamp;
      if (inCallback) framesDuringCallback++;
      const entry = `frame${state()}`;
      if (log[log.length - 1] !== entry) log.push(entry);
      if (running) requestAnimationFrame(tick);
    };
    // Queued before the start and the frame it requests, so the task stays ahead of that
    // frame however long the start itself takes; it still observes the post-start state.
    setTimeout(() => log.push(`task${state()}`));
    const transition = start(
      () => {
        log.push(`callback${state()}`);
      },
      (value) => {
        inCallback = value;
      },
    );
    log.push(`started${state()}`);
    Promise.resolve().then(() => log.push(`microtask${state()}`));
    requestAnimationFrame(tick);
    const watch = (promise, name, after) =>
      promise.then(
        () => {
          log.push(`${name}${state()}`);
          if (after) after();
        },
        (caught) => log.push(`${name}:${caught.name}${state()}`),
      );
    watch(transition.updateCallbackDone, "ucd");
    watch(transition.ready, "ready", () => {
      readyAt = log.length;
    });
    await watch(transition.finished, "finished", () => {
      finishedFrame = [previousFrame, lastFrame];
    });
    log.push(`after${state()}`);
    await frame();
    running = false;
    await frame();
    const timing =
      firstAfterReady === null || finishedFrame === null
        ? null
        : [finishedFrame[1] - firstAfterReady >= 249, finishedFrame[0] - firstAfterReady < 251];
    return { log, timing, framesDuringCallback };
  }
  if (wanted("lifecycle")) {
    const lifecycle = {};
    lifecycle.basic = await trace((callback) => document.startViewTransition(callback));
    lifecycle.noCallback = await trace(() => document.startViewTransition());
    lifecycle.options = await trace((callback) =>
      document.startViewTransition({ update: callback, types: ["a"] }),
    );
    lifecycle.asyncCallback = await trace((callback, mark) =>
      document.startViewTransition(async () => {
        callback();
        mark(true);
        for (let i = 0; i < 3; i++) await task();
        await new Promise((resolve) => setTimeout(resolve, 40));
        mark(false);
      }),
    );
    lifecycle.skipImmediately = await trace((callback) => {
      const transition = document.startViewTransition(callback);
      transition.skipTransition();
      return transition;
    });
    lifecycle.throws = await trace((callback) =>
      document.startViewTransition(() => {
        callback();
        throw new TypeError("boom");
      }),
    );
    lifecycle.rejects = await trace((callback) =>
      document.startViewTransition(() => {
        callback();
        return Promise.reject(new RangeError("nope"));
      }),
    );
    lifecycle.element = await trace((callback) => {
      const element = document.body.appendChild(document.createElement("div"));
      element.textContent = "scoped";
      const transition = element.startViewTransition(callback);
      transition.finished.then(() => element.remove());
      return transition;
    });
    out.lifecycle = lifecycle;
  }

  // Frames requested from inside the update callback run once rendering resumes.
  if (wanted("frameFromCallback")) {
    await fresh();
    const log = [];
    const transition = document.startViewTransition(() => {
      log.push("callback");
      requestAnimationFrame(() => log.push("frame-from-callback"));
    });
    transition.updateCallbackDone.then(() => log.push("ucd"));
    transition.ready.then(() => log.push("ready"));
    await transition.finished;
    out.frameFromCallback = log;
  }

  // Values, identity and transition roots.
  if (wanted("values")) {
    await fresh();
    const transition = document.startViewTransition({ types: ["x"] });
    out.values = [
      document.activeViewTransition === transition,
      root.activeViewTransition === transition,
      document.body.activeViewTransition,
      transition.transitionRoot === root,
      transition.types === transition.types,
      transition.updateCallbackDone === transition.updateCallbackDone,
      transition.ready === transition.ready,
      transition.finished === transition.finished,
      Object.prototype.toString.call(transition),
      Object.prototype.toString.call(transition.types),
      transition.skipTransition(),
    ];
    await quick(transition);
    const viaRoot = root.startViewTransition();
    out.rootElement = [
      document.activeViewTransition === viaRoot,
      root.activeViewTransition === viaRoot,
      viaRoot.transitionRoot === root,
      await quick(viaRoot),
      document.activeViewTransition,
    ];
  }

  // Skipping, by phase.
  if (wanted("skips")) {
    const skips = {};
    for (const when of ["beforeCallback", "inCallback", "afterUcd", "afterReady"]) {
      await fresh();
      const log = [];
      let transition = null;
      transition = document.startViewTransition(() => {
        log.push("callback");
        if (when === "inCallback") {
          transition.skipTransition();
          log.push(`skipped${state()}`);
        }
      });
      if (when === "beforeCallback") {
        await task();
        transition.skipTransition();
        log.push(`skipped${state()}`);
      }
      if (when === "afterUcd") {
        await transition.updateCallbackDone;
        transition.skipTransition();
        log.push(`skipped${state()}`);
      }
      if (when === "afterReady") {
        await transition.ready;
        transition.skipTransition();
        log.push(`skipped${state()}`, attempt(() => transition.skipTransition()));
      }
      skips[when] = { log, outcome: await outcome(transition) };
    }
    out.skips = skips;
  }

  // A new transition skips the active one in every phase.
  if (wanted("abort")) {
    const abort = {};
    await fresh();
    {
      const calls = [];
      const first = document.startViewTransition(() => calls.push("first"));
      const second = document.startViewTransition(() => calls.push("second"));
      abort.pendingCapture = {
        active: document.activeViewTransition === second,
        first: await quick(first),
        second: await quick(second),
        calls,
        after: document.activeViewTransition,
      };
    }
    await fresh();
    {
      const log = [];
      let release;
      const first = document.startViewTransition(() => {
        log.push("first-callback");
        return new Promise((resolve) => {
          release = resolve;
        });
      });
      first.updateCallbackDone.then(() => log.push("first-ucd"));
      first.ready.catch((caught) => log.push(`first-ready:${caught.name}`));
      first.finished.then(() => log.push("first-finished"));
      while (!release) await task();
      const second = document.startViewTransition(() => log.push("second-callback"));
      log.push("second-started");
      await second.ready;
      log.push("second-ready");
      release();
      await first.finished;
      log.push("first-done");
      await second.finished;
      abort.pendingCallback = log;
    }
    await fresh();
    {
      const log = [];
      const first = document.startViewTransition();
      await first.ready;
      first.finished.then(() => log.push("first-finished"));
      const second = document.startViewTransition(() => log.push("second-callback"));
      log.push("second-started");
      second.ready.then(() => log.push("second-ready"));
      await second.finished;
      log.push("second-finished");
      abort.animating = log;
    }
    out.abort = abort;
  }

  // Placement of ready's rejection and of the update callback once a transition is
  // skipped before its callback ran: ready rejects one microtask later, the callback runs
  // in a task queued at the skip.
  if (wanted("skipOrder")) {
    const order = {};
    const watch = async (name, start) => {
      await fresh();
      const log = [];
      const transition = start(() => log.push("callback"));
      Promise.resolve()
        .then(() => log.push("m1"))
        .then(() => log.push("m2"))
        .then(() => log.push("m3"))
        .then(() => log.push("m4"))
        .then(() => log.push("m5"));
      transition.updateCallbackDone.then(() => log.push("ucd"));
      transition.ready.catch(() => log.push("ready"));
      transition.finished.then(() => log.push("finished"));
      setTimeout(() => log.push("task"));
      await new Promise((resolve) => setTimeout(resolve, 10));
      order[name] = log;
    };
    await watch("skipTransition", (callback) => {
      const transition = document.startViewTransition(callback);
      transition.skipTransition();
      return transition;
    });
    await watch("inertDocument", (callback) =>
      document.implementation.createHTMLDocument("").startViewTransition(callback),
    );
    await watch("detachedElement", (callback) =>
      document.createElement("div").startViewTransition(callback),
    );
    let replacement = null;
    await watch("replaced", (callback) => {
      const transition = document.startViewTransition(callback);
      replacement = document.startViewTransition();
      return transition;
    });
    await quick(replacement);
    out.skipOrder = order;
  }

  // Style rules: :active-view-transition in the cascade.
  if (wanted("cascade")) {
    await fresh();
    const style = document.head.appendChild(document.createElement("style"));
    const marker = document.body.appendChild(document.createElement("div"));
    marker.id = "typed";
    style.textContent =
      ":root:active-view-transition body { color: rgb(1, 2, 3) } :root:active-view-transition-type(fade) #typed { color: rgb(4, 5, 6) }";
    const read = () => [getComputedStyle(document.body).color, getComputedStyle(marker).color];
    const before = read();
    const transition = document.startViewTransition({ types: ["fade"] });
    const during = read();
    transition.types.delete("fade");
    const untyped = read();
    await transition.finished;
    out.cascade = [before, during, untyped, read()];
    style.remove();
    marker.remove();
  }

  // Update callback results.
  if (wanted("results")) {
    const results = {};
    await fresh();
    results.nonPromise = await outcome(document.startViewTransition(() => 42));
    results.thenable = await outcome(
      document.startViewTransition(() => ({
        then(resolve) {
          setTimeout(resolve, 5);
        },
      })),
    );
    results.rejectsLater = await outcome(
      document.startViewTransition(
        () => new Promise((_, reject) => setTimeout(() => reject(new SyntaxError("late")), 5)),
      ),
    );
    {
      const started = performance.now();
      const transition = document.startViewTransition(() => new Promise(() => {}));
      const ready = await settle(transition.ready);
      const pending = (promise) =>
        Promise.race([
          settle(promise),
          new Promise((resolve) => setTimeout(() => resolve("pending"), 50)),
        ]);
      results.timeout = [
        ready,
        Math.round((performance.now() - started) / 1000),
        await pending(transition.updateCallbackDone),
        await pending(transition.finished),
        document.activeViewTransition,
      ];
    }
    out.results = results;
  }

  // Documents without a browsing context.
  if (wanted("documents")) {
    const documents = {};
    const factories = {
      createHTMLDocument: () => document.implementation.createHTMLDocument(""),
      createDocument: () => document.implementation.createDocument(null, "x"),
      createEmptyDocument: () => document.implementation.createDocument(null, null),
      xhtml: () =>
        document.implementation.createDocument("http://www.w3.org/1999/xhtml", "html"),
      domParser: () => new DOMParser().parseFromString("<p>", "text/html"),
      parseHTMLUnsafe: () => Document.parseHTMLUnsafe("<p>"),
      constructed: () => new Document(),
    };
    for (const [name, create] of Object.entries(factories)) {
      await fresh();
      const inert = attempt(create),
        log = [];
      if (!inert || typeof inert.startViewTransition !== "function") {
        documents[name] = ["unavailable", inert];
        continue;
      }
      const transition = attempt(() =>
        inert.startViewTransition(() => log.push(`callback:${inert.activeViewTransition}`)),
      );
      if (transition && typeof transition === "object" && "ready" in transition) {
        const sync = [
          inert.activeViewTransition,
          document.activeViewTransition,
          transition.transitionRoot === inert.documentElement,
          inert.documentElement ? inert.documentElement.matches(":active-view-transition") : null,
        ];
        Promise.resolve().then(() => log.push("microtask"));
        documents[name] = { sync, outcome: await quick(transition), log };
      } else documents[name] = transition === null ? "null" : transition;
    }
    await fresh();
    {
      const main = document.startViewTransition();
      const inert = document.implementation.createHTMLDocument("");
      const other = inert.startViewTransition();
      documents.alongsideMain = [
        document.activeViewTransition === main,
        await quick(other),
        await quick(main),
      ];
    }
    out.documents = documents;
  }

  // Element-scoped transitions: which boxes can be a transition root.
  if (wanted("scopes")) {
    const scopes = {};
    const make = (tag, style) => {
      const element = document.body.appendChild(document.createElement(tag));
      if (style) element.setAttribute("style", style);
      if (!["img", "input", "canvas", "textarea"].includes(tag)) element.textContent = "x";
      return element;
    };
    const cases = {
      div: () => make("div"),
      span: () => make("span"),
      displayNone: () => make("div", "display: none"),
      contents: () => make("div", "display: contents"),
      hiddenAttribute: () => {
        const element = make("div");
        element.hidden = true;
        return element;
      },
      insideDisplayNone: () =>
        make("div", "display: none").appendChild(document.createElement("div")),
      visibilityHidden: () => make("div", "visibility: hidden"),
      inlineBlock: () => make("span", "display: inline-block"),
      inlineFlex: () => make("span", "display: inline-flex"),
      listItem: () => make("span", "display: list-item"),
      tableRow: () => make("div", "display: table-row"),
      tableCell: () => make("div", "display: table-cell"),
      tableRowGroup: () => make("div", "display: table-row-group"),
      tableColumn: () => make("div", "display: table-column"),
      tableCaption: () => make("div", "display: table-caption"),
      flexChildSpan: () =>
        make("div", "display: flex").appendChild(document.createElement("span")),
      ruby: () => make("span", "display: ruby"),
      details: () => make("details"),
      summaryInClosedDetails: () =>
        make("details").appendChild(document.createElement("summary")),
      contentInClosedDetails: () =>
        make("details").appendChild(document.createElement("div")),
      closedDialog: () => make("dialog"),
      head: () => document.head,
      table: () => make("div", "display: table"),
      absoluteSpan: () => make("span", "position: absolute"),
      floatSpan: () => make("span", "float: left"),
      img: () => make("img"),
      input: () => make("input"),
      button: () => make("button"),
      canvas: () => make("canvas"),
      textarea: () => make("textarea"),
      anchor: () => make("a"),
      svg: () =>
        document.body.appendChild(document.createElementNS("http://www.w3.org/2000/svg", "svg")),
      detached: () => document.createElement("div"),
      body: () => document.body,
    };
    for (const [name, create] of Object.entries(cases)) {
      await fresh();
      const element = create(),
        log = [];
      const transition = element.startViewTransition(() =>
        log.push(`callback:${element.activeViewTransition !== null}`),
      );
      const sync = [
        element.activeViewTransition === transition,
        document.activeViewTransition,
        transition.transitionRoot === element,
        element.matches(":active-view-transition"),
        root.matches(":active-view-transition"),
      ];
      scopes[name] = { sync, outcome: await quick(transition), log };
      if (element !== document.body && element !== document.head) {
        let top = element;
        while (top.parentNode && top.parentNode !== document.body) top = top.parentNode;
        top.remove();
      }
    }
    out.scopes = scopes;
  }

  // Element-scoped concurrency and removal.
  if (wanted("scoped")) {
    const scoped = {};
    const make = () => {
      const element = document.body.appendChild(document.createElement("div"));
      element.textContent = "x";
      return element;
    };
    await fresh();
    {
      const a = make(),
        b = make(),
        inner = a.appendChild(document.createElement("div"));
      inner.textContent = "inner";
      const first = a.startViewTransition(),
        other = b.startViewTransition(),
        whole = document.startViewTransition();
      const sync = [
        a.activeViewTransition === first,
        b.activeViewTransition === other,
        document.activeViewTransition === whole,
      ];
      const second = a.startViewTransition(),
        nested = inner.startViewTransition();
      scoped.concurrent = {
        sync,
        replaced: [a.activeViewTransition === second, inner.activeViewTransition === nested],
        first: await quick(first),
        other: await quick(other),
        whole: await quick(whole),
        second: await quick(second),
        nested: await quick(nested),
      };
      a.remove();
      b.remove();
    }
    await fresh();
    {
      const a = make();
      const whole = document.startViewTransition(),
        part = a.startViewTransition();
      scoped.documentThenElement = [await quick(whole), await quick(part)];
      a.remove();
    }
    await fresh();
    {
      const a = make();
      const part = a.startViewTransition(),
        whole = document.startViewTransition();
      scoped.elementThenDocument = [await quick(whole), await quick(part)];
      a.remove();
    }
    await fresh();
    {
      const a = make();
      const transition = a.startViewTransition();
      a.remove();
      scoped.removedSync = [a.activeViewTransition === transition, await quick(transition)];
    }
    await fresh();
    {
      const a = make();
      const transition = a.startViewTransition(() => {});
      await transition.updateCallbackDone;
      a.remove();
      scoped.removedAfterCallback = await quick(transition);
    }
    await fresh();
    {
      const a = make();
      const transition = a.startViewTransition(() => {});
      await transition.ready;
      a.remove();
      scoped.removedAnimating = [a.activeViewTransition === transition, await outcome(transition)];
    }
    await fresh();
    {
      const a = make();
      const transition = a.startViewTransition({ types: ["t"] });
      scoped.types = [
        a.matches(":active-view-transition-type(t)"),
        root.matches(":active-view-transition-type(t)"),
        document.querySelectorAll(":active-view-transition").length,
        document.querySelector(":active-view-transition") === a,
      ];
      await quick(transition);
      a.remove();
    }
    await fresh();
    {
      const log = [];
      let running = true;
      const tick = () => {
        log.push("frame");
        if (running) requestAnimationFrame(tick);
      };
      const a = make();
      const transition = a.startViewTransition(async () => {
        log.push("callback");
        requestAnimationFrame(tick);
        await new Promise((resolve) => setTimeout(resolve, 60));
        log.push("callback-done");
      });
      await transition.updateCallbackDone;
      running = false;
      await quick(transition);
      scoped.renderingDuringCallback = log.indexOf("frame") < log.indexOf("callback-done");
      a.remove();
    }
    out.scoped = scoped;
  }

  // view-transition-name uniqueness in the old and the new state.
  if (wanted("names")) {
    const names = {};
    const host = document.body.appendChild(document.createElement("div"));
    const run = async (markup, update) => {
      await fresh();
      host.innerHTML = markup;
      const log = [];
      const transition = document.startViewTransition(() => {
        log.push("callback");
        if (update) update();
      });
      return { outcome: await quick(transition), log };
    };
    const dup = 'style="view-transition-name: dup"';
    names.old = await run(`<div ${dup}></div><div ${dup}></div>`);
    names.unique = await run(
      '<div style="view-transition-name: one"></div><div style="view-transition-name: two"></div>',
    );
    names.newState = await run(`<div ${dup}></div><div id=late></div>`, () =>
      host.querySelector("#late").setAttribute("style", "view-transition-name: dup"),
    );
    names.removedInCallback = await run(`<div ${dup}></div><div ${dup} id=gone></div>`, () =>
      host.querySelector("#gone").remove(),
    );
    names.renamedInCallback = await run(
      '<div style="view-transition-name: a"></div><div style="view-transition-name: b" id=b></div>',
      () => host.querySelector("#b").setAttribute("style", "view-transition-name: a"),
    );
    names.displayNone = await run(
      `<div ${dup}></div><div style="view-transition-name: dup; display: none"></div>`,
    );
    names.insideDisplayNone = await run(
      `<div ${dup}></div><div style="display: none"><div ${dup}></div></div>`,
    );
    names.hiddenAttribute = await run(`<div ${dup}></div><div ${dup} hidden></div>`);
    names.visibilityHidden = await run(
      `<div ${dup}></div><div style="view-transition-name: dup; visibility: hidden"></div>`,
    );
    names.root = await run('<div style="view-transition-name: root"></div>');
    names.none = await run(
      '<div style="view-transition-name: none"></div><div style="view-transition-name: none"></div>',
    );
    names.auto = await run(
      '<div style="view-transition-name: auto"></div><div style="view-transition-name: auto"></div>',
    );
    names.stylesheet = await run(
      "<style>.n { view-transition-name: sheet }</style><div class=n></div><div class=n></div>",
    );
    names.span = await run(`<span ${dup}>a</span><span ${dup}>b</span>`);
    names.contents = await run(
      `<div ${dup}></div><div style="view-transition-name: dup; display: contents">c</div>`,
    );
    names.closedDetails = await run(`<div ${dup}></div><details><div ${dup}>d</div></details>`);
    {
      await fresh();
      host.innerHTML = `<div ${dup}></div><div id=shadow-host></div>`;
      const shadow = host.querySelector("#shadow-host").attachShadow({ mode: "open" });
      shadow.innerHTML = `<div ${dup}>s</div>`;
      names.shadow = await quick(document.startViewTransition());
      await fresh();
      host.innerHTML = "<div id=shadow-host></div>";
      const inner = host.querySelector("#shadow-host").attachShadow({ mode: "open" });
      inner.innerHTML = `<div ${dup}>s</div><div ${dup}>t</div>`;
      names.shadowPair = await quick(document.startViewTransition());
    }
    host.innerHTML = "";
    await fresh();
    {
      const scope = host.appendChild(document.createElement("div"));
      scope.setAttribute("style", "view-transition-name: q");
      scope.appendChild(document.createElement("div")).setAttribute("style", "view-transition-name: q");
      names.scopeAndChild = await quick(scope.startViewTransition());
      host.innerHTML = "";
    }
    await fresh();
    {
      const scope = host.appendChild(document.createElement("div"));
      scope.textContent = "x";
      scope.appendChild(document.createElement("div")).setAttribute("style", "view-transition-name: q");
      const outside = host.appendChild(document.createElement("div"));
      outside.setAttribute("style", "view-transition-name: q");
      names.outsideScope = await quick(scope.startViewTransition());
      host.innerHTML = "";
    }
    host.remove();
    out.names = names;
  }

  // ViewTransitionTypeSet and the :active-view-transition pseudo-classes.
  if (wanted("selectors")) {
    await fresh();
    const selectors = {};
    const transition = document.startViewTransition({ types: ["Slide", "fade"] });
    const types = transition.types;
    selectors.sync = [
      matches(root, ":active-view-transition"),
      matches(root, ":active-view-transition-type(Slide)"),
      matches(root, ":active-view-transition-type(slide)"),
      matches(root, ":active-view-transition-type(fade, x)"),
      matches(root, ":active-view-transition-type(x)"),
      matches(document.body, ":active-view-transition"),
      document.querySelectorAll(":active-view-transition").length,
      matches(root, ":root:active-view-transition-type(fade)"),
      matches(root, ":not(:active-view-transition)"),
      matches(root, ":is(:active-view-transition-type(fade))"),
      document.body.closest(":active-view-transition") === root,
      document.querySelector(":active-view-transition-type(fade)") === root,
    ];
    selectors.syntax = [
      ":active-view-transition-type()",
      ":active-view-transition-type(*)",
      ':active-view-transition-type("fade")',
      ":active-view-transition()",
      ":active-view-transition-type(1)",
      ":active-view-transition-type(a b)",
      ":active-view-transition-type(a,)",
    ].map((selector) => matches(root, selector));
    selectors.mutation = [
      attempt(() => types.add("later")),
      types.size,
      matches(root, ":active-view-transition-type(later)"),
      types.has("later"),
      types.delete("later"),
      types.delete("later"),
      matches(root, ":active-view-transition-type(later)"),
      attempt(() => types.add(5)),
      types.has("5"),
      attempt(() => types.add(Symbol("s"))),
      attempt(() => types.has()),
      attempt(() => types.add()),
      attempt(() => types.delete()),
      [...types],
      [...types.keys()],
      [...types.entries()].map(String),
    ];
    const calls = [];
    const receiver = {};
    types.forEach(function (value, key, owner) {
      calls.push([value, key, owner === types, this === receiver]);
    }, receiver);
    selectors.forEach = [calls, attempt(() => types.forEach()), attempt(() => types.forEach(1))];
    const iterator = types.values(),
      iteratorProto = Object.getPrototypeOf(iterator);
    selectors.iterator = [
      Object.prototype.toString.call(iterator),
      Object.getOwnPropertyNames(iteratorProto),
      describe(iteratorProto, "next"),
      describe(iteratorProto, Symbol.toStringTag),
      iterator[Symbol.iterator]() === iterator,
      iterator.next(),
    ];
    types.add("tail");
    selectors.iterator.push(iterator.next(), iterator.next(), iterator.next(), iterator.next());
    await transition.ready;
    selectors.ready = [
      matches(root, ":active-view-transition"),
      matches(root, ":active-view-transition-type(fade)"),
    ];
    types.clear();
    selectors.cleared = [
      types.size,
      matches(root, ":active-view-transition-type(fade)"),
      matches(root, ":active-view-transition"),
    ];
    types.add("fade");
    await transition.finished;
    selectors.finished = [
      matches(root, ":active-view-transition"),
      matches(root, ":active-view-transition-type(fade)"),
      [...transition.types],
    ];
    transition.types.add("post");
    selectors.afterFinish = [...transition.types];
    await fresh();
    {
      const skipped = document.startViewTransition({ types: ["gone"] });
      skipped.skipTransition();
      selectors.skipped = [
        matches(root, ":active-view-transition"),
        matches(root, ":active-view-transition-type(gone)"),
        [...skipped.types],
      ];
      await outcome(skipped);
    }
    out.selectors = selectors;
  }

  // waitUntil extends the transition until every promise settles.
  if (wanted("waitUntil")) {
    const wait = {};
    await fresh();
    {
      const transition = document.startViewTransition();
      wait.noArgument = attempt(() => transition.waitUntil());
      wait.number = attempt(() => transition.waitUntil(5));
      let release;
      wait.returned = attempt(() =>
        transition.waitUntil(
          new Promise((resolve) => {
            release = resolve;
          }),
        ),
      );
      const log = [];
      let frames = 0,
        counting = false;
      const tick = () => {
        if (counting) frames++;
        if (log.length < 2) requestAnimationFrame(tick);
      };
      transition.finished.then(() => log.push(`finished:${frames >= 20}`));
      await transition.ready;
      counting = true;
      requestAnimationFrame(tick);
      while (frames < 25) await frame();
      log.push(`still-active:${document.activeViewTransition === transition}`);
      log.push(`matches:${root.matches(":active-view-transition")}`);
      release();
      await transition.finished;
      log.push(`after:${document.activeViewTransition}`);
      wait.held = log;
      wait.afterFinished = attempt(() => transition.waitUntil(Promise.resolve()));
    }
    await fresh();
    {
      const transition = document.startViewTransition();
      transition.waitUntil(Promise.reject(new Error("wait")));
      wait.rejected = await outcome(transition);
    }
    await fresh();
    {
      const transition = document.startViewTransition();
      await transition.ready;
      transition.waitUntil(new Promise(() => {}));
      transition.skipTransition();
      wait.skipped = [document.activeViewTransition, await settle(transition.finished)];
    }
    await fresh();
    {
      const transition = document.startViewTransition();
      transition.skipTransition();
      wait.afterSkip = [attempt(() => transition.waitUntil(new Promise(() => {}))), await outcome(transition)];
    }
    out.waitUntil = wait;
  }

  // Unhandled rejections are reported for the transition's promises.
  if (wanted("unhandled")) {
    await fresh();
    const reported = [];
    const listener = (event) => {
      reported.push(event.reason && event.reason.name);
      event.preventDefault();
    };
    window.addEventListener("unhandledrejection", listener);
    document.startViewTransition().skipTransition();
    await new Promise((resolve) => setTimeout(resolve, 50));
    document.startViewTransition(() => {
      throw new SyntaxError("s");
    });
    await new Promise((resolve) => setTimeout(resolve, 400));
    window.removeEventListener("unhandledrejection", listener);
    out.unhandled = reported;
  }
  return out;
})();

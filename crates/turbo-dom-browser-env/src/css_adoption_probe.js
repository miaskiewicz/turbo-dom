(() => {
  const first = new CSSStyleSheet();
  first.replaceSync(".a {width:20px}");
  const second = new CSSStyleSheet();
  second.replaceSync(".a {width:40px}");
  const before = document.adoptedStyleSheets;
  const source = [first];
  document.adoptedStyleSheets = source;
  const assigned = document.adoptedStyleSheets;
  source.push(second);
  const detachedSource = assigned.length;
  document.adoptedStyleSheets = [second];
  const retained = {
    sameBefore: assigned === before,
    sameAfter: assigned === document.adoptedStyleSheets,
    detachedSource,
    retainedContent: assigned[0] === first,
    currentContent: document.adoptedStyleSheets[0] === second,
  };
  const active = document.adoptedStyleSheets;
  active.push(first);
  const pushed = {
    length: document.adoptedStyleSheets.length,
    first: document.adoptedStyleSheets[0] === second,
    second: document.adoptedStyleSheets[1] === first,
    same: active === document.adoptedStyleSheets,
    array: Array.isArray(active),
    prototype: Object.getPrototypeOf(active) === Array.prototype,
    keys: Reflect.ownKeys(active).map(String),
    lengthDescriptor: Object.getOwnPropertyDescriptor(active, "length"),
    indexDescriptor: (() => {
      const value = Object.getOwnPropertyDescriptor(active, "0");
      return {
        writable: value.writable,
        enumerable: value.enumerable,
        configurable: value.configurable,
      };
    })(),
  };
  const errors = {};
  const fail = (name, callback) => {
    try {
      callback();
      errors[name] = "accepted";
    } catch (error) {
      errors[name] = error.name;
    }
  };
  const style = document.createElement("style");
  style.textContent = ".ordinary {width:3px}";
  document.head.append(style);
  fail("ordinary", () => {
    document.adoptedStyleSheets = [style.sheet];
  });
  fail("forged", () => {
    document.adoptedStyleSheets = [{}];
  });
  fail("null", () => {
    document.adoptedStyleSheets = null;
  });
  fail("nonIterable", () => {
    document.adoptedStyleSheets = { 0: first, length: 1 };
  });
  fail("invalidPush", () => active.push({}));
  fail("invalidIndex", () => {
    active[0] = {};
  });
  fail("deleteIndex", () => {
    delete active[0];
  });
  fail("lengthGrow", () => {
    active.length = 10;
  });
  fail("accessorIndex", () =>
    Object.defineProperty(active, "0", {
      get() {
        return first;
      },
    }),
  );
  const afterErrors = {
    length: active.length,
    values: Array.from(active, (sheet) =>
      sheet === first ? "first" : sheet === second ? "second" : "other",
    ),
  };
  document.adoptedStyleSheets = [first];
  const indexCases = {};
  const values = () =>
    Array.from(active, (sheet) =>
      sheet === first ? "first" : sheet === second ? "second" : "other",
    );
  const attempt = (name, fn) => {
    try {
      fn();
      indexCases[name] = { result: "accepted", values: values() };
    } catch (error) {
      indexCases[name] = { result: error.name, values: values() };
    }
  };
  attempt("gap", () => {
    active[3] = second;
  });
  attempt("append", () => {
    active[1] = second;
  });
  attempt("defineValue", () => Object.defineProperty(active, "0", { value: second }));
  attempt("defineReadonly", () =>
    Object.defineProperty(active, "0", { value: first, writable: false }),
  );
  attempt("deleteTail", () => {
    delete active[1];
  });
  attempt("invalidLength", () => {
    active.length = -1;
  });
  document.adoptedStyleSheets = [first];
  attempt("nonconstructedSequence", () => {
    document.adoptedStyleSheets = [second, style.sheet];
  });
  document.adoptedStyleSheets = [first];
  attempt("invalidSequence", () => {
    document.adoptedStyleSheets = [second, {}];
  });
  active.length = 0;
  document.adoptedStyleSheets = [];
  style.remove();
  return { retained, pushed, errors, afterErrors, indexCases };
})();

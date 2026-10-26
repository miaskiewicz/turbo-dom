(async () => {
  const result = {},
    attempt = (fn) => {
      try {
        const value = fn();
        return value === undefined ? "undefined" : value;
      } catch (error) {
        return error.name;
      }
    };
  const names = [];
  const options = {
    get customElementRegistry() {
      names.push("registry");
      return customElements;
    },
    get is() {
      names.push("is");
      return "x-idl";
    },
  };
  result.invalidCreate = attempt(() =>
    document.createElement(
      {
        toString() {
          names.push("name");
          return "!!";
        },
      },
      options,
    ),
  );
  result.createReads = names;
  const reads = [],
    host = document.createElement("div");
  const shadowOptions = {
    get clonable() {
      reads.push("clonable");
      return false;
    },
    get customElementRegistry() {
      reads.push("registry");
      return {};
    },
    get delegatesFocus() {
      reads.push("delegatesFocus");
      return false;
    },
    get mode() {
      reads.push("mode");
      return "open";
    },
    get serializable() {
      reads.push("serializable");
      return false;
    },
    get slotAssignment() {
      reads.push("slotAssignment");
      return "named";
    },
  };
  result.invalidShadow = attempt(() => host.attachShadow(shadowOptions));
  result.shadowReads = reads;
  result.atomicShadow = host.shadowRoot === null;
  result.createOptions = [];
  for (const options of [
    true,
    0,
    "x-idl",
    null,
    { customElementRegistry: null },
    { customElementRegistry: {} },
    { customElementRegistry: customElements },
  ])
    result.createOptions.push(attempt(() => document.createElement("div", options).localName));
  result.brands = [];
  for (const [proto, name] of [
    [Document.prototype, "customElementRegistry"],
    [ShadowRoot.prototype, "customElementRegistry"],
  ]) {
    const get = Object.getOwnPropertyDescriptor(proto, name).get;
    result.brands.push([
      attempt(() => get.call({})),
      attempt(() => get.call(document.createElement("div"))),
    ]);
  }
  let nested = [];
  class Define extends HTMLElement {}
  Object.defineProperty(Define.prototype, "connectedCallback", {
    get() {
      nested.push(attempt(() => customElements.define("x-nested", class extends HTMLElement {})));
      throw new SyntaxError("getter");
    },
  });
  result.reentrant = attempt(() => customElements.define("x-outer", Define));
  result.reentrantAgain = attempt(() =>
    customElements.define("x-outer", class extends HTMLElement {}),
  );
  result.nested = nested;
  const sequence = [];
  class Order extends HTMLElement {
    static get observedAttributes() {
      sequence.push("observed");
      return {
        get [Symbol.iterator]() {
          sequence.push("iterator");
          return function* () {
            sequence.push("yield");
            yield {
              toString() {
                sequence.push("string");
                return "a";
              },
            };
          };
        },
      };
    }
    attributeChangedCallback() {}
  }
  result.sequence = attempt(() => customElements.define("x-order", Order));
  result.sequenceReads = sequence;
  result.constructorBrands = [
    attempt(() => new HTMLUnknownElement()),
    attempt(() => HTMLElement.call({})),
    attempt(() => Reflect.construct(HTMLElement, [], function () {})),
  ];
  return result;
})();

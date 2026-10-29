(() => {
  const rows = [];
  for (const associated of [false, true]) {
    const log = [];
    class X extends HTMLElement {}
    for (const key of [
      "connectedCallback",
      "disconnectedCallback",
      "connectedMoveCallback",
      "adoptedCallback",
      "attributeChangedCallback",
      "formAssociatedCallback",
      "formDisabledCallback",
      "formResetCallback",
      "formStateRestoreCallback",
    ])
      Object.defineProperty(X.prototype, key, {
        get() {
          log.push(key);
          return () => {};
        },
      });
    for (const key of ["observedAttributes", "disabledFeatures", "formAssociated"])
      Object.defineProperty(X, key, {
        get() {
          log.push(key);
          return key === "formAssociated" ? associated : [];
        },
      });
    customElements.define("x-order-" + associated, X);
    rows.push(log);
  }
  return rows;
})();

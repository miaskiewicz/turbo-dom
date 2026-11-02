(async () => {
  if (!globalThis.child) {
    const frame = document.createElement("iframe");
    document.body.append(frame);
    globalThis.child = frame.contentWindow;
  }
  const result = {},
    attempt = (fn) => {
      try {
        const value = fn();
        return value === undefined ? "undefined" : value;
      } catch (error) {
        return error.name;
      }
    };
  child.eval(
    `globalThis.events=[];globalThis.Foreign=class Foreign extends HTMLElement{static observedAttributes=['data-v'];constructor(){super();events.push(['constructor']);}connectedCallback(){events.push(['connected',this.ownerDocument===document]);}disconnectedCallback(){events.push(['disconnected',this.ownerDocument===document]);}adoptedCallback(old,next){events.push(['adopted',old===document,next===document]);}attributeChangedCallback(name,old,value){events.push(['attribute',old,value]);}};customElements.define('x-realm',Foreign);`,
  );
  const node = child.document.createElement("x-realm");
  child.document.body.append(node);
  child.events.length = 0;
  document.body.append(node);
  node.setAttribute("data-v", "parent");
  child.document.body.append(node);
  node.setAttribute("data-v", "child");
  result.lifecycle = child.events;
  result.brands = [
    CustomElementRegistry.prototype.get.call(child.customElements, "x-realm") === child.Foreign,
    Object.getOwnPropertyDescriptor(Document.prototype, "customElementRegistry").get.call(
      child.document,
    ) === child.customElements,
    child.CustomElementRegistry.prototype.getName.call(child.customElements, child.Foreign),
  ];
  const parentCreated = Document.prototype.createElement.call(child.document, "x-realm");
  result.borrowedCreate = [
    parentCreated instanceof child.Foreign,
    parentCreated.ownerDocument === child.document,
  ];
  result.upgradeBorrow = attempt(() =>
    CustomElementRegistry.prototype.upgrade.call(child.customElements, parentCreated),
  );
  const clone = node.cloneNode(true);
  result.clone = [clone instanceof child.Foreign, clone.ownerDocument === child.document];
  const imported = document.importNode(node, true);
  result.import = [imported instanceof child.Foreign, imported.ownerDocument === document];
  return result;
})();

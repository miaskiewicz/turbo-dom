(async () => {
  const rows = [];
  class Moving extends HTMLElement {
    connectedCallback() {
      rows.push([this.id, "connected", this.isConnected]);
    }
    disconnectedCallback() {
      rows.push([this.id, "disconnected", this.isConnected]);
    }
    connectedMoveCallback() {
      rows.push([this.id, "moved", this.isConnected]);
    }
  }
  class Fallback extends HTMLElement {
    connectedCallback() {
      rows.push([this.id, "connected", this.isConnected]);
    }
    disconnectedCallback() {
      rows.push([this.id, "disconnected", this.isConnected]);
    }
  }
  customElements.define("x-move", Moving);
  customElements.define("x-fallback", Fallback);
  const first = document.createElement("section"),
    second = document.createElement("section");
  document.body.append(first, second);
  const root = document.createElement("x-move");
  root.id = "root";
  const child = document.createElement("x-fallback");
  child.id = "fallback";
  const nested = document.createElement("x-move");
  nested.id = "nested";
  root.append(child, nested);
  first.append(root);
  rows.length = 0;
  second.moveBefore(root, null);
  const connected = rows.slice();
  rows.length = 0;
  second.moveBefore(root, root);
  const samePosition = rows.slice();
  rows.length = 0;
  second.append(root);
  const append = rows.slice();
  rows.length = 0;
  const fragment = document.createDocumentFragment(),
    one = document.createElement("div"),
    two = document.createElement("div"),
    detached = document.createElement("x-move");
  detached.id = "detached";
  fragment.append(one, two);
  one.append(detached);
  two.moveBefore(detached, null);
  const disconnected = rows.slice();
  return { connected, samePosition, append, disconnected };
})();

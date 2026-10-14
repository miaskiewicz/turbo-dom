(() => {
  const id = (node) =>
    node === null ? null : node.nodeType === 3 ? "#text" : node.getAttribute("id") || node.nodeName;
  const records = (observer) =>
    observer.takeRecords().map((record) => ({
      type: record.type,
      target: id(record.target),
      added: Array.from(record.addedNodes, id),
      removed: Array.from(record.removedNodes, id),
      previous: id(record.previousSibling),
      next: id(record.nextSibling),
      attribute: record.attributeName,
      namespace: record.attributeNamespace,
      oldValue: record.oldValue,
    }));
  const container = document.createElement("div");
  document.body.append(container);
  const rows = {};
  try {
    container.innerHTML =
      '<section id="source"><i id="first"></i><b id="node"></b><u id="last"></u></section><aside id="target"><em id="reference"></em></aside>';
    const source = container.querySelector("section"),
      target = container.querySelector("aside"),
      node = container.querySelector("b");
    const oldObserver = new MutationObserver(() => {}),
      newObserver = new MutationObserver(() => {});
    oldObserver.observe(source, { childList: true });
    newObserver.observe(target, { childList: true });
    target.insertBefore(node, target.firstChild);
    rows.move = [...records(oldObserver), ...records(newObserver)];
    oldObserver.disconnect();
    newObserver.disconnect();
    const observer = new MutationObserver(() => {});
    observer.observe(node, { attributes: true, attributeOldValue: true });
    node.setAttributeNS("urn:authored", "p:value", "first");
    node.setAttribute("p:value", "second");
    node.removeAttribute("p:value");
    rows.attributes = records(observer);
    observer.disconnect();
    observer.observe(source, { childList: true });
    source.innerHTML = '<p id="replacement">one</p>tail';
    rows.innerHTML = records(observer);
    observer.disconnect();
    const legacy = new MutationObserver(() => {});
    legacy.observe(target, { attributes: true, subtree: true, attributeOldValue: true });
    node.setAttribute("class", "changed");
    source.append(node);
    rows.mutationTimeAncestry = records(legacy);
    legacy.disconnect();
    target.textContent = "";
    observer.observe(target, { childList: true, attributes: true });
    target.removeAttribute("absent");
    target.textContent = "";
    target.innerHTML = "";
    rows.nonMutations = records(observer);
    observer.disconnect();
    const host = document.createElement("div");
    host.id = "host";
    container.append(host);
    const shadow = host.attachShadow({ mode: "open" }),
      inside = document.createElement("b");
    inside.id = "inside";
    shadow.append(inside);
    const hostObserver = new MutationObserver(() => {});
    hostObserver.observe(host, { attributes: true, subtree: true });
    observer.observe(shadow, { attributes: true, subtree: true });
    inside.setAttribute("class", "changed");
    rows.shadow = records(observer);
    rows.shadowHost = records(hostObserver);
    observer.disconnect();
    hostObserver.disconnect();
    return rows;
  } finally {
    container.remove();
  }
})();

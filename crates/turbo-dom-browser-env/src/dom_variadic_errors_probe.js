(async () => {
  const frame = document.createElement("iframe");
  const loaded = new Promise((resolve) => {
    frame.onload = resolve;
  });
  document.body.append(frame);
  await loaded;
  const source = frame.contentDocument;
  const rows = {};
  const operations = {
    documentAppend: (node) => document.append(node, "text"),
    documentPrepend: (node) => document.prepend(node, "text"),
    documentReplaceChildren: (node) => document.replaceChildren(node, "text"),
    rootBefore: (node) => document.documentElement.before(node, "text"),
    rootAfter: (node) => document.documentElement.after(node, "text"),
    rootReplaceWith: (node) => document.documentElement.replaceWith(node, "text"),
  };
  try {
    for (const [name, operation] of Object.entries(operations)) {
      const holder = source.createElement("section");
      const node = source.createElement("b");
      node.textContent = "foreign";
      holder.append(node);
      source.body.append(holder);
      const root = document.documentElement;
      const range = source.createRange();
      range.selectNodeContents(node.firstChild);
      let error = null;
      try {
        operation(node);
      } catch (caught) {
        error = caught.name;
      }
      rows[name] = [
        error,
        node.ownerDocument === document,
        node.parentNode?.nodeType ?? null,
        holder.childNodes.length,
        document.documentElement === root && root.parentNode === document,
        range.startContainer === holder,
        range.startOffset,
        range.endOffset,
        node.parentNode?.textContent ?? null,
      ];
      node.remove();
      holder.remove();
    }
    {
      const target = document.createElement("section");
      target.innerHTML = "<i>retained</i>";
      document.body.append(target);
      const originalParent = target.parentNode;
      let error = null;
      try {
        target.append(target, "text");
      } catch (caught) {
        error = caught.name;
      }
      rows.selfAncestor = [
        error,
        target.parentNode === originalParent,
        target.parentNode?.nodeType ?? null,
        target.innerHTML,
        target.parentNode?.textContent ?? null,
      ];
      target.remove();
    }
    rows.fragmentOverlap = {};
    for (const order of ["child-text-fragment", "fragment-child-fragment"]) {
      const target = document.createElement("section");
      document.body.append(target);
      const fragment = source.createDocumentFragment();
      const first = source.createElement("b"),
        last = source.createElement("i");
      first.textContent = "first";
      last.textContent = "last";
      fragment.append(first, last);
      if (order === "child-text-fragment") target.append(first, "mid", fragment);
      else target.append(fragment, first, fragment);
      rows.fragmentOverlap[order] = [
        target.innerHTML,
        fragment.ownerDocument === source,
        fragment.childNodes.length,
        first.ownerDocument === document,
        last.ownerDocument === document,
      ];
      target.remove();
    }
    rows.fragmentFailures = {};
    for (const count of [1, 2]) {
      const fragment = source.createDocumentFragment(),
        children = [];
      for (let index = 0; index < count; index++) {
        const node = source.createElement("b");
        node.textContent = String(index);
        fragment.append(node);
        children.push(node);
      }
      let error = null;
      try {
        document.append(fragment);
      } catch (caught) {
        error = caught.name;
      }
      rows.fragmentFailures[count] = [
        error,
        fragment.ownerDocument === source,
        fragment.childNodes.length,
        children.map((node) => [node.ownerDocument === source, node.parentNode === fragment]),
      ];
    }
    return rows;
  } finally {
    frame.remove();
  }
})();

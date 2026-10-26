(async () => {
  const frame = document.createElement("iframe");
  const loaded = new Promise((resolve) => {
    frame.onload = resolve;
  });
  document.body.appendChild(frame);
  await loaded;
  const source = frame.contentDocument;
  const rows = {};
  const operations = {
    appendChild: (parent, node) => parent.appendChild(node),
    insertBefore: (parent, node) => parent.insertBefore(node, parent.lastChild),
    replaceChild: (parent, node) => parent.replaceChild(node, parent.firstChild),
    append: (parent, node) => parent.append(node, "tail"),
    prepend: (parent, node) => parent.prepend("head", node),
    before: (parent, node) => parent.firstChild.before(node),
    after: (parent, node) => parent.firstChild.after(node),
    replaceWith: (parent, node) => parent.firstChild.replaceWith(node),
    replaceChildren: (parent, node) => parent.replaceChildren("head", node, "tail"),
    adjacentElement: (parent, node) => parent.firstChild.insertAdjacentElement("afterend", node),
    borrowedAppend: (parent, node) =>
      frame.contentWindow.Node.prototype.appendChild.call(parent, node),
  };
  const parent = () => {
    const target = document.createElement("section");
    target.innerHTML = "<i>first</i><u>last</u>";
    document.body.appendChild(target);
    return target;
  };
  const foreign = () => {
    const holder = source.createElement("div");
    const node = source.createElement("b");
    node.textContent = "foreign";
    holder.appendChild(node);
    source.body.appendChild(holder);
    return { holder, node };
  };
  try {
    for (const [name, operation] of Object.entries(operations)) {
      const target = parent();
      const { holder, node } = foreign();
      const text = node.firstChild;
      const range = source.createRange();
      range.selectNodeContents(text);
      const expando = {};
      node.expando = expando;
      let calls = 0;
      node.addEventListener("authored", () => calls++);
      const result = operation(target, node);
      node.dispatchEvent(new Event("authored"));
      rows[name] = {
        owner: node.ownerDocument === document,
        identity: target.contains(node) && node.firstChild === text && node.expando === expando,
        oldParentEmpty: holder.childNodes.length === 0,
        listeners: calls,
        originalRealm: node instanceof frame.contentWindow.HTMLElement,
        sourceRange: [
          range.startContainer === holder,
          range.startOffset,
          range.endOffset,
          String(range),
        ],
        returned: result === node ? "incoming" : result === undefined ? "undefined" : "removed",
        html: target.innerHTML,
      };
      target.remove();
      holder.remove();
    }
    {
      const target = parent();
      const { holder, node } = foreign();
      target.append(node, node, "end");
      rows.duplicate = [
        target.innerHTML,
        target.children[2] === node,
        node.ownerDocument === document,
      ];
      target.remove();
      holder.remove();
    }
    {
      const target = parent();
      const fragment = source.createDocumentFragment();
      const first = source.createElement("b");
      const last = source.createTextNode("tail");
      first.textContent = "head";
      fragment.append(first, last);
      const result = target.insertBefore(fragment, target.lastChild);
      rows.fragment = [
        result === fragment,
        fragment.ownerDocument === document,
        fragment.childNodes.length,
        first.ownerDocument === document && last.ownerDocument === document,
        target.childNodes[1] === first && target.childNodes[2] === last,
        target.innerHTML,
      ];
      target.remove();
    }
    rows.mixedFragments = {};
    for (const order of ["node-text-fragment", "fragment-node-fragment", "node-fragment-node"]) {
      const target = parent();
      const { holder, node } = foreign();
      const fragment = source.createDocumentFragment();
      const first = source.createElement("em"),
        last = source.createTextNode("tail");
      first.textContent = "fragment";
      fragment.append(first, last);
      if (order === "node-text-fragment") target.append(node, "mid", fragment);
      else if (order === "fragment-node-fragment") target.append(fragment, node, fragment);
      else target.append(node, fragment, node);
      rows.mixedFragments[order] = [
        target.innerHTML,
        fragment.ownerDocument === source,
        fragment.childNodes.length,
        node.ownerDocument === document &&
          first.ownerDocument === document &&
          last.ownerDocument === document,
        holder.childNodes.length,
        target.contains(node) && target.contains(first) && target.contains(last),
      ];
      target.remove();
      holder.remove();
    }
    {
      const target = parent();
      const node = source.createElementNS("http://www.w3.org/2000/svg", "svg");
      node.setAttribute("viewBox", "0 0 10 20");
      target.appendChild(node);
      rows.namespace = [
        node.ownerDocument === document,
        node.namespaceURI,
        node.getAttribute("viewBox"),
      ];
      target.remove();
    }
    {
      const target = parent();
      const holder = source.createElement("div");
      source.body.appendChild(holder);
      const root = holder.attachShadow({ mode: "open" });
      root.innerHTML = "<b>shadow</b>tail";
      const first = root.firstChild,
        last = root.lastChild;
      const range = source.createRange();
      range.selectNodeContents(first.firstChild);
      const result = target.appendChild(root);
      rows.shadowInsertion = [
        result === root,
        root.ownerDocument === source,
        root.host === holder,
        root.childNodes.length,
        first.ownerDocument === document && last.ownerDocument === document,
        target.childNodes[2] === first && target.childNodes[3] === last,
        range.startContainer === root,
        range.startOffset,
        range.endOffset,
        target.innerHTML,
      ];
      target.remove();
      holder.remove();
    }
    const failures = {
      secondDocumentElement: { run: (_target, node) => document.appendChild(node) },
      invalidParent: {
        run: (_target, node) => document.createTextNode("parent").appendChild(node),
      },
      foreignReference: { run: (target, node) => target.insertBefore(node, source.body) },
      foreignRemoval: { run: (target, node) => target.removeChild(node) },
      invalidAdjacentPosition: {
        run: (target, node) => target.insertAdjacentElement("invalid", node),
      },
      documentNode: { make: () => source, run: (target, node) => target.appendChild(node) },
      attributeNode: {
        make: (holder) => {
          const attribute = source.createAttribute("authored");
          attribute.value = "retained";
          holder.setAttributeNode(attribute);
          return attribute;
        },
        run: (target, node) => target.appendChild(node),
      },
      documentFragment: {
        make: () => {
          const fragment = source.createDocumentFragment();
          fragment.append(source.createElement("b"), source.createElement("i"));
          return fragment;
        },
        run: (_target, node) => document.replaceChild(node, document.documentElement),
      },
    };
    rows.failures = {};
    for (const [name, operation] of Object.entries(failures)) {
      const target = parent();
      const { holder, node } = foreign();
      const argument = operation.make ? operation.make(holder) : node;
      const originalOwner = argument.ownerDocument,
        originalParent = argument.parentNode;
      const originalChildren = Array.from(argument.childNodes);
      const originalAttributeOwner = argument.nodeType === 2 ? argument.ownerElement : null;
      const range = source.createRange();
      range.selectNodeContents(node.firstChild);
      const originalRange = [
        range.startContainer,
        range.startOffset,
        range.endContainer,
        range.endOffset,
      ];
      const original = target.innerHTML;
      let error = null;
      try {
        operation.run(target, argument);
      } catch (caught) {
        error = caught.name;
      }
      rows.failures[name] = [
        error,
        argument.ownerDocument === originalOwner && argument.parentNode === originalParent,
        originalChildren.length === argument.childNodes.length &&
          originalChildren.every((child, index) => child === argument.childNodes[index]),
        argument.nodeType !== 2 || argument.ownerElement === originalAttributeOwner,
        node.ownerDocument === source && node.parentNode === holder,
        range.startContainer === originalRange[0] &&
          range.startOffset === originalRange[1] &&
          range.endContainer === originalRange[2] &&
          range.endOffset === originalRange[3],
        target.innerHTML === original,
      ];
      target.remove();
      holder.remove();
    }
    return rows;
  } finally {
    frame.remove();
  }
})();

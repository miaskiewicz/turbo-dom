(() => {
  const result = { contents: {}, errors: {}, comparisons: {} };
  const error = (callback) => {
    try {
      callback();
      return null;
    } catch (error) {
      return { name: error.name };
    }
  };
  const fixture = (html) => {
    const host = document.createElement("section");
    host.innerHTML = html;
    document.body.appendChild(host);
    return host;
  };
  const markup = (fragment) => {
    const host = document.createElement("div");
    host.appendChild(fragment.cloneNode(true));
    return host.innerHTML;
  };
  const boundary = (range) => ({
    collapsed: range.collapsed,
    node:
      range.startContainer.nodeType === 3
        ? "#text:" + range.startContainer.data
        : range.startContainer.localName || range.startContainer.nodeName,
    offset: range.startOffset,
  });
  const scenarios = {
    text: {
      html: "<p>abcdef</p>",
      select: (host, range) => {
        const text = host.firstChild.firstChild;
        range.setStart(text, 1);
        range.setEnd(text, 4);
      },
    },
    nested: {
      html: "<div><b>ab<i>cd</i>ef</b><em>gh</em><u>ij</u></div>",
      select: (host, range) => {
        const node = host.firstChild;
        range.setStart(node.firstChild.firstChild, 1);
        range.setEnd(node.children[1].firstChild, 1);
      },
    },
    siblings: {
      html: "<div><a>x</a><b>y</b><i>z</i><u>w</u></div>",
      select: (host, range) => {
        range.setStart(host.firstChild, 1);
        range.setEnd(host.firstChild, 3);
      },
    },
    surroundElement: {
      html: "<p>ab<b>cd</b>ef</p>",
      select: (host, range) => {
        range.setStart(host.firstChild, 1);
        range.setEnd(host.firstChild, 2);
      },
    },
  };
  for (const [name, description] of Object.entries(scenarios)) {
    result.contents[name] = {};
    for (const operation of [
      "cloneContents",
      "extractContents",
      "deleteContents",
      "surroundContents",
    ]) {
      const host = fixture(description.html),
        range = document.createRange();
      description.select(host, range);
      const original = host.firstChild.childNodes[1];
      const selected = String(range);
      let output, fragment, thrown;
      try {
        if (operation === "surroundContents") {
          const wrapper = document.createElement("mark");
          wrapper.textContent = "discard";
          range.surroundContents(wrapper);
          output = wrapper.innerHTML;
        } else {
          fragment = range[operation]();
          output = fragment ? markup(fragment) : null;
        }
      } catch (e) {
        thrown = e.name;
      }
      result.contents[name][operation] = {
        selected,
        output,
        remaining: host.innerHTML,
        boundary: boundary(range),
        error: thrown || null,
        movedOriginal:
          operation === "extractContents" && !!fragment && fragment.firstChild === original,
      };
      host.remove();
    }
  }
  const host = fixture("<p>abcdef</p>"),
    text = host.firstChild.firstChild,
    range = document.createRange();
  range.setStart(text, 1);
  range.setEnd(text, 4);
  const detached = document.createElement("b"),
    foreign = document.implementation.createHTMLDocument("foreign"),
    other = foreign.createRange();
  for (const [name, callback] of Object.entries({
    largeOffset: () => range.setStart(text, 99),
    negativeOffset: () => range.setEnd(text, -1),
    doctype: () => range.setStart(document.doctype, 0),
    rootSelect: () => range.selectNode(document),
    detachedBefore: () => range.setStartBefore(detached),
    missingArgument: () => range.setStart(text),
    forgedClone: () => Range.prototype.cloneContents.call({}),
    invalidParent: () => range.surroundContents(document.createDocumentFragment()),
    badComparison: () => range.compareBoundaryPoints(123, range),
    foreignComparison: () => range.compareBoundaryPoints(0, other),
    doctypePoint: () => range.comparePoint(document.doctype, 0),
  }))
    result.errors[name] = error(callback);
  const second = document.createRange();
  second.setStart(text, 2);
  second.setEnd(text, 5);
  result.comparisons.modes = [0, 1, 2, 3].map((mode) => range.compareBoundaryPoints(mode, second));
  result.comparisons.points = [0, 1, 3, 4, 5].map((offset) => range.comparePoint(text, offset));
  result.comparisons.inside = [0, 1, 3, 4, 5].map((offset) => range.isPointInRange(text, offset));
  result.comparisons.foreignInside = range.isPointInRange(foreign.body, 0);
  const retarget = document.createRange();
  retarget.setStart(foreign.body, 0);
  result.comparisons.retarget = {
    start: retarget.startContainer === foreign.body,
    end: retarget.endContainer === foreign.body,
    collapsed: retarget.collapsed,
  };
  host.remove();
  return result;
})();

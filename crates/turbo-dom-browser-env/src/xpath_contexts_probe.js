// XPath from contexts inside a DocumentFragment, a shadow root and a detached subtree, and
// the namespace axis (turbo-dom#185). Run by scripts/xpath-contexts-chrome-control.mjs in
// Chrome 156 and by xpath_contexts_tests.rs in the binding; both must return the same JSON.
(() => {
  const describe = (node) =>
    node === null ? null : node.nodeType + ":" + node.nodeName + (node.id ? "#" + node.id : "");
  const evaluate = (context, expression) => {
    try {
      if (expression.startsWith("count("))
        return document.evaluate(expression, context, null, 1, null).numberValue;
      const result = document.evaluate(expression, context, null, 7, null);
      const items = [];
      for (let i = 0; i < result.snapshotLength; i++) items.push(describe(result.snapshotItem(i)));
      return items;
    } catch (error) {
      return error.name;
    }
  };
  const fragment = document.createDocumentFragment();
  fragment.append(document.createElement("div"), document.createElement("p"));
  fragment.firstChild.id = "fd";
  fragment.firstChild.innerHTML = '<span id="fs">x</span><b></b>';
  fragment.firstChild.setAttribute("data-a", "1");
  const host = document.body.appendChild(document.createElement("section"));
  host.id = "host";
  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = '<div id="sd"><span id="ss">y</span></div><i id="si"></i>';
  const detached = document.createElement("article");
  detached.id = "dt";
  detached.innerHTML = '<span id="ds"></span>';
  document.body.insertAdjacentHTML("beforeend", '<svg id="sv"><g id="sg"></g></svg>');
  const expressions = [
    ".//span",
    "//span",
    "/",
    "/*",
    "/descendant::*",
    "ancestor::node()",
    "ancestor-or-self::*",
    "..",
    "following::*",
    "preceding::*",
    "following-sibling::*",
    "preceding-sibling::node()",
    "id('fs')",
    "id('ss')",
    "namespace::*",
    "count(namespace::node())",
  ];
  const contexts = {
    fragment_div: fragment.firstChild,
    fragment_span: fragment.firstChild.firstChild,
    fragment_text: fragment.firstChild.firstChild.firstChild,
    fragment_attr: fragment.firstChild.getAttributeNode("data-a"),
    fragment: fragment,
    shadow_div: shadow.firstChild,
    shadow_span: shadow.firstChild.firstChild,
    shadow_root: shadow,
    detached_span: detached.firstChild,
    html: document.documentElement,
    svg_g: document.getElementById("sg"),
    document: document,
  };
  const out = {};
  for (const [name, context] of Object.entries(contexts)) {
    out[name] = {};
    for (const expression of expressions) out[name][expression] = evaluate(context, expression);
  }
  return out;
})()

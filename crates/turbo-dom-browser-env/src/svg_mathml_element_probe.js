(() => {
  // turbo-dom#87: SVGElement and MathMLElement include the ElementCSSInlineStyle (style),
  // HTMLOrSVGElement (dataset, nonce, autofocus, tabIndex, focus(), blur()) and
  // GlobalEventHandlers (on*) mixins, and MathML-namespace elements are MathMLElements.
  const SVG = "http://www.w3.org/2000/svg", MATHML = "http://www.w3.org/1998/Math/MathML";
  const result = {};
  // turbo-test#79 item 4 (`svg.style.flex = '1'` threw): React sets `style` on SVG icons. The
  // longhand keeps the shorthand serialization (turbo-dom#90) out of this probe.
  const svg = document.createElementNS(SVG, "svg");
  svg.style.flexGrow = "1";
  result.repro = { flexGrow: svg.style.flexGrow, attribute: svg.getAttribute("style") };
  const keys = ["style", "dataset", "nonce", "tabIndex", "autofocus", "focus", "blur", "onclick", "onfocus"];
  result.descriptors = [];
  for (const owner of ["Element", "HTMLElement", "SVGElement", "MathMLElement"]) {
    for (const key of keys) {
      const d = Object.getOwnPropertyDescriptor(globalThis[owner].prototype, key);
      result.descriptors.push(!d ? { owner, key, missing: true } : "value" in d ? {
        owner, key, method: typeof d.value, name: d.value.name, length: d.value.length,
        enumerable: d.enumerable, configurable: d.configurable, writable: d.writable,
      } : {
        owner, key, enumerable: d.enumerable, configurable: d.configurable,
        getter: d.get && d.get.name, setter: d.set ? d.set.name : null,
        getLength: d.get && d.get.length, setLength: d.set ? d.set.length : null,
      });
    }
  }
  const mathml = globalThis.MathMLElement;
  const math = document.createElementNS(MATHML, "math");
  const parsedHost = document.createElement("div");
  parsedHost.innerHTML = "<math><mi>x</mi></math><svg><a href='#'></a></svg>";
  let constructed;
  try { new mathml(); constructed = "constructed"; } catch (error) { constructed = error.name; }
  result.mathml = {
    type: typeof mathml, name: mathml.name, length: mathml.length,
    parent: Object.getPrototypeOf(mathml.prototype) === Element.prototype,
    ctorParent: Object.getPrototypeOf(mathml) === Element,
    tag: Object.prototype.toString.call(math), proto: Object.getPrototypeOf(math) === mathml.prototype,
    parsed: [parsedHost.firstChild instanceof mathml, parsedHost.firstChild.firstChild instanceof mathml],
    notSvg: [math instanceof SVGElement, math instanceof HTMLElement, svg instanceof mathml],
    constructed,
    windowDescriptor: (({ writable, enumerable, configurable }) => ({ writable, enumerable, configurable }))(Object.getOwnPropertyDescriptor(globalThis, "MathMLElement")),
  };
  const make = {
    svg: () => document.createElementNS(SVG, "svg"),
    rect: () => document.createElementNS(SVG, "rect"),
    svgA: () => { const a = document.createElementNS(SVG, "a"); a.setAttribute("href", "#x"); return a; },
    math: () => document.createElementNS(MATHML, "math"),
    mi: () => document.createElementNS(MATHML, "mi"),
    unknownNs: () => document.createElementNS("urn:x", "svg"),
  };
  result.elements = {};
  for (const [name, create] of Object.entries(make)) {
    const el = create(), record = {};
    for (const key of keys) record["has_" + key] = key in el;
    if (record.has_style) {
      const style = el.style;
      record.styleTag = Object.prototype.toString.call(style);
      record.isDeclaration = style instanceof CSSStyleDeclaration;
      record.same = el.style === style;
      record.emptyText = style.cssText;
      style.color = "red";
      style.setProperty("--x", "1");
      record.afterSet = el.getAttribute("style");
      el.setAttribute("style", "width: 2px; fill: blue");
      record.fromAttribute = [style.width, style.fill, style.length];
      el.style = "stroke: green";
      record.putForwards = [el.getAttribute("style"), style.stroke, el.style === style];
      style.removeProperty("stroke");
      record.afterRemove = [el.getAttribute("style"), style.length];
      el.removeAttribute("style");
      record.afterRemoveAttribute = [style.cssText, style.length, el.hasAttribute("style")];
    }
    if (record.has_dataset) {
      el.dataset.fooBar = "x";
      el.setAttribute("data-a", "1");
      record.dataset = [el.getAttribute("data-foo-bar"), el.dataset.a, Object.keys(el.dataset).join(","), el.dataset === el.dataset];
    }
    if (record.has_nonce) {
      const nonce = [el.nonce];
      el.setAttribute("nonce", "n1");
      nonce.push(el.nonce);
      el.nonce = "n2";
      nonce.push(el.nonce, el.getAttribute("nonce"));
      record.nonce = nonce;
    }
    if (record.has_tabIndex) {
      const tab = [el.tabIndex];
      el.setAttribute("tabindex", " 3x");
      tab.push(el.tabIndex);
      el.setAttribute("tabindex", "bogus");
      tab.push(el.tabIndex);
      el.tabIndex = 5.7;
      tab.push(el.tabIndex, el.getAttribute("tabindex"));
      el.removeAttribute("tabindex");
      record.tabIndex = tab;
    }
    if (record.has_autofocus) {
      const auto = [el.autofocus];
      el.autofocus = true;
      auto.push(el.autofocus, el.getAttribute("autofocus"));
      el.autofocus = false;
      auto.push(el.autofocus, el.hasAttribute("autofocus"));
      record.autofocus = auto;
    }
    if (record.has_focus) {
      // Fresh elements in a rendered context (an SVG / MathML child inside its root in the body):
      // without tabindex only an SVG <a href> is focusable; tabindex=0 makes any of them focusable.
      const focus = [];
      for (const tabindex of [null, "0"]) {
        const target = create(), log = [];
        if (tabindex !== null) target.setAttribute("tabindex", tabindex);
        // Blink makes an SVG element with its own focus / blur listener focusable; logging
        // from the document keeps the target listener-free.
        const listen = (event) => { if (event.target === target) log.push(event.type); };
        document.addEventListener("focus", listen, true);
        document.addEventListener("blur", listen, true);
        const root = name === "svg" || name === "math" || name === "unknownNs" ? null : document.createElementNS(target.namespaceURI, target.namespaceURI === SVG ? "svg" : "math");
        if (root) { root.append(target); document.body.append(root); } else document.body.append(target);
        target.focus();
        const active = document.activeElement === target;
        target.blur();
        document.removeEventListener("focus", listen, true);
        document.removeEventListener("blur", listen, true);
        focus.push([active, document.activeElement === document.body, log.join(",")]);
        (root || target).remove();
      }
      record.focus = focus;
    }
    if (record.has_onclick) {
      let calls = 0;
      el.onclick = function (event) { calls++; return event.type; };
      el.dispatchEvent(new MouseEvent("click"));
      el.onclick = null;
      el.setAttribute("onclick", "return 1");
      record.onclick = [calls, typeof el.onclick, el.onclick && el.onclick.name];
    }
    result.elements[name] = record;
  }
  const host = document.createElement("div");
  host.innerHTML = '<svg style="fill: red" data-k="v" tabindex="2"><circle style="stroke: blue; stroke-width: 2px"></circle></svg><math style="color: red" data-m="w"></math>';
  const parsedSvg = host.firstChild, circle = parsedSvg.firstChild, parsedMath = host.lastChild;
  result.parsed = [parsedSvg.style.fill, circle.style.stroke, circle.style.strokeWidth, circle.style.length, parsedSvg.dataset.k, parsedSvg.tabIndex, parsedMath.style.color, parsedMath.dataset.m];
  result.brand = [];
  const receivers = { html: document.createElement("div"), svg: document.createElementNS(SVG, "svg"), math: document.createElementNS(MATHML, "math"), other: document.createElementNS("urn:x", "x") };
  for (const owner of ["HTMLElement", "SVGElement", "MathMLElement"]) {
    for (const key of ["style", "dataset", "nonce", "tabIndex", "autofocus", "focus", "onclick"]) {
      const d = Object.getOwnPropertyDescriptor(globalThis[owner].prototype, key);
      for (const [label, receiver] of Object.entries(receivers)) {
        try {
          if (d.get) d.get.call(receiver); else d.value.call(receiver);
          result.brand.push([owner, key, label, "ok"]);
        } catch (error) {
          result.brand.push([owner, key, label, error.name]);
        }
      }
    }
  }
  return result;
})();

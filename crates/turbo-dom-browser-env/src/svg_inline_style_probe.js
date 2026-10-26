(() => {
  const SVG = "http://www.w3.org/2000/svg", MATHML = "http://www.w3.org/1998/Math/MathML";
  const result = {};
  // turbo-test#79 item 4 (`svg.style.flex = '1'` threw): React sets `style` on SVG icons. The
  // longhand keeps the shorthand serialization (turbo-dom#90) out of this probe.
  const svg = document.createElementNS(SVG, "svg");
  svg.style.flexGrow = "1";
  result.repro = { flexGrow: svg.style.flexGrow, attribute: svg.getAttribute("style") };
  result.descriptors = [];
  for (const owner of ["Element", "HTMLElement", "SVGElement", "MathMLElement"]) {
    for (const key of ["style", "dataset"]) {
      const d = Object.getOwnPropertyDescriptor(globalThis[owner].prototype, key);
      result.descriptors.push(d ? {
        owner, key, enumerable: d.enumerable, configurable: d.configurable,
        getter: d.get && d.get.name, setter: d.set ? d.set.name : null,
        getLength: d.get && d.get.length, setLength: d.set ? d.set.length : null,
      } : { owner, key, missing: true });
    }
  }
  const make = {
    svg: () => document.createElementNS(SVG, "svg"),
    rect: () => document.createElementNS(SVG, "rect"),
    math: () => document.createElementNS(MATHML, "math"),
    mi: () => document.createElementNS(MATHML, "mi"),
    unknownNs: () => document.createElementNS("urn:x", "svg"),
  };
  result.elements = {};
  for (const [name, create] of Object.entries(make)) {
    const el = create(), record = { hasStyle: "style" in el, hasDataset: "dataset" in el };
    if (record.hasStyle) {
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
    if (record.hasDataset) {
      el.dataset.fooBar = "x";
      el.setAttribute("data-a", "1");
      record.dataset = [el.getAttribute("data-foo-bar"), el.dataset.a, Object.keys(el.dataset).join(",")];
    }
    result.elements[name] = record;
  }
  const host = document.createElement("div");
  host.innerHTML = '<svg style="fill: red"><circle style="stroke: blue; stroke-width: 2px"></circle></svg>';
  const parsedSvg = host.firstChild, circle = parsedSvg.firstChild;
  result.parsed = [parsedSvg.style.fill, circle.style.stroke, circle.style.strokeWidth, circle.style.length];
  result.brand = [];
  for (const [owner, receiver] of [["SVGElement", document.createElement("div")], ["HTMLElement", document.createElementNS(SVG, "svg")], ["MathMLElement", document.createElementNS(SVG, "svg")]]) {
    try {
      Object.getOwnPropertyDescriptor(globalThis[owner].prototype, "style").get.call(receiver);
      result.brand.push([owner, "ok"]);
    } catch (error) {
      result.brand.push([owner, error.name]);
    }
  }
  return result;
})();

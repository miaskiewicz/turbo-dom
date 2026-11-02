(() => {
  // turbo-dom#97: a longhand set on an inline style reads back in Chrome's specified-value
  // spelling ("" when the value is invalid). Pairs come from css-longhand-cases.json
  // (scripts/make-css-longhand-cases.py).
  const pairs = JSON.parse(document.getElementById("values").textContent);
  return pairs.map(([name, value]) => {
    const element = document.createElement("div");
    element.style.setProperty(name, value);
    return [name, value, element.style.getPropertyValue(name), element.style.cssText];
  });
})();

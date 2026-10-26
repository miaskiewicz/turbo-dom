(() => {
  // turbo-dom#90: CSSOM shorthand expansion and serialization on an element's inline style.
  // Scenarios come from css-shorthand-cases.json (scripts/make-css-shorthand-cases.py).
  const scenarios = JSON.parse(document.getElementById("scenarios").textContent);
  // The shorthand table: a property set to `initial` lists the longhands it sets (the
  // candidate names are Chrome's shorthands and aliases, from the cases page).
  const table = {}, aliases = {};
  const scratch = document.createElement("div");
  const names = JSON.parse(document.getElementById("names").textContent);
  for (const name of names) {
    scratch.removeAttribute("style");
    scratch.style.setProperty(name, "initial");
    const items = [...scratch.style];
    if (items.length > 1) table[name] = items;
    else if (items.length === 1 && items[0] !== name) aliases[name] = items[0];
  }
  const shorthandsOf = {};
  for (const [shorthand, longhands] of Object.entries(table)) for (const longhand of longhands) (shorthandsOf[longhand] ||= []).push(shorthand);
  const record = (style) => {
    const items = [], shorthands = {};
    for (let index = 0; index < style.length; index++) {
      const name = style.item(index), priority = style.getPropertyPriority(name);
      items.push(priority ? [name, style.getPropertyValue(name), priority] : [name, style.getPropertyValue(name)]);
      for (const shorthand of shorthandsOf[name] || []) if (!(shorthand in shorthands)) shorthands[shorthand] = style.getPropertyValue(shorthand);
    }
    return { cssText: style.cssText, items, shorthands };
  };
  const run = (ops) => {
    const element = document.createElement("div"), returns = [];
    for (const [op, name, value, priority] of ops) {
      if (op === "css") element.setAttribute("style", name);
      else if (op === "set") element.style.setProperty(name, value, priority);
      else if (op === "remove") returns.push(element.style.removeProperty(name));
      else if (op === "get") returns.push(element.style.getPropertyValue(name));
      else if (op === "recss") { const copy = document.createElement("div"); copy.setAttribute("style", element.style.cssText); returns.push(copy.style.cssText); }
    }
    const out = record(element.style);
    if (returns.length) out.returns = returns;
    return { out, element };
  };
  const results = [];
  for (const scenario of scenarios) {
    const { out, element } = run(scenario.ops);
    const row = { id: scenario.id, ...out };
    const items = [];
    for (let index = 0; index < element.style.length; index++) { const name = element.style.item(index); items.push([name, element.style.getPropertyValue(name)]); }
    if (scenario.assemble && items.length > 1 && items.every(([, value]) => value !== "")) {
      const build = (list) => { const target = document.createElement("div"); for (const [name, value, priority] of [...list].reverse()) target.style.setProperty(name, value, priority || ""); return record(target.style); };
      row.assembled = {
        all: build(items),
        partial: build(items.slice(1)),
        keyword: build([[items[0][0], "inherit"], ...items.slice(1)]),
        important: build([[items[0][0], items[0][1], "important"], ...items.slice(1)]),
      };
    }
    results.push(row);
  }
  // CSSStyleRule.style is the same declaration block.
  const rules = [];
  for (const text of ["margin: 1px 2px", "margin-top: 1px; margin-right: 1px; margin-bottom: 1px; margin-left: 1px", "flex: 1", "border: 1px solid red", "background: red; color: blue", "font: 12px Arial"]) {
    try {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(`a { ${text} }`);
      rules.push([text, sheet.cssRules[0].cssText, sheet.cssRules[0].style.cssText, sheet.cssRules[0].style.length]);
    } catch (error) {
      rules.push([text, error.name]);
    }
  }
  return { table, aliases, results, rules };
})();

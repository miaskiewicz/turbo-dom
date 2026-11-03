(() => {
  // Grouping / condition rule interfaces (turbo-dom#254): each at-rule's CSSOM object, its
  // own attributes, cssText, child list and CSSGroupingRule insertRule / deleteRule.
  const style = document.createElement("style");
  style.textContent =
    "@container card (min-width:200px){.cq>i{width:17px}}" +
    "@container (orientation:landscape) and (min-width : 10px){b{color:red}}" +
    "@container  side {p{margin:0}}" +
    "@media screen and (min-width:100px){a{color:red}b{color:blue}}" +
    "@supports (display:grid) and (gap:1px){c{display:grid}}" +
    "@layer base{d{color:green}}" +
    "@layer{e{color:green}}" +
    "@layer one, two;" +
    "@scope (.a) to (.b){f{color:red}}" +
    "@scope{g{color:red}}" +
    "@starting-style{h{opacity:0}}";
  document.head.append(style);
  const sheet = style.sheet;
  const attr = (rule, name) => {
    if (!(name in rule)) return "<absent>";
    const value = rule[name];
    if (value && typeof value === "object")
      return value.mediaText !== undefined && !Array.isArray(value)
        ? ["media", value.mediaText]
        : [Object.prototype.toString.call(value), Array.from(value)];
    return value === undefined ? "<undefined>" : value;
  };
  const chain = (rule) => {
    const names = [];
    for (
      let proto = Object.getPrototypeOf(rule);
      proto && proto !== Object.prototype;
      proto = Object.getPrototypeOf(proto)
    )
      names.push(proto.constructor.name);
    return names;
  };
  const tryRun = (fn) => {
    try {
      return ["ok", fn()];
    } catch (error) {
      return [error.name];
    }
  };
  const fields = [
    "conditionText",
    "containerName",
    "containerQuery",
    "conditions",
    "media",
    "name",
    "nameList",
    "start",
    "end",
  ];
  const rules = Array.from(sheet.cssRules).map((rule) => ({
    ctor: rule.constructor.name,
    tag: Object.prototype.toString.call(rule),
    chain: chain(rule),
    type: rule.type,
    cssText: rule.cssText,
    fields: fields.map((name) => [name, attr(rule, name)]),
    children:
      "cssRules" in rule
        ? Array.from(rule.cssRules, (child) => [
            child.constructor.name,
            child.cssText,
            child.parentRule === rule,
          ])
        : "<absent>",
  }));
  const mutations = [];
  for (let index = 0; index < sheet.cssRules.length; index++) {
    const rule = sheet.cssRules[index];
    if (typeof rule.insertRule !== "function") {
      mutations.push([index, "<none>"]);
      continue;
    }
    const row = [index];
    row.push(tryRun(() => rule.insertRule("x{color:blue}")));
    row.push(tryRun(() => rule.insertRule("y{color:red}", rule.cssRules.length)));
    row.push(tryRun(() => rule.insertRule("@media print{z{color:red}}", 1)));
    row.push(tryRun(() => rule.insertRule("x{", 0)));
    row.push(tryRun(() => rule.insertRule("x{} y{}", 0)));
    row.push(tryRun(() => rule.insertRule("x{}", 99)));
    row.push(tryRun(() => rule.insertRule("@import url(a.css);", 0)));
    row.push(tryRun(() => rule.insertRule()));
    row.push(rule.cssText);
    const inserted = rule.cssRules[0];
    row.push(tryRun(() => rule.deleteRule(0)));
    row.push(inserted.parentRule === null, inserted.parentStyleSheet === null);
    row.push(tryRun(() => rule.deleteRule(99)));
    row.push(tryRun(() => rule.deleteRule()));
    row.push(rule.cssRules.length, rule.cssText);
    mutations.push(row);
  }
  const interfaces = [
    "CSSRule",
    "CSSGroupingRule",
    "CSSConditionRule",
    "CSSContainerRule",
    "CSSMediaRule",
    "CSSSupportsRule",
    "CSSLayerBlockRule",
    "CSSLayerStatementRule",
    "CSSScopeRule",
    "CSSStartingStyleRule",
  ].map((name) => {
    const ctor = globalThis[name];
    if (typeof ctor !== "function") return [name, "<missing>"];
    const own = Object.getOwnPropertyNames(ctor.prototype)
      .filter((key) => key !== "constructor")
      .map((key) => {
        const descriptor = Object.getOwnPropertyDescriptor(ctor.prototype, key);
        return [key, descriptor.get ? "get" : "value", !!descriptor.set, descriptor.enumerable];
      })
      .sort((a, b) => (a[0] < b[0] ? -1 : 1));
    return [
      name,
      Object.getPrototypeOf(ctor).name,
      ctor.length,
      tryRun(() => new ctor()),
      tryRun(() => ctor()),
      Object.prototype.toString.call(ctor.prototype),
      own,
      Object.getOwnPropertyDescriptor(globalThis, name).enumerable,
    ];
  });
  const getter = (ctor, name) => Object.getOwnPropertyDescriptor(ctor.prototype, name).get;
  const illegal = [
    tryRun(() => getter(CSSContainerRule, "containerName").call(sheet.cssRules[3])),
    tryRun(() => getter(CSSConditionRule, "conditionText").call(sheet.cssRules[0])),
    tryRun(() => getter(CSSConditionRule, "conditionText").call({})),
    tryRun(() => CSSGroupingRule.prototype.insertRule.call(sheet.cssRules[7], "x{}")),
    tryRun(() => CSSGroupingRule.prototype.insertRule.call({}, "x{}")),
    tryRun(() => getter(CSSGroupingRule, "cssRules").call(sheet.cssRules[7])),
    tryRun(() => getter(CSSMediaRule, "media").call(sheet.cssRules[0])),
  ];
  const inserted = [
    "@container card (min-width:200px){i{width:1px}}",
    "@container (width > 10px) { }",
    "@media print{}",
    "@supports (color:red){}",
    "@layer a.b{}",
    "@layer a, b.c;",
    "@scope (.x){}",
    "@starting-style{}",
  ].map((text) => {
    const at = sheet.insertRule(text, sheet.cssRules.length);
    const rule = sheet.cssRules[at];
    return [
      rule.constructor.name,
      rule.type,
      rule.cssText,
      fields.map((name) => [name, attr(rule, name)]),
    ];
  });
  const media = sheet.cssRules[3].media;
  const mediaRows = [media === sheet.cssRules[3].media, media.length, media.mediaText];
  media.appendMedium("print");
  mediaRows.push(sheet.cssRules[3].conditionText, sheet.cssRules[3].cssText.split("\n")[0]);
  return { rules, mutations, interfaces, illegal, inserted, mediaRows };
})();

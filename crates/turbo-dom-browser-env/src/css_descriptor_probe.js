(() => {
  const rules = [
    '@property --paint { syntax: "<color>"; initial-value: transparent; inherits: false; }',
    '@property --count { syntax: "<number>"; inherits: true; initial-value: 2; }',
    '@property --anything { syntax: "*"; inherits: false; }',
    '@property --broken { syntax: "<color>"; inherits: false; }',
    '@property plain { syntax: "*"; inherits: false; }',
    '@view-transition { navigation: auto; }',
    '@view-transition { navigation: none; types: card modal; }',
    '@view-transition { navigation: nonsense; width: 20px; }',
  ];
  const normalize = value => value.replace(/\s+/g, " ").replace(/\{\s*\}/g, "{}");
  return rules.map(source => {
    const sheet = new CSSStyleSheet();
    let error = null;
    try { sheet.insertRule(source); } catch (value) { error = value.name; }
    return { error, rules: Array.from(sheet.cssRules, rule => ({type: rule.type, childRules: "cssRules" in rule, cssText: normalize(rule.cssText)})) };
  });
})();

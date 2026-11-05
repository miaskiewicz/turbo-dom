(() => {
  'use strict';
  // turbo-dom#358: CSS Nesting (CSSStyleRule.cssRules / insertRule / deleteRule, cssText and
  // the cascade of nested rules): parsing styles the engine must keep nested, the CSSOM read
  // through .cssRules and .cssText, insertRule / deleteRule, and getComputedStyle of three
  // elements per sheet (an outer, an inner, a grandchild).
  const sheets = [
    '.outer { color: red; & .inner { color: blue; } }',
    '.outer { color: red; .inner { color: blue; } }',
    '.outer { color: red; &.outer { color: green; } }',
    '.outer { color: red; & > .inner { color: blue; } }',
    '.outer { color: red; & .inner { color: blue; & #c { color: lime; } } }',
    '.outer { color: red; @media screen { color: blue; } }',
    '.outer { color: red; @media screen { color: blue; & .inner { color: lime; } } }',
    '.outer { color: red; & .inner, & #c { color: blue; } }',
    '.outer { color: red; & .inner { color: blue } color: cyan; }',
    '.outer { color: red; &:hover .inner { color: blue; } }',
    'div { color: red; .outer { color: blue; } }',
    '.outer { color: red; .inner { color: green; }  &.outer { color: magenta; } }',
    '.outer { & { color: red; } color: blue; }',
    '.a { color: red; } .outer { color: red; .inner { color: blue; } }',
    '.outer { color: red; @supports (color: red) { color: blue; } }',
    '.outer { color: red; @layer x { color: blue; } }',
    '.outer { color: red; & { color: blue; } }',
    '.outer { color: red; & .inner { .deep { color: blue; } } }',
    '.outer { .inner { color: blue; } color: red; .inner { color: green; } }',
  ];
  const ids = ['a', 'b', 'c'];
  const results = sheets.map((text) => {
    const style = document.createElement('style');
    style.textContent = text;
    document.head.append(style);
    const sheet = style.sheet;
    const dump = (rule) => rule && ({ text: rule.cssText, selector: rule.selectorText || null, kind: rule.constructor.name, children: Array.from(rule.cssRules || []).map(dump) });
    const result = {
      sheet: sheet.cssRules[0] ? dump(sheet.cssRules[0]) : null,
      colors: ids.map((id) => getComputedStyle(document.getElementById(id)).color),
    };
    style.remove();
    return [text, result];
  });
  // CSSOM mutations: insertRule inside a style rule, deleteRule, selectorText set.
  const style = document.createElement('style');
  style.textContent = '.outer { color: red; }';
  document.head.append(style);
  const rule = style.sheet.cssRules[0];
  const mutations = [];
  const record = (label, action) => {
    try { action(); mutations.push([label, rule.cssText, ids.map((id) => getComputedStyle(document.getElementById(id)).color)]); }
    catch (error) { mutations.push([label, error.name + ': ' + error.message]); }
  };
  record('after .inner insert', () => rule.insertRule('.inner { color: blue; }'));
  record('after nested-nested insert', () => rule.cssRules[0].insertRule('#c { color: lime; }'));
  record('after delete first', () => rule.deleteRule(0));
  record('insert at index past end throws', () => rule.insertRule('& #c { color: lime; }', 5));
  record('insert declaration bare throws', () => rule.insertRule('color: blue', 0));
  style.remove();
  return { results, mutations };
})()

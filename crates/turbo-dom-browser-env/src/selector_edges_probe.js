(async () => {
  // Selector admission and serialization edges through querySelectorAll,
  // insertRule / selectorText and stylesheet parsing: attribute operators and namespace
  // prefixes, combinator placement, compound pseudo-element arguments, An+B serialization and
  // the "of S" clause, plus at-rule preludes and declaration serialization in the CSSOM.
  const selectors = [
    '[a=b]', '[a~=b]', '[a|=b]', '[a^=b]', '[a$=b]', '[a*=b]', '[a!=b]', '[a=b i]', '[a=b s]', '[a="b" x]', '[a b]', '[*|a]', '[|a]', '[*|a=b]', '[ns|a]', '[*a]', '[a|]', '[1]',
    '*|*', '|b', '*|b', 'ns|b', '*|', '*b', 'a|', '.', '.1', '#', 'a,', ',a', 'a,,b', 'a b', 'a > b', 'a >', '> a', 'a ~ ~ b', 'a + > b',
    'a::before b', '::before', 'a::before.x', '::slotted(b)', '::slotted(*)', '::slotted(.x)', '::slotted(b c)', '::part(x)', '::part()', '::highlight(x)', '::cue(b)',
    ':nth-child(n)', ':nth-child(1n)', ':nth-child(-n)', ':nth-child(-1n+0)', ':nth-child(+n+1)', ':nth-child(2n-1)', ':nth-child(0n+3)', ':nth-child(n of b)', ':nth-child(n off b)', ':nth-child(n of ::before)', ':nth-last-child(2n of .x, b)', ':nth-of-type(n of b)', ':nth-child(3 x)',
    ':is(a b, c)', ':is()', ':where(,a)', ':not(a, ::before)', ':has(> a)', ':has()', ':host(b)', ':host(b c)', ':host', ':host-context(b)', ':state(x)', ':state()', ':dir(ltr)', ':lang(en, "fr")',
    'svg|a', 'a::after::before', '::before:hover', '::-webkit-scrollbar:horizontal', ':horizontal', '::-webkit-scrollbar-thumb:window-inactive', 'a:hover::before',
  ];
  const result = {};
  const sheet = document.head.appendChild(document.createElement('style')).sheet;
  for (const selector of selectors) {
    let query;
    try { document.querySelectorAll(selector); query = 'ok'; } catch (e) { query = e.name; }
    let inserted;
    try { sheet.insertRule(selector + '{}', 0); inserted = sheet.cssRules[0].selectorText; sheet.deleteRule(0); } catch (e) { inserted = e.name; }
    const style = document.createElement('style');
    style.textContent = selector + '{color:red}';
    document.head.append(style);
    const parsed = style.sheet.cssRules.length ? style.sheet.cssRules[0].selectorText : null;
    style.remove();
    result[selector] = [query, inserted, parsed];
  }
  // selectorText assignment keeps the old text when the new list is invalid.
  sheet.insertRule('p{}', 0);
  const rule = sheet.cssRules[0];
  const assigned = [];
  for (const text of ['q', 'q >', '[a~=b]', '*|*', ':nth-child(1n+0)']) { rule.selectorText = text; assigned.push(rule.selectorText); }
  // At-rule preludes and declarations the CSSOM parser serializes.
  const rules = [
    '@keyframes k { from { color: red } }', '@keyframes "k2" { 0% { color: red } }', '@keyframes 1k { }', '@keyframes none { }', '@keyframes initial { }',
    '@font-face { font-feature-settings: "liga" 1 }', '@font-face { font-variation-settings: "wght" 400 }', '@font-face { font-feature-settings: liga }', '@font-face { font-variation-settings: normal }',
    '@counter-style c { system: cyclic; symbols: a }', '@counter-style none { system: cyclic }', '@property --p { syntax: "*"; inherits: false }', '@property p { syntax: "*" }',
    '@view-transition { navigation: auto }', '@view-transition { navigation: bogus }', '@page :first { margin: 1px }', '@page :bogus { }', '@layer a, b;', '@layer a b;', '@container (width > 1px) { }', '@container bogus bogus { }', '@scope (a) to (b) { }', '@starting-style { a { color: red } }',
    'a { zoom: 2 !important }', 'a { zoom: 50% }', 'a { zoom: bogus }', 'a { --x: var(--y) }', 'a { color: var(--x) !important }', 'a { margin: var(--x) 1px }', 'a { scroll-timeline: --t block }',
  ];
  const at = {};
  for (const text of rules) {
    const style = document.createElement('style');
    style.textContent = text;
    document.head.append(style);
    const list = style.sheet.cssRules;
    at[text] = list.length ? list[0].cssText : null;
    let inserted;
    try { sheet.insertRule(text, 0); inserted = sheet.cssRules[0].cssText; sheet.deleteRule(0); } catch (e) { inserted = e.name; }
    at[text] = [at[text], inserted];
    style.remove();
  }
  return { result, assigned, at };
})()

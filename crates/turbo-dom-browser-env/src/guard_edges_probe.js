(async () => {
  // Hostile-input edges of the selector rewriters (:popover-open, :modal / :open,
  // :active-view-transition), the selector admission grammar, the declaration parser, keyframe
  // values and element-scoped view-transition roots.
  // Every row is the result, or the thrown exception's name.
  const out = {}, body = document.body;
  const caught = run => { try { const value = run(); return value === undefined ? 'undefined' : value; } catch (error) { return 'E:' + error.name; } };
  const box = body.appendChild(document.createElement('div'));

  // Selector rewriters and admission: querySelector / matches / closest / insertRule.
  {
    const r = {};
    const pop = box.appendChild(document.createElement('div'));
    pop.popover = 'manual'; pop.id = 'pop';
    const dialog = box.appendChild(document.createElement('dialog'));
    const selectors = [
      ':popover-open(x)', ':POPOVER-OPEN()', ':popover-open, )', ':popover-open:is(', 'div:popover-open)', ':is(:popover-open(x))',
      ':modal(x)', ':open(x)', ':modal, )', ':open, )', ':modal)', ':is(:modal(x))', ':where(:open(x), div)',
      ':active-view-transition(x)', ':active-view-transition, )', ':active-view-transition)',
      ':active-view-transition-type(a), )', ':active-view-transition-type(a))', ':active-view-transition-type()',
      'a!', 'a)', 'a]', '*|1', 'svg|1', '*|*', '|a', '::view-transition-group(1)', '::view-transition-group()',
      '::view-transition-group(.)', '::view-transition-group(*.a)', ':is(a!)', ':where(a)b', ':not(a, )',
      ':has(> a, )', 'div:popover-open', 'dialog:modal', 'dialog:open',
      ':where(:popover-open(x), div)', ':is( :popover-open() , :popover-open)', ':modal, [a', ':open, :state(x', ':popover-open, :-webkit-any(a',
      ':active-view-transition, [a', ':popover-open, [a', ':popover-open /* x', ':modal, a /* x', ':is(:active-view-transition(x))',
      'a"x"', 'a{}', 'a(b)', 'a#', 'a.b"c"', 'a::before b', 'a[x]"y"'
    ];
    for (const selector of selectors) {
      r[selector] = [
        caught(() => String(document.querySelector(selector))),
        caught(() => pop.matches(selector)),
        caught(() => String(dialog.closest(selector))),
      ];
    }
    pop.showPopover();
    r.openPopover = [caught(() => document.querySelectorAll(':popover-open').length), caught(() => document.querySelectorAll(':popover-open, )').length)];
    pop.hidePopover();
    out.selectors = r;
  }

  // Declarations: zoom, scroll keywords and ordinary properties, with and without !important.
  {
    const r = {};
    const sheet = new CSSStyleSheet();
    const decls = [
      'zoom: 2', 'zoom: 2 !important', 'zoom: 200% !IMPORTANT', 'zoom: var(--z) !important', 'zoom: var(--z)', 'zoom: env(x)', 'zoom: 2 !important x',
      'scroll-behavior: smooth !important', 'scroll-snap-align: start end !important', 'scroll-behavior: smooth',
      'color: red !important', '--x: 1 !important', 'COLOR: red', 'font-variant: small-caps !important', 'writing-mode: vertical-rl !important'
    ];
    for (const decl of decls) {
      r[decl] = [
        caught(() => { sheet.replaceSync(`p { ${decl}; }`); return sheet.cssRules[0].style.cssText; }),
        caught(() => { const el = document.createElement('p'); el.style.cssText = decl; return el.style.cssText; }),
      ];
    }
    const rules = ['@keyframes k { 0% {} }', '@keyframes k { x {} }', '@keyframes k { 10%, 20% {} from {} }', '@keyframes k { -1% {} }', '@keyframes k { 101% {} }',
      '@font-face { font-family: f; src: url(a) }', '@font-face { font-weight: 300 }', '@font-face { color: red }',
      '@view-transition { navigation: auto }', '@view-transition { types: a b }', '@property --p { syntax: "<length>"; inherits: false; initial-value: 1px }',
      '@counter-style c { system: cyclic; symbols: a }'];
    for (const rule of rules) {
      r[rule] = caught(() => { sheet.replaceSync(rule); return sheet.cssRules.length ? sheet.cssRules[0].cssText : 'none'; });
    }
    out.declarations = r;
  }

  // createDocument's doctype argument, unnamed radio groups, labels of foreign elements and
  // an inert document's style sheet.
  {
    const r = {};
    r.createDocument = [
      caught(() => String(document.implementation.createDocument(null, 'a', document.createElement('x')))),
      caught(() => String(document.implementation.createDocument(null, 'a', document.createTextNode('x')))),
      caught(() => String(document.implementation.createDocument(null, 'a', {}))),
    ];
    const form = box.appendChild(document.createElement('form'));
    form.innerHTML = '<input type=radio name=""><input type=radio name=""><input type=radio><input type=radio>';
    const radios = [...form.querySelectorAll('input')];
    radios.forEach(radio => { radio.checked = true; });
    r.radios = [...radios.map(radio => radio.checked), form.querySelectorAll(':checked').length];
    radios[0].click();
    r.radiosClick = [radios[0].checked, radios[1].checked];
    form.remove();
    box.innerHTML = '<label id=gl for=gs>x</label><svg><circle id=gs /></svg><label id=gm for=gt>y</label><math id=gt></math>';
    r.labels = [String(document.getElementById('gl').control), String(document.getElementById('gm').control)];
    box.replaceChildren();
    const doc = document.implementation.createHTMLDocument();
    const style = doc.head.appendChild(doc.createElement('style'));
    style.textContent = 'p { color: red }';
    r.inertSheet = [String(style.sheet), style.sheet && style.sheet.cssRules.length];
    out.misc = r;
  }

  // Keyframe values that do not tokenize as a declaration value are dropped.
  {
    const r = {};
    const el = box.appendChild(document.createElement('div'));
    for (const value of ['1px)', '1px]', '1px}', '1px;', 'var(--x))', '(1px', 'red !important']) {
      r[value] = caught(() => { const frames = el.animate([{ width: value }, { width: '2px' }]).effect.getKeyframes(); return [frames[0].width === undefined ? 'none' : frames[0].width, frames.length]; });
    }
    el.getAnimations().forEach(animation => animation.cancel());
    out.keyframes = r;
  }

  // Element-scoped view transitions: which UA / authored displays give the root a box.
  {
    const r = {};
    const svg = 'http://www.w3.org/2000/svg', math = 'http://www.w3.org/1998/Math/MathML';
    const settle = promise => promise.then(() => 'ok', error => 'E:' + error.name);
    const make = (tag, style, parent = box) => {
      const element = parent.appendChild(document.createElement(tag));
      if (style) element.setAttribute('style', style);
      return element;
    };
    const cases = {
      inheritBlock: () => make('span', 'display: inherit', make('div')),
      inheritInline: () => make('span', 'display: inherit', make('span')),
      inheritFromFlex: () => make('span', 'display: inherit', make('div', 'display: flex')),
      rubyBase: () => make('div', 'display: ruby-base'),
      rubyText: () => make('div', 'display: ruby-text-container'),
      revert: () => make('div', 'display: revert'),
      unknown: () => make('div', 'display: bogus'),
      hiddenInput: () => { const input = make('input'); input.type = 'HIDDEN'; return input; },
      textInput: () => make('input'),
      nestedSvg: () => box.appendChild(document.createElementNS(svg, 'svg')).appendChild(document.createElementNS(svg, 'svg')),
      svgRect: () => box.appendChild(document.createElementNS(svg, 'svg')).appendChild(document.createElementNS(svg, 'rect')),
      mathml: () => box.appendChild(document.createElementNS(math, 'math')),
      mathmlBlock: () => { const element = box.appendChild(document.createElementNS(math, 'math')); element.setAttribute('style', 'display: block'); return element; },
      rubyBaseSpan: () => make('span', 'display: ruby-base', make('span')),
      rubyTextDiv: () => make('div', 'display: ruby-text'),
      rubyBaseContainer: () => make('div', 'display: ruby-base-container'),
      mathInSpan: () => make('span').appendChild(document.createElementNS(math, 'math')),
      mrow: () => box.appendChild(document.createElementNS(math, 'math')).appendChild(document.createElementNS(math, 'mrow')),
      mathInline: () => { const element = box.appendChild(document.createElementNS(math, 'math')); element.setAttribute('style', 'display: inline'); return element; },
      otherNamespace: () => box.appendChild(document.createElementNS('urn:x', 'svg')),
      inHiddenSvg: () => box.appendChild(document.createElementNS(svg, 'svg')).appendChild(document.createElementNS(svg, 'svg')).appendChild(document.createElement('div')),
    };
    for (const [name, create] of Object.entries(cases)) {
      const element = create();
      if (element.namespaceURI === document.documentElement.namespaceURI && element.localName !== 'input') element.textContent = 'x';
      let transition;
      try { transition = element.startViewTransition(); } catch (error) { r[name] = 'E:' + error.name; box.replaceChildren(); continue; }
      const ready = await settle(transition.ready);
      transition.skipTransition();
      r[name] = [ready, await settle(transition.finished)];
      box.replaceChildren();
    }
    out.transitions = r;
  }

  box.remove();
  return out;
})();

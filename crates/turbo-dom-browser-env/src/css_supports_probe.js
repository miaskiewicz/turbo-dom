(() => {
  'use strict';
  // turbo-dom#322: CSS.supports() for the Chrome 156 features turbo-surf's identity battery
  // checks, their neighbours that must stay unsupported, both call forms, and the value a
  // declaration's style then holds.
  const declarations = [
    'anchor-name: --a', 'anchor-name: --a, --b', 'anchor-name: none', 'anchor-name: a', 'anchor-name: --a --b',
    'anchor-name: --a,', 'position-anchor: --a', 'position-anchor: auto', 'position-anchor: none', 'position-anchor: normal', 'position-anchor: a', 'position-anchor: --a, --b',
    'anchor-scope: --a, --b', 'anchor-scope: --a --b', 'timeline-scope: --t, --u', 'timeline-scope: --t --u',
    'position-try-fallbacks: flip-block', 'interpolate-size: allow-keywords', 'field-sizing: content',
    'text-box: trim-both cap alphabetic', 'reading-flow: flex-visual', 'corner-shape: squircle', 'dynamic-range-limit: standard',
    'width: calc(sibling-index() * 1px)', 'width: calc(1px * sibling-count())', 'order: sibling-index()', 'z-index: calc(sibling-count() + 1)', 'width: sibling-index()', 'width: calc(sibling-index(1) * 1px)', 'opacity: calc(1 / sibling-count())',
    'color: if(style(--x: 1): red; else: blue)', 'color: if(media(width > 1px): red)', 'width: if(supports(display: grid): 1px; else: 2px)', 'color: if(red)', 'color: if()', 'color: if(: red)', 'color: if(style(--x); red)', 'color: if(not (style(--x)) and media(print): red; or: blue)', 'color: if(not (style(--x)) and media(print): (red); else: blue;)', 'color: if(else: red;)', 'color: if(else: red;;)', 'margin: if(style(--x: 1): 1px)',
    'clip-path: shape(from 0 0, line to 1px 1px)', 'clip-path: shape(evenodd from 10% 20%, hline by 5px, vline to 3em, curve to 1px 2px with 3px 4px, smooth by 1px 1px, arc to 10px 10px of 5px cw large rotate 10deg, close)', 'clip-path: shape(from 0 0, move to 1px 1px, line by 2px 2px, curve by 1px 1px with 0 0 / 2px 2px, smooth to 3px 3px with 1px 1px, close)', 'offset-path: shape(from 0 0, line to 1px 1px)', 'clip-path: shape(line to 1px 1px)', 'clip-path: shape(from 0 0)', 'clip-path: shape(from 0 0, line 1px 1px)', 'clip-path: shape(from 0 0, close) border-box',
    'scroll-marker-group: after', 'view-transition-class: a', 'text-wrap: pretty', 'color: contrast-color(red)', 'font-size: 1rcap', 'anchor-scope: all', 'overlay: auto', 'timeline-scope: --t', 'content-visibility: auto',
    'color: red !important', 'colour: red', 'color: bogus',
  ];
  const div = document.createElement('div');
  const style = (text) => { div.removeAttribute('style'); const i = text.indexOf(':'); div.style.setProperty(text.slice(0, i).trim(), text.slice(i + 1).trim()); return div.style.getPropertyValue(text.slice(0, i).trim()); };
  const out = { declarations: declarations.map((text) => { const i = text.indexOf(':'); return [text, CSS.supports(text), CSS.supports('(' + text + ')'), CSS.supports(text.slice(0, i), text.slice(i + 1).trim()), style(text)]; }) };
  out.forms = [
    CSS.supports('not (color: bogus)'), CSS.supports('(color: red) and (anchor-name: --a)'), CSS.supports('(color: red) or (colour: red)'), CSS.supports('(color: red) and (colour: red)'),
    CSS.supports('selector(:has(a))'), CSS.supports('color', 'red !important'), CSS.supports(' color', 'red'), CSS.supports('COLOR', 'red'), CSS.supports('--x', 'anything {}'), CSS.supports('--X', '1'), CSS.supports('color: red;'), CSS.supports('color', ''),
  ];
  return out;
})()

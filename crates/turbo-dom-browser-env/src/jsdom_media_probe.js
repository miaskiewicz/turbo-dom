(() => {
  // turbo-dom MediaMode::Jsdom: jsdom 27.4.0 has no matchMedia, and its getComputedStyle
  // applies a top-level @media rule only when its media list has an item that is exactly
  // `screen` (forEachMatchingSheetRuleOfElement); a <style media> attribute is not read.
  const cases = [
    ['a', '@media screen'], ['b', '@media SCREEN'], ['c', '@media screen, print'], ['d', '@media screen and (min-width:0)'],
    ['e', '@media all'], ['f', '@media (min-width:0)'], ['g', '@media print'], ['h', '@media not screen'],
    ['j', '@media  screen '], ['k', '@media only screen'], ['m', '@media (min-width: 900px)'], ['n', '@media (max-width: 100px)'],
  ];
  let css = '';
  for (const [id, prelude] of cases) css += `#${id} { color: rgb(0, 0, 1) } ${prelude} { #${id} { color: rgb(0, 0, 2) } } `;
  css += '#i { color: rgb(0, 0, 1) } @media screen { @media screen { #i { color: rgb(0, 0, 2) } } } ';
  css += '#l { color: rgb(0, 0, 1) } #o { color: rgb(0, 0, 2) } ';
  const ids = cases.map(([id]) => id).concat(['i', 'l', 'o']);
  document.body.innerHTML = `<style>${css}</style><style media="print">#l { color: rgb(0, 0, 2) }</style><style media="(max-width: 100px)">#o { color: rgb(0, 0, 3) }</style>` + ids.map(id => `<div id="${id}"></div>`).join('');
  const out = {};
  for (const id of ids) out[id] = getComputedStyle(document.getElementById(id)).color;
  out.matchMedia = typeof window.matchMedia;
  out.inWindow = ['matchMedia', 'MediaQueryList', 'MediaQueryListEvent'].map(name => name in window);
  return out;
})()

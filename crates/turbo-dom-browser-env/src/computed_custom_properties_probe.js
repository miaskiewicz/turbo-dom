(() => {
  // turbo-dom#286: getComputedStyle reports custom properties (inline, stylesheet,
  // style attribute, inherited) with their specified text.
  const style = document.head.appendChild(document.createElement('style'));
  style.textContent = ':root { --root-color: #00f; --spaced:   a   b  ; } .sheet { --from-sheet: #00ff00; --over: sheet; } #win { --over: id; } .child { --child-only: 1px }';
  document.body.innerHTML = '<div id=outer class=sheet style="--g:#ABC; --over: inline"><div id=win class="sheet"><span id=leaf class=child></span></div></div><p id=plain></p>';
  const outer = document.getElementById('outer'), win = document.getElementById('win'), leaf = document.getElementById('leaf'), plain = document.getElementById('plain');
  outer.style.setProperty('--a', '#ff0000');
  const names = ['--a', '--g', '--from-sheet', '--over', '--root-color', '--spaced', '--child-only', '--missing'];
  const read = el => { const cs = getComputedStyle(el); const row = {}; for (const name of names) row[name] = cs.getPropertyValue(name); return row; };
  const out = { outer: read(outer), win: read(win), leaf: read(leaf), plain: read(plain), root: read(document.documentElement) };
  outer.style.setProperty('--a', 'changed');
  out.afterChange = getComputedStyle(leaf).getPropertyValue('--a');
  const detached = document.createElement('div');
  detached.style.setProperty('--d', '1');
  out.detached = getComputedStyle(detached).getPropertyValue('--d');
  style.remove();
  document.body.innerHTML = '';
  return out;
})()

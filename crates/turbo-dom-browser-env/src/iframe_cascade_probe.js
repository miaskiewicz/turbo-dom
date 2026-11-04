(async () => {
  // turbo-dom#299: a child iframe document cascades its own style sheets (inline styles,
  // <style> sheets, inheritance, custom properties, @supports / @layer / specificity) with
  // @media evaluated against the iframe's viewport, from the child window and the parent.
  const frames = () => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
  const out = {};
  const topSheet = document.createElement('style');
  topSheet.textContent = '#outer { color: rgb(1, 2, 3); background-color: rgb(1, 1, 1); margin-left: 0px; padding-top: 0px; } @media (max-width: 600px) { #outer { margin-left: 4px; } } @media (min-width: 900px) { #outer { padding-top: 6px; } } span { color: rgb(9, 9, 9); }';
  document.head.appendChild(topSheet);
  const outer = document.createElement('p');
  outer.id = 'outer';
  document.body.appendChild(outer);
  const iframe = document.createElement('iframe');
  iframe.setAttribute('width', '400');
  iframe.setAttribute('height', '200');
  iframe.srcdoc = '<style>' +
    '* { background-color: rgb(1, 1, 1); margin-left: 0px; padding-top: 0px; }' +
    '#a { color: rgb(0, 128, 0); font-size: 20px; --accent: rgb(10, 20, 30); }' +
    '@media (min-width: 500px) { #a { color: rgb(0, 0, 255); margin-left: 5px; } }' +
    '@media (max-width: 350px) { #b { background-color: rgb(255, 0, 0); } }' +
    '@supports (display: grid) { #b { display: block; } }' +
    '@supports (display: no-such-display) { #b { display: flex; } }' +
    '@layer base { #b { padding-top: 3px; margin-left: 9px; } }' +
    '#b { margin-left: 2px; } div > #b.k { margin-left: 7px; } span#b { margin-left: 1px; }' +
    '</style><div id="a"><span id="b" class="k">x</span><i id="c" style="color: rgb(4, 5, 6); padding-top: 8px">y</i></div><em id="d">z</em>';
  document.body.appendChild(iframe);
  for (let i = 0; i < 500 && !(iframe.contentDocument && iframe.contentDocument.getElementById('d')); i++) await frames();
  const w = iframe.contentWindow, doc = iframe.contentDocument;
  const props = ['color', 'background-color', 'margin-left', 'padding-top', 'font-size', '--accent', 'display'];
  const read = (view, id) => { const el = id === 'outer' ? outer : doc.getElementById(id); const style = view.getComputedStyle(el); return props.map(name => style.getPropertyValue(name)); };
  const all = view => ['a', 'b', 'c', 'd'].map(id => read(view, id));
  out.child = all(w);
  out.parent = all(window);
  out.outer = [read(window, 'outer'), read(w, 'outer')];
  iframe.setAttribute('width', '700');
  await frames();
  out.wide = [w.innerWidth, all(w), all(window)];
  iframe.style.width = '300px';
  await frames();
  out.narrow = [w.innerWidth, all(w), all(window)];
  const injected = doc.createElement('style');
  injected.textContent = '#d { color: rgb(7, 8, 9); } @media (max-width: 320px) { #d { margin-left: 11px; } }';
  doc.body.appendChild(injected);
  doc.getElementById('a').style.paddingTop = '12px';
  out.injected = [read(w, 'a'), read(w, 'd'), read(window, 'd')];
  iframe.removeAttribute('style');
  await frames();
  out.restored = [read(w, 'a'), read(w, 'd')];
  iframe.remove();
  topSheet.remove();
  outer.remove();
  return out;
})()

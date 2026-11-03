(() => {
  // Inline style writes and the computed styles they can change (turbo-dom#271): reads are
  // interleaved with the writes, so a reused computed style that should have changed shows.
  const out = {};
  const sheet = document.createElement('style');
  sheet.textContent = '[style*="orange"] + p { color: blue } section:has(> [style*="gold"]) { color: green } .t { color: red }';
  document.head.appendChild(sheet);
  const host = document.createElement('div');
  host.innerHTML = '<div id=a><span id=s>x</span></div><b id=b></b><p id=p>p</p><section id=sec><i id=i></i></section>';
  document.body.appendChild(host);
  const $ = (id) => document.getElementById(id);
  const read = () => [getComputedStyle($('s')).color, getComputedStyle($('p')).color, getComputedStyle($('sec')).color, getComputedStyle($('a')).color].join('|');
  out.selectors = [read()];
  $('a').style.color = 'rgb(1, 2, 3)'; out.selectors.push(read());
  $('b').setAttribute('style', 'background: orange'); out.selectors.push(read());
  $('i').style.outlineColor = 'gold'; out.selectors.push(read());
  $('a').removeAttribute('style'); $('b').removeAttribute('style'); $('i').removeAttribute('style'); out.selectors.push(read());
  // An unrelated element's inline writes, deep in the tree: the other element keeps its style.
  let parent = document.body;
  for (let k = 0; k < 30; k++) { const d = document.createElement('div'); parent.appendChild(d); parent = d; }
  const target = document.createElement('textarea'); target.className = 't'; parent.appendChild(target);
  const hidden = document.createElement('textarea'); parent.appendChild(hidden);
  out.unrelated = [getComputedStyle(target).color];
  for (let k = 0; k < 5; k++) { hidden.style.width = k + 'px'; out.unrelated.push(getComputedStyle(target).color + '|' + getComputedStyle(hidden).width); }
  // An adopted sheet's `style` attribute selector.
  const e = document.createElement('em'), q = document.createElement('q'); q.textContent = 'q';
  document.body.append(e, q);
  const adopted = new CSSStyleSheet(); adopted.replaceSync('[style*="teal"] + q { color: rgb(0, 128, 128) }');
  document.adoptedStyleSheets = [adopted];
  out.adopted = [getComputedStyle(q).color];
  e.style.borderColor = 'teal'; out.adopted.push(getComputedStyle(q).color);
  e.removeAttribute('style'); out.adopted.push(getComputedStyle(q).color);
  document.adoptedStyleSheets = [];
  // A shadow tree's ::slotted rule over a light element's style attribute.
  const shadowHost = document.createElement('div'), light = document.createElement('span'); light.textContent = 'x';
  shadowHost.appendChild(light); document.body.appendChild(shadowHost);
  const root = shadowHost.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>::slotted([style*="lime"]) { color: rgb(9, 9, 9) }</style><slot></slot>';
  out.slotted = [getComputedStyle(light).color];
  light.style.backgroundColor = 'lime'; out.slotted.push(getComputedStyle(light).color);
  return JSON.stringify(out);
})()

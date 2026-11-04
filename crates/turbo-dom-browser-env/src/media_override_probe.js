(async () => {
  // turbo-dom 0.7.1 app regressions: getComputedStyle evaluates @media against the
  // viewport itself; a page that replaces window.matchMedia (a test mock) is never asked.
  const style = document.createElement('style');
  style.textContent = '#a { margin-left: 1px; color: rgb(0, 0, 1) } @media (min-width: 600px) { #a { margin-left: 2px } } @media (max-width: 600px) { #a { color: rgb(0, 0, 3) } }';
  document.head.append(style);
  const a = document.createElement('div');
  a.id = 'a';
  document.body.append(a);
  const read = () => [getComputedStyle(a).marginLeft, getComputedStyle(a).color, getComputedStyle(a).paddingLeft];
  const own = window.matchMedia;
  const before = read();
  let calls = 0;
  window.matchMedia = query => { calls++; return { matches: !query.includes('min-width'), media: query, addListener() {}, removeListener() {} }; };
  a.className = 'x';
  const mocked = read();
  style.textContent += ' @media (min-width: 700px) { #a { padding-left: 4px } } @media (max-width: 700px) { #a { padding-left: 5px } }';
  const added = read();
  window.matchMedia = own;
  return { before, mocked, added, calls, restored: window.matchMedia === own };
})()

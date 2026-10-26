(() => {
  'use strict';
  // turbo-dom#73: URL-reflecting attributes (src, href) return the attribute parsed against
  // the document base URL — the first <base href> (here https://x.test/dir/), else the document
  // URL — serialized; an unparsable value is returned as is and an absent one is "".
  const values = ['../pic.png', 'pic.png?a=b#c', '/root.png', 'https://other.test/./a/../b.png', '//cdn.test/x.png', 'data:image/png,AA', 'http://[bad', '  spaced.png  ', 'a b.png'];
  const owners = { img: 'src', script: 'src', iframe: 'src', source: 'src', embed: 'src', track: 'src', input: 'src', video: 'src', audio: 'src', link: 'href' };
  const out = {};
  for (const [tag, name] of Object.entries(owners)) {
    const el = document.createElement(tag);
    const row = [el[name]];
    for (const value of values) {
      el.setAttribute(name, value);
      row.push(el[name]);
    }
    el.setAttribute(name, '');
    row.push(el[name]);
    out[tag] = row;
  }
  return out;
})()

(async () => {
  // turbo-dom#287: matchMedia returns a live MediaQueryList evaluated against the
  // viewport (1024x768 at device scale factor 1), firing `change` on a resize.
  const queries = ['(min-width: 600px)', '(max-width: 600px)', '(min-width: 1024px)', '(min-width: 1025px)', '(width: 1024px)', '(height: 768px)', '(min-height: 800px)',
    '(orientation: landscape)', '(orientation: portrait)', 'screen', 'print', 'all', 'not print', 'only screen and (min-width: 100px)', 'not screen and (min-width: 100px)',
    'screen, print', 'print, (max-width: 100px)', '(min-width: 100px) and (max-width: 2000px)', '(aspect-ratio: 4/3)', '(min-aspect-ratio: 1/1)', '(max-aspect-ratio: 1/1)',
    '(min-resolution: 1dppx)', '(min-resolution: 2dppx)', '(resolution: 96dpi)', '(-webkit-min-device-pixel-ratio: 1)', '(prefers-color-scheme: dark)', '(prefers-color-scheme: light)',
    '(prefers-reduced-motion: reduce)', '(prefers-reduced-motion: no-preference)', '(hover: hover)', '(pointer: fine)', '(any-hover: hover)', '(color)', '(monochrome)',
    '(min-width: 40em)', '(max-width: 63.9375em)', '(min-width: 600px', 'garbage', '', '(width >= 600px)', '(400px <= width <= 1100px)', '(width < 1024px)', '(min-width: 600px) , print',
    '(MIN-WIDTH: 600PX)', 'not all and (monochrome)'];
  const out = { queries: {} };
  for (const query of queries) { const list = matchMedia(query); out.queries[query] = [list.matches, list.media]; }
  const list = matchMedia('(min-width: 800px)');
  out.shape = {
    tag: Object.prototype.toString.call(list), isList: list instanceof MediaQueryList, isTarget: list instanceof EventTarget,
    onchange: list.onchange, protoKeys: ['matches', 'media', 'onchange', 'addListener', 'removeListener'].map(key => key in MediaQueryList.prototype),
    own: Object.keys(list), distinct: matchMedia('(min-width: 800px)') !== list,
  };
  const log = [];
  const legacy = event => log.push(['legacy', event.type, event.matches, event.media, event instanceof MediaQueryListEvent, event.target === list]);
  list.addListener(legacy);
  list.addEventListener('change', event => log.push(['listener', event.matches]));
  list.onchange = event => log.push(['onchange', event.matches]);
  const removed = () => log.push(['removed']);
  list.addListener(removed); list.removeListener(removed);
  const narrow = matchMedia('(max-width: 600px)');
  narrow.addEventListener('change', event => log.push(['narrow', event.matches, event.media]));
  const waiters = {}; let next = 0;
  globalThis.__resizeDone = id => { const done = waiters[id]; delete waiters[id]; done(); };
  const resize = (width, height) => new Promise(done => { const id = ++next; waiters[id] = done; __resize(JSON.stringify({ id, width, height })); });
  await resize(500, 768);
  out.narrow = [list.matches, narrow.matches, innerWidth, innerHeight, matchMedia('(orientation: portrait)').matches];
  await resize(700, 768);
  await resize(1024, 768);
  out.wide = [list.matches, narrow.matches, innerWidth];
  out.log = log;
  return out;
})()

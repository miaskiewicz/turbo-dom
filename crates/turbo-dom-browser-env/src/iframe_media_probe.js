(async () => {
  // turbo-dom#288 follow-up: an iframe's width / height attributes are presentational hints
  // (author CSS wins), and a child window evaluates matchMedia against its own viewport (the
  // iframe's content box), firing `change` when the iframe resizes.
  const frames = () => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
  const out = {};
  const sized = (setup) => {
    const iframe = document.createElement('iframe');
    setup(iframe);
    document.body.appendChild(iframe);
    const style = getComputedStyle(iframe), w = iframe.contentWindow;
    const row = [style.width, style.height, w.innerWidth, w.innerHeight];
    iframe.remove();
    return row;
  };
  out.default = sized(() => {});
  out.attributes = sized(iframe => { iframe.setAttribute('width', '250'); iframe.setAttribute('height', '100'); });
  out.authorWins = sized(iframe => { iframe.setAttribute('width', '250'); iframe.setAttribute('height', '100'); iframe.style.cssText = 'width: 300px; height: 150px'; });
  out.authorOneAxis = sized(iframe => { iframe.setAttribute('width', '250'); iframe.setAttribute('height', '100'); iframe.style.height = '300px'; });
  out.attributePixels = sized(iframe => { iframe.setAttribute('width', '120px'); iframe.setAttribute('height', ' 80.5 '); });

  const iframe = document.createElement('iframe');
  iframe.setAttribute('width', '250');
  iframe.setAttribute('height', '100');
  document.body.appendChild(iframe);
  const w = iframe.contentWindow;
  const queries = ['(max-width: 300px)', '(min-width: 600px)', '(width: 250px)', '(height: 100px)', '(orientation: landscape)', '(aspect-ratio: 5/2)', 'screen'];
  out.child = queries.map(query => w.matchMedia(query).matches);
  out.top = queries.map(query => matchMedia(query).matches);
  out.childList = [w.matchMedia('(max-width: 300px)') instanceof w.MediaQueryList, w.matchMedia('(max-width: 300px)').media];
  const narrow = w.matchMedia('(max-width: 300px)'), tall = w.matchMedia('(orientation: portrait)');
  const log = [];
  narrow.addEventListener('change', event => log.push(['narrow', event.matches, event.media]));
  tall.addEventListener('change', event => log.push(['tall', event.matches]));
  iframe.style.width = '400px';
  await frames();
  out.wider = [narrow.matches, w.innerWidth];
  iframe.style.height = '500px';
  await frames();
  iframe.removeAttribute('style');
  await frames();
  out.restored = [narrow.matches, tall.matches, w.innerWidth, w.innerHeight];
  out.log = log;
  iframe.remove();
  return out;
})()

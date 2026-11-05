(() => {
  // turbo-dom#288: a child iframe window's innerWidth / innerHeight are the iframe's
  // content box; outerWidth / outerHeight are the top-level window's.
  const out = {};
  const frame = (setup) => { const iframe = document.createElement('iframe'); setup(iframe); document.body.appendChild(iframe); const w = iframe.contentWindow; const row = [w.innerWidth, w.innerHeight, w.outerWidth === outerWidth, w.outerHeight === outerHeight]; iframe.remove(); return row; };
  out.default = frame(() => {});
  out.attributes = frame(iframe => { iframe.width = '250'; iframe.height = '100'; });
  out.style = frame(iframe => { iframe.style.cssText = 'width: 400px; height: 200px'; });
  out.boxed = frame(iframe => { iframe.style.cssText = 'width: 400px; height: 200px; padding: 10px; border: 5px solid'; });
  out.borderBox = frame(iframe => { iframe.style.cssText = 'box-sizing: border-box; width: 400px; height: 200px; padding: 10px; border: 5px solid'; });
  const live = document.createElement('iframe');
  document.body.appendChild(live);
  const w = live.contentWindow;
  out.liveBefore = [w.innerWidth, w.innerHeight];
  live.style.width = '320px'; live.style.height = '240px';
  out.liveAfter = [w.innerWidth, w.innerHeight];
  live.remove();
  return out;
})()

(() => {
  // CanvasRenderingContext2D fillStyle / strokeStyle / shadowColor (turbo-dom#108): what each
  // assignment reads back, `currentcolor` resolved against the canvas at assignment, and the
  // colour the next fillRect paints. `paint` is the canvas pixel in Chrome; offline it is the
  // colour the op log recorded for that fillRect (the raster replays it), which the test maps
  // to a pixel.
  const props = ['fillStyle', 'strokeStyle', 'shadowColor'];
  const fresh = () => document.createElement('canvas').getContext('2d');
  const paint = (ctx, x) => ctx._ops ? ctx._ops[ctx._ops.length - 1][2] : Array.from(ctx.getImageData(x, 0, 1, 1).data);
  const out = {};
  const initial = fresh();
  out.initial = props.map(p => initial[p]);
  // Every value is assigned over '#123456', so a rejected one reads that back.
  const STRINGS = [
    '#f00', '#0f08', '#00ff00', '#0000ff80', '#ABCDEF', '#AbC', 'rgb(255, 255, 0)', 'rgba(255,0,255,0.5)',
    'rgb(0 255 255 / 50%)', 'rgba(10, 20, 30, 25%)', 'rgb(10 20 30 / 0.25)', 'rgba(0, 0, 0, 0.123456)',
    'rgba(0, 0, 0, 0.001)', 'rgba(1, 2, 3, 1)', 'rgba(1, 2, 3, 0)', 'rgba(0,0,255,2)', 'rgb(300, -5, 128)',
    'rgb(12.9, 100.5, 200.4)', 'rgb(50%, 0%, 100%)', 'rgb(10% 20% 30% / 50%)', 'rgba(100%, 50%, 0%, 0.25)',
    'hsl(0, 100%, 50%)', 'hsl(0,100%,50%)', 'hsl(120deg 100% 25%)', 'hsla(240, 100%, 50%, 0.5)',
    'hsl(60 100% 50% / 25%)', 'hsl(-120, 100%, 50%)', 'hsl(200, 60%, 70%)',
    'transparent', 'TRANSPARENT', 'black', 'red', '  RED  ', 'rebeccapurple', 'CornflowerBlue', 'grey',
    'currentcolor', 'CurrentColor', ' currentColor ',
    '#12345', '#ggg', '#', 'rgb(1,2)', 'rgb(1,2,3,4,5)', 'hsl(0, 100%)', '', ' ', 'none', 'inherit', 'initial',
    'red blue', 'notacolor', 'rgb(', 'url(#a)',
  ];
  out.strings = STRINGS.map(s => {
    const ctx = fresh();
    return [s, props.map(p => { ctx[p] = '#123456'; ctx[p] = s; return ctx[p]; })];
  });
  // Non-string values: IDL conversion to a string (an object's toString counts), a gradient
  // object for the style properties.
  const others = [['5', 5], ['null', null], ['undefined', undefined], ['true', true], ['{}', {}],
    ['toString red', { toString() { return 'red'; } }], ['array', ['blue']]];
  out.others = others.map(([label, v]) => {
    const ctx = fresh();
    return [label, props.map(p => { ctx[p] = '#123456'; ctx[p] = v; return ctx[p]; })];
  });
  {
    const ctx = fresh(), g = ctx.createLinearGradient(0, 0, 1, 1);
    ctx.fillStyle = g; ctx.strokeStyle = g; ctx.shadowColor = '#123456'; ctx.shadowColor = g;
    out.gradient = [ctx.fillStyle === g, ctx.strokeStyle === g, ctx.shadowColor];
    ctx.fillStyle = 'nope';
    out.gradientKept = ctx.fillStyle === g;
    ctx.fillStyle = 'lime';
    out.gradientReplaced = ctx.fillStyle;
  }
  // currentcolor: the canvas's computed colour at assignment; later colour changes don't follow.
  const body = document.body;
  {
    const canvas = document.createElement('canvas');
    canvas.style.color = 'rgb(255,0,0)';
    body.appendChild(canvas);
    const ctx = canvas.getContext('2d');
    const log = [];
    ctx.fillStyle = 'currentcolor';
    log.push(ctx.fillStyle);
    ctx.fillRect(0, 0, 1, 1);
    log.push(paint(ctx, 0));
    canvas.style.color = 'blue';
    log.push(ctx.fillStyle);
    ctx.fillRect(1, 0, 1, 1);
    log.push(paint(ctx, 1));
    ctx.strokeStyle = 'currentColor';
    ctx.shadowColor = 'CURRENTCOLOR';
    log.push(ctx.strokeStyle, ctx.shadowColor);
    ctx.fillStyle = 'currentcolor';
    log.push(ctx.fillStyle);
    ctx.fillRect(2, 0, 1, 1);
    log.push(paint(ctx, 2));
    out.attached = log;
    canvas.remove();
  }
  // Only the connected canvas's own inline `color` counts: an inherited colour, a stylesheet
  // rule or an inline `inherit` / `currentcolor` resolve to black (Chrome 156 measured).
  const resolve = (label, build) => {
    const canvas = document.createElement('canvas');
    const remove = build(canvas);
    const ctx = canvas.getContext('2d');
    for (const p of props) ctx[p] = 'currentcolor';
    const read = props.map(p => ctx[p]);
    remove();
    return [label, read];
  };
  const wrapped = (color, canvasColor) => canvas => {
    const wrap = document.createElement('div');
    wrap.style.color = color;
    if (canvasColor) canvas.style.color = canvasColor;
    wrap.appendChild(canvas);
    body.appendChild(wrap);
    return () => wrap.remove();
  };
  const inline = (color, extra) => canvas => {
    canvas.setAttribute('style', color);
    body.appendChild(canvas);
    return () => canvas.remove();
  };
  out.resolution = [
    resolve('inherited', wrapped('rgb(0, 128, 0)')),
    resolve('inheritedTranslucent', wrapped('rgba(0, 128, 0, 0.5)')),
    resolve('inlineInherit', wrapped('rgb(0, 128, 0)', 'inherit')),
    resolve('inlineCurrentcolor', wrapped('rgb(0, 128, 0)', 'currentcolor')),
    resolve('inlineOverParent', wrapped('rgb(0, 128, 0)', 'rgb(1, 2, 3)')),
    resolve('sheet', canvas => {
      const style = document.createElement('style');
      style.textContent = 'canvas.x { color: rgb(1, 2, 3) }';
      document.head.appendChild(style);
      canvas.className = 'x';
      body.appendChild(canvas);
      return () => { canvas.remove(); style.remove(); };
    }),
    resolve('bodyColor', canvas => {
      body.style.color = 'rgb(9, 9, 9)';
      body.appendChild(canvas);
      return () => { canvas.remove(); body.removeAttribute('style'); };
    }),
    resolve('inlineTranslucent', inline('color: rgba(0, 128, 0, 0.5)')),
    resolve('inlineHsl', inline('color: hsl(240, 100%, 50%)')),
    resolve('inlineNamed', inline('color: CornflowerBlue')),
    resolve('inlineHidden', inline('color: red; display: none')),
    resolve('inlineImportant', inline('color: lime !important')),
  ];
  {
    // A context taken while detached resolves against the canvas once it is connected.
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    canvas.style.color = 'red';
    body.appendChild(canvas);
    ctx.fillStyle = 'currentcolor';
    out.contextBeforeConnect = ctx.fillStyle;
    canvas.remove();
    ctx.fillStyle = 'currentcolor';
    out.afterDisconnect = ctx.fillStyle;
  }
  {
    const canvas = document.createElement('canvas');
    canvas.style.color = 'red';
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = 'currentcolor';
    ctx.strokeStyle = 'currentcolor';
    ctx.shadowColor = 'currentcolor';
    out.detached = props.map(p => ctx[p]);
    ctx.fillRect(0, 0, 1, 1);
    out.detachedPaint = paint(ctx, 0);
  }
  {
    const canvas = document.createElement('canvas');
    body.appendChild(canvas);
    const ctx = canvas.getContext('2d');
    const log = [];
    for (const [i, s] of ['hsl(0,100%,50%)', 'notacolor', 'rgb(0 0 255)', '#00ff00', 'CornflowerBlue'].entries()) {
      ctx.fillStyle = s;
      ctx.fillRect(i, 0, 1, 1);
      log.push([s, paint(ctx, i)]);
    }
    out.paints = log;
    canvas.remove();
  }
  return out;
})()

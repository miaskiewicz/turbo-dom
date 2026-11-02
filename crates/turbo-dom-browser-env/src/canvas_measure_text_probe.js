(() => {
  // CanvasRenderingContext2D.measureText (turbo-dom#237): every TextMetrics field per canvas
  // `font` (family list, size, weight, style) with the repo's fixture fonts (Liberation
  // Serif, Inter, Fira Code; loaded through @font-face in Chrome, registered by the host
  // offline), then the textAlign x textBaseline offsets for one run.
  const KEYS = ['width', 'actualBoundingBoxLeft', 'actualBoundingBoxRight', 'actualBoundingBoxAscent',
    'actualBoundingBoxDescent', 'fontBoundingBoxAscent', 'fontBoundingBoxDescent', 'emHeightAscent',
    'emHeightDescent', 'hangingBaseline', 'alphabeticBaseline', 'ideographicBaseline'];
  const ctx = document.createElement('canvas').getContext('2d');
  const read = (text) => { const m = ctx.measureText(text); return KEYS.map((key) => m[key]); };
  const FONTS = ['16px Inter', 'bold 16px Inter', 'italic 13px "Liberation Serif"',
    'italic bold 21.5px "Liberation Serif"', '10px "Liberation Serif"', '20px "Fira Code"',
    '700 12px "Fira Code", serif', '15px Nope, Inter', 'oblique 300 18pt Inter'];
  const TEXTS = ['', ' ', 'Typography', 'AVATAR Wave', 'Hello, World!', 'gjpqy', 'To fi ffi',
    'éäo', '  two  spaces '];
  const out = { keys: KEYS, metrics: [], aligned: [] };
  for (const font of FONTS) {
    ctx.font = font;
    for (const text of TEXTS) out.metrics.push([font, text, read(text)]);
  }
  ctx.font = 'italic 16px Inter';
  for (const align of ['start', 'end', 'left', 'right', 'center'])
    for (const baseline of ['alphabetic', 'top', 'hanging', 'middle', 'ideographic', 'bottom']) {
      ctx.textAlign = align;
      ctx.textBaseline = baseline;
      out.aligned.push([align, baseline, read('Wave gj')]);
    }
  return out;
})()

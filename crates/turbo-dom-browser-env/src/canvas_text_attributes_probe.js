(() => {
  // CanvasRenderingContext2D text attributes (turbo-dom#237): letterSpacing / wordSpacing
  // (CSS lengths) and the fontKerning / fontStretch / textRendering enums, what each
  // assignment reads back, and their effect on measureText with the fixture fonts.
  const fresh = () => document.createElement('canvas').getContext('2d');
  const ATTRS = ['letterSpacing', 'wordSpacing', 'fontKerning', 'fontStretch', 'textRendering'];
  const out = {};
  const initial = fresh();
  out.initial = ATTRS.map((a) => initial[a]);
  out.inContext = ATTRS.map((a) => a in initial);
  const LENGTHS = ['2px', '2PX', ' 3px ', '1.5px', '-1px', '0', '0px', '1em', '0.5rem', '10%', '2', 'px',
    'calc(1px + 2px)', '1pt', '1in', '+4px', '.5px', '1e1px', 'inherit', 'normal', '', null, 4, 'abc', '2px 3px', '1ex', '1ch', '1vw'];
  out.lengths = {};
  for (const attr of ['letterSpacing', 'wordSpacing'])
    out.lengths[attr] = LENGTHS.map((v) => { const c = fresh(); c[attr] = '7px'; c[attr] = v; return c[attr]; });
  const ENUMS = {
    fontKerning: ['auto', 'normal', 'none', 'NONE', 'bogus', ''],
    fontStretch: ['ultra-condensed', 'extra-condensed', 'condensed', 'semi-condensed', 'normal', 'semi-expanded',
      'expanded', 'extra-expanded', 'ultra-expanded', 'CONDENSED', '50%', 'bogus'],
    textRendering: ['auto', 'optimizeSpeed', 'optimizeLegibility', 'geometricPrecision', 'optimizespeed', 'bogus'],
  };
  out.enums = {};
  for (const [attr, values] of Object.entries(ENUMS))
    out.enums[attr] = values.map((v) => { const c = fresh(); c[attr] = v; return c[attr]; });
  // save/restore covers the new state.
  const s = fresh();
  s.letterSpacing = '3px'; s.fontKerning = 'none'; s.save();
  s.letterSpacing = '1px'; s.fontKerning = 'normal'; s.restore();
  out.restored = [s.letterSpacing, s.fontKerning];
  // Widths (and the ink box) under each setting.
  const TEXTS = ['AVATAR Wave', 'To fi ffi', 'a b  c', ''];
  const SETTINGS = [
    {}, { letterSpacing: '2px' }, { letterSpacing: '-1.5px' }, { letterSpacing: '0.25em' },
    { wordSpacing: '5px' }, { wordSpacing: '-2px' }, { wordSpacing: '1em' }, { letterSpacing: '1px', wordSpacing: '3px' },
    { fontKerning: 'none' }, { fontKerning: 'normal' }, { textRendering: 'optimizeSpeed' },
    { textRendering: 'optimizeLegibility' }, { textRendering: 'geometricPrecision' }, { fontStretch: 'condensed' },
    { font: '20px "Liberation Serif"', letterSpacing: '1em' },
  ];
  out.widths = [];
  for (const setting of SETTINGS)
    for (const font of ['16px Inter', '16px "Liberation Serif"']) {
      const c = fresh();
      c.font = font;
      Object.assign(c, setting);
      for (const text of TEXTS) {
        const m = c.measureText(text);
        out.widths.push([font, JSON.stringify(setting), text,
          [m.width, m.actualBoundingBoxLeft, m.actualBoundingBoxRight]]);
      }
    }
  // An em letterSpacing resolves against the font at assignment.
  const e = fresh();
  e.font = '10px Inter'; e.letterSpacing = '1em'; e.font = '30px Inter';
  out.emAtAssignment = [e.letterSpacing, e.measureText('ab').width];
  return out;
})()

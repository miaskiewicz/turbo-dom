(() => {
  // Canvas letterSpacing / wordSpacing length units (turbo-dom#237): which units an
  // assignment accepts, and how many px one unit adds per character in measureText for a
  // canvas in the document and a detached one. The root is `20px Inter` (root-relative
  // units), the canvas font `10px Inter` (font-relative units), the viewport 800x600.
  const UNITS = ['px', 'em', 'rem', 'ex', 'rex', 'ch', 'rch', 'ic', 'ric', 'cap', 'rcap', 'lh', 'rlh',
    'vw', 'vh', 'vi', 'vb', 'vmin', 'vmax', 'svw', 'svh', 'svi', 'svb', 'svmin', 'svmax', 'lvw', 'lvh',
    'lvi', 'lvb', 'lvmin', 'lvmax', 'dvw', 'dvh', 'dvi', 'dvb', 'dvmin', 'dvmax', 'cqw', 'cqh', 'cqi',
    'cqb', 'cqmin', 'cqmax', 'cm', 'mm', 'q', 'in', 'pt', 'pc', 'x', 'dppx', 'fr', 'deg', 's', 'vx', 'Q', 'REM', 'Cqw'];
  document.documentElement.style.font = '20px Inter';
  const make = (attached) => {
    const canvas = document.createElement('canvas');
    if (attached) document.body.append(canvas);
    const ctx = canvas.getContext('2d');
    ctx.font = '10px Inter';
    return ctx;
  };
  const out = { accepted: {}, perUnit: {} };
  for (const unit of UNITS) {
    const ctx = make(false);
    ctx.letterSpacing = '2' + unit;
    ctx.wordSpacing = '3' + unit;
    out.accepted[unit] = [ctx.letterSpacing, ctx.wordSpacing];
  }
  for (const attached of [true, false]) {
    const rows = {};
    for (const unit of UNITS.slice(0, 49)) {
      const ctx = make(attached);
      const base = ctx.measureText('ab').width;
      ctx.letterSpacing = '1' + unit;
      rows[unit] = (ctx.measureText('ab').width - base) / 2;
    }
    out.perUnit[attached ? 'attached' : 'detached'] = rows;
  }
  document.documentElement.style.font = '';
  return out;
})()

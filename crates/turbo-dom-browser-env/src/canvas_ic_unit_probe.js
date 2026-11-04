(() => {
  // The CSS `ic` unit with a face that has U+6C34 (turbo-dom#237): `Ic Fixture` maps it to a
  // 0.9em advance. Per unit, the px one unit of letterSpacing adds per character, and the
  // width of the glyph itself.
  const ctx = document.createElement('canvas').getContext('2d');
  const out = {};
  for (const size of [10, 16, 21.5]) {
    ctx.font = `${size}px "Ic Fixture"`;
    ctx.letterSpacing = '0px';
    const base = ctx.measureText('ab').width;
    const row = { water: ctx.measureText('水').width };
    for (const unit of ['ic', 'ric', 'em']) {
      ctx.letterSpacing = '1' + unit;
      row[unit] = (ctx.measureText('ab').width - base) / 2;
    }
    out[size] = row;
  }
  return out;
})()

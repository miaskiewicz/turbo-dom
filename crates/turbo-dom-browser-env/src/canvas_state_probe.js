(() => {
  // CanvasRenderingContext2D save() / restore() (turbo-dom#110): the drawing-state stack.
  // Every state attribute and the line dash read back at each level of a nested
  // save/restore; restore() with an empty stack is a no-op; restoring brings back a
  // gradient by identity; and the colour the next fillRect paints is the restored one
  // (`paint` is the canvas pixel in Chrome; offline it is the colour the op log recorded for
  // that fillRect, which the test maps to a pixel).
  const props = [
    'fillStyle', 'strokeStyle', 'shadowColor', 'globalAlpha', 'globalCompositeOperation', 'lineWidth',
    'lineCap', 'lineJoin', 'miterLimit', 'lineDashOffset', 'shadowBlur', 'shadowOffsetX', 'shadowOffsetY',
    'font', 'textAlign', 'textBaseline', 'direction', 'imageSmoothingEnabled', 'imageSmoothingQuality',
  ];
  const fresh = () => document.createElement('canvas').getContext('2d');
  const paint = (ctx, x) => ctx._ops ? ctx._ops[ctx._ops.length - 1][2] : Array.from(ctx.getImageData(x, 0, 1, 1).data);
  const read = ctx => [props.map(p => ctx[p]), ctx.getLineDash()];
  const first = {
    fillStyle: '#ff0000', strokeStyle: '#00ff00', shadowColor: 'rgba(0, 0, 255, 0.5)', globalAlpha: 0.5,
    globalCompositeOperation: 'multiply', lineWidth: 3, lineCap: 'round', lineJoin: 'bevel', miterLimit: 4,
    lineDashOffset: 2, shadowBlur: 5, shadowOffsetX: 6, shadowOffsetY: 7, font: '20px serif',
    textAlign: 'center', textBaseline: 'top', direction: 'rtl', imageSmoothingEnabled: false,
    imageSmoothingQuality: 'high',
  };
  const second = {
    fillStyle: '#0000ff', strokeStyle: '#ffff00', shadowColor: '#ff00ff', globalAlpha: 0.25,
    globalCompositeOperation: 'xor', lineWidth: 8, lineCap: 'square', lineJoin: 'round', miterLimit: 2,
    lineDashOffset: 9, shadowBlur: 1, shadowOffsetX: -2, shadowOffsetY: -3, font: 'bold 12px monospace',
    textAlign: 'right', textBaseline: 'middle', direction: 'ltr', imageSmoothingEnabled: true,
    imageSmoothingQuality: 'medium',
  };
  const assign = (ctx, values) => { for (const p of props) ctx[p] = values[p]; };
  const out = {};
  {
    const ctx = fresh();
    const log = [read(ctx)];
    ctx.save();
    assign(ctx, first);
    ctx.setLineDash([1, 2]);
    log.push(read(ctx));
    ctx.save();
    assign(ctx, second);
    ctx.setLineDash([3, 4, 5]);
    log.push(read(ctx));
    ctx.restore();
    log.push(read(ctx));
    ctx.restore();
    log.push(read(ctx));
    // The stack is empty again: nothing changes.
    assign(ctx, first);
    ctx.restore();
    log.push(read(ctx));
    out.nested = log;
  }
  {
    // restore() on a fresh context keeps what was assigned.
    const ctx = fresh();
    ctx.fillStyle = 'red';
    ctx.lineWidth = 4;
    ctx.restore();
    ctx.restore();
    out.emptyRestore = [ctx.fillStyle, ctx.lineWidth];
  }
  {
    // A state saved twice in a row restores twice.
    const ctx = fresh();
    ctx.fillStyle = '#111111';
    ctx.save();
    ctx.save();
    ctx.fillStyle = '#222222';
    const log = [ctx.fillStyle];
    ctx.restore();
    log.push(ctx.fillStyle);
    ctx.fillStyle = '#333333';
    ctx.restore();
    log.push(ctx.fillStyle);
    ctx.restore();
    log.push(ctx.fillStyle);
    out.doubleSave = log;
  }
  {
    const ctx = fresh(), gradient = ctx.createLinearGradient(0, 0, 1, 1);
    ctx.fillStyle = gradient;
    ctx.save();
    ctx.fillStyle = 'red';
    ctx.restore();
    out.gradient = ctx.fillStyle === gradient;
  }
  {
    const canvas = document.createElement('canvas');
    document.body.appendChild(canvas);
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#00ff00';
    ctx.save();
    ctx.fillStyle = '#ff0000';
    ctx.fillRect(0, 0, 1, 1);
    const log = [paint(ctx, 0)];
    ctx.restore();
    ctx.fillRect(1, 0, 1, 1);
    log.push(paint(ctx, 1));
    out.paints = log;
    canvas.remove();
  }
  return out;
})()

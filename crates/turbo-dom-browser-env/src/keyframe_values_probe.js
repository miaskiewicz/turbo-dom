(async () => {
  // getKeyframes() member values (miaskiewicz/turbo-dom#155): numbers keep six significant
  // digits in printf %g form (exponent below 1e-4 and from 1e6), a value with var() / env()
  // keeps its specified text (trimmed, without a trailing comment or !important), and a bare
  // zero serializes per the property's grammar (0px for lengths, 0 for numbers).
  const target = document.body.appendChild(document.createElement('div'));
  const value = (property, text) => {
    try {
      const animation = target.animate([{ [property]: text }], 1000);
      const frame = animation.effect.getKeyframes()[0];
      animation.cancel();
      return property in frame ? frame[property] : null;
    } catch (error) { return error.name; }
  };
  const numbers = ['1e-7', '1.5e-7', '0.000001', '0.0000012345678', '0.00001', '0.0001', '1234567', '123456.7', '999999.5', '1e21', '0.30000000000000004', '-0', '-0.0000000000001', '1e-45', '3.4e38', '12345.678', '0.1234567', '1E3', '+.5'];
  const out = {};
  out.opacity = numbers.map(number => value('opacity', number));
  out.translate = numbers.map(number => value('translate', number + 'px'));
  out.lineHeight = numbers.map(number => value('lineHeight', number));
  out.units = ['1e-7%', '12.3456789%', '1E3PX', '1.50PX', '+.5Em', '-0.0px'].map(text => value('width', text));
  out.functions = ['translateX(1E-7PX)', 'ROTATE(1E-10DEG)'].map(text => value('transform', text));
  out.vars = [' var(--w,  (1PX) ) ', 'var(--w, (1PX)) /*c*/', 'calc(var(--a) + 1PX)', 'VAR(--x)', 'var(--w,\n1PX)', '1PX var(--x)', 'var(--w,/*c*/1PX)', '/*a*/var(--w)', 'calc(1PX /*c*/ + var(--a))', 'var(--a)  ,  1PX', '\tvar(--a)\n', 'var(--a) !important', 'env(SAFE-area-inset-top)', 'var(--a,)', 'var(--a', 'var(--a))', '1E3PX var(--x)', '"str" var(--x)', "var(--a, 'q\\'')"].map(text => value('width', text));
  out.margin = ['var(--a)  1PX', 'var(--a) /**/'].map(text => value('margin', text));
  out.custom = [' x  /*c*/ ', ' VAR(--Y) '].map(text => value('--c', text));
  out.zeros = ['top', 'right', 'bottom', 'left', 'opacity', 'columnWidth', 'outlineOffset', 'columnGap', 'columnRuleWidth', 'outlineWidth', 'borderTopWidth', 'borderSpacing', 'perspective', 'tabSize', 'wordSpacing', 'scrollMarginTop', 'scrollPaddingTop', 'strokeWidth', 'x', 'r', 'fontSize', 'textIndent', 'verticalAlign', 'shapeMargin', 'offsetDistance', 'maxWidth', 'minHeight', 'backgroundPositionX', 'textUnderlineOffset', 'textDecorationThickness', 'outline', 'borderWidth', 'inset', 'margin', 'gap', 'flex', 'flexBasis', 'lineHeight'].map(property => [property, value(property, '0')]);
  out.lengthsOnly = [['columnWidth', 'auto'], ['columnWidth', 'red'], ['columnWidth', '-1px'], ['columnWidth', '2EM'], ['outlineOffset', '-1PX'], ['outlineOffset', 'auto'], ['outlineOffset', '1']].map(([property, text]) => [property, text, value(property, text)]);
  target.remove();
  return out;
})()

(() => {
  const properties = ['display', 'fontFamily', 'fontSize', 'fontStyle', 'fontWeight', 'fontStretch', 'fontVariantCaps', 'lineHeight', 'marginTop', 'marginRight', 'marginBottom', 'marginLeft', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft', 'borderTopWidth', 'borderRightWidth', 'borderBottomWidth', 'borderLeftWidth', 'verticalAlign'];
  const host = document.createElement('div');
  document.body.append(host);
  const style = document.createElement('style');
  document.head.append(style);
  const rows = [];
  for (const [name, parentStyle, authorStyle, inlineStyle] of [
    ['default', '', '', ''],
    ['parent-font', 'font:italic 700 26px/2 monospace', '', ''],
    ['universal-font', 'font:italic 700 26px/2 monospace', '*{font:italic 700 19px/1.5 monospace;padding:3px 7px;border-width:4px;margin:5px}', ''],
    ['inline-font', 'font:italic 700 26px/2 monospace', '', 'font:italic 700 17px/1.25 serif'],
    ['font-inherit', 'font:italic 700 26px/2 monospace', '', 'font:inherit'],
    ['longhand-inherit', 'font:italic 700 26px/2 monospace', '', 'font-family:inherit;font-size:inherit;font-style:inherit;font-weight:inherit;line-height:inherit'],
    ['font-initial', 'font:italic 700 26px/2 monospace', '', 'font:initial'],
    ['font-revert', 'font:italic 700 26px/2 monospace', '', 'font:revert'],
  ]) {
    host.style.cssText = parentStyle;
    style.textContent = authorStyle;
    const button = document.createElement('button');
    button.textContent = 'Reject';
    button.style.cssText = inlineStyle;
    host.append(button);
    const computed = getComputedStyle(button);
    rows.push({name, values:Object.fromEntries(properties.map(property => [property, computed[property]]))});
    button.remove();
  }
  style.remove();
  host.remove();
  return {rows};
})()

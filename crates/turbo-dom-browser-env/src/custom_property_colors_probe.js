(() => {
  // turbo-dom#285: custom properties keep their authored text; only real colour
  // properties canonicalize hex colours.
  const out = {};
  const el = document.body.appendChild(document.createElement('div'));
  for (const name of ['--theme-color', '--color', '--bg-color', '--x', 'color', 'background-color']) {
    el.removeAttribute('style');
    el.style.setProperty(name, '#ff0000');
    out[name] = { value: el.style.getPropertyValue(name), cssText: el.style.cssText };
  }
  el.removeAttribute('style');
  el.style.setProperty('--short-color', '#AbC');
  el.style.setProperty('--spaced-color', '  #00ff00  ');
  el.style.setProperty('--named-color', 'red');
  out.mixed = { short: el.style.getPropertyValue('--short-color'), spaced: el.style.getPropertyValue('--spaced-color'), named: el.style.getPropertyValue('--named-color'), cssText: el.style.cssText };
  el.setAttribute('style', '--attr-color: #123456; color: #123456');
  out.attribute = { custom: el.style.getPropertyValue('--attr-color'), color: el.style.getPropertyValue('color'), cssText: el.style.cssText };
  el.style.cssText = '--text-color:#0000ff;border-color:#0000ff';
  out.cssText = { custom: el.style.getPropertyValue('--text-color'), border: el.style.getPropertyValue('border-color'), cssText: el.style.cssText };
  el.remove();
  return out;
})()

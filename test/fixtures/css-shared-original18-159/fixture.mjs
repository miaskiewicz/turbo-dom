export function readCase(window, document, scenario) {
  const target=document.getElementById('target');
  if(scenario.action==='focus') target.focus();
  const style=window.getComputedStyle(target,scenario.pseudo||null);
  return {id:scenario.id,values:Object.fromEntries(scenario.properties.map(property=>[property,style.getPropertyValue(property)]))};
}
export function caseHTML(scenario) {
  const tag=scenario.tag||'div';
  return `<!doctype html><html><head><style>${scenario.css}</style></head><body>${scenario.wrapper||''}<${tag} id=target style="${scenario.inline||''}"></${tag}>${scenario.wrapper?'</div>':''}</body></html>`;
}

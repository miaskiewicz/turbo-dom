export function caseHTML(scenario) {
  return '<!doctype html><html><head></head><body><div class="x" style="'+scenario.inline.replaceAll('&','&amp;').replaceAll('"','&quot;')+'">text</div></body></html>';
}
const value=v=>v===undefined?{type:'undefined'}:{type:typeof v,value:v};
export function readCase(window,document,scenario) {
  const el=document.querySelector('.x'),computed=window.getComputedStyle(el),authored=el.style;
  return {id:scenario.id,cssText:authored.cssText,values:scenario.properties.map(([name,camel])=>({name,camel,
    computed:{getPropertyValue:value(computed.getPropertyValue(name)),camel:value(computed[camel])},
    authored:{getPropertyValue:value(authored.getPropertyValue(name)),camel:value(authored[camel])},
  }))};
}

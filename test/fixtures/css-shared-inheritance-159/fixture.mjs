export function caseHTML(scenario){
 const parent=scenario.family==='percentage'?'width:50%;font-size:10px':'width:20em;font-size:10px';
 return '<!doctype html><html><head><style>body{margin:0}#outer{width:800px}#child{font-size:20px;width:inherit}</style></head><body><div id="outer"><div id="parent" class="'+(scenario.target==='parent'?'x':'')+'" style="'+parent+'"><div id="child" class="'+(scenario.target==='child'?'x':'')+'"></div></div></div></body></html>';
}
const value=v=>v===undefined?{type:'undefined'}:{type:typeof v,value:v};
export function readCase(window,document,scenario) {
  const el=document.querySelector('.x'),computed=window.getComputedStyle(el),authored=el.style;
  return {id:scenario.id,cssText:authored.cssText,values:scenario.properties.map(([name,camel])=>({name,camel,
    computed:{getPropertyValue:value(computed.getPropertyValue(name)),camel:value(computed[camel])},
    authored:{getPropertyValue:value(authored.getPropertyValue(name)),camel:value(authored[camel])},
  }))};
}

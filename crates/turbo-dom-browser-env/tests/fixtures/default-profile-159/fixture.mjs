export function html(scenario){return '<!doctype html><html><head>'+(scenario.css?'<style>'+scenario.css+'</style>':'')+'</head><body><div class=x style="'+(scenario.style||'')+'"></div></body></html>';}
export function viewportDescriptors(window){
 const result={};for(const name of ['innerWidth','innerHeight']){const descriptor=Object.getOwnPropertyDescriptor(window,name);result[name]=!descriptor?{kind:'absent'}:'value'in descriptor?{kind:'data',value:descriptor.value}:{kind:'accessor',get:typeof descriptor.get,set:typeof descriptor.set};}return result;
}
export function values(window,document,scenario){const target=document.querySelector('.x'),style=window.getComputedStyle(target);return{values:Object.fromEntries(scenario.properties.map(name=>[name,style.getPropertyValue(name)])),authored:target.style.cssText};}

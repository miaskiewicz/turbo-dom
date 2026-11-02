// Stack-edge probe: every op runs at each depth while a recursion unwinds, so some call
// enters a native with almost no stack left. Recorded in headless Chrome 156 over localhost.
(function(){
const d=document, host=d.body;
const ops={
  createElement:()=>d.createElement('div'),
  appendChild:()=>host.appendChild(d.createElement('span')).remove(),
  innerHTML:()=>{const e=d.createElement('div');e.innerHTML='<b>x</b><i style="color:red">y</i>';return e.firstChild;},
  querySelector:()=>d.querySelector('div > p, #a'),
  setStyle:()=>{const e=d.createElement('div');e.setAttribute('style','color:red');return e.style.color;},
  attachShadow:()=>d.createElement('div').attachShadow({mode:'open'}),
  textContent:()=>{const e=d.createElement('p');e.textContent='a';return e.textContent;},
  range:()=>{const r=d.createRange();r.selectNodeContents(host);return r.createContextualFragment('<a>q</a>');},
  cloneNode:()=>host.cloneNode(true),
  childNodes:()=>host.childNodes[0],
  dialog:()=>{const e=d.createElement('dialog');host.appendChild(e);e.showModal();e.close();e.remove();},
  popover:()=>{const e=d.createElement('div');e.popover='auto';host.appendChild(e);e.showPopover();e.hidePopover();e.remove();},
  event:()=>{const e=d.createElement('div');e.addEventListener('x',()=>{});e.dispatchEvent(new Event('x'));},
  mutation:()=>{const m=new MutationObserver(()=>{});m.observe(host,{childList:true});host.append('t');m.takeRecords();m.disconnect();},
  custom:()=>{try{customElements.define('x-'+(Math.random()*1e9|0),class extends HTMLElement{});}catch(e){if(e.name!=='RangeError')throw e;}},
  getComputedStyle:()=>getComputedStyle(host).color,
  selection:()=>{const s=getSelection();s.selectAllChildren(host);return s.toString();},
  outerHTML:()=>host.outerHTML,
  template:()=>{const t=d.createElement('template');t.innerHTML='<b>1</b>';return t.content.cloneNode(true);},
  slot:()=>{const h=d.createElement('div');const r=h.attachShadow({mode:'open',slotAssignment:'manual'});const s=d.createElement('slot');r.append(s);const c=d.createElement('i');h.append(c);s.assign(c);return s.assignedNodes();},
  xpath:()=>d.evaluate('//p',d,null,7,null).snapshotLength,
  sanitizer:()=>{const e=d.createElement('div');e.setHTML('<b onclick=x>1</b>');return e.innerHTML;},
  onclick:()=>{const e=d.createElement('div');e.setAttribute('onclick','1');return typeof e.onclick;},
  domain:()=>d.domain,
  href:()=>{const a=d.createElement('a');a.href='http://x.test/p?q#h';return a.host+a.pathname;},
  webkitMatches:()=>host.webkitMatchesSelector('body'),
  pi:()=>d.createProcessingInstruction('x','y').data,
  animate:()=>d.createElement('div').animate([{opacity:0},{opacity:1}],10),
  script:()=>{const s=d.createElement('script');s.async=false;return s.async;},
};
const out={};
for (const name in ops){
  const op=ops[name]; const names=new Set();
  // Unwinding runs op only in the deepest 200 frames (max is this descent's own depth),
  // each with a different, small stack margin.
  let depth=0,max=0;
  const f=armed=>{depth++;if(depth>max)max=depth;try{f(armed);}catch(e){}
    if(armed&&depth>max-200){try{op();}catch(e){names.add(e&&e.name);}}depth--;};
  f(true);
  out[name]=[...names].sort().join(',');
}
return JSON.stringify(out);
})()

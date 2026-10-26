(async()=>{
 const id=document.documentElement.dataset.case, violations=[], trace=[];
 document.addEventListener('securitypolicyviolation',e=>violations.push({effectiveDirective:e.effectiveDirective,blockedURI:e.blockedURI,disposition:e.disposition,sample:e.sample}));
 if(id==='sink-default'||id==='sink-default-null')trustedTypes.createPolicy('default',Object.fromEntries(['createHTML','createScript','createScriptURL'].map(name=>[name,(value,...args)=>{trace.push([name,value,args]);return id==='sink-default-null'?null:value}])));
 const error=f=>{try{const value=f();return{ok:true,value:value===undefined?null:value}}catch(e){return{error:e.name,message:e.message}}};
 const div=document.createElement('div'), script=document.createElement('script'), iframe=document.createElement('iframe'), embed=document.createElement('embed'), object=document.createElement('object');
 const host=document.createElement('div');host.append(div);
 const rows=[
  ['innerHTML',()=>{div.innerHTML='<b>X</b>';return div.innerHTML}],
  ['outerHTML',()=>{div.outerHTML='<b>Y</b>';return host.innerHTML}],
  ['insertAdjacentHTML',()=>{host.insertAdjacentHTML('beforeend','<i>Z</i>');return host.innerHTML}],
  ['script.text',()=>{script.text='void 0';return script.text}],
  ['script.textContent',()=>{script.textContent='void 1';return script.textContent}],
  ['script.innerText',()=>{script.innerText='void 2';return script.innerText}],
  ['script.src',()=>{script.src='/script.js';return script.getAttribute('src')}],
  ['setAttribute.src',()=>{script.setAttribute('src','/attr.js');return script.getAttribute('src')}],
  ['setAttribute.onclick',()=>{div.setAttribute('onclick','void 3');return div.getAttribute('onclick')}],
  ['iframe.srcdoc',()=>{iframe.srcdoc='<p>frame</p>';return iframe.srcdoc}],
  ['embed.src',()=>{embed.src='/embed';return embed.getAttribute('src')}],
  ['object.data',()=>{object.data='/object';return object.getAttribute('data')}],
  ['DOMParser',()=>new DOMParser().parseFromString('<b>parse</b>','text/html').body.innerHTML],
  ['Range',()=>{const r=document.createRange();r.selectNodeContents(host);return r.createContextualFragment('<b>range</b>').textContent}],
 ].map(([name,f])=>[name,error(f)]);
 const p=trustedTypes.createPolicy('alpha',{createHTML:s=>s,createScript:s=>s,createScriptURL:s=>s}),typed=p.createHTML('<em>typed</em>');Object.defineProperty(typed,'toString',{value(){throw Error('authored poison')}});
 rows.push(['typed-innerHTML',error(()=>{host.innerHTML=typed;return host.innerHTML})]);
 await new Promise(resolve=>setTimeout(resolve,50));return{rows,trace,violations};
})()

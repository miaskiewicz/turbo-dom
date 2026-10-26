(()=>{
  const rows=[],error=fn=>{try{fn();return 'accepted'}catch(e){return e.name}};
  for(const operation of ['append','prepend','replaceChildren']){
    const t=document.createElement('template');t.innerHTML='<b>x</b>';const f=t.content,own=f.firstChild;
    rows.push({operation,self:error(()=>f[operation](f)),count:f.childNodes.length,first:f.firstChild===own});
  }
  const t=document.createElement('template');t.innerHTML='<button onclick="return 7">seven</button><template><i>nested</i></template>';document.body.append(t);
  let outer=[],inner=[],nested=[];const a=new MutationObserver(v=>outer.push(...v)),b=new MutationObserver(v=>inner.push(...v)),c=new MutationObserver(v=>nested.push(...v));
  a.observe(t,{subtree:true,childList:true,attributes:true,characterData:true});b.observe(t.content,{subtree:true,childList:true,attributes:true,characterData:true});c.observe(t.content.lastChild.content,{subtree:true,childList:true});
  const old=t.content.firstChild;t.innerHTML='<p>replacement</p>';
  const observer={outer:a.takeRecords().length,inner:b.takeRecords().map(r=>[r.type,r.target===t.content,r.addedNodes.length,r.removedNodes.length]),oldOwner:old.ownerDocument===t.content.ownerDocument};
  const fresh=document.createElement('template');fresh.innerHTML='<button onclick="return 9">nine</button>';const button=fresh.content.firstChild;document.body.append(button);
  const unreadHandler={value:button.onclick.call(button,new Event('click')),native:String(button.onclick)};
  const parsed=document.createElement('div');parsed.innerHTML='<template><template><i id="deep">payload</i></template></template>';const parsedT=parsed.firstChild;
  const parse={ordinary:parsedT.childNodes.length,nested:parsedT.content.firstChild.childNodes.length,text:parsedT.content.firstChild.content.firstChild.textContent,bodyQuery:parsed.querySelector('#deep')===null,markup:parsed.innerHTML};
  const scriptRuns=[];globalThis.__templateScriptRuns=scriptRuns;
  const scripted=document.createElement('template');scripted.innerHTML='<script>__templateScriptRuns.push("inner")</script>';document.body.append(scripted.content);
  const cloned=document.createElement('template');cloned.innerHTML='<script>__templateScriptRuns.push("clone")</script>';document.body.append(cloned.content.cloneNode(true));
  const contextual=document.createRange().createContextualFragment('<template><script>__templateScriptRuns.push("contextual")</script></template>');document.body.append(contextual.firstChild.content);
  const parser=new DOMParser().parseFromString('<template><script>__templateScriptRuns.push("parser")</script></template>','text/html');document.body.append(parser.querySelector('template').content);
  t.remove();button.remove();return {rows,observer,unreadHandler,parse,scriptRuns};
})()

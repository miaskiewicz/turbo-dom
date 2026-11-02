(() => {
// miaskiewicz/turbo-dom#172: the page replaces every configurable member of the builtins the
// engine could use with throwing functions, calls only originals it captured first, then
// restores them. Expected: Chrome 156.0.8078.47 (headless, localhost) running this file
// (with and without __withStyle).

const A=Reflect.apply,gopd=Object.getOwnPropertyDescriptor,def=Object.defineProperty,own=Reflect.ownKeys,Err=Error,Ev=Event,str=JSON.stringify;
const fn=(o,k)=>gopd(o,k).value,getter=(o,k)=>gopd(o,k).get,setter=(o,k)=>gopd(o,k).set;
const create=fn(Document.prototype,'createElement'),createText=fn(Document.prototype,'createTextNode'),append=fn(Node.prototype,'appendChild'),
  insertBefore=fn(Node.prototype,'insertBefore'),remove=fn(Element.prototype,'remove'),listen=fn(EventTarget.prototype,'addEventListener'),
  dispatch=fn(EventTarget.prototype,'dispatchEvent'),parent=getter(Node.prototype,'parentNode'),bodyOf=getter(Document.prototype,'body'),
  setText=setter(Node.prototype,'textContent'),next=getter(Element.prototype,'nextElementSibling'),
  onclick=setter(HTMLElement.prototype,'onclick'),sheet=getter(HTMLStyleElement.prototype,'sheet'),
  rules=getter(CSSStyleSheet.prototype,'cssRules'),len=getter(CSSRuleList.prototype,'length'),
  qs=fn(Document.prototype,'querySelector'),byTag=fn(Document.prototype,'getElementsByTagName'),hlen=getter(HTMLCollection.prototype,'length');
const out={__proto__:null};const saved=[];let count=0;
const thrower=(name)=>function(){throw new Err('patched '+name)};
const targets=[[Array.prototype,'Array.prototype'],[Array,'Array'],[Map.prototype,'Map.prototype'],[Set.prototype,'Set.prototype'],
  [WeakMap.prototype,'WeakMap.prototype'],[WeakSet.prototype,'WeakSet.prototype'],[Object,'Object'],[Reflect,'Reflect'],
  [String.prototype,'String.prototype'],[Function.prototype,'Function.prototype'],[Promise,'Promise'],[Promise.prototype,'Promise.prototype'],
  [JSON,'JSON'],[Symbol,'Symbol'],[Number,'Number'],[Math,'Math'],[RegExp.prototype,'RegExp.prototype'],
  [EventTarget.prototype,'EventTarget.prototype'],[Node.prototype,'Node.prototype'],[Element.prototype,'Element.prototype'],
  [Document.prototype,'Document.prototype'],[HTMLElement.prototype,'HTMLElement.prototype']];
const skip={__proto__:null,constructor:1,prototype:1,length:1,name:1};
const keep=(o,k)=>{saved[saved.length]=[o,k,gopd(o,k)];count++};
for(let t=0;t<targets.length;t++){const o=targets[t][0],label=targets[t][1],ks=own(o);
  for(let i=0;i<ks.length;i++){const k=ks[i];if(typeof k==='string'&&skip[k])continue;if(k===Symbol.hasInstance)continue;
    const d=gopd(o,k);if(!d.configurable)continue;const n=label+'.'+(typeof k==='symbol'?'@@':'')+String(k);
    if(typeof d.value==='function'){keep(o,k);def(o,k,{value:thrower(n),writable:true,configurable:true,enumerable:d.enumerable})}
    else if(d.get||d.set){keep(o,k);def(o,k,{get:thrower('get '+n),set:thrower('set '+n),configurable:true,enumerable:d.enumerable})}}}
for(let t=0;t<2;t++)for(let i=0;i<4;i++){const o=t?Object.prototype:Array.prototype,k=''+i;keep(o,k);def(o,k,{get:thrower('get index'),set:thrower('set index'),configurable:true})}
const attempt=(name,f)=>{try{out[name]=f()}catch(e){out[name]='threw '+(e&&e.message)}};
const body=A(bodyOf,document,[]);
attempt('div',()=>{const d=A(create,document,['div']);A(append,body,[d]);return A(parent,d,[])===body});
attempt('text',()=>{const s=A(create,document,['span']),t=A(createText,document,['hi']);A(append,s,[t]);A(append,body,[s]);return A(parent,t,[])===s});
attempt('script',()=>{const s=A(create,document,['script']);A(setText,s,['1']);A(append,body,[s]);return A(parent,s,[])===body});
attempt('iframe',()=>{const f=A(create,document,['iframe']);A(append,body,[f]);return A(parent,f,[])===body});
attempt('insert',()=>{const p=A(create,document,['p']);A(insertBefore,body,[p,null]);A(remove,p,[]);return A(parent,p,[])===null});
attempt('next',()=>{const a=A(create,document,['i']),b=A(create,document,['b']);A(append,body,[a]);A(append,body,[b]);return A(next,a,[])===b});
// <style> parsing (the CSSOM) is checked only when the caller sets __withStyle.
if(globalThis.__withStyle)attempt('style',()=>{const s=A(create,document,['style']);A(setText,s,['p{color:red}i{color:blue}']);A(append,body,[s]);return A(len,A(rules,A(sheet,s,[]),[]),[])});
attempt('event',()=>{const d=A(create,document,['div']);let seen='';A(listen,d,['click',()=>{seen+='l'}]);A(onclick,d,[()=>{seen+='h'}]);A(dispatch,d,[new Ev('click')]);return seen});
attempt('query',()=>A(qs,document,['span'])!==null&&A(hlen,A(byTag,document,['iframe']),[]));
for(let i=saved.length-1;i>=0;i--){const r=saved[i];if(r[2])def(r[0],r[1],r[2]);else delete r[0][r[1]]}
out.patched=count>600;
return out;
})()

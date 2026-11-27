(function () {
  'use strict';
  const g=globalThis,ns=__tsNs,resolve=g.__historyURL,clone=g.__historyClone,schedule=g.__historySchedule;
  delete g.__historyURL;delete g.__historyClone;delete g.__historySchedule;
  const location=ns.defaultLocation;delete ns.defaultLocation;
  if(!location||g.location!==location||g.history!==undefined)return;
  const make=ns.makeNativeFunction,define=Object.defineProperty,apply=Reflect.apply;
  const TypeErrorValue=TypeError,StringValue=String,NumberValue=Number,Exception=ns.DOMException;
  const dispatch=g.dispatchEvent,PopState=g.PopStateEvent,HashChange=g.HashChangeEvent;
  const keys=['href','origin','protocol','host','hostname','port','pathname','search','hash'];
  let entries=[{url:location.href,state:null}],index=0,restoration='auto',activeState=null;
  const History=make(function(){throw new TypeErrorValue('Illegal constructor');},'History',0,true,true);
  const prototype=History.prototype,history=Object.create(prototype);
  function check(receiver){if(receiver!==history)throw new TypeErrorValue('Illegal invocation');}
  function string(value){
    if(typeof value==='symbol')throw new TypeErrorValue('Cannot convert a Symbol value to a string');
    return StringValue(value);
  }
  function update(url){
    for(let i=0;i<keys.length;i++)location[keys[i]]=url[keys[i]];
  }
  function change(receiver,state,unused,url,count,push){
    check(receiver);
    if(count<2)throw new TypeErrorValue('History requires 2 arguments');
    string(unused);
    const next=resolve(location.href,url==null||url===''?location.href:string(url));
    if(next===null)throw new Exception('The URL cannot be rewritten by this document.','SecurityError');
    const saved=clone(state),entry={url:next.href,state:saved},current=clone(saved);
    if(push){entries.length=index+1;entries[++index]=entry;}else entries[index]=entry;
    activeState=current;
    update(next);
  }
  function travel(receiver,delta){
    check(receiver);
    delta=NumberValue(delta)|0;
    if(delta===0)throw new Exception('Reload requires a navigation host.','NotSupportedError');
    if(!schedule(function(){
      const target=index+delta;
      if(target<0||target>=entries.length)return;
      const oldURL=location.href,oldHash=location.hash,entry=entries[target];
      const next=resolve(location.href,entry.url);
      if(next===null)return;
      const restored=clone(entry.state);
      index=target;activeState=restored;update(next);
      apply(dispatch,g,[new PopState('popstate',{state:activeState})]);
      if(oldHash!==location.hash)apply(dispatch,g,[new HashChange('hashchange',{oldURL,newURL:location.href})]);
    }))throw new Exception('History traversal requires a task scheduler.','NotSupportedError');
  }
  function method(name,length,fn){define(prototype,name,{__proto__:null,value:make(fn,name,length),writable:true,enumerable:true,configurable:true});}
  function getter(name,fn){define(prototype,name,{__proto__:null,get:make(fn,'get '+name,0),enumerable:true,configurable:true});}
  getter('length',function(){check(this);return entries.length;});
  getter('state',function(){check(this);return activeState;});
  define(prototype,'scrollRestoration',{__proto__:null,get:make(function(){check(this);return restoration;},'get scrollRestoration',0),set:make(function(value){check(this);value=string(value);if(value==='auto'||value==='manual')restoration=value;},'set scrollRestoration',1),enumerable:true,configurable:true});
  method('go',0,function(delta){travel(this,delta);});
  method('back',0,function(){travel(this,-1);});
  method('forward',0,function(){travel(this,1);});
  method('pushState',2,function(state,unused,url){change(this,state,unused,url,arguments.length,true);});
  method('replaceState',2,function(state,unused,url){change(this,state,unused,url,arguments.length,false);});
  delete prototype.constructor;
  define(prototype,'constructor',{__proto__:null,value:History,writable:true,configurable:true});
  define(prototype,Symbol.toStringTag,{__proto__:null,value:'History',configurable:true});
  define(History,'prototype',{__proto__:null,writable:false});
  define(g,'History',{__proto__:null,value:History,writable:true,configurable:true});
  define(g,'history',{__proto__:null,get:make(function(){return history;},'get history',0),enumerable:true,configurable:true});
})();

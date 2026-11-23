(function () {
  'use strict';
  const g=globalThis,native=g.__domParserNative;delete g.__domParserNative;
  if(typeof g.DOMParser==='function'&&typeof g.DOMParser.prototype.parseFromString==='function')return;
  const make=__tsNs.makeNativeFunction,define=Object.defineProperty,apply=Reflect.apply;
  const StringValue=String,TypeErrorValue=TypeError,instances=new WeakSet();
  const has=WeakSet.prototype.has,add=WeakSet.prototype.add;
  const factory=g.trustedTypes,isHTML=TrustedTypePolicyFactory.prototype.isHTML;
  const types=['text/html','text/xml','application/xml','application/xhtml+xml','image/svg+xml'];
  const includes=Array.prototype.includes;
  const own=Object.prototype.hasOwnProperty;
  const view=Object.getOwnPropertyDescriptor(Document.prototype,'defaultView').get;
  const locationGet=make(function(){
    const window=apply(view,this,[]);
    return window===null?null:window.location;
  },'get location',0);
  const locationSet=make(function(value){
    const location=apply(locationGet,this,[]);
    if(location!==null)location.href=string(value);
  },'set location',1);
  const prefix="Failed to execute 'parseFromString' on 'DOMParser': ";
  function string(value) {
    if(typeof value==='symbol')throw new TypeErrorValue('Cannot convert a Symbol value to a string');
    return StringValue(value);
  }
  const DOMParser=make(function(){
    if(!new.target)throw new TypeErrorValue("Failed to construct 'DOMParser': Please use the 'new' operator.");
    apply(add,instances,[this]);
  },'DOMParser',0,true,true);
  const prototype=DOMParser.prototype;
  define(prototype,'parseFromString',{__proto__:null,value:make(function(input,type){
    if(!apply(has,instances,[this]))throw new TypeErrorValue('Illegal invocation');
    if(arguments.length<2)throw new TypeErrorValue(prefix+'2 arguments required, but only '+arguments.length+' present.');
    if(!apply(isHTML,factory,[input]))input=string(input);
    type=string(type);
    if(!apply(includes,types,[type]))throw new TypeErrorValue(prefix+"The provided value '"+type+"' is not a valid enum value of type SupportedType.");
    const parsed=native(input,type);
    if(!apply(own,parsed,['location']))define(parsed,'location',{__proto__:null,get:locationGet,set:locationSet,enumerable:true,configurable:false});
    return parsed;
  },'parseFromString',2),writable:true,enumerable:true,configurable:true});
  delete prototype.constructor;
  define(prototype,'constructor',{__proto__:null,value:DOMParser,writable:true,configurable:true});
  define(prototype,Symbol.toStringTag,{__proto__:null,value:'DOMParser',configurable:true});
  define(DOMParser,'prototype',{__proto__:null,writable:false});
  define(g,'DOMParser',{__proto__:null,value:DOMParser,writable:true,configurable:true});
})();

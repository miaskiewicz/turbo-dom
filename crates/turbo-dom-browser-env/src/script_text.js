(function () {
  'use strict';
  const g=globalThis,ns=g[Symbol.for('ts')],native=g.__scriptTextNative;
  delete g.__scriptTextNative;
  const apply=Reflect.apply;
  const check=ns.domAccessor('nodeType',false,'HTML:script');
  Object.defineProperty(g.HTMLScriptElement.prototype,'text',{
    enumerable:true,configurable:true,
    get:ns.makeNativeFunction(function(){return apply(native,this,[]);},'get text',0),
    set:ns.makeNativeFunction(function(value){apply(check,this,[]);return apply(native,this,[value]);},'set text',1),
  });
})();

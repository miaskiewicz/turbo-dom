// turbo-dom#488: AbortSignal.timeout / .any polyfill — re-routes both static
// methods through `globalThis.setTimeout` (resolved at call time so the
// embedder's wrapped one is picked up) and a local AbortController.abort. This
// ensures the abort reactions flow through the embedder's dispatch path, where
// the engine-task bracket (`__engineTaskScope`) can fire the per-listener
// microtask drain.
//
// Separated from event_listeners.js (turbo-dom#488): the main IIFE ran inside
// a single `run_js` call whose TryCatch swallowed any exception, so a throw
// anywhere in the IIFE silently prevented the polyfill from installing. By
// running the polyfill as its own `run_js` call, a partial failure of the main
// IIFE does not block the override, and an error here is reported independently.
//
// The polyfill is installed unconditionally: V8's own AbortSignal.timeout fires
// through an internal C++ path the embedder cannot bracket, so replacing it is
// the fix (see event_listeners.js comments above `__engineTaskScope`).
(function(){
  'use strict';
  var g=globalThis,make=__tsNs.makeNativeFunction,define=Object.defineProperty,apply=Reflect.apply;
  var NativeAbortController=g.AbortController,NativeAbortSignal=g.AbortSignal;
  if(!NativeAbortSignal||!NativeAbortController)return;
  var NativeException=g.DOMException;
  var desc=Object.getOwnPropertyDescriptor(NativeAbortSignal.prototype,'aborted');
  if(!desc||typeof desc.get!=='function')return;
  var signalGetter=desc.get;
  var localAborted=function(signal){return apply(signalGetter,signal,[]);};
  var signalAdd=NativeAbortSignal.prototype.addEventListener;
  var slice=function(value){var result=[];for(var i=0;i<value.length;i++){Object.defineProperty(result,i,{__proto__:null,value:value[i],writable:true,configurable:true,enumerable:true});}return result;};
  var timeoutImpl=function timeout(delay){
    var ms=+delay;
    var controller=new NativeAbortController();
    var signal=controller.signal;
    var fire=function(){
      if(!localAborted(signal))
        controller.abort(new NativeException('signal timed out','TimeoutError'));
    };
    g.setTimeout(fire,ms>=0?ms:0);
    return signal;
  };
  define(NativeAbortSignal,'timeout',{value:make(timeoutImpl,'timeout',1),writable:true,configurable:true});
  var scheduleForward=function(controller,target,reason){
    g.setTimeout(function(){if(!localAborted(target))controller.abort(reason);},0);
  };
  var anyImpl=function any(signals){
    var controller=new NativeAbortController();
    var target=controller.signal;
    var list=slice(signals==null?[]:signals);
    for(var index=0;index<list.length;index++){
      var input=list[index];
      localAborted(input);
      if(localAborted(input)){scheduleForward(controller,target,input.reason);return target;}
    }
    for(var index=0;index<list.length;index++){
      var input=list[index];
      apply(signalAdd,input,['abort',function(inp){return function(){scheduleForward(controller,target,inp.reason);};}(input),{once:true}]);
    }
    return target;
  };
  define(NativeAbortSignal,'any',{value:make(anyImpl,'any',1),writable:true,configurable:true});
})();

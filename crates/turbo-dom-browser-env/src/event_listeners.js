(function(){
  'use strict';
  const g=globalThis,ns=__tsNs,native=g.__windowListenerNative;delete g.__windowListenerNative;
  const make=ns.makeNativeFunction,apply=Reflect.apply,create=Object.create,StringValue=String,ErrorType=TypeError,define=Object.defineProperty,getPrototype=Object.getPrototypeOf,setPrototype=Object.setPrototypeOf,objectFreeze=Object.freeze;
  const Exception=__tsNs.DOMException;
  const oldHost=g.__captureHostEventDispatch,BaseTarget=g.EventTarget,NodeConstructor=g.Node,EventType=g.Event,document=g.document;
  const ownIndex=(value,index,item)=>define(value,index,{__proto__:null,value:item,writable:true,configurable:true,enumerable:true});
  const push=(value,item)=>{ownIndex(value,value.length,item);return value.length;};
  const slice=value=>{const result=[];for(let index=0;index<value.length;index++)ownIndex(result,index,value[index]);return result;};
  const some=(value,test)=>{for(let index=0;index<value.length;index++)if(test(value[index]))return true;return false;};
  const indexOf=(value,item)=>{for(let index=0;index<value.length;index++)if(value[index]===item)return index;return -1;};
  const splice=(value,index)=>{for(let at=index;at+1<value.length;at++)ownIndex(value,at,value[at+1]);delete value[value.length-1];value.length--;};
  const NativeTarget=make(class EventTarget extends BaseTarget{constructor(){super();native('targetSet',this,{__proto__:null,listeners:[],handlers:create(null),parent:null})}},'EventTarget',0,true,true);
  NativeTarget.prototype=BaseTarget.prototype;define(NativeTarget.prototype,'constructor',{value:NativeTarget,writable:true,configurable:true});define(NativeTarget,'prototype',{writable:false});define(g,'EventTarget',{value:NativeTarget,writable:true,configurable:true});ns.eventTargetConstructor=NativeTarget;
  // An engine object that is an EventTarget without being constructed as one (html_media.js's
  // TextTrackList, a named object for its indexed tracks).
  ns.makeEventTarget=object=>{native('targetSet',object,{__proto__:null,listeners:[],handlers:create(null),parent:null});return object;};
  // turbo-dom#381: embedder-settable parent hook on a non-Node EventTarget. `fn(target)` returns
  // the next ancestor (or null/undefined to stop). Chain walks are capped at 100 hops.
  ns.setEventTargetParent=(target,fn)=>{const state=native('targetGet',target);if(!state)throw new ErrorType('Illegal invocation');if(fn!=null&&typeof fn!=='function')throw new ErrorType('parent must be a function');state.parent=fn==null?null:fn;};
  // Node was projected before the listener-backed constructor replaced EventTarget.
  // Keep its constructor inheritance aligned without replacing its instance prototype.
  if(getPrototype(NodeConstructor)===BaseTarget)setPrototype(NodeConstructor,NativeTarget);
  const readEvent=event=>{const state=native('event',event);if(!state)throw new ErrorType('Argument must be an Event');return state;};
  const method=(object,name,fn,length)=>define(object,name,{value:make(fn,name,length),writable:true,configurable:true,enumerable:true});
  const getter=(object,name,fn)=>define(object,name,{get:make(fn,'get '+name,0),configurable:true,enumerable:true});
  const string=value=>{if(typeof value==='symbol')throw new ErrorType('Cannot convert a Symbol to a string');return StringValue(value);};
  if(!Object.getOwnPropertyDescriptor(g.ErrorEvent.prototype,'message')?.get){
    const states=new WeakMap(),read=states.get.bind(states),write=states.set.bind(states);
    const fields=['message','filename','lineno','colno','error'];
    class ErrorImplementation extends EventType{
      constructor(type,init={}){if(!arguments.length)throw new ErrorType('ErrorEvent requires a type');super(type,init);init=init??{};const colno=(+init.colno)>>>0,error=init.error??null,rawFilename=init.filename,filename=rawFilename===undefined?'':string(rawFilename),lineno=(+init.lineno)>>>0,rawMessage=init.message,message=rawMessage===undefined?'':string(rawMessage);write(this,{message,filename,lineno,colno,error});}
    }
    const ErrorCtor=make(ErrorImplementation,'ErrorEvent',1,true,true);ErrorCtor.prototype=ErrorImplementation.prototype;define(ErrorCtor.prototype,'constructor',{value:ErrorCtor,writable:true,configurable:true});define(ErrorCtor,'prototype',{writable:false});define(ErrorCtor.prototype,Symbol.toStringTag,{value:'ErrorEvent',configurable:true});
    for(const name of fields)getter(ErrorCtor.prototype,name,function(){const state=read(this);if(!state)throw new ErrorType('Illegal invocation');return state[name];});
    define(g,'ErrorEvent',{value:ErrorCtor,writable:true,configurable:true});
  }
  let signalGetter=Object.getOwnPropertyDescriptor(g.AbortSignal.prototype,'aborted')?.get;
  // Preserve an embedding runtime's genuine implementation. The standalone DOM
  // supplies a real abort primitive instead of its old empty constructor shells.
  if(!signalGetter){
    const Signal=make(function(){throw new ErrorType('Illegal constructor');},'AbortSignal',0,true,true);
    Object.setPrototypeOf(Signal.prototype,BaseTarget.prototype);Object.setPrototypeOf(Signal,NativeTarget);define(Signal.prototype,Symbol.toStringTag,{value:'AbortSignal',configurable:true});define(Signal,'prototype',{writable:false});
    const read=signal=>{const record=native('signalGet',signal);if(!record)throw new ErrorType('Illegal invocation');return record;};
    getter(Signal.prototype,'aborted',function(){return read(this).aborted;});getter(Signal.prototype,'reason',function(){return read(this).reason;});
    define(Signal.prototype,'onabort',{configurable:true,enumerable:true,get:make(function(){read(this);return targetRecord(this).handlers.abort?.callback??null},'get onabort',0),set:make(function(value){read(this);targetHandler(this,'abort',value)},'set onabort',1)});
    method(Signal.prototype,'throwIfAborted',function(){const record=read(this);if(record.aborted)throw record.reason;},0);
    const Controller=make(function(){const signal=new NativeTarget();setPrototype(signal,Signal.prototype);native('signalSet',signal,{aborted:false,reason:undefined});native('controllerSet',this,{signal});},'AbortController',0,true,true);
    const controller=receiver=>{const record=native('controllerGet',receiver);if(!record)throw new ErrorType('Illegal invocation');return record;};
    getter(Controller.prototype,'signal',function(){return controller(this).signal;});
    method(Controller.prototype,'abort',function(reason){const signal=controller(this).signal,record=read(signal);if(record.aborted)return;record.aborted=true;record.reason=reason===undefined?new Exception('signal is aborted without reason','AbortError'):reason;dispatchTarget(new EventType('abort'),signal,true);},0);
    define(Controller.prototype,Symbol.toStringTag,{value:'AbortController',configurable:true});define(Controller,'prototype',{writable:false});
    method(Signal,'abort',function(reason){const controller=new Controller();controller.abort(reason);return controller.signal;},0);
    define(g,'AbortSignal',{value:Signal,writable:true,configurable:true});define(g,'AbortController',{value:Controller,writable:true,configurable:true});
    signalGetter=Object.getOwnPropertyDescriptor(Signal.prototype,'aborted').get;
  }
  const localAborted=signal=>apply(signalGetter,signal,[]);
  const aborted=signal=>{const value=native("aborted",signal);if(typeof value!=="boolean")throw new ErrorType("Signal must be an AbortSignal");return value;};
  let signalAdd=g.AbortSignal.prototype.addEventListener,signalRemove=g.AbortSignal.prototype.removeEventListener;
  g.__captureWindowEventInterfaces=interfaces=>{const signal=interfaces.AbortSignal;if(signal&&typeof signal[0]==='function'){signalGetter=signal[0];signalAdd=signal[1];signalRemove=signal[2];}};
  // Reads an options dictionary (or the legacy capture boolean) into a listener record.
  function readOptions(record,options,remove){
    if(options!=null&&(typeof options==='object'||typeof options==='function')){
      record.capture=!!options.capture;
      if(!remove){record.once=!!options.once;const value=options.passive;if(value!==undefined)record.passive=!!value;let signal=options.signal;if(signal===undefined)signal=null;else aborted(signal);record.signal=signal;}
    }else record.capture=!!options;
  }
  function flatten(type,callback,options,remove=false,passiveDefault=false){
    type=string(type);
    if(callback!=null&&typeof callback!=='function'&&typeof callback!=='object')throw new ErrorType('Listener must be an object');
    const record={__proto__:null,type,callback:callback??null,capture:false,once:false,passive:passiveDefault&&(type==='touchstart'||type==='touchmove'||type==='wheel'||type==='mousewheel'),signal:null,removed:false,abortHook:null};
    readOptions(record,options,remove);
    return record;
  }
  ns.eventTargetNativeBrand=receiver=>!!native('targetGet',receiver);
  function targetRecord(receiver){const record=native('targetGet',receiver);if(!record)throw new ErrorType('Illegal invocation');return record;}
  function targetRemove(target,state,record){record.removed=true;const index=indexOf(state.listeners,record);if(index>=0)splice(state.listeners,index);if(record.signal&&record.abortHook)native('subscription',record.signal,record.abortHook,false);native('targetChanged',target);}
  function targetHandler(target,type,value){const state=targetRecord(target);value=typeof value==='function'?value:null;const previous=state.handlers[type];if(previous){if(value===null){targetRemove(target,state,previous);delete state.handlers[type];}else previous.callback=value;}else if(value){const record={__proto__:null,type,callback:value,capture:false,once:false,passive:false,signal:null,removed:false,handler:true};state.handlers[type]=record;push(state.listeners,record);native('targetChanged',target);}}
  function targetListen(type,callback,options){const state=targetRecord(this);if(arguments.length<2)throw new ErrorType('addEventListener requires 2 arguments');const record=flatten(type,callback,options);if(record.callback===null||record.signal&&aborted(record.signal))return;if(some(state.listeners,item=>!item.handler&&!item.removed&&item.type===record.type&&item.callback===record.callback&&item.capture===record.capture))return;record.removed=false;push(state.listeners,record);if(record.signal){const target=this;record.abortHook=()=>targetRemove(target,state,record);native('subscription',record.signal,record.abortHook,true);}native('targetChanged',this);}
  function targetUnlisten(type,callback,options){const state=targetRecord(this);if(arguments.length<2)throw new ErrorType('removeEventListener requires 2 arguments');const wanted=flatten(type,callback,options,true);for(let index=0,list=slice(state.listeners);index<list.length;index++){const record=list[index];if(!record.handler&&!record.removed&&record.type===wanted.type&&record.callback===wanted.callback&&record.capture===wanted.capture)targetRemove(this,state,record);}}
  // turbo-dom#381: build the dispatch path by walking the embedder-set parent hook up the chain
  // (`request -> tx -> db` for IDB). Each hop reads its own target record; a missing or null
  // parent stops the walk.
  //
  // R1 expert H1: Chrome has no cap on the chain's depth. The pre-R1 limit of 100 silently
  // truncated deeper legitimate chains, so a bubble listener past hop 100 never fired. The cap
  // is now 10 000 and a chain that exceeds it throws a loud TypeError instead of silently
  // dropping ancestors (an adversarial infinite chain would otherwise DoS the dispatch).
  function collectPath(target,state){
    const path=[target];let node=target,hop=state.parent;
    while(hop){
      if(path.length>=10000)throw new ErrorType('EventTarget parent chain exceeded 10000 hops (runaway chain)');
      const next=hop(node);if(next==null)break;
      const record=native('targetGet',next);if(!record)break;
      push(path,next);node=next;hop=record.parent;
    }
    return path;
  }
  // Any listener for `type` on any level of `path`? Used by the listener-less fast-skip.
  function anyPathListener(path,type){
    for(let i=0;i<path.length;i++){const s=native('targetGet',path[i]);if(!s)continue;const list=s.listeners;for(let j=0;j<list.length;j++){const l=list[j];if(!l.removed&&l.type===type)return true;}}
    return false;
  }
  // Per-listener microtask drain is the OUTERMOST dispatch's job only.
  // The HTML spec's "clean up after running a callback" checkpoints
  // microtasks only when the realm is at the top of the script
  // settings stack — a page-script dispatch (`dispatchEvent`,
  // `AbortController.abort()`) that runs INSIDE another listener is
  // nested and never checkpoints between its own listeners. Chrome 156
  // matches: the surf abort_signal_any probe's `timeoutNestedAbort`
  // records `inner, after-inner, inner-microtask` — the inner
  // listener's microtask fires only after its outer continuation
  // (turbo-dom#456).
  //
  // `listenerDepth` tracks how deeply this dispatch is reentered;
  // `afterListener` decrements it and drains microtasks only when the
  // current listener call was the outermost one. Keeping the check in a
  // helper keeps the hot `invokeAt` / `invoke` loops at cc <= 15.
  let listenerDepth=0;
  function afterListener(record){record.passive=false;if(--listenerDepth===0)drainMicrotasksAcrossDispatch(record);}
  // Review R2 V8 M3: the per-listener microtask drain (R1 M3 / turbo-dom#382)
  // runs INSIDE dispatchTarget's try/finally while record.dispatching stays
  // true, so a microtask that synchronously re-dispatches the same event
  // would see `dispatching=true` on the shared record and throw
  // InvalidStateError — Chrome 156's dispatch flag is per-dispatch and
  // allows the re-dispatch (Chrome 156 probe:
  // `scripts/microtask-redispatch-chrome-control.mjs` records
  // `count=2, error=null`). Save every dispatch-scoped field, clear
  // dispatching for the drain, drain, then restore. A microtask's own
  // dispatch runs in its scope; the outer dispatch resumes unchanged.
  //
  // Review R3 H1: `stop` and `stopImmediate` are dispatch-scoped too —
  // dispatchTarget's outer finally unconditionally resets them, so a
  // nested re-dispatch's finally would otherwise wipe an outer
  // listener's `stopPropagation()`. Save and restore both, so the outer
  // dispatch's stop-propagation state survives the drain window. The
  // inner dispatch still sees its own fresh `stop`/`stopImmediate`
  // (restored to the saved values once the inner dispatch's finally has
  // reset them — which is its right: a re-dispatched event's stopping
  // state is its own, not the outer's).
  //
  // Review R4 H1: the save-set alone is not enough — the drain must also
  // CLEAR `stop`/`stopImmediate` BEFORE the checkpoint so a nested
  // re-dispatch (from a microtask) starts fresh. `dispatchTarget`'s entry
  // writes `dispatching` / `trusted` / `target` / `currentTarget` /
  // `eventPhase` / `path` but not `stop` / `stopImmediate`; it relies on
  // the previous dispatch's finally to have reset them. Without the
  // clear, an inner `dispatchMulti` enters with the outer's
  // `record.stop = true` and bails on its capture loop's first
  // `if (record.stop || record.stopImmediate) return;` — AT_TARGET and
  // bubble never run either, because `dispatchMulti` returns from the
  // capture loop, not per-phase.
  //
  // Review R4 H2: `record.trusted` is dispatch-scoped too —
  // `dispatchTarget`'s entry writes it from the `host` parameter but no
  // finally restores it. A trusted outer whose microtask re-dispatches
  // the same event (non-host) therefore flips the shared record's
  // `trusted` to false and leaves it false; the outer's subsequent
  // bubble listeners see `event.isTrusted === false`. Save `trusted`
  // alongside stop/stopImmediate. (No pre-checkpoint clear needed: the
  // inner's own `dispatchTarget` entry overwrites it anyway.)
  function drainMicrotasksAcrossDispatch(record){
    const savedDispatching=record.dispatching;
    const savedCurrent=record.currentTarget;
    const savedPhase=record.eventPhase;
    const savedPath=record.path;
    const savedTarget=record.target;
    const savedStop=record.stop;
    const savedStopImmediate=record.stopImmediate;
    const savedTrusted=record.trusted;
    record.dispatching=false;
    record.stop=false;
    record.stopImmediate=false;
    try{native('microtaskCheckpoint');}
    finally{
      record.dispatching=savedDispatching;
      record.currentTarget=savedCurrent;
      record.eventPhase=savedPhase;
      record.path=savedPath;
      record.target=savedTarget;
      record.stop=savedStop;
      record.stopImmediate=savedStopImmediate;
      record.trusted=savedTrusted;
    }
  }
  // Run every listener of `event` on `level` whose capture flag matches `captureFilter`
  // (null = any, which is AT_TARGET's contract). Per-listener invocation sets
  // currentTarget/eventPhase and scopes `composedPath()` to `[level]` (Chrome 156's IDB
  // contract). Handler records that return false (legacy on* cancel) set defaultPrevented.
  function invokeAt(event,record,level,captureFilter,phase){
    const state=native('targetGet',level);if(!state)return;
    record.currentTarget=level;record.eventPhase=phase;record.path=objectFreeze([level]);
    const list=slice(state.listeners);
    for(let index=0;index<list.length;index++){
      const listener=list[index];
      if(record.stopImmediate)break;
      if(listener.removed||listener.type!==record.type)continue;
      if(captureFilter!==null&&listener.capture!==captureFilter)continue;
      if(listener.signal&&aborted(listener.signal))continue;
      if(listener.once)targetRemove(level,state,listener);
      record.passive=listener.passive;
      listenerDepth++;
      try{const result=native('call',event,listener.callback,level);if(listener.handler&&result===false&&record.cancelable&&!record.passive)record.defaultPrevented=true;}finally{afterListener(record);}
    }
  }
  // Single-target fast path (no parent hook). Preserved from the pre-#381 implementation: at
  // AT_TARGET both capture and non-capture listeners fire in registration order.
  function dispatchSingle(event,record,target){invokeAt(event,record,target,null,2);}
  // Multi-level dispatch across the parent chain: capture -> AT_TARGET -> bubble, with the
  // listener-less fast-skip. `host` keeps the listener-less fast-skip from applying to
  // host-initiated dispatch (bubbling reaches side-effects on empty paths; preserved).
  function dispatchMulti(event,record,target,state,host){
    const path=collectPath(target,state);
    if(!host&&!anyPathListener(path,record.type))return;
    for(let i=path.length-1;i>0;i--){if(record.stop||record.stopImmediate)return;invokeAt(event,record,path[i],true,1);}
    if(!record.stopImmediate)invokeAt(event,record,target,null,2);
    if(record.bubbles)for(let i=1;i<path.length;i++){if(record.stop||record.stopImmediate)return;invokeAt(event,record,path[i],false,3);}
  }
  function dispatchTarget(event,target,host=false){
    const state=targetRecord(target),record=readEvent(event);if(record.dispatching||!record.type)throw new Exception('The event is already being dispatched or uninitialized','InvalidStateError');record.dispatching=true;record.trusted=host;record.target=target;record.currentTarget=target;record.eventPhase=2;record.path=objectFreeze([target]);
    try{if(state.parent)dispatchMulti(event,record,target,state,host);else dispatchSingle(event,record,target);}finally{record.currentTarget=null;record.eventPhase=0;record.path=objectFreeze([]);record.dispatching=false;record.stop=false;record.stopImmediate=false;record.passive=false;}return !record.defaultPrevented;
  }
  for(const [name,fn]of [['addEventListener',targetListen],['removeEventListener',targetUnlisten],['dispatchEvent',function(event){targetRecord(this);if(!arguments.length)throw new ErrorType('dispatchEvent requires 1 argument');return dispatchTarget(event,this)}]])ns.eventTargetCallbacks[name]=fn;
  const registry=new Map(),get=registry.get.bind(registry),set=registry.set.bind(registry);
  function remove(record){record.removed=true;const list=get(record.type),index=list?indexOf(list,record):-1;if(index>=0)splice(list,index);if(record.signal&&record.abortHook)native('subscription',record.signal,record.abortHook,false);}
  function add(type,callback,options){
    if(this!==g)throw new ErrorType('Illegal invocation');if(arguments.length<2)throw new ErrorType('addEventListener requires 2 arguments');
    type=string(type);const record=flatten(type,callback,options,false,true);
    if(record.callback===null||record.signal&&aborted(record.signal))return;
    const list=get(record.type)||[];if(some(list,value=>!value.removed&&!value.handler&&value.callback===record.callback&&value.capture===record.capture))return;
    record.removed=false;push(list,record);set(record.type,list);
    if(record.signal){record.abortHook=()=>remove(record);native('subscription',record.signal,record.abortHook,true);}
  }
  function removeListener(type,callback,options){
    if(this!==g)throw new ErrorType('Illegal invocation');if(arguments.length<2)throw new ErrorType('removeEventListener requires 2 arguments');
    const wanted=flatten(type,callback,options,true);for(let index=0,list=slice(get(wanted.type)||[]);index<list.length;index++){const record=list[index];if(!record.removed&&!record.handler&&record.callback===wanted.callback&&record.capture===wanted.capture)remove(record);}
  }
  // An on* handler: error events pass their five arguments and cancel on true.
  function callHandler(event,state,record){let args,special=false;if(state.type==='error'){const values=native('error',event);if(values){args=values;special=true;}}const result=native('call',event,record.callback,g,args);if(state.cancelable&&(special?result===true:result===false))state.defaultPrevented=true;}
  function invoke(event,capture,phase){
    const state=readEvent(event),records=slice(get(state.type)||[]);state.currentTarget=g;state.eventPhase=phase;
    for(let index=0;index<records.length;index++){const record=records[index];if(state.stopImmediate)break;if(record.removed||(capture!==null&&record.capture!==capture)||record.signal&&aborted(record.signal))continue;if(record.once)remove(record);state.passive=record.passive;
      listenerDepth++;
      try{if(record.handler)callHandler(event,state,record);else native('call',event,record.callback,g);}finally{listenerDepth--;state.passive=false;}
    }
  }
  function dispatch(event,host=false){
    const state=readEvent(event);if(state.dispatching||!state.type)throw new Exception('The event is already being dispatched or uninitialized','InvalidStateError');
    state.dispatching=true;state.trusted=host;state.target=g;state.currentTarget=g;state.eventPhase=2;state.path=objectFreeze([g]);
    const legacy=host&&(state.type==='load'||state.type==='pageshow');if(legacy)state.target=document;
    try{invoke(event,null,2);}finally{if(!legacy){state.currentTarget=null;state.eventPhase=0;state.path=objectFreeze([]);state.dispatching=false;state.stop=false;state.stopImmediate=false;}state.passive=false;}
    return !state.defaultPrevented;
  }
  for(const type of 'abort blur cancel change click close contextmenu dblclick error focus hashchange input invalid keydown keypress keyup languagechange load message messageerror offline online pagehide pageshow popstate rejectionhandled reset resize scroll storage submit toggle unhandledrejection unload wheel beforeunload'.split(' ')){
    let current=null,record=null;define(g,'on'+type,{enumerable:true,configurable:true,get:make(function(){if(this!==g)throw new ErrorType('Illegal invocation');return current;},'get on'+type,0),set:make(function(value){if(this!==g)throw new ErrorType('Illegal invocation');value=typeof value==='function'?value:null;current=value;if(record){if(value===null){remove(record);record=null;}else record.callback=value;}else if(value){record={__proto__:null,type,callback:value,capture:false,once:false,passive:false,signal:null,removed:false,handler:true};const list=get(type)||[];push(list,record);set(type,list);}},'set on'+type,1)});
  }
  method(g,'addEventListener',add,2);method(g,'removeEventListener',removeListener,2);method(g,'dispatchEvent',function(event){if(this!==g)throw new ErrorType('Illegal invocation');if(!arguments.length)throw new ErrorType('dispatchEvent requires 1 argument');return dispatch(event);},1);
  for(const name of ['addEventListener','removeEventListener','dispatchEvent'])ns.windowEventCallbacks[name]=g[name];
  g.__captureHostEventDispatch=(event,target)=>target===g?dispatch(event,true):native('targetGet',target)?dispatchTarget(event,target,true):oldHost(event,target);
  g.__captureWindowEventPhase=(event,capture)=>invoke(event,capture,capture?1:3);
  g.__captureListenerOptions=flatten;g.__captureSignalAborted=localAborted;g.__captureSignalSubscription=(signal,hook,subscribe)=>apply(subscribe?signalAdd:signalRemove,signal,['abort',hook,subscribe?{once:true}:undefined]);
})();

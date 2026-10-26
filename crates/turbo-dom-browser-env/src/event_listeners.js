(function(){
  'use strict';
  const g=globalThis,ns=g[Symbol.for('ts')],native=g.__windowListenerNative;delete g.__windowListenerNative;
  const make=ns.makeNativeFunction,apply=Reflect.apply,create=Object.create,StringValue=String,ErrorType=TypeError,define=Object.defineProperty,getPrototype=Object.getPrototypeOf,setPrototype=Object.setPrototypeOf;
  let Exception=g.DOMException;
  const oldHost=g.__captureHostEventDispatch,BaseTarget=g.EventTarget,NodeConstructor=g.Node,EventType=g.Event,document=g.document;
  const ownIndex=(value,index,item)=>define(value,index,{__proto__:null,value:item,writable:true,configurable:true,enumerable:true});
  const push=(value,item)=>{ownIndex(value,value.length,item);return value.length;};
  const slice=value=>{const result=[];for(let index=0;index<value.length;index++)ownIndex(result,index,value[index]);return result;};
  const some=(value,test)=>{for(let index=0;index<value.length;index++)if(test(value[index]))return true;return false;};
  const indexOf=(value,item)=>{for(let index=0;index<value.length;index++)if(value[index]===item)return index;return -1;};
  const splice=(value,index)=>{for(let at=index;at+1<value.length;at++)ownIndex(value,at,value[at+1]);delete value[value.length-1];value.length--;};
  const NativeTarget=make(class EventTarget extends BaseTarget{constructor(){super();native('targetSet',this,{__proto__:null,listeners:[],handlers:create(null)})}},'EventTarget',0,true,true);
  NativeTarget.prototype=BaseTarget.prototype;define(NativeTarget.prototype,'constructor',{value:NativeTarget,writable:true,configurable:true});define(NativeTarget,'prototype',{writable:false});define(g,'EventTarget',{value:NativeTarget,writable:true,configurable:true});
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
    const Controller=make(function(){const signal=new NativeTarget();Object.setPrototypeOf(signal,Signal.prototype);native('signalSet',signal,{aborted:false,reason:undefined});native('controllerSet',this,{signal});},'AbortController',0,true,true);
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
  g.__captureWindowEventInterfaces=interfaces=>{if(typeof interfaces.DOMException==='function')Exception=interfaces.DOMException;const signal=interfaces.AbortSignal;if(signal&&typeof signal[0]==='function'){signalGetter=signal[0];signalAdd=signal[1];signalRemove=signal[2];}};
  function flatten(type,callback,options,remove=false,passiveDefault=false){
    type=string(type);
    if(callback!=null&&typeof callback!=='function'&&typeof callback!=='object')throw new ErrorType('Listener must be an object');
    let capture=false,once=false,passive=passiveDefault&&(type==='touchstart'||type==='touchmove'||type==='wheel'||type==='mousewheel'),signal=null;
    if(options!=null&&(typeof options==='object'||typeof options==='function')){
      capture=!!options.capture;
      if(!remove){once=!!options.once;const value=options.passive;if(value!==undefined)passive=!!value;signal=options.signal??null;if(signal!==null)aborted(signal);}
    }else capture=!!options;
    return {__proto__:null,type,callback:callback??null,capture,once,passive,signal,removed:false,abortHook:null};
  }
  ns.eventTargetNativeBrand=receiver=>!!native('targetGet',receiver);
  function targetRecord(receiver){const record=native('targetGet',receiver);if(!record)throw new ErrorType('Illegal invocation');return record;}
  function targetRemove(target,state,record){record.removed=true;const index=indexOf(state.listeners,record);if(index>=0)splice(state.listeners,index);if(record.signal&&record.abortHook)native('subscription',record.signal,record.abortHook,false);native('targetChanged',target);}
  function targetHandler(target,type,value){const state=targetRecord(target);value=typeof value==='function'?value:null;const previous=state.handlers[type];if(previous){if(value===null){targetRemove(target,state,previous);delete state.handlers[type];}else previous.callback=value;}else if(value){const record={__proto__:null,type,callback:value,capture:false,once:false,passive:false,signal:null,removed:false,handler:true};state.handlers[type]=record;push(state.listeners,record);native('targetChanged',target);}}
  function targetListen(type,callback,options){const state=targetRecord(this);if(arguments.length<2)throw new ErrorType('addEventListener requires 2 arguments');const record=flatten(type,callback,options);if(record.callback===null||record.signal&&aborted(record.signal))return;if(some(state.listeners,item=>!item.handler&&!item.removed&&item.type===record.type&&item.callback===record.callback&&item.capture===record.capture))return;record.removed=false;push(state.listeners,record);if(record.signal){const target=this;record.abortHook=()=>targetRemove(target,state,record);native('subscription',record.signal,record.abortHook,true);}native('targetChanged',this);}
  function targetUnlisten(type,callback,options){const state=targetRecord(this);if(arguments.length<2)throw new ErrorType('removeEventListener requires 2 arguments');const wanted=flatten(type,callback,options,true);for(let index=0,list=slice(state.listeners);index<list.length;index++){const record=list[index];if(!record.handler&&!record.removed&&record.type===wanted.type&&record.callback===wanted.callback&&record.capture===wanted.capture)targetRemove(this,state,record);}}
  function dispatchTarget(event,target,host=false){
    const state=targetRecord(target),record=readEvent(event);if(record.dispatching||!record.type)throw new Exception('The event is already being dispatched or uninitialized','InvalidStateError');record.dispatching=true;record.trusted=host;record.target=target;record.currentTarget=target;record.eventPhase=2;record.path=[target];
    try{const list=slice(state.listeners);for(let index=0;index<list.length;index++){const listener=list[index];if(record.stopImmediate)break;if(listener.removed||listener.type!==record.type||listener.signal&&aborted(listener.signal))continue;if(listener.once)targetRemove(target,state,listener);record.passive=listener.passive;try{let result;if(typeof listener.callback==='function')result=apply(listener.callback,target,[event]);else result=apply(listener.callback.handleEvent,listener.callback,[event]);if(listener.handler&&result===false&&record.cancelable&&!record.passive)record.defaultPrevented=true;}catch(error){/* Complete listener exception reporting is an embedding followup. */}finally{record.passive=false;}}}finally{record.currentTarget=null;record.eventPhase=0;record.path=[];record.dispatching=false;record.stop=false;record.stopImmediate=false;record.passive=false;}return !record.defaultPrevented;
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
  function invoke(event,capture,phase){
    const state=readEvent(event),records=slice(get(state.type)||[]);state.currentTarget=g;state.eventPhase=phase;
    for(let index=0;index<records.length;index++){const record=records[index];if(state.stopImmediate)break;if(record.removed||(capture!==null&&record.capture!==capture)||record.signal&&aborted(record.signal))continue;if(record.once)remove(record);state.passive=record.passive;
      try{if(record.handler){let args=[event],special=false;if(state.type==='error'){const values=native('error',event);if(values){args=values;special=true;}}const result=apply(record.callback,g,args);if(state.cancelable&&(special?result===true:result===false))state.defaultPrevented=true;}else if(typeof record.callback==='function')apply(record.callback,g,[event]);else apply(record.callback.handleEvent,record.callback,[event]);}catch(error){/* The embedding runtime still needs a complete listener-error reporter. */}finally{state.passive=false;}
    }
  }
  function dispatch(event,host=false){
    const state=readEvent(event);if(state.dispatching||!state.type)throw new Exception('The event is already being dispatched or uninitialized','InvalidStateError');
    state.dispatching=true;state.trusted=host;state.target=g;state.currentTarget=g;state.eventPhase=2;state.path=[g];
    const legacy=host&&(state.type==='load'||state.type==='pageshow');if(legacy)state.target=document;
    try{invoke(event,null,2);}finally{if(!legacy){state.currentTarget=null;state.eventPhase=0;state.path=[];state.dispatching=false;state.stop=false;state.stopImmediate=false;}state.passive=false;}
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

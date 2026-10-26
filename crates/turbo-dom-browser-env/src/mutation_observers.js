(function () {
  'use strict';
  const g=globalThis,native=g.__mutationObserverNative;delete g.__mutationObserverNative;
  const ns=g[Symbol.for('ts')],make=ns.makeNativeFunction,named=ns.makeNamedObject;
  const apply=Reflect.apply,define=Object.defineProperty,create=Object.create;
  const TypeErrorValue=TypeError,StringValue=String,NumberValue=Number;
  const weakGet=WeakMap.prototype.get,weakSet=WeakMap.prototype.set;
  const WeakRefValue=WeakRef,deref=WeakRef.prototype.deref,iteratorSymbol=Symbol.iterator;
  const type=Object.getOwnPropertyDescriptor(Node.prototype,'nodeType').get;
  const listPrototype=NodeList.prototype,states=new WeakMap(),records=new WeakMap();
  const own=(rows,index,value)=>define(rows,index,{__proto__:null,value,writable:true,enumerable:true,configurable:true});
  const append=(rows,value)=>own(rows,rows.length,value);
  const get=(map,key)=>apply(weakGet,map,[key]);
  const put=(map,key,value)=>apply(weakSet,map,[key,value]);
  const read=reference=>apply(deref,reference,[]);
  const string=value=>{if(typeof value==='symbol')throw new TypeErrorValue('Cannot convert a Symbol to a string');return StringValue(value);};
  const state=observer=>{const value=get(states,observer);if(!value)throw new TypeErrorValue('Illegal invocation');return value;};
  const contains=(rows,value)=>{for(let index=0;index<rows.length;index++)if(rows[index]===value)return true;return false;};
  function list(rows) {
    const object=named(listPrototype,function(operation,key){
      const index=NumberValue(key),indexed=index>=0&&index<4294967295&&StringValue(index)===key&&index%1===0;
      if(operation==='indices'){const keys=[];for(let index=0;index<rows.length;index++)append(keys,index);return keys;}
      if(operation==='keys')return [];
      if(!indexed||index>=rows.length)return undefined;
      if(operation==='get')return rows[index];
      if(operation==='has')return true;
      if(operation==='descriptor')return {__proto__:null,value:rows[index],writable:false,enumerable:true,configurable:true};
      if(operation==='delete')return false;
      if(operation==='set'||operation==='define')return true;
    },true);
    native('record',object,{__proto__:null,kind:'NodeList',read:()=>rows});
    return object;
  }
  const MutationRecord=make(function(){throw new TypeErrorValue('Illegal constructor');},'MutationRecord',0,true,true);
  const recordPrototype=MutationRecord.prototype;
  for(const name of ['type','target','addedNodes','removedNodes','previousSibling','nextSibling','attributeName','attributeNamespace','oldValue']){
    define(recordPrototype,name,{__proto__:null,get:make(function(){const value=get(records,this);if(!value)throw new TypeErrorValue('Illegal invocation');return value[name];},'get '+name,0),enumerable:true,configurable:true});
  }
  define(recordPrototype,Symbol.toStringTag,{__proto__:null,value:'MutationRecord',configurable:true});
  define(MutationRecord,'prototype',{__proto__:null,writable:false});
  define(g,'MutationRecord',{__proto__:null,value:MutationRecord,writable:true,configurable:true});
  function options(value) {
    if(value==null)value={__proto__:null};
    if(typeof value!=='object'&&typeof value!=='function')throw new TypeErrorValue('Options must be a dictionary');
    const filter=value.attributeFilter;
    let attributeFilter;
    if(filter!==undefined){
      if(filter==null||typeof filter!=='object'&&typeof filter!=='function')throw new TypeErrorValue('attributeFilter must be a sequence');
      const iterator=filter[iteratorSymbol];if(typeof iterator!=='function')throw new TypeErrorValue('attributeFilter must be iterable');
      const cursor=apply(iterator,filter,[]),next=cursor.next;attributeFilter=[];
      for(;;){const row=apply(next,cursor,[]);if(row==null||typeof row!=='object'&&typeof row!=='function')throw new TypeErrorValue('Iterator result is not an object');if(row.done)break;
        append(attributeFilter,string(row.value));
      }
    }
    const attributeOldRaw=value.attributeOldValue,attributeOldValue=!!attributeOldRaw,attributeValue=value.attributes;
    const characterValue=value.characterData,characterOldRaw=value.characterDataOldValue,characterDataOldValue=!!characterOldRaw;
    const childList=!!value.childList,subtree=!!value.subtree;
    const attributes=attributeValue===undefined?(attributeOldRaw!==undefined||attributeFilter!==undefined):!!attributeValue;
    const characterData=characterValue===undefined?characterOldRaw!==undefined:!!characterValue;
    if(!attributes&&!characterData&&!childList||!attributes&&(attributeOldValue||attributeFilter!==undefined)||!characterData&&characterDataOldValue)throw new TypeErrorValue('Invalid observer options');
    return {__proto__:null,attributeFilter,attributeOldValue,attributes,characterData,characterDataOldValue,childList,subtree};
  }
  function release(observer,rows) {
    for(let index=0;index<rows.length;index++){const target=read(rows[index].target);if(target)native('anchor',observer,target,false);}
  }
  function removeTransients(observer,value) {
    release(observer,value.transients);value.transients=[];
    for(let index=0;index<value.registrations.length;index++){const target=read(value.registrations[index].target);if(target)native('anchor',observer,target,true);}
  }
  function matching(registration,event) {
    const target=read(registration.target),option=registration.options;
    if(!target||!contains(event.ancestors,target)||event.target!==target&&!option.subtree)return false;
    if(event.type==='attributes')return option.attributes&&(option.attributeFilter===undefined||event.attributeNamespace===null&&contains(option.attributeFilter,event.attributeName));
    if(event.type==='characterData')return option.characterData;
    return option.childList;
  }
  function queue(observer,value,event) {
    let interested=false,old=false;
    const groups=[value.registrations,value.transients];
    for(let group=0;group<groups.length;group++){const rows=groups[group];for(let index=0;index<rows.length;index++){
      const registration=rows[index];
      if(matching(registration,event)){interested=true;old=old||(event.type==='attributes'?registration.options.attributeOldValue:registration.options.characterDataOldValue);}
    }}
    if(event.type==='childList'&&event.removed.length){
      const sources=[];
      for(let group=0;group<groups.length;group++){const rows=groups[group];for(let index=0;index<rows.length;index++){
        const registration=rows[index],target=read(registration.target);
        if(target&&registration.options.subtree&&contains(event.ancestors,target))append(sources,registration);
      }}
      for(let index=0;index<sources.length;index++)for(let at=0;at<event.removed.length;at++){
        const target=event.removed[at];append(value.transients,{__proto__:null,target:new WeakRefValue(target),options:sources[index].options,source:sources[index].source||sources[index]});
        native('anchor',observer,target,true);
      }
      if(sources.length)native('schedule',observer);
    }
    if(!interested)return;
    const record=create(recordPrototype);
    put(records,record,{__proto__:null,type:event.type,target:event.target,addedNodes:list(event.added),removedNodes:list(event.removed),previousSibling:event.previousSibling,nextSibling:event.nextSibling,attributeName:event.attributeName,attributeNamespace:event.attributeNamespace,oldValue:old?event.oldValue:null});
    append(value.queue,record);native('schedule',observer);
  }
  const MutationObserver=make(function(callback){
    if(!new.target)throw new TypeErrorValue('Please use the new operator');
    if(typeof callback!=='function')throw new TypeErrorValue('Callback must be a function');
    const observer=this,value={__proto__:null,callback,queue:[],registrations:[],transients:[]};put(states,observer,value);
    native('register',observer,event=>queue(observer,value,event),()=>{
      const rows=value.queue;value.queue=[];removeTransients(observer,value);
      if(rows.length)apply(value.callback,observer,[rows,observer]);
    });
  },'MutationObserver',1,true,true);
  const prototype=MutationObserver.prototype;
  function method(name,length,fn){define(prototype,name,{__proto__:null,value:make(fn,name,length),writable:true,enumerable:true,configurable:true});}
  method('observe',1,function(target,init){
    const value=state(this);if(!arguments.length)throw new TypeErrorValue('observe requires a target');apply(type,target,[]);
    const option=options(init);native('flush');
    let registration;
    for(let index=0;index<value.registrations.length;index++)if(read(value.registrations[index].target)===target){registration=value.registrations[index];break;}
    if(registration){
      const keep=[];for(let index=0;index<value.transients.length;index++){const row=value.transients[index];if(row.source===registration){const node=read(row.target);if(node)native('anchor',this,node,false);}else append(keep,row);}
      value.transients=keep;registration.options=option;
      for(let index=0;index<keep.length;index++){const node=read(keep[index].target);if(node)native('anchor',this,node,true);}
    }else append(value.registrations,{__proto__:null,target:new WeakRefValue(target),options:option});
    native('anchor',this,target,true);native('recordTree',target);
  });
  method('disconnect',0,function(){const value=state(this);native('flush');release(this,value.registrations);value.registrations=[];removeTransients(this,value);value.queue=[];});
  method('takeRecords',0,function(){const value=state(this);native('flush');const rows=value.queue;value.queue=[];return rows;});
  define(prototype,Symbol.toStringTag,{__proto__:null,value:'MutationObserver',configurable:true});
  define(MutationObserver,'prototype',{__proto__:null,writable:false});
  define(g,'MutationObserver',{__proto__:null,value:MutationObserver,writable:true,configurable:true});
})();

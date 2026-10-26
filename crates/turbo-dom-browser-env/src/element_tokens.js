(function () {
  const g = globalThis, ns = g[Symbol.for('ts')];
  const getRecord = g.__getCollectionRecord, setRecord = g.__setCollectionRecord;
  const make = ns.makeNativeFunction, named = ns.makeNamedObject;
  const hasOwn=Function.prototype.call.bind(Object.prototype.hasOwnProperty),create=Object.create,define=Object.defineProperty,set=Reflect.set,lower=Function.prototype.call.bind(String.prototype.toLowerCase);
  const tag = Object.getOwnPropertyDescriptor(g.Element.prototype, 'tagName').get;
  const iframe = ns.domAccessor('nodeType',false,'HTML:iframe');
  const sandboxTokens = new Set(['allow-downloads','allow-forms','allow-modals','allow-orientation-lock','allow-pointer-lock','allow-popups','allow-popups-to-escape-sandbox','allow-presentation','allow-same-origin','allow-scripts','allow-storage-access-by-user-activation','allow-top-navigation','allow-top-navigation-by-user-activation','allow-same-site-none-cookies']);
  const supportedSandboxToken = Function.prototype.call.bind(Set.prototype.has,sandboxTokens);
  const focusGroupTokens = ['toolbar','tablist','radiogroup','listbox','menu','menubar','inline','block','wrap','nowrap','nomemory','none'];
  const nodeKind=Function.prototype.call.bind(Object.getOwnPropertyDescriptor(g.Node.prototype,'nodeType').get),nodeSpace=Function.prototype.call.bind(Object.getOwnPropertyDescriptor(g.Element.prototype,'namespaceURI').get);
  const elementSpace = space => function(){let valid=false;try{valid=nodeKind(this)===1&&nodeSpace(this)===space;}catch{}if(!valid)throw new TypeError('Illegal invocation');};
  const getAttribute = g.Element.prototype.getAttribute, setAttribute = g.Element.prototype.setAttribute;
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return String(value); };
  const split = value => Array.from(new Set(value.split(/[\t\n\f\r ]+/).filter(Boolean)));
  const record = list => { const value = getRecord(list); if (!value || value.kind !== 'DOMTokenList') throw new TypeError('Illegal invocation'); return value; };
  const raw = list => { const state = record(list); return getAttribute.call(state.owner, state.attribute) || ''; };
  const tokens = list => split(raw(list));
  const update = (list, values) => { const state = record(list); if (values.length || getAttribute.call(state.owner, state.attribute) !== null) setAttribute.call(state.owner, state.attribute, values.join(' ')); };
  const token = value => { value = string(value); if (!value) throw new g.DOMException('The token is empty', 'SyntaxError'); if (/[\t\n\f\r ]/.test(value)) throw new g.DOMException('The token contains ASCII whitespace', 'InvalidCharacterError'); return value; };
  const ctor = make(function () { throw new TypeError('Illegal constructor'); }, 'DOMTokenList', 0, true);
  const proto = ctor.prototype;
  Object.defineProperty(proto, Symbol.toStringTag, {value:'DOMTokenList', configurable:true});
  Object.defineProperty(ctor, 'prototype', {writable:false});
  Object.defineProperty(g, 'DOMTokenList', {value:ctor,writable:true,configurable:true});
  function method(name, arity, delegate) {
    Object.defineProperty(proto,name,{value:make(function(){record(this);if(arguments.length<arity)throw new TypeError(name+' requires '+arity+' arguments');return delegate.apply(this,arguments);},name,arity),writable:true,enumerable:true,configurable:true});
  }
  method('item',1,function(index){return tokens(this)[(+index)>>>0]||null;});
  method('contains',1,function(value){return tokens(this).includes(string(value));});
  method('add',0,function(){const additions=Array.from(arguments,token),values=tokens(this);for(const value of additions)if(!values.includes(value))values.push(value);update(this,values);});
  method('remove',0,function(){const removed=Array.from(arguments,token);update(this,tokens(this).filter(value=>!removed.includes(value)));});
  method('toggle',1,function(value,force){value=token(value);const values=tokens(this),index=values.indexOf(value),present=index>=0;const enabled=arguments.length>1?!!force:!present;if(enabled&&!present){values.push(value);update(this,values);}else if(!enabled&&present){values.splice(index,1);update(this,values);}return enabled;});
  method('replace',2,function(oldValue,newValue){oldValue=token(oldValue);newValue=token(newValue);const values=tokens(this),index=values.indexOf(oldValue);if(index<0)return false;const replacement=values.indexOf(newValue);if(replacement>=0){if(replacement>index){values.splice(replacement,1);values[index]=newValue;}else if(replacement!==index)values.splice(index,1);}else values[index]=newValue;update(this,values);return true;});
  method('supports',1,function(value){value=string(value);if(record(this).attribute==='sandbox')return supportedSandboxToken(lower(value));if(record(this).attribute==='rel')return ['noopener','noreferrer','opener'].indexOf(lower(value))>=0;if(record(this).attribute==='focusgroup')return focusGroupTokens.indexOf(lower(value))>=0;throw new TypeError('This attribute has no supported tokens');});
  method('forEach',1,function(callback,thisArg){if(typeof callback!=='function')throw new TypeError('The callback is not callable');for(let index=0;index<tokens(this).length;index++)callback.call(thisArg,tokens(this)[index],index,this);});
  method('toString',0,function(){return raw(this);});
  for (const name of ['entries','keys','values']) Object.defineProperty(proto,name,{value:Array.prototype[name],writable:true,enumerable:true,configurable:true});
  Object.defineProperty(proto,Symbol.iterator,{value:proto.values,writable:true,configurable:true});
  Object.defineProperty(proto,'length',{get:make(function(){return tokens(this).length;},'get length',0),enumerable:true,configurable:true});
  Object.defineProperty(proto,'value',{get:make(function(){return raw(this);},'get value',0),set:make(function(value){const state=record(this);setAttribute.call(state.owner,state.attribute,string(value));},'set value',1),enumerable:true,configurable:true});
  for (const [owner,name,attribute,check] of [[g.Element.prototype,'classList','class',tag],[g.Element.prototype,'part','part',tag],[g.HTMLIFrameElement.prototype,'sandbox','sandbox',iframe],[g.HTMLFormElement.prototype,'relList','rel',ns.domAccessor('nodeType',false,'HTML:form')],[g.HTMLOutputElement.prototype,'htmlFor','for',ns.domAccessor('nodeType',false,'HTML:output')],[g.HTMLElement.prototype,'focusGroup','focusgroup',elementSpace('http://www.w3.org/1999/xhtml')],[g.SVGElement.prototype,'focusGroup','focusgroup',elementSpace('http://www.w3.org/2000/svg')]]) {
    const read = function(){check.call(this);let shared=getRecord(this);if(!shared){shared=create(null);shared.kind='Element';setRecord(this,shared);}if(!hasOwn(shared,'tokenLists'))define(shared,'tokenLists',{value:create(null),writable:true});const cache=shared.tokenLists;if(cache[attribute])return cache[attribute];const owner=this;
      const list=named(proto,function(operation,key){const values=tokens(list),index=/^(0|[1-9]\d*)$/.test(key)?Number(key):null,exists=index!==null&&index<values.length;if(operation==='indices')return values.map((_,i)=>i);if(operation==='keys')return [];if(operation==='get')return exists?values[index]:undefined;if(operation==='has')return exists;if(operation==='descriptor')return exists?{value:values[index],writable:false,enumerable:true,configurable:true}:undefined;if(operation==='set'||operation==='define')return exists?true:undefined;if(operation==='delete')return exists?false:undefined;},true);
      const state=create(null);state.kind='DOMTokenList';state.owner=owner;state.attribute=attribute;setRecord(list,state);cache[attribute]=list;return list;};
    Object.defineProperty(owner,name,{get:make(read,'get '+name,0),set:make(function(value){const list=read.call(this);set(list,'value',value,list);},'set '+name,1),enumerable:true,configurable:true});
  }
})();

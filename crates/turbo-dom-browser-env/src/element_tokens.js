(function () {
  const g = globalThis, ns = __tsNs;
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
  // Engine calls go through captured intrinsics, never a page-replaceable builtin (turbo-dom#194).
  const apply = Reflect.apply, StringValue = String, NumberValue = Number;
  const add = (list, value) => { define(list, list.length, { value, writable: true, enumerable: true, configurable: true }); };
  const indexOf = (list, value) => { for (let index = 0; index < list.length; index++) if (list[index] === value) return index; return -1; };
  const join = list => { let out = ''; for (let index = 0; index < list.length; index++) out += (index ? ' ' : '') + list[index]; return out; };
  const removeAt = (list, at) => { const out = []; for (let index = 0; index < list.length; index++) if (index !== at) add(out, list[index]); return out; };
  const space = c => c === ' ' || c === '\t' || c === '\n' || c === '\f' || c === '\r';
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return StringValue(value); };
  const split = value => { const out = []; let word = ''; for (let index = 0; index <= value.length; index++) { const c = index < value.length ? value[index] : ' '; if (space(c)) { if (word && indexOf(out, word) < 0) add(out, word); word = ''; } else word += c; } return out; };
  const arrayIndex = key => { if (typeof key !== 'string' || !key.length || (key.length > 1 && key[0] === '0')) return null; for (let index = 0; index < key.length; index++) if (key[index] < '0' || key[index] > '9') return null; return NumberValue(key); };
  const record = list => { const value = getRecord(list); if (!value || value.kind !== 'DOMTokenList') throw new TypeError('Illegal invocation'); return value; };
  const raw = list => { const state = record(list); return apply(getAttribute, state.owner, [state.attribute]) || ''; };
  const tokens = list => split(raw(list));
  const update = (list, values) => { const state = record(list); if (values.length || apply(getAttribute, state.owner, [state.attribute]) !== null) apply(setAttribute, state.owner, [state.attribute, join(values)]); };
  const token = value => { value = string(value); if (!value) throw new ns.DOMException('The token is empty', 'SyntaxError'); let spaced = false; for (let index = 0; index < value.length; index++) if (space(value[index])) spaced = true; if (spaced) throw new ns.DOMException('The token contains ASCII whitespace', 'InvalidCharacterError'); return value; };
  const ctor = make(function () { throw new TypeError('Illegal constructor'); }, 'DOMTokenList', 0, true);
  const proto = ctor.prototype;
  Object.defineProperty(proto, Symbol.toStringTag, {value:'DOMTokenList', configurable:true});
  Object.defineProperty(ctor, 'prototype', {writable:false});
  Object.defineProperty(g, 'DOMTokenList', {value:ctor,writable:true,configurable:true});
  function method(name, arity, delegate) {
    Object.defineProperty(proto,name,{value:make(function(){record(this);if(arguments.length<arity)throw new TypeError(name+' requires '+arity+' arguments');return apply(delegate,this,arguments);},name,arity),writable:true,enumerable:true,configurable:true});
  }
  method('item',1,function(index){return tokens(this)[(+index)>>>0]||null;});
  method('contains',1,function(value){return indexOf(tokens(this),string(value))>=0;});
  method('add',0,function(){const additions=[];for(let index=0;index<arguments.length;index++)add(additions,token(arguments[index]));const values=tokens(this);for(let index=0;index<additions.length;index++)if(indexOf(values,additions[index])<0)add(values,additions[index]);update(this,values);});
  method('remove',0,function(){const removed=[];for(let index=0;index<arguments.length;index++)add(removed,token(arguments[index]));const values=tokens(this),kept=[];for(let index=0;index<values.length;index++)if(indexOf(removed,values[index])<0)add(kept,values[index]);update(this,kept);});
  method('toggle',1,function(value,force){value=token(value);const values=tokens(this),index=indexOf(values,value),present=index>=0;const enabled=arguments.length>1?!!force:!present;if(enabled&&!present){add(values,value);update(this,values);}else if(!enabled&&present)update(this,removeAt(values,index));return enabled;});
  method('replace',2,function(oldValue,newValue){oldValue=token(oldValue);newValue=token(newValue);let values=tokens(this);const index=indexOf(values,oldValue);if(index<0)return false;const replacement=indexOf(values,newValue);if(replacement>=0){if(replacement>index){values=removeAt(values,replacement);values[index]=newValue;}else if(replacement!==index)values=removeAt(values,index);}else values[index]=newValue;update(this,values);return true;});
  method('supports',1,function(value){value=string(value);if(record(this).attribute==='sandbox')return supportedSandboxToken(lower(value));if(record(this).attribute==='rel')return indexOf(['noopener','noreferrer','opener'],lower(value))>=0;if(record(this).attribute==='focusgroup')return indexOf(focusGroupTokens,lower(value))>=0;throw new TypeError('This attribute has no supported tokens');});
  method('forEach',1,function(callback,thisArg){if(typeof callback!=='function')throw new TypeError('The callback is not callable');for(let index=0;index<tokens(this).length;index++)apply(callback,thisArg,[tokens(this)[index],index,this]);});
  method('toString',0,function(){return raw(this);});
  for (const name of ['entries','keys','values']) Object.defineProperty(proto,name,{value:Array.prototype[name],writable:true,enumerable:true,configurable:true});
  Object.defineProperty(proto,Symbol.iterator,{value:proto.values,writable:true,configurable:true});
  Object.defineProperty(proto,'length',{get:make(function(){return tokens(this).length;},'get length',0),enumerable:true,configurable:true});
  Object.defineProperty(proto,'value',{get:make(function(){return raw(this);},'get value',0),set:make(function(value){const state=record(this);apply(setAttribute,state.owner,[state.attribute,string(value)]);},'set value',1),enumerable:true,configurable:true});
  for (const [owner,name,attribute,check] of [[g.Element.prototype,'classList','class',tag],[g.Element.prototype,'part','part',tag],[g.HTMLIFrameElement.prototype,'sandbox','sandbox',iframe],[g.HTMLFormElement.prototype,'relList','rel',ns.domAccessor('nodeType',false,'HTML:form')],[g.HTMLOutputElement.prototype,'htmlFor','for',ns.domAccessor('nodeType',false,'HTML:output')],[g.HTMLElement.prototype,'focusGroup','focusgroup',elementSpace('http://www.w3.org/1999/xhtml')],[g.SVGElement.prototype,'focusGroup','focusgroup',elementSpace('http://www.w3.org/2000/svg')],[g.MathMLElement.prototype,'focusGroup','focusgroup',elementSpace('http://www.w3.org/1998/Math/MathML')]]) {
    const read = function(){apply(check,this,[]);let shared=getRecord(this);if(!shared){shared=create(null);shared.kind='Element';setRecord(this,shared);}if(!hasOwn(shared,'tokenLists'))define(shared,'tokenLists',{value:create(null),writable:true});const cache=shared.tokenLists;if(cache[attribute])return cache[attribute];const owner=this;
      const list=named(proto,function(operation,key){const values=tokens(list),index=arrayIndex(key),exists=index!==null&&index<values.length;if(operation==='indices'){const out=[];for(let i=0;i<values.length;i++)add(out,i);return out;}if(operation==='keys')return [];if(operation==='get')return exists?values[index]:undefined;if(operation==='has')return exists;if(operation==='descriptor')return exists?{value:values[index],writable:false,enumerable:true,configurable:true}:undefined;if(operation==='set'||operation==='define')return exists?true:undefined;if(operation==='delete')return exists?false:undefined;},true);
      const state=create(null);state.kind='DOMTokenList';state.owner=owner;state.attribute=attribute;setRecord(list,state);cache[attribute]=list;return list;};
    Object.defineProperty(owner,name,{get:make(read,'get '+name,0),set:make(function(value){const list=apply(read,this,[]);set(list,'value',value,list);},'set '+name,1),enumerable:true,configurable:true});
  }
})();

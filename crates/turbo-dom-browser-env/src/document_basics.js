(function () {
  const g = globalThis;
  const ns = g[Symbol.for('ts')];
  const native = g.__documentBasicsNative;
  delete g.__documentBasicsNative;
  const getRecord=g.__getCollectionRecord, setRecord=g.__setCollectionRecord;
  delete g.__getCollectionRecord; delete g.__setCollectionRecord;
  const make = ns.makeNativeFunction;
  const type = Object.getOwnPropertyDescriptor(g.Node.prototype, 'nodeType').get;
  const rawChildren = Object.getOwnPropertyDescriptor(g.Node.prototype, 'childNodes').get;
  const tag = Object.getOwnPropertyDescriptor(g.Element.prototype, 'tagName').get;
  const namespace = Object.getOwnPropertyDescriptor(g.Element.prototype, 'namespaceURI').get;
  const getAttribute = g.Element.prototype.getAttribute;
  const hasAttribute = g.Element.prototype.hasAttribute;
  const setAttribute = g.Element.prototype.setAttribute;
  const html = 'http://www.w3.org/1999/xhtml';
  const StringValue = String;
  const string = value => { if (typeof value === 'symbol') throw new TypeError('Cannot convert a Symbol to a string'); return StringValue(value); };
  const check = (receiver, allowed) => { if (!allowed.includes(apply(type, receiver, []))) throw new TypeError('Illegal invocation'); };
  const state = document => native('state', document);
  const property = (proto, name, read, write) => {
    const descriptor = { configurable: true, enumerable: true, get: make(read, 'get ' + name, 0) };
    if (write) descriptor.set = make(write, 'set ' + name, 1);
    Object.defineProperty(proto, name, descriptor);
  };
  const method = (proto, name, delegate, arity) => Object.defineProperty(proto, name, { value: make(delegate, name, arity), writable: true, enumerable: true, configurable: true });
  const apply = Reflect.apply, ErrorType = TypeError;
  // HTML's obsolete Document methods explicitly preserve the document and ignore arguments.
  for (const name of ['clear', 'captureEvents', 'releaseEvents']) {
    method(g.Document.prototype, name, function () {
      if (apply(type, this, []) !== 9) throw new ErrorType('Illegal invocation');
    }, 0);
  }
  // XML Name uses code-point ranges, rather than HTML's permissive attribute names.
  const xmlName = /^[:A-Z_a-z\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02ff\u0370-\u037d\u037f-\u1fff\u200c-\u200d\u2070-\u218f\u2c00-\u2fef\u3001-\udfff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}][:A-Z_a-z\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02ff\u0370-\u037d\u037f-\u1fff\u200c-\u200d\u2070-\u218f\u2c00-\u2fef\u3001-\udfff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}\-\.0-9\u00b7\u0300-\u036f\u203f-\u2040]*$/u;
  method(g.Document.prototype, 'createProcessingInstruction', function (target, data) {
    check(this, [9]);
    if (arguments.length < 2) throw new TypeError('createProcessingInstruction requires 2 arguments');
    target = string(target); data = string(data);
    if (!xmlName.test(target) || /[\ud800-\udfff\ufffd]/u.test(target) || data.includes('?>')) throw new g.DOMException('Invalid processing instruction', 'InvalidCharacterError');
    return native('createPI', this, target, data);
  }, 2);
  method(g.Document.prototype, 'createCDATASection', function (data) {
    check(this, [9]);
    if (!arguments.length) throw new TypeError('createCDATASection requires 1 argument');
    string(data);
    throw new g.DOMException('CDATA sections are unsupported in HTML documents', 'NotSupportedError');
  }, 1);
  property(g.ProcessingInstruction.prototype, 'target', function () { check(this, [7]); return native('piTarget', this); });
  const data = Object.getOwnPropertyDescriptor(g.CharacterData.prototype, 'data');
  property(g.CharacterData.prototype, 'length', function () { check(this, [3, 4, 7, 8]); return data.get.call(this).length; });
  method(g.Node.prototype, 'normalize', function () { type.call(this); native('normalize', this); }, 0);
  const datasetValues = new WeakMap(), datasetGet=WeakMap.prototype.get, datasetSet=WeakMap.prototype.set;
  const nativeAttributes = Object.getOwnPropertyDescriptor(g.Element.prototype,'attributes').get;
  const nativeAttrName = Object.getOwnPropertyDescriptor(g.Attr.prototype,'name').get;
  const nativeRemoveAttribute = g.Element.prototype.removeAttribute, datasetFactory=ns.makeNamedObject;
  const datasetReplace=String.prototype.replace, datasetSlice=String.prototype.slice, datasetLower=String.prototype.toLowerCase, datasetUpper=String.prototype.toUpperCase;
  const datasetTest=RegExp.prototype.test;
  const propertyToAttribute=name=>'data-'+apply(datasetReplace,name,[/[A-Z]/g,character=>'-'+apply(datasetLower,character,[])]);
  const attributeToProperty=name=>apply(datasetReplace,apply(datasetSlice,name,[5]),[/-([a-z])/g,(_,character)=>apply(datasetUpper,character,[])]);
  const dataset=function(){
    check(this,[1]);let value=apply(datasetGet,datasetValues,[this]);if(value)return value;const owner=this;
    value=datasetFactory(g.DOMStringMap.prototype,function(operation,key,value){
      if(operation==='keys'||operation==='indices'){
        const keys=[],attrs=apply(nativeAttributes,owner,[]);for(let index=0;index<attrs.length;index++){
          const name=apply(nativeAttrName,attrs[index],[]);if(apply(datasetSlice,name,[0,5])!=='data-'||apply(datasetTest,/[A-Z]/,[name]))continue;
          const property=attributeToProperty(name);if(operation==='keys')keys[keys.length]=property;else if(apply(datasetTest,/^(0|[1-9][0-9]*)$/,[property])&&+property<4294967295)keys[keys.length]=+property;
        }return keys;
      }
      const attribute=propertyToAttribute(key),current=apply(getAttribute,owner,[attribute]);
      if(operation==='get')return current===null?undefined:current;
      if(operation==='has')return current!==null;
      if(operation==='descriptor')return current===null?undefined:{value:current,writable:true,enumerable:true,configurable:true};
      if(operation==='delete'){apply(nativeRemoveAttribute,owner,[attribute]);return true;}
      if(operation==='set'||operation==='define'){
        if(operation==='define'){if('get'in value||'set'in value)return false;value=value.value;}
        if(apply(datasetTest,/-[a-z]/,[key]))throw new g.DOMException('Invalid dataset property name','SyntaxError');
        apply(setAttribute,owner,[attribute,string(value)]);return true;
      }return undefined;
    },true,true);
    apply(datasetSet,datasetValues,[owner,value]);return value;
  };
  for(const name of ['HTMLElement','SVGElement'])if(g[name])property(g[name].prototype,'dataset',dataset);
  const uint = value => (+value) >>> 0;
  const character = receiver => { check(receiver, [3, 4, 7, 8]); return apply(data.get, receiver, []); };
  const range = (value, offset) => { if (offset > value.length) throw new g.DOMException('Character-data offset is out of range', 'IndexSizeError'); };
  // Character-data offsets and native payloads both retain original UTF-16 units.
  method(g.CharacterData.prototype, 'substringData', function (offset, count) { const value = character(this); if (arguments.length < 2) throw new TypeError('substringData requires 2 arguments'); offset = uint(offset); count = uint(count); range(value, offset); return value.slice(offset, offset + count); }, 2);
  method(g.CharacterData.prototype, 'replaceData', function (offset, count, value) { const current = character(this); if (arguments.length < 3) throw new TypeError('replaceData requires 3 arguments'); offset = uint(offset); count = uint(count); value = string(value); range(current, offset); native('replaceCharacterData', this, offset, count, value); }, 3);
  method(g.CharacterData.prototype, 'insertData', function (offset, value) { const current = character(this); if (arguments.length < 2) throw new TypeError('insertData requires 2 arguments'); offset = uint(offset); value = string(value); range(current, offset); native('replaceCharacterData', this, offset, 0, value); }, 2);
  method(g.CharacterData.prototype, 'deleteData', function (offset, count) { const current = character(this); if (arguments.length < 2) throw new TypeError('deleteData requires 2 arguments'); offset = uint(offset); count = uint(count); range(current, offset); native('replaceCharacterData', this, offset, count, ''); }, 2);
  method(g.CharacterData.prototype, 'appendData', function (value) { const current = character(this); if (!arguments.length) throw new TypeError('appendData requires 1 argument'); native('replaceCharacterData', this, current.length, 0, string(value)); }, 1);
  for (const name of ['xmlEncoding', 'rootElement']) property(g.Document.prototype, name, function () { check(this, [9]); return null; });
  property(g.Document.prototype, 'xmlStandalone', function () { check(this, [9]); return !!state(this).xmlStandalone; }, function (value) { check(this, [9]); state(this).xmlStandalone = !!value; });
  property(g.Document.prototype, 'xmlVersion', function () { check(this, [9]); return state(this).xmlVersion || null; }, function (value) { check(this, [9]); value = string(value); if (value !== '1.0') throw new g.DOMException('Only XML version 1.0 is supported', 'NotSupportedError'); state(this).xmlVersion = value; });
  const collectionDefine=Object.defineProperty, collectionApply=Reflect.apply;
  const collectionMapGet=Map.prototype.get, collectionMapHas=Map.prototype.has, collectionMapSet=Map.prototype.set;
  const collectionValues=Array.prototype.values, collectionEntries=Array.prototype.entries, collectionKeys=Array.prototype.keys, collectionForEach=Array.prototype.forEach;
  const collectionOwn=(rows,index,value)=>collectionDefine(rows,index,{__proto__:null,value,writable:true,configurable:true,enumerable:true});
  const copyRows=(rows,match)=>{const result=[];for(let index=0;index<rows.length;index++){const value=rows[index];if(!match||match(value))collectionOwn(result,result.length,value);}return result;};
  function descendants(receiver, match) {
    const result = [];
    function visit(node) {
      const children = rawChildren.call(node);
      for (let i = 0; i < children.length; i++) {
        const child = children[i];
        if (type.call(child) === 1 && match(child)) collectionOwn(result,result.length,child);
        visit(child);
      }
    }
    visit(receiver);
    return result;
  }

  let formIteratorRead = null;
  function collectionInterface(name) {
    const ctor = make(function () { throw new TypeError('Illegal constructor'); }, name, 0, true);
    const proto = ctor.prototype;
    Object.defineProperty(proto, Symbol.toStringTag, { value: name, configurable: true });
    Object.defineProperty(ctor, 'prototype', { writable: false });
    Object.defineProperty(g, name, { value: ctor, writable: true, configurable: true });
    const record = receiver => { const value = getRecord(receiver); if (!value || !(value.kind===name || name==='HTMLCollection'&&(value.kind==='HTMLFormControlsCollection'||value.kind==='HTMLOptionsCollection') || name==='NodeList'&&value.kind==='RadioNodeList')) throw new TypeError('Illegal invocation'); return value.read; };
    property(proto, 'length', function () { return record(this)().length; });
    method(proto, 'item', function (index) { if (!arguments.length) throw new TypeError('item requires 1 argument'); return record(this)()[(+index) >>> 0] || null; }, 1);
    Object.defineProperty(proto, Symbol.iterator, { value: make(function () { if(name==='HTMLCollection'&&formIteratorRead){const target=formIteratorRead(this);if(target)return collectionApply(collectionValues,target,[]);}record(this); return collectionApply(collectionValues,this,[]); }, 'values', 0), writable: true, configurable: true });
    return { proto, record };
  }
  const collection = collectionInterface('HTMLCollection');
  const nodeList = collectionInterface('NodeList');
  // An id match anywhere in the collection wins over an earlier name match.
  const named = (list,name)=>{if(name==='')return null;for(let index=0;index<list.length;index++){const node=list[index];if(getAttribute.call(node,'id')===name)return node;}for(let index=0;index<list.length;index++){const node=list[index];if(namespace.call(node)===html&&getAttribute.call(node,'name')===name)return node;}return null;};
  method(collection.proto, 'namedItem', function (name) { if (!arguments.length) throw new TypeError('namedItem requires 1 argument'); return named(collection.record(this)(), string(name)); }, 1);
  for (const [name,implementation] of [['entries',collectionEntries],['keys',collectionKeys],['values',collectionValues]]) method(nodeList.proto,name,function(){nodeList.record(this);return collectionApply(implementation,this,[]);},0);
  Object.defineProperty(nodeList.proto, Symbol.iterator, {value: nodeList.proto.values, writable: true, configurable: true});
  method(nodeList.proto, 'forEach', function (callback, thisArg) { nodeList.record(this); if (typeof callback !== 'function') throw new TypeError('callback must be a function'); collectionApply(collectionForEach,this,[callback,thisArg]); }, 1);
  function live(list, namedProperties, options) {
    const proto = options ? options.prototype : namedProperties ? collection.proto : nodeList.proto;
    const namedLookup = options && options.named || named;
    const object = ns.makeNamedObject(proto, function (operation, key, value) {
      const indexed = /^(0|[1-9][0-9]*)$/.test(key) && Number(key) < 4294967295;
      // An indexed setter (HTMLOptionsCollection) owns every index, supported or not.
      if(indexed&&options&&options.setIndex&&(operation==='set'||operation==='define'))return options.setIndex(Number(key),value,operation==='define');
      const rows = list();
      if(indexed&&options&&options.readonlyIndices&&(operation==='set'||operation==='define'))return true;
      const hiddenNames=!!(options&&options.hiddenNames);
      const supported = indexed ? Number(key) < rows.length || hiddenNames&&operation==='descriptor'&&namedLookup(rows,key)!==null : namedProperties && (hiddenNames||!(key in proto)) && namedLookup(rows, key) !== null;
      if (operation === 'indices') {const indices=[];for(let index=0;index<rows.length;index++)collectionOwn(indices,index,index);return indices;}
      if (operation === 'keys') {
        const names = [];
        if (namedProperties) for (let index=0;index<rows.length;index++) {
          const node=rows[index],values=[getAttribute.call(node,'id'),namespace.call(node)===html?getAttribute.call(node,'name'):null];
          for(let at=0;at<values.length;at++){
            const value=values[at];let duplicate=false;
            for(let existing=0;existing<names.length;existing++)if(names[existing]===value){duplicate=true;break;}
            if(value&&(hiddenNames||!/^(0|[1-9][0-9]*)$/.test(value)&&!(value in proto))&&!duplicate)collectionOwn(names,names.length,value);
          }
        }
        return names;
      }
      if (!supported) return undefined;
      const found = indexed && Number(key)<rows.length ? rows[Number(key)] : namedLookup(rows, key);
      if (operation === 'get') return found;
      if (operation === 'has') return true;
      if (operation === 'descriptor') return { value: found, writable: indexed && !!(options&&options.setIndex), enumerable: indexed || !!(options&&options.namedEnumerable), configurable: true };
      if (operation === 'delete') return false;
      if (operation === 'set' || operation === 'define') return true;
      return undefined;
    }, true);
    setRecord(object,{kind:options?options.kind:namedProperties?'HTMLCollection':'NodeList',read:list});
    return object;
  }
  ns.internalsLiveCollection = live;
  ns.formCollectionFactory={live,getRecord,setIterator:read=>{formIteratorRead=read;}};
  const caches = new WeakMap(), cacheGet=WeakMap.prototype.get.bind(caches),cacheSet=WeakMap.prototype.set.bind(caches);
  function cached(receiver, key, read, namedProperties = true) {
    let cache = cacheGet(receiver);
    if (!cache) { cache = new Map(); cacheSet(receiver, cache); }
    if (!collectionApply(collectionMapHas,cache,[key])) {
      let stamp = null, rows = null;
      const current = () => { const next = native('stamp', receiver); if(next !== stamp) { rows = read(); stamp = next; } return rows; };
      collectionApply(collectionMapSet,cache,[key,live(current,namedProperties)]);
    }
    return collectionApply(collectionMapGet,cache,[key]);
  }
  property(g.Node.prototype, 'childNodes', function () {
    check(this, [1, 2, 3, 4, 7, 8, 9, 10, 11]);
    const receiver = this;
    return cached(receiver, 'childNodes', () => copyRows(rawChildren.call(receiver)), false);
  });
  for (const owner of ['Element', 'Document', 'DocumentFragment']) property(g[owner].prototype, 'children', function () {
    check(this, owner === 'Element' ? [1] : owner === 'Document' ? [9] : [11]);
    const receiver = this;
    return cached(receiver, 'children', () => copyRows(rawChildren.call(receiver),node=>type.call(node)===1));
  });
  for (const owner of ['Document', 'Element', 'DocumentFragment']) {
    const proto = g[owner].prototype;
    const query = proto.querySelectorAll;
    method(proto, 'querySelectorAll', function (selector) {
      check(this, owner === 'Document' ? [9] : owner === 'Element' ? [1] : [11]);
      if (!arguments.length) throw new TypeError('querySelectorAll requires 1 argument');
      const rows = copyRows(query.call(this,string(selector)));
      return live(() => rows, false);
    }, 1);
  }
  const fragmentIdError = TypeError;
  method(g.DocumentFragment.prototype, 'getElementById', function (elementId) {
    if (!arguments.length) throw new fragmentIdError('getElementById requires 1 argument');
    return native('fragmentById', this, elementId);
  }, 1);
  const selectors = {
    forms: node => tag.call(node) === 'FORM' && namespace.call(node) === html,
    images: node => tag.call(node) === 'IMG' && namespace.call(node) === html,
    links: node => ['A', 'AREA'].includes(tag.call(node)) && namespace.call(node) === html && hasAttribute.call(node, 'href'),
    anchors: node => tag.call(node) === 'A' && namespace.call(node) === html && hasAttribute.call(node, 'name'),
    embeds: node => tag.call(node) === 'EMBED' && namespace.call(node) === html,
    scripts: node => tag.call(node) === 'SCRIPT' && namespace.call(node) === html,
    // Applet support was removed from HTML; Chrome's legacy collection stays empty.
    applets: () => false,
  };
  for (const [name, match] of Object.entries(selectors)) property(g.Document.prototype, name, function () {
    check(this, [9]); const receiver = this;
    return cached(receiver, name, () => descendants(receiver, match));
  });
  // plugins is its own accessor returning the very collection embeds returns, as in Chrome.
  property(g.Document.prototype, 'plugins', function () {
    check(this, [9]); const receiver = this;
    return cached(receiver, 'embeds', () => descendants(receiver, selectors.embeds));
  });
  for (const owner of ['Document', 'Element']) {
    const valid = owner === 'Document' ? [9] : [1];
    method(g[owner].prototype, 'getElementsByTagName', function (name) {
      check(this, valid); if (!arguments.length) throw new TypeError('getElementsByTagName requires 1 argument');
      name = string(name); const folded = name.replace(/[a-z]/g, value=>value.toUpperCase()), receiver = this;
      return cached(receiver, 'tag:' + name, () => descendants(receiver, node => name === '*' || tag.call(node) === (namespace.call(node) === html ? folded : name)));
    }, 1);
    method(g[owner].prototype, 'getElementsByTagNameNS', function (uri, local) {
      check(this, valid); if (arguments.length < 2) throw new TypeError('getElementsByTagNameNS requires 2 arguments');
      uri = uri == null || uri === '' ? null : string(uri); local = string(local); const receiver = this;
      return cached(receiver, JSON.stringify(['ns', uri, local]), () => descendants(receiver, node => (uri === '*' || namespace.call(node) === uri) && (local === '*' || node.localName === local)));
    }, 2);
    method(g[owner].prototype, 'getElementsByClassName', function (classes) {
      check(this, valid); if (!arguments.length) throw new TypeError('getElementsByClassName requires 1 argument');
      classes = string(classes); const receiver=this;
      const tokens = classes.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g,'').split(/[\t\n\f\r ]+/).filter(Boolean);
      return cached(receiver, 'class:' + classes, () => {
        const document=type.call(receiver)===9?receiver:receiver.ownerDocument, insensitive=document&&native('compatMode',document)==='BackCompat';
        const fold=value=>insensitive?value.replace(/[A-Z]/g,value=>value.toLowerCase()):value;
        const wanted=tokens.map(fold);
        return wanted.length ? descendants(receiver,node=>{const actual=(getAttribute.call(node,'class')||'').split(/[\t\n\f\r ]+/).map(fold);return wanted.every(value=>actual.includes(value));}):[];
      });
    }, 1);
  }
  method(g.Document.prototype, 'getElementsByName', function (name) {
    check(this, [9]); if (!arguments.length) throw new TypeError('getElementsByName requires 1 argument');
    name = string(name); const receiver = this;
    return cached(receiver, 'name:' + name, () => descendants(receiver, node => namespace.call(node) === html && getAttribute.call(node, 'name') === name), false);
  }, 1);
  for (const name of ['createElement','createAttribute','createAttributeNS']) {
    const descriptor=Object.getOwnPropertyDescriptor(g.document,name);
    if(descriptor){Object.defineProperty(g.Document.prototype,name,{...descriptor,enumerable:true});delete g.document[name];}
  }
  const createElement=g.Document.prototype.createElement;
  method(g.Document.prototype,'createElement',function(name){
    check(this,[9]);if(!arguments.length)throw new TypeError('createElement requires 1 argument');
    name=string(name);if(!xmlName.test(name))throw new g.DOMException('Invalid element name','InvalidCharacterError');
    return apply(createElement,this,[name,arguments[1]]);
  },1);
  method(g.Document.prototype,'createNSResolver',function(node){
    check(this,[9]);if(!arguments.length)throw new TypeError('createNSResolver requires 1 argument');
    type.call(node);return node;
  },1);
  for (const [owner, localName] of [['HTMLScriptElement','script'],['HTMLLinkElement','link']]) {
    const branded = receiver => { check(receiver,[1]); if(namespace.call(receiver)!==html||receiver.localName!==localName)throw new TypeError('Illegal invocation'); };
    property(g[owner].prototype,'integrity',function(){branded(this);return getAttribute.call(this,'integrity')||'';},function(value){branded(this);setAttribute.call(this,'integrity',string(value));});
  }
  property(g.Document.prototype, 'documentElement', function () { check(this, [9]); return Array.from(rawChildren.call(this)).find(node => type.call(node) === 1) || null; });
  property(g.Document.prototype, 'head', function () { check(this, [9]); const root=this.documentElement; return root ? Array.from(rawChildren.call(root)).find(node => type.call(node) === 1 && node.localName === 'head' && namespace.call(node) === html) || null : null; });
  property(g.Document.prototype, 'body', function () { check(this, [9]); const root=this.documentElement; return root ? Array.from(rawChildren.call(root)).find(node => type.call(node) === 1 && ['body','frameset'].includes(node.localName) && namespace.call(node) === html) || null : null; }, function (value) {
    check(this,[9]); if (!value || type.call(value)!==1 || !['body','frameset'].includes(value.localName) || namespace.call(value)!==html) throw new g.DOMException('The new body is not a body or frameset element','HierarchyRequestError');
    const current=this.body;if(value===current)return;if(current)current.parentNode.replaceChild(value,current);else if(this.documentElement)this.documentElement.appendChild(value);else throw new g.DOMException('The document has no document element','HierarchyRequestError');
  });
  for (const name of ['documentElement','head','body']) if(Object.hasOwn(g.document,name))delete g.document[name];
  property(g.Document.prototype, 'title', function () { check(this,[9]); const title=descendants(this,node=>namespace.call(node)===html&&node.localName==='title')[0];return title?(title.textContent||'').replace(/[\t\n\f\r ]+/g,' ').trim():''; },function(value){check(this,[9]);value=string(value);let title=descendants(this,node=>namespace.call(node)===html&&node.localName==='title')[0];if(!title){if(!this.head)return;title=this.createElement('title');this.head.append(title);}title.textContent=value;});
  property(g.Document.prototype, 'compatMode', function () { return native('compatMode', this); });
  property(g.Document.prototype, 'doctype', function () {
    const node = native('doctype', this);
    if (node && Object.prototype.toString.call(node) !== '[object DocumentType]') Object.setPrototypeOf(node, documentTypeProto);
    return node;
  });
  const documentType = make(function () { throw new TypeError('Illegal constructor'); }, 'DocumentType', 0, true);
  const documentTypeProto = documentType.prototype;
  Object.setPrototypeOf(documentTypeProto, g.Node.prototype);
  Object.setPrototypeOf(documentType, g.Node);
  Object.defineProperty(documentTypeProto, Symbol.toStringTag, { value: 'DocumentType', configurable: true });
  Object.defineProperty(documentType, 'prototype', { writable: false });
  Object.defineProperty(g, 'DocumentType', { value: documentType, writable: true, configurable: true });
  for (const [index, name] of ['name', 'publicId', 'systemId'].entries()) property(documentTypeProto, name, function () { return native('doctypeFields', this)[index]; });
  for (const name of ['before', 'after', 'remove', 'replaceWith']) Object.defineProperty(documentTypeProto, name, Object.getOwnPropertyDescriptor(g.Element.prototype, name));
  if(Object.hasOwn(g.document,'title'))delete g.document.title;
  property(g.Document.prototype, 'designMode', function () { return state(this).designMode || 'off'; }, function (value) {
    const record = state(this); value = string(value).toLowerCase(); if (value === 'on' || value === 'off') record.designMode = value;
  });
  property(g.Document.prototype, 'dir', function () { check(this, [9]); return this.documentElement ? this.documentElement.dir : ''; }, function (value) { check(this, [9]); if (this.documentElement) this.documentElement.dir = string(value); });
  for (const [name, attribute] of Object.entries({ bgColor: 'bgcolor', fgColor: 'text', linkColor: 'link', vlinkColor: 'vlink', alinkColor: 'alink' })) property(g.Document.prototype, name, function () {
    check(this, [9]); return this.body ? getAttribute.call(this.body, attribute) || '' : '';
  }, function (value) { check(this, [9]); if (this.body) setAttribute.call(this.body, attribute, string(value)); });
  property(g.HTMLElement.prototype, 'nonce', function () { check(this, [1]); if (namespace.call(this) !== html) throw new TypeError('Illegal invocation'); return native('nonce', this); }, function (value) { check(this, [1]); if (namespace.call(this) !== html) throw new TypeError('Illegal invocation'); native('setNonce', this, string(value)); });
  property(g.HTMLElement.prototype, 'tabIndex', function () {
    check(this, [1]); if (namespace.call(this) !== html) throw new TypeError('Illegal invocation');
    const fallback = ['A', 'INPUT', 'BUTTON', 'SELECT', 'TEXTAREA', 'IFRAME', 'OBJECT', 'AUDIO', 'VIDEO'].includes(tag.call(this)) ? 0 : -1;
    const value = getAttribute.call(this, 'tabindex'); const found = value == null ? null : /^[\t\n\f\r ]*([+-]?[0-9]+)/.exec(value);
    if (!found) return fallback; const number = Number(found[1]); return number >= -2147483648 && number <= 2147483647 ? number : fallback;
  }, function (value) { check(this, [1]); if (namespace.call(this) !== html) throw new TypeError('Illegal invocation'); setAttribute.call(this, 'tabindex', String((+value) >> 0)); });
  method(g.Document.prototype, 'importNode', function(node, deep=false) {
    check(this,[9]); if(!arguments.length) throw new TypeError('importNode requires 1 argument');
    apply(type,node,[]); const result=native('import',this,node,!!deep);
    if(typeof result==='string')throw new g.DOMException(result,'NotSupportedError'); return result;
  },1);
  method(g.Document.prototype, 'adoptNode', function (node) {
    check(this, [9]); if (!arguments.length) throw new TypeError('adoptNode requires 1 argument');
    const result = native('adopt', this, node); if (typeof result === 'string') throw new g.DOMException(result, 'NotSupportedError'); return result;
  }, 1);
  for (const owner of ['Document', 'Element', 'DocumentFragment']) {
    const allowed = owner === 'Document' ? [9] : owner === 'Element' ? [1] : [11];
    method(g[owner].prototype, 'moveBefore', function (node, child) {
      check(this, allowed);
      if (arguments.length < 2) throw new TypeError('moveBefore requires 2 arguments');
      const error = native('move', this, node, child);
      if (typeof error === 'string') throw new g.DOMException('The node cannot be moved into this parent', error);
    }, 2);
  }
  const scoped = {
    Document: ['prepend', 'append', 'replaceChildren', 'moveBefore'],
    DocumentFragment: ['prepend', 'append', 'replaceChildren', 'moveBefore'],
    Element: ['prepend', 'append', 'replaceChildren', 'moveBefore', 'before', 'after', 'replaceWith', 'remove'],
    CharacterData: ['before', 'after', 'replaceWith', 'remove'],
    DocumentType: ['before', 'after', 'replaceWith', 'remove'],
  };
  for (const [owner, names] of Object.entries(scoped)) {
    const blocked = Object.create(null);
    for (const name of names) blocked[name] = true;
    Object.defineProperty(g[owner].prototype, Symbol.unscopables, { value: blocked, configurable: true });
  }
  // Alias the current document state; parsed documents and child realms supply
  // their own visibility/fullscreen state rather than inheriting this document's.
  for (const [alias, name] of Object.entries({webkitHidden: 'hidden', webkitVisibilityState: 'visibilityState', webkitFullscreenElement: 'fullscreenElement', webkitCurrentFullScreenElement: 'fullscreenElement'})) {
    property(g.Document.prototype, alias, function () { check(this, [9]); return this[name]; });
  }
  property(g.Document.prototype, 'webkitIsFullScreen', function () { check(this, [9]); return this.fullscreenElement != null; });
  const project = ns.projectNativeNode, projectSetPrototype = Object.setPrototypeOf;
  ns.projectNativeNode = function (node) { project(node); if (apply(type, node, []) === 10) projectSetPrototype(node, documentTypeProto); return node; };
  const projectedDoctype = g.document.doctype;
  if (projectedDoctype) ns.projectNativeNode(projectedDoctype);
  const handlerNames = 'abort animationcancel animationend animationiteration animationstart auxclick beforecopy beforecut beforeinput beforematch beforepaste beforetoggle beforexrselect blur cancel canplay canplaythrough change click close command contentvisibilityautostatechange contextlost contextmenu contextrestored copy cuechange cut dblclick drag dragend dragenter dragleave dragover dragstart drop durationchange emptied ended error focus formdata freeze fullscreenchange fullscreenerror gotpointercapture input invalid keydown keypress keyup load loadeddata loadedmetadata loadstart lostpointercapture mousedown mouseenter mouseleave mousemove mouseout mouseover mouseup mousewheel paste pause play playing pointercancel pointerdown pointerenter pointerleave pointerlockchange pointerlockerror pointermove pointerout pointerover pointerrawupdate pointerup prerenderingchange progress ratechange readystatechange reset resize resume scroll scrollend scrollsnapchange scrollsnapchanging search securitypolicyviolation seeked seeking select selectionchange selectstart slotchange stalled submit suspend timeupdate toggle transitioncancel transitionend transitionrun transitionstart visibilitychange volumechange waiting webkitanimationend webkitanimationiteration webkitanimationstart webkitfullscreenchange webkitfullscreenerror webkittransitionend wheel'.split(' ');
  for (const event of handlerNames) {
    const name = 'on' + event;
    property(g.Document.prototype, name, function () { const handlers = state(this).handlers; return handlers && handlers.get(name)?.value || null; }, function (value) {
      const record = state(this); const handlers = record.handlers || (record.handlers = new Map());
      value = typeof value === 'function' ? value : null;
      let entry = handlers.get(name);
      if (!entry) { entry = { value: null, callback: make(function (event) { const handler = state(this).handlers.get(name).value; if (handler && handler.call(this, event) === false) event.preventDefault(); }, name, 1) }; handlers.set(name, entry); }
      if (value && !entry.value) this.addEventListener(event, entry.callback);
      if (!value && entry.value) this.removeEventListener(event, entry.callback);
      entry.value = value;
    });
  }
})();

(function () {
  'use strict';
  const g=globalThis,native=g.__domImplementationNative;delete g.__domImplementationNative;
  const make=g[Symbol.for('ts')].makeNativeFunction, define=Object.defineProperty,apply=Reflect.apply,setPrototype=Object.setPrototypeOf,getPrototype=Object.getPrototypeOf;
  const TypeErrorValue=TypeError,StringValue=String,Exception=g.DOMException,regexTest=RegExp.prototype.test,regexExec=RegExp.prototype.exec,stringSlice=String.prototype.slice;
  const string=value=>{if(typeof value==='symbol')throw new TypeErrorValue('Cannot convert a Symbol value to a string');return StringValue(value);};
  const type=Object.getOwnPropertyDescriptor(Node.prototype,'nodeType').get;
  const namespaceXML='http://www.w3.org/XML/1998/namespace',namespaceXMLNS='http://www.w3.org/2000/xmlns/';
  const xmlName=/^[:A-Z_a-z\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02ff\u0370-\u037d\u037f-\u1fff\u200c-\u200d\u2070-\u218f\u2c00-\u2fef\u3001-\udfff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}][:A-Z_a-z\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02ff\u0370-\u037d\u037f-\u1fff\u200c-\u200d\u2070-\u218f\u2c00-\u2fef\u3001-\udfff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}\-.0-9\u00b7\u0300-\u036f\u203f-\u2040]*$/u;
  define(xmlName,'exec',{__proto__:null,value:regexExec});
  function qualified(namespace,name) {
    if(!apply(regexTest,xmlName,[name]))throw new Exception('Invalid element name','InvalidCharacterError');
    let colon=-1;
    for(let index=0;index<name.length;index++)if(name[index]===':'){
      if(colon!==-1)throw new Exception('Invalid qualified name','NamespaceError');colon=index;
    }
    let prefix=null;
    if(colon!==-1){
      prefix=apply(stringSlice,name,[0,colon]);const local=apply(stringSlice,name,[colon+1]);
      if(!prefix||!local||!apply(regexTest,xmlName,[prefix])||!apply(regexTest,xmlName,[local]))throw new Exception('Invalid qualified name','NamespaceError');
    }
    if(prefix&&!namespace||prefix==='xml'&&namespace!==namespaceXML||(name==='xmlns'||prefix==='xmlns')&&namespace!==namespaceXMLNS||namespace===namespaceXMLNS&&name!=='xmlns'&&prefix!=='xmlns')throw new Exception('Invalid namespace for qualified name','NamespaceError');
  }

  const DOMImplementation=make(function(){throw new TypeErrorValue('Illegal constructor');},'DOMImplementation',0,true,true);
  const prototype=DOMImplementation.prototype;
  define(prototype,Symbol.toStringTag,{__proto__:null,value:'DOMImplementation',configurable:true});
  function method(name,length,implementation){define(prototype,name,{__proto__:null,value:make(implementation,name,length),writable:true,enumerable:true,configurable:true});}
  method('createDocumentType',3,function(name,publicId,systemId){
    native('check',this);if(arguments.length<3)throw new TypeErrorValue('createDocumentType requires 3 arguments');
    name=string(name);publicId=string(publicId);systemId=string(systemId);
    for(let index=0;index<name.length;index++){
      const character=name[index];
      if(character==='\0'||character==='>'||character==='\t'||character==='\n'||character==='\f'||character==='\r'||character===' ')throw new Exception('Invalid doctype name','InvalidCharacterError');
    }
    return native('doctype',this,name,publicId,systemId);
  });
  method('createDocument',2,function(namespace,name,doctype){
    native('check',this);if(arguments.length<2)throw new TypeErrorValue('createDocument requires 2 arguments');
    namespace=namespace==null?null:string(namespace);if(namespace==='')namespace=null;
    name=name==null?'':string(name);
    if(doctype!=null&&apply(type,doctype,[])!==10)throw new TypeErrorValue('The value is not a DocumentType');
    if(name)qualified(namespace,name);
    const document=native('xml',this,namespace,name,doctype==null?null:doctype);
    setPrototype(document,XMLDocument.prototype);
    return document;
  });
  method('createHTMLDocument',0,function(title){
    native('check',this);const supplied=title!==undefined;
    return native('html',this,supplied,supplied?string(title):'');
  });
  method('hasFeature',0,function(){native('check',this);return true;});
  delete prototype.constructor;
  define(prototype,'constructor',{__proto__:null,value:DOMImplementation,writable:true,configurable:true});
  define(DOMImplementation,'prototype',{__proto__:null,writable:false});
  define(g,'DOMImplementation',{__proto__:null,value:DOMImplementation,writable:true,configurable:true});
  define(Document.prototype,'implementation',{__proto__:null,get:make(function(){return native('implementation',this,prototype);},'get implementation',0),enumerable:true,configurable:true});
  if(Object.hasOwn(g.document,'implementation'))delete g.document.implementation;
  for(const [name,operation] of [['contentType','contentType'],['URL','url'],['documentURI','url'],['defaultView','view'],['characterSet','encoding'],['charset','encoding'],['inputEncoding','encoding']]) {
    define(Document.prototype,name,{__proto__:null,get:make(function(){return native(operation,this);},'get '+name,0),enumerable:true,configurable:true});
    if(Object.hasOwn(g.document,name))delete g.document[name];
  }
  define(Node.prototype,'baseURI',{__proto__:null,get:make(function(){return native('baseURI',this);},'get baseURI',0),enumerable:true,configurable:true});
  const contextDocument=g.document,documentPrototype=Document.prototype;
  const DocumentConstructor=make(function(){
    if(!new.target)throw new TypeErrorValue("Please use the new operator");
    const implementation=native('implementation',contextDocument,prototype);
    const document=native('xml',implementation,null,'',null),target=new.target.prototype;
    setPrototype(document,target!==null&&(typeof target==='object'||typeof target==='function')?target:documentPrototype);
    return document;
  },'Document',0,true,true);
  DocumentConstructor.prototype=documentPrototype;setPrototype(DocumentConstructor,Node);
  define(documentPrototype,'constructor',{__proto__:null,value:DocumentConstructor,writable:true,configurable:true});
  define(DocumentConstructor,'prototype',{__proto__:null,writable:false});
  define(g,'Document',{__proto__:null,value:DocumentConstructor,writable:true,configurable:true});
  setPrototype(HTMLDocument,DocumentConstructor);
  const createText=Document.prototype.createTextNode,createFragment=Document.prototype.createDocumentFragment;
  function nodeConstructor(name,parent,factory){
    const instancePrototype=g[name].prototype;
    const Constructor=make(function(data){
      if(!new.target)throw new TypeErrorValue("Please use the new operator");
      const node=apply(factory,contextDocument,name==='Text'?[data===undefined?'':string(data)]:[]),target=new.target.prototype;
      setPrototype(node,target!==null&&(typeof target==='object'||typeof target==='function')?target:instancePrototype);
      return node;
    },name,0,true,true);
    Constructor.prototype=instancePrototype;setPrototype(Constructor,parent);
    define(instancePrototype,'constructor',{__proto__:null,value:Constructor,writable:true,configurable:true});
    define(Constructor,'prototype',{__proto__:null,writable:false});
    define(g,name,{__proto__:null,value:Constructor,writable:true,configurable:true,enumerable:false});
    return Constructor;
  }
  nodeConstructor('Text',CharacterData,createText);
  const OldFragment=DocumentFragment,Fragment=nodeConstructor('DocumentFragment',Node,createFragment);
  if(getPrototype(ShadowRoot)===OldFragment)setPrototype(ShadowRoot,Fragment);

  const XMLDocument=make(function(){throw new TypeErrorValue('Illegal constructor');},'XMLDocument',0,true,true);
  setPrototype(XMLDocument,Document);setPrototype(XMLDocument.prototype,Document.prototype);
  define(XMLDocument.prototype,Symbol.toStringTag,{__proto__:null,value:'XMLDocument',configurable:true});
  define(XMLDocument,'prototype',{__proto__:null,writable:false});
  define(g,'XMLDocument',{__proto__:null,value:XMLDocument,writable:true,configurable:true});
  for(const [prototype,name] of [[Node.prototype,'nodeName'],[Element.prototype,'tagName']]) {
    const descriptor=Object.getOwnPropertyDescriptor(prototype,name),original=descriptor.get;
    descriptor.get=make(function(){const qualified=native('qualifiedName',this);return qualified===undefined?apply(original,this,[]):qualified;},'get '+name,0);
    define(prototype,name,descriptor);
  }
})();

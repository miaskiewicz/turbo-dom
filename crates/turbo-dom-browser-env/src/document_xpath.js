// DOM XPath over a private, bounded snapshot of the native Tree. Provider traversal
// stays in V8 and every snapshot read spends an operation budget.
(function () {
  'use strict';
  const g=globalThis.__xpathTarget, provider=globalThis.__xpathProvider, native=globalThis.__xpathNative;
  delete globalThis.__xpathTarget;
  delete globalThis.__xpathProvider;delete globalThis.__xpathNative;
  const make=g[Symbol.for('ts')].makeNativeFunction, NativeException=g.DOMException, NativeTypeError=g.TypeError;
  const getType=Object.getOwnPropertyDescriptor(g.Node.prototype,'nodeType').get;
  const StringValue=String, ProxyValue=Proxy, create=Object.create, define=Object.defineProperty;
  const html='http://www.w3.org/1999/xhtml', apply=Reflect.apply;
  const nodeType=receiver=>apply(getType,receiver,[]);
  const failure=(name,message)=>new NativeException(message,name);
  const text=value=>{if(typeof value==='symbol')throw new NativeTypeError('Cannot convert a Symbol to a string');return StringValue(value);};
  const document=receiver=>{if(nodeType(receiver)!==9)throw new NativeTypeError('Illegal invocation');};
  const readExpression=receiver=>native('expressionGet',receiver),writeExpression=(receiver,record)=>native('expressionSet',receiver,record),readResult=receiver=>native('resultGet',receiver),writeResult=(receiver,record)=>native('resultSet',receiver,record);
  const unsigned=(value,bits)=>{try{return bits===16?((+value)&65535):((+value)>>>0);}catch(error){if(error instanceof TypeError)throw new NativeTypeError(error.message);throw error;}};
  const method=(proto,name,fn,length)=>define(proto,name,{value:make(fn,name,length),writable:true,configurable:true,enumerable:true});
  const property=(proto,name,fn)=>define(proto,name,{get:make(fn,'get '+name,0),configurable:true,enumerable:true});
  function iface(name){
    const ctor=make(function(){throw new NativeTypeError('Illegal constructor');},name,0,true,true);
    define(ctor.prototype,Symbol.toStringTag,{value:name,configurable:true});
    define(ctor,'prototype',{writable:false});define(g,name,{value:ctor,writable:true,configurable:true});
    return ctor;
  }
  const Expression=iface('XPathExpression'),Result=iface('XPathResult');
  const types=['ANY_TYPE','NUMBER_TYPE','STRING_TYPE','BOOLEAN_TYPE','UNORDERED_NODE_ITERATOR_TYPE','ORDERED_NODE_ITERATOR_TYPE','UNORDERED_NODE_SNAPSHOT_TYPE','ORDERED_NODE_SNAPSHOT_TYPE','ANY_UNORDERED_NODE_TYPE','FIRST_ORDERED_NODE_TYPE'];
  types.forEach((name,index)=>{for(const target of [Result,Result.prototype])define(target,name,{value:index,enumerable:true,configurable:true});});
  const functions=new Set('last position count local-name namespace-uri name string concat starts-with contains substring-before substring-after substring string-length normalize-space translate boolean not true false lang number sum floor ceiling round id node text comment processing-instruction'.split(' '));
  function compile(source,resolver,interfaces){
    source=text(source);
    if(source.length>4096)throw failure('NotSupportedError','XPath expression exceeds its UTF-16 length budget');
    const masked=source.replace(/'[^']*'|"[^"]*"/g,"''");
    const tokens=masked.match(/[A-Za-z_][\w.-]*|[^\s]/g)||[];
    if(tokens.length>256)throw failure('NotSupportedError','XPath expression exceeds its token budget');
    let depth=0;for(const token of tokens){if(token==='('||token==='['){if(++depth>32)throw failure('NotSupportedError','XPath expression exceeds its nesting budget');}else if(token===')'||token===']')depth--;}
    if(/\bnamespace\s*::/.test(masked))throw failure('NotSupportedError','XPath namespace-axis nodes require a native namespace-node representation');
    if(masked.includes('$'))throw failure('SyntaxError','DOM XPath expressions do not support variables');
    for(const found of masked.matchAll(/([A-Za-z_][\w.-]*(?::[A-Za-z_][\w.-]*)?)\s*\(/g))if(!functions.has(found[1]))throw failure('SyntaxError','Unknown XPath function');
    let callback=null;
    if(resolver!=null){if(typeof resolver==='function')callback=resolver;else{const lookup=resolver.lookupNamespaceURI;if(typeof lookup!=='function')throw new NativeTypeError('XPath resolver must provide lookupNamespaceURI');callback=prefix=>apply(lookup,resolver,[prefix]);}}
    const namespaces=new Map([['xml','http://www.w3.org/XML/1998/namespace']]);
    for(const found of masked.matchAll(/([A-Za-z_][\w.-]*)\s*:(?!:)\s*(?:[A-Za-z_*])/g)){
      const prefix=found[1];if(namespaces.has(prefix))continue;
      const value=callback&&callback(prefix);if(value==null)throw failure('NamespaceError','Unresolved XPath namespace prefix');
      namespaces.set(prefix,text(value));
    }
    let expression;try{expression=new provider.XPathParser().parse(source);}catch(error){throw failure('SyntaxError','Invalid XPath expression');}
    return {expression,namespaces,provider,interfaces};
  }
  // HTML name tests match its namespace; foreign names and namespace URIs remain case-sensitive.
  const namespaceMatches=provider.NodeTest.nameSpaceMatches,localMatches=provider.NodeTest.localNameMatches;
  provider.NodeTest.nameSpaceMatches=function(prefix,context,node){if(!prefix)return !node.namespaceURI||node.nodeType===1&&node.namespaceURI===html;return namespaceMatches(prefix,context,node);};
  provider.NodeTest.localNameMatches=function(name,context,node){if(node.nodeType===1&&node.namespaceURI===html)return name.toLowerCase()===node.localName.toLowerCase();return localMatches(name,context,node);};
  const applyStep=provider.PathExpr.applyStep;
  provider.PathExpr.applyStep=function(step,context,node){
    if(step.axis!==provider.Step.FOLLOWING&&step.axis!==provider.Step.PRECEDING)return applyStep(step,context,node);
    context.contextNode=node;
    const attribute=node.nodeType===2,anchor=attribute?node.ownerElement:node;
    let root=anchor;while(root.parentNode)root=root.parentNode;
    const ancestors=new Set();for(let current=anchor.parentNode;current;current=current.parentNode)ancestors.add(current);
    const nodes=[],stack=[root];let after=false;
    while(stack.length){
      const current=stack.pop();
      if(current===anchor){after=true;if(!attribute&&step.axis===provider.Step.FOLLOWING)continue;}
      else if(step.axis===provider.Step.FOLLOWING?after:!after&&!ancestors.has(current))if(step.nodeTest.matches(current,context))nodes.push(current);
      if(step.axis===provider.Step.PRECEDING&&after)break;
      for(let child=current.lastChild;child;child=child.previousSibling)stack.push(child);
    }
    return nodes;
  };
  function adapter(data){
    let remaining=200000;
    const spend=()=>{if(--remaining<0)throw failure('NotSupportedError','XPath evaluation exceeds its operation budget');};
    const rows=data.rows,children=rows.map(()=>[]),proxies=[];
    rows.forEach((row,index)=>{if(row[6]!=null)children[row[6]].push(index);});
    const docIndex=rows.findIndex(row=>row[0]===9);
    const parent=index=>rows[index][0]===2?rows[index][8]:rows[index][6];
    const ancestor=(a,b)=>{let current=parent(b);while(current!=null){spend();if(current===a)return true;current=parent(current);}return false;};
    const descendants=index=>{const values=[],stack=children[index].slice().reverse();while(stack.length){spend();const current=stack.pop();values.push(current);for(let i=children[current].length-1;i>=0;i--)stack.push(children[current][i]);}return values;};
    const list=indices=>new ProxyValue(indices.map(index=>proxies[index]),{get(target,key){spend();return key==='item'?index=>{spend();return target[index]||null;}:Reflect.get(target,key);}});
    const attribute=(index,namespace,name)=>{spend();return rows[index][7].find(id=>{spend();return (namespace===undefined?rows[id][1]===name:(rows[id][3]||null)===(namespace||null)&&rows[id][2]===name);});};
    rows.forEach((row,index)=>{
      const target={};proxies.push(new ProxyValue(target,{get(_target,key){
        spend();
        const siblings=row[6]==null?[]:children[row[6]],position=siblings.indexOf(index);
        switch(key){
          case 'nodeType':return row[0];case 'nodeName':return row[0]===1&&row[3]===html?row[1].toUpperCase():row[1];
          case 'tagName':return row[1];case 'localName':return row[2];case 'namespaceURI':return row[3];case 'prefix':return row[4];case 'nodeValue':case 'value':case 'data':return row[5];
          case 'parentNode':return row[6]==null?null:proxies[row[6]];case 'ownerElement':return row[8]==null?null:proxies[row[8]];
          case 'ownerDocument':return row[0]===9?null:proxies[docIndex];case 'childNodes':return list(children[index]);case 'attributes':return row[0]===1?list(row[7]):null;
          case 'firstChild':return proxies[children[index][0]]||null;case 'lastChild':return proxies[children[index].at(-1)]||null;
          case 'previousSibling':return position>0?proxies[siblings[position-1]]:null;case 'nextSibling':return position>=0?proxies[siblings[position+1]]||null:null;
          case 'documentElement':return proxies[children[index].find(id=>rows[id][0]===1)]||null;
          case 'getAttribute':return name=>{const id=attribute(index,undefined,name);return id==null?null:rows[id][5];};
          case 'getAttributeNS':return (namespace,name)=>{const id=attribute(index,namespace,name);return id==null?null:rows[id][5];};
          case 'getAttributeNode':return name=>{const id=attribute(index,undefined,name);return id==null?null:proxies[id];};
          case 'hasAttribute':return name=>attribute(index,undefined,name)!=null;
          case 'getElementById':return value=>{const id=descendants(index).find(id=>rows[id][0]===1&&rows[id][7].some(attr=>{spend();return rows[attr][1]==='id'&&rows[attr][5]===value;}));return proxies[id]||null;};
          case 'contains':return other=>{const id=proxies.indexOf(other);return id===index||id>=0&&ancestor(index,id);};
          case 'compareDocumentPosition':return other=>{spend();const id=proxies.indexOf(other);if(id===index)return 0;if(id<0)return 1;if(ancestor(index,id))return 20;if(ancestor(id,index))return 10;return id>index?4:2;};
          default:return undefined;
        }
      }}));
    });
    return {node:proxies[data.context],actual:node=>{spend();const index=proxies.indexOf(node);if(index<0)throw failure('NotSupportedError','XPath result has no native Node representation');return data.nodes[index];}};
  }
  function execute(record,node,type){
    const kind=nodeType(node);if(![1,2,3,4,7,8,9].includes(kind))throw failure('NotSupportedError','Unsupported XPath context node');
    type=type===undefined?0:unsigned(type,16);if(type>9)throw new NativeTypeError('Unknown XPath result type');
    const data=native('snapshot',node),view=adapter(data),context=new record.provider.XPathContext();
    context.expressionContextNode=view.node;context.caseInsensitive=false;context.allowAnyNamespaceForNoPrefix=false;
    context.namespaceResolver={getNamespace:prefix=>record.namespaces.get(prefix)??null};
    let value;
    try{value=record.expression.evaluate(context);}catch(error){if(error instanceof NativeException)throw error;throw failure('NotSupportedError','XPath evaluation failed: '+error.message);}
    if(type===0)type=value instanceof record.provider.XNodeSet?4:value instanceof record.provider.XNumber?1:value instanceof record.provider.XBoolean?3:2;
    if(type>=4&&!(value instanceof record.provider.XNodeSet))throw new NativeTypeError('XPath value cannot be converted to a node result');
    const state={type,node,stamp:data.stamp,index:0};
    if(type===1)state.value=value.numberValue();else if(type===2)state.value=value.stringValue();else if(type===3)state.value=value.booleanValue();else state.nodes=value.toArray().map(view.actual);
    const result=create(record.interfaces[1]);writeResult(result,state);return result;
  }
  const state=receiver=>{const value=readResult(receiver);if(!value)throw new NativeTypeError('Illegal invocation');return value;};
  const typed=(receiver,allowed)=>{const value=state(receiver);if(!allowed.includes(value.type))throw new NativeTypeError('Wrong XPath result type');return value;};
  property(Result.prototype,'resultType',function(){return state(this).type;});
  for(const [name,type] of [['numberValue',1],['stringValue',2],['booleanValue',3]])property(Result.prototype,name,function(){return typed(this,[type]).value;});
  property(Result.prototype,'singleNodeValue',function(){return typed(this,[8,9]).nodes[0]||null;});
  property(Result.prototype,'snapshotLength',function(){return typed(this,[6,7]).nodes.length;});
  property(Result.prototype,'invalidIteratorState',function(){const value=state(this);return [4,5].includes(value.type)&&native('stamp',value.node)!==value.stamp;});
  method(Result.prototype,'iterateNext',function(){const value=typed(this,[4,5]);if(native('stamp',value.node)!==value.stamp)throw failure('InvalidStateError','The native document changed after XPath evaluation');return value.nodes[value.index++]||null;},0);
  method(Result.prototype,'snapshotItem',function(index){const value=typed(this,[6,7]);if(!arguments.length)throw new NativeTypeError('snapshotItem requires 1 argument');index=unsigned(index,32);return value.nodes[index]||null;},1);
  method(Expression.prototype,'evaluate',function(node,type){const record=readExpression(this);if(!record)throw new NativeTypeError('Illegal invocation');if(!arguments.length)throw new NativeTypeError('evaluate requires 1 argument');return execute(record,node,type);},1);
  const interfaces=[Expression.prototype,Result.prototype];
  const expression=(source,resolver,prototypes)=>{const value=create(prototypes[0]);writeExpression(value,compile(source,resolver,prototypes));return value;};
  method(g.Document.prototype,'createExpression',function(source,resolver){document(this);if(!arguments.length)throw new NativeTypeError('createExpression requires 1 argument');return expression(source,resolver,native('interfaces',this));},1);
  method(g.Document.prototype,'evaluate',function(source,node,resolver,type){document(this);if(arguments.length<2)throw new NativeTypeError('evaluate requires 2 arguments');nodeType(node);return execute(compile(source,resolver,native('interfaces',this)),node,type);},2);
  const Evaluator=make(function(){native('evaluatorSet',this,{interfaces});},'XPathEvaluator',0,true,true);
  define(Evaluator.prototype,Symbol.toStringTag,{value:'XPathEvaluator',configurable:true});define(Evaluator,'prototype',{writable:false});define(g,'XPathEvaluator',{value:Evaluator,writable:true,configurable:true});
  const evaluator=receiver=>{const record=native('evaluatorGet',receiver);if(!record)throw new NativeTypeError('Illegal invocation');return record;};
  method(Evaluator.prototype,'createExpression',function(source,resolver){const record=evaluator(this);if(!arguments.length)throw new NativeTypeError('createExpression requires 1 argument');return expression(source,resolver,record.interfaces);},1);
  method(Evaluator.prototype,'evaluate',function(source,node,resolver,type){const record=evaluator(this);if(arguments.length<2)throw new NativeTypeError('evaluate requires 2 arguments');nodeType(node);return execute(compile(source,resolver,record.interfaces),node,type);},2);
  method(Evaluator.prototype,'createNSResolver',function(node){evaluator(this);if(!arguments.length)throw new NativeTypeError('createNSResolver requires 1 argument');nodeType(node);return node;},1);
  native('registerInterfaces',g.document,interfaces);
})();

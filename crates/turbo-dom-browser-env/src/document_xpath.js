// DOM XPath over a private, bounded snapshot of the native Tree. Provider traversal
// stays in V8 and every snapshot read spends an operation budget.
(function () {
  'use strict';
  const g=globalThis.__xpathTarget, provider=globalThis.__xpathProvider, native=globalThis.__xpathNative;
  delete globalThis.__xpathTarget;
  delete globalThis.__xpathProvider;delete globalThis.__xpathNative;
  const make=__tsNs.makeNativeFunction, NativeException=__tsNs.DOMException, NativeTypeError=g.TypeError;
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
  // The token-count and nesting budgets over the string-masked source.
  function checkBudgets(masked){
    const tokens=masked.match(/[A-Za-z_][\w.-]*|[^\s]/g)||[];
    if(tokens.length>256)throw failure('NotSupportedError','XPath expression exceeds its token budget');
    let depth=0;for(const token of tokens){if(token==='('||token==='['){if(++depth>32)throw failure('NotSupportedError','XPath expression exceeds its nesting budget');}else if(token===')'||token===']')depth--;}
  }
  // An XPathNSResolver: a function, or an object's lookupNamespaceURI (null when absent).
  function resolverCallback(resolver){
    if(resolver==null)return null;
    if(typeof resolver==='function')return resolver;
    const lookup=resolver.lookupNamespaceURI;if(typeof lookup!=='function')throw new NativeTypeError('XPath resolver must provide lookupNamespaceURI');
    return prefix=>apply(lookup,resolver,[prefix]);
  }
  function resolveNamespaces(masked,callback){
    const namespaces=new Map([['xml','http://www.w3.org/XML/1998/namespace']]);
    for(const found of masked.matchAll(/([A-Za-z_][\w.-]*)\s*:(?!:)\s*(?:[A-Za-z_*])/g)){
      const prefix=found[1];if(namespaces.has(prefix))continue;
      const value=callback&&callback(prefix);if(value==null)throw failure('NamespaceError','Unresolved XPath namespace prefix');
      namespaces.set(prefix,text(value));
    }
    return namespaces;
  }
  function compile(source,resolver,interfaces){
    source=text(source);
    if(source.length>4096)throw failure('NotSupportedError','XPath expression exceeds its UTF-16 length budget');
    const masked=source.replace(/'[^']*'|"[^"]*"/g,"''");
    checkBudgets(masked);
    if(masked.includes('$'))throw failure('SyntaxError','DOM XPath expressions do not support variables');
    for(const found of masked.matchAll(/([A-Za-z_][\w.-]*(?::[A-Za-z_][\w.-]*)?)\s*\(/g))if(!functions.has(found[1]))throw failure('SyntaxError','Unknown XPath function');
    const callback=resolverCallback(resolver);
    const namespaces=resolveNamespaces(masked,callback);
    let expression;try{expression=new provider.XPathParser().parse(source);}catch(error){throw failure('SyntaxError','Invalid XPath expression');}
    return {expression,namespaces,provider,interfaces};
  }
  // HTML name tests match its namespace; foreign names and namespace URIs remain case-sensitive.
  const namespaceMatches=provider.NodeTest.nameSpaceMatches,localMatches=provider.NodeTest.localNameMatches;
  provider.NodeTest.nameSpaceMatches=function(prefix,context,node){if(!prefix)return !node.namespaceURI||node.nodeType===1&&node.namespaceURI===html;return namespaceMatches(prefix,context,node);};
  provider.NodeTest.localNameMatches=function(name,context,node){if(node.nodeType===1&&node.namespaceURI===html)return name.toLowerCase()===node.localName.toLowerCase();return localMatches(name,context,node);};
  const applyStep=provider.PathExpr.applyStep;
  // Like Chrome, `/` is the context's highest parentNode ancestor (a fragment, a shadow root,
  // a detached top node, or an Attr whose parentNode is null), and the namespace axis is empty.
  // node() matches every node, including fragment / shadow roots and the doctype.
  provider.NodeTest.nodeTest.matches=function(){return true;};
  provider.PathExpr.getRoot=function(_xpc,nodes){let root=nodes[0];while(root.parentNode)root=root.parentNode;return root;};
  provider.PathExpr.applyStep=function(step,context,node){
    if(step.axis===provider.Step.NAMESPACE)return [];
    // A name test on self / ancestor-or-self selects elements only (the axis' principal node type), never the Attr context.
    if(step.axis===provider.Step.SELF||step.axis===provider.Step.ANCESTORORSELF){const nodes=applyStep(step,context,node);return step.nodeTest===provider.NodeTest.nodeTest?nodes:nodes.filter(item=>item.nodeType!==2);}
    if(step.axis!==provider.Step.FOLLOWING&&step.axis!==provider.Step.PRECEDING)return applyStep(step,context,node);
    return documentOrderAxis(step,context,node);
  };
  // following / preceding in document order from the context's topmost ancestor.
  function includesInAxis(step,after,ancestors,current){
    if(step.axis===provider.Step.FOLLOWING)return after;
    return !after&&!ancestors.has(current);
  }
  function ancestorsAndRoot(anchor){
    let root=anchor;while(root.parentNode)root=root.parentNode;
    const ancestors=new Set();
    for(let current=anchor.parentNode;current;current=current.parentNode)ancestors.add(current);
    return {root,ancestors};
  }
  function walkAxisStep(step,context,anchor,attribute,stack,after,ancestors,nodes){
    const current=stack.pop();
    if(current===anchor){after[0]=true;if(!attribute&&step.axis===provider.Step.FOLLOWING)return false;}
    else if(includesInAxis(step,after[0],ancestors,current)&&step.nodeTest.matches(current,context))nodes.push(current);
    if(step.axis===provider.Step.PRECEDING&&after[0])return true;
    for(let child=current.lastChild;child;child=child.previousSibling)stack.push(child);
    return false;
  }
  function documentOrderAxis(step,context,node){
    context.contextNode=node;
    const attribute=node.nodeType===2,anchor=attribute?node.ownerElement:node;
    const {root,ancestors}=ancestorsAndRoot(anchor);
    const nodes=[],stack=[root],after=[false];
    while(stack.length){if(walkAxisStep(step,context,anchor,attribute,stack,after,ancestors,nodes))break;}
    return nodes;
  }
  function adapter(data){
    let remaining=200000;
    const spend=()=>{if(--remaining<0)throw failure('NotSupportedError','XPath evaluation exceeds its operation budget');};
    const rows=data.rows,children=rows.map(()=>[]),proxies=[];
    rows.forEach((row,index)=>{if(row[6]!=null)children[row[6]].push(index);});
    const docIndex=rows.findIndex(row=>row[0]===9);
    const parent=index=>rows[index][0]===2?rows[index][8]:rows[index][6];
    // The provider's id() searches ownerDocument: a shadow tree's own root scopes it like Chrome's TreeScope,
    // anything else its owner document (a fragment's or detached subtree's too). The snapshot always
    // carries that document (document_xpath.rs walks tree.root() first), so docIndex is never -1.
    const scope=index=>{let top=index;for(let current=parent(top);current!=null;current=parent(current)){spend();top=current;}return rows[top][9]?top:docIndex;};
    const ancestor=(a,b)=>{let current=parent(b);while(current!=null){spend();if(current===a)return true;current=parent(current);}return false;};
    const descendants=index=>{const values=[],stack=children[index].slice().reverse();while(stack.length){spend();const current=stack.pop();values.push(current);for(let i=children[current].length-1;i>=0;i--)stack.push(children[current][i]);}return values;};
    const list=indices=>new ProxyValue(indices.map(index=>proxies[index]),{get(target,key){spend();return key==='item'?index=>{spend();return target[index]||null;}:Reflect.get(target,key);}});
    const attribute=(index,namespace,name)=>{spend();return rows[index][7].find(id=>{spend();return (namespace===undefined?rows[id][1]===name:(rows[id][3]||null)===(namespace||null)&&rows[id][2]===name);});};
    function methodMemberOf(rows,index,attribute,descendants,ancestor,proxies,spend){
      return key=>{
        switch(key){
          case 'getAttribute':return name=>{const id=attribute(index,undefined,name);return id==null?null:rows[id][5];};
          case 'getAttributeNS':return (namespace,name)=>{const id=attribute(index,namespace,name);return id==null?null:rows[id][5];};
          case 'getAttributeNode':return name=>{const id=attribute(index,undefined,name);return id==null?null:proxies[id];};
          case 'hasAttribute':return name=>attribute(index,undefined,name)!=null;
          case 'getElementById':return value=>{const id=descendants(index).find(id=>rows[id][0]===1&&rows[id][7].some(attr=>{spend();return rows[attr][1]==='id'&&rows[attr][5]===value;}));return proxies[id]||null;};
          case 'contains':return other=>{const id=proxies.indexOf(other);return id===index||id>=0&&ancestor(index,id);};
          case 'compareDocumentPosition':return other=>{spend();const id=proxies.indexOf(other);if(id===index)return 0;if(id<0)return 1;if(ancestor(index,id))return 20;if(ancestor(id,index))return 10;return id>index?4:2;};
          default:return undefined;
        }
      };
    }
    function siblingMemberOf(children,index,rows,proxies,methodMember){
      const resolvers={__proto__:null,
        firstChild:(siblings,position)=>proxies[children[index][0]]||null,
        lastChild:(siblings,position)=>proxies[children[index].at(-1)]||null,
        previousSibling:(siblings,position)=>position>0?proxies[siblings[position-1]]:null,
        nextSibling:(siblings,position)=>position>=0?proxies[siblings[position+1]]||null:null,
        documentElement:(siblings,position)=>proxies[children[index].find(id=>rows[id][0]===1)]||null,
      };
      return (key,siblings,position)=>{const resolver=resolvers[key];return resolver?resolver(siblings,position):methodMember(key);};
    }
    function treeMemberOf(row,index,scope,children,list,proxies,siblingMember){
      const resolvers={__proto__:null,
        parentNode:()=>row[6]==null?null:proxies[row[6]],
        ownerElement:()=>row[8]==null?null:proxies[row[8]],
        ownerDocument:()=>row[0]===9?null:proxies[scope(index)],
        childNodes:()=>list(children[index]),
        attributes:()=>row[0]===1?list(row[7]):null,
      };
      return (key,siblings,position)=>{const resolver=resolvers[key];return resolver?resolver():siblingMember(key,siblings,position);};
    }
    function fieldMemberOf(row){
      const nodeNameOf=()=>row[0]===1&&row[3]===html?row[1].toUpperCase():row[1];
      const resolvers={__proto__:null,nodeType:()=>row[0],nodeName:nodeNameOf,tagName:()=>row[1],localName:()=>row[2],namespaceURI:()=>row[3],prefix:()=>row[4],nodeValue:()=>row[5],value:()=>row[5],data:()=>row[5]};
      return key=>{const resolver=resolvers[key];return resolver?resolver():undefined;};
    }
    rows.forEach((row,index)=>{
      const methodMember=methodMemberOf(rows,index,attribute,descendants,ancestor,proxies,spend);
      const siblingMember=siblingMemberOf(children,index,rows,proxies,methodMember);
      const treeMember=treeMemberOf(row,index,scope,children,list,proxies,siblingMember);
      const fieldMember=fieldMemberOf(row);
      const target={};proxies.push(new ProxyValue(target,{get(_target,key){
        spend();
        const siblings=row[6]==null?[]:children[row[6]],position=siblings.indexOf(index);
        const field=fieldMember(key);
        return field!==undefined?field:treeMember(key,siblings,position);
      }}));
    });
    return {node:proxies[data.context],actual:node=>{spend();const index=proxies.indexOf(node);if(index<0)throw failure('NotSupportedError','XPath result has no native Node representation');return data.nodes[index];}};
  }
  function inferResultType(record,value){
    if(value instanceof record.provider.XNodeSet)return 4;
    if(value instanceof record.provider.XNumber)return 1;
    if(value instanceof record.provider.XBoolean)return 3;
    return 2;
  }
  function populateResultState(state,value,view){
    if(state.type===1)state.value=value.numberValue();
    else if(state.type===2)state.value=value.stringValue();
    else if(state.type===3)state.value=value.booleanValue();
    else state.nodes=value.toArray().map(view.actual);
  }
  function execute(record,node,type){
    const kind=nodeType(node);if(![1,2,3,4,7,8,9].includes(kind))throw failure('NotSupportedError','Unsupported XPath context node');
    type=type===undefined?0:unsigned(type,16);if(type>9)throw new NativeTypeError('Unknown XPath result type');
    const data=native('snapshot',node),view=adapter(data),context=new record.provider.XPathContext();
    context.expressionContextNode=view.node;context.caseInsensitive=false;context.allowAnyNamespaceForNoPrefix=false;
    context.namespaceResolver={getNamespace:prefix=>record.namespaces.get(prefix)??null};
    let value;
    try{value=record.expression.evaluate(context);}catch(error){if(error instanceof NativeException)throw error;throw failure('NotSupportedError','XPath evaluation failed: '+error.message);}
    if(type===0)type=inferResultType(record,value);
    if(type>=4&&!(value instanceof record.provider.XNodeSet))throw new NativeTypeError('XPath value cannot be converted to a node result');
    const state={type,node,stamp:data.stamp,index:0};
    populateResultState(state,value,view);
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

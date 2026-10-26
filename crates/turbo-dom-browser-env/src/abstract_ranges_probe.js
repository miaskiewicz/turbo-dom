(() => {
  const out = {}, attempt = fn => { try { return fn(); } catch (error) { return error.name; } };
  const node = document.createTextNode('abcdef'), other = document.createTextNode('other');
  document.body.append(node);
  const init = {startContainer:node,startOffset:1,endContainer:node,endOffset:4};
  const make = changes => new StaticRange({...init,...changes});
  const shape = constructor => ({name:constructor.name,length:constructor.length,source:Function.prototype.toString.call(constructor),
    enumerable:Object.getOwnPropertyDescriptor(globalThis,constructor.name).enumerable,
    keys:constructor===Range?undefined:Reflect.ownKeys(constructor.prototype).map(String).sort(),parent:Object.getPrototypeOf(constructor.prototype).constructor.name,
    constructorParent:Object.getPrototypeOf(constructor).name});
  out.shapes = [AbstractRange,NodeRange,Range,StaticRange].map(shape);
  out.getters = ['startContainer','startOffset','endContainer','endOffset','collapsed'].map(name=>{
    let owner=new StaticRange(init);let depth=0;while(owner&&!Object.hasOwn(owner,name)){owner=Object.getPrototypeOf(owner);depth++}const d=owner&&Object.getOwnPropertyDescriptor(owner,name);
    return d?[name,depth,d.enumerable,d.configurable,d.set===undefined,d.get?.name,d.get?.length,attempt(()=>d.get.call({}))]:[name,'absent'];
  });
  out.illegal = [attempt(()=>AbstractRange()),attempt(()=>new AbstractRange()),attempt(()=>StaticRange(init)),attempt(()=>new StaticRange()),attempt(()=>new StaticRange(null)),attempt(()=>new StaticRange({}))];
  const reads=[];const input={};for(const name of ['startContainer','startOffset','endContainer','endOffset'])Object.defineProperty(input,name,{get(){reads.push(name);return init[name]}});
  const fixed = new StaticRange(input), live = document.createRange();live.setStart(node,1);live.setEnd(node,4);
  out.reads=reads;out.initial=[fixed instanceof StaticRange,fixed instanceof AbstractRange,live instanceof AbstractRange,fixed.collapsed,fixed.startContainer===node,fixed.startOffset,fixed.endOffset];
  out.loose = [[100,200],[-1,1],[4,1],[Infinity,NaN],[4294967297,4294967300]].map(([startOffset,endOffset])=>attempt(()=>{const r=make({startOffset,endOffset});return [r.startOffset,r.endOffset,r.collapsed]}));
  out.roots = attempt(()=>{const r=make({endContainer:other});return [r.startContainer===node,r.endContainer===other,r.collapsed]});
  out.invalidNodes=[null,{},document.doctype,document.createAttribute('data-x')].map(value=>attempt(()=>make({startContainer:value})));
  const order=[];out.failure=attempt(()=>new StaticRange({get endContainer(){order.push('endContainer');return node},get endOffset(){order.push('endOffset');return Symbol()},get startContainer(){order.push('startContainer');return node}}));out.failureReads=order;
  node.deleteData(0,3);out.mutation=[fixed.startOffset,fixed.endOffset,live.startOffset,live.endOffset];node.remove();out.detached=[fixed.startContainer===node,fixed.endContainer===node];
  class Sub extends StaticRange {}const sub=new Sub(init);out.subclass=[sub instanceof Sub,sub instanceof StaticRange,sub.startContainer===node];
  const get=Object.getOwnPropertyDescriptor(AbstractRange.prototype,'collapsed').get;
  out.brands=[attempt(()=>get.call(Object.create(StaticRange.prototype))),attempt(()=>get.call(new Proxy(fixed,{}))),attempt(()=>Range.prototype.setStart.call(fixed,node,0))];
  const savedIterator=Array.prototype[Symbol.iterator],savedType=Object.getOwnPropertyDescriptor(Node.prototype,'nodeType');let hooks=0;
  const poison=()=>{hooks++;throw new Error('guest hook')};let poisoned;
  try{Array.prototype[Symbol.iterator]=poison;Object.defineProperty(Node.prototype,'nodeType',{get:poison,configurable:true});poisoned=new StaticRange(init)}finally{Array.prototype[Symbol.iterator]=savedIterator;Object.defineProperty(Node.prototype,'nodeType',savedType)}
  out.poison=[hooks,poisoned.startContainer===node,poisoned.endOffset];
  return out;
})();

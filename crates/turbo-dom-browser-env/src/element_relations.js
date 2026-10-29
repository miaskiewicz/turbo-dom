(function () {
  const g=globalThis,ns=g[Symbol.for('ts')];
  const getRecord=g.__getCollectionRecord,setRecord=g.__setCollectionRecord,make=ns.makeNativeFunction;
  const type=Object.getOwnPropertyDescriptor(g.Node.prototype,'nodeType').get;
  const children=Object.getOwnPropertyDescriptor(g.Node.prototype,'childNodes').get;
  const getAttribute=g.Element.prototype.getAttribute,setAttribute=g.Element.prototype.setAttribute,removeAttribute=g.Element.prototype.removeAttribute;
  const rootOf=g.Node.prototype.getRootNode;
  const shadowHost=Object.getOwnPropertyDescriptor(g.ShadowRoot.prototype,'host').get;
  const freeze=Object.freeze,hasOwn=Object.hasOwn,create=Object.create,define=Object.defineProperty,apply=Reflect.apply;
  // Engine calls go through captured intrinsics, never a page-replaceable builtin (turbo-dom#194).
  const add=(list,value)=>{define(list,list.length,{value,writable:true,enumerable:true,configurable:true});};
  const has=(list,value)=>{for(let index=0;index<list.length;index++)if(list[index]===value)return true;return false;};
  const words=value=>{const out=[];let word='';for(let index=0;index<=value.length;index++){const c=index<value.length?value[index]:' ';if(c===' '||c==='\t'||c==='\n'||c==='\f'||c==='\r'){if(word)add(out,word);word='';}else word+=c;}return out;};
  function element(value){if(apply(type,value,[])!==1)throw new TypeError('An Element is required');return value;}
  function state(owner){element(owner);let shared=getRecord(owner);if(!shared){shared=create(null);shared.kind='Element';setRecord(owner,shared);}if(!hasOwn(shared,'aria'))define(shared,'aria',{value:create(null),writable:true});return shared.aria;}
  function inScope(owner,target){const wanted=apply(rootOf,target,[]);let current=apply(rootOf,owner,[]);while(current){if(current===wanted)return true;let host;try{host=apply(shadowHost,current,[]);}catch(_){return false;}current=host?apply(rootOf,host,[]):null;}return false;}
  function findId(root,id){if(!id)return null;const stack=[root];let depth=1;while(depth){const node=stack[--depth];if(apply(type,node,[])===1&&apply(getAttribute,node,['id'])===id)return node;const list=apply(children,node,[]);for(let index=list.length-1;index>=0;index--)define(stack,depth++,{value:list[index],writable:true,enumerable:true,configurable:true});}return null;}
  function read(owner,attribute,many){const records=state(owner);let entry=records[attribute];const value=apply(getAttribute,owner,[attribute]);
    if(!entry)entry=records[attribute]=create(null);
    const found=[];if(entry.references){const references=entry.references;for(let index=0;index<references.length;index++)if(inScope(owner,references[index]))add(found,references[index]);}else{if(value===null)return null;const root=apply(rootOf,owner,[]),list=many?words(value):[value];for(let index=0;index<list.length;index++){const target=findId(root,list[index]);if(target)add(found,target);}}
    if(!many)return found[0]||null;
    let same=!!entry.cache&&entry.cache.length===found.length;for(let index=0;same&&index<found.length;index++)if(entry.cache[index]!==found[index])same=false;if(!same)entry.cache=freeze(found);
    return entry.cache;
  }
  const properties={ariaActionsElements:'aria-actions',ariaActiveDescendantElement:'aria-activedescendant',ariaControlsElements:'aria-controls',ariaDescribedByElements:'aria-describedby',ariaDetailsElements:'aria-details',ariaErrorMessageElements:'aria-errormessage',ariaFlowToElements:'aria-flowto',ariaLabelledByElements:'aria-labelledby'};
  // Explicit references clear the ID attribute. Native attribute mutation hooks
  // also clear this private assignment, including writes of the same empty value.
  for(const [name,attribute] of Object.entries(properties)){
    const many=name.endsWith('Elements');
    Object.defineProperty(g.Element.prototype,name,{get:make(function(){return read(this,attribute,many);},'get '+name,0),set:make(function(value){if(many&&value!=null&&typeof value!=='object'&&typeof value!=='function')throw new TypeError("Failed to set the '"+name+"' property on 'Element': The provided value cannot be converted to a sequence.");assign(this,attribute,many,value);},'set '+name,1),enumerable:true,configurable:true});
  }
  function assign(owner,attribute,many,value){element(owner);if(value==null){apply(removeAttribute,owner,[attribute]);delete state(owner)[attribute];return;}let references;
    if(many){references=[];for(const target of value){element(target);if(!has(references,target))add(references,target);}}
    else references=[element(value)];
    apply(setAttribute,owner,[attribute,'']);const entry=create(null);entry.references=references;state(owner)[attribute]=entry;
  }
  // Single-element HTML reflection (popovertarget, commandfor) shares the scoped explicit state.
  ns.elementReference={read:(owner,attribute)=>read(owner,attribute,false),assign:(owner,attribute,value)=>assign(owner,attribute,false,value)};
})();

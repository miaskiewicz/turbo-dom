(function () {
  const g=globalThis,ns=g[Symbol.for('ts')];
  const getRecord=g.__getCollectionRecord,setRecord=g.__setCollectionRecord,make=ns.makeNativeFunction;
  const type=Object.getOwnPropertyDescriptor(g.Node.prototype,'nodeType').get;
  const children=Object.getOwnPropertyDescriptor(g.Node.prototype,'childNodes').get;
  const getAttribute=g.Element.prototype.getAttribute,setAttribute=g.Element.prototype.setAttribute,removeAttribute=g.Element.prototype.removeAttribute;
  const rootOf=g.Node.prototype.getRootNode;
  const shadowHost=Object.getOwnPropertyDescriptor(g.ShadowRoot.prototype,'host').get;
  const freeze=Object.freeze,hasOwn=Function.prototype.call.bind(Object.prototype.hasOwnProperty),create=Object.create;
  function element(value){if(type.call(value)!==1)throw new TypeError('An Element is required');return value;}
  function state(owner){element(owner);let shared=getRecord(owner);if(!shared){shared=create(null);shared.kind='Element';setRecord(owner,shared);}if(!hasOwn(shared,'aria'))Object.defineProperty(shared,'aria',{value:create(null),writable:true});return shared.aria;}
  function inScope(owner,target){const wanted=rootOf.call(target);let current=rootOf.call(owner);while(current){if(current===wanted)return true;let host;try{host=shadowHost.call(current);}catch(_){return false;}current=host?rootOf.call(host):null;}return false;}
  function findId(root,id){if(!id)return null;const stack=[root];while(stack.length){const node=stack.pop();if(type.call(node)===1&&getAttribute.call(node,'id')===id)return node;const list=children.call(node);for(let index=list.length-1;index>=0;index--)stack.push(list[index]);}return null;}
  function read(owner,attribute,many){const records=state(owner);let entry=records[attribute];const value=getAttribute.call(owner,attribute);
    if(!entry)entry=records[attribute]=create(null);
    let found;if(entry.references){found=entry.references.filter(target=>inScope(owner,target));}else{if(value===null)return null;const root=rootOf.call(owner);found=(many?value.split(/[\t\n\f\r ]+/).filter(Boolean):[value]).map(id=>findId(root,id)).filter(Boolean);}
    if(!many)return found[0]||null;
    if(!entry.cache||entry.cache.length!==found.length||entry.cache.some((target,index)=>target!==found[index]))entry.cache=freeze(found);
    return entry.cache;
  }
  const properties={ariaActionsElements:'aria-actions',ariaActiveDescendantElement:'aria-activedescendant',ariaControlsElements:'aria-controls',ariaDescribedByElements:'aria-describedby',ariaDetailsElements:'aria-details',ariaErrorMessageElements:'aria-errormessage',ariaFlowToElements:'aria-flowto',ariaLabelledByElements:'aria-labelledby'};
  // Explicit references clear the ID attribute. Native attribute mutation hooks
  // also clear this private assignment, including writes of the same empty value.
  for(const [name,attribute] of Object.entries(properties)){
    const many=name.endsWith('Elements');
    Object.defineProperty(g.Element.prototype,name,{get:make(function(){return read(this,attribute,many);},'get '+name,0),set:make(function(value){assign(this,attribute,many,value);},'set '+name,1),enumerable:true,configurable:true});
  }
  function assign(owner,attribute,many,value){element(owner);if(value==null){removeAttribute.call(owner,attribute);delete state(owner)[attribute];return;}let references;
    if(many){references=[];for(const target of value){element(target);if(!references.includes(target))references.push(target);}}
    else references=[element(value)];
    setAttribute.call(owner,attribute,'');const entry=create(null);entry.references=references;state(owner)[attribute]=entry;
  }
  // Single-element HTML reflection (popovertarget, commandfor) shares the scoped explicit state.
  ns.elementReference={read:(owner,attribute)=>read(owner,attribute,false),assign:(owner,attribute,value)=>assign(owner,attribute,false,value)};
})();

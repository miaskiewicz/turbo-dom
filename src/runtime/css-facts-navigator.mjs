// Private primitive snapshots, not DOM wrappers or a native-capture authority.
// Input reflection can trap; errors propagate. Evaluation uses owned data only.
const descriptors = Object.getOwnPropertyDescriptor;
const own = Object.hasOwn;
const keys = Object.keys;
const freeze = Object.freeze;
const safe = Number.isSafeInteger;
const apply = Reflect.apply;
const define = Object.defineProperty;
const MapClass = Map;
const WeakMapClass = WeakMap, WeakSetClass = WeakSet;
const mapGet = Map.prototype.get, mapSet = Map.prototype.set, mapHas = Map.prototype.has;
const weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set;
const budgetHas = WeakSet.prototype.has, budgetAdd = WeakSet.prototype.add, budgetDelete = WeakSet.prototype.delete;
const mapSize = Object.getOwnPropertyDescriptor(Map.prototype, 'size').get;
const maps = new WeakMapClass();
const budgets = new WeakSetClass(), navigatorLimits = new WeakMapClass();
const budgetNames = ['maxNodes','maxEdges','maxAttributeUnits','maxTextUnits'];
function put(object, key, value) { define(object, key, {value, writable:true, enumerable:true, configurable:true}); }
function lookup(map,key) { return apply(mapGet,map,[key]); }
function contains(array,value) { for(let i=0;i<array.length;i++)if(array[i]===value)return true;return false; }
export class CssFactsUnavailable extends Error {
  constructor(fact, id) { super(`CSS primitive fact unavailable: ${fact}${id === undefined ? '' : ` (${id})`}`); this.name = 'CssFactsUnavailable'; }
}
function missing(name, id) { throw new CssFactsUnavailable(name, id); }
function data(object, name, optional = false) {
  const descriptor = descriptors(object, name);
  if (!descriptor) { if (optional) return undefined; missing(name); }
  if (!own(descriptor, 'value')) missing(`accessor ${name}`);
  return descriptor.value;
}
function identity(value) { if (!safe(value) || value < 0 || value > 0xffffffff) missing('identity'); return value; }
function charge(budget, name, count) { if (!safe(count) || count < 0 || count > budget[name]) missing(`${name} limit`); budget[name] -= count; }
function text(value, budget, name) { if (typeof value !== 'string') missing(name); charge(budget, 'maxTextUnits', value.length); return value; }
function array(value, budget, bound, copy) {
  const length = data(value, 'length');charge(budget,bound,length);
  const result=[];for(let index=0;index<length;index++)put(result,index,copy(data(value,index)));return freeze(result);
}
function nullableId(value) { return value === null ? null : identity(value); }
function attributes(value,budget) {
  return array(value,budget,'maxEdges',(row)=>{
    const result={};
    const names=['name','value','prefix','namespaceURI'];
    for(let index=0;index<names.length;index++) { const name=names[index];
      const value=data(row,name,true);
      if(value===undefined)continue;
      if(value!==null&&typeof value!=='string')missing(`attribute ${name}`);
      if(value!==null)charge(budget,'maxAttributeUnits',value.length);
      put(result,name,value);
    }
    if(typeof result.name!=='string'||typeof result.value!=='string')missing('attribute name/value');
    return freeze(result);
  });
}
function states(value,budget) {
  const result={};
  if(value===undefined)return freeze(result);
  const names=keys(value);
  for(let index=0;index<names.length;index++){
    const name=names[index],entry=data(value,name);
    if(entry!==null&&typeof entry!=='undefined'&&typeof entry!=='boolean'&&typeof entry!=='number'&&typeof entry!=='string')missing(`state ${name}`);
    charge(budget,'maxTextUnits',name.length+(typeof entry==='string'?entry.length:0));
    put(result,name,entry);
  }
  return freeze(result);
}
function node(source,budget) {
  const result={id:identity(data(source,'id')),nodeType:identity(data(source,'nodeType')),parentNode:nullableId(data(source,'parentId')),ownerDocument:nullableId(data(source,'ownerDocumentId'))};
  put(result,'children',array(data(source,'childIds'),budget,'maxEdges',identity));
  const names=['localName','namespaceURI','text'];for(let i=0;i<names.length;i++){const key=names[i];
    const value=data(source,key,true);if(value===undefined)continue;
    put(result,key,value===null?null:text(value,budget,key));
  }
  const sources=['hostId','assignedSlotId'], targets=['host','assignedSlot'];for(let i=0;i<sources.length;i++){const sourceKey=sources[i],key=targets[i];
    const value=data(source,sourceKey,true);if(value!==undefined)put(result,key,nullableId(value));
  }
  const attrs=data(source,'attributes',true);if(attrs!==undefined)put(result,'attributes',attributes(attrs,budget));
  put(result,'state',states(data(source,'state',true),budget));
  return freeze(result);
}
function get(rows,id) { const row=lookup(rows,identity(id));if(!row)missing('unknown node',id);return row; }
function property(rows,id,key){const row=get(rows,id);if(!own(row,key))missing(key,id);return row[key];}
function attribute(rows,id,name){const attrs=property(rows,id,'attributes');for(let at=0;at<attrs.length;at++){const row=attrs[at];if(!own(row,'prefix')||row.prefix===undefined)missing('attribute prefix',id);const qualified=row.prefix?`${row.prefix}:${row.name}`:row.name;if(qualified===name)return row.value;}return null;}
function sibling(rows,id,delta){const row=get(rows,id);if(row.parentNode===null)return null;const children=property(rows,row.parentNode,'children');let index=-1;for(let i=0;i<children.length;i++)if(children[i]===id){index=i;break;}if(index<0)missing('parent/child provenance',id);const at=index+delta;return at<0||at>=children.length?null:children[at];}
function validate(rows,input){for(let i=0;i<input.length;i++){const row=input[i];if(row.parentNode!==null){const parent=get(rows,row.parentNode);if(!contains(parent.children,row.id))missing('parent/child provenance',row.id);}for(let at=0;at<row.children.length;at++){const child=row.children[at];if(get(rows,child).parentNode!==row.id)missing('child/parent provenance',child);}let current=row,count=0;while(current.parentNode!==null){if(++count>apply(mapSize,rows,[]))missing('cyclic parent',row.id);current=get(rows,current.parentNode);}}}
export function captureCssFactsBudget(limits){
 const budget={};for(let i=0;i<budgetNames.length;i++){const name=budgetNames[i],value=data(limits,name);if(!safe(value)||value<0)missing('limit');put(budget,name,value);}
 freeze(budget);apply(budgetAdd,budgets,[budget]);return budget;
}
function budgetEntry(token){if(!apply(budgetHas,budgets,[token]))missing('unadmitted budget');return token;}
export function reuseCssFactsBudget(token,navigator){
 budgetEntry(token);const previous=apply(weakGet,navigatorLimits,[navigator]);
 if(!previous)return false;
 for(let i=0;i<budgetNames.length;i++){const name=budgetNames[i];if(token[name]!==previous[name])return false;}
 apply(budgetDelete,budgets,[token]);return true;
}
export function createCssFactsNavigator(source,limits){return createCssFactsNavigatorFromBudget(source,captureCssFactsBudget(limits));}
export function createCssFactsNavigatorFromBudget(source,budgetToken){
 const entry=budgetEntry(budgetToken),budget={maxNodes:entry.maxNodes,maxEdges:entry.maxEdges,maxAttributeUnits:entry.maxAttributeUnits,maxTextUnits:entry.maxTextUnits};apply(budgetDelete,budgets,[budgetToken]);
 const documentId=identity(data(source,'documentId')),documentElementId=nullableId(data(source,'documentElementId'));const rows=new MapClass();
 const input=array(data(source,'nodes'),budget,'maxNodes',value=>node(value,budget));for(let index=0;index<input.length;index++){const row=input[index];if(apply(mapHas,rows,[row.id]))missing('duplicate node',row.id);apply(mapSet,rows,[row.id,row]);}
 validate(rows,input);get(rows,documentId);if(documentElementId!==null)get(rows,documentElementId);
 const focus=data(source,'focusTarget',true);if(focus!==undefined&&focus!==null)get(rows,focus);
 const token=freeze({});const operations=freeze({
  property:(id,key)=>property(rows,id,key),
  state(id,key){const state=property(rows,id,'state');if(!own(state,key))missing(`state ${key}`,id);return state[key];},
  attribute:(id,name)=>attribute(rows,id,name),
  attrs:id=>property(rows,id,'attributes'),
  children:id=>property(rows,id,'children'),
  previous:id=>sibling(rows,id,-1),next:id=>sibling(rows,id,1),
  documentId,documentElementId,
  focusTarget(){if(focus===undefined)missing('focusTarget');return focus;},
 });apply(weakSet,maps,[token,operations]);apply(weakSet,navigatorLimits,[token,entry]);return token;
}
export function readCssFactsNavigator(token){const operations=apply(weakGet,maps,[token]);if(!operations)missing('unadmitted navigator');return operations;}

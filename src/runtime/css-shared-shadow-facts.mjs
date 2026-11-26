// Context-keyed mechanical answers only. Rust owns rule admission, scope roots,
// proximity and ranking. Snapshot roots/slots are trusted original native facts.
import {compileCssMatchFacts, compileCssPrimitiveMatchFacts, CssMatchFactsLimitError, CssMatchFactsCapabilityError} from './css-match-facts.mjs';
import {cssFocusTarget} from './css-invalidation.mjs';
import {sharedText} from './css-shared-codec.mjs';
import {createSharedRouting} from './css-shared-routing.mjs';
function positive(value) {
  if (!Number.isSafeInteger(value) || value < 0) throw new TypeError('invalid CSS contextual fact limits');
  return value;
}
function collectMetadata(metadata, primitive) {
  const selectors=[], groups=[], items=new Map(), contextual=new Map();
  let next=0;
  function program(item) {
    const value={...item,id:next++,shadow:true,allowScopeReference:true};
    if(item.seed)value.seed={kind:item.seed.kind,value:sharedText(item.seed.valueUnits)};
    return value;
  }
  for(const selector of metadata.selectors){
    if(selector.items===null)continue;
    for(const item of selector.items){const policy=selector.policy?.find(value=>value.index===item.index);const value=program(item);
      if(policy?.target.kind==='slotted'){value.pseudo=policy.target.pseudo;value.seed=null;}
      selectors.push(value);items.set(value.id,{selectorId:selector.id,itemId:item.index});contextual.set(value.id,item);}
  }
  collectGroups(metadata.groups,groups,program);
  return {collector:(primitive ? compileCssPrimitiveMatchFacts : compileCssMatchFacts)({selectors,scopeGroups:groups}),items,contextual,groups};
}
function queries(program,snapshot) {
  const ids=snapshot.records.filter(row=>row.kind==='element').map(row=>row.id);
  const roots=[...ids,...snapshot.roots.map(row=>row.id)];
  const selectorQueries=[],scopeQueries=[];
  for(const [selectorId,item] of program.contextual){
    const scopeRootIds=item.implicitScopeAnchor?roots:item.referencesScope?[null,...roots]:[null];
    selectorQueries.push({selectorId,scopeRootIds});
  }
  for(const group of program.groups)for(const boundary of ['start','end'])if(group[boundary]!==undefined)scopeQueries.push({groupId:group.id,boundary,enclosingRootIds:roots,candidateIds:ids});
  return {ids,selectorQueries,scopeQueries};
}
function slotsOf(snapshot) {
  const records=new Map(snapshot.records.map(row=>[row.id,row])),slots=new Map();
  for(const row of snapshot.records){
    if(row.assignedSlot===null||row.assignedSlot===undefined)continue;
    const slot=records.get(row.assignedSlot);
    if(!slot)throw new TypeError('foreign CSS assigned slot');
    slots.set(row.id,{slot:slot.id,treeScope:slot.treeScope});
  }
  return slots;
}
function addDomains(output,facts,snapshot,remaining) {
  const map=new Map();
  for(const domain of facts.domains){
    const nodeIds=[...domain.nodeIds,...snapshot.roots.map(row=>row.id)];
    remaining.units-=nodeIds.length;
    if(remaining.units<0)throw new CssMatchFactsLimitError('domain units');
    const id=output.domains.length;map.set(domain.id,id);output.domains.push({id,nodeIds});
  }
  return map;
}
function addAnswers(output,facts,program,treeScope,domains) {
  for(const row of facts.selectorAnswers)output.contextAnswers.selectorAnswers.push({...program.items.get(row.selectorId),treeScope,rootId:row.scopeRootId,domainId:domains.get(row.domainId),matches:row.matches});
  for(const row of facts.scopeAnswers)output.contextAnswers.scopeAnswers.push({treeScope,groupId:row.groupId,boundary:row.boundary,rootId:row.enclosingRootId,domainId:domains.get(row.domainId),matches:row.matches});
}
export function compileSharedShadowFacts(metadata, primitive = false) {
  const program=collectMetadata(metadata, primitive);
  return {collect(snapshot,document,limits,routing,navigator){return collect(program,snapshot,document,limits,routing,navigator);}};
}
function collect(program,snapshot,document,limits,originalRouting,navigator) {
  const maximumContexts=positive(limits.maxContexts),remaining={answers:positive(limits.maxAnswers),units:positive(limits.maxDomainUnits)};
  if(snapshot.roots.length>maximumContexts)throw new CssMatchFactsLimitError('contexts');
  const routing=originalRouting??createSharedRouting(snapshot,limits.routing);
  const request=queries(program,snapshot),slots=slotsOf(snapshot);
  const scopeNodes=snapshot.roots.map(row=>({id:row.id,element:row.element}));
  const sheetRootIds=snapshot.matchNodes.filter(row=>row.parent===null).map(row=>row.id);
  const output={domains:[],selectorAnswers:[],scopeAnswers:[],contextAnswers:{selectorAnswers:[],scopeAnswers:[]},contextRoots:routing.contextRoots};
  for(const root of snapshot.roots){
    const facts=program.collector.collect(snapshot.matchNodes,{ownerDocument:navigator === undefined ? document : snapshot.documentId,navigator,scopeNodes,sheetRootIds,targetIds:request.ids,selectorQueries:request.selectorQueries,scopeQueries:request.scopeQueries,liveState:{focusTarget:cssFocusTarget},cssContext:{shadowRoot:root.host===null?null:(navigator === undefined ? root.element : root.id),slots},collectUsage:true},{maxNodes:positive(limits.maxNodes),maxAncestorDepth:positive(limits.maxAncestorDepth),maxAnswers:remaining.answers});
    remaining.answers-=facts.used.answers;
    const domains=addDomains(output,facts,snapshot,remaining);
    addAnswers(output,facts,program,root.id,domains);
  }
  return output;
}

function collectGroups(source,groups,program) {
  for(const group of source){
    if(!group.scope||group.invalidScope)continue;
    const row={id:group.id};
    for(const boundary of ['start','end'])if(group.scope[boundary]!==null){
      const values=group.scope[boundary+'Items'];
      row[boundary]=values===null?null:values.map(item=>{boundaryPolicy(item);return program(item);});
    }
    groups.push(row);
  }
 }

function boundaryPolicy(item) {
  const descriptor=Object.getOwnPropertyDescriptor(item,'policy');
  if(!descriptor||!Object.hasOwn(descriptor,'value'))throw new CssMatchFactsCapabilityError('scope boundary policy unavailable');
  const policy=descriptor.value;
  if(policy!==null&&policy.target?.kind==='unsupported_composed')throw new CssMatchFactsCapabilityError('unsupported composed scope boundary');
  if(item.pseudo!==null)throw new CssMatchFactsCapabilityError('non-element scope boundary unavailable');
  if(policy!==null&&!policy.target)throw new CssMatchFactsCapabilityError('invalid scope boundary policy');
}

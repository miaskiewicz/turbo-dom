// Test-only capability provider: public DOM reads are intentional and source IDs
// come from the independently admitted public snapshot, never a production brand.
import {captureCssPrimitivePublicFacts} from './css-primitive-public-facts.mjs';
import {createCssFactsNavigator} from '../../src/runtime/css-facts-navigator.mjs';
import {cssFocusTarget} from '../../src/runtime/css-invalidation.mjs';
export function configurePrimitiveReal(real) {
real.captureSelectorFacts=(snapshot,context)=>{
 const extras=new WeakMap();let next=Math.max(...snapshot.records.map(row=>row.id))+1;
 const idOf=node=>{if(node.nodeType!==10)return snapshot.nodeId(node);if(!extras.has(node))extras.set(node,next++);return extras.get(node);};
 return createCssFactsNavigator(captureCssPrimitivePublicFacts(context.document,{idOf,focusTarget:cssFocusTarget(context.document)}),{maxNodes:256,maxEdges:16384,maxTextUnits:262144,maxAttributeUnits:65536});
};
real.fixturePolicy={contextualMatching({snapshot}){const n=snapshot.records.length;const css=snapshot.records.reduce((sum,row)=>sum+(row.textUnits?.length??0),0)+1;return{maxContexts:snapshot.roots.length,maxNodes:n,maxAncestorDepth:64,maxAnswers:css*(n+1)**2,maxDomainUnits:css*(n+1)**2,routing:{maxRoots:snapshot.roots.length,maxNodes:n,maxPathUnits:snapshot.used.maxPathUnits*2+n*n,maxScopeDepth:64}};}};
return real;
}

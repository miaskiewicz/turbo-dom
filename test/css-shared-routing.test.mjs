import assert from 'node:assert/strict';
import test from 'node:test';
import {createSharedRouting,SharedRoutingError} from '../src/runtime/css-shared-routing.mjs';
import {createEnvironment} from '../src/runtime/index.mjs';
import {createTestPublicCssFacts} from './helpers/css-shared-public-facts.mjs';
const captureLimits={maxNodes:128,maxDepth:32,maxPathUnits:4096,maxTextUnits:4096,maxAttributes:128,maxAttributeUnits:4096};
const routingLimits={maxRoots:8,maxNodes:128,maxPathUnits:8192,maxScopeDepth:8};
function fixture(){
 const env=createEnvironment('<html><body><div id="host"><b slot="named">light</b></div></body></html>');
 const {document}=env,host=document.querySelector('#host'),light=host.firstElementChild;
 const shadow=host.attachShadow({mode:'open'});shadow.innerHTML='<section><slot name="named"></slot><i id="nested"></i></section>';
 const nested=shadow.querySelector('#nested'),inner=nested.attachShadow({mode:'open'});inner.innerHTML='<em>inner</em>';
 const snapshot=createTestPublicCssFacts().capture(document,captureLimits);
 return {env,document,host,light,shadow,nested,inner,snapshot,route:createSharedRouting(snapshot,routingLimits)};
}
test('explicit genuine public test snapshot keeps physical routing and composed inheritance distinct',()=>{
 const f=fixture(),id=node=>f.snapshot.nodeId(node);
 assert.equal(f.route.targetFacts(id(f.host)).hostOf,id(f.shadow));
 assert.equal(f.route.targetFacts(id(f.nested)).hostOf,id(f.inner));
 assert.equal(f.route.targetFacts(id(f.nested)).scopeDepth,1);
 assert.equal(f.route.targetFacts(id(f.inner.firstElementChild)).scopeDepth,2);
 const slot=f.shadow.querySelector('slot'),light=f.route.targetFacts(id(f.light));
 assert.equal(light.parentId,id(slot));
 assert.equal(light.treeScope,id(f.document));
 assert.deepEqual(light.path,[id(f.light),id(f.host),...f.route.targetFacts(id(f.host)).path.slice(1)]);
 assert.deepEqual(light.assignedSlot,{slot:id(slot),treeScope:id(f.shadow),path:f.route.targetFacts(id(slot)).path});
 assert.equal(f.route.targetFacts(id(f.shadow.firstElementChild)).parentId,id(f.host));
 assert.equal(f.route.targetFacts(id(f.inner.firstElementChild)).parentId,id(f.nested));
 assert.equal(f.route.scopeDepth(id(f.document)),0);
 const encoded=JSON.stringify(f.route.contextRoots);
 assert.ok(!encoded.includes('element'));assert.ok(!encoded.includes('ownerDocument'));
});
test('limits and foreign structural facts fail explicitly',()=>{
 const {snapshot}=fixture();
 for(const name of ['maxRoots','maxNodes','maxPathUnits','maxScopeDepth'])assert.throws(()=>createSharedRouting(snapshot,{...routingLimits,[name]:0}),SharedRoutingError);
 assert.throws(()=>createSharedRouting(snapshot,{}),SharedRoutingError);
 const bad={...snapshot,roots:snapshot.roots.map(root=>({...root,parentScope:root.id}))};
 assert.throws(()=>createSharedRouting(bad,routingLimits),SharedRoutingError);
 const route=createSharedRouting(snapshot,routingLimits);
 assert.throws(()=>route.targetFacts(999999),SharedRoutingError);assert.throws(()=>route.scopeDepth(999999),SharedRoutingError);
});

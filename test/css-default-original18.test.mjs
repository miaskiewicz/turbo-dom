import test from 'node:test';import assert from 'node:assert/strict';import{readFileSync}from'node:fs';import{createHash}from'node:crypto';
import{createEnvironment,setParserMode}from'./opt-in-helpers/runtime.mjs';
import{caseHTML,readCase}from'./fixtures/css-shared-original18-159/fixture.mjs';
const casesURL=new URL('./fixtures/css-shared-original18-159/cases.json',import.meta.url),cases=JSON.parse(readFileSync(casesURL)),oracle=JSON.parse(readFileSync(new URL('./fixtures/css-shared-original18-159/CHROME159.json',import.meta.url)));
test('ordinary native default getComputedStyle matches original18 actual Chrome controls without private hooks',()=>{
 assert.equal(createHash('sha256').update(readFileSync(casesURL)).digest('hex'),oracle.casesSha256);assert.equal(oracle.browser,'159.0.8154.6');setParserMode('native');
 for(const scenario of cases){const env=createEnvironment(caseHTML(scenario));const actual=readCase(env.window,env.document,scenario);assert.deepEqual(actual.values,oracle.result.find(row=>row.id===scenario.id).values,scenario.id);}
});

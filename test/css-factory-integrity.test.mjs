import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import test from 'node:test';

const factoryUrl = new URL('../src/runtime/css-gcs-factory.mjs', import.meta.url).href;
const wireUrl = new URL('../src/runtime/css-rust-wire.mjs', import.meta.url).href;
const program = `
import assert from 'node:assert/strict';
import {createSharedComputedStyle} from ${JSON.stringify(factoryUrl)};
import {encodeCssWire} from ${JSON.stringify(wireUrl)};
const define=Object.defineProperty,descriptor=Object.getOwnPropertyDescriptor,remove=Reflect.deleteProperty;
const document={},element={},context={document,version:1,renderToken:{},environmentKey:{},resultKey:'target'};
let disposals=0,traps=0,presentations=0;
const backend={CompiledStylesV1:class{
 metadata(){return '{}'}
 prepare(){return{metadata:()=> '{}',resolveBatch:()=> '{"targets":[{"id":7,"values":{"color":[114,101,100],"width":[53,48,37]},"custom":{},"usedProperties":["width"]}]}',dispose(){disposals++}}}
 dispose(){disposals++}
}};
const factory=createSharedComputedStyle({
 selectBackend:()=>backend,readContext:()=>context,captureSources:()=>({status:'captured'}),
 compileRequest:()=>({}),prepareRequest:()=>({}),resolveRequest:()=>({request:{},resultId:7,providerContext:{data:[{value:'cell'}]}}),
 finalizeResult:row=>row,encodeRequest:encodeCssWire,encodeResultKey:encodeCssWire,
 resolveProperty(name,base,retained){presentations++;assert.equal(name,'width');assert.equal(retained.providerContext.data[0].value,'cell');return base},
});
const mode=process.argv[1];
const location={json:[Object.prototype,'toJSON'],push:[Array.prototype,'push'],iterator:[Array.prototype,Symbol.iterator],
 assign:[Object,'assign'],weakGet:[WeakMap.prototype,'get'],setAdd:[Set.prototype,'add'],
 numeric:[Array.prototype,'0'],species:[Array,Symbol.species]}[mode];
const original=descriptor(location[0],location[1]),sentinel=new Error(mode);
function trap(){traps++;throw sentinel}
define(location[0],location[1],{configurable:true,...(mode==='numeric'?{set:trap}:mode==='json'||mode==='species'?{get:trap}:{value:trap,writable:true})});
let style,error;
try{style=factory.get(element);assert.equal(style.color,'red');assert.equal(style.width,'50%');assert.equal(factory.get(element),style);factory.dispose();assert.equal(style.color,'red');assert.equal(style.width,'50%')}
catch(caught){error=caught}
finally{if(original)define(location[0],location[1],original);else remove(location[0],location[1])}
assert.equal(traps,0,'private computation must not invoke poisoned '+mode);
assert.equal(error,undefined);
assert.equal(disposals,2);
assert.equal(presentations,1);
`;

for (const mode of ['json', 'push', 'iterator', 'assign', 'weakGet', 'setAdd', 'numeric', 'species']) {
  test(`factory retains owned results without after-import ${mode} callouts`, () => {
    assert.equal(execFileSync(process.execPath, ['--input-type=module', '-e', program, mode], { encoding: 'utf8' }), '');
  });
}

test('result-key encoding preserves independent identity transport, omitted fields and original proxy errors', async () => {
  const { createSharedComputedStyle } = await import(factoryUrl);
  const { encodeCssWire } = await import(wireUrl);
  const document = {},
    element = {},
    compile = {},
    prepare = {};
  const context = { document, version: 1, renderToken: {}, environmentKey: {} };
  let resolves = 0,
    disposed = 0,
    keys = 0;
  const backend = {
    CompiledStylesV1: class {
      metadata() {
        return {};
      }
      prepare() {
        return {
          metadata: () => ({}),
          resolveBatch() {
            resolves++;
            return {
              targets: [{ id: 1, values: { color: [114, 101, 100] }, custom: {} }],
            };
          },
          dispose() {
            disposed++;
          },
        };
      }
      dispose() {
        disposed++;
      }
    },
  };
  const factory = createSharedComputedStyle({
    selectBackend: () => backend,
    readContext: () => context,
    captureSources: () => ({ status: 'captured' }),
    compileRequest: () => compile,
    prepareRequest: () => prepare,
    resolveRequest: () => ({ request: {}, resultId: 1 }),
    finalizeResult: (row) => row,
    encodeRequest: (value) => value,
    decodeResponse: (value) => value,
    encodeResultKey(value) {
      keys++;
      assert.equal(Object.getPrototypeOf(value), null);
      assert.equal(Object.hasOwn(value, 'pseudo'), false);
      return encodeCssWire(value);
    },
  });
  const first = factory.get(element);
  assert.equal(factory.get(element), first);
  assert.equal(first.color, 'red');
  assert.equal(resolves, 1);
  const sentinel = new Error('uncertified key proxy');
  context.resultKey = new Proxy(
    {},
    {
      ownKeys() {
        throw sentinel;
      },
    },
  );
  assert.throws(
    () => factory.get(element),
    (error) => error === sentinel,
  );
  assert.equal(keys, 3);
  factory.reset(document);
  assert.equal(disposed, 2);
  assert.equal(first.color, 'red');
  factory.dispose();
});

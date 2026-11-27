import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

test('holding a rule alone does not root its former document; an explicit document hold still does', () => {
  const runtime = new URL('../src/runtime/index.mjs', import.meta.url).href;
  const tracking = new URL('../src/runtime/css-invalidation.mjs', import.meta.url).href;
  const program = `
    const {createEnvironment}=await import(${JSON.stringify(runtime)});
    const {ensureCssState,cssFocusTarget}=await import(${JSON.stringify(tracking)});
    let retainedRule, heldDocument=null;
    function make(hold){const env=createEnvironment('<style></style><div>owner</div>',{parser:'native'});ensureCssState(env.document);const sheet=env.document.querySelector('style').sheet;sheet.insertRule('retained{}');env.document.body.focus();if(cssFocusTarget(env.document)!==env.document.body)throw new Error('focus fact missing');ensureCssState(env.document);retainedRule=sheet.cssRules[0];if(hold)heldDocument=env.document;return new WeakRef(env.document);}
    const job=()=>new Promise(resolve=>setImmediate(resolve));
    const collect=()=>{for(let index=0;index<3;index++)globalThis.gc();};
    const first=make(false);await job();collect();if(first.deref()!==undefined)throw new Error('rule retained document');if(retainedRule.selectorText!=='retained')throw new Error('rule stopped working');retainedRule.cssText='after{}';
    const second=make(true);await job();collect();if(second.deref()===undefined)throw new Error('explicit hold control lost document');heldDocument=null;await job();collect();if(second.deref()!==undefined)throw new Error('released control remained rooted');if(retainedRule.selectorText!=='retained')throw new Error('released rule stopped working');
  `;
  const child = spawnSync(process.execPath, ['--expose-gc', '--input-type=module', '-e', program], {
    encoding: 'utf8',
  });
  assert.equal(child.status, 0, child.stderr);
});

import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {resolve} from 'node:path';
const css='.x{text-decoration:underline dotted red;background:red;color:var(--c,blue);width:2em}@media(min-width:1000px){.x{color:red}}';
const server=createServer((request,response)=>{
 if(request.url==='/'){response.setHeader('content-type','text/html');response.end('<style>'+css+'</style><div class=x>t</div>');return;}
 const name=request.url?.slice(1);if(!['turbo_dom_css.js','turbo_dom_css_bg.wasm'].includes(name)){response.writeHead(404).end();return;}
 response.setHeader('content-type',name.endsWith('.wasm')?'application/wasm':'text/javascript');response.end(readFileSync(resolve('.evidence/payload/pkg-css-web',name)));
});
await new Promise(done=>server.listen(0,'127.0.0.1',done));
const url=`http://127.0.0.1:${server.address().port}`;
const {chromium}=createRequire(import.meta.url)('/Users/grzegorz/github-flux/turbo-surf/node_modules/playwright-core');
let browser;
try{
 browser=await chromium.launch({executablePath:'/Users/grzegorz/Library/Caches/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-mac-arm64/chrome-headless-shell',args:['--use-mock-keychain','--password-store=basic']});
 const page=await browser.newPage({viewport:{width:800,height:600}});await page.goto(url);
 const result=await page.evaluate(async({url,css})=>{
  const u=text=>Array.from({length:text.length},(_,index)=>text.charCodeAt(index));
  const loadedAt=performance.now();const backend=await import(url+'/turbo_dom_css.js');await backend.default({module_or_path:url+'/turbo_dom_css_bg.wasm'});const loadMs=performance.now()-loadedAt;
  const computation={maxNesting:32,maxOutputUnits:8192,maxTotalUnits:16384};
  const model=new backend.CompiledStylesV1(JSON.stringify({abiVersion:1,sources:[{id:7,text:u(css)}],limits:{sources:4,sourceUnits:8192,rules:128,sourceRulePreludes:128,selectors:128,declarations:256,properties:1024,groupDepth:16,tokenNesting:32}}));
  const metadata=JSON.parse(model.metadata()), target=document.querySelector('.x'), actual=getComputedStyle(target);
  const prepared=model.prepare(JSON.stringify({abiVersion:1,uses:[{sourceId:7,origin:'author',treeScope:0,scopeDepth:0,disabled:false,media:null}],environment:{mediaType:'screen',features:{width:{kind:'lengthPx',value:innerWidth}},unitBases:[['em',16],['rem',16]],selectorSupport:{},stylePhase:'steady'},limits:{uses:4,rules:128,layers:128,computation}}));
  const selectorAnswers=metadata.selectors.flatMap(selector=>selector.items.map(item=>({selectorId:selector.id,itemId:item.index,rootId:null,domainId:0,matches:target.matches(String.fromCharCode(...item.units))?[2]:[]})));
  const context={initialColor:'rgb(0, 0, 0)',initialFontFamily:'serif',initialFontSize:'16px',unitBases:[['rem',16]],fontSizeUnitBases:[],isRoot:false};
  const limits={targets:8,answerRows:128,answerIds:1024,pathNodes:128,inputUnits:8192,retainedResultUnits:16384,matchingDecisions:1024,inlineDeclarations:128,inlineProperties:1024,customRollbacks:128,valueRetainedUnits:8192,computation};
  model.dispose();const row=JSON.parse(prepared.resolveBatch(JSON.stringify({abiVersion:1,domains:[{id:0,nodeIds:[2]}],selectorAnswers,scopeAnswers:[],targets:[{id:2,parentId:null,path:[2,1],implicitScopeRoot:1,treeScope:0,scopeDepth:0,pseudo:null,inline:null,context}],limits}))).targets[0];
  const values=Object.fromEntries(Object.entries(row.values).map(([name,value])=>[name,String.fromCharCode(...value)]));
  prepared.dispose();const retained=object=>typeof object.retainedBytes==='function'?object.retainedBytes():object.retainedBytes;
  return {abi:backend.cssAbiVersion(),version:backend.cssBackendVersion(),loadMs,values:{color:values.color,width:values.width,backgroundPositionX:values['background-position-x']},oracle:{color:actual.color,width:actual.width,backgroundPositionX:actual.backgroundPositionX,computedTextDecoration:actual.textDecoration},declaredTextDecoration:String.fromCharCode(...row.declaredTextDecoration.value),publicKeys:row.publicKeys,disposed:[retained(model),retained(prepared)]};
 },{url,css});
 assert.equal(result.abi,1);assert.equal(result.version,'0.8.7');assert.deepEqual(result.values,{color:result.oracle.color,width:result.oracle.width,backgroundPositionX:result.oracle.backgroundPositionX});assert.equal(result.declaredTextDecoration,'underline dotted red');assert.equal(result.oracle.computedTextDecoration,'underline dotted rgb(255, 0, 0)');assert.deepEqual(result.disposed,[0,0]);
 result.browser=browser.version();result.sourceSha256=createHash('sha256').update(css).digest('hex');result.wasmSha256=createHash('sha256').update(readFileSync('.evidence/payload/pkg-css-web/turbo_dom_css_bg.wasm')).digest('hex');writeFileSync('.evidence/payload/WEB-WASM-PROBE.json',JSON.stringify(result,null,2)+'\n');
}finally{await browser?.close();await new Promise(done=>server.close(done));}

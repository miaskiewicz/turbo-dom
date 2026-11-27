// Genuine optional-capability absence: actual module graph, no Node loader/WASM.
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {makeGetComputedStyle} from '../src/runtime/cascade.mjs';
function absent(body){
 const script=`
 import vm from 'node:vm';import fs from 'node:fs';import assert from 'node:assert/strict';import {createRequire} from 'node:module';import {pathToFileURL,fileURLToPath} from 'node:url';
 const parser=createRequire(import.meta.url)('./index.js'),minimal={parse:parser.parse,parseBuffer:parser.parseBuffer,parseFragment:parser.parseFragment};
 const context=vm.createContext({WebAssembly:undefined,__TURBO_DOM_PARSER__:minimal}),modules=new Map(),dynamic=[];
 function load(url){if(modules.has(url))return modules.get(url);const module=new vm.SourceTextModule(fs.readFileSync(fileURLToPath(url),'utf8'),{context,identifier:url,initializeImportMeta(meta){meta.url=url;},importModuleDynamically(name){dynamic.push(name);throw Error('bare host: no Node modules');}});modules.set(url,module);return module;}
 const entry=load(pathToFileURL(process.cwd()+'/src/runtime/index.mjs').href);await entry.link((name,from)=>{if(!name.startsWith('.'))throw Error('unexpected static import '+name);return load(new URL(name,from.identifier).href);});await entry.evaluate();
 const runtime=entry.namespace;
 assert.equal(vm.runInContext('typeof WebAssembly',context),'undefined');assert.equal(vm.runInContext('typeof process',context),'undefined');assert.deepEqual(Object.keys(minimal),['parse','parseBuffer','parseFragment']);
 ${body}
 assert.deepEqual(dynamic,['node:module']);assert.ok(modules.size>30);console.log('genuine capability-absent public GCS PASS');`;
 const result=spawnSync(process.execPath,['--experimental-vm-modules','--input-type=module','-e',script],{encoding:'utf8'});assert.equal(result.status,0,result.stderr);assert.match(result.stdout,/genuine capability-absent public GCS PASS/);
}
test('absent CSS capability preserves ordinary public cascade specificity, expansions and declared values',()=>absent(`
 const {document,window}=runtime.createEnvironment('<body><style>/*x*/ ; } @media screen {p{color:blue}} p { color: red; margin:1px 2px 3px 4px; padding:0; border:1px solid red; background:blue; broken; :empty; } #target { color:green!important; } p {} trailing</style><p id="target">x</p>');
 const p=document.querySelector('p'),style=window.getComputedStyle(p);assert.equal(style.color,'rgb(0, 128, 0)');assert.equal(style.marginLeft,'4px');assert.equal(style.marginTop,'1px');assert.equal(style.paddingRight,'0px');assert.equal(style.borderTopWidth,'1px');assert.equal(style.borderStyle,'solid');assert.equal(style.borderColor,'rgb(255, 0, 0)');assert.equal(style.backgroundColor,'rgb(0, 0, 255)');assert.equal(style.unknown,'');
 p.setAttribute('style','color:purple; width:0; font-family:A,B;');const inline=window.getComputedStyle(p);assert.equal(inline.color,'rgb(128, 0, 128)');assert.equal(inline.width,'0px');assert.equal(inline.fontFamily,'A, B');
`));
test('absent public style views retain inheritance, cache invalidation and original proxy operations',()=>absent(`
 const {document,window}=runtime.createEnvironment('<body><style>div{color:red;font-size:12px;cursor:pointer} span{margin:2px}</style><div><span>x</span></div>');const span=document.querySelector('span');const first=window.getComputedStyle(span);assert.equal(first.color,'rgb(255, 0, 0)');assert.equal(first.fontSize,'12px');assert.equal(first.cursor,'pointer');assert.equal(first.marginRight,'2px');assert.equal(first.getPropertyPriority('color'),'');assert.ok(first.length>0);assert.ok(first.item(0));assert.equal(first.item(999),'');assert.ok([...first].includes('color'));assert.equal(first[Symbol.toStringTag],undefined);assert.match(first.__honest,/no layout/);assert.equal(window.getComputedStyle(span),first);
 span.style.color='blue';assert.equal(window.getComputedStyle(span).color,'rgb(0, 0, 255)');assert.equal(first.color,'rgb(255, 0, 0)');
`));
test('absent public cascade consumes real injected CSSOM rules and preserves raw unresolved values',()=>absent(`
 const {document,window}=runtime.createEnvironment('<body><style></style><p>x</p>');const sheet=document.querySelector('style').sheet,p=document.querySelector('p');sheet.insertRule('p{color:currentColor;width:var(--x);border:thick dotted; margin:1px 2px 3px;padding:4px 5px}');let style=window.getComputedStyle(p);assert.equal(style.color,'currentColor');assert.equal(style.width,'var(--x)');assert.equal(style.borderWidth,'thick');assert.equal(style.borderStyle,'dotted');assert.equal(style.marginLeft,'2px');assert.equal(style.paddingBottom,'4px');sheet.deleteRule(0);assert.equal(window.getComputedStyle(p).color,'');sheet.insertRule('p{border:red;background:linear-gradient(red,blue)}');assert.equal(window.getComputedStyle(p).borderColor,'rgb(255, 0, 0)');
`));
test('absent public shadow cascade preserves own host, slotted and inherited routing',()=>absent(`
 const {document,window}=runtime.createEnvironment('<body><style>span{color:purple}</style><div class="on"><span>light</span></div>');const host=document.querySelector('div'),light=document.querySelector('span'),root=host.attachShadow({mode:'open'});root.innerHTML='<style>:host{color:red}:host(.on){font-size:20px}:host(.off){color:blue}::slotted(span){background:blue} p{color:green}</style><slot></slot><p>shadow</p>';assert.equal(window.getComputedStyle(host).color,'rgb(255, 0, 0)');assert.equal(window.getComputedStyle(host).fontSize,'20px');assert.equal(window.getComputedStyle(light).backgroundColor,'rgb(0, 0, 255)');const shadow=root.querySelector('p');assert.equal(window.getComputedStyle(shadow).color,'rgb(0, 128, 0)');assert.equal(window.getComputedStyle(shadow).fontSize,'20px');
`));
test('absent CSS capability preserves the legacy malformed hex value',()=>absent(`
 const {document,window}=runtime.createEnvironment('<body><p style="color:#ff">x</p>');
 assert.equal(window.getComputedStyle(document.querySelector('p')).color,'#ff');
`));
test('legacy kernel unit contract for absent target/document stays separate from default routing proof',()=>{
 const gcs=makeGetComputedStyle();assert.equal(gcs(null).color,'');assert.equal(gcs(null).__v,-1);
 const orphan={ownerDocument:null};assert.equal(gcs(orphan).color,'');assert.equal(gcs(orphan).__v,0);assert.equal(gcs(orphan),gcs(orphan));
});

test('optional internal CSS fallback stays distinct from explicit Rust and missing parser refusal',()=>absent(`
 const {document}=runtime.createEnvironment('<body><p style="color:red">x</p>');
 const defaults=modules.get(pathToFileURL(process.cwd()+'/src/runtime/css-default.mjs').href).namespace;
 assert.equal(defaults.makeDefaultGetComputedStyle()(document.querySelector('p')).color,'rgb(255, 0, 0)');
 assert.throws(()=>defaults.makeDefaultGetComputedStyle({requireBackend:true})(document.querySelector('p')),/Rust CSS artifact/);
 runtime.setParser(null);delete context.__TURBO_DOM_PARSER__;runtime.setParserMode('native');
 assert.throws(()=>runtime.createEnvironment('<p>without parser</p>',{cssBackend:'rust'}),/no parser available/);
`));

import { createRequire } from 'node:module';
import { writeFile, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({headless:true,...(process.env.TURBO_CHROME_HEADLESS?{executablePath:process.env.TURBO_CHROME_HEADLESS}:{})});
try {
 const page=await browser.newPage();
 await page.setContent('<!doctype html><div id="target" style="width:100px;height:20px"></div>');
 const result=await page.evaluate(()=>{
   const target=document.getElementById('target');const motion=target.animate([{width:'100px'},{width:'200px'}],{duration:1000,fill:'both'});motion.pause();motion.currentTime=500;
   const before=target.getBoundingClientRect().width,absent=!Object.hasOwn(Error,'prepareStackTrace'),stacks=[];
   function capture(){return new Error('stack-control').stack;}
   for(let i=0;i<2;i++){if(i)Error.prepareStackTrace=undefined;stacks.push(capture());}
   const after=target.getBoundingClientRect().width,d=Object.getOwnPropertyDescriptor(Error,'prepareStackTrace');
   let getterCalls=0;Object.defineProperty(Error,'prepareStackTrace',{configurable:true,get(){getterCalls++;return undefined}});
   const withGetter=target.getBoundingClientRect().width;
   return {absent,before,after,withGetter,getterCalls,sameStack:stacks[0]===stacks[1],stackPrefix:stacks[0].split('\n')[0],descriptor:{data:Object.hasOwn(d,'value'),undefined:d.value===undefined,writable:d.writable,enumerable:d.enumerable,configurable:d.configurable}};
 });
 const source=await readFile(new URL(import.meta.url));
 await writeFile(new URL('./error_stack_hook_chrome159.json',import.meta.url),JSON.stringify({browser:browser.version(),probeSha256:createHash('sha256').update(source).digest('hex'),result},null,2)+'\n');
}finally{await browser.close();}

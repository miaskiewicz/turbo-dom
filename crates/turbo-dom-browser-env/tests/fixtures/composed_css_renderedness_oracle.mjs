import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const source=await readFile(new URL('./composed_css_renderedness_cases.json',import.meta.url));
const cases=JSON.parse(source);
const html=await readFile(new URL('./composed_css_renderedness.html',import.meta.url));
const browser=await chromium.launch({headless:true,...(process.env.TURBO_CHROME_HEADLESS?{executablePath:process.env.TURBO_CHROME_HEADLESS}:{})});
const rows=[];
try{
 for(const input of cases){
  const page=await browser.newPage();
  await page.setContent(html.toString('utf8'));
  await page.evaluate(code=>(0,eval)(code),input.setup);
  const steps=[];
  for(const step of input.steps){
   await page.evaluate(code=>(0,eval)(code),step.script);
   const targets=await page.evaluate(()=>globalThis.targets.map(target=>{
    const effects=target.getAnimations();
    return{id:target.id,css:effects.filter(effect=>effect instanceof CSSAnimation).length,waapi:effects.filter(effect=>!(effect instanceof CSSAnimation)&&!(effect instanceof CSSTransition)).length,cssNames:effects.filter(effect=>effect instanceof CSSAnimation).map(effect=>effect.animationName)};
   }));
   steps.push({id:step.id,targets});
  }
  rows.push({id:input.id,steps});await page.close();
 }
 const probe=await readFile(new URL(import.meta.url));const hash=data=>createHash('sha256').update(data).digest('hex');
 await writeFile(new URL('./composed_css_renderedness_chrome159.json',import.meta.url),JSON.stringify({browser:browser.version(),casesSha256:hash(source),htmlSha256:hash(html),probeSha256:hash(probe),rows},null,2)+'\n');
}finally{await browser.close();}

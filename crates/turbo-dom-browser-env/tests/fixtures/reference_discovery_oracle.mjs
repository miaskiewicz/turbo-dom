import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({headless:true,...(process.env.TURBO_CHROME_HEADLESS?{executablePath:process.env.TURBO_CHROME_HEADLESS}:{})});
const html="<!doctype html><style>@keyframes probe{from{opacity:.5}to{opacity:1}}</style><div style='font-size:24px'><div id=target style='width:100px;height:20px;transform:translateX(50%);animation:probe 1000ms linear both paused'></div></div>";
try{
 const page=await browser.newPage();await page.setContent(html);
 const result=await page.evaluate(()=>{
  const target=document.getElementById('target');const rows=[];
  for(const [operation,effects]of [['document',document.getAnimations()],['element',target.getAnimations()]])rows.push({operation,count:effects.length,name:effects[0].animationName,state:effects[0].playState,time:effects[0].currentTime,frames:effects[0].effect.getKeyframes()});
  const first=getComputedStyle(target).transform;target.style.width='200px';const second=getComputedStyle(target).transform;
  return{rows,first,second};
 });
 const shadowPage=await browser.newPage();await shadowPage.setContent('<!doctype html>');
 const shadowResult=await shadowPage.evaluate(()=>{
   const host=document.createElement('div');document.body.appendChild(host);const root=host.attachShadow({mode:'closed'});
   root.innerHTML='<style>@keyframes probe{from{opacity:.5}to{opacity:1}}div{animation:probe 1s linear both paused}</style><div></div>';
   const before=root.getAnimations().length;host.style.display='none';const after=root.getAnimations().length;
   const script=root.querySelector('div').animate([{opacity:.2},{opacity:.8}],{duration:1000,fill:'both'});script.pause();
   return{cssBefore:before,cssAfterHiddenHost:after,waapiAfterHiddenHost:root.getAnimations().filter(a=>!(a instanceof CSSAnimation)).length};
 });
 const source=await readFile(new URL(import.meta.url));const hash=value=>createHash('sha256').update(value).digest('hex');
 await writeFile(new URL('./reference_discovery_chrome159.json',import.meta.url),JSON.stringify({browser:browser.version(),probeSha256:hash(source),htmlSha256:hash(html),html,result,shadowResult},null,2)+'\n');
}finally{await browser.close();}

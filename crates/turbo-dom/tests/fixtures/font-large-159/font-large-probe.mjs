import {createRequire} from 'node:module';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
const {chromium}=createRequire(import.meta.url)('/Users/grzegorz/github-flux/turbo-surf/node_modules/playwright-core');
const cases=[['1e20px','1e20'],['1e30px','1e30'],['3.402823466e38px','1'],['1e20px','1'],['12px','1e20'],['12px','1e38'],['0px','1e38'],['1e38px','0'],['9999px','1'],['10001px','1'],['20000px','50000px']];
const browser=await chromium.launch({executablePath:'/Users/grzegorz/Library/Caches/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-mac-arm64/chrome-headless-shell',args:['--use-mock-keychain','--password-store=basic']});
try {
 const session=await browser.newBrowserCDPSession();const version=await session.send('Browser.getVersion');const page=await browser.newPage();
 const result=await page.evaluate(cases=>cases.flatMap(([size,height])=>['inline','stylesheet'].map(mode=>{
  document.body.innerHTML='<style></style><div id=p><span id=c>x</span></div>';
  const p=document.getElementById('p'),c=document.getElementById('c');
  if(mode==='inline'){p.style.cssText=`font-size:${size};line-height:${height};width:1em`;c.style.cssText='font-size:inherit;line-height:inherit;display:block;width:1em';}
  else document.querySelector('style').textContent=`#p{font-size:${size};line-height:${height};width:1em}#c{font-size:inherit;line-height:inherit;display:block;width:1em}`;
  const css=getComputedStyle(p),child=getComputedStyle(c);
  return {mode,size,height,authoredSize:p.style.fontSize,authoredHeight:p.style.lineHeight,fontSize:css.fontSize,lineHeight:css.lineHeight,width:css.width,childFontSize:child.fontSize,childLineHeight:child.lineHeight,childWidth:child.width};
 })),cases);
 const source=readFileSync(new URL('./font-large-probe.mjs',import.meta.url));const receipt={browser:browser.version(),version,sourceSha256:createHash('sha256').update(source).digest('hex'),result};
 writeFileSync(new URL('./FONT-LARGE-CHROME159.json',import.meta.url),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify({browser:receipt.browser,revision:version.revision,cases:result.length,first:result.slice(0,2)}));
}finally{await browser.close();}

import { createRequire } from 'node:module';
import { readFileSync,writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const {chromium}=createRequire(import.meta.url)('/Users/grzegorz/github-flux/turbo-surf/node_modules/playwright-core');
const browser=await chromium.launch({executablePath:'/Users/grzegorz/Library/Caches/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-mac-arm64/chrome-headless-shell',args:['--use-mock-keychain','--password-store=basic']});
try {const page=await browser.newPage();const result=await page.evaluate(()=>{
 document.body.innerHTML='<style id=s></style><div class=x></div>';const style=document.getElementById('s'),target=document.querySelector('.x');
 const read=(css)=>{style.textContent=css;return {css,declared:style.sheet.cssRules[0].style.getPropertyValue('display'),computed:getComputedStyle(target).display};};
 return {supports:CSS.supports('display','not-a-display'),invalidParsed:read('.x{display:inline-block;display:not-a-display}'),invalidSubstituted:read('.x{--bad:not-a-display;display:inline-block;display:var(--bad)}')};
});const source=readFileSync(new URL('./invalid-display-probe.mjs',import.meta.url));const receipt={browser:browser.version(),sourceSha256:createHash('sha256').update(source).digest('hex'),result};writeFileSync(new URL('./INVALID-DISPLAY-ORACLE.json',import.meta.url),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify(receipt));}finally{await browser.close();}

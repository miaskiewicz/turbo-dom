import {createRequire} from 'node:module';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
const {chromium}=createRequire(import.meta.url)('/Users/grzegorz/github-flux/turbo-surf/node_modules/playwright-core');
const casesSource=readFileSync(new URL('./cases.json',import.meta.url),'utf8');
const fixtureSource=readFileSync(new URL('./fixture.mjs',import.meta.url),'utf8');
const cases=JSON.parse(casesSource);
const probe=({scenario,fixture})=>{
 const execute=new Function('window','document','scenario',fixture.replaceAll('export ','')+';document.open();document.write(caseHTML(scenario));document.close();return readCase(window,document,scenario);');
 return execute(window,document,scenario);
};
const browser=await chromium.launch({executablePath:'/Users/grzegorz/Library/Caches/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-mac-arm64/chrome-headless-shell',args:['--use-mock-keychain','--password-store=basic']});
try{const page=await browser.newPage({viewport:{width:1024,height:768}});const results=[];for(const scenario of cases)results.push(await page.evaluate(probe,{scenario,fixture:fixtureSource}));
 const hash=value=>createHash('sha256').update(value).digest('hex');
 const receipt={browser:browser.version(),viewport:{width:1024,height:768},fixtureSha256:hash(fixtureSource),casesSha256:hash(casesSource),probeSha256:hash(probe.toString()),results};writeFileSync(new URL('./CHROME159.json',import.meta.url),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify({browser:receipt.browser,cases:results.length}));
}finally{await browser.close();}

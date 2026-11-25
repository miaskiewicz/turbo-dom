import {createRequire} from 'node:module';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
const modulePath=process.env.CHROME159_PLAYWRIGHT_MODULE;
const executablePath=process.env.CHROME159_EXECUTABLE;
if(!modulePath||!executablePath)throw Error('Explicit CHROME159_PLAYWRIGHT_MODULE and CHROME159_EXECUTABLE required');
const {chromium}=createRequire(import.meta.url)(modulePath);
const source=readFileSync(new URL('./cases.json',import.meta.url),'utf8');const cases=JSON.parse(source);
const browser=await chromium.launch({executablePath,args:['--use-mock-keychain','--password-store=basic']});
const probe=scenario=>{
 document.open();document.write('<!doctype html><html><head><style>body{margin:0}'+scenario.css+'</style></head><body>'+scenario.html+'</body></html>');document.close();
 const target=document.querySelector('.x'),style=getComputedStyle(target);return{id:scenario.id,values:Object.fromEntries(scenario.properties.map(name=>[name,style.getPropertyValue(name)])),authored:target.style.cssText};
};
try{const page=await browser.newPage({viewport:{width:1024,height:768}});const identity=await(await browser.newBrowserCDPSession()).send('Browser.getVersion');const results=[];for(const scenario of cases)results.push(await page.evaluate(probe,scenario));const receipt={browser:browser.version(),identity,viewport:{width:1024,height:768},casesSha256:createHash('sha256').update(source).digest('hex'),probeSha256:createHash('sha256').update(probe.toString()).digest('hex'),results};writeFileSync(new URL('./CHROME159.json',import.meta.url),JSON.stringify(receipt,null,2)+'\n');console.log(JSON.stringify({browser:receipt.browser,results}));}finally{await browser.close();}

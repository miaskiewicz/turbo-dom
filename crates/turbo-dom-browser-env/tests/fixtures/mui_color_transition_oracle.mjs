import { createRequire } from 'node:module';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const html = await readFile(new URL('./mui_color_transition.html', import.meta.url), 'utf8');
const browser = await chromium.launch({headless:true,...(process.env.TURBO_CHROME_HEADLESS?{executablePath:process.env.TURBO_CHROME_HEADLESS}:{})});
try {
  const page = await browser.newPage();
  await page.setContent(html);
  const result = await page.evaluate(() => {
    const target=document.getElementById('target');getComputedStyle(target).color;
    target.className='enabled';getComputedStyle(target).color;
    const effects=target.getAnimations();for(const effect of effects){effect.pause();effect.currentTime=0;}
    const before=effects.map(a=>({property:a.transitionProperty,frames:a.effect.getKeyframes()}));
    const samples=[];
    for(const time of [0,125,250]){for(const effect of effects)effect.currentTime=time;const rect=target.getBoundingClientRect();samples.push({time,rect:[rect.x,rect.y,rect.width,rect.height],color:getComputedStyle(target).color,background:getComputedStyle(target).backgroundColor});}
    target.style.setProperty('--variant-containedBg','rgb(200, 30, 40)');
    getComputedStyle(target).backgroundColor;
    return {before,after:effects.map(a=>({property:a.transitionProperty,frames:a.effect.getKeyframes()})),samples};
  });
  const output={browser:browser.version(),sha256:createHash('sha256').update(html).digest('hex'),result};
  await writeFile(new URL('./mui_color_transition_chrome159.json',import.meta.url),JSON.stringify(output,null,2)+'\n');
} finally {await browser.close();}

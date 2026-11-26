// Serial fresh-process runner. Execute only in the coordinated quiet window.
import {spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdirSync,readdirSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import os from 'node:os';
import {performance} from 'node:perf_hooks';
const [consumerArg,outputArg,roundsArg='5',phase='baseline']=process.argv.slice(2);
if(!consumerArg||!outputArg)throw new Error('Usage: node benchmark.mjs CONSUMER OUTPUT_DIR [ROUNDS] [baseline|after]');
const consumer=resolve(consumerArg),output=resolve(outputArg),rounds=Number(roundsArg);
if(!Number.isInteger(rounds)||rounds<3||rounds>9)throw new Error('ROUNDS must be 3..9');
if(!['baseline','after'].includes(phase))throw new Error('phase must be baseline or after');
mkdirSync(output,{recursive:true});
const excluded='getByLabelText: wrapping <label> labels only its FIRST control';
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const filter=`^(?!${escape(excluded)}$).*$`;
const rows=[{id:'happy-dom',env:'happy-dom',parser:null,css:'happy-dom'}, {id:'jsdom',env:'jsdom',parser:null,css:'jsdom'}, {id:phase==='after'?'turbo-native-public-css':'turbo-native-legacy-css',env:'turbo-dom',parser:'native',css:phase==='after'?'public CSS default (sentinel required)':'LEGACY JavaScript CSS (pre-cutover)'}, {id:phase==='after'?'turbo-wasm-public-css':'turbo-wasm-legacy-css',env:'turbo-dom',parser:'wasm',css:phase==='after'?'public CSS default (sentinel required)':'LEGACY JavaScript CSS (pre-cutover)'}];
const hash=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
const artifactRoot=join(consumer,'node_modules/turbo-dom');
function artifactManifest(directory=artifactRoot,prefix=''){
 const result={};for(const entry of readdirSync(directory,{withFileTypes:true})){const name=prefix+entry.name;if(entry.isDirectory())Object.assign(result,artifactManifest(join(directory,entry.name),`${name}/`));else if(entry.isFile())result[name]=hash(join(directory,entry.name));else throw new Error(`unexpected packed artifact entry ${name}`);}return result;
}
const artifacts=artifactManifest();
function verifyArtifacts(){if(JSON.stringify(artifactManifest())!==JSON.stringify(artifacts))throw new Error('packed artifact changed during acceptance');}
const versions={};for(const name of ['vitest','happy-dom','jsdom','react','react-dom','@testing-library/react','@testing-library/user-event'])versions[name]=JSON.parse(readFileSync(join(consumer,'node_modules',name,'package.json'))).version;
const metadata={phase,artifacts,artifactRoot,date:new Date().toISOString(),node:process.version,platform:process.platform,arch:process.arch,os:os.release(),cpus:os.cpus()[0]?.model,cpuCount:os.cpus().length,memoryBytes:os.totalmem(),versions,consumer,excluded,filter,rounds,runnerSha256:hash(new URL(import.meta.url)),baseline:JSON.parse(readFileSync(new URL('./BASELINE.json',import.meta.url))),loadBefore:os.loadavg(),runs:[]};
function run(row,phase,index){
 const stem=`${phase}-${index}-${row.id}`,report=join(output,`${stem}.json`);
 const env={...process.env,CONSUMER_ENV:row.env};delete env.CSS_ACCEPTANCE;delete env.TURBO_DOM_PARSER;if(row.parser)env.TURBO_DOM_PARSER=row.parser;
 verifyArtifacts();
 const started=performance.now();
 const result=spawnSync(process.execPath,['node_modules/vitest/vitest.mjs','run','--config','vitest.config.mjs','--maxWorkers=1','--testNamePattern',filter,'--reporter=json','--outputFile',report],{cwd:consumer,env,encoding:'utf8'});
 const wallMs=performance.now()-started;verifyArtifacts();writeFileSync(join(output,`${stem}.log`),result.stdout+result.stderr);
 if(result.status!==0)throw new Error(`${stem} failed (exit ${result.status}); see log`);
 const data=JSON.parse(readFileSync(report));const tests=data.testResults.flatMap(file=>file.assertionResults);
 const passed=tests.filter(x=>x.status==='passed'),skipped=tests.filter(x=>x.status!=='passed');
 if(passed.length!==28||skipped.length!==1||skipped[0].fullName!==excluded||!['pending','skipped','todo'].includes(skipped[0].status))throw new Error(`${stem}: expected exactly 28 passes plus ONLY excluded title; actual ${JSON.stringify(tests.map(x=>[x.fullName,x.status]))}`);
 const entry={row:row.id,css:row.css,parser:row.parser,phase,index,wallMs,passes:28,excluded:skipped[0].fullName,load:os.loadavg(),reportSha256:hash(report)};
 metadata.runs.push(entry);writeFileSync(join(output,'RAW.json'),JSON.stringify(metadata,null,2)+'\n');
}
function sentinel(parser){
 const report=join(output,`sentinel-${parser}.json`),env={...process.env,CONSUMER_ENV:'turbo-dom',TURBO_DOM_PARSER:parser,CSS_ACCEPTANCE:'1'};
 verifyArtifacts();
 const result=spawnSync(process.execPath,['node_modules/vitest/vitest.mjs','run','--config','vitest.config.mjs','--maxWorkers=1','--reporter=json','--outputFile',report],{cwd:consumer,env,encoding:'utf8'});
 writeFileSync(join(output,`sentinel-${parser}.log`),result.stdout+result.stderr);
 if(result.status!==0)throw new Error(`after-phase public CSS ${parser} sentinel failed; no timings started`);
 const data=JSON.parse(readFileSync(report)),tests=data.testResults.flatMap(file=>file.assertionResults);
 if(tests.length!==1||tests[0].status!=='passed')throw new Error(`after-phase ${parser} sentinel must report exactly one passing CSS assertion`);
 verifyArtifacts();return {parser,passed:1,reportSha256:hash(report)};
}
if(phase==='after'){
 metadata.sentinels=[sentinel('native'),sentinel('wasm')];
 writeFileSync(join(output,'PREFLIGHT.json'),JSON.stringify(metadata,null,2)+'\n');
}
for(let index=0;index<rows.length;index++)run(rows[index],'warmup',index);
for(let round=0;round<rounds;round++)for(let offset=0;offset<rows.length;offset++)run(rows[(round+offset)%rows.length],'timed',round);
const median=values=>{const s=[...values].sort((a,b)=>a-b);return s.length%2?s[(s.length-1)/2]:(s[s.length/2-1]+s[s.length/2])/2;};
metadata.summary=rows.map(row=>{const values=metadata.runs.filter(x=>x.phase==='timed'&&x.row===row.id).map(x=>x.wallMs);return {...row,medianWallMs:median(values),minWallMs:Math.min(...values),maxWallMs:Math.max(...values),samples:values};});metadata.loadAfter=os.loadavg();writeFileSync(join(output,'RESULT.json'),JSON.stringify(metadata,null,2)+'\n');console.log(JSON.stringify(metadata.summary,null,2));

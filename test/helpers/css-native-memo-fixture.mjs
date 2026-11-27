import {loadNativeCss} from '../opt-in-helpers/native.mjs';
const units=value=>Array.from(value,c=>c.charCodeAt(0));
export function nativeMemoFixture(){
 const native=loadNativeCss();
 const computation={maxNesting:32,maxOutputUnits:8192,maxTotalUnits:16384};
 const compiled=new native.CompiledStylesV1(JSON.stringify({abiVersion:1,sources:[{id:7,text:units('.x{color:red}')}],limits:{sources:4,sourceUnits:8192,rules:128,sourceRulePreludes:128,selectors:128,declarations:256,properties:1024,groupDepth:16,tokenNesting:32}}));
 const prepared=compiled.prepare(JSON.stringify({abiVersion:1,uses:[{sourceId:7,origin:'author',treeScope:0,scopeDepth:0,disabled:false,media:null}],environment:{mediaType:'screen',features:{},unitBases:[['em',16],['rem',16]],selectorSupport:{},stylePhase:'steady'},limits:{uses:4,rules:128,layers:128,computation}}));
 const limits={targets:8,answerRows:128,answerIds:1024,pathNodes:128,inputUnits:8192,retainedResultUnits:16384,matchingDecisions:1024,inlineDeclarations:128,inlineProperties:1024,customRollbacks:128,valueRetainedUnits:8192,computation};
 const facts={domains:[{id:0,nodeIds:[2]}],selectorAnswers:[{selectorId:0,itemId:0,rootId:null,domainId:0,matches:[2]}],scopeAnswers:[]};
 const input={abiVersion:1,targets:[{id:2,parentId:null,path:[2,1],implicitScopeRoot:1,treeScope:0,scopeDepth:0,pseudo:null,inline:null,context:{initialColor:'rgb(0, 0, 0)',initialFontFamily:'serif',initialFontSize:'16px',unitBases:[['rem',16]],fontSizeUnitBases:[],isRoot:false}}],limits};
 const token=prepared.storeMatchingV1(JSON.stringify(facts),JSON.stringify(limits));
 return {prepared,compiled,input,facts,token,wire:JSON.stringify(input),dispose(){prepared.dispose();compiled.dispose()}};
}
export function oversizedMemoResponse(input){
 const copy=JSON.parse(JSON.stringify(input));
 copy.targets[0].inline=units('--blob:'+'x'.repeat(20000));
 for(const name of ['inputUnits','retainedResultUnits','valueRetainedUnits'])copy.limits[name]=60000;
 copy.limits.computation.maxOutputUnits=60000;
 copy.limits.computation.maxTotalUnits=120000;
 return JSON.stringify(copy);
}

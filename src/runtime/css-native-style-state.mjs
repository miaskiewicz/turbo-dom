// Explicit native-state-v1 protocol. The default wire-v1 path stays observable.
import {sharedEncode} from './css-shared-codec.mjs';
const parse = JSON.parse, ownKeys = Reflect.ownKeys, descriptor = Object.getOwnPropertyDescriptor;
const own = Object.hasOwn, define = Object.defineProperty, create = Object.create, freeze = Object.freeze;
const safeInteger = Number.isSafeInteger, isArray = Array.isArray, apply = Reflect.apply;
const Weak = WeakMap, weakGet = WeakMap.prototype.get, weakSet = WeakMap.prototype.set;
function data(value, name) {
 if (value === null || typeof value !== 'object') return undefined;
 const field = descriptor(value, name);
 return field && own(field, 'value') ? field.value : undefined;
}
function exact(value, names) {
 if (value === null || typeof value !== 'object') return false;
 const keys = ownKeys(value);
 if (keys.length !== names.length) return false;
 for (let i=0;i<names.length;i++) if (!own(value,names[i])) return false;
 return true;
}
function copyNumbers(value, names) {
 const result=create(null);
 for(let i=0;i<names.length;i++) {
  const number=data(value,names[i]);
  if(!safeInteger(number)||number<0)return undefined;
  define(result,names[i],{value:number,enumerable:true});
 }
 return result;
}
function policyCopy(policy) {
 if(data(policy,'abiVersion')!==1)return undefined;
 const source=data(data(policy,'adapter'),'serialize');
 const sizes=['targets','inputUnits','retainedUnits'];
 if(!exact(source,['targets','inputUnits','retainedUnits','computation']))return undefined;
 const computation=data(source,'computation'), fields=['maxNesting','maxOutputUnits','maxTotalUnits'];
 if(!exact(computation,fields))return undefined;
 const nested=copyNumbers(computation,fields), result=copyNumbers(source,sizes);
 if(!nested||!result)return undefined;
 define(result,'computation',{value:freeze(nested),enumerable:true});
 return freeze(result);
}
export function captureStyleStatePolicy(wire) {
 if(typeof wire!=='string')return undefined;
 try{return policyCopy(parse(wire))}catch{return undefined}
}
function matchingRow(rows,id) {
 if(!isArray(rows))throw new TypeError('CSS native state target list');
 let found;
 for(let i=0;i<rows.length;i++) {
  if(data(rows[i],'id')!==id)continue;
  if(found!==undefined)throw new TypeError('CSS native state duplicate result target');
  found=rows[i];
 }
 if(found===undefined)throw new TypeError('CSS native state result target missing');
 return found;
}
function envelope(wire,id) {
 if(typeof wire!=='string')throw new TypeError('CSS style state response must be JSON');
 const batch=parse(wire);
 if(data(batch,'abiVersion')!==1||data(batch,'geometryOnly')!==false)throw new TypeError('CSS style state response ABI');
 const row=matchingRow(data(batch,'targets'),id);
 if(!isArray(data(row,'usedProperties'))||typeof data(row,'values')!=='object'||data(row,'values')===null)
  throw new TypeError('CSS style state response shape');
 return {resolved:batch,row};
}
export function createStyleStateSession(limits) {
 const requests=new Weak(), projections=new Weak();
 return {
  request(result) {
   if(limits!==undefined)apply(weakSet,requests,[result.request,result.resultId]);
   return result;
  },
  requireMatching(request) {
   if(apply(weakGet,requests,[request])!==undefined)throw new TypeError('turbo-dom: native-state-v1 requires retained matching capability');
  },
  resolve(prepared,wire,token,request) {
   const id=apply(weakGet,requests,[request]);
   if(id===undefined)return undefined;
   if(typeof prepared.resolveStyleStateV1!=='function')throw new TypeError('turbo-dom: native-state-v1 requires resolveStyleStateV1');
   const options=sharedEncode({abiVersion:1,targetId:id,textDecoration:'declared',includeUsedProperties:true,limits});
   const output=envelope(prepared.resolveStyleStateV1(wire,token,options),id);
   apply(weakSet,projections,[output.row,true]);
   return output.resolved;
  },
  finalize(row) {
   const projection=apply(weakGet,projections,[row]);
   if(projection===undefined)return undefined;
   return row;
  },
 };
}

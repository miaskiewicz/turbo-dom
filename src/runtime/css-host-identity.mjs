// Optional host identity only, never a DOM/style brand or an injected mode flag.
import {createParserRequire} from './parser.mjs';
const apply=Reflect.apply;
let checker=null, failure=null;
try{
 const require=createParserRequire();
 if(require)checker=require('node:util').types.isProxy;
}catch(error){failure=error;}
export class CssHostIdentityUnavailable extends Error{
 constructor(){super('turbo-dom: native CSS proxy admission unavailable');this.name='CssHostIdentityUnavailable';}
}
export function isCssHostProxy(value){
 if(typeof checker!=='function'){if(failure!==null)throw failure;throw new CssHostIdentityUnavailable();}
 return apply(checker,undefined,[value]);
}

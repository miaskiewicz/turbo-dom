// A host-visible addon prototype was changed before the first default CSS load.
// Same request must still invoke the callback on its second use.
import assert from 'node:assert/strict';
import {createEnvironment} from '../opt-in-helpers/runtime.mjs';
import {loadNativeCss} from '../opt-in-helpers/native.mjs';
const native=loadNativeCss(),proto=native.PreparedStylesV1.prototype;
const old=Object.getOwnPropertyDescriptor(proto,'resolveMatchingV1');
let calls=0; const sentinel=new Error("third callback must execute");
Object.defineProperty(proto,'resolveMatchingV1',{...old,value(input,token){
 calls++;if(calls===3)throw sentinel;const result=JSON.parse(Reflect.apply(old.value,this,[input,token]));
 if(calls===2)for(const row of result.targets)row.rendererValues.color=Array.from('rgb(0, 0, 255)',c=>c.charCodeAt(0));
 return JSON.stringify(result);
}});
try{
 const {window,document}=createEnvironment('<style>div{color:red}</style><div>x</div>');
 const el=document.querySelector('div');
 const first=window.getComputedStyle(el).color,second=window.getComputedStyle(el).color;
 console.log(JSON.stringify({first,second,calls}));
 assert.equal(first,'rgb(255, 0, 0)');assert.equal(second,'rgb(0, 0, 255)');assert.equal(calls,2);
 assert.throws(()=>window.getComputedStyle(el),error=>error===sentinel);assert.equal(calls,3);
}finally{Object.defineProperty(proto,'resolveMatchingV1',old)}

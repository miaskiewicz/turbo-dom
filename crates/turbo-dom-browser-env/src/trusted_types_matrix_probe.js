(async()=>{
 const error=f=>{try{const v=f();return{ok:true,value:v===undefined?null:v}}catch(e){return{error:e.name,message:e.message}}};
 const namespaces=[undefined,null,'','http://www.w3.org/1999/xhtml','http://www.w3.org/2000/svg','http://www.w3.org/1998/Math/MathML','urn:unknown'];
 const tags=['script','SCRIPT','iframe','object','embed','div','svg','*'];
 const attrs=['src','SRC','href','codebase','data','srcdoc','onclick','onmadeup','onconstructor'];
 const props=['src','href','codeBase','data','srcdoc','text','textContent','innerText','innerHTML','outerHTML','onclick'];
 const matrix=[];for(const ns of namespaces)for(const tag of tags)for(const name of [...new Set([...attrs,...props])]){const a=trustedTypes.getAttributeType(tag,name,ns),p=trustedTypes.getPropertyType(tag,name,ns);if(a||p)matrix.push([ns===undefined?'undefined':ns,tag,name,a,p]);}
 const conversion=[];for(const value of [{toString(){return Symbol('x')}},{[Symbol.toPrimitive](hint){conversion.push(hint);return 'str'}},Object(Symbol('x'))]){
  conversion.push(error(()=>trustedTypes.createPolicy(value,{createHTML:s=>s}).name));
 }
 const mappingArgs=[undefined,null,'','html','svg','http://www.w3.org/1999/xhtml',Symbol('x')].map(ns=>[typeof ns==='symbol'?'symbol':ns,error(()=>{const m=trustedTypes.getTypeMapping(ns);return m?Object.keys(m):m})]);
 return{matrix,conversion,mappingArgs,ownFactory:Object.getOwnPropertyNames(trustedTypes),frozen:error(()=>{Object.freeze(trustedTypes);return trustedTypes.createPolicy('after-freeze',{createHTML:s=>s}).createHTML('x').toString()})};
})()

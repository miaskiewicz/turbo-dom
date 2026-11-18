(() => {
  const target=globalThis.__windowIndexTarget, entry=globalThis.__windowNameCase;
  const caught=fn=>{try{return{ok:true,value:fn()}}catch(e){return{ok:false,name:e.name,accessingRealm:e instanceof DOMException}}};
  const childIndex=value=>{for(let i=0;i<entry.names.length;i++)if(target[i]===value)return i;return null};
  return {
    properties:[...new Set([...entry.names,...(entry.extraKeys||[])])].map(key=>({keyUnits:key.split('').map(unit=>unit.charCodeAt(0)),
      read:caught(()=>{const value=target[key];return{childIndex:childIndex(value),isUndefined:value===undefined}}),
      has:caught(()=>key in target),
      descriptor:caught(()=>{const d=Object.getOwnPropertyDescriptor(target,key);return d===undefined?null:{kind:('value'in d?'data':'accessor'),childIndex:childIndex(d.value),writable:d.writable,enumerable:d.enumerable,configurable:d.configurable,accessingRealm:Object.getPrototypeOf(d)===Object.prototype}})
    })),
    ownKeys:caught(()=>Reflect.ownKeys(target).map(k=>typeof k==='symbol'?String(k):k)),
    objectKeys:caught(()=>Object.keys(target))
  };
})()

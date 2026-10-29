(() => {
  const result = {}, fields = ["formAction","formEnctype","formMethod","formTarget"];
  const normalize = value => {
    if(typeof value !== "string" || !/^http:\/\/(?:127\.0\.0\.1|localhost)(?::\d+)?\//.test(value)) return value;
    const path=value.replace(/^http:\/\/(?:127\.0\.0\.1|localhost)(?::\d+)?/,"");
    return path==="/"||path==="/?case=0"?"<document>":"<origin>"+path;
  };
  for(const [name,kind,other] of [["HTMLButtonElement","button","input"],["HTMLInputElement","input","button"]]) {
    const element=document.createElement(kind);document.body.append(element);
    const output=result[name]={fields:{},actions:[],poison:{}};
    for(const field of fields) {
      const descriptor=Object.getOwnPropertyDescriptor(globalThis[name].prototype,field);
      const attribute=field.toLowerCase();
      const record=output.fields[field]={descriptor:[descriptor.enumerable,descriptor.configurable,descriptor.get.name,descriptor.get.length,descriptor.set.name,descriptor.set.length],missing:normalize(element[field]),values:[],errors:[]};
      for(const value of ["","POST","dialog","invalid","TEXT/PLAIN","multipart/form-data","_blank",null,undefined,"a\ud800b"]) {
        element[field]=value;record.values.push([normalize(element[field]),element.getAttribute(attribute)]);
      }
      for(const receiver of [{},document.createElement(other),document.createElementNS("http://www.w3.org/2000/svg",kind),document.createElementNS("http://www.w3.org/1999/xhtml",kind.toUpperCase()),Object.create(globalThis[name].prototype)]) {
        for(const method of [descriptor.get,descriptor.set])try{method.call(receiver,"x");record.errors.push("accepted");}catch(error){record.errors.push(error.name);}
      }
      try{element[field]=Symbol();record.symbol="accepted";}catch(error){record.symbol=error.name;}
      element.removeAttribute(attribute);
      element.setAttribute(attribute,"POST");record.attributeWrite=normalize(element[field]);element.removeAttribute(attribute);
      const borrowed=Object.create(null);borrowed.value=descriptor.get.call(element);record.borrowed=normalize(borrowed.value);
    }
    const base=document.createElement("base");base.href="https://base.example/dir/";document.head.append(base);
    for(const value of [null,"","child?q=1","../up","http://[","https://","http://bad host","https://[::1","http://%zz","https://a:bad","nosuch:abc","#fragment"]) {
      if(value===null)element.removeAttribute("formaction");else element.setAttribute("formaction",value);
      output.actions.push(normalize(element.formAction));
    }
    element.setAttribute("formaction","safe");
    let reads=0;Object.defineProperty(element,"ownerDocument",{get(){reads++;throw Error("guest owner");}});
    element.getAttribute=()=>{throw Error("guest attribute");};element.setAttribute=()=>{throw Error("guest write");};
    const d=Object.getOwnPropertyDescriptor(globalThis[name].prototype,"formAction");
    output.poison.before=normalize(d.get.call(element));d.set.call(element,"after");output.poison.after=normalize(d.get.call(element));output.poison.reads=reads;
    base.remove();element.remove();
  }
  return JSON.parse(JSON.stringify(result,(_key,value)=>typeof value==='string'&&/[\ud800-\udfff]/.test(value)?{utf16:Array.from({length:value.length},(_,index)=>value.charCodeAt(index))}:value));
})();

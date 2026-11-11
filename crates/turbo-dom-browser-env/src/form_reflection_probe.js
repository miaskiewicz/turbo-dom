(() => {
  const form = document.createElement("form"); document.body.append(form);
  const fields = ["action","method","enctype","encoding","target","name","acceptCharset","autocomplete","rel"];
  const result = {descriptors:{}, defaults:{}, values:{}, errors:{}};
  const normalize = value => {
    if(typeof value !== "string" || !/^http:\/\/(?:127\.0\.0\.1|localhost)(?::\d+)?\//.test(value)) return value;
    const path = value.replace(/^http:\/\/(?:127\.0\.0\.1|localhost)(?::\d+)?/, "");
    return path === "/" || path === "/?case=0" ? "<document>" : "<origin>"+path;
  };
  for (const name of fields) {
    const d = Object.getOwnPropertyDescriptor(HTMLFormElement.prototype,name);
    result.descriptors[name] = [d.enumerable,d.configurable,d.get.name,d.get.length,d.set.name,d.set.length];
    result.defaults[name] = normalize(form[name]);
    result.values[name] = [];
    for (const value of ["", "MiXeD", "POST", "dialog", "multipart/form-data", "text/plain", "off", null, "a\ud800b"]) {
      form[name] = value;
      result.values[name].push([normalize(form[name]),form.getAttribute(name === "encoding" ? "enctype" : name === "acceptCharset" ? "accept-charset" : name)]);
    }
    try { d.get.call({}); } catch(e) { result.errors[name] = e.name; }
    try { form[name] = Symbol(); } catch(e) { result.errors[name+"Symbol"] = e.name; }
    form.removeAttribute(name === "encoding" ? "enctype" : name === "acceptCharset" ? "accept-charset" : name);
  }
  const base=document.createElement("base");base.href="https://base.example/dir/";document.head.append(base);
  result.actions=[];
  for(const value of [null,"","child?q=1","../up","http://[", "https://", "http://bad host", "https://[::1", "http://%zz", "https://a:bad", "nosuch:abc", "https://[invalid]/x", "#x"]){if(value===null)form.removeAttribute("action");else form.setAttribute("action",value);result.actions.push(normalize(form.action));}
  base.remove();
  const list=form.relList,d=Object.getOwnPropertyDescriptor(HTMLFormElement.prototype,"relList");
  result.relList={same:list===form.relList,tag:Object.prototype.toString.call(list),descriptor:[d.enumerable,d.configurable,d.get.name,d.get.length,d.set.name,d.set.length],supports:{}};
  for(const token of ["noopener","noreferrer","opener","external","help","next","search","tag","NOOPENER"]){try{result.relList.supports[token]=list.supports(token);}catch(e){result.relList.supports[token]=e.name;}}
  form.rel="noopener next";list.add("external");result.relList.tokens=[...list];result.relList.attribute=form.getAttribute("rel");form.relList="opener";result.relList.assigned=[...list];
  form.remove();return JSON.parse(JSON.stringify(result,(_key,value)=>typeof value==='string'&&/[\ud800-\udfff]/.test(value)?{utf16:Array.from({length:value.length},(_,index)=>value.charCodeAt(index))}:value));
})();

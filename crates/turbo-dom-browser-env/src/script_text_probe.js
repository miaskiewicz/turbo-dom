(()=>{
  const units=v=>Array.from({length:v.length},(_,i)=>v.charCodeAt(i)),s=document.createElement('script'),descriptor=Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype,'text');
  const shape={get:descriptor.get.name,set:descriptor.set.name,getLength:descriptor.get.length,setLength:descriptor.set.length,enumerable:descriptor.enumerable,configurable:descriptor.configurable,getSource:String(descriptor.get),setSource:String(descriptor.set)};
  const lone=String.fromCharCode(0xd800),pair=String.fromCharCode(0xd83d,0xde80);s.text=lone+pair;
  const original={text:units(s.text),content:units(s.textContent),child:units(s.firstChild.data),children:s.childNodes.length,own:Object.hasOwn(s,'text')};
  const nested=document.createElement('span');nested.textContent='hidden';s.append(nested,document.createTextNode('tail'));
  const direct={text:units(s.text),content:units(s.textContent),children:s.childNodes.length};
  const clone=s.cloneNode(true),cloneValue={text:units(clone.text),content:units(clone.textContent),children:clone.childNodes.length};
  s.text=null;const nullValue=s.text;s.text=undefined;const undefinedValue=s.text;let calls=0;s.text={toString(){calls++;return lone;}};
  const coerced={calls,text:units(s.text),children:s.childNodes.length};let symbol;try{s.text=Symbol()}catch(e){symbol=e.name}const afterSymbol=units(s.text);
  const brands=[];for(const object of [{},Object.create(HTMLScriptElement.prototype),document.createElement('div'),document.createElementNS('http://www.w3.org/2000/svg','script'),new Proxy(s,{})]){
    let get,set;try{Reflect.apply(descriptor.get,object,[])}catch(e){get=e.name}try{Reflect.apply(descriptor.set,object,[{toString(){throw Error('wrong')}}])}catch(e){set=e.name}brands.push([get,set]);
  }
  return {shape,original,direct,cloneValue,nullValue,undefinedValue,coerced,symbol,afterSymbol,brands};
})()

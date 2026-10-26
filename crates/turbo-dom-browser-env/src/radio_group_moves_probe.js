(() => {
const out={};const flags=rows=>rows.map(node=>node.checked);
for(const mode of ['container','fragment','shadow']){
 const form=document.createElement('form');form.innerHTML='<input type=radio name=g checked value=old>';document.body.append(form);const a=form.firstChild;
 const box=mode==='fragment'?document.createDocumentFragment():document.createElement('div');
 box.append(document.createElement('input'));const b=box.firstChild;b.type='radio';b.name='g';b.value='new';b.checked=true;
 if(mode==='shadow'){const host=document.createElement('div');form.append(host);const shadow=host.attachShadow({mode:'open'});shadow.append(box);out[mode]=flags([a,b]);}
 else{form.append(box);out[mode]=flags([a,b]);}
 form.remove();
}
{
 const root=document.createElement('div');root.innerHTML='<form id=f1><input type=radio name=g checked value=inside></form><input type=radio name=g form=f2 checked value=outside>';
 document.body.append(root);const form=root.firstChild,a=form.firstChild,b=root.lastChild;const rows=[flags([a,b])];form.id='f2';rows.push(flags([a,b]));form.id='f1';rows.push(flags([a,b]));b.setAttribute('form','f1');rows.push(flags([a,b]));out.external=rows;root.remove();
}
{
 const f=document.createElement('form');f.innerHTML='<input type=radio name=g checked value=old>';document.body.append(f);const a=f.firstChild;
 const box=document.createElement('div');box.innerHTML='<form><input type=radio name=g checked value=first></form><form><input type=radio name=g checked value=last></form>';
 const b=box.firstChild.firstChild,c=box.lastChild.firstChild;
 const fragment=document.createDocumentFragment();fragment.append(b,c);f.append(fragment);out.fragmentOrder=[flags([a,b,c]),f.elements.namedItem('g').value];f.remove();
}
{
 const root=document.createElement('div');root.innerHTML='<form id="remove-owner"></form><input type=radio name=g form=remove-owner checked value=external><input type=radio name=g checked value=free>';
 const f=root.firstChild,b=f.nextSibling,c=root.lastChild;out.ownerRemovalDetached=[flags([b,c]),b.form===f,b.form===null];document.body.append(root);out.ownerRemoval=[flags([b,c])];f.remove();out.ownerRemoval.push(flags([b,c]));root.remove();
}
out.brands=[];
for(const name of ['checked','defaultChecked'])for(const receiver of [{},Object.create(HTMLInputElement.prototype),document.createElement('div'),document.createElementNS('http://www.w3.org/2000/svg','input'),null]){
 const descriptor=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,name);
 for(const method of [descriptor.get,descriptor.set]){try{Reflect.apply(method,receiver,[true]);out.brands.push('accepted');}catch(error){out.brands.push(error.name);}}
}
{
 const root=document.createElement('div');root.innerHTML='<input type=radio name=g form=missing checked><input type=radio name=g checked>';const a=root.firstChild,b=root.lastChild;
 out.unresolved=[flags([a,b])];a.checked=true;out.unresolved.push(flags([a,b]));b.checked=true;out.unresolved.push(flags([a,b]));document.body.append(root);out.unresolved.push(flags([a,b]));root.remove();
}
for(const kind of ['detachedForm','detachedFragment','detachedShadow']){
 const host=document.createElement(kind==='detachedForm'?'form':'div');
 const root=kind==='detachedShadow'?host.attachShadow({mode:'open'}):kind==='detachedFragment'?document.createDocumentFragment():host;
 const a=document.createElement('input'),b=document.createElement('input');for(const node of [a,b]){node.type='radio';node.name='g';node.defaultChecked=true;}root.append(a,b);
 out[kind]=[flags([a,b])];a.checked=true;out[kind].push(flags([a,b]));if(kind==='detachedShadow'){document.body.append(host);out[kind].push(flags([a,b]));host.remove();}
}
{
 const form=document.createElement('form');form.id='outside-shadow-form';document.body.append(form);
 const host=document.createElement('div');document.body.append(host);const root=host.attachShadow({mode:'open'});
 root.innerHTML='<input type=radio name=g form=outside-shadow-form checked><input type=radio name=g checked>';
 out.shadowFormBoundary=[root.firstChild.form===null,root.lastChild.form===null,flags([root.firstChild,root.lastChild])];
 root.innerHTML='<form id=inside-shadow-form></form><input type=radio name=g form=inside-shadow-form checked><input type=radio name=g checked>';
 out.shadowInternalForm=[root.children[1].form===root.firstChild,flags([root.children[1],root.lastChild])];host.remove();form.remove();
}
out.formGetterBrands=[];
for(const name of ['HTMLInputElement','HTMLButtonElement','HTMLTextAreaElement','HTMLSelectElement','HTMLFieldSetElement','HTMLOutputElement','HTMLObjectElement']){
 const getter=Object.getOwnPropertyDescriptor(globalThis[name].prototype,'form').get;
 for(const receiver of [{},document.createElement('div'),document.createElementNS('http://www.w3.org/2000/svg','input'),document.createElement(name==='HTMLInputElement'?'button':'input'),null])try{getter.call(receiver);out.formGetterBrands.push('accepted');}catch(error){out.formGetterBrands.push(error.name);}
}
out.formGetters=[];
{
 const form=document.createElement('form');form.id='getter-owner';document.body.append(form);
 for(const [name,tag] of [['HTMLInputElement','input'],['HTMLButtonElement','button'],['HTMLTextAreaElement','textarea'],['HTMLSelectElement','select'],['HTMLFieldSetElement','fieldset'],['HTMLOutputElement','output'],['HTMLObjectElement','object']]){
  const descriptor=Object.getOwnPropertyDescriptor(globalThis[name].prototype,'form'),element=document.createElement(tag);element.setAttribute('form','getter-owner');document.body.append(element);
  const owned=descriptor.get.call(element)===form;element.remove();form.append(element);element.removeAttribute('form');
  out.formGetters.push([name,descriptor.enumerable,descriptor.configurable,descriptor.get.name,descriptor.get.length,descriptor.set===undefined,owned,descriptor.get.call(element)===form]);
 }
 form.remove();
}
return out;
})();

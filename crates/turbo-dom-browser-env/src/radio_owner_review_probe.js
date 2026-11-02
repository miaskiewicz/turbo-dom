(() => {
 const out={};const flags=nodes=>nodes.map(node=>node.checked);
 for(const kind of ['nonform-first','svg-first','html-case-first','form-first']){
  const root=document.createElement('div'),form=document.createElement('form'),input=document.createElement('input');form.id='duplicate-owner';input.setAttribute('form','duplicate-owner');input.type='radio';input.name='g';
  const blocker=kind==='svg-first'?document.createElementNS('http://www.w3.org/2000/svg','form'):kind==='html-case-first'?document.createElementNS('http://www.w3.org/1999/xhtml','FORM'):document.createElement('div');blocker.id='duplicate-owner';
  root.append(...(kind==='form-first'?[form,blocker,input]:[blocker,form,input]));document.body.append(root);
  out[kind]=[input.form===form,input.form===null];blocker.remove();out[kind].push(input.form===form);root.remove();
 }
 for(const mode of ['open','closed']){
  const host=document.createElement('div');document.body.append(host);const root=host.attachShadow({mode});root.innerHTML='<div id=shadow-review></div><form id=shadow-review></form><input type=radio name=g form=shadow-review checked><input type=radio name=g checked>';
  const first=root.children[2],last=root.lastChild,form=root.children[1];out['shadow-'+mode]=[first.form===null,flags([first,last])];root.firstChild.remove();out['shadow-'+mode].push(first.form===form,flags([first,last]));host.remove();
 }
 {
  const form=document.createElement('form');form.innerHTML='<input type=radio name=g checked><input type=radio name=g>';document.body.append(form);const a=form.firstChild,b=form.lastChild;
  out.disabled=[];for(const mode of ['control','fieldset','form']){form.reset();if(mode==='control')b.disabled=true;if(mode==='fieldset'){const fs=document.createElement('fieldset');fs.disabled=true;form.append(fs);fs.append(b);}if(mode==='form')form.setAttribute('disabled','');b.checked=true;out.disabled.push(flags([a,b]));b.disabled=false;form.append(b);form.removeAttribute('disabled');}form.remove();
 }
 {
  const root=document.createElement('div');root.innerHTML='<input type=radio name=g form=reset-review checked value=before><form id=reset-review><input type=radio name=g checked value=inside></form><input type=radio name=g form=reset-review checked value=after>';document.body.append(root);
  const form=root.children[1],nodes=[root.firstChild,form.firstChild,root.lastChild];out.resetOrder=[flags(nodes),form.elements.namedItem('g').value];nodes[0].checked=true;form.reset();out.resetOrder.push(flags(nodes),form.elements.namedItem('g').value);root.remove();
 }
 {
  const root=document.createElement('div');root.innerHTML='<form><div><input type=radio name=g checked value=moved></div></form><form><input type=radio name=g checked value=target></form>';document.body.append(root);
  const first=root.firstChild,last=root.lastChild,box=first.firstChild,a=box.firstChild,b=last.firstChild;out.split=[flags([a,b])];last.append(box);out.split.push(flags([a,b]));first.append(box);out.split.push(flags([a,b]));root.remove();
 }
 {
  const doc=document.implementation.createDocument('http://www.w3.org/1999/xhtml','root'),root=doc.documentElement;
  const form=doc.createElementNS('http://www.w3.org/1999/xhtml','form'),input=doc.createElementNS('http://www.w3.org/1999/xhtml','input');form.id='xml-owner';input.setAttribute('form','xml-owner');input.setAttribute('type','radio');input.setAttribute('name','g');input.setAttribute('checked','');root.append(form,input);
  out.xml=[input.form===form,input.form===null,input.localName,form.localName];
 }
 return out;
})();

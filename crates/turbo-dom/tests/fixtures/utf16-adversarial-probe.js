(() => {
 const result={};
 const unit=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
 for(const before of [false,true]) {
  const d=document.implementation.createHTMLDocument(''), e=d.createElement('div');
  d.body.appendChild(e);
  const ns=()=>e.setAttributeNS('urn:adversarial','id','\uD800');
  const plain=()=>e.setAttributeNS(null,'id','t');
  if(before){ns();plain()}else{plain();ns()}
  result['namespacedId'+before]={byId:d.getElementById('t')===e,css:e.matches('#t'), getAttribute:unit(e.getAttribute('id')), attributes:[...e.attributes].map(a=>({namespace:a.namespaceURI,name:a.name,value:unit(a.value)}))};
 }
 const prefixElement=document.createElement('div');
 prefixElement.setAttributeNS('urn:adversarial','p:v','\uD800');
 result.prefixed={actual:prefixElement.matches("[p\\:v='\uD800']"),replacement:prefixElement.matches("[p\\:v='\uFFFD']"),getAttribute:unit(prefixElement.getAttribute('p:v'))};
 const e=document.createElement('div');e.className='ab';e.id='t';
 result.hexCRLF=e.matches('.\\61\r\nb');
 result.attributeFlag=e.matches("[id='T' i]");
 const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
 svg.innerHTML="<circle data-u='\uD800'></circle>";
 result.svgFragment={namespace:svg.firstChild.namespaceURI,localName:svg.firstChild.localName, units:[...svg.firstChild.getAttribute('data-u')].map(c=>c.charCodeAt(0))};
 result.fragments=[];
 const contexts=[['http://www.w3.org/2000/svg','svg',null],['http://www.w3.org/2000/svg','foreignObject',null],['http://www.w3.org/1998/Math/MathML','math',null],['http://www.w3.org/1998/Math/MathML','mi',null],['http://www.w3.org/1998/Math/MathML','annotation-xml','text/html'],['http://www.w3.org/1998/Math/MathML','annotation-xml','application/xhtml+xml'],['http://www.w3.org/1998/Math/MathML','annotation-xml','other']];
 const flatten=parent=>[...parent.childNodes].map(n=>({type:n.nodeType,namespace:n.namespaceURI??null,name:n.localName??null,value:n.nodeValue===null?null:unit(n.nodeValue),attribute:n.nodeType===1?unit(n.getAttribute('data-u')??''):null,children:flatten(n)}));
 for(const [namespace,name,encoding] of contexts)for(const value of ['A','\uFEFFA','\uD800','\uDC00'])for(const position of ['inner','afterbegin','beforeend','beforebegin','afterend']){
  const host=document.createElementNS(namespace,name);
  if(encoding)host.setAttribute('encoding',encoding);
  let source="<a data-u='"+value+"'>"+value+"</a>";
  if(value[0]==='\uFEFF')source='\uFEFF'+source;
  let output=host;
  if(position==='inner')host.innerHTML=source;
  else if(position==='beforebegin'||position==='afterend'){
   const anchor=document.createElementNS(namespace,namespace.includes('/svg')?'circle':'mn');host.appendChild(anchor);anchor.insertAdjacentHTML(position,source);host.removeChild(anchor);
  }else host.insertAdjacentHTML(position,source);
  result.fragments.push({namespace,name,encoding,position,source:unit(source),output:flatten(output)});
 }
 return result;
})()

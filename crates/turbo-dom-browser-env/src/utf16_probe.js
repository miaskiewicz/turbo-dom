(async () => {
  const units = value => Array.from({length:String(value).length}, (_,i)=>String(value).charCodeAt(i));
  const source = String.fromCharCode(0x41,0xd800,0x42,0xdc00,0xd83d,0xde00,0);
  const text = document.createTextNode(source), comment=document.createComment(source);
  const element=document.createElement('div');element.setAttribute('data-units',source);element.append(text);
  const input=document.createElement('input');input.value=source;
  const rendered=document.createElement('div');document.body.append(rendered);rendered.innerText=source;const innerTextUnits=units(rendered.innerText);const replacement=document.createElement('b');rendered.append(replacement);replacement.outerText=source;const outerTextUnits=units(rendered.textContent);
  const reflected=document.createElement('div');reflected.id=source;reflected.className=source;reflected.append(source);const reflectedValues={id:units(reflected.id),className:units(reflected.className),variadic:units(reflected.textContent),dataset:units(element.dataset.units)};
  const parsed=typeof DOMParser.prototype.parseFromString==='function'?new DOMParser().parseFromString('<p data-u="'+source+'">'+source+'</p>','text/html').querySelector('p'):null;
  const target=document.implementation.createHTMLDocument('surrogates');
  const clone=element.cloneNode(true), imported=typeof target.importNode==='function'?target.importNode(element,true):globalThis.__utf16HostImport(target,element,true), adopted=target.adoptNode(clone);
  const pair=document.createTextNode('😀');const tail=pair.splitText(1);const halves=[units(pair.data),units(tail.data)];
  const normalized=document.createElement('div');normalized.append(pair,tail);normalized.normalize();
  const range=document.createRange();range.setStart(text,1);range.setEnd(text,6);
  const rangeValue=units(range.toString()), rangeClone=units(range.cloneContents().textContent);
  const mutations=[];const observer=new MutationObserver(records=>mutations.push(...records));
  observer.observe(element,{characterData:true,characterDataOldValue:true,attributes:true,attributeOldValue:true,subtree:true});
  text.replaceData(2,1,String.fromCharCode(0xdfff));element.setAttribute('data-units',String.fromCharCode(0xd801));
  await Promise.resolve();observer.disconnect();
  const html=document.createElement('div');html.innerHTML='<b>'+source+'</b>';
  const charOps=document.createTextNode(source);const substring=units(charOps.substringData(1,5));charOps.deleteData(2,2);charOps.insertData(2,String.fromCharCode(0xd888));charOps.appendData(String.fromCharCode(0xd999));
  return {innerTextUnits,outerTextUnits,reflectedValues,source:units(source),created:units(comment.data),input:units(input.value),attribute:units(imported.getAttribute('data-units')),attrNode:units(imported.attributes[0].value),imported:units(imported.textContent),adopted:units(adopted.textContent),halves,normalized:units(normalized.textContent),substring,charOps:units(charOps.data),rangeValue,rangeClone,mutationOld:mutations.map(record=>({type:record.type,old:units(record.oldValue)})),parsedText:parsed?units(parsed.textContent):null,parsedAttr:parsed?units(parsed.getAttribute('data-u')):null,innerHTML:units(html.textContent),serialized:units(imported.outerHTML)};
})()
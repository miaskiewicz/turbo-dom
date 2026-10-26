(() => {
const unit=value=>value===null?null:Array.from({length:value.length},(_,i)=>value.charCodeAt(i));
const high=String.fromCharCode(0xd800),low=String.fromCharCode(0xdc00),marker=String.fromCodePoint(0xf0000),payload=high+low+high+marker+marker+String.fromCodePoint(0xf0800)+low;
const source='<!DOCTYPE '+high+' PUBLIC "'+low+'" "'+high+'"><body><a '+high+'="'+high+'" '+low+'="'+low+'" �="replacement" data-u="'+payload+'&#983040;&#985088;">'+payload+'&#983040;&#985088;&amp;\0</a><!--'+payload+'--><textarea>'+payload+'&#983040;</textarea><script>'+payload+'&#983040;</script><table>'+high+'<tr><td>'+low+'</table><svg><text>'+high+'</text></svg>';
const parsed=typeof __utf16HostParse==='function'?__utf16HostParse(source):new DOMParser().parseFromString(source,'text/html');
const snapshot=node=>({type:node.nodeType,name:unit(node.nodeName),namespace:unit(node.namespaceURI??null),value:unit(node.nodeValue),attrs:Array.from(node.attributes??[],a=>({name:unit(a.name),namespace:unit(a.namespaceURI),value:unit(a.value)})),children:Array.from(node.childNodes,snapshot)});
const inner=document.createElement('div');inner.innerHTML='<a '+high+'="'+high+'" '+low+'="'+low+'">'+payload+'&#983040;&#985088;</a><table>'+high+'<tr><td>'+low+'</table><!--'+payload+'-->';
const textarea=document.createElement('textarea');textarea.innerHTML=high+'&#983040;';
const script=document.createElement('script');script.innerHTML=low+'&#983040;';
return {tree:snapshot(parsed),inner:snapshot(inner),serialized:unit(inner.innerHTML),textarea:unit(textarea.textContent),script:unit(script.textContent),doctype:[unit(parsed.doctype.name),unit(parsed.doctype.publicId),unit(parsed.doctype.systemId)]};
})()

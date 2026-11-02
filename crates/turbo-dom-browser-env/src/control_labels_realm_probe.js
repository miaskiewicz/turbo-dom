(() => {
 const frame=document.createElement('iframe');document.body.append(frame);const child=frame.contentWindow,doc=frame.contentDocument;
 doc.body.innerHTML='<label for=child-input></label><input id=child-input>';
 const input=doc.querySelector('input'),get=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'labels').get;
 const first=get.call(input);
 const parent=document.createElement('input');parent.id='parent-input';document.body.append(parent);
 const reverse=Object.getOwnPropertyDescriptor(child.HTMLInputElement.prototype,'labels').get.call(parent);
 const result={parentBorrowFirst:[first instanceof NodeList,first instanceof child.NodeList,input.labels===first,Object.getPrototypeOf(first)===child.NodeList.prototype],childBorrowFirst:[reverse instanceof NodeList,reverse instanceof child.NodeList,parent.labels===reverse,Object.getPrototypeOf(reverse)===NodeList.prototype],baseBorrow:NodeList.prototype.item.call(first,0).localName};
 frame.remove();result.retained=[first.length,first[0].htmlFor,input.labels===first];parent.remove();return result;
})();
